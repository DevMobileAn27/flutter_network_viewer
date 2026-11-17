use flutter_network_viewer::{
    lifecycle::{CaptureEngine, CaptureTransport, PollConfig, Request},
    vm::RpcError,
};
use serde_json::{Value, json};
use std::collections::VecDeque;

#[derive(Default)]
struct FakeTransport {
    calls: Vec<Request>,
    replies: VecDeque<(u64, Result<Value, RpcError>)>,
    events: VecDeque<Value>,
    stopped: bool,
}
impl CaptureTransport for FakeTransport {
    fn submit(&mut self, request: Request) -> Result<(), RpcError> {
        self.calls.push(request);
        Ok(())
    }
    fn take_reply(&mut self) -> Option<(u64, Result<Value, RpcError>)> {
        self.replies.pop_front()
    }
    fn take_notification(&mut self) -> Option<Value> {
        self.events.pop_front()
    }
    fn discard_notifications(&mut self) {
        self.events.clear();
    }
    fn stop(&mut self) {
        self.stopped = true;
    }
}
type Engine = CaptureEngine<FakeTransport>;
fn engine(config: PollConfig) -> Engine {
    let mut e = CaptureEngine::new(
        FakeTransport::default(),
        json!({"id":"app"}),
        "isolates/1",
        "main",
        config,
    );
    e.start(0);
    e
}
fn reply(e: &mut Engine, result: Result<Value, RpcError>, now: u64) {
    let id = e.transport_mut().calls.last().unwrap().id;
    e.transport_mut().replies.push_back((id, result));
    e.tick(now);
}
fn summary(id: &str, complete: bool) -> Value {
    json!({"id":id,"method":"GET","uri":"https://example.com","startTime":1,"request":{},"response":if complete {json!({"endTime":5,"statusCode":200})} else {json!({})}})
}
fn profile(e: &mut Engine, p: Vec<Value>, stamp: u64, now: u64) {
    reply(e, Ok(json!({"timestamp":stamp,"requests":p})), now);
}
fn log(e: &mut Engine, s: &str) {
    use base64::Engine;
    e.transport_mut().events.push_back(json!({"method":"streamNotify","params":{"streamId":"Stdout","event":{"isolate":{"id":"isolates/1"},"timestamp":1000,"bytes":base64::engine::general_purpose::STANDARD.encode(s)}}}));
}
#[test]
fn defaults_backoff_cap_and_no_overlap_with_slow_poll() {
    let mut e = engine(PollConfig::default());
    assert_eq!((e.network_interval(), e.console_interval()), (1400, 1400));
    e.tick(0);
    assert_eq!(e.transport_mut().calls.len(), 1);
    e.tick(100_000);
    assert_eq!(e.transport_mut().calls.len(), 1);
    profile(&mut e, vec![], 20, 100_000);
    assert_eq!(e.network_interval(), 1600);
    e.tick(101_599);
    assert_eq!(e.transport_mut().calls.len(), 1);
    let mut now = 101_600;
    for _ in 0..25 {
        e.tick(now);
        profile(&mut e, vec![], 20, now);
        now += e.network_interval();
    }
    assert_eq!(e.network_interval(), 5000);
}
#[test]
fn unchanged_summary_skips_detail_and_completion_resets_only_network() {
    let mut e = engine(PollConfig::default());
    e.tick(0);
    let a = summary("a", false);
    profile(&mut e, vec![a.clone()], 20, 0);
    reply(&mut e, Ok(a.clone()), 0);
    assert!(e.store().network().values().next().unwrap().pending());
    e.tick(1400);
    profile(&mut e, vec![a], 20, 1400);
    assert_eq!(e.network_interval(), 1600);
    assert_eq!(e.transport_mut().calls.len(), 3);
    e.tick(3000);
    let b = summary("a", true);
    profile(&mut e, vec![b.clone()], 21, 3000);
    reply(&mut e, Ok(b), 3000);
    assert_eq!(e.store().network().len(), 1);
    assert!(!e.store().network().values().next().unwrap().pending());
    assert_eq!(e.network_interval(), 1400);
    assert!(e.console_interval() > 1400);
}
#[test]
fn console_batches_in_order_during_hung_network_and_resets_independently() {
    let mut e = engine(PollConfig::default());
    e.tick(0);
    e.tick(1400);
    log(&mut e, "first\nsecond\n");
    log(&mut e, "third\n");
    e.tick(1500);
    assert!(e.store().logs().is_empty());
    e.tick(3000);
    assert_eq!(
        e.store()
            .logs()
            .iter()
            .map(|r| r.event["message"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["first", "second", "third"]
    );
    assert_eq!((e.network_interval(), e.console_interval()), (1400, 1400));
    assert_eq!(e.console_batches(), 1);
    assert_eq!(e.transport_mut().calls.len(), 1);
}
#[test]
fn stop_cancels_both_deadlines_and_discards_late_detail() {
    let mut e = engine(PollConfig::default());
    e.tick(0);
    profile(&mut e, vec![summary("a", true)], 10, 0);
    let id = e.transport_mut().calls.last().unwrap().id;
    log(&mut e, "pending\n");
    e.tick(1);
    e.stop();
    assert!(e.transport_mut().stopped);
    e.transport_mut()
        .replies
        .push_back((id, Ok(summary("a", true))));
    e.tick(100_000);
    assert!(e.store().network().is_empty());
    assert!(e.store().logs().is_empty());
    assert_eq!(e.transport_mut().calls.len(), 2);
    assert!(!e.running());
}
#[test]
fn clear_drops_pending_partial_and_inflight_detail_and_excludes_inclusive_cursor() {
    let mut e = engine(PollConfig::default());
    e.tick(0);
    profile(&mut e, vec![summary("a", true)], 20, 0);
    log(&mut e, "partial");
    e.tick(1);
    log(&mut e, "queued\n");
    e.clear(2);
    reply(&mut e, Ok(summary("a", true)), 3);
    e.tick(5000);
    assert_eq!(
        e.transport_mut().calls.last().unwrap().params["updatedSince"],
        "21"
    );
    profile(&mut e, vec![], 21, 5000);
    log(&mut e, "fresh\n");
    e.tick(10_000);
    assert!(e.store().network().is_empty());
    assert_eq!(e.store().logs().len(), 1);
    assert_eq!(e.store().logs()[0].event["message"], "fresh");
}
#[test]
fn clear_while_summary_pending_advances_returned_timestamp_without_detail() {
    let mut e = engine(PollConfig::default());
    e.tick(0);
    e.clear(1);
    profile(&mut e, vec![summary("old", true)], 100, 2);
    assert_eq!(e.transport_mut().calls.len(), 1);
    e.tick(5000);
    assert_eq!(
        e.transport_mut().calls.last().unwrap().params["updatedSince"],
        "101"
    );
    assert!(e.store().sessions().is_empty());
}
#[test]
fn pause_discards_queued_chunks_and_advances_network_without_resume_replay() {
    let mut e = engine(PollConfig::default());
    e.tick(0);
    log(&mut e, "before\n");
    e.tick(1);
    e.set_paused(true);
    log(&mut e, "paused\n");
    profile(&mut e, vec![summary("a", true)], 20, 2);
    assert_eq!(e.transport_mut().calls.len(), 1);
    assert_eq!(e.store().statistics.ignored_while_paused, 2);
    e.set_paused(false);
    e.tick(1402);
    profile(&mut e, vec![summary("a", true)], 20, 1402);
    assert_eq!(e.transport_mut().calls.len(), 2);
    assert!(e.store().network().is_empty());
    log(&mut e, "after\n");
    e.tick(5000);
    assert_eq!(e.store().logs().len(), 1);
    assert_eq!(e.store().logs()[0].event["message"], "after");
}
#[test]
fn pending_queue_count_and_utf8_byte_budget_are_bounded() {
    let mut e = engine(PollConfig {
        max_records: 10,
        max_bytes: 10_000,
        ..PollConfig::default()
    });
    for i in 0..25 {
        log(&mut e, &format!("line {i}\n"));
    }
    e.tick(1);
    assert_eq!(e.pending_console_len(), 10);
    assert_eq!(e.store().statistics.evicted, 15);
    e.tick(1400);
    assert_eq!(e.store().logs()[0].event["message"], "line 15");
    let mut e = engine(PollConfig {
        max_records: 100,
        max_bytes: 1,
        ..PollConfig::default()
    });
    log(&mut e, "🦀\n");
    e.tick(1);
    assert_eq!(e.pending_console_len(), 0);
    assert_eq!(e.pending_console_bytes(), 0);
    assert_eq!(e.store().statistics.evicted, 1);
}
#[test]
fn rpc_timeout_omits_detail_and_does_not_disconnect_or_stop_console() {
    let mut e = engine(PollConfig::default());
    e.tick(0);
    profile(&mut e, vec![summary("a", true)], 20, 0);
    reply(&mut e, Err(RpcError::Timeout), 1);
    assert!(e.running());
    assert_eq!(e.store().network().len(), 1);
    let r = e.store().network().values().next().unwrap();
    assert_eq!(
        r.request_event["request"]["bodyOmitted"],
        "profile-read-error"
    );
    e.tick(1401);
    reply(&mut e, Err(RpcError::Timeout), 1401);
    assert!(e.running());
    log(&mut e, "alive\n");
    e.tick(5000);
    assert_eq!(e.store().logs().len(), 1);
}
#[test]
fn inline_logging_and_stdout_preserve_batch_order() {
    let mut e = engine(PollConfig::default());
    log(&mut e, "hel");
    log(&mut e, "lo\n");
    e.transport_mut().events.push_back(json!({"method":"streamNotify","params":{"streamId":"Logging","event":{"isolate":{"id":"isolates/1"},"logRecord":{"message":{"valueAsString":"problem"},"loggerName":{"valueAsString":"test"},"level":1000}}}}));
    e.tick(1400);
    assert_eq!(e.store().logs().len(), 2);
    assert_eq!(e.store().logs()[0].event["message"], "hello");
    assert_eq!(e.store().logs()[1].event["level"], "error");
}

#[test]
fn pause_resume_invalidates_summary_already_in_flight() {
    let mut e = engine(PollConfig::default());
    e.tick(0);
    e.set_paused(true);
    e.set_paused(false);
    profile(&mut e, vec![summary("paused", true)], 20, 1);
    assert_eq!(e.transport_mut().calls.len(), 1);
    assert!(e.store().network().is_empty());
    e.tick(5000);
    profile(&mut e, vec![summary("paused", true)], 20, 5000);
    assert_eq!(e.transport_mut().calls.len(), 2);
}
#[test]
fn stop_and_disconnect_remove_retained_session_and_release_pending_state() {
    for disconnected in [false, true] {
        let mut e = engine(PollConfig::default());
        e.tick(0);
        let p = summary("a", true);
        profile(&mut e, vec![p.clone()], 20, 0);
        reply(&mut e, Ok(p), 0);
        log(&mut e, "retained\n");
        e.tick(1400);
        assert!(!e.store().sessions().is_empty());
        if disconnected {
            e.transport_mut()
                .replies
                .push_back((0, Err(RpcError::Transport("closed".into()))));
            e.tick(1401);
        } else {
            e.stop();
        }
        assert!(!e.running());
        assert!(e.store().sessions().is_empty());
        assert!(e.store().network().is_empty());
        assert!(e.store().logs().is_empty());
        assert_eq!(e.cursor(), None);
        e.start(5000);
        e.tick(5000);
        assert!(!e.running(), "a stopped transport cannot be reused");
    }
}
#[test]
fn clear_then_new_request_at_later_cursor_resets_network_only() {
    let mut e = engine(PollConfig::default());
    e.tick(0);
    profile(&mut e, vec![], 20, 0);
    e.clear(1);
    e.tick(5000);
    let p = summary("new", true);
    profile(&mut e, vec![p.clone()], 21, 5000);
    reply(&mut e, Ok(p), 5000);
    assert_eq!(e.network_interval(), 1400);
    assert!(e.console_interval() > 1400);
    assert_eq!(e.store().network().len(), 1);
}
#[test]
fn stdout_utf8_split_across_chunks_and_stderr_keep_order() {
    use base64::Engine;
    let mut e = engine(PollConfig::default());
    for (stream, bytes) in [
        ("Stdout", vec![0xf0, 0x9f]),
        ("Stdout", vec![0xa6, 0x80, 10]),
        ("Stderr", b"error\n".to_vec()),
    ] {
        e.transport_mut().events.push_back(json!({"method":"streamNotify","params":{"streamId":stream,"event":{"isolate":{"id":"isolates/1"},"bytes":base64::engine::general_purpose::STANDARD.encode(bytes)}}}));
    }
    e.tick(1400);
    assert_eq!(e.store().logs()[0].event["message"], "🦀");
    assert_eq!(e.store().logs()[1].event["source"], "dart.stderr");
    assert_eq!(e.store().logs()[1].event["level"], "error");
}

#[test]
fn bounded_profile_work_keeps_latest_summaries_in_order() {
    let mut e = engine(PollConfig {
        max_records: 2,
        ..PollConfig::default()
    });
    e.tick(0);
    profile(
        &mut e,
        vec![summary("a", true), summary("b", true), summary("c", true)],
        20,
        0,
    );
    assert_eq!(e.transport_mut().calls.last().unwrap().params["id"], "b");
    reply(&mut e, Ok(summary("b", true)), 1);
    assert_eq!(e.transport_mut().calls.last().unwrap().params["id"], "c");
    reply(&mut e, Ok(summary("c", true)), 2);
    assert_eq!(e.store().network().len(), 2);
    assert_eq!(e.store().statistics.evicted, 1);
}
