//! Single-isolate capture lifecycle. Drive `tick` with a monotonic millisecond
//! clock; socket I/O runs separately through CaptureTransport. Reconnect belongs
//! to the session coordinator, not the polling state machine.
use crate::{
    capture::CaptureStore,
    vm::{self, RpcError},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use indexmap::IndexMap;
use serde_json::{Value, json};
use std::collections::{HashMap, VecDeque};

#[derive(Clone, Debug)]
pub struct Request {
    pub id: u64,
    pub method: String,
    pub params: Value,
}
/// Implementations must never block these methods. There is at most one RPC
/// outstanding. Notifications must be pumped even while that RPC is pending.
pub trait CaptureTransport {
    fn submit(&mut self, request: Request) -> Result<(), RpcError>;
    fn take_reply(&mut self) -> Option<(u64, Result<Value, RpcError>)>;
    fn take_notification(&mut self) -> Option<Value>;
    fn discard_notifications(&mut self);
    /// Number of raw stream messages evicted since the previous call.
    fn take_dropped_notifications(&mut self) -> u64 {
        0
    }
    fn stop(&mut self);
}
#[derive(Clone, Debug)]
pub struct PollConfig {
    pub base_ms: u64,
    pub idle_increment_ms: u64,
    pub max_ms: u64,
    pub max_records: usize,
    pub max_bytes: usize,
}
impl Default for PollConfig {
    fn default() -> Self {
        Self {
            base_ms: 1400,
            idle_increment_ms: 200,
            max_ms: 5000,
            max_records: 5000,
            max_bytes: 64 * 1024 * 1024,
        }
    }
}
struct Pending {
    id: u64,
    revision: u64,
    capture_epoch: u64,
    summary: Option<Value>,
}
pub struct CaptureEngine<T: CaptureTransport> {
    transport: T,
    store: CaptureStore,
    session: Value,
    isolate: String,
    isolate_name: String,
    config: PollConfig,
    managed: bool,
    baseline_after_clear: bool,
    running: bool,
    stopped: bool,
    network_due: Option<u64>,
    console_due: Option<u64>,
    network_interval: u64,
    console_interval: u64,
    next_id: u64,
    pending: Option<Pending>,
    revision: u64,
    capture_epoch: u64,
    cursor: Option<u64>,
    summaries: VecDeque<Value>,
    fingerprints: IndexMap<String, String>,
    fingerprint_bytes: usize,
    changed: bool,
    console: VecDeque<(Value, usize)>,
    console_bytes: usize,
    buffers: HashMap<String, Vec<u8>>,
    batches: u64,
    last_error: Option<RpcError>,
}
impl<T: CaptureTransport> CaptureEngine<T> {
    pub fn new(
        transport: T,
        session: Value,
        isolate: &str,
        isolate_name: &str,
        config: PollConfig,
    ) -> Self {
        assert!(config.base_ms > 0 && config.max_ms >= config.base_ms);
        assert!(session["id"].as_str().is_some_and(|id| !id.is_empty()));
        Self {
            transport,
            store: CaptureStore::new(config.max_records, config.max_bytes),
            session,
            isolate: isolate.into(),
            isolate_name: isolate_name.into(),
            network_interval: config.base_ms,
            console_interval: config.base_ms,
            config,
            managed: false,
            baseline_after_clear: false,
            running: false,
            stopped: false,
            network_due: None,
            console_due: None,
            next_id: 0,
            pending: None,
            revision: 0,
            capture_epoch: 0,
            cursor: None,
            summaries: VecDeque::new(),
            fingerprints: IndexMap::new(),
            fingerprint_bytes: 0,
            changed: false,
            console: VecDeque::new(),
            console_bytes: 0,
            buffers: HashMap::new(),
            batches: 0,
            last_error: None,
        }
    }
    /// Start once on an attached transport. Construct a new transport/engine after
    /// Stop; reconnect identity/cursor transfer is the F11 coordinator's concern.
    pub fn start(&mut self, now: u64) {
        if self.running || self.stopped {
            return;
        }
        self.running = true;
        self.network_interval = self.config.base_ms;
        self.console_interval = self.config.base_ms;
        self.network_due = Some(now);
        self.console_due = Some(now.saturating_add(self.config.base_ms));
    }
    pub(crate) fn manage(&mut self, cursor: Option<u64>, baseline: bool) {
        self.managed = true;
        self.baseline_after_clear = baseline;
        self.cursor = cursor;
    }
    pub(crate) fn needs_clear_baseline(&self) -> bool {
        self.baseline_after_clear
    }
    pub(crate) fn receive_console(&mut self, notification: Value) -> Vec<Value> {
        self.notification(notification);
        self.console_bytes = 0;
        self.console.drain(..).map(|(event, _)| event).collect()
    }
    pub(crate) fn reset_console(&mut self) {
        self.discard_console();
    }
    pub(crate) fn shared(
        &mut self,
        store: &mut CaptureStore,
        session: &Value,
        f: impl FnOnce(&mut Self),
    ) {
        self.session = session.clone();
        std::mem::swap(&mut self.store, store);
        f(self);
        std::mem::swap(&mut self.store, store);
    }
    pub fn poll_in_flight(&self) -> bool {
        self.pending.is_some()
    }
    pub fn running(&self) -> bool {
        self.running
    }
    pub fn store(&self) -> &CaptureStore {
        &self.store
    }
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }
    pub fn network_interval(&self) -> u64 {
        self.network_interval
    }
    pub fn console_interval(&self) -> u64 {
        self.console_interval
    }
    pub fn pending_console_len(&self) -> usize {
        self.console.len()
    }
    pub fn pending_console_bytes(&self) -> usize {
        self.console_bytes
    }
    pub fn console_batches(&self) -> u64 {
        self.batches
    }
    pub fn cursor(&self) -> Option<u64> {
        self.cursor
    }
    pub(crate) fn take_last_error(&mut self) -> Option<RpcError> {
        self.last_error.take()
    }
    pub fn last_error(&self) -> Option<&RpcError> {
        self.last_error.as_ref()
    }
    fn discard_console(&mut self) {
        self.console.clear();
        self.console_bytes = 0;
        self.buffers.clear();
        self.transport.discard_notifications();
    }
    pub fn clear(&mut self, _now: u64) {
        self.transport.take_dropped_notifications();
        if !self.managed {
            self.store.clear();
        }
        self.baseline_after_clear |= self.cursor.is_none();
        self.revision += 1;
        self.capture_epoch += 1;
        self.discard_console();
        self.summaries.clear();
        self.fingerprints.clear();
        self.fingerprint_bytes = 0;
        self.cursor = self.cursor.map(|n| n.saturating_add(1));
    }
    pub fn set_paused(&mut self, paused: bool) {
        if self.store.paused == paused {
            return;
        }
        // Process queued notifications under the old pause state, then discard
        // pending/partial output. This also counts lines received while paused.
        let dropped = self.transport.take_dropped_notifications();
        self.store.statistics.evicted += dropped;
        if dropped > 0 {
            self.buffers.clear();
        }
        for _ in 0..4096 {
            let Some(n) = self.transport.take_notification() else {
                break;
            };
            self.notification(n);
        }
        self.store.paused = paused;
        self.capture_epoch += 1;
        self.discard_console();
        if paused {
            self.store.statistics.ignored_while_paused += self.summaries.len() as u64;
            self.summaries.clear();
        }
    }
    pub fn stop(&mut self) {
        self.running = false;
        self.stopped = true;
        if !self.managed {
            self.cursor = None;
        }
        self.fingerprints.clear();
        self.fingerprint_bytes = 0;
        if !self.managed
            && let Some(id) = self.session["id"].as_str()
        {
            self.store.remove_session(id);
        }
        self.network_due = None;
        self.console_due = None;
        self.pending = None;
        self.summaries.clear();
        self.discard_console();
        self.transport.stop();
    }
    fn next_interval(&self, current: u64, changed: bool) -> u64 {
        if changed {
            self.config.base_ms
        } else {
            current
                .saturating_add(self.config.idle_increment_ms)
                .min(self.config.max_ms)
        }
    }
    fn ingest(&mut self, events: Vec<Value>) {
        if events.is_empty() {
            return;
        }
        self.store
            .ingest(&json!({"session":self.session,"sdk":{"schemaVersion":1},"events":events}))
            .expect("valid capture session");
    }
    fn finish(&mut self, now: u64) {
        self.network_interval = self.next_interval(self.network_interval, self.changed);
        self.network_due = Some(now.saturating_add(self.network_interval));
        self.changed = false;
    }
    fn submit(&mut self, method: &str, params: Value, summary: Option<Value>) {
        self.next_id += 1;
        let id = self.next_id;
        match self.transport.submit(Request {
            id,
            method: method.into(),
            params,
        }) {
            Ok(()) => {
                self.pending = Some(Pending {
                    id,
                    revision: self.revision,
                    capture_epoch: self.capture_epoch,
                    summary,
                })
            }
            Err(e) => self.fail(e),
        }
    }
    fn fail(&mut self, error: RpcError) {
        self.last_error = Some(error);
        self.stop();
    }
    fn advance(&mut self, now: u64) {
        while self.running && self.pending.is_none() {
            let Some(summary) = self.summaries.pop_front() else {
                self.finish(now);
                break;
            };
            if self.store.paused {
                self.store.statistics.ignored_while_paused += 1;
                continue;
            }
            if vm::exceeds_body_limit(&summary) {
                self.ingest_profile(vm::omit_bodies(&summary, "profile-size-limit"));
                continue;
            }
            self.submit(
                "ext.dart.io.getHttpProfileRequest",
                json!({"isolateId":self.isolate,"id":summary["id"]}),
                Some(summary),
            );
        }
    }
    fn ingest_profile(&mut self, profile: Value) {
        self.ingest(vm::scoped_events(
            &profile,
            &self.isolate,
            &self.isolate_name,
        ));
    }
    fn response(&mut self, id: u64, result: Result<Value, RpcError>, now: u64) {
        if id == 0 {
            if let Err(e) = result {
                self.fail(e);
            }
            return;
        }
        if self.pending.as_ref().is_none_or(|p| p.id != id) {
            return;
        }
        let pending = self.pending.take().unwrap();
        if let Err(ref e) = result {
            self.last_error = Some(e.clone());
            if !matches!(e, RpcError::Timeout | RpcError::Remote { .. }) {
                self.fail(e.clone());
                return;
            }
        }
        if let Some(summary) = pending.summary {
            if pending.revision == self.revision
                && pending.capture_epoch == self.capture_epoch
                && !self.store.paused
            {
                let full =
                    result.unwrap_or_else(|_| vm::omit_bodies(&summary, "profile-read-error"));
                self.ingest_profile(full);
            }
            self.advance(now);
            return;
        }
        match result {
            Ok(profile) => {
                let stale = pending.revision != self.revision || self.baseline_after_clear;
                self.baseline_after_clear = false;
                if let Some(n) = profile["timestamp"].as_u64() {
                    let n = n.saturating_add(u64::from(stale));
                    self.cursor = Some(self.cursor.unwrap_or(0).max(n));
                }
                if !stale {
                    let mut bytes = 0usize;
                    if let Some(requests) = profile["requests"].as_array() {
                        for summary in requests {
                            let Some(key) = summary["id"].as_str() else {
                                continue;
                            };
                            let fingerprint = summary.to_string();
                            if self.fingerprints.get(key) == Some(&fingerprint) {
                                continue;
                            }
                            self.changed = true;
                            if let Some(old) = self.fingerprints.shift_remove(key) {
                                self.fingerprint_bytes -= old.len();
                            }
                            self.fingerprint_bytes += fingerprint.len();
                            self.fingerprints.insert(key.into(), fingerprint);
                            while self.fingerprints.len()
                                > self.config.max_records.saturating_mul(2)
                                || self.fingerprint_bytes > self.config.max_bytes
                            {
                                if let Some((_, old)) = self.fingerprints.shift_remove_index(0) {
                                    self.fingerprint_bytes -= old.len();
                                } else {
                                    break;
                                }
                            }
                            if self.store.paused || pending.capture_epoch != self.capture_epoch {
                                self.store.statistics.ignored_while_paused += 1;
                                continue;
                            }
                            let size = summary.to_string().len();
                            bytes = bytes.saturating_add(size);
                            self.summaries.push_back(summary.clone());
                            while self.summaries.len() > self.config.max_records
                                || bytes > self.config.max_bytes
                            {
                                if let Some(old) = self.summaries.pop_front() {
                                    bytes -= old.to_string().len();
                                    self.store.statistics.evicted += 1;
                                } else {
                                    break;
                                }
                            }
                        }
                    }
                }
                self.advance(now);
            }
            Err(_) => self.finish(now),
        }
    }
    pub fn tick(&mut self, now: u64) {
        if !self.running {
            return;
        }
        let dropped = self.transport.take_dropped_notifications();
        self.store.statistics.evicted += dropped;
        if dropped > 0 {
            self.buffers.clear();
        }
        for _ in 0..4096 {
            let Some(n) = self.transport.take_notification() else {
                break;
            };
            self.notification(n);
        }
        while let Some((id, result)) = self.transport.take_reply() {
            self.response(id, result, now);
            if !self.running {
                return;
            }
        }
        if !self.managed && self.console_due.is_some_and(|due| due <= now) {
            let changed = !self.console.is_empty();
            let events = self.console.drain(..).map(|(v, _)| v).collect();
            self.console_bytes = 0;
            if changed {
                self.batches += 1;
                self.ingest(events);
            }
            self.console_interval = self.next_interval(self.console_interval, changed);
            self.console_due = Some(now.saturating_add(self.console_interval));
        }
        if self.pending.is_none() && self.network_due.is_some_and(|due| due <= now) {
            self.network_due = None;
            self.changed = false;
            self.submit("ext.dart.io.getHttpProfile",json!({"isolateId":self.isolate,"updatedSince":self.cursor.map(|n|n.to_string()).unwrap_or_else(||"0".into())}),None);
        }
    }
    fn queue(&mut self, event: Value) {
        if self.store.paused {
            self.store.statistics.ignored_while_paused += 1;
            return;
        }
        let bytes = event.to_string().len();
        self.console_bytes += bytes;
        self.console.push_back((event, bytes));
        while self.console.len() > self.config.max_records
            || self.console_bytes > self.config.max_bytes
        {
            if let Some((_, bytes)) = self.console.pop_front() {
                self.console_bytes -= bytes;
                self.store.statistics.evicted += 1;
            } else {
                break;
            }
        }
    }
    fn notification(&mut self, n: Value) {
        if n["method"] != "streamNotify" {
            return;
        }
        let params = &n["params"];
        let event = &params["event"];
        if event["isolate"]["id"] != self.isolate {
            return;
        }
        let stream = params["streamId"].as_str().unwrap_or("");
        let timestamp = event["timestamp"]
            .as_i64()
            .and_then(chrono::DateTime::from_timestamp_millis)
            .map(|d| d.to_rfc3339_opts(chrono::SecondsFormat::Millis, true));
        if matches!(stream, "Stdout" | "Stderr") {
            let Some(raw) = event["bytes"].as_str() else {
                return;
            };
            let Ok(bytes) = STANDARD.decode(raw) else {
                return;
            };
            let buffer = self.buffers.entry(stream.into()).or_default();
            let mut lines = Vec::new();
            // Limit partial line storage even for a huge single VM chunk.
            for b in bytes {
                if b == b'\n' {
                    lines.push(std::mem::take(buffer));
                } else {
                    buffer.push(b);
                    if buffer.len() > 65536 {
                        lines.push(std::mem::take(buffer));
                    }
                }
            }
            for line in lines {
                self.queue(json!({"source":if stream=="Stderr"{"dart.stderr"}else{"dart.stdout"},"level":if stream=="Stderr"{"error"}else{"log"},"message":String::from_utf8_lossy(&line),"args":[],"timestamp":timestamp,"isolateId":self.isolate,"isolateName":self.isolate_name}));
            }
        } else if stream == "Logging" {
            let record = &event["logRecord"];
            let severity = record["level"].as_i64().unwrap_or(0);
            let text = |v: &Value| {
                v.as_str()
                    .or_else(|| v["valueAsString"].as_str())
                    .unwrap_or("")
                    .to_owned()
            };
            let error = text(&record["error"]);
            let stack = text(&record["stackTrace"]);
            let has_error = !error.is_empty() && error != "null";
            let level = if has_error || severity >= 1000 {
                "error"
            } else if severity >= 900 {
                "warn"
            } else if severity >= 800 {
                "info"
            } else if severity > 0 {
                "debug"
            } else {
                "log"
            };
            self.queue(json!({"source":"dart.logging","level":level,"message":text(&record["message"]),"loggerName":text(&record["loggerName"]),"severity":severity,"error":error,"stackTrace":stack,"args":[],"timestamp":timestamp,"isolateId":self.isolate,"isolateName":self.isolate_name}));
        }
    }
}
impl<T: CaptureTransport> Drop for CaptureEngine<T> {
    fn drop(&mut self) {
        self.stop();
    }
}
