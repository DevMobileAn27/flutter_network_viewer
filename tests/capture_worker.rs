use flutter_network_viewer::{
    lifecycle::{CaptureEngine, CaptureTransport, PollConfig, Request},
    vm::RpcError,
    vm_worker::VmWorker,
};
use serde_json::{Value, json};
use std::{
    net::TcpListener,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use tungstenite::{Message, WebSocket};
fn send(ws: &mut WebSocket<std::net::TcpStream>, v: Value) {
    ws.send(Message::Text(v.to_string().into())).unwrap();
}
fn read(ws: &mut WebSocket<std::net::TcpStream>) -> Value {
    serde_json::from_str(ws.read().unwrap().to_text().unwrap()).unwrap()
}
fn subscriptions(ws: &mut WebSocket<std::net::TcpStream>) {
    for stream in ["Stdout", "Stderr", "Logging"] {
        let r = read(ws);
        assert_eq!(r["method"], "streamListen");
        assert_eq!(r["params"]["streamId"], stream);
        send(
            ws,
            json!({"jsonrpc":"2.0","id":r["id"],"result":{"type":"Success"}}),
        );
    }
}
fn notification(s: &str) -> Value {
    use base64::Engine;
    json!({"jsonrpc":"2.0","method":"streamNotify","params":{"streamId":"Stdout","event":{"isolate":{"id":"i"},"bytes":base64::engine::general_purpose::STANDARD.encode(s)}}})
}
fn until(mut f: impl FnMut() -> bool) {
    let start = Instant::now();
    while !f() {
        assert!(
            start.elapsed() < Duration::from_secs(3),
            "worker did not progress"
        );
        thread::sleep(Duration::from_millis(1));
    }
}
#[test]
fn real_vm_console_flows_during_hung_detail_and_stop_interrupts_read() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let uri = format!("ws://{}/token=/ws", listener.local_addr().unwrap());
    let (held_tx, held_rx) = mpsc::channel();
    let (closed_tx, closed_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        let (s, _) = listener.accept().unwrap();
        s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let mut ws = tungstenite::accept(s).unwrap();
        subscriptions(&mut ws);
        let r = read(&mut ws);
        assert_eq!(r["method"], "ext.dart.io.getHttpProfile");
        send(
            &mut ws,
            json!({"jsonrpc":"2.0","id":r["id"],"result":{"timestamp":20,"requests":[{"id":"a","method":"GET","uri":"https://example.com","request":{},"response":{}}]}}),
        );
        let r = read(&mut ws);
        assert_eq!(r["method"], "ext.dart.io.getHttpProfileRequest");
        send(&mut ws, notification("while held\n"));
        held_tx.send(()).unwrap();
        assert!(ws.read().is_err());
        closed_tx.send(()).unwrap();
    });
    let worker = VmWorker::connect(&uri, Duration::from_secs(60), 10, 4096).unwrap();
    let mut e = CaptureEngine::new(
        worker,
        json!({"id":"app"}),
        "i",
        "main",
        PollConfig::default(),
    );
    e.start(0);
    e.tick(0);
    until(|| {
        e.tick(1);
        held_rx.try_recv().is_ok()
    });
    until(|| {
        e.tick(2);
        e.pending_console_len() == 1
    });
    e.tick(1400);
    assert_eq!(e.store().logs()[0].event["message"], "while held");
    assert!(e.store().network().is_empty());
    e.stop();
    closed_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    server.join().unwrap();
}
#[test]
fn idle_pump_is_bounded_discards_and_receives_without_rpc() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let uri = format!("ws://{}/ws", listener.local_addr().unwrap());
    let (ready_tx, ready_rx) = mpsc::channel();
    let (gate_tx, gate_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        let (s, _) = listener.accept().unwrap();
        let mut ws = tungstenite::accept(s).unwrap();
        subscriptions(&mut ws);
        for i in 0..25 {
            send(&mut ws, notification(&format!("line {i}\n")));
        }
        ready_tx.send(()).unwrap();
        gate_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        let r = read(&mut ws);
        send(
            &mut ws,
            json!({"jsonrpc":"2.0","id":r["id"],"result":{"ok":true}}),
        );
        let _ = ws.read();
    });
    let mut w = VmWorker::connect(&uri, Duration::from_secs(2), 10, 4096).unwrap();
    ready_rx.recv_timeout(Duration::from_secs(3)).unwrap();
    until(|| w.dropped_notifications() == 15);
    assert_eq!(w.queued_notifications(), 10);
    assert!(w.queued_bytes() <= 4096);
    let n = w.take_notification().unwrap();
    assert_eq!(n["method"], "streamNotify");
    w.discard_notifications();
    assert_eq!(w.queued_bytes(), 0);
    gate_tx.send(()).unwrap();
    w.submit(Request {
        id: 42,
        method: "getVM".into(),
        params: json!({}),
    })
    .unwrap();
    let mut result = None;
    until(|| {
        result = w.take_reply();
        result.is_some()
    });
    assert_eq!(result.unwrap(), (42, Ok(json!({"ok":true}))));
    w.stop();
    server.join().unwrap();
}
#[test]
fn timeout_late_reply_and_next_rpc_keep_socket_alive() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let uri = format!("ws://{}/ws", listener.local_addr().unwrap());
    let (gate_tx, gate_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        let (s, _) = listener.accept().unwrap();
        let mut ws = tungstenite::accept(s).unwrap();
        subscriptions(&mut ws);
        let old = read(&mut ws);
        gate_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        send(
            &mut ws,
            json!({"jsonrpc":"2.0","id":old["id"],"result":{"late":true}}),
        );
        send(&mut ws, notification("alive\n"));
        let r = read(&mut ws);
        send(
            &mut ws,
            json!({"jsonrpc":"2.0","id":r["id"],"result":{"new":true}}),
        );
        let _ = ws.read();
    });
    let mut w = VmWorker::connect(&uri, Duration::from_millis(100), 10, 4096).unwrap();
    w.submit(Request {
        id: 1,
        method: "slow".into(),
        params: json!({}),
    })
    .unwrap();
    let mut result = None;
    until(|| {
        result = w.take_reply();
        result.is_some()
    });
    assert_eq!(result.take().unwrap(), (1, Err(RpcError::Timeout)));
    gate_tx.send(()).unwrap();
    until(|| w.queued_notifications() > 0);
    w.submit(Request {
        id: 2,
        method: "next".into(),
        params: json!({}),
    })
    .unwrap();
    until(|| {
        result = w.take_reply();
        result.is_some()
    });
    assert_eq!(result.unwrap(), (2, Ok(json!({"new":true}))));
    w.stop();
    server.join().unwrap();
}

#[test]
fn real_clear_while_detail_pending_uses_exclusive_boundary_and_accepts_new_data() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let uri = format!("ws://{}/ws", listener.local_addr().unwrap());
    let (held_tx, held_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        let (s, _) = listener.accept().unwrap();
        s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let mut ws = tungstenite::accept(s).unwrap();
        subscriptions(&mut ws);
        let old = json!({"id":"old","method":"GET","uri":"https://example.com/old","request":{},"response":{"endTime":5}});
        let r = read(&mut ws);
        send(
            &mut ws,
            json!({"jsonrpc":"2.0","id":r["id"],"result":{"timestamp":20,"requests":[old.clone()]}}),
        );
        let detail = read(&mut ws);
        send(&mut ws, notification("ready\npartial before clear"));
        held_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        send(
            &mut ws,
            json!({"jsonrpc":"2.0","id":detail["id"],"result":old}),
        );
        let r = read(&mut ws);
        assert_eq!(r["params"]["updatedSince"], "21");
        // The fake VM applies the inclusive >= cursor filter: timestamp 20 is
        // excluded; only the new request at 21 survives.
        let new = json!({"id":"new","method":"GET","uri":"https://example.com/new","request":{},"response":{"endTime":6}});
        send(
            &mut ws,
            json!({"jsonrpc":"2.0","id":r["id"],"result":{"timestamp":21,"requests":[new.clone()]}}),
        );
        let r = read(&mut ws);
        send(&mut ws, json!({"jsonrpc":"2.0","id":r["id"],"result":new}));
        send(&mut ws, notification("fresh\n"));
        let _ = ws.read();
    });
    let w = VmWorker::connect(&uri, Duration::from_secs(3), 100, 100_000).unwrap();
    let mut e = CaptureEngine::new(w, json!({"id":"app"}), "i", "main", PollConfig::default());
    e.start(0);
    e.tick(0);
    until(|| {
        e.tick(1);
        held_rx.try_recv().is_ok()
    });
    until(|| {
        e.tick(2);
        // An empty mailbox can mean the worker has not read the socket yet.
        // This complete line proves the chunk and its partial tail were decoded.
        e.pending_console_len() == 1
    });
    e.clear(3);
    release_tx.send(()).unwrap();
    until(|| {
        e.tick(4);
        !e.poll_in_flight()
    });
    until(|| {
        e.tick(5000);
        e.store().network().len() == 1
    });
    assert_eq!(
        e.store().network().values().next().unwrap().request_event["vmRequestId"],
        "new"
    );
    until(|| {
        e.tick(10_000);
        !e.store().logs().is_empty()
    });
    assert_eq!(e.store().logs()[0].event["message"], "fresh");
    e.stop();
    server.join().unwrap();
}
#[test]
fn worker_byte_budget_and_drop_close_idle_socket() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let uri = format!("ws://{}/ws", listener.local_addr().unwrap());
    let (done_tx, done_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        let (s, _) = listener.accept().unwrap();
        s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let mut ws = tungstenite::accept(s).unwrap();
        subscriptions(&mut ws);
        send(&mut ws, notification("too large\n"));
        assert!(ws.read().is_err());
        done_tx.send(()).unwrap();
    });
    let w = VmWorker::connect(&uri, Duration::from_secs(3), 100, 1).unwrap();
    until(|| w.dropped_notifications() == 1);
    assert_eq!(w.queued_bytes(), 0);
    assert_eq!(w.queued_notifications(), 0);
    drop(w);
    done_rx.recv_timeout(Duration::from_secs(2)).unwrap();
    server.join().unwrap();
}
#[test]
fn idle_disconnect_reaches_lifecycle_and_cleans_session() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let uri = format!("ws://{}/ws", listener.local_addr().unwrap());
    let (close_tx, close_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        let (s, _) = listener.accept().unwrap();
        let mut ws = tungstenite::accept(s).unwrap();
        subscriptions(&mut ws);
        send(&mut ws, notification("retained\n"));
        let r = read(&mut ws);
        send(
            &mut ws,
            json!({"jsonrpc":"2.0","id":r["id"],"result":{"timestamp":20,"requests":[]}}),
        );
        close_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        ws.close(None).unwrap();
    });
    let w = VmWorker::connect(&uri, Duration::from_secs(3), 10, 4096).unwrap();
    let mut e = CaptureEngine::new(w, json!({"id":"app"}), "i", "main", PollConfig::default());
    e.start(0);
    e.tick(0);
    until(|| {
        e.tick(1);
        e.cursor() == Some(20) && e.pending_console_len() == 1
    });
    e.tick(1400);
    assert_eq!(e.store().logs().len(), 1);
    close_tx.send(()).unwrap();
    until(|| {
        e.tick(1401);
        !e.running()
    });
    assert!(e.store().sessions().is_empty());
    assert!(e.store().logs().is_empty());
    server.join().unwrap();
}

#[test]
fn worker_overflow_is_counted_by_capture_store_and_keeps_latest_logs() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let uri = format!("ws://{}/ws", listener.local_addr().unwrap());
    let (gate_tx, gate_rx) = mpsc::channel();
    let server = thread::spawn(move || {
        let (s, _) = listener.accept().unwrap();
        let mut ws = tungstenite::accept(s).unwrap();
        subscriptions(&mut ws);
        for i in 0..25 {
            send(&mut ws, notification(&format!("line {i}\n")));
        }
        gate_rx.recv_timeout(Duration::from_secs(3)).unwrap();
        let _ = ws.read();
    });
    let w = VmWorker::connect(&uri, Duration::from_secs(3), 10, 4096).unwrap();
    until(|| w.dropped_notifications() == 15);
    let mut e = CaptureEngine::new(
        w,
        json!({"id":"app"}),
        "i",
        "main",
        PollConfig {
            max_records: 10,
            ..PollConfig::default()
        },
    );
    e.start(0);
    e.tick(1400);
    assert_eq!(e.store().logs().len(), 10);
    assert_eq!(e.store().logs()[0].event["message"], "line 15");
    assert_eq!(e.store().statistics.evicted, 15);
    gate_tx.send(()).unwrap();
    e.stop();
    server.join().unwrap();
}
