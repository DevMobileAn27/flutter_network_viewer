use flutter_network_viewer::{
    coordinator::{Coordinator, CoordinatorConfig, ThreadConnector},
    lifecycle::PollConfig,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, VecDeque},
    net::{Shutdown, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};
use tungstenite::Message;
#[track_caller]
fn until(mut f: impl FnMut() -> bool) {
    // This bounds wall-clock scheduling/socket progress, not an RPC deadline.
    // Windows socket timers and shared CI workers can exceed three seconds
    // across subscriptions, bootstrap and the first capture request.
    let start = Instant::now();
    while !f() {
        assert!(
            start.elapsed() < Duration::from_secs(15),
            "VM did not progress within 15 seconds"
        );
        thread::sleep(Duration::from_millis(1));
    }
}
#[derive(Default)]
struct State {
    calls: Vec<Value>,
    events: VecDeque<Value>,
    isolates: Vec<String>,
    profiles: BTreeMap<String, Value>,
    hang: bool,
    hang_isolate: Option<String>,
    closed: bool,
    connections: usize,
}
struct Vm {
    uri: String,
    state: Arc<Mutex<State>>,
    stop: Arc<AtomicBool>,
    sockets: Arc<Mutex<Vec<TcpStream>>>,
    join: Option<thread::JoinHandle<()>>,
}
impl Vm {
    fn new() -> Self {
        Self::with_handshake_delay(Duration::ZERO)
    }
    fn with_handshake_delay(handshake_delay: Duration) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let uri = format!("ws://{}/auth=/ws", listener.local_addr().unwrap());
        let state = Arc::new(Mutex::new(State {
            isolates: vec!["i".into()],
            ..Default::default()
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let sockets = Arc::new(Mutex::new(Vec::new()));
        let st = state.clone();
        let end = stop.clone();
        let handles = sockets.clone();
        let join = thread::spawn(move || {
            let mut children = Vec::new();
            while !end.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((s, _)) => {
                        handles.lock().unwrap().push(s.try_clone().unwrap());
                        let st = st.clone();
                        let end = end.clone();
                        children.push(thread::spawn(move||{
                    // Windows and BSD/macOS can inherit the listener's mode.
                    // The WebSocket handshake below is a blocking operation.
                    s.set_nonblocking(false).unwrap();
                    s.set_nodelay(true).unwrap();
                    thread::sleep(handshake_delay);
                    s.set_read_timeout(Some(Duration::from_secs(2))).unwrap();let Ok(mut ws)=tungstenite::accept(s) else{return};
                    ws.get_mut().set_read_timeout(Some(Duration::from_millis(5))).unwrap();st.lock().unwrap().connections+=1;
                    let mut held=Vec::new();
                    while !end.load(Ordering::Acquire) {
                        let events={let mut st=st.lock().unwrap();if st.closed {st.closed=false;break;}st.events.drain(..).collect::<Vec<_>>()};
                        for event in events {if ws.send(Message::Text(event.to_string().into())).is_err(){return;}}
                        if !st.lock().unwrap().hang {for r in held.drain(..) {if ws.send(Message::Text(serde_json::to_string(&r).unwrap().into())).is_err(){return;}}}
                        let r=match ws.read(){Ok(Message::Text(s))=>serde_json::from_str::<Value>(&s).unwrap(),Ok(Message::Close(_))=>break,Ok(_)=>continue,Err(tungstenite::Error::Io(e)) if matches!(e.kind(),std::io::ErrorKind::WouldBlock|std::io::ErrorKind::TimedOut)=>continue,Err(_)=>break};
                        let (value,hang)={let mut st=st.lock().unwrap();st.calls.push(r.clone());let p=&r["params"];let method=r["method"].as_str().unwrap();let value=match method {
                            "getVM"=>json!({"pid":10,"startTime":100,"operatingSystem":"android","isolates":st.isolates.iter().map(|id|json!({"id":id,"name":"main"})).collect::<Vec<_>>()}),
                            "getIsolate"=>json!({"name":"main","rootLib":{"uri":"package:shop/main.dart"},"extensionRPCs":["ext.dart.io.getHttpProfile"]}),
                            "ext.dart.io.getHttpProfile"=>{let since=p["updatedSince"].as_str().unwrap_or("0").parse::<u64>().unwrap();json!({"timestamp":100,"requests":st.profiles.values().filter(|v|v["updated"].as_u64().unwrap()>=since).cloned().collect::<Vec<_>>()})},
                            "ext.dart.io.getHttpProfileRequest"=>st.profiles[p["id"].as_str().unwrap()].clone(),
                            _=>json!({"type":"Success"}),
                        };(value,st.hang && method=="ext.dart.io.getHttpProfile" && st.hang_isolate.as_deref().is_none_or(|id|p["isolateId"]==id))};
                        let reply=json!({"jsonrpc":"2.0","id":r["id"],"result":value});if hang {held.push(reply);}else if ws.send(Message::Text(reply.to_string().into())).is_err(){break;}
                    }
                    let _=ws.get_mut().shutdown(Shutdown::Both);
                }));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(1))
                    }
                    Err(_) => break,
                }
            }
            for c in children {
                c.join().unwrap();
            }
        });
        Self {
            uri,
            state,
            stop,
            sockets,
            join: Some(join),
        }
    }
    fn profile(&self, id: &str, updated: u64) {
        self.state.lock().unwrap().profiles.insert(id.into(),json!({"id":id,"updated":updated,"method":"GET","uri":"https://example.com","request":{},"response":{"statusCode":200,"endTime":2}}));
    }
    fn event(&self, id: &str, kind: &str) {
        self.state.lock().unwrap().events.push_back(json!({"jsonrpc":"2.0","method":"streamNotify","params":{"streamId":"Isolate","event":{"kind":kind,"isolate":{"id":id}}}}));
    }
}
impl Drop for Vm {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        for s in self.sockets.lock().unwrap().iter() {
            let _ = s.shutdown(Shutdown::Both);
        }
        self.join.take().unwrap().join().unwrap();
    }
}
fn viewer(timeout: Duration) -> Coordinator<ThreadConnector> {
    Coordinator::new(
        ThreadConnector {
            timeout,
            ..Default::default()
        },
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
    )
}
#[test]
fn real_exit_interrupts_hung_rpc_same_socket_hot_restart_and_late_reply_is_ignored() {
    let vm = Vm::new();
    vm.profile("before", 100);
    let mut c = viewer(Duration::from_secs(30));
    c.connect(&vm.uri, 0).unwrap();
    until(|| {
        c.tick(0);
        c.store().network().len() == 1
    });
    let session = c.store().sessions().keys().next().unwrap().clone();
    let initial = vm.state.lock().unwrap().calls.len();
    {
        let mut st = vm.state.lock().unwrap();
        st.hang = true;
        st.hang_isolate = Some("i".into());
    }
    until(|| {
        c.tick(10);
        vm.state.lock().unwrap().calls[initial..]
            .iter()
            .any(|v| v["method"] == "ext.dart.io.getHttpProfile")
    });
    vm.event("i", "IsolateExit");
    {
        let mut st = vm.state.lock().unwrap();
        st.isolates = vec!["new".into()];
        st.profiles.clear();
    }
    vm.profile("after", 101);
    vm.event("new", "IsolateStart");
    // The old reply remains gated until the replacement has captured data.
    until(|| {
        c.tick(11);
        c.store().network().len() == 2
    });
    assert!(vm.state.lock().unwrap().hang);
    let before_release = vm.state.lock().unwrap().calls.len();
    vm.state.lock().unwrap().hang = false;
    until(|| {
        c.tick(21);
        vm.state.lock().unwrap().calls.len() > before_release + 1
    });
    assert_eq!(vm.state.lock().unwrap().connections, 1);
    assert_eq!(c.store().sessions().len(), 1);
    assert!(c.is_online(&session));
    assert_eq!(
        c.store()
            .network()
            .values()
            .map(|v| v.request_event["vmRequestId"].as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["before", "after"]
    );
    c.stop();
}
#[test]
fn real_socket_reconnect_keeps_clear_cursor_and_timeout_reattach_keeps_transport() {
    let vm = Vm::new();
    vm.profile("old", 100);
    // Keep setup RPCs viable on shared runners. The deliberately held profile
    // below must still time out without dropping or replacing the transport.
    let mut c = viewer(Duration::from_secs(2));
    c.connect(&vm.uri, 0).unwrap();
    until(|| {
        c.tick(0);
        c.store().network().len() == 1
    });
    c.clear(0);
    vm.state.lock().unwrap().hang = true;
    until(|| {
        c.tick(10);
        matches!(
            c.last_error(),
            Some(flutter_network_viewer::vm::RpcError::Timeout)
        )
    });
    assert!(c.running());
    assert_eq!(vm.state.lock().unwrap().connections, 1);
    vm.state.lock().unwrap().hang = false;
    vm.profile("fresh", 101);
    until(|| {
        c.tick(20);
        c.store().network().len() == 1
    });
    assert_eq!(
        c.store().network().values().next().unwrap().request_event["vmRequestId"],
        "fresh"
    );
    vm.state.lock().unwrap().closed = true;
    until(|| {
        c.tick(21);
        !c.running()
    });
    until(|| {
        c.tick(51);
        c.running()
    });
    until(|| {
        c.tick(51);
        vm.state
            .lock()
            .unwrap()
            .calls
            .iter()
            .rev()
            .take(3)
            .any(|v| v["method"] == "ext.dart.io.getHttpProfile")
    });
    assert_eq!(vm.state.lock().unwrap().connections, 2);
    assert_eq!(c.store().network().len(), 1);
    c.stop();
}
#[test]
fn cancellation_wakes_pending_websocket_handshake_and_subscription_reads() {
    assert_setup_cancellation(Duration::ZERO);
}

#[test]
fn cancellation_still_wakes_reads_after_slow_fixture_startup() {
    assert_setup_cancellation(Duration::from_millis(3200));
}

fn assert_setup_cancellation(ready_delay: Duration) {
    // Connection readiness is scheduling/handshake work. The cancellation
    // deadline starts only after the peer confirms a pending read.
    let startup_budget = Duration::from_secs(15);
    let cancellation_budget = Duration::from_secs(2);
    for handshake in [true, false] {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let uri = format!("ws://{}/ws", listener.local_addr().unwrap());
        let (ready_tx, ready_rx) = mpsc::channel();
        let (closed_tx, closed_rx) = mpsc::channel();
        let server = thread::spawn(move || {
            let (s, _) = listener.accept().unwrap();
            s.set_nonblocking(false).unwrap();
            s.set_read_timeout(Some(startup_budget)).unwrap();
            let mut socket = if handshake {
                use std::io::Read;
                let mut s = s;
                let mut b = [0; 4096];
                assert!(s.read(&mut b).unwrap() > 0);
                s
            } else {
                let mut ws = tungstenite::accept(s).unwrap();
                assert!(ws.read().is_ok());
                ws.into_inner()
            };
            thread::sleep(ready_delay);
            // Keep the peer's fallback timeout long enough for the owner to
            // be scheduled. closed_rx below enforces 2 s starting at Stop/Drop.
            ready_tx.send(()).unwrap();
            use std::io::Read;
            // A read timeout is not evidence that cancellation closed the peer.
            // Windows may report a reset rather than EOF after shutdown(Both).
            match socket.read(&mut [0; 4096]) {
                Ok(0) => {}
                Err(e)
                    if matches!(
                        e.kind(),
                        std::io::ErrorKind::ConnectionReset
                            | std::io::ErrorKind::ConnectionAborted
                            | std::io::ErrorKind::BrokenPipe
                            | std::io::ErrorKind::NotConnected
                    ) => {}
                result => panic!("expected peer closure after cancellation, got {result:?}"),
            }
            closed_tx.send(()).unwrap();
        });
        let mut c = viewer(Duration::from_secs(30));
        c.connect(&uri, 0).unwrap();
        ready_rx
            .recv_timeout(startup_budget)
            .expect("VM fixture did not reach pending handshake/subscription read");
        if handshake {
            c.stop();
        } else {
            drop(c);
        }
        closed_rx
            .recv_timeout(cancellation_budget)
            .expect("Stop/Drop did not close the pending connection within 2 seconds");
        server.join().unwrap();
    }
}

#[test]
fn delayed_vm_startup_still_captures_without_advancing_the_logical_clock() {
    // A busy CI runner may schedule the connection worker late. The harness
    // deadline is separate from the RPC timeout and the coordinator clock.
    let vm = Vm::with_handshake_delay(Duration::from_millis(3200));
    vm.profile("delayed", 100);
    let mut c = viewer(Duration::from_secs(30));
    c.connect(&vm.uri, 0).unwrap();
    until(|| {
        c.tick(0);
        c.store().network().len() == 1
    });
    assert_eq!(vm.state.lock().unwrap().connections, 1);
    assert!(c.last_error().is_none(), "{:?}", c.last_error());
    c.stop();
}

#[test]
fn fake_vm_keeps_accepted_socket_open_until_handshake_arrives() {
    let vm = Vm::new();
    let uri = url::Url::parse(&vm.uri).unwrap();
    let socket = TcpStream::connect(("127.0.0.1", uri.port().unwrap())).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    // Accept the TCP socket before the client sends its HTTP upgrade. Accepted
    // sockets can inherit nonblocking mode from the listening socket.
    thread::sleep(Duration::from_millis(100));
    let (mut ws, _) = tungstenite::client(vm.uri.as_str(), socket).unwrap();
    ws.send(Message::Text(
        json!({"jsonrpc":"2.0","id":1,"method":"streamListen","params":{"streamId":"Isolate"}})
            .to_string()
            .into(),
    ))
    .unwrap();
    let reply: Value = serde_json::from_str(ws.read().unwrap().to_text().unwrap()).unwrap();
    assert_eq!(reply["result"]["type"], "Success");
    assert_eq!(vm.state.lock().unwrap().connections, 1);
}
