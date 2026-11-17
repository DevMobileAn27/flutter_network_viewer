use flutter_network_viewer::{
    discovery::{discover_processes, resolve_dds},
    vm::ConnectionCancel,
};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex, mpsc},
    thread,
    time::Duration,
};
use tungstenite::{
    Message,
    handshake::server::{Request, Response},
};
#[allow(clippy::result_large_err)] // tungstenite callback fixes this error type.
fn vm(
    pid: u64,
    flutter: bool,
    root: &'static str,
    requests: usize,
) -> (String, thread::JoinHandle<()>, Arc<Mutex<Vec<String>>>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let uri = format!("ws://{}/a%2Fb%3D/ws", l.local_addr().unwrap());
    let paths = Arc::new(Mutex::new(vec![]));
    let p = paths.clone();
    let join = thread::spawn(move || {
        // Each candidate gets an HTTP resolution request, then a probe WebSocket.
        for _ in 0..requests {
            let (mut s, _) = l.accept().unwrap();
            s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
            let mut buf = [0; 4096];
            let n = s.peek(&mut buf).unwrap();
            let request = String::from_utf8_lossy(&buf[..n]);
            if !request.to_ascii_lowercase().contains("upgrade: websocket") {
                let mut rd = BufReader::new(s.try_clone().unwrap());
                loop {
                    let mut line = String::new();
                    rd.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                }
                s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                    .unwrap();
                continue;
            }
            let p = p.clone();
            let mut ws = tungstenite::accept_hdr(s, move |r: &Request, res: Response| {
                p.lock().unwrap().push(r.uri().path().into());
                Ok(res)
            })
            .unwrap();
            while let Ok(Message::Text(text)) = ws.read() {
                let r: Value = serde_json::from_str(&text).unwrap();
                if r["params"]["isolateId"] == "exited" {
                    ws.send(Message::Text(json!({"jsonrpc":"2.0","id":r["id"],"error":{"code":-32000,"message":"isolate exited"}}).to_string().into())).unwrap();
                    continue;
                }
                let value = if r["method"] == "getVM" {
                    json!({"pid":pid,"startTime":100,"operatingSystem":"android","isolates":[{"id":"i"},{"id":"exited"}]})
                } else {
                    json!({"name":"main","rootLib":{"uri":root},"extensionRPCs":if flutter{vec!["ext.flutter.inspector"]}else{vec![]}})
                };
                ws.send(Message::Text(
                    json!({"jsonrpc":"2.0","id":r["id"],"result":value})
                        .to_string()
                        .into(),
                ))
                .unwrap();
            }
        }
    });
    (uri, join, paths)
}
fn redirect(location: String) -> (String, thread::JoinHandle<()>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let uri = format!("http://{}/old=/", l.local_addr().unwrap());
    let join = thread::spawn(move || {
        let (mut s, _) = l.accept().unwrap();
        s.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
        let mut rd = BufReader::new(s.try_clone().unwrap());
        loop {
            let mut line = String::new();
            rd.read_line(&mut line).unwrap();
            if line == "\r\n" {
                break;
            }
        }
        write!(
            s,
            "HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\n\r\n"
        )
        .unwrap();
    });
    (uri, join)
}
#[test]
fn real_redirect_encoded_token_and_vm_deduplication() {
    let (uri, join, paths) = vm(20, true, "package:shop/main.dart", 1);
    let nested = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("uri", &uri)
        .finish();
    let (source, rjoin) = redirect(format!("http://localhost:1234/devtools/?{nested}"));
    // Redirect resolves directly to the nested socket; a second candidate is
    // deliberately identical at the VM identity level, not at the source URI.
    let (uri2, join2, _) = vm(20, true, "package:shop/main.dart", 2);
    let processes = format!(
        "dart development-service --vm-service-uri={source}\ndart dds.dart --vm-service-uri={uri2}"
    );
    let found = discover_processes(
        &processes,
        &ConnectionCancel::default(),
        Duration::from_secs(2),
        1,
        "macos",
    )
    .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].name, "shop");
    assert_eq!(&*paths.lock().unwrap(), &["/a%2Fb%3D/ws"]);
    rjoin.join().unwrap();
    join.join().unwrap();
    join2.join().unwrap();
}
#[test]
fn real_probe_ignores_non_flutter_and_viewer() {
    let (uri, j, _) = vm(20, false, "package:dart/main.dart", 2);
    let (viewer, j2, _) = vm(30, true, "package:flutter_network_viewer/main.dart", 2);
    let found=discover_processes(&format!("dart development-service --vm-service-uri={uri}\ndart development-service --vm-service-uri={viewer}"),&ConnectionCancel::default(),Duration::from_secs(2),1,"macos").unwrap();
    assert!(found.is_empty());
    j.join().unwrap();
    j2.join().unwrap();
}
#[test]
fn rejects_remote_redirect_and_cancellation_interrupts_hung_http() {
    let (uri, j) = redirect("http://example.com:9999/token=/".into());
    assert!(resolve_dds(&uri, &ConnectionCancel::default(), Duration::from_secs(2)).is_err());
    j.join().unwrap();
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let uri = format!("http://{}/a=/", l.local_addr().unwrap());
    let (accepted, rx) = mpsc::channel::<TcpStream>();
    let server = thread::spawn(move || {
        accepted.send(l.accept().unwrap().0).unwrap();
    });
    let cancel = ConnectionCancel::default();
    let c = cancel.clone();
    let (tx, done) = mpsc::channel();
    let client = thread::spawn(move || {
        tx.send(resolve_dds(&uri, &c, Duration::from_secs(8)))
            .unwrap();
    });
    let _socket = rx.recv_timeout(Duration::from_secs(2)).unwrap();
    cancel.cancel();
    assert!(done.recv_timeout(Duration::from_secs(1)).unwrap().is_err());
    server.join().unwrap();
    client.join().unwrap();
}
