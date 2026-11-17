use std::{collections::VecDeque, fmt};

use indexmap::IndexMap;
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Clone, Debug, Serialize)]
pub struct DeviceSession {
    pub session: Value,
    pub sdk: Value,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkRecord {
    #[serde(skip)]
    pub request_revision: u64,
    #[serde(skip)]
    pub completion_revision: u64,
    pub session_id: String,
    pub request_id: String,
    pub request_event: Value,
    pub completion_event: Value,
}

impl NetworkRecord {
    fn new(session_id: &str, request_id: &str) -> Self {
        Self {
            request_revision: 0,
            completion_revision: 0,
            session_id: session_id.into(),
            request_id: request_id.into(),
            request_event: json!({}),
            completion_event: json!({}),
        }
    }

    pub fn pending(&self) -> bool {
        self.completion_event
            .as_object()
            .is_some_and(|v| v.is_empty())
    }

    pub fn status(&self) -> Option<i64> {
        self.completion_event["response"]["statusCode"]
            .as_f64()
            .map(|n| n as i64)
    }

    pub fn failed(&self) -> bool {
        self.completion_event["error"]
            .as_object()
            .is_some_and(|v| !v.is_empty())
            || self.status().is_some_and(|v| v >= 400)
    }

    /// Shell-safe POSIX command; binary bodies are decoded through stdin.
    pub fn curl(&self) -> String {
        fn quote(s: &str) -> String {
            format!("'{}'", s.replace('\'', "'\\''"))
        }
        fn text(v: &Value) -> String {
            v.as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| v.to_string())
        }
        let request = &self.request_event["request"];
        let mut parts = vec![
            "curl".into(),
            "-X".into(),
            request["method"].as_str().unwrap_or("—").into(),
            quote(request["url"].as_str().unwrap_or("(request not received)")),
        ];
        if let Some(headers) = request["headers"].as_object() {
            for (name, value) in headers {
                parts.extend(["-H".into(), quote(&format!("{name}: {}", text(value)))]);
            }
        }
        if request["bodyEncoding"] == "base64" {
            parts.extend(["--data-binary".into(), "@-".into()]);
            return format!(
                "printf %s {} | base64 --decode | {}",
                quote(&text(&request["body"])),
                parts.join(" ")
            );
        }
        if !request["body"].is_null() {
            parts.extend(["--data-raw".into(), quote(&text(&request["body"]))]);
        }
        parts.join(" ")
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsoleRecord {
    #[serde(skip)]
    pub id: u64,
    pub session_id: String,
    pub event: Value,
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Statistics {
    pub rejected: u64,
    pub evicted: u64,
    pub ignored_while_paused: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct EnvelopeError(pub &'static str);
impl fmt::Display for EnvelopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
impl std::error::Error for EnvelopeError {}

#[derive(Debug)]
enum Retained {
    Network((String, String)),
    Log(u64, usize),
}

pub struct CaptureStore {
    max_records: usize,
    max_bytes: usize,
    sessions: IndexMap<String, DeviceSession>,
    network: IndexMap<(String, String), NetworkRecord>,
    logs: VecDeque<ConsoleRecord>,
    retention: VecDeque<Retained>,
    record_bytes: IndexMap<(String, String), usize>,
    bytes: usize,
    next_log: u64,
    pub statistics: Statistics,
    pub paused: bool,
    pub revision: u64,
}

impl Default for CaptureStore {
    fn default() -> Self {
        Self::new(5000, 64 * 1024 * 1024)
    }
}

impl CaptureStore {
    pub fn new(max_records: usize, max_bytes: usize) -> Self {
        Self {
            max_records,
            max_bytes,
            sessions: IndexMap::new(),
            network: IndexMap::new(),
            logs: VecDeque::new(),
            retention: VecDeque::new(),
            record_bytes: IndexMap::new(),
            bytes: 0,
            next_log: 0,
            statistics: Statistics::default(),
            paused: false,
            revision: 0,
        }
    }
    pub fn sessions(&self) -> &IndexMap<String, DeviceSession> {
        &self.sessions
    }
    pub fn network(&self) -> &IndexMap<(String, String), NetworkRecord> {
        &self.network
    }
    pub fn logs(&self) -> &VecDeque<ConsoleRecord> {
        &self.logs
    }
    pub fn max_records(&self) -> usize {
        self.max_records
    }
    pub fn retained_bytes(&self) -> usize {
        self.bytes
    }

    /// Validate the complete envelope before changing state. Bad individual events
    /// are counted and skipped, so a malformed event cannot poison the batch.
    pub fn ingest(&mut self, payload: &Value) -> Result<String, EnvelopeError> {
        if !payload.is_object() {
            return Err(EnvelopeError("Expected an SDK envelope object."));
        }
        let id = payload["session"]["id"]
            .as_str()
            .filter(|s| !s.is_empty())
            .ok_or(EnvelopeError(
                "Envelope must contain session.id and events[].",
            ))?;
        let events = payload["events"].as_array().ok_or(EnvelopeError(
            "Envelope must contain session.id and events[].",
        ))?;
        if payload["sdk"]["schemaVersion"].as_f64() != Some(1.0) {
            return Err(EnvelopeError("Unsupported SDK schemaVersion; expected 1."));
        }
        self.sessions.insert(
            id.into(),
            DeviceSession {
                session: payload["session"].clone(),
                sdk: payload["sdk"].clone(),
            },
        );
        while self.sessions.len() > 100 {
            let unused = self
                .sessions
                .keys()
                .find(|key| {
                    key.as_str() != id
                        && !self.network.values().any(|r| &r.session_id == *key)
                        && !self.logs.iter().any(|r| &r.session_id == *key)
                })
                .cloned();
            match unused {
                Some(key) => {
                    self.sessions.shift_remove(&key);
                }
                None => break,
            }
        }
        for event in events {
            if self.paused {
                self.statistics.ignored_while_paused += 1;
                continue;
            }
            if !event.is_object() {
                self.statistics.rejected += 1;
                continue;
            }
            if matches!(
                event["source"].as_str(),
                Some("js.console" | "dart.stdout" | "dart.stderr" | "dart.logging")
            ) {
                let bytes = serde_json::to_vec(event).expect("JSON value").len();
                self.logs.push_back(ConsoleRecord {
                    id: self.next_log,
                    session_id: id.into(),
                    event: event.clone(),
                });
                self.retention
                    .push_back(Retained::Log(self.next_log, bytes));
                self.next_log += 1;
                self.bytes += bytes;
            } else if event["requestId"].is_string()
                && matches!(
                    event["phase"].as_str(),
                    Some("request" | "response" | "error" | "abort")
                )
                && (event["durationMs"].is_null() || event["durationMs"].is_number())
                && (event["response"]["statusCode"].is_null()
                    || event["response"]["statusCode"].is_number())
            {
                let request_id = event["requestId"].as_str().expect("validated");
                let key = (id.to_owned(), request_id.to_owned());
                let record = self.network.entry(key.clone()).or_insert_with(|| {
                    self.retention.push_back(Retained::Network(key.clone()));
                    NetworkRecord::new(id, request_id)
                });
                self.bytes -= self.record_bytes.get(&key).copied().unwrap_or(0);
                if event["phase"] == "request" {
                    record.request_revision += 1;
                    record.request_event = event.clone();
                } else {
                    record.completion_revision += 1;
                    record.completion_event = event.clone();
                }
                let bytes = serde_json::to_vec(record).expect("JSON record").len();
                self.record_bytes.insert(key, bytes);
                self.bytes += bytes;
            } else {
                self.statistics.rejected += 1;
            }
            self.trim();
        }
        Ok(id.into())
    }

    fn trim(&mut self) {
        while self.retention.len() > self.max_records || self.bytes > self.max_bytes {
            let Some(oldest) = self.retention.pop_front() else {
                break;
            };
            match oldest {
                Retained::Log(_, bytes) => {
                    self.logs.pop_front();
                    self.bytes -= bytes;
                }
                Retained::Network(key) => {
                    self.network.shift_remove(&key);
                    self.bytes -= self.record_bytes.shift_remove(&key).unwrap_or(0);
                }
            }
            self.statistics.evicted += 1;
        }
    }

    pub fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    pub fn clear(&mut self) {
        self.revision += 1;
        self.sessions.clear();
        self.network.clear();
        self.logs.clear();
        self.retention.clear();
        self.record_bytes.clear();
        self.bytes = 0;
        self.statistics = Statistics::default();
    }

    pub fn remove_session(&mut self, id: &str) {
        if self.sessions.shift_remove(id).is_none() {
            return;
        }
        self.revision += 1;
        self.network.retain(|_, r| r.session_id != id);
        self.logs.retain(|r| r.session_id != id);
        self.record_bytes
            .retain(|key, _| self.network.contains_key(key));
        self.retention.retain(|entry| match entry {
            Retained::Network(key) => self.network.contains_key(key),
            Retained::Log(id, _) => self.logs.iter().any(|r| r.id == *id),
        });
        self.bytes = self
            .retention
            .iter()
            .map(|entry| match entry {
                Retained::Network(key) => self.record_bytes[key],
                Retained::Log(_, bytes) => *bytes,
            })
            .sum();
    }

    pub fn export(&self) -> Value {
        self.export_at(&chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
    }

    /// Explicit time makes fixture comparisons deterministic.
    pub fn export_at(&self, exported_at: &str) -> Value {
        json!({
            "format": "flutter_network_viewer", "version": 1, "exportedAt": exported_at,
            "sessions": self.sessions.values().collect::<Vec<_>>(),
            "network": self.network.values().collect::<Vec<_>>(),
            "console": self.logs, "statistics": self.statistics,
        })
    }
}
