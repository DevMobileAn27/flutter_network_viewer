//! F14 fixture: notifications sent during subscriptions, before getVM bootstrap.
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
            let Ok(mut ws) = tungstenite::accept(stream) else {
                return;
            };

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
                if r["method"] == "streamListen" {
                    let stream = r["params"]["streamId"].as_str().unwrap();
                    let mut event = json!({"isolate":{"id":"i"},"timestamp":1000});
                    match stream {
                        "Stdout" => event["bytes"] = json!(base64::engine::general_purpose::STANDARD.encode("Native early console\nflutter: {\nflutter: \"answer\":42\nflutter: }\n")),
                        "Stderr" => event["bytes"] = json!(base64::engine::general_purpose::STANDARD.encode("══╡ EXCEPTION caught\nbroken layout\n════════════\n")),
                        "Logging" => event["logRecord"] = json!({"level":1000,"message":{"valueAsString":"logger failure"},"loggerName":{"valueAsString":"orders"},"error":{"valueAsString":"native-error"},"stackTrace":{"valueAsString":"native-stack"}}),
                        _ => {}
                    }
                    if stream != "Isolate" {
                        ws.send(Message::Text(json!({"jsonrpc":"2.0","method":"streamNotify","params":{"streamId":stream,"event":event}}).to_string().into())).unwrap();
                    }
                }
                let result = match r["method"].as_str().unwrap_or("") {
                    "f13.complete" => {
                        c.store(true, Ordering::Release);
                        json!({"type":"Success"})
                    }
                    "getVM" => {
                        json!({"pid":424242,"startTime":100,"operatingSystem":"android","isolates":[{"id":"i","name":"main"}]})
                    }
                    "getIsolate" => {
                        json!({"name":"main","rootLib":{"uri":"package:f14_smoke/main.dart"},"extensionRPCs":["ext.flutter.inspector","ext.dart.io.getHttpProfile"]})
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
            }
        });
    }
}
