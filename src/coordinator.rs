//! One VM socket, one application history, multiple isolate polling engines.
//! All owner-side methods are nonblocking; drive `tick` with monotonic milliseconds.
use crate::{
    capture::CaptureStore,
    lifecycle::{CaptureEngine, CaptureTransport, PollConfig, Request},
    vm::{ConnectionCancel, RpcError, normalize_uri},
    vm_worker::VmWorker,
};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, VecDeque},
    rc::Rc,
    sync::mpsc,
    thread,
    time::Duration,
};

pub trait ConnectionFactory {
    type Transport: CaptureTransport;
    fn begin(
        &mut self,
        uri: &str,
        cancel: ConnectionCancel,
    ) -> mpsc::Receiver<Result<Self::Transport, RpcError>>;
}
/// Setup runs off the owner thread. Cancel closes setup's socket before handshake
/// and subscription reads. System DNS/TCP connect may finish in the background;
/// their cancelled result can never be installed by the coordinator.
#[derive(Clone)]
pub struct ThreadConnector {
    pub timeout: Duration,
    pub max_records: usize,
    pub max_bytes: usize,
}
impl Default for ThreadConnector {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(8),
            max_records: 5000,
            max_bytes: 64 * 1024 * 1024,
        }
    }
}
impl ConnectionFactory for ThreadConnector {
    type Transport = VmWorker;
    fn begin(
        &mut self,
        uri: &str,
        cancel: ConnectionCancel,
    ) -> mpsc::Receiver<Result<VmWorker, RpcError>> {
        let (tx, rx) = mpsc::channel();
        let uri = uri.to_owned();
        let timeout = self.timeout;
        let records = self.max_records;
        let bytes = self.max_bytes;
        let failure = tx.clone();
        if let Err(e) = thread::Builder::new()
            .name("vm-connect".into())
            .spawn(move || {
                let result = VmWorker::connect_coordinated(&uri, timeout, records, bytes, &cancel);
                let _ = tx.send(result);
            })
        {
            let _ = failure.send(Err(RpcError::Transport(e.to_string())));
        }
        rx
    }
}
#[derive(Clone)]
pub struct CoordinatorConfig {
    pub retry_ms: u64,
    pub discovery_ms: u64,
    pub poll: PollConfig,
}
impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self::production()
    }
}
impl CoordinatorConfig {
    pub fn production() -> Self {
        Self {
            retry_ms: 3000,
            discovery_ms: 1400,
            poll: PollConfig::default(),
        }
    }
}
type Reply = (u64, Result<Value, RpcError>);
#[derive(Default)]
struct Inbox {
    replies: VecDeque<Reply>,
    notifications: VecDeque<Value>,
    bytes: usize,
    dropped: u64,
}
struct Router<T: CaptureTransport> {
    wire: T,
    queues: VecDeque<(String, Request)>,
    inboxes: BTreeMap<String, Inbox>,
    pending: Option<(u64, String, u64)>,
    next: u64,
    terminal: Option<RpcError>,
    max_records: usize,
    max_bytes: usize,
}
impl<T: CaptureTransport> Router<T> {
    fn new(wire: T, config: &PollConfig) -> Self {
        let mut s = Self {
            wire,
            queues: VecDeque::new(),
            inboxes: BTreeMap::new(),
            pending: None,
            next: 0,
            terminal: None,
            max_records: config.max_records,
            max_bytes: config.max_bytes,
        };
        s.inboxes.insert(String::new(), Inbox::default());
        s
    }
    fn pump(&mut self) {
        let dropped = self.wire.take_dropped_notifications();
        if let Some(i) = self.inboxes.get_mut("") {
            i.dropped += dropped;
        }
        for _ in 0..4096 {
            let Some(n) = self.wire.take_notification() else {
                break;
            };
            let route = "";
            if let Some(inbox) = self.inboxes.get_mut(route) {
                inbox.bytes += n.to_string().len();
                inbox.notifications.push_back(n);
                while inbox.notifications.len() > self.max_records || inbox.bytes > self.max_bytes {
                    if let Some(old) = inbox.notifications.pop_front() {
                        inbox.bytes -= old.to_string().len();
                        inbox.dropped += 1;
                    }
                }
            }
        }
        while let Some((id, result)) = self.wire.take_reply() {
            if id == 0 {
                if let Err(e) = result {
                    self.terminal = Some(e);
                }
                continue;
            }
            if self.pending.as_ref().is_none_or(|p| p.0 != id) {
                continue;
            }
            let (_, route, local) = self.pending.take().unwrap();
            if matches!(
                result,
                Err(RpcError::Transport(_) | RpcError::InvalidMessage)
            ) {
                self.terminal = result.err();
                break;
            }
            if let Some(inbox) = self.inboxes.get_mut(&route) {
                inbox.replies.push_back((local, result));
            }
        }
    }
    fn flush(&mut self) {
        if self.pending.is_some() || self.terminal.is_some() {
            return;
        }
        if let Some((route, mut r)) = self.queues.pop_front() {
            self.next += 1;
            let local = r.id;
            r.id = self.next;
            match self.wire.submit(r) {
                Ok(()) => self.pending = Some((self.next, route, local)),
                Err(e) => self.terminal = Some(e),
            }
        }
    }
    fn retire(&mut self, route: &str) {
        self.queues.retain(|(key, _)| key != route);
        self.inboxes.remove(route);
        // Keep the wire correlation until the worker's cancellation reply. A late
        // response cannot be routed into a replacement isolate's inbox.
        if let Some(p) = self.pending.as_mut()
            && p.1 == route
        {
            p.1 = "\0retired".into();
        }
    }
}
struct Routed<T: CaptureTransport> {
    router: Rc<RefCell<Router<T>>>,
    route: String,
}
impl<T: CaptureTransport> CaptureTransport for Routed<T> {
    fn submit(&mut self, r: Request) -> Result<(), RpcError> {
        let mut router = self.router.borrow_mut();
        if let Some(e) = &router.terminal {
            return Err(e.clone());
        }
        if self.route.is_empty() {
            router.queues.push_front((self.route.clone(), r));
        } else {
            router.queues.push_back((self.route.clone(), r));
        }
        Ok(())
    }
    fn take_reply(&mut self) -> Option<Reply> {
        self.router
            .borrow_mut()
            .inboxes
            .get_mut(&self.route)?
            .replies
            .pop_front()
    }
    fn take_notification(&mut self) -> Option<Value> {
        let mut router = self.router.borrow_mut();
        let inbox = router.inboxes.get_mut(&self.route)?;
        let n = inbox.notifications.pop_front()?;
        inbox.bytes -= n.to_string().len();
        Some(n)
    }
    fn take_dropped_notifications(&mut self) -> u64 {
        self.router
            .borrow_mut()
            .inboxes
            .get_mut(&self.route)
            .map(|i| std::mem::take(&mut i.dropped))
            .unwrap_or(0)
    }
    fn discard_notifications(&mut self) {
        if let Some(i) = self.router.borrow_mut().inboxes.get_mut(&self.route) {
            i.notifications.clear();
            i.bytes = 0;
        }
    }
    fn stop(&mut self) {
        self.router.borrow_mut().retire(&self.route);
    }
}
enum Stage {
    Idle,
    Vm,
    Isolate(String),
    Enable(String, Value),
}
struct Attempt<T> {
    cancel: ConnectionCancel,
    receiver: mpsc::Receiver<Result<T, RpcError>>,
}
pub struct Coordinator<F: ConnectionFactory> {
    factory: F,
    config: CoordinatorConfig,
    store: CaptureStore,
    endpoint: Option<String>,
    wanted: bool,
    attempt: Option<Attempt<F::Transport>>,
    router: Option<Rc<RefCell<Router<F::Transport>>>>,
    control: Option<Routed<F::Transport>>,
    stage: Stage,
    control_id: u64,
    retry_due: Option<u64>,
    discover_due: u64,
    todo: VecDeque<String>,
    engines: BTreeMap<String, CaptureEngine<Routed<F::Transport>>>,
    names: BTreeMap<String, String>,
    exited: BTreeSet<String>,
    cursors: BTreeMap<String, u64>,
    session: Value,
    vm: Value,
    last_error: Option<RpcError>,
    refresh: bool,
    clear_baselines: BTreeSet<String>,
    console: VecDeque<(Value, usize)>,
    console_bytes: usize,
    early_console: VecDeque<(Value, usize)>,
    early_bytes: usize,
    console_due: u64,
    console_interval: u64,
}
impl<F: ConnectionFactory> Coordinator<F> {
    pub fn new(factory: F, config: CoordinatorConfig) -> Self {
        Self {
            store: CaptureStore::new(config.poll.max_records, config.poll.max_bytes),
            factory,
            config: config.clone(),
            endpoint: None,
            wanted: false,
            attempt: None,
            router: None,
            control: None,
            stage: Stage::Idle,
            control_id: 0,
            retry_due: None,
            discover_due: 0,
            todo: VecDeque::new(),
            engines: BTreeMap::new(),
            names: BTreeMap::new(),
            exited: BTreeSet::new(),
            cursors: BTreeMap::new(),
            session: Value::Null,
            vm: Value::Null,
            last_error: None,
            refresh: false,
            clear_baselines: BTreeSet::new(),
            console: VecDeque::new(),
            console_bytes: 0,
            early_console: VecDeque::new(),
            early_bytes: 0,
            console_due: 0,
            console_interval: config.poll.base_ms,
        }
    }
    pub(crate) fn vm_key(&self) -> Option<String> {
        (!self.vm.is_null()).then(|| {
            format!(
                "{}:{}:{}",
                self.vm["pid"],
                self.vm["startTime"],
                self.vm["operatingSystem"].as_str().unwrap_or("")
            )
        })
    }
    pub fn endpoint(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }
    pub(crate) fn swap_store(&mut self, store: &mut CaptureStore) {
        std::mem::swap(&mut self.store, store);
    }
    pub fn store(&self) -> &CaptureStore {
        &self.store
    }
    pub fn running(&self) -> bool {
        self.router.is_some() && !self.vm.is_null()
    }
    pub fn last_error(&self) -> Option<&RpcError> {
        self.last_error.as_ref()
    }
    pub fn is_online(&self, session: &str) -> bool {
        self.running() && !self.engines.is_empty() && self.session["id"] == session
    }
    pub fn connect(&mut self, uri: &str, now: u64) -> Result<(), RpcError> {
        let uri = match normalize_uri(uri) {
            Ok(v) => v,
            Err(e) => {
                self.last_error = Some(e.clone());
                return Err(e);
            }
        };
        if self.endpoint.as_deref() != Some(&uri) {
            self.stop();
            self.session = Value::Null;
            self.cursors.clear();
        } else {
            self.detach();
        }
        self.endpoint = Some(uri);
        self.wanted = true;
        self.last_error = None;
        self.begin(now);
        Ok(())
    }
    fn begin(&mut self, now: u64) {
        let cancel = ConnectionCancel::default();
        let receiver = self
            .factory
            .begin(self.endpoint.as_deref().unwrap(), cancel.clone());
        self.attempt = Some(Attempt { cancel, receiver });
        self.retry_due = None;
        self.discover_due = now;
    }
    fn retire(&mut self, id: &str, keep_cursor: bool) {
        if let Some(mut e) = self.engines.remove(id) {
            if keep_cursor {
                if let Some(n) = e.cursor() {
                    self.cursors.insert(id.into(), n);
                }
                if e.needs_clear_baseline() {
                    self.clear_baselines.insert(id.into());
                }
            }
            e.stop();
        }
        self.names.remove(id);
        if !keep_cursor {
            self.cursors.remove(id);
            self.clear_baselines.remove(id);
        }
    }
    fn detach(&mut self) {
        if let Some(a) = self.attempt.take() {
            a.cancel.cancel();
        }
        let ids = self.engines.keys().cloned().collect::<Vec<_>>();
        for id in ids {
            self.retire(&id, true);
        }
        if let Some(r) = self.router.take() {
            r.borrow_mut().wire.stop();
        }
        self.control = None;
        self.stage = Stage::Idle;
        self.todo.clear();
        self.exited.clear();
        self.vm = Value::Null;
        self.refresh = false;
        self.console.clear();
        self.console_bytes = 0;
        self.early_console.clear();
        self.early_bytes = 0;
    }
    pub fn stop(&mut self) {
        self.wanted = false;
        self.detach();
        self.retry_due = None;
        self.cursors.clear();
        self.clear_baselines.clear();
        if let Some(id) = self.session["id"].as_str() {
            self.store.remove_session(id);
        }
        self.session = Value::Null;
    }
    fn lost(&mut self, e: RpcError, now: u64) {
        self.last_error = Some(e);
        self.detach();
        if self.wanted {
            self.retry_due = Some(now.saturating_add(self.config.retry_ms));
        }
    }
    pub fn clear(&mut self, now: u64) {
        self.store.clear();
        self.discard_raw_console();
        self.console.clear();
        self.console_bytes = 0;
        self.early_console.clear();
        self.early_bytes = 0;
        for (id, n) in &mut self.cursors {
            if !self.engines.contains_key(id) {
                *n = n.saturating_add(1);
            }
        }
        for (id, e) in &mut self.engines {
            e.shared(&mut self.store, &self.session, |e| e.clear(now));
            if let Some(n) = e.cursor() {
                self.cursors.insert(id.clone(), n);
            }
            if e.needs_clear_baseline() {
                self.clear_baselines.insert(id.clone());
            }
        }
    }
    fn discard_raw_console(&mut self) {
        if let Some(router) = &self.router {
            let mut router = router.borrow_mut();
            // Preserve lifecycle events while discarding capture data at the boundary.
            router.pump();
            if let Some(inbox) = router.inboxes.get_mut("") {
                inbox
                    .notifications
                    .retain(|n| n["params"]["streamId"] == "Isolate");
                inbox.bytes = inbox
                    .notifications
                    .iter()
                    .map(|n| n.to_string().len())
                    .sum();
            }
        }
    }
    pub fn set_paused(&mut self, paused: bool) {
        if self.store.paused == paused {
            return;
        }
        self.discard_raw_console();
        self.console.clear();
        self.console_bytes = 0;
        self.early_console.clear();
        self.early_bytes = 0;
        for e in self.engines.values_mut() {
            self.store.paused = !paused;
            e.shared(&mut self.store, &self.session, |e| e.set_paused(paused));
        }
        self.store.paused = paused;
    }
    fn request(&mut self, method: &str, params: Value, stage: Stage) {
        self.control_id += 1;
        self.stage = stage;
        if let Err(e) = self.control.as_mut().unwrap().submit(Request {
            id: self.control_id,
            method: method.into(),
            params,
        }) {
            self.last_error = Some(e);
        }
    }
    fn discover_next(&mut self, now: u64) {
        if let Some(id) = self.todo.pop_front() {
            self.request("getIsolate", json!({"isolateId":id}), Stage::Isolate(id));
        } else {
            self.stage = Stage::Idle;
            self.discover_due = now.saturating_add(self.config.discovery_ms);
        }
    }
    fn metadata(&mut self) {
        if !self.session.is_object() {
            return;
        }
        self.session["isolates"] = json!(
            self.names
                .iter()
                .map(|(id, name)| json!({"id":id,"name":name}))
                .collect::<Vec<_>>()
        );
        self.store
            .ingest(&json!({"session":self.session,"sdk":{"schemaVersion":1},"events":[]}))
            .expect("valid app metadata");
    }
    fn control_reply(&mut self, result: Result<Value, RpcError>, now: u64) {
        let stage = std::mem::replace(&mut self.stage, Stage::Idle);
        let v = match result {
            Ok(v) => v,
            Err(e) => {
                self.last_error = Some(e.clone());
                // A profile/extension timeout never declares the transport lost.
                // VM health failure is retried on a new connection.
                if matches!(stage, Stage::Vm)
                    || matches!(e, RpcError::Transport(_) | RpcError::InvalidMessage)
                {
                    self.lost(e, now);
                } else {
                    self.discover_next(now);
                }
                return;
            }
        };
        match stage {
            Stage::Vm => {
                self.vm = v;
                let ids = self.vm["isolates"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .take(64)
                    .filter_map(|r| r["id"].as_str().map(str::to_owned))
                    .collect::<BTreeSet<_>>();
                let removed = self
                    .engines
                    .keys()
                    .filter(|id| !ids.contains(*id))
                    .cloned()
                    .collect::<Vec<_>>();
                for id in removed {
                    self.retire(&id, false);
                }
                self.exited.retain(|id| ids.contains(id));
                self.todo = ids
                    .into_iter()
                    .filter(|id| !self.engines.contains_key(id) && !self.exited.contains(id))
                    .collect();
                self.metadata();
                self.discover_next(now);
            }
            Stage::Isolate(id) => {
                if self.exited.contains(&id) {
                    self.discover_next(now);
                    return;
                }
                if !v["extensionRPCs"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|r| r == "ext.dart.io.getHttpProfile"))
                {
                    self.discover_next(now);
                    return;
                }
                self.request(
                    "ext.dart.io.httpEnableTimelineLogging",
                    json!({"isolateId":id,"enabled":"true"}),
                    Stage::Enable(id, v),
                );
            }
            Stage::Enable(id, detail) => {
                if self.exited.contains(&id) {
                    self.discover_next(now);
                    return;
                }
                let authority = url::Url::parse(self.endpoint.as_deref().unwrap())
                    .unwrap()
                    .authority()
                    .to_owned();
                let identity = json!([authority, self.vm["pid"], self.vm["startTime"]]).to_string();
                if self.session["id"].as_str() != Some(&identity) {
                    if let Some(old) = self.session["id"].as_str() {
                        self.store.remove_session(old);
                    }
                    self.cursors.clear();
                    self.clear_baselines.clear();
                    let root = detail["rootLib"]["uri"].as_str().unwrap_or("Flutter");
                    let app = root
                        .strip_prefix("package:")
                        .unwrap_or(root)
                        .split('/')
                        .next()
                        .unwrap_or("Flutter");
                    self.session = json!({"id":identity,"appName":app,"platform":self.vm["operatingSystem"],"deviceName":self.vm["operatingSystem"]});
                }
                let name = detail["name"].as_str().unwrap_or("main").to_owned();
                self.names.insert(id.clone(), name.clone());
                self.metadata();
                let router = self.router.as_ref().unwrap().clone();
                router
                    .borrow_mut()
                    .inboxes
                    .insert(id.clone(), Inbox::default());
                let mut e = CaptureEngine::new(
                    Routed {
                        router,
                        route: id.clone(),
                    },
                    self.session.clone(),
                    &id,
                    &name,
                    self.config.poll.clone(),
                );
                e.manage(
                    self.cursors.get(&id).copied(),
                    self.clear_baselines.contains(&id),
                );
                e.start(now);
                self.engines.insert(id, e);
                self.discover_next(now);
            }
            Stage::Idle => {}
        }
    }
    pub fn tick(&mut self, now: u64) {
        if !self.wanted {
            return;
        }
        if self.attempt.is_some() {
            let result = self.attempt.as_ref().unwrap().receiver.try_recv();
            match result {
                Ok(Ok(wire)) => {
                    self.attempt.take();
                    let router = Rc::new(RefCell::new(Router::new(wire, &self.config.poll)));
                    self.control = Some(Routed {
                        router: router.clone(),
                        route: String::new(),
                    });
                    self.router = Some(router);
                    self.console_interval = self.config.poll.base_ms;
                    self.console_due = now.saturating_add(self.console_interval);
                    self.request("getVM", json!({}), Stage::Vm);
                }
                Ok(Err(e)) => self.lost(e, now),
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.lost(RpcError::Transport("connection worker stopped".into()), now)
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if let Some(router) = self.router.clone() {
            router.borrow_mut().pump();
            let terminal = router.borrow_mut().terminal.take();
            if let Some(e) = terminal {
                self.lost(e, now);
                return;
            }
            let dropped = self.control.as_mut().unwrap().take_dropped_notifications();
            self.store.statistics.evicted += dropped;
            if dropped > 0 {
                for e in self.engines.values_mut() {
                    e.reset_console();
                }
            }
            while let Some(n) = self.control.as_mut().unwrap().take_notification() {
                let event = &n["params"]["event"];
                if n["params"]["streamId"] != "Isolate" {
                    if !self.store.paused {
                        let size = n.to_string().len();
                        self.early_bytes += size;
                        self.early_console.push_back((n, size));
                        while self.early_console.len() > self.config.poll.max_records
                            || self.early_bytes > self.config.poll.max_bytes
                        {
                            if let Some((_, size)) = self.early_console.pop_front() {
                                self.early_bytes -= size;
                                self.store.statistics.evicted += 1;
                            }
                        }
                    }
                    continue;
                }
                if let Some(id) = event["isolate"]["id"].as_str() {
                    match event["kind"].as_str() {
                        Some("IsolateExit") => {
                            self.retire(id, false);
                            self.exited.insert(id.into());
                        }
                        Some("IsolateStart") => {
                            self.retire(id, false);
                            self.exited.remove(id);
                        }
                        _ => {}
                    }
                    self.refresh = true;
                    self.metadata();
                }
            }
            if let Some((id, result)) = self.control.as_mut().unwrap().take_reply()
                && id == self.control_id
            {
                self.control_reply(result, now);
            }
            if self.router.is_none() {
                return;
            }
            // Subscriptions precede getVM/getIsolate. Retain their notifications
            // under the same bounded policy until the destination engine exists.
            // Replay once after bootstrap, before the console batch is committed.
            let pending = std::mem::take(&mut self.early_console);
            self.early_bytes = 0;
            for (n, size) in pending {
                let isolate = n["params"]["event"]["isolate"]["id"].as_str().unwrap_or("");
                if self.exited.contains(isolate) {
                    continue;
                }
                if let Some(e) = self.engines.get_mut(isolate) {
                    let mut events = Vec::new();
                    e.shared(&mut self.store, &self.session, |e| {
                        events = e.receive_console(n)
                    });
                    for event in events {
                        let size = event.to_string().len();
                        self.console_bytes += size;
                        self.console.push_back((event, size));
                        while self.console.len() > self.config.poll.max_records
                            || self.console_bytes > self.config.poll.max_bytes
                        {
                            if let Some((_, size)) = self.console.pop_front() {
                                self.console_bytes -= size;
                                self.store.statistics.evicted += 1;
                            }
                        }
                    }
                } else {
                    self.early_bytes += size;
                    self.early_console.push_back((n, size));
                }
            }
            if now >= self.console_due {
                let changed = !self.console.is_empty();
                let events = self
                    .console
                    .drain(..)
                    .map(|(event, _)| event)
                    .collect::<Vec<_>>();
                self.console_bytes = 0;
                if changed {
                    self.store.ingest(&json!({"session":self.session,"sdk":{"schemaVersion":1},"events":events})).expect("valid console envelope");
                }
                self.console_interval = if changed {
                    self.config.poll.base_ms
                } else {
                    self.console_interval
                        .saturating_add(self.config.poll.idle_increment_ms)
                        .min(self.config.poll.max_ms)
                };
                self.console_due = now.saturating_add(self.console_interval);
            }
            if matches!(self.stage, Stage::Idle) && (self.refresh || now >= self.discover_due) {
                self.refresh = false;
                self.request("getVM", json!({}), Stage::Vm);
            }
            // Bootstrap all isolates before beginning profile RPCs. Afterwards
            // each engine can consume replies while control requests are queued.
            if matches!(self.stage, Stage::Idle)
                || self.engines.values().any(|e| e.poll_in_flight())
            {
                let mut reattach = Vec::new();
                for (id, e) in &mut self.engines {
                    e.shared(&mut self.store, &self.session, |e| e.tick(now));
                    if let Some(n) = e.cursor() {
                        self.cursors.insert(id.clone(), n);
                    }
                    if !e.needs_clear_baseline() {
                        self.clear_baselines.remove(id);
                    }
                    if let Some(error) = e.take_last_error() {
                        if matches!(error, RpcError::Timeout) {
                            reattach.push(id.clone());
                        }
                        self.last_error = Some(error);
                    }
                }
                for id in reattach {
                    self.retire(&id, true);
                }
            }
            router.borrow_mut().flush();
        } else if self.attempt.is_none() && self.retry_due.is_some_and(|due| due <= now) {
            self.begin(now);
        }
    }
}
impl<F: ConnectionFactory> Drop for Coordinator<F> {
    fn drop(&mut self) {
        self.stop();
    }
}
