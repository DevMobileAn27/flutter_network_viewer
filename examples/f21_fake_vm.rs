//! Manual native UI smoke fixture. Prints an ephemeral URI; enter `complete`
//! on stdin to finish the selected pending request. Never contacts a real VM.
use base64::Engine;
use serde_json::{Value, json};
use std::{
    io::{self, BufRead},
    net::TcpListener,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
};
use tungstenite::Message;
fn main() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    println!("ws://{}/smoke=/ws", listener.local_addr().unwrap());
    let complete = Arc::new(AtomicBool::new(false));
    let c = complete.clone();
    thread::spawn(move || {
        for command in io::stdin().lock().lines().map_while(Result::ok) {
            if command == "complete" {
                c.store(true, Ordering::Release);
            }
        }
    });
    for stream in listener.incoming() {
        let Ok(stream) = stream else {
            break;
        };
        let c = complete.clone();
        thread::spawn(move || {
            let mut stream = stream;
            use std::io::{Read, Write};
            let mut peek = [0; 4096];
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let start = std::time::Instant::now();
            let count = loop {
                let count = stream.peek(&mut peek).unwrap_or(0);
                if count == 0 {
                    return;
                }
                if peek[..count].windows(4).any(|w| w == b"\r\n\r\n") {
                    break count;
                }
                if start.elapsed() > std::time::Duration::from_secs(5) {
                    return;
                }
                std::thread::sleep(std::time::Duration::from_millis(1));
            };
            if !String::from_utf8_lossy(&peek[..count])
                .to_ascii_lowercase()
                .contains("upgrade: websocket")
            {
                let header_end = String::from_utf8_lossy(&peek[..count])
                    .find("\r\n\r\n")
                    .map(|n| n + 4)
                    .unwrap_or(count);
                stream.read_exact(&mut peek[..header_end]).unwrap();
                let _ = stream.write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                );
                return;
            }
            let Ok(mut ws) = tungstenite::accept(stream) else {
                return;
            };
            let mut console_sent = false;
            while let Ok(Message::Text(message)) = ws.read() {
                let r: Value = serde_json::from_str(&message).unwrap();
                let done = c.load(Ordering::Acquire);
                let profile = |id: &str| {
                    let finished = id == "orders" || done;
                    let mut p = json!({"id":id,"updated":if done {200}else{100},"startTime":1000000,"method":if id=="orders" {"POST"}else{"GET"},"uri":format!("https://api.example.com/{id}"),"request":{"headers":{"accept":["application/json"]}},"requestBody":if id=="orders" {b"{\"item\":\"Coffee\"}".to_vec()}else{vec![]},"response":{}});
                    if finished {
                        p["response"] = json!({"statusCode":201,"reasonPhrase":"Created","headers":{"content-type":["application/json"]},"endTime":1124000});
                        p["responseBody"] = json!(b"{\"id\":42,\"status\":\"created\"}".to_vec());
                    }
                    p
                };
                let result = match r["method"].as_str().unwrap_or("") {
                    "f13.complete" => {
                        c.store(true, Ordering::Release);
                        json!({"type":"Success"})
                    }
                    "getVM" => {
                        json!({"pid":std::process::id(),"startTime":100,"operatingSystem":"android","isolates":[{"id":"i","name":"main"}]})
                    }
                    "getIsolate" => {
                        json!({"name":"main","rootLib":{"uri":"package:f13_smoke/main.dart"},"extensionRPCs":["ext.flutter.inspector","ext.dart.io.getHttpProfile"]})
                    }
                    "ext.dart.io.getHttpProfile" => {
                        json!({"timestamp":if done{200}else{100},"requests":[profile("orders"),profile("feed")]})
                    }
                    "ext.dart.io.getHttpProfileRequest" => {
                        profile(r["params"]["id"].as_str().unwrap())
                    }
                    _ => json!({"type":"Success"}),
                };
                if ws
                    .send(Message::Text(
                        json!({"jsonrpc":"2.0","id":r["id"],"result":result})
                            .to_string()
                            .into(),
                    ))
                    .is_err()
                {
                    break;
                }
                if r["method"] == "ext.dart.io.getHttpProfile" && !console_sent {
                    console_sent = true;
                    let event = json!({"jsonrpc":"2.0","method":"streamNotify","params":{"streamId":"Stdout","event":{"isolate":{"id":"i"},"bytes":base64::engine::general_purpose::STANDARD.encode("Native smoke console\n")}}});
                    if ws.send(Message::Text(event.to_string().into())).is_err() {
                        break;
                    }
                }
            }
        });
    }
}
