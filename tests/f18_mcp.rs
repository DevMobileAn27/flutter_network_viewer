use flutter_network_viewer::{
    capture::CaptureStore,
    mcp::{McpServer, query, tools},
    settings::AppSettings,
};
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    thread,
    time::{Duration, Instant},
};
fn fixture() -> CaptureStore {
    let mut s = CaptureStore::default();
    s.ingest(&serde_json::from_str(include_str!("fixtures/sdk_envelope.json")).unwrap())
        .unwrap();
    s
}
fn http(
    server: &mut McpServer,
    store: &CaptureStore,
    method: &str,
    path: &str,
    headers: &str,
    body: Value,
) -> (u16, Value, String) {
    let port = server.port();
    let (method, path, headers) = (method.to_owned(), path.to_owned(), headers.to_owned());
    let h = thread::spawn(move || {
        let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
        s.set_read_timeout(Some(Duration::from_secs(4))).unwrap();
        let b = body.to_string();
        write!(s,"{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\n{headers}Content-Length: {}\r\n\r\n{b}",b.len()).unwrap();
        let mut out = String::new();
        s.read_to_string(&mut out).unwrap();
        out
    });
    let start = Instant::now();
    while !h.is_finished() {
        server.poll(store, &HashSet::new());
        assert!(start.elapsed() < Duration::from_secs(5));
        thread::sleep(Duration::from_millis(2));
    }
    let out = h.join().unwrap();
    let (head, body) = out.split_once("\r\n\r\n").unwrap();
    (
        head.split_whitespace().nth(1).unwrap().parse().unwrap(),
        serde_json::from_str(body).unwrap_or(Value::Null),
        head.into(),
    )
}
#[test]
fn queries_match_flutter_and_are_read_only() {
    let mut s = fixture();
    let before = s.export_at("fixed");
    let on = HashSet::from(["ios-session-1".into()]);
    assert_eq!(tools()["tools"].as_array().unwrap().len(), 5);
    for t in tools()["tools"].as_array().unwrap() {
        assert_eq!(t["annotations"]["readOnlyHint"], true);
        assert_eq!(t["annotations"]["destructiveHint"], false);
    }
    let q = |n, a| query(&s, &on, n, &a).unwrap();
    assert_eq!(q("list_sessions", json!({}))["items"][0]["online"], true);
    let p = q("list_requests", json!({"limit":1}));
    assert_eq!(p["total"], 3);
    assert_eq!(p["items"][0]["request_id"], "fetch-3");
    assert_eq!(p["next_offset"], 1);
    assert_eq!(p["items"][0]["state"], "Pending");
    assert_eq!(
        q("list_requests", json!({"method":"POST"}))["items"][0]["response_bytes"],
        28
    );
    assert_eq!(
        q(
            "list_requests",
            json!({"method":"post","url":"/ORDERS","status_code":201,"status_class":2,"min_duration_ms":100,"since":"2026-10-14T08:00:00Z","until":"2026-10-14T08:00:01Z"})
        )["total"],
        1
    );
    for (key, total) in [("failed", 1), ("pending", 1)] {
        assert_eq!(q("list_requests", json!({key:true}))["total"], total);
    }
    let d = q(
        "get_request",
        json!({"session_id":"ios-session-1","request_id":"fetch-1","body_offset":5,"body_limit":10}),
    );
    assert_eq!(d["request"]["headers"]["authorization"], "[REDACTED]");
    assert_eq!(d["request"]["body"]["next_offset"], 15);
    assert!(
        q(
            "get_request",
            json!({"session_id":"ios-session-1","request_id":"fetch-1","include_body":false})
        )["response"]
            .get("body")
            .is_none()
    );
    assert_eq!(
        q(
            "list_logs",
            json!({"level":"info","query":"COFFEE","include_arguments":true,"text_limit":8})
        )["items"][0]["message"]["text"],
        "Order cr"
    );
    let summary = q("get_capture_summary", json!({}));
    assert_eq!(summary["request_count"], 3);
    assert_eq!(summary["average_duration_ms"], 103.0);
    assert_eq!(s.export_at("fixed"), before);
    s.clear();
    assert_eq!(
        query(&s, &on, "list_requests", &json!({})).unwrap()["total"],
        0
    );
}
#[test]
fn validation_and_utf16_chunks() {
    let mut s = fixture();
    let on = HashSet::new();
    for (n, a) in [
        ("list_requests", json!({"limit":0})),
        ("list_logs", json!({"text_limit":16001})),
        ("get_request", json!({})),
        ("list_requests", json!({"limit":1.5})),
        ("list_requests", json!({"since":"2026-01-01"})),
        (
            "get_capture_summary",
            json!({"since":"2026-11-02T00:00:00Z","until":"2026-11-01T00:00:00Z"}),
        ),
        ("list_requests", json!({"clear":true})),
        ("clear", json!({})),
        ("list_requests", json!({"pending":"yes"})),
    ] {
        assert!(query(&s, &on, n, &a).is_err(), "{n} {a}");
    }
    s.ingest(&json!({"sdk":{"schemaVersion":1},"session":{"id":"u"},"events":[{"requestId":"x","phase":"request","request":{"body":"a😀b"}}]})).unwrap();
    let d = query(
        &s,
        &on,
        "get_request",
        &json!({"session_id":"u","request_id":"x","body_offset":1,"body_limit":2}),
    )
    .unwrap();
    assert_eq!(d["request"]["body"]["text"], "😀");
    assert_eq!(d["request"]["body"]["total_length"], 4);
}
#[test]
fn http_protocol_security_and_live_store() {
    let mut m = McpServer::default();
    assert!(!m.running());
    assert!(!AppSettings::default().values().mcp_enabled);
    m.configure(true, 0, false);
    assert!(m.running());
    let mut s = fixture();
    for version in ["2024-11-05", "2025-03-26", "2025-06-18", "2025-11-25"] {
        let (code, b, h) = http(
            &mut m,
            &s,
            "POST",
            "/mcp",
            "",
            json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":version,"capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
        );
        assert_eq!(code, 200);
        assert_eq!(b["result"]["protocolVersion"], version);
        assert!(h.to_lowercase().contains("mcp-session-id"));
    }
    for (method, path, headers, expected) in [
        ("POST", "/mcp", "Origin: https://evil.example\r\n", 403),
        ("POST", "/mcp", "Origin: null\r\n", 403),
        ("POST", "/mcp", "Host: evil.example\r\n", 403),
        ("POST", "/unknown", "", 404),
        ("POST", "/mcp", "MCP-Protocol-Version: 2099-01-01\r\n", 400),
        ("GET", "/mcp", "", 405),
    ] {
        assert_eq!(
            http(
                &mut m,
                &s,
                method,
                path,
                headers,
                json!({"jsonrpc":"2.0","id":1,"method":"ping"})
            )
            .0,
            expected
        );
    }
    let rpc = |name, args| json!({"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":name,"arguments":args}});
    assert_eq!(
        http(
            &mut m,
            &s,
            "POST",
            "/mcp",
            "",
            rpc("get_capture_summary", json!({}))
        )
        .1["result"]["structuredContent"]["request_count"],
        3
    );
    s.clear();
    assert_eq!(
        http(
            &mut m,
            &s,
            "POST",
            "/mcp",
            "",
            rpc("list_requests", json!({}))
        )
        .1["result"]["structuredContent"]["total"],
        0
    );
    assert_eq!(
        http(&mut m, &s, "POST", "/mcp", "", rpc("clear", json!({}))).1["result"]["isError"],
        true
    );
    assert_eq!(
        http(
            &mut m,
            &s,
            "POST",
            "/mcp",
            "",
            json!({"jsonrpc":"2.0","method":"notifications/initialized"})
        )
        .0,
        202
    );
    assert_eq!(
        http(
            &mut m,
            &s,
            "POST",
            "/mcp",
            "",
            json!({"jsonrpc":"2.0","id":3,"method":"destroy"})
        )
        .1["error"]["code"],
        -32601
    );
}
#[test]
fn conflicts_retry_reconfiguration_and_drop_release_port() {
    let occupied = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let p = occupied.local_addr().unwrap().port();
    let mut m = McpServer::default();
    m.configure(true, p, false);
    assert!(!m.running());
    assert!(m.status().error.is_some());
    drop(occupied);
    m.configure(true, p, true);
    assert!(m.running());
    m.configure(false, p, false);
    m.configure(true, p, false);
    m.configure(false, p, false);
    assert!(!m.running());
    let released = TcpListener::bind(("127.0.0.1", p)).unwrap();
    drop(released);
    m.configure(true, p, false);
    drop(m);
    assert!(TcpListener::bind(("127.0.0.1", p)).is_ok());
}
#[test]
fn http_all_five_tools_session_isolation_and_invalid_arguments() {
    let mut m = McpServer::default();
    m.configure(true, 0, false);
    let mut s = fixture();
    let mut second: Value =
        serde_json::from_str(include_str!("fixtures/sdk_envelope.json")).unwrap();
    second["session"]["id"] = json!("second");
    s.ingest(&second).unwrap();
    let call = |name: &str, args: Value| json!({"jsonrpc":"2.0","id":"client-id","method":"tools/call","params":{"name":name,"arguments":args}});
    let (_, init, head) = http(
        &mut m,
        &s,
        "POST",
        "/mcp",
        "",
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2099-01-01"}}),
    );
    assert_eq!(init["result"]["protocolVersion"], "2025-11-25");
    let session = head
        .lines()
        .find_map(|l| l.strip_prefix("Mcp-Session-Id: "))
        .unwrap()
        .trim();
    let headers = format!("Mcp-Session-Id: {session}\r\nMCP-Protocol-Version: 2025-11-25\r\n");
    let list = http(
        &mut m,
        &s,
        "POST",
        "/mcp",
        &headers,
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
    );
    assert_eq!(list.1["result"]["tools"].as_array().unwrap().len(), 5);
    for (name, args) in [
        ("list_sessions", json!({"offset":1,"limit":1})),
        (
            "list_requests",
            json!({"session_id":"second","offset":1,"limit":1}),
        ),
        (
            "get_request",
            json!({"session_id":"second","request_id":"fetch-1"}),
        ),
        (
            "list_logs",
            json!({"session_id":"second","include_arguments":true,"query":"Coffee"}),
        ),
        ("get_capture_summary", json!({"session_id":"second"})),
    ] {
        let (status, result, _) = http(&mut m, &s, "POST", "/mcp", &headers, call(name, args));
        assert_eq!(status, 200);
        assert_eq!(result["id"], "client-id");
        let r = &result["result"];
        assert_eq!(r["isError"], false, "{name}: {r}");
        let text: Value = serde_json::from_str(r["content"][0]["text"].as_str().unwrap()).unwrap();
        assert_eq!(text, r["structuredContent"]);
        match name {
            "list_sessions" => assert_eq!(text["items"][0]["session_id"], "second"),
            "list_requests" => {
                assert_eq!(text["items"][0]["request_id"], "xhr-2");
                assert_eq!(text["items"][0]["state"], "Error");
            }
            "get_request" => assert_eq!(text["session_id"], "second"),
            "list_logs" => assert!(
                text["items"][0]["arguments"]["text"]
                    .as_str()
                    .unwrap()
                    .contains("Coffee")
            ),
            _ => assert_eq!(text["request_count"], 3),
        }
    }
    for args in [
        json!({"session_id":"missing","request_id":"fetch-1"}),
        json!({"session_id":"second","request_id":"fetch-1","body_limit":64001}),
    ] {
        assert_eq!(
            http(
                &mut m,
                &s,
                "POST",
                "/mcp",
                &headers,
                call("get_request", args)
            )
            .1["result"]["isError"],
            true
        );
    }
    assert_eq!(
        http(&mut m, &s, "DELETE", "/mcp", &headers, Value::Null).0,
        200
    );
    assert_eq!(
        http(
            &mut m,
            &s,
            "POST",
            "/mcp",
            &headers,
            call("list_sessions", json!({}))
        )
        .0,
        404
    );
}
#[test]
fn fragmented_client_and_pending_socket_stop_do_not_block_owner() {
    let mut m = McpServer::default();
    m.configure(true, 0, false);
    let port = m.port();
    let s = fixture();
    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    stream.write_all(b"POST /mcp HTTP/1.1\r\nHost: localhost\r\nContent-Type: application/json\r\nContent-Length: 40\r\n\r\n").unwrap();
    let start = Instant::now();
    for _ in 0..20 {
        m.poll(&s, &HashSet::new());
    }
    assert!(start.elapsed() < Duration::from_millis(500));
    stream
        .write_all(br#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#)
        .unwrap();
    // Deliberately wrong length remains pending; stop must close that socket too.
    m.poll(&s, &HashSet::new());
    m.configure(false, port, false);
    let mut byte = [0];
    match stream.read(&mut byte) {
        Ok(0) => {}
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::ConnectionReset | std::io::ErrorKind::ConnectionAborted
            ) => {}
        other => panic!("socket not closed: {other:?}"),
    }
    assert!(TcpListener::bind(("127.0.0.1", port)).is_ok());
    for _ in 0..20 {
        m.configure(true, 0, true);
        assert!(m.running());
        let p = m.port();
        m.configure(false, p, false);
        assert!(TcpListener::bind(("127.0.0.1", p)).is_ok());
    }
}
#[test]
fn setup_configs_and_formatted_payload_metadata_match_flutter() {
    for port in [1, 38943, 65535] {
        let url = format!("http://127.0.0.1:{port}/mcp");
        let codex = flutter_network_viewer::mcp::setup(port, false);
        assert!(codex.contains(&format!("url = \"{url}\"")));
        let claude = flutter_network_viewer::mcp::setup(port, true);
        assert!(claude.contains(&format!("\"url\": \"{url}\"")));
        assert!(claude.contains("mcp-remote"));
        assert!(claude.contains("requires Node.js"));
    }
    let s = fixture();
    let on = HashSet::new();
    let d = query(
        &s,
        &on,
        "get_request",
        &json!({"session_id":"ios-session-1","request_id":"fetch-1"}),
    )
    .unwrap();
    assert_eq!(
        d["request"]["body"]["text"],
        "{\n  \"product\": \"Coffee\",\n  \"quantity\": 2\n}"
    );
    assert_eq!(d["error"], json!({}));
    let mut s = CaptureStore::default();
    s.ingest(&json!({"sdk":{"schemaVersion":1},"session":{"id":"minimal"},"events":[{"phase":"request","requestId":"x","timestamp":"2026-10-14T10:00:00+02:00","request":{"url":"http://test"}}]})).unwrap();
    let sessions = query(&s, &on, "list_sessions", &json!({})).unwrap();
    assert_eq!(sessions["items"][0]["app_name"], "Flutter");
    assert_eq!(sessions["items"][0]["device"], "Unknown device");
    let requests = query(&s, &on, "list_requests", &json!({})).unwrap();
    assert_eq!(
        requests["items"][0]["timestamp"],
        "2026-10-14T08:00:00.000Z"
    );
    assert_eq!(requests["items"][0]["source"], "");
}
