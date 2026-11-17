use flutter_network_viewer::vm::{
    Incoming, MAX_BODY_BYTES, RpcCore, RpcError, VmClient, body, declared_body_length,
    exceeds_body_limit, network, normalize_uri, omit_bodies, scoped_events,
};
use serde_json::{Value, json};
use std::{net::TcpListener, thread, time::Duration};

fn profile() -> Value {
    json!({"id":"r1","method":"POST","uri":"https://example.com/orders","startTime":1000000,"endTime":1010000,"request":{"headers":{"x-tags":["one","two"]}},"requestBody":[123,125],"response":{"endTime":1124000,"statusCode":201,"reasonPhrase":"Created","headers":{"content-type":["application/json"]}},"responseBody":"Cà phê 🦀","events":[{"name":"send"}]})
}

#[test]
fn normalize_service_and_devtools() {
    let ws = "ws://127.0.0.1:57037/SfP6AjEheVE=/ws";
    for input in [
        ws.to_string(),
        " http://127.0.0.1:57037/SfP6AjEheVE=/ ".into(),
        format!("http://localhost/devtools/?uri={ws}"),
        "http://localhost:9100/?uri=ws%3A%2F%2F127.0.0.1%3A57037%2FSfP6AjEheVE%3D%2Fws".into(),
    ] {
        assert_eq!(normalize_uri(&input).unwrap(), ws);
    }
    assert_eq!(
        normalize_uri("https://example.com/token=/").unwrap(),
        "wss://example.com/token=/ws"
    );
    assert_eq!(
        normalize_uri("http://[::1]:1234/a%2Fb=/ws?foo=bar").unwrap(),
        "ws://[::1]:1234/a%2Fb=/ws"
    );
}
#[test]
fn invalid_endpoints() {
    for input in [
        "",
        "70000",
        "file:///tmp/test",
        "http:///missing",
        "http://localhost/devtools/",
        "ws://user:pass@localhost/ws",
        "ws://localhost/ws#fragment",
        "ws://localhost:99999/",
    ] {
        assert!(normalize_uri(input).is_err(), "{input}");
    }
}
#[test]
fn rpc_correlates_out_of_order_and_ignores_unknown() {
    let mut rpc = RpcCore::default();
    let a = rpc.request("getVM", json!({}), 0, 100).unwrap();
    let b = rpc
        .request("getIsolate", json!({"isolateId":"i"}), 0, 100)
        .unwrap();
    assert_eq!(a["jsonrpc"], "2.0");
    assert_eq!(rpc.pending_len(), 2);
    assert_eq!(
        rpc.receive(&json!({"jsonrpc":"2.0","id":b["id"],"result":{"type":"Isolate"}}).to_string())
            .unwrap(),
        Incoming::Response {
            id: b["id"].as_u64().unwrap(),
            result: Ok(json!({"type":"Isolate"}))
        }
    );
    assert_eq!(
        rpc.receive(r#"{"jsonrpc":"2.0","id":999,"result":{}}"#)
            .unwrap(),
        Incoming::Ignored
    );
    assert_eq!(rpc.pending_len(), 1);
    assert_ne!(a["id"], b["id"]);
}
#[test]
fn rpc_error_notification_and_invalid_message() {
    let mut rpc = RpcCore::default();
    let a = rpc.request("bad", json!({}), 0, 100).unwrap();
    let incoming=rpc.receive(&json!({"jsonrpc":"2.0","id":a["id"],"error":{"code":-32000,"message":"gone","data":{"why":1}}}).to_string()).unwrap();
    assert!(matches!(
        incoming,
        Incoming::Response {
            result: Err(RpcError::Remote { code: -32000, .. }),
            ..
        }
    ));
    assert!(matches!(
        rpc.receive(r#"{"jsonrpc":"2.0","method":"streamNotify","params":{"streamId":"Stdout"}}"#)
            .unwrap(),
        Incoming::Notification { .. }
    ));
    for raw in ["garbage", "[]", r#"{"jsonrpc":"1.0","id":1,"result":{}}"#] {
        assert!(rpc.receive(raw).is_err());
    }
}
#[test]
fn rpc_expiry_is_inclusive_and_late_reply_ignored() {
    let mut rpc = RpcCore::default();
    let a = rpc.request("getVM", json!({}), 20, 80).unwrap();
    assert!(rpc.expire(99).is_empty());
    assert_eq!(rpc.expire(100), vec![a["id"].as_u64().unwrap()]);
    assert_eq!(rpc.pending_len(), 0);
    assert_eq!(
        rpc.receive(&json!({"jsonrpc":"2.0","id":a["id"],"result":{}}).to_string())
            .unwrap(),
        Incoming::Ignored
    );
    assert!(rpc.request("", json!({}), 0, 1).is_err());
    assert!(rpc.request("getVM", json!([]), 0, 1).is_err());
}
#[test]
fn malformed_response_does_not_consume_pending() {
    let mut rpc = RpcCore::default();
    let a = rpc.request("getVM", json!({}), 0, 100).unwrap();
    assert!(
        rpc.receive(&json!({"jsonrpc":"2.0","id":a["id"]}).to_string())
            .is_err()
    );
    assert_eq!(rpc.pending_len(), 1);
}
#[test]
fn profile_timing_headers_and_unicode() {
    let mut p = profile();
    p["responseBody"] = json!("Cà phê 🦀".as_bytes());
    let e = network(&p);
    assert_eq!(e.len(), 2);
    assert_eq!(e[0]["request"]["headers"]["x-tags"], "one, two");
    assert_eq!(e[1]["response"]["body"], "Cà phê 🦀");
    assert_eq!(e[1]["response"]["bodyEncoding"], "utf-8");
    assert_eq!(e[1]["durationMs"], 124.0);
    assert_eq!(e[0]["timestamp"], "1970-01-01T00:00:01.000Z");
    assert_eq!(e[1]["response"]["statusText"], "Created");
    assert_eq!(e[0]["profileEvents"], p["events"]);
}
#[test]
fn pending_and_network_error() {
    let mut p = profile();
    p["response"].as_object_mut().unwrap().remove("endTime");
    assert_eq!(network(&p).len(), 1);
    p["request"]["error"] = json!("Connection refused");
    let e = network(&p);
    assert_eq!(e[1]["phase"], "error");
    assert_eq!(e[1]["durationMs"], 10.0);
    assert_eq!(e[1]["error"]["message"], "Connection refused");
}
#[test]
fn body_unicode_binary_and_malformed() {
    assert_eq!(body(&json!([255, 254]))["body"], "//4=");
    assert_eq!(body(&json!([255, 254]))["bodyEncoding"], "base64");
    assert_eq!(body(&json!([]))["bodyByteLength"], 0);
    for raw in [json!([0, "bad"]), json!([256]), json!([-1]), json!([1.5])] {
        assert_eq!(body(&raw)["bodyOmitted"], "invalid-body-bytes");
    }
    assert_eq!(body(&json!("original")), json!({"body":"original"}));
    assert_eq!(body(&Value::Null), json!({}));
    assert_eq!(body(&json!({})), json!({}));
}
#[test]
fn metadata_combined_guard_and_boundary() {
    assert_eq!(
        declared_body_length(
            &json!({"contentLength":-1,"headers":{"Content-Length":["59774016"]}})
        ),
        Some(59774016)
    );
    assert_eq!(
        declared_body_length(&json!({"headers":{"content-length":"invalid"}})),
        None
    );
    assert!(!exceeds_body_limit(
        &json!({"request":{"contentLength":MAX_BODY_BYTES}})
    ));
    assert!(exceeds_body_limit(
        &json!({"request":{"contentLength":MAX_BODY_BYTES-1},"response":{"contentLength":2}})
    ));
    assert!(exceeds_body_limit(
        &json!({"request":{"contentLength":u64::MAX},"response":{"contentLength":1}})
    ));
}
#[test]
fn oversized_actual_body_and_omission_preserve_metadata() {
    let raw = json!(vec![65; MAX_BODY_BYTES + 1]);
    assert_eq!(body(&raw)["bodyOmitted"], "profile-size-limit");
    assert!(body(&raw).get("body").is_none());
    let mut p = profile();
    p["request"]["contentLength"] = json!(42);
    let omitted = omit_bodies(&p, "profile-read-error");
    assert!(omitted.get("requestBody").is_none());
    assert_eq!(omitted["request"]["bodyByteLength"], 42);
    assert_eq!(network(&omitted)[1]["response"]["statusCode"], 201);
    assert_eq!(p["requestBody"], json!([123, 125]));
}
#[test]
fn scoped_ids_are_unambiguous_and_store_compatible() {
    let a = scoped_events(&profile(), "isolates/1", "main");
    let b = scoped_events(&profile(), "isolates/2", "worker");
    assert_ne!(a[0]["requestId"], b[0]["requestId"]);
    assert_eq!(a[0]["vmRequestId"], "r1");
    assert_eq!(a[0]["requestId"], r#"["isolates/1","r1"]"#);
    assert_eq!(a[0]["isolateName"], "main");
    let mut store = flutter_network_viewer::capture::CaptureStore::default();
    store.ingest(&json!({"session":{"id":"vm"},"sdk":{"name":"dart_vm_service","schemaVersion":1},"events":([a,b].concat())})).unwrap();
    assert_eq!(store.network().len(), 2);
}

// A real WebSocket peer, independent of the client; no timer/poll lifecycle here.
// tungstenite requires this concrete HTTP error type for its handshake callback.
#[allow(clippy::result_large_err)]
fn fake_vm(
    f: impl FnOnce(tungstenite::WebSocket<std::net::TcpStream>) + Send + 'static,
) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let uri = format!("ws://{}/token=/ws", listener.local_addr().unwrap());
    let handle = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        let socket = tungstenite::accept_hdr(
            stream,
            |req: &tungstenite::handshake::server::Request, res| {
                assert_eq!(req.uri().path(), "/token=/ws");
                Ok(res)
            },
        )
        .unwrap();
        f(socket);
    });
    (uri, handle)
}
#[test]
fn websocket_fake_vm_profile_to_capture() {
    let (uri, handle) = fake_vm(|mut socket| {
        let call: Value = serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap();
        assert_eq!(call["method"], "ext.dart.io.getHttpProfileRequest");
        assert_eq!(call["params"], json!({"isolateId":"isolates/1","id":"r1"}));
        socket
            .send(tungstenite::Message::Text(
                json!({"jsonrpc":"2.0","id":call["id"],"result":profile()})
                    .to_string()
                    .into(),
            ))
            .unwrap();
    });
    let mut client = VmClient::connect(&uri, Duration::from_secs(2)).unwrap();
    let full = client
        .call(
            "ext.dart.io.getHttpProfileRequest",
            json!({"isolateId":"isolates/1","id":"r1"}),
        )
        .unwrap();
    assert_eq!(
        scoped_events(&full, "isolates/1", "main")[1]["response"]["body"],
        "Cà phê 🦀"
    );
    handle.join().unwrap();
}
#[test]
fn detail_guard_skips_rpc_and_detail_error_keeps_connection() {
    let (uri, handle) = fake_vm(|mut socket| {
        for i in 0..2 {
            let call: Value =
                serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap();
            assert_eq!(call["params"]["id"], if i == 0 { "r1" } else { "next" });
            let reply = if i == 0 {
                json!({"jsonrpc":"2.0","id":call["id"],"error":{"code":-32000,"message":"gone"}})
            } else {
                json!({"jsonrpc":"2.0","id":call["id"],"result":profile()})
            };
            socket
                .send(tungstenite::Message::Text(reply.to_string().into()))
                .unwrap();
        }
    });
    let mut client = VmClient::connect(&uri, Duration::from_secs(2)).unwrap();
    let mut large = profile();
    large["request"]["contentLength"] = json!(MAX_BODY_BYTES + 1);
    assert_eq!(
        client.profile_detail("i", &large).unwrap()["request"]["bodyOmitted"],
        "profile-size-limit"
    );
    assert_eq!(
        client.profile_detail("i", &profile()).unwrap()["request"]["bodyOmitted"],
        "profile-read-error"
    );
    let mut next = profile();
    next["id"] = json!("next");
    assert!(client.profile_detail("i", &next).unwrap()["response"]["bodyOmitted"].is_null());
    handle.join().unwrap();
}
#[test]
fn websocket_timeout_is_bounded() {
    let (uri, handle) = fake_vm(|mut socket| {
        socket.read().unwrap();
        thread::sleep(Duration::from_millis(120));
    });
    let mut client = VmClient::connect(&uri, Duration::from_millis(40)).unwrap();
    assert!(matches!(
        client.call("getVM", json!({})),
        Err(RpcError::Timeout)
    ));
    handle.join().unwrap();
}

#[test]
fn websocket_timeout_then_late_reply_notification_and_next_call() {
    let (uri, handle) = fake_vm(|mut socket| {
        let first: Value = serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap();
        let second: Value =
            serde_json::from_str(socket.read().unwrap().to_text().unwrap()).unwrap();
        for reply in [
            json!({"jsonrpc":"2.0","id":first["id"],"result":"late"}),
            json!({"jsonrpc":"2.0","method":"streamNotify","params":{"streamId":"Stdout","event":{"bytes":"aGk="}}}),
            json!({"jsonrpc":"2.0","id":second["id"],"result":{"type":"VM"}}),
        ] {
            socket
                .send(tungstenite::Message::Text(reply.to_string().into()))
                .unwrap();
        }
    });
    let mut client = VmClient::connect(&uri, Duration::from_millis(200)).unwrap();
    assert_eq!(client.call("ignored", json!({})), Err(RpcError::Timeout));
    assert_eq!(
        client.call("getVM", json!({})).unwrap(),
        json!({"type":"VM"})
    );
    let notification = client.take_notification().unwrap();
    assert_eq!(notification["params"]["streamId"], "Stdout");
    assert!(client.take_notification().is_none());
    handle.join().unwrap();
}
#[test]
fn websocket_malformed_reply_returns_error() {
    let (uri, handle) = fake_vm(|mut socket| {
        socket.read().unwrap();
        socket
            .send(tungstenite::Message::Text("not JSON".into()))
            .unwrap();
    });
    let mut client = VmClient::connect(&uri, Duration::from_secs(2)).unwrap();
    assert_eq!(
        client.call("getVM", json!({})),
        Err(RpcError::InvalidMessage)
    );
    handle.join().unwrap();
}
#[test]
fn websocket_close_returns_transport_error() {
    let (uri, handle) = fake_vm(|mut socket| {
        socket.read().unwrap();
        socket.close(None).unwrap();
    });
    let mut client = VmClient::connect(&uri, Duration::from_secs(2)).unwrap();
    assert!(matches!(
        client.call("getVM", json!({})),
        Err(RpcError::Transport(_))
    ));
    handle.join().unwrap();
}
#[test]
fn timestamps_microseconds_negative_and_scalar_headers() {
    let mut p = profile();
    p["startTime"] = json!(-1);
    p["request"]["headers"] = json!({"x":42,"empty":[],"bool":true});
    let event = &network(&p)[0];
    assert_eq!(event["timestamp"], "1969-12-31T23:59:59.999999Z");
    assert_eq!(
        event["request"]["headers"],
        json!({"x":"42","empty":"","bool":"true"})
    );
    assert_eq!(network(&json!({})).len(), 1);
}

#[test]
fn numeric_metadata_cannot_bypass_body_guard() {
    assert!(exceeds_body_limit(
        &json!({"request":{"contentLength":1e30}})
    ));
    assert_eq!(
        declared_body_length(&json!({"headers":{"Content-Length":" 42 "}})),
        Some(42)
    );
    assert_eq!(
        declared_body_length(&json!({"contentLength":42.9})),
        Some(42)
    );
}

#[test]
fn profile_detail_does_not_hide_transport_closure() {
    let (uri, handle) = fake_vm(|mut socket| {
        socket.read().unwrap();
        socket.close(None).unwrap();
    });
    let mut client = VmClient::connect(&uri, Duration::from_secs(2)).unwrap();
    assert!(matches!(
        client.profile_detail("i", &profile()),
        Err(RpcError::Transport(_))
    ));
    handle.join().unwrap();
}
