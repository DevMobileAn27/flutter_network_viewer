//! Bounded single-RPC worker with a continuous notification pump. The cloned TCP
//! shutdown handle interrupts both plain WS and native TLS reads on Stop/Drop.
use crate::{
    lifecycle::{CaptureTransport, Request},
    vm::{ConnectionCancel, RpcError, VmClient},
};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    net::{Shutdown, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender, TryRecvError},
    },
    thread,
    time::Duration,
};
#[derive(Default)]
struct Mailbox {
    notifications: VecDeque<(Value, usize)>,
    bytes: usize,
    dropped: u64,
    reported_dropped: u64,
    reply: Option<(u64, Result<Value, RpcError>)>,
    busy: bool,
    terminal: Option<RpcError>,
}
pub struct VmWorker {
    commands: SyncSender<Request>,
    mailbox: Arc<Mutex<Mailbox>>,
    stopped: Arc<AtomicBool>,
    socket: TcpStream,
}
impl VmWorker {
    /// Connection and the three stream subscriptions are blocking setup. Invoke
    /// this factory on the application's connection worker, then tick the engine
    /// on its owner thread. Capture RPCs and notification reads never block it.
    pub fn connect(
        uri: &str,
        timeout: Duration,
        max_records: usize,
        max_bytes: usize,
    ) -> Result<Self, RpcError> {
        Self::connect_setup(
            uri,
            timeout,
            max_records,
            max_bytes,
            &ConnectionCancel::default(),
            false,
        )
    }
    pub fn connect_coordinated(
        uri: &str,
        timeout: Duration,
        max_records: usize,
        max_bytes: usize,
        cancel: &ConnectionCancel,
    ) -> Result<Self, RpcError> {
        Self::connect_setup(uri, timeout, max_records, max_bytes, cancel, true)
    }
    fn connect_setup(
        uri: &str,
        timeout: Duration,
        max_records: usize,
        max_bytes: usize,
        cancel: &ConnectionCancel,
        coordinated: bool,
    ) -> Result<Self, RpcError> {
        let mut client = VmClient::connect_cancellable(uri, timeout, cancel)?;
        let streams = if coordinated {
            vec!["Isolate", "Stdout", "Stderr", "Logging"]
        } else {
            vec!["Stdout", "Stderr", "Logging"]
        };
        for stream in streams {
            match client.call("streamListen", json!({"streamId":stream})) {
                Ok(_) | Err(RpcError::Remote { code: 103, .. }) => {}
                Err(e) => return Err(e),
            }
        }
        if cancel.is_cancelled() {
            return Err(RpcError::Transport("connection cancelled".into()));
        }
        let socket = client.shutdown_handle()?;
        let mailbox = Arc::new(Mutex::new(Mailbox::default()));
        let stopped = Arc::new(AtomicBool::new(false));
        let (commands, rx) = mpsc::sync_channel::<Request>(1);
        let shared = mailbox.clone();
        let stop = stopped.clone();
        thread::Builder::new()
            .name("vm-capture".into())
            .spawn(move || {
                let mut sink = |event: Value| {
                    if stop.load(Ordering::Acquire) {
                        return;
                    }
                    let bytes = event.to_string().len();
                    let mut m = shared.lock().unwrap();
                    if stop.load(Ordering::Acquire) {
                        return;
                    }
                    if bytes > max_bytes || max_records == 0 {
                        m.dropped += 1;
                        return;
                    }
                    m.bytes += bytes;
                    m.notifications.push_back((event, bytes));
                    while m.notifications.len() > max_records || m.bytes > max_bytes {
                        if let Some((_, bytes)) = m.notifications.pop_front() {
                            m.bytes -= bytes;
                            m.dropped += 1;
                        }
                    }
                };
                while let Some(event) = client.take_notification() {
                    sink(event);
                }
                while !stop.load(Ordering::Acquire) {
                    match rx.try_recv() {
                        Ok(request) => {
                            let result =
                                client.call_streaming(&request.method, request.params, &mut sink);
                            let terminal = matches!(
                                result,
                                Err(RpcError::Transport(_) | RpcError::InvalidMessage)
                            );
                            if stop.load(Ordering::Acquire) {
                                break;
                            }
                            let mut mailbox = shared.lock().unwrap();
                            if stop.load(Ordering::Acquire) {
                                break;
                            }
                            mailbox.reply = Some((request.id, result));
                            drop(mailbox);
                            if terminal {
                                break;
                            }
                        }
                        Err(TryRecvError::Disconnected) => break,
                        Err(TryRecvError::Empty) => match client.pump() {
                            Ok(Some(event)) => sink(event),
                            Ok(None) => {}
                            Err(e) => {
                                shared.lock().unwrap().terminal = Some(e);
                                break;
                            }
                        },
                    }
                }
            })
            .map_err(|e| RpcError::Transport(e.to_string()))?;
        Ok(Self {
            commands,
            mailbox,
            stopped,
            socket,
        })
    }
    pub fn queued_notifications(&self) -> usize {
        self.mailbox.lock().unwrap().notifications.len()
    }
    pub fn queued_bytes(&self) -> usize {
        self.mailbox.lock().unwrap().bytes
    }
    pub fn dropped_notifications(&self) -> u64 {
        self.mailbox.lock().unwrap().dropped
    }
}
impl CaptureTransport for VmWorker {
    fn submit(&mut self, request: Request) -> Result<(), RpcError> {
        if self.stopped.load(Ordering::Acquire) {
            return Err(RpcError::Transport("worker stopped".into()));
        }
        let mut m = self.mailbox.lock().unwrap();
        if m.busy {
            return Err(RpcError::InvalidRequest);
        }
        m.busy = true;
        self.commands.try_send(request).map_err(|_| {
            m.busy = false;
            RpcError::Transport("worker unavailable".into())
        })
    }
    fn take_reply(&mut self) -> Option<(u64, Result<Value, RpcError>)> {
        let mut m = self.mailbox.lock().unwrap();
        let reply = m
            .reply
            .take()
            .or_else(|| m.terminal.take().map(|e| (0, Err(e))));
        if reply.is_some() {
            m.busy = false;
        }
        reply
    }
    fn take_dropped_notifications(&mut self) -> u64 {
        let mut m = self.mailbox.lock().unwrap();
        let delta = m.dropped - m.reported_dropped;
        m.reported_dropped = m.dropped;
        delta
    }
    fn take_notification(&mut self) -> Option<Value> {
        let mut m = self.mailbox.lock().unwrap();
        let (event, bytes) = m.notifications.pop_front()?;
        m.bytes -= bytes;
        Some(event)
    }
    fn discard_notifications(&mut self) {
        let mut m = self.mailbox.lock().unwrap();
        m.notifications.clear();
        m.bytes = 0;
    }
    fn stop(&mut self) {
        if !self.stopped.swap(true, Ordering::AcqRel) {
            let _ = self.socket.shutdown(Shutdown::Both);
            self.discard_notifications();
            let mut m = self.mailbox.lock().unwrap();
            m.reply = None;
            m.terminal = None;
            m.busy = false;
        }
    }
}
impl Drop for VmWorker {
    fn drop(&mut self) {
        self.stop();
    }
}
