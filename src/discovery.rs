//! Local DDS discovery and owner-driven multi-application capture.
//! Process enumeration, HTTP and VM probes run off the UI thread. Drive tick
//! with monotonic milliseconds; cancelling a scan never stops established apps.
use crate::{
    capture::CaptureStore,
    coordinator::{ConnectionFactory, Coordinator, CoordinatorConfig},
    vm::{ConnectionCancel, RpcError, VmClient, normalize_uri},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub uri: String,
    pub name: String,
    pub project_key: String,
    pub vm_key: String,
}
fn failure(e: impl std::fmt::Display) -> RpcError {
    RpcError::Transport(e.to_string())
}
fn check(cancel: &ConnectionCancel) -> Result<(), RpcError> {
    if cancel.is_cancelled() {
        Err(failure("discovery cancelled"))
    } else {
        Ok(())
    }
}
fn local_uri(raw: &str) -> Result<Url, RpcError> {
    let authority = raw
        .split_once("://")
        .ok_or(RpcError::InvalidUri)?
        .1
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("");
    let port = authority
        .rsplit_once(':')
        .and_then(|(_, s)| s.parse::<u16>().ok())
        .filter(|p| *p > 0)
        .ok_or(RpcError::InvalidUri)?;
    let u = Url::parse(raw).map_err(|_| RpcError::InvalidUri)?;
    if !matches!(u.scheme(), "http" | "ws")
        || !matches!(u.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
        || !u.username().is_empty()
        || u.password().is_some()
        || u.fragment().is_some()
        || port == 0
        || raw.contains(['\\', '\r', '\n', '\t'])
    {
        return Err(RpcError::InvalidUri);
    }
    Ok(u)
}
/// Only authenticated URLs explicitly advertised by DDS processes, at most 24.
pub fn process_uris(processes: &str) -> Vec<String> {
    let dds = regress::Regex::new(r"(?:development-service|dds(?:_aot)?\.dart(?:\.snapshot)?)\b")
        .unwrap();
    let argument = regress::Regex::new(r#"--vm-service-uri(?:=|\s+)["']?([^\s"']+)"#).unwrap();
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for line in processes.lines().filter(|line| dds.find(line).is_some()) {
        for m in argument.find_iter(line) {
            let arg = &line[m.range()];
            let raw = arg["--vm-service-uri".len()..]
                .trim_start_matches('=')
                .trim_start()
                .trim_start_matches(['\'', '"']);
            if local_uri(raw).is_ok() && seen.insert(raw.to_owned()) {
                out.push(raw.to_owned());
                if out.len() == 24 {
                    return out;
                }
            }
        }
    }
    out
}
/// Inspect the first eight isolates, as Flutter's discovery does. HTTP-profile
/// extensions alone do not establish that this VM is a Flutter application.
pub fn target_from_vm(
    uri: &str,
    vm: &Value,
    isolates: &[Value],
    own_pid: u32,
    own_os: &str,
) -> Option<Target> {
    if vm["pid"].as_u64() == Some(own_pid as u64) && vm["operatingSystem"] == own_os {
        return None;
    }
    for iso in isolates.iter().take(8) {
        if !iso["extensionRPCs"].as_array().is_some_and(|a| {
            a.iter()
                .any(|r| r.as_str().is_some_and(|s| s.starts_with("ext.flutter.")))
        }) {
            continue;
        }
        let root = iso["rootLib"]["uri"].as_str().unwrap_or("");
        if root.starts_with("package:flutter_network_viewer/") {
            return None;
        }
        let name = root
            .strip_prefix("package:")
            .and_then(|s| s.split_once('/').map(|(p, _)| p))
            .unwrap_or_else(|| iso["name"].as_str().unwrap_or("Flutter app"));
        return Some(Target {
            uri: uri.into(),
            name: name.into(),
            project_key: if root.is_empty() {
                iso["name"].as_str().unwrap_or("Flutter app").into()
            } else {
                root.into()
            },
            vm_key: format!(
                "{}:{}:{}",
                vm["pid"],
                vm["startTime"],
                vm["operatingSystem"].as_str().unwrap_or("")
            ),
        });
    }
    None
}
fn remaining(start: Instant, timeout: Duration) -> Result<Duration, RpcError> {
    timeout
        .checked_sub(start.elapsed())
        .filter(|d| !d.is_zero())
        .ok_or(RpcError::Timeout)
}
/// HTTP headers only, bounded to 16KiB and one deadline. No remote redirects or
/// port scanning; retain percent-encoded auth paths when deriving /ws.
pub fn resolve_dds(
    source: &str,
    cancel: &ConnectionCancel,
    timeout: Duration,
) -> Result<String, RpcError> {
    let mut uri = local_uri(source)?;
    uri.set_scheme("http").map_err(|_| RpcError::InvalidUri)?;
    if uri.path().ends_with("/ws") {
        let path = uri.path()[..uri.path().len() - 2].to_owned();
        uri.set_path(&path);
    }
    for _ in 0..4 {
        check(cancel)?;
        let start = Instant::now();
        let host = uri.host_str().unwrap().trim_matches(['[', ']']);
        let addresses = (host, uri.port_or_known_default().unwrap())
            .to_socket_addrs()
            .map_err(failure)?;
        let mut socket = None;
        for addr in addresses {
            check(cancel)?;
            if let Ok(s) = TcpStream::connect_timeout(&addr, remaining(start, timeout)?) {
                socket = Some(s);
                break;
            }
        }
        let mut socket = socket.ok_or_else(|| failure("DDS TCP connection failed"))?;
        cancel.register(&socket)?;
        socket
            .set_read_timeout(Some(remaining(start, timeout)?))
            .map_err(failure)?;
        socket
            .set_write_timeout(Some(remaining(start, timeout)?))
            .map_err(failure)?;
        let path = &uri[url::Position::BeforePath..url::Position::AfterQuery];
        write!(
            socket,
            "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            uri.authority()
        )
        .map_err(failure)?;
        let mut reader = BufReader::new(socket);
        let mut bytes = Vec::new();
        while bytes.len() < 16384 {
            check(cancel)?;
            reader
                .get_ref()
                .set_read_timeout(Some(remaining(start, timeout)?))
                .map_err(failure)?;
            let available = reader.fill_buf().map_err(failure)?;
            if available.is_empty() {
                return Err(RpcError::InvalidMessage);
            }
            // Copy one byte at a time from the buffer to stop at the header boundary.
            let byte = available[0];
            reader.consume(1);
            bytes.push(byte);
            if bytes.ends_with(b"\r\n\r\n") {
                break;
            }
        }
        if !bytes.ends_with(b"\r\n\r\n") {
            return Err(RpcError::InvalidMessage);
        }
        let header = std::str::from_utf8(&bytes).map_err(|_| RpcError::InvalidMessage)?;
        let mut lines = header.split("\r\n");
        let status = lines
            .next()
            .and_then(|s| s.split_whitespace().nth(1))
            .and_then(|s| s.parse::<u16>().ok())
            .ok_or(RpcError::InvalidMessage)?;
        let location = lines
            .filter_map(|s| s.split_once(':'))
            .find(|(k, _)| k.eq_ignore_ascii_case("location"))
            .map(|(_, v)| v.trim());
        if !matches!(status, 301 | 302 | 303 | 307 | 308) || location.is_none() {
            return normalize_uri(uri.as_str());
        }
        let next = uri
            .join(location.unwrap())
            .map_err(|_| RpcError::InvalidUri)?;
        if let Some(nested) = next
            .query_pairs()
            .find(|(k, _)| k == "uri")
            .map(|(_, v)| v.into_owned())
        {
            local_uri(&nested)?;
            return normalize_uri(&nested);
        }
        local_uri(next.as_str())?;
        uri = next;
    }
    Err(failure("too many DDS redirects"))
}
/// Candidate failures are expected when processes exit during enumeration.
/// Enumeration failure is reported by ThreadDiscovery rather than hidden here.
pub fn discover_processes(
    processes: &str,
    cancel: &ConnectionCancel,
    timeout: Duration,
    own_pid: u32,
    own_os: &str,
) -> Result<Vec<Target>, RpcError> {
    let mut found = Vec::<Target>::new();
    for source in process_uris(processes) {
        check(cancel)?;
        let inspect = || -> Result<Option<Target>, RpcError> {
            let uri = resolve_dds(&source, cancel, timeout)?;
            check(cancel)?;
            let mut client = VmClient::connect_cancellable(&uri, timeout, cancel)?;
            let vm = client.call("getVM", json!({}))?;
            if vm["pid"].as_u64() == Some(own_pid as u64) && vm["operatingSystem"] == own_os {
                return Ok(None);
            }
            for id in vm["isolates"]
                .as_array()
                .into_iter()
                .flatten()
                .take(8)
                .filter_map(|r| r["id"].as_str())
            {
                check(cancel)?;
                let isolate = client.call("getIsolate", json!({"isolateId":id}))?;
                if isolate["extensionRPCs"].as_array().is_some_and(|a| {
                    a.iter()
                        .any(|r| r.as_str().is_some_and(|s| s.starts_with("ext.flutter.")))
                }) {
                    return Ok(target_from_vm(&uri, &vm, &[isolate], own_pid, own_os));
                }
            }
            Ok(None)
        };
        let result = inspect();
        cancel.close_registered();
        if let Ok(Some(t)) = result {
            if let Some(old) = found.iter_mut().find(|old| old.vm_key == t.vm_key) {
                *old = t;
            } else {
                found.push(t);
            }
        }
    }
    check(cancel)?;
    found.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(found)
}
pub trait DiscoveryFactory {
    fn begin(&mut self, cancel: ConnectionCancel) -> mpsc::Receiver<Result<Vec<Target>, RpcError>>;
}
/// Default process source is /bin/ps on Unix, CIM on Windows; no SDK required.
#[derive(Clone)]
pub struct ThreadDiscovery {
    pub timeout: Duration,
}
impl Default for ThreadDiscovery {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(2),
        }
    }
}
fn processes(cancel: &ConnectionCancel) -> Result<String, RpcError> {
    let mut command = if cfg!(windows) {
        let mut c = Command::new("powershell.exe");
        c.args(["-NoProfile","-NonInteractive","-Command",r#"Get-CimInstance Win32_Process | Where-Object { $_.Name -in @("dart.exe", "dartaotruntime.exe") } | Select-Object -ExpandProperty CommandLine"#]);
        c
    } else {
        let mut c = Command::new("/bin/ps");
        c.args(["-ww", "-axo", "command="]);
        c
    };
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(failure)?;
    let output = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = output
            .take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = tx.send(result);
    });
    let start = Instant::now();
    let result = loop {
        if cancel.is_cancelled() {
            break Err(failure("discovery cancelled"));
        }
        if start.elapsed() >= Duration::from_secs(5) {
            break Err(RpcError::Timeout);
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    break Err(failure("process list failed"));
                }
                break rx
                    .recv_timeout(Duration::from_secs(1))
                    .map_err(failure)
                    .and_then(|r| r.map_err(failure))
                    .and_then(|b| {
                        if b.len() > 4 * 1024 * 1024 {
                            Err(failure("process list too large"))
                        } else {
                            String::from_utf8(b).map_err(failure)
                        }
                    });
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(e) => break Err(failure(e)),
        }
    };
    let _ = child.kill();
    let _ = child.wait();
    result
}
impl DiscoveryFactory for ThreadDiscovery {
    fn begin(&mut self, cancel: ConnectionCancel) -> mpsc::Receiver<Result<Vec<Target>, RpcError>> {
        let (tx, rx) = mpsc::channel();
        let failure_tx = tx.clone();
        let timeout = self.timeout;
        if let Err(e) = thread::Builder::new()
            .name("vm-discovery".into())
            .spawn(move || {
                let result = processes(&cancel).and_then(|p| {
                    discover_processes(
                        &p,
                        &cancel,
                        timeout,
                        std::process::id(),
                        if cfg!(target_os = "macos") {
                            "macos"
                        } else {
                            std::env::consts::OS
                        },
                    )
                });
                let _ = tx.send(result);
            })
        {
            let _ = failure_tx.send(Err(failure(e)));
        }
        rx
    }
}
struct Scan {
    cancel: ConnectionCancel,
    rx: mpsc::Receiver<Result<Vec<Target>, RpcError>>,
}
struct App<F: ConnectionFactory> {
    coordinator: Coordinator<F>,
    key: Option<String>,
}
/// A single globally budgeted store, with independent connections and cursors.
/// Manual connect replaces all apps; explicit scan adds at most one new app.
pub struct AppDiscovery<F: ConnectionFactory + Clone, D: DiscoveryFactory> {
    factory: F,
    discovery: D,
    config: CoordinatorConfig,
    store: CaptureStore,
    apps: Vec<App<F>>,
    scan: Option<Scan>,
    searching: bool,
    targets: Vec<Target>,
    candidate: Option<usize>,
    endpoint: Option<String>,
    error: Option<RpcError>,
}
impl<F: ConnectionFactory + Clone, D: DiscoveryFactory> AppDiscovery<F, D> {
    pub fn new(factory: F, discovery: D, config: CoordinatorConfig) -> Self {
        Self {
            factory,
            discovery,
            store: CaptureStore::new(config.poll.max_records, config.poll.max_bytes),
            config,
            apps: vec![],
            scan: None,
            searching: false,
            targets: vec![],
            candidate: None,
            endpoint: None,
            error: None,
        }
    }
    pub fn store(&self) -> &CaptureStore {
        &self.store
    }
    pub fn searching(&self) -> bool {
        self.searching
    }
    pub fn busy(&self) -> bool {
        self.scan.is_some() || self.candidate.is_some()
    }
    pub fn targets(&self) -> &[Target] {
        &self.targets
    }
    pub fn endpoint(&self) -> Option<&str> {
        self.endpoint.as_deref()
    }
    pub fn last_error(&self) -> Option<&RpcError> {
        self.error.as_ref().or_else(|| {
            self.candidate
                .and_then(|i| self.apps[i].coordinator.last_error())
        })
    }
    pub fn connections(&self) -> usize {
        self.apps.iter().filter(|a| a.coordinator.running()).count()
    }
    pub fn is_online(&self, id: &str) -> bool {
        self.apps.iter().any(|a| a.coordinator.is_online(id))
    }
    fn shared(&mut self, index: usize, action: impl FnOnce(&mut Coordinator<F>)) {
        let c = &mut self.apps[index].coordinator;
        c.swap_store(&mut self.store);
        action(c);
        c.swap_store(&mut self.store);
    }
    fn begin_scan(&mut self) {
        let cancel = ConnectionCancel::default();
        let rx = self.discovery.begin(cancel.clone());
        self.scan = Some(Scan { cancel, rx });
    }
    pub fn start_scan(&mut self, _now: u64) {
        self.stop_discovery();
        self.searching = true;
        self.error = None;
        self.begin_scan();
    }
    pub fn stop_discovery(&mut self) {
        if let Some(s) = self.scan.take() {
            s.cancel.cancel();
        }
        self.searching = false;
        self.targets.clear();
        if let Some(i) = self.candidate.take() {
            self.shared(i, |c| c.stop());
            self.apps.remove(i);
        }
    }
    fn add(&mut self, uri: &str, key: Option<String>, now: u64) -> Result<(), RpcError> {
        let mut c = Coordinator::new(self.factory.clone(), self.config.clone());
        c.connect(uri, now)?;
        self.endpoint = c.endpoint().map(str::to_owned);
        self.apps.push(App {
            coordinator: c,
            key,
        });
        self.candidate = Some(self.apps.len() - 1);
        Ok(())
    }
    pub fn select(&mut self, key: &str, now: u64) -> bool {
        if !self.searching || self.candidate.is_some() {
            return false;
        }
        let Some(t) = self.targets.iter().find(|t| t.vm_key == key).cloned() else {
            return false;
        };
        if let Some(s) = self.scan.take() {
            s.cancel.cancel();
        }
        match self.add(&t.uri, Some(t.vm_key), now) {
            Ok(()) => {
                self.targets.clear();
                true
            }
            Err(e) => {
                self.error = Some(e);
                false
            }
        }
    }
    pub fn connect_manual(&mut self, uri: &str, now: u64) -> Result<(), RpcError> {
        self.stop();
        let uri = match normalize_uri(uri) {
            Ok(uri) => uri,
            Err(e) => {
                self.error = Some(e.clone());
                return Err(e);
            }
        };
        self.error = None;
        self.add(&uri, None, now)
    }
    pub fn stop(&mut self) {
        self.stop_discovery();
        for i in 0..self.apps.len() {
            self.shared(i, |c| c.stop());
        }
        self.apps.clear();
        self.store.clear();
    }
    pub fn clear(&mut self, now: u64) {
        for i in 0..self.apps.len() {
            self.shared(i, |c| c.clear(now));
        }
        self.store.clear();
    }
    pub fn set_paused(&mut self, paused: bool) {
        if self.store.paused == paused {
            return;
        }
        for i in 0..self.apps.len() {
            self.store.paused = !paused;
            self.shared(i, |c| c.set_paused(paused));
        }
        self.store.paused = paused;
    }
    pub fn tick(&mut self, now: u64) {
        for i in 0..self.apps.len() {
            self.shared(i, |c| c.tick(now));
            if let Some(key) = self.apps[i].coordinator.vm_key() {
                self.apps[i].key = Some(key);
            }
        }
        if let Some(i) = self.candidate {
            let c = &self.apps[i].coordinator;
            if self.store.sessions().keys().any(|id| c.is_online(id)) {
                self.candidate = None;
                self.searching = false;
                self.targets.clear();
                self.error = None;
            } else if self.searching
                && let Some(e) = c.last_error().cloned()
            {
                self.candidate = None;
                self.shared(i, |c| c.stop());
                self.apps.remove(i);
                self.targets.clear();
                self.error = Some(e);
                self.searching = false;
            }
        }
        if !self.searching || self.candidate.is_some() {
            return;
        }
        let result = self.scan.as_ref().and_then(|s| match s.rx.try_recv() {
            Ok(v) => Some(v),
            Err(mpsc::TryRecvError::Disconnected) => Some(Err(failure("discovery worker stopped"))),
            Err(mpsc::TryRecvError::Empty) => None,
        });
        if let Some(result) = result {
            self.scan = None;
            match result {
                Ok(found) => {
                    self.error = None;
                    let mut seen = BTreeSet::new();
                    self.targets = found
                        .into_iter()
                        .filter(|t| {
                            !self.apps.iter().any(|a| a.key.as_ref() == Some(&t.vm_key))
                                && seen.insert(t.vm_key.clone())
                        })
                        .collect();
                    self.targets.sort_by(|a, b| a.name.cmp(&b.name));
                    if self.targets.is_empty() {
                        self.searching = false;
                        self.error = Some(failure(
                            "No new Flutter projects found. Scan again when your app is running.",
                        ));
                    } else if self.targets.len() == 1 {
                        let key = self.targets[0].vm_key.clone();
                        self.select(&key, now);
                    }
                }
                Err(e) => {
                    self.error = Some(e);
                    self.searching = false;
                    self.targets.clear();
                }
            }
        }
    }
}
impl<F: ConnectionFactory + Clone, D: DiscoveryFactory> Drop for AppDiscovery<F, D> {
    fn drop(&mut self) {
        self.stop();
    }
}
