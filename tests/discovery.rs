use flutter_network_viewer::{
    coordinator::{ConnectionFactory, CoordinatorConfig},
    discovery::{AppDiscovery, DiscoveryFactory, Target, process_uris, target_from_vm},
    lifecycle::{CaptureTransport, Request},
    vm::{ConnectionCancel, RpcError},
};
use serde_json::{Value, json};
use std::{cell::RefCell, collections::VecDeque, rc::Rc, sync::mpsc};
fn target(id: &str) -> Target {
    Target {
        uri: format!(
            "ws://localhost:{}/a%2Fb%3D/ws",
            if id == "one" { 1111 } else { 2222 }
        ),
        name: id.into(),
        project_key: format!("package:{id}/main.dart"),
        vm_key: if id == "one" {
            "10:100:android".into()
        } else {
            "20:100:android".into()
        },
    }
}
#[test]
fn only_local_dds_arguments_preserve_encoded_auth_and_cap_candidates() {
    let text = r#"
/sdk/dart development-service --vm-service-uri=http://127.0.0.1:1234/a%2Fb%3D/
/sdk/dart development-service --vm-service-uri=http://127.0.0.1:1234/a%2Fb%3D/
"C:\sdk\dart.exe" development-service --vm-service-uri "http://localhost:2345/b=/"
/sdk/dartaotruntime /cache/dds_aot.dart.snapshot --vm-service-uri=http://[::1]:3456/c=/
/sdk/dart devtools --vm-service-uri=http://127.0.0.1:4567/no=/
/sdk/dart development-service --vm-service-uri=http://example.com:5678/no=/
/sdk/dart development-service --vm-service-uri=https://localhost:42/no=/
/sdk/dart development-service --vm-service-uri=http://user@localhost:42/no=/
/sdk/dart development-service --vm-service-uri=http://localhost/no=/
/sdk/dart development-service --vm-service-uri=file:///tmp/no
"#;
    assert_eq!(
        process_uris(text),
        vec![
            "http://127.0.0.1:1234/a%2Fb%3D/",
            "http://localhost:2345/b=/",
            "http://[::1]:3456/c=/"
        ]
    );
    let many = (1..40)
        .map(|p| format!("dart dds.dart --vm-service-uri=ws://localhost:{p}/ws\n"))
        .collect::<String>();
    assert_eq!(process_uris(&many).len(), 24);
}
#[test]
fn metadata_filters_non_flutter_self_and_viewer_and_uses_vm_identity() {
    let vm = json!({"pid":20,"startTime":99,"operatingSystem":"android"});
    let iso = json!({"name":"main","rootLib":{"uri":"package:shop/main.dart"},"extensionRPCs":["ext.flutter.inspector"]});
    let t = target_from_vm(
        "ws://localhost:1/a%2F/ws",
        &vm,
        std::slice::from_ref(&iso),
        10,
        "macos",
    )
    .unwrap();
    assert_eq!(
        (t.name.as_str(), t.project_key.as_str(), t.vm_key.as_str()),
        ("shop", "package:shop/main.dart", "20:99:android")
    );
    assert!(
        target_from_vm(
            &t.uri,
            &vm,
            &[json!({"extensionRPCs":["ext.dart.io.getHttpProfile"]})],
            10,
            "macos"
        )
        .is_none()
    );
    assert!(target_from_vm(&t.uri, &vm, std::slice::from_ref(&iso), 20, "android").is_none());
    let mut viewer = iso.clone();
    viewer["rootLib"]["uri"] = json!("package:flutter_network_viewer/main.dart");
    assert!(target_from_vm(&t.uri, &vm, &[viewer], 10, "macos").is_none());
    assert!(
        target_from_vm(
            &t.uri,
            &vm,
            &vec![json!({}); 9]
                .into_iter()
                .chain([iso])
                .collect::<Vec<_>>(),
            10,
            "macos"
        )
        .is_none()
    );
}
type ScanRequest = (
    ConnectionCancel,
    mpsc::Sender<Result<Vec<Target>, RpcError>>,
);
#[derive(Clone, Default)]
struct Scans(Rc<RefCell<Vec<ScanRequest>>>);
impl DiscoveryFactory for Scans {
    fn begin(&mut self, cancel: ConnectionCancel) -> mpsc::Receiver<Result<Vec<Target>, RpcError>> {
        let (tx, rx) = mpsc::channel();
        self.0.borrow_mut().push((cancel, tx));
        rx
    }
}
#[derive(Default)]
struct Wire {
    replies: VecDeque<(u64, Result<Value, RpcError>)>,
    calls: Vec<String>,
    stopped: bool,
    fail: bool,
    count: u64,
}
#[derive(Clone)]
struct Transport(Rc<RefCell<Wire>>, u64);
impl CaptureTransport for Transport {
    fn submit(&mut self, r: Request) -> Result<(), RpcError> {
        let mut w = self.0.borrow_mut();
        w.calls.push(r.method.clone());
        let value = match r.method.as_str() {
            "getVM" => {
                json!({"pid":self.1,"startTime":100,"operatingSystem":"android","isolates":[{"id":"i","name":"main"}]})
            }
            "getIsolate" => {
                json!({"name":"main","rootLib":{"uri":"package:shop/main.dart"},"extensionRPCs":["ext.dart.io.getHttpProfile"]})
            }
            "ext.dart.io.getHttpProfile" => {
                let requests = if r.params["updatedSince"]
                    .as_str()
                    .unwrap_or("0")
                    .parse::<u64>()
                    .unwrap()
                    > w.count + 100
                {
                    vec![]
                } else {
                    vec![
                        json!({"id":format!("r{}",w.count),"method":"GET","uri":"https://example.com","request":{},"response":{"statusCode":200,"endTime":2}}),
                    ]
                };
                json!({"timestamp":w.count+100,"requests":requests})
            }
            "ext.dart.io.getHttpProfileRequest" => {
                json!({"id":r.params["id"],"method":"GET","uri":"https://example.com","request":{},"response":{"statusCode":200,"endTime":2}})
            }
            _ => json!({}),
        };
        w.replies.push_back((r.id, Ok(value)));
        Ok(())
    }
    fn take_reply(&mut self) -> Option<(u64, Result<Value, RpcError>)> {
        let mut w = self.0.borrow_mut();
        if w.fail {
            w.fail = false;
            return Some((0, Err(RpcError::Transport("closed".into()))));
        }
        w.replies.pop_front()
    }
    fn take_notification(&mut self) -> Option<Value> {
        None
    }
    fn discard_notifications(&mut self) {}
    fn stop(&mut self) {
        self.0.borrow_mut().stopped = true;
    }
}
type Attempts = Vec<(String, Rc<RefCell<Wire>>)>;
#[derive(Clone, Default)]
struct Factory(Rc<RefCell<Attempts>>);
impl ConnectionFactory for Factory {
    type Transport = Transport;
    fn begin(
        &mut self,
        uri: &str,
        _: ConnectionCancel,
    ) -> mpsc::Receiver<Result<Transport, RpcError>> {
        let (tx, rx) = mpsc::channel();
        let mut f = self.0.borrow_mut();
        let w = Rc::new(RefCell::new(Wire::default()));
        let pid = if uri.contains("1111") { 10 } else { 20 };
        f.push((uri.into(), w.clone()));
        tx.send(Ok(Transport(w, pid))).unwrap();
        rx
    }
}
type Viewer = AppDiscovery<Factory, Scans>;
fn setup() -> (Viewer, Factory, Scans) {
    let f = Factory::default();
    let d = Scans::default();
    let mut config = CoordinatorConfig {
        retry_ms: 30,
        discovery_ms: 10,
        ..Default::default()
    };
    config.poll.base_ms = 10;
    config.poll.max_ms = 10;
    config.poll.idle_increment_ms = 0;
    (AppDiscovery::new(f.clone(), d.clone(), config), f, d)
}
fn finish(d: &Scans, n: usize, targets: Vec<Target>) {
    d.0.borrow()[n].1.send(Ok(targets)).unwrap();
}
fn drive(c: &mut Viewer, now: u64) {
    for _ in 0..16 {
        c.tick(now);
    }
}
#[test]
fn empty_scan_stops_until_user_rescans_then_connects_one_and_reconnects_same_endpoint() {
    let (mut c, f, d) = setup();
    c.start_scan(0);
    assert!(c.searching() && c.busy());
    finish(&d, 0, vec![]);
    c.tick(0);
    assert!(!c.searching());
    assert!(c.last_error().is_some());
    c.tick(30_000);
    assert_eq!(d.0.borrow().len(), 1);
    c.start_scan(30);
    finish(&d, 1, vec![target("one")]);
    drive(&mut c, 30);
    assert_eq!(c.connections(), 1);
    assert!(!c.searching());
    assert!(c.targets().is_empty());
    f.0.borrow()[0].1.borrow_mut().fail = true;
    drive(&mut c, 31);
    drive(&mut c, 61);
    assert_eq!(f.0.borrow().len(), 2);
    assert_eq!(f.0.borrow()[0].0, f.0.borrow()[1].0);
    c.tick(10000);
    assert_eq!(d.0.borrow().len(), 2);
}
#[test]
fn multiple_apps_require_choice_invalid_choice_has_no_effect() {
    let (mut c, f, d) = setup();
    c.start_scan(0);
    finish(&d, 0, vec![target("one"), target("two"), target("one")]);
    c.tick(0);
    assert_eq!(c.targets().len(), 2);
    assert!(c.searching());
    assert_eq!(c.connections(), 0);
    assert!(f.0.borrow().is_empty());
    assert!(!c.select("missing", 0));
    assert!(c.select(&target("two").vm_key, 0));
    drive(&mut c, 0);
    assert_eq!(c.endpoint(), Some(target("two").uri.as_str()));
    assert_eq!(c.connections(), 1);
    assert!(!c.searching());
}
#[test]
fn each_scan_adds_one_app_preserves_live_history_and_global_budget() {
    let (mut c, f, d) = setup();
    c.start_scan(0);
    finish(&d, 0, vec![target("one")]);
    drive(&mut c, 0);
    assert_eq!(c.store().network().len(), 1);
    let session = c.store().sessions().keys().next().unwrap().clone();
    c.start_scan(1);
    finish(&d, 1, vec![target("one"), target("two")]);
    drive(&mut c, 1);
    assert_eq!(c.connections(), 2);
    assert_eq!(c.store().network().len(), 2);
    assert!(c.store().sessions().contains_key(&session));
    assert!(c.store().sessions().keys().all(|id| c.is_online(id)));
    for (_, w) in f.0.borrow().iter() {
        w.borrow_mut().count = 1;
    }
    drive(&mut c, 20);
    assert_eq!(c.store().network().len(), 4);
    assert_eq!(d.0.borrow().len(), 2);
    c.start_scan(21);
    finish(&d, 2, vec![target("one"), target("two")]);
    c.tick(21);
    assert!(!c.searching());
    c.stop_discovery();
    assert_eq!(c.connections(), 2);
    assert_eq!(c.store().network().len(), 4);
    assert!(f.0.borrow().iter().all(|(_, w)| !w.borrow().stopped));
    c.stop();
    assert_eq!(c.connections(), 0);
    assert!(c.store().sessions().is_empty());
    assert!(f.0.borrow().iter().all(|(_, w)| w.borrow().stopped));
}
#[test]
fn stop_manual_and_drop_cancel_unfinished_scan_and_ignore_late_results() {
    let (mut c, _, d) = setup();
    c.start_scan(0);
    c.stop();
    assert!(d.0.borrow()[0].0.is_cancelled());
    assert!(d.0.borrow()[0].1.send(Ok(vec![target("two")])).is_err());
    c.start_scan(1);
    c.connect_manual(&target("one").uri, 1).unwrap();
    drive(&mut c, 1);
    assert!(d.0.borrow()[1].0.is_cancelled());
    assert_eq!(c.connections(), 1);
    assert!(!c.searching());
    c.start_scan(2);
    drop(c);
    assert!(d.0.borrow()[2].0.is_cancelled());
}
#[test]
fn discovery_failure_stops_and_requires_explicit_retry() {
    let (mut c, _, d) = setup();
    c.start_scan(0);
    c.tick(100);
    assert_eq!(d.0.borrow().len(), 1);
    d.0.borrow()[0]
        .1
        .send(Err(RpcError::Transport("process failed".into())))
        .unwrap();
    c.tick(100);
    assert!(c.last_error().is_some());
    assert!(!c.searching());
    assert!(!c.busy());
    c.tick(13000);
    assert_eq!(d.0.borrow().len(), 1);
    c.start_scan(13001);
    assert_eq!(d.0.borrow().len(), 2);
}
#[test]
fn pause_clear_apply_to_all_apps_without_resurrecting_history() {
    let (mut c, f, d) = setup();
    c.start_scan(0);
    finish(&d, 0, vec![target("one")]);
    drive(&mut c, 0);
    c.start_scan(1);
    finish(&d, 1, vec![target("two")]);
    drive(&mut c, 1);
    c.set_paused(true);
    for (_, w) in f.0.borrow().iter() {
        w.borrow_mut().count = 1;
    }
    drive(&mut c, 20);
    assert_eq!(c.store().network().len(), 2);
    c.clear(21);
    c.set_paused(false);
    drive(&mut c, 40);
    assert!(c.store().network().is_empty());
}

#[test]
fn manual_app_is_excluded_from_later_scans() {
    let (mut c, f, d) = setup();
    c.connect_manual(&target("one").uri, 0).unwrap();
    drive(&mut c, 0);
    c.start_scan(1);
    finish(&d, 0, vec![target("one"), target("two")]);
    drive(&mut c, 1);
    assert_eq!(c.connections(), 2);
    assert!(!c.searching());
    assert_eq!(f.0.borrow().len(), 2);
}
#[test]
fn failed_single_project_connection_requires_user_retry() {
    let (mut c, f, d) = setup();
    c.start_scan(0);
    finish(&d, 0, vec![target("one")]);
    c.tick(0);
    f.0.borrow()[0].1.borrow_mut().fail = true;
    drive(&mut c, 1);
    assert!(!c.searching());
    assert!(!c.busy());
    assert!(c.last_error().is_some());
    c.tick(31000);
    assert_eq!(d.0.borrow().len(), 1);
    c.start_scan(31);
    finish(&d, 1, vec![target("two")]);
    drive(&mut c, 31);
    assert_eq!(c.connections(), 1);
    assert!(!c.searching());
    assert_eq!(c.endpoint(), Some(target("two").uri.as_str()));
}
#[test]
fn retained_record_budget_is_global_across_apps() {
    let f = Factory::default();
    let d = Scans::default();
    let mut config = CoordinatorConfig::default();
    config.poll.max_records = 2;
    config.poll.base_ms = 10;
    config.poll.max_ms = 10;
    config.poll.idle_increment_ms = 0;
    let mut c = AppDiscovery::new(f.clone(), d.clone(), config);
    c.start_scan(0);
    finish(&d, 0, vec![target("one")]);
    drive(&mut c, 0);
    c.start_scan(1);
    finish(&d, 1, vec![target("two")]);
    drive(&mut c, 1);
    for (_, w) in f.0.borrow().iter() {
        w.borrow_mut().count = 1;
    }
    drive(&mut c, 20);
    assert_eq!(c.store().network().len(), 2);
    assert!(
        c.store()
            .network()
            .values()
            .all(|r| r.request_event["vmRequestId"] == "r1")
    );
    assert!(c.store().sessions().keys().all(|id| c.is_online(id)));
}

#[test]
fn invalid_manual_submission_also_cancels_pending_discovery() {
    let (mut c, _, d) = setup();
    c.start_scan(0);
    assert!(c.connect_manual("not a VM URL", 1).is_err());
    assert!(!c.searching());
    assert!(d.0.borrow()[0].0.is_cancelled());
}
