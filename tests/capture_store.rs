use flutter_network_viewer::capture::CaptureStore;
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/sdk_envelope.json")).unwrap()
}
fn batch(id: &str, events: Value) -> Value {
    json!({"session":{"id":id}, "sdk":{"schemaVersion":1}, "events":events})
}
fn key(s: &str, r: &str) -> (String, String) {
    (s.into(), r.into())
}

#[test]
fn merges_phases_preserving_full_bodies_and_console_args() {
    let mut store = CaptureStore::default();
    assert_eq!(store.ingest(&fixture()).unwrap(), "ios-session-1");
    assert_eq!(
        store.sessions()["ios-session-1"].session["appName"],
        "Example Shop"
    );
    assert_eq!(store.network().len(), 3);
    let r = &store.network()[&key("ios-session-1", "fetch-1")];
    assert_eq!(r.status(), Some(201));
    assert_eq!(r.completion_event["durationMs"], 124);
    assert!(
        r.request_event["request"]["body"]
            .as_str()
            .unwrap()
            .contains("quantity")
    );
    assert!(
        r.completion_event["response"]["body"]
            .as_str()
            .unwrap()
            .contains("created")
    );
    assert!(r.curl().contains("--data-raw"));
    assert!(store.network()[&key("ios-session-1", "xhr-2")].failed());
    assert!(store.network()[&key("ios-session-1", "fetch-3")].pending());
    assert_eq!(
        store.logs()[0].event["args"][1]["value"]["customer"]["name"],
        "An"
    );
}

#[test]
fn replay_is_idempotent_for_network_but_console_remains_append_only() {
    let mut store = CaptureStore::default();
    store.ingest(&fixture()).unwrap();
    let bytes = store.retained_bytes();
    store.ingest(&fixture()).unwrap();
    assert_eq!(store.network().len(), 3);
    assert_eq!(store.logs().len(), 2);
    assert_eq!(
        store.retained_bytes() - bytes,
        serde_json::to_vec(&fixture()["events"][2]).unwrap().len()
    );
}

#[test]
fn request_ids_are_scoped_to_sessions_without_delimiter_collisions() {
    let mut store = CaptureStore::default();
    for (s, r) in [("a:b", "c"), ("a", "b:c")] {
        store
            .ingest(&batch(s, json!([{"requestId":r,"phase":"request"}])))
            .unwrap();
    }
    assert_eq!(store.network().len(), 2);
    assert_eq!(store.sessions().len(), 2);
}

#[test]
fn completion_before_request_and_abort_are_preserved() {
    let mut store = CaptureStore::default();
    let f = fixture();
    store.ingest(&batch("s", json!([f["events"][1]]))).unwrap();
    assert_eq!(store.network()[&key("s", "fetch-1")].status(), Some(201));
    store.ingest(&batch("s", json!([f["events"][0]]))).unwrap();
    assert!(!store.network()[&key("s", "fetch-1")].pending());
    store
        .ingest(&batch(
            "s",
            json!([{"requestId":"abort","phase":"abort","error":{"message":"cancelled"}}]),
        ))
        .unwrap();
    assert!(store.network()[&key("s", "abort")].failed());
}

#[test]
fn pause_discards_events_but_keeps_session_metadata() {
    let mut store = CaptureStore::default();
    store.toggle_pause();
    store.ingest(&fixture()).unwrap();
    assert!(store.network().is_empty());
    assert!(store.logs().is_empty());
    assert_eq!(store.sessions().len(), 1);
    assert_eq!(store.statistics.ignored_while_paused, 6);
    store.toggle_pause();
    store.ingest(&fixture()).unwrap();
    assert_eq!(store.network().len(), 3);
    assert_eq!(store.logs().len(), 1);
}

#[test]
fn clear_resets_capture_counters_and_bytes_but_keeps_pause_and_monotonic_log_ids() {
    let mut store = CaptureStore::default();
    store.ingest(&fixture()).unwrap();
    let first_id = store.logs()[0].id;
    store.toggle_pause();
    store.clear();
    assert!(store.paused);
    assert_eq!(store.revision, 1);
    assert!(store.sessions().is_empty());
    assert!(store.network().is_empty());
    assert!(store.logs().is_empty());
    assert_eq!(store.retained_bytes(), 0);
    assert_eq!(store.statistics.ignored_while_paused, 0);
    store.toggle_pause();
    store.ingest(&fixture()).unwrap();
    assert!(store.logs()[0].id > first_id);
    assert_eq!(store.network().len(), 3);
}

#[test]
fn shared_fifo_eviction_counts_records_rather_than_phases() {
    let mut store = CaptureStore::new(2, usize::MAX);
    store.ingest(&fixture()).unwrap();
    assert_eq!(store.network().len() + store.logs().len(), 2);
    assert_eq!(store.statistics.evicted, 2);
    assert!(store.logs().is_empty());
    assert!(store.network().contains_key(&key("ios-session-1", "xhr-2")));
}

#[test]
fn large_updates_trigger_byte_eviction_and_removed_records_can_return() {
    let mut store = CaptureStore::new(20, 500);
    store
        .ingest(&batch("s", json!([{"requestId":"r","phase":"request"}])))
        .unwrap();
    store
        .ingest(&batch(
            "s",
            json!([{"requestId":"r","phase":"response","response":{"body":"đ".repeat(600)}}]),
        ))
        .unwrap();
    assert!(store.network().is_empty());
    assert_eq!(store.retained_bytes(), 0);
    assert_eq!(store.statistics.evicted, 1);
    store
        .ingest(&batch("s", json!([{"requestId":"r","phase":"request"}])))
        .unwrap();
    assert_eq!(store.network().len(), 1);
}

#[test]
fn zero_record_and_tiny_byte_limits_retain_nothing() {
    for (count, bytes) in [(0, usize::MAX), (5000, 20)] {
        let mut store = CaptureStore::new(count, bytes);
        store.ingest(&fixture()).unwrap();
        assert!(store.network().is_empty());
        assert!(store.logs().is_empty());
        assert_eq!(store.retained_bytes(), 0);
        assert!(store.statistics.evicted > 0);
    }
}

#[test]
fn invalid_envelopes_fail_without_mutating_store() {
    let mut store = CaptureStore::default();
    for value in [
        json!([]),
        json!({"events":[]}),
        batch("", json!([])),
        json!({"session":{"id":"s"},"sdk":{"schemaVersion":2},"events":[]}),
        batch("s", json!(null)),
    ] {
        assert!(store.ingest(&value).is_err());
        assert!(store.sessions().is_empty());
        assert_eq!(store.retained_bytes(), 0);
    }
}

#[test]
fn malformed_events_do_not_poison_valid_events() {
    let mut store = CaptureStore::default();
    let mut f = fixture();
    let events = f["events"].as_array_mut().unwrap();
    events.extend([
        Value::Null,
        json!({"source":"future"}),
        json!({"requestId":"bad","phase":"response","response":{"statusCode":"bad"}}),
        json!({"requestId":"bad","phase":"response","durationMs":"bad"}),
    ]);
    store.ingest(&f).unwrap();
    assert_eq!(store.statistics.rejected, 4);
    assert_eq!(store.network().len(), 3);
}

#[test]
fn all_console_sources_preserve_original_events() {
    let mut store = CaptureStore::default();
    for source in ["js.console", "dart.stdout", "dart.stderr", "dart.logging"] {
        let event = json!({"source":source,"message":"flutter: \u{1b}[31mLỗi tiếng Việt\r\n","args":[{"type":"stackTrace","value":"frame"}]});
        store.ingest(&batch("s", json!([event]))).unwrap();
        assert_eq!(store.logs().back().unwrap().event, event);
    }
    assert_eq!(store.logs().len(), 4);
}

#[test]
fn removing_session_releases_only_its_data_and_accounting() {
    let mut store = CaptureStore::new(20, usize::MAX);
    store.ingest(&fixture()).unwrap();
    let bytes = store.retained_bytes();
    let mut other = fixture();
    other["session"]["id"] = json!("other");
    store.ingest(&other).unwrap();
    store.remove_session("other");
    assert_eq!(store.revision, 1);
    assert_eq!(store.retained_bytes(), bytes);
    assert_eq!(store.network().len(), 3);
    assert_eq!(store.logs().len(), 1);
    store.remove_session("missing");
    assert_eq!(store.revision, 1);
    store.remove_session("ios-session-1");
    assert_eq!(store.retained_bytes(), 0);
    store.ingest(&fixture()).unwrap();
    assert_eq!(store.statistics.evicted, 0);
}

#[test]
fn session_history_is_bounded_but_never_orphans_retained_data() {
    let mut store = CaptureStore::default();
    store.ingest(&fixture()).unwrap();
    for n in 0..110 {
        store.ingest(&batch(&format!("s{n}"), json!([]))).unwrap();
    }
    assert_eq!(store.sessions().len(), 100);
    assert!(store.sessions().contains_key("ios-session-1"));
    assert!(store.sessions().contains_key("s109"));
    assert!(!store.sessions().contains_key("s0"));
}

#[test]
fn export_matches_flutter_wire_schema_and_retains_order_and_raw_values() {
    let mut store = CaptureStore::default();
    store.ingest(&fixture()).unwrap();
    let export = store.export_at("2026-11-12T00:00:00.000Z");
    assert_eq!(export["format"], "flutter_network_viewer");
    assert_eq!(export["version"], 1);
    assert_eq!(export["exportedAt"], "2026-11-12T00:00:00.000Z");
    assert_eq!(export["sessions"][0]["session"], fixture()["session"]);
    assert_eq!(export["network"][0]["requestEvent"], fixture()["events"][0]);
    assert_eq!(
        export["network"][0]["completionEvent"],
        fixture()["events"][1]
    );
    assert_eq!(
        export["console"][0],
        json!({"sessionId":"ios-session-1", "event":fixture()["events"][2]})
    );
    assert_eq!(
        export["statistics"],
        json!({"rejected":0,"evicted":0,"ignoredWhilePaused":0})
    );
    assert_eq!(
        serde_json::from_str::<Value>(&export.to_string()).unwrap(),
        export
    );
    assert!(
        chrono::DateTime::parse_from_rfc3339(store.export()["exportedAt"].as_str().unwrap())
            .is_ok()
    );
}

#[test]
fn curl_escapes_quotes_and_preserves_binary_stdin() {
    let mut store = CaptureStore::default();
    store.ingest(&batch("s", json!([{"requestId":"r","phase":"request","request":{
        "method":"POST", "url":"https://example.com/a'b", "headers":{"x-test":"a'b"}, "body":"a'b"}}]))).unwrap();
    let curl = store.network()[&key("s", "r")].curl();
    assert!(curl.contains("'https://example.com/a'\\''b'"));
    assert!(curl.contains("'x-test: a'\\''b'"));
    assert!(curl.ends_with("--data-raw 'a'\\''b'"));
    store
        .ingest(&batch(
            "s",
            json!([{"requestId":"r","phase":"request","request":{
        "method":"POST", "url":"https://example.com", "body":"AAEC", "bodyEncoding":"base64"}}]),
        ))
        .unwrap();
    assert_eq!(
        store.network()[&key("s", "r")].curl(),
        "printf %s 'AAEC' | base64 --decode | curl -X POST 'https://example.com' --data-binary @-"
    );
}
