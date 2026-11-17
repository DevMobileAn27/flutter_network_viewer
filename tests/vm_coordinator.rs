use flutter_network_viewer::{
    coordinator::{ConnectionFactory, Coordinator, CoordinatorConfig},
    lifecycle::{CaptureTransport, PollConfig, Request},
    vm::{ConnectionCancel, RpcError},
};
use serde_json::{Value, json};
use std::{cell::RefCell, collections::VecDeque, rc::Rc, sync::mpsc};

#[derive(Default)]
struct Wire {
    calls: Vec<Request>,
    replies: VecDeque<(u64, Result<Value, RpcError>)>,
    notifications: VecDeque<Value>,
    stopped: bool,
}
#[derive(Clone)]
struct Transport(Rc<RefCell<Wire>>);
impl CaptureTransport for Transport {
    fn submit(&mut self, r: Request) -> Result<(), RpcError> {
        self.0.borrow_mut().calls.push(r);
        Ok(())
    }
    fn take_reply(&mut self) -> Option<(u64, Result<Value, RpcError>)> {
        self.0.borrow_mut().replies.pop_front()
    }
    fn take_notification(&mut self) -> Option<Value> {
        self.0.borrow_mut().notifications.pop_front()
    }
    fn discard_notifications(&mut self) {
        self.0.borrow_mut().notifications.clear();
    }
    fn stop(&mut self) {
        self.0.borrow_mut().stopped = true;
    }
}
struct Attempt {
    cancel: ConnectionCancel,
    tx: mpsc::Sender<Result<Transport, RpcError>>,
    uri: String,
}
#[derive(Clone, Default)]
struct Factory(Rc<RefCell<Vec<Attempt>>>);
impl ConnectionFactory for Factory {
    type Transport = Transport;
    fn begin(
        &mut self,
        uri: &str,
        cancel: ConnectionCancel,
    ) -> mpsc::Receiver<Result<Transport, RpcError>> {
        let (tx, rx) = mpsc::channel();
        self.0.borrow_mut().push(Attempt {
            cancel,
            tx,
            uri: uri.into(),
        });
        rx
    }
}
type Viewer = Coordinator<Factory>;
fn setup() -> (Viewer, Factory) {
    let f = Factory::default();
    (
        Coordinator::new(
            f.clone(),
            CoordinatorConfig {
                retry_ms: 30,
                discovery_ms: 10,
                poll: PollConfig {
                    base_ms: 10,
                    idle_increment_ms: 0,
                    max_ms: 10,
                    ..Default::default()
                },
            },
        ),
        f,
    )
}
fn connected(c: &mut Viewer, f: &Factory, attempt: usize, now: u64) -> Transport {
    let t = Transport(Rc::default());
    f.0.borrow()[attempt].tx.send(Ok(t.clone())).unwrap();
    c.tick(now);
    t
}
fn reply(t: &Transport, method: &str, v: Result<Value, RpcError>) {
    let r = t.0.borrow().calls.last().unwrap().clone();
    assert_eq!(r.method, method);
    t.0.borrow_mut().replies.push_back((r.id, v));
}
fn vm(ids: &[&str]) -> Value {
    json!({"pid":10,"startTime":100,"operatingSystem":"android","isolates":ids.iter().map(|id| json!({"id":id,"name":"main"})).collect::<Vec<_>>()})
}
fn isolate(available: bool) -> Value {
    json!({"name":"main","rootLib":{"uri":"package:shop/main.dart"},"extensionRPCs":if available {vec!["ext.dart.io.getHttpProfile"]} else {vec![]}})
}
fn bootstrap(c: &mut Viewer, t: &Transport, ids: &[&str], now: u64) {
    reply(t, "getVM", Ok(vm(ids)));
    c.tick(now);
    for _ in ids {
        reply(t, "getIsolate", Ok(isolate(true)));
        c.tick(now);
        reply(t, "ext.dart.io.httpEnableTimelineLogging", Ok(json!({})));
        c.tick(now);
    }
}
fn profile(id: &str) -> Value {
    json!({"id":id,"method":"GET","uri":"https://example.com","request":{},"response":{"statusCode":200,"endTime":2}})
}
fn capture(c: &mut Viewer, t: &Transport, id: &str, stamp: u64, now: u64) {
    reply(
        t,
        "ext.dart.io.getHttpProfile",
        Ok(json!({"timestamp":stamp,"requests":[profile(id)]})),
    );
    c.tick(now);
    reply(t, "ext.dart.io.getHttpProfileRequest", Ok(profile(id)));
    c.tick(now);
}
#[test]
fn latest_connect_wins_and_stop_cancel_pending_attempts() {
    let (mut c, f) = setup();
    c.connect("ws://localhost:1/a=/ws", 0).unwrap();
    c.connect("ws://localhost:2/b=/ws", 0).unwrap();
    assert!(f.0.borrow()[0].cancel.is_cancelled());
    let _late = connected(&mut c, &f, 1, 0);
    assert_eq!(c.endpoint(), Some("ws://localhost:2/b=/ws"));
    assert!(
        f.0.borrow()[0]
            .tx
            .send(Ok(Transport(Rc::default())))
            .is_err()
    );
    c.connect("ws://localhost:3/ws", 1).unwrap();
    c.stop();
    assert!(f.0.borrow()[2].cancel.is_cancelled());
    c.tick(1000);
    assert_eq!(f.0.borrow().len(), 3);
    assert!(!c.running());
    assert!(c.store().sessions().is_empty());
}
#[test]
fn dispose_is_terminal_and_invalid_input_does_not_start_attempt() {
    let (mut c, f) = setup();
    assert!(c.connect("bad", 0).is_err());
    assert!(f.0.borrow().is_empty());
    c.connect("ws://localhost:1/ws", 0).unwrap();
    drop(c);
    assert!(f.0.borrow()[0].cancel.is_cancelled());
}
#[test]
fn bootstrap_delayed_extensions_and_enable_before_capture() {
    let (mut c, f) = setup();
    c.connect("ws://localhost:1/ws", 0).unwrap();
    let t = connected(&mut c, &f, 0, 0);
    reply(&t, "getVM", Ok(vm(&["i"])));
    c.tick(0);
    reply(&t, "getIsolate", Ok(isolate(false)));
    c.tick(0);
    assert!(c.store().sessions().is_empty());
    c.tick(10);
    reply(&t, "getVM", Ok(vm(&["i"])));
    c.tick(10);
    reply(&t, "getIsolate", Ok(isolate(true)));
    c.tick(10);
    assert_eq!(
        t.0.borrow().calls.last().unwrap().params,
        json!({"isolateId":"i","enabled":"true"})
    );
    reply(&t, "ext.dart.io.httpEnableTimelineLogging", Ok(json!({})));
    c.tick(10);
    capture(&mut c, &t, "r", 20, 10);
    assert_eq!(c.store().network().len(), 1);
    assert_eq!(c.store().sessions().len(), 1);
    assert!(c.running());
}
#[test]
fn profile_timeout_preserves_socket_history_and_reattaches_exclusive_clear_cursor() {
    let (mut c, f) = setup();
    c.connect("ws://localhost:1/ws", 0).unwrap();
    let t = connected(&mut c, &f, 0, 0);
    bootstrap(&mut c, &t, &["i"], 0);
    capture(&mut c, &t, "old", 100, 0);
    c.clear(0);
    // Control refresh precedes the next profile cycle.
    c.tick(10);
    reply(&t, "getVM", Ok(vm(&["i"])));
    c.tick(10);
    assert_eq!(
        t.0.borrow().calls.last().unwrap().params["updatedSince"],
        "101"
    );
    reply(&t, "ext.dart.io.getHttpProfile", Err(RpcError::Timeout));
    c.tick(10);
    assert!(c.running());
    assert!(!t.0.borrow().stopped);
    c.tick(20);
    reply(&t, "getVM", Ok(vm(&["i"])));
    c.tick(20);
    reply(&t, "getIsolate", Ok(isolate(true)));
    c.tick(20);
    reply(&t, "ext.dart.io.httpEnableTimelineLogging", Ok(json!({})));
    c.tick(20);
    assert_eq!(
        t.0.borrow().calls.last().unwrap().params["updatedSince"],
        "101"
    );
    capture(&mut c, &t, "fresh", 102, 20);
    assert_eq!(c.store().network().len(), 1);
    assert_eq!(f.0.borrow().len(), 1);
}
#[test]
fn socket_closure_reconnects_same_url_with_clear_boundary_and_endpoint_switch_cleans_session() {
    let (mut c, f) = setup();
    c.connect("ws://localhost:1/token=/ws", 0).unwrap();
    let t = connected(&mut c, &f, 0, 0);
    bootstrap(&mut c, &t, &["i"], 0);
    capture(&mut c, &t, "old", 100, 0);
    c.clear(0);
    t.0.borrow_mut()
        .replies
        .push_back((0, Err(RpcError::Transport("closed".into()))));
    c.tick(1);
    assert!(!c.running());
    c.tick(30);
    assert_eq!(f.0.borrow().len(), 1);
    c.tick(31);
    assert_eq!(f.0.borrow()[1].uri, "ws://localhost:1/token=/ws");
    let t2 = connected(&mut c, &f, 1, 31);
    bootstrap(&mut c, &t2, &["i"], 31);
    assert_eq!(
        t2.0.borrow().calls.last().unwrap().params["updatedSince"],
        "101"
    );
    capture(&mut c, &t2, "fresh", 102, 31);
    c.connect("ws://localhost:2/ws", 32).unwrap();
    assert!(c.store().sessions().is_empty());
    assert!(c.store().network().is_empty());
    c.stop();
}
#[test]
fn multiple_isolates_share_one_bounded_store_and_hot_restart_one_tab() {
    let (mut c, f) = setup();
    c.connect("ws://localhost:1/ws", 0).unwrap();
    let t = connected(&mut c, &f, 0, 0);
    bootstrap(&mut c, &t, &["a", "b"], 0);
    // Profiles are serialized on the shared transport.
    reply(
        &t,
        "ext.dart.io.getHttpProfile",
        Ok(json!({"timestamp":10,"requests":[profile("same")]})),
    );
    c.tick(0);
    reply(
        &t,
        "ext.dart.io.getHttpProfile",
        Ok(json!({"timestamp":10,"requests":[profile("same")]})),
    );
    c.tick(0);
    reply(&t, "ext.dart.io.getHttpProfileRequest", Ok(profile("same")));
    c.tick(0);
    reply(&t, "ext.dart.io.getHttpProfileRequest", Ok(profile("same")));
    c.tick(0);
    assert_eq!(c.store().sessions().len(), 1);
    assert_eq!(c.store().network().len(), 2);
    let app = c.store().sessions().keys().next().unwrap().clone();
    c.tick(10);
    reply(&t, "getVM", Ok(vm(&[])));
    c.tick(10);
    assert!(!c.is_online(&app));
    assert_eq!(c.store().network().len(), 2);
    c.tick(20);
    bootstrap(&mut c, &t, &["new"], 20);
    capture(&mut c, &t, "same", 20, 20);
    assert!(c.is_online(&app));
    assert_eq!(c.store().sessions().len(), 1);
    assert_eq!(c.store().network().len(), 3);
    assert_eq!(
        c.store().sessions()[&app].session["isolates"],
        json!([{"id":"new","name":"main"}])
    );
    c.stop();
    assert!(c.store().network().is_empty());
    assert!(t.0.borrow().stopped);
}
#[test]
fn isolate_exit_discards_pending_and_late_reply_without_dropping_history() {
    let (mut c, f) = setup();
    c.connect("ws://localhost:1/ws", 0).unwrap();
    let t = connected(&mut c, &f, 0, 0);
    bootstrap(&mut c, &t, &["old"], 0);
    capture(&mut c, &t, "before", 20, 0);
    c.tick(10);
    reply(&t, "getVM", Ok(vm(&["old"])));
    c.tick(10);
    let hung = t.0.borrow().calls.last().unwrap().id;
    t.0.borrow_mut().notifications.push_back(json!({"method":"streamNotify","params":{"streamId":"Isolate","event":{"kind":"IsolateExit","isolate":{"id":"old"}}}}));
    // Wire worker reports cancelled RPC while preserving its socket.
    t.0.borrow_mut().replies.push_back((
        hung,
        Err(RpcError::Remote {
            code: -32800,
            message: "isolate exited".into(),
            data: Value::Null,
        }),
    ));
    c.tick(11);
    bootstrap(&mut c, &t, &["new"], 11);
    capture(&mut c, &t, "after", 30, 11);
    t.0.borrow_mut().replies.push_back((
        hung,
        Ok(json!({"timestamp":40,"requests":[profile("late")]})),
    ));
    c.tick(12);
    assert_eq!(c.store().network().len(), 2);
    assert_eq!(c.store().sessions().len(), 1);
    assert_eq!(f.0.borrow().len(), 1);
}
#[test]
fn clear_while_disconnected_advances_saved_position_before_reconnect() {
    let (mut c, f) = setup();
    c.connect("ws://localhost:1/ws", 0).unwrap();
    let t = connected(&mut c, &f, 0, 0);
    bootstrap(&mut c, &t, &["i"], 0);
    capture(&mut c, &t, "old", 100, 0);
    t.0.borrow_mut()
        .replies
        .push_back((0, Err(RpcError::Transport("closed".into()))));
    c.tick(1);
    c.clear(2);
    c.tick(31);
    let t = connected(&mut c, &f, 1, 31);
    bootstrap(&mut c, &t, &["i"], 31);
    assert_eq!(
        t.0.borrow().calls.last().unwrap().params["updatedSince"],
        "101"
    );
}
#[test]
fn multi_isolate_console_keeps_wire_order_and_separate_partial_buffers() {
    use base64::Engine;
    let (mut c, f) = setup();
    c.connect("ws://localhost:1/ws", 0).unwrap();
    let t = connected(&mut c, &f, 0, 0);
    bootstrap(&mut c, &t, &["a", "b"], 0);
    for (id, text) in [
        ("b", "partial "),
        ("a", "first\n"),
        ("b", "second\n"),
        ("a", "third\n"),
    ] {
        t.0.borrow_mut().notifications.push_back(json!({"method":"streamNotify","params":{"streamId":"Stdout","event":{"isolate":{"id":id,"name":"main"},"bytes":base64::engine::general_purpose::STANDARD.encode(text)}}}));
    }
    c.tick(10);
    assert_eq!(
        c.store()
            .logs()
            .iter()
            .map(|l| l.event["message"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["first", "partial second", "third"]
    );
    assert_eq!(
        c.store()
            .logs()
            .iter()
            .map(|l| l.event["isolateId"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["a", "b", "a"]
    );
    assert_eq!(c.store().sessions().len(), 1);
}
#[test]
fn clear_before_first_summary_survives_reconnect_without_replaying_unknown_history() {
    let (mut c, f) = setup();
    c.connect("ws://localhost:1/ws", 0).unwrap();
    let t = connected(&mut c, &f, 0, 0);
    bootstrap(&mut c, &t, &["i"], 0);
    c.clear(0);
    t.0.borrow_mut()
        .replies
        .push_back((0, Err(RpcError::Transport("closed".into()))));
    c.tick(1);
    c.tick(31);
    let t = connected(&mut c, &f, 1, 31);
    bootstrap(&mut c, &t, &["i"], 31);
    reply(
        &t,
        "ext.dart.io.getHttpProfile",
        Ok(json!({"timestamp":100,"requests":[profile("cleared")]})),
    );
    c.tick(31);
    assert!(c.store().network().is_empty());
    c.tick(41);
    reply(&t, "getVM", Ok(vm(&["i"])));
    c.tick(41);
    assert_eq!(
        t.0.borrow().calls.last().unwrap().params["updatedSince"],
        "101"
    );
}
#[test]
fn clear_and_pause_drop_raw_notifications_across_all_isolates() {
    use base64::Engine;
    let (mut c, f) = setup();
    c.connect("ws://localhost:1/ws", 0).unwrap();
    let t = connected(&mut c, &f, 0, 0);
    bootstrap(&mut c, &t, &["a", "b"], 0);
    let event = |id: &str, text: &str| json!({"method":"streamNotify","params":{"streamId":"Stdout","event":{"isolate":{"id":id},"bytes":base64::engine::general_purpose::STANDARD.encode(text)}}});
    for id in ["a", "b"] {
        t.0.borrow_mut().notifications.push_back(event(id, "old "));
    }
    c.tick(1);
    t.0.borrow_mut()
        .notifications
        .push_back(event("a", "clear\n"));
    c.clear(2);
    c.tick(2);
    for id in ["a", "b"] {
        t.0.borrow_mut()
            .notifications
            .push_back(event(id, "discard "));
    }
    c.tick(3);
    c.set_paused(true);
    c.set_paused(false);
    for id in ["a", "b"] {
        t.0.borrow_mut()
            .notifications
            .push_back(event(id, "fresh\n"));
    }
    c.tick(10);
    assert_eq!(c.store().logs().len(), 2);
    assert!(
        c.store()
            .logs()
            .iter()
            .all(|l| l.event["message"] == "fresh")
    );
}
#[test]
fn all_isolates_obey_shared_record_and_pending_console_budget() {
    use base64::Engine;
    let f = Factory::default();
    let mut c = Coordinator::new(
        f.clone(),
        CoordinatorConfig {
            retry_ms: 30,
            discovery_ms: 10,
            poll: PollConfig {
                base_ms: 10,
                max_ms: 10,
                max_records: 2,
                ..Default::default()
            },
        },
    );
    c.connect("ws://localhost:1/ws", 0).unwrap();
    let t = connected(&mut c, &f, 0, 0);
    bootstrap(&mut c, &t, &["a", "b"], 0);
    for (i, id) in ["a", "b", "a", "b", "a"].iter().enumerate() {
        t.0.borrow_mut().notifications.push_back(json!({"method":"streamNotify","params":{"streamId":"Stdout","event":{"isolate":{"id":id},"bytes":base64::engine::general_purpose::STANDARD.encode(format!("{i}\n"))}}}));
        c.tick(1);
    }
    c.tick(10);
    assert_eq!(c.store().logs().len(), 2);
    assert_eq!(c.store().statistics.evicted, 3);
    assert_eq!(c.store().logs().front().unwrap().event["message"], "3");
    assert_eq!(c.store().logs().back().unwrap().event["message"], "4");
}
#[test]
fn vm_health_timeout_retries_same_endpoint_and_stop_cancels_retry() {
    let (mut c, f) = setup();
    c.connect("ws://localhost:1/auth=/ws", 0).unwrap();
    let t = connected(&mut c, &f, 0, 0);
    bootstrap(&mut c, &t, &["i"], 0);
    capture(&mut c, &t, "before", 100, 0);
    c.tick(10);
    reply(&t, "getVM", Err(RpcError::Timeout));
    c.tick(10);
    assert!(!c.running());
    assert!(t.0.borrow().stopped);
    assert_eq!(c.store().network().len(), 1);
    c.tick(40);
    assert_eq!(f.0.borrow()[1].uri, "ws://localhost:1/auth=/ws");
    c.stop();
    c.tick(1000);
    assert_eq!(f.0.borrow().len(), 2);
    assert!(f.0.borrow()[1].cancel.is_cancelled());
}
#[test]
fn initial_and_later_profile_timeouts_keep_data_and_socket() {
    let (mut c, f) = setup();
    c.connect("ws://localhost:1/ws", 0).unwrap();
    let t = connected(&mut c, &f, 0, 0);
    bootstrap(&mut c, &t, &["i"], 0);
    reply(&t, "ext.dart.io.getHttpProfile", Err(RpcError::Timeout));
    c.tick(0);
    assert!(c.running());
    assert!(!t.0.borrow().stopped);
    c.tick(10);
    bootstrap(&mut c, &t, &["i"], 10);
    capture(&mut c, &t, "before", 100, 10);
    c.tick(20);
    reply(&t, "getVM", Ok(vm(&["i"])));
    c.tick(20);
    reply(&t, "ext.dart.io.getHttpProfile", Err(RpcError::Timeout));
    c.tick(20);
    assert_eq!(c.store().network().len(), 1);
    assert!(c.running());
    c.tick(30);
    bootstrap(&mut c, &t, &["i"], 30);
    capture(&mut c, &t, "after", 101, 30);
    assert_eq!(c.store().network().len(), 2);
    assert_eq!(f.0.borrow().len(), 1);
    assert!(!t.0.borrow().stopped);
}

#[test]
fn early_stdout_before_vm_isolate_and_profile_bootstrap_is_retained() {
    use base64::Engine;
    for boundary in ["normal", "clear", "pause"] {
        let (mut c, f) = setup();
        c.connect("ws://localhost:1/ws", 0).unwrap();
        let t = connected(&mut c, &f, 0, 0);
        t.0.borrow_mut().notifications.push_back(json!({"method":"streamNotify","params":{"streamId":"Stdout","event":{"isolate":{"id":"i"},"bytes":base64::engine::general_purpose::STANDARD.encode("early\n")}}}));
        c.tick(1); // getVM has not replied; no engine/profile/session exists.
        if boundary == "clear" {
            c.clear(2);
        }
        if boundary == "pause" {
            c.set_paused(true);
            c.set_paused(false);
        }
        bootstrap(&mut c, &t, &["i"], 3);
        c.tick(10);
        let messages: Vec<_> = c.store().logs().iter().map(|r| r.message()).collect();
        assert_eq!(
            messages,
            if boundary == "normal" {
                vec!["early".to_owned()]
            } else {
                vec![]
            },
            "{boundary}"
        );
    }
}

#[test]
fn early_console_queue_is_bounded_by_count_and_bytes_before_bootstrap() {
    use base64::Engine;
    for (max_records, max_bytes) in [(2, 65536), (20, 300)] {
        let f = Factory::default();
        let mut c = Coordinator::new(
            f.clone(),
            CoordinatorConfig {
                poll: PollConfig {
                    base_ms: 10,
                    idle_increment_ms: 0,
                    max_ms: 10,
                    max_records,
                    max_bytes,
                },
                ..Default::default()
            },
        );
        c.connect("ws://localhost:1/ws", 0).unwrap();
        let t = connected(&mut c, &f, 0, 0);
        for i in 0..10 {
            t.0.borrow_mut().notifications.push_back(json!({"method":"streamNotify","params":{"streamId":"Stdout","event":{"isolate":{"id":"i"},"bytes":base64::engine::general_purpose::STANDARD.encode(format!("{i}\n"))}}}));
            c.tick(1);
        }
        assert!(c.store().statistics.evicted >= 8);
        bootstrap(&mut c, &t, &["i"], 2);
        c.tick(10);
        assert!(c.store().logs().len() <= 2);
        assert_eq!(c.store().logs().back().unwrap().message(), "9");
        c.stop();
        assert!(c.store().logs().is_empty());
    }
}
