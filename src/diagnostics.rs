//! Bounded metadata-only JSONL. No caller-controlled strings or captured data
//! enter this API. The owner drives batching; there are no detached timers.
use serde::Serialize;
use std::{
    collections::VecDeque,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::PathBuf,
};
const QUEUE_BYTES: usize = 128 * 1024;
pub const FILE_BYTES: usize = 512 * 1024;
#[derive(Clone, Copy, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    AppStarted,
    AppStopped,
    CaptureStarted {
        automatic: bool,
    },
    DiscoveryStopped,
    ConnectionAttempt,
    ConnectionFailed,
    ConnectionState {
        online: usize,
        busy: bool,
        searching: bool,
        failed: bool,
    },
    PauseChanged {
        paused: bool,
    },
    CaptureCleared,
}
pub struct Diagnostics {
    directory: PathBuf,
    max_file_bytes: usize,
    pending: VecDeque<Vec<u8>>,
    pending_bytes: usize,
    due: Option<u64>,
    stopped: bool,
}
impl Diagnostics {
    pub fn new(directory: PathBuf, max_file_bytes: usize) -> Self {
        Self {
            directory,
            max_file_bytes,
            pending: VecDeque::new(),
            pending_bytes: 0,
            due: None,
            stopped: false,
        }
    }
    pub fn desktop_directory() -> PathBuf {
        if let Some(p) = std::env::var_os("FNV_DIAGNOSTICS_DIR") {
            return p.into();
        }
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let root = if cfg!(target_os = "macos") {
            home.join("Library/Logs")
        } else if cfg!(windows) {
            std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
        } else {
            std::env::var_os("XDG_STATE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".local/state"))
        };
        root.join("Flutter Network Viewer")
    }
    pub fn path(&self) -> PathBuf {
        self.directory.join("viewer.log")
    }
    pub fn pending_bytes(&self) -> usize {
        self.pending_bytes
    }
    pub fn record(&mut self, event: Event, now_ms: u64) {
        if self.stopped {
            return;
        }
        let mut value = serde_json::to_value(event).expect("fixed metadata serializes");
        value["time"] = chrono::Utc::now()
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
            .into();
        value["pid"] = std::process::id().into();
        let mut line = serde_json::to_vec(&value).expect("fixed metadata serializes");
        line.push(b'\n');
        if line.len() > self.max_file_bytes || line.len() > QUEUE_BYTES {
            return;
        }
        self.pending_bytes += line.len();
        self.pending.push_back(line);
        while self.pending_bytes > QUEUE_BYTES {
            if let Some(old) = self.pending.pop_front() {
                self.pending_bytes -= old.len();
            }
        }
        self.due.get_or_insert(now_ms.saturating_add(250));
    }
    pub fn poll(&mut self, now_ms: u64) {
        if !self.stopped && self.due.is_some_and(|d| now_ms >= d) {
            self.flush();
        }
    }
    pub fn flush(&mut self) {
        self.due = None;
        if !self.pending.is_empty() {
            let _ = self.write_pending();
        }
        // Disk failures discard a bounded batch, rather than accumulate/replay it.
        self.pending.clear();
        self.pending_bytes = 0;
    }
    fn write_pending(&self) -> io::Result<()> {
        fs::create_dir_all(&self.directory)?;
        let path = self.path();
        let mut length = match fs::metadata(&path) {
            Ok(m) => m.len(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => 0,
            Err(e) => return Err(e),
        };
        let mut file = None;
        for line in &self.pending {
            if length.saturating_add(line.len() as u64) > self.max_file_bytes as u64 {
                drop(file.take());
                let oldest = self.directory.join("viewer.log.2");
                let previous = self.directory.join("viewer.log.1");
                if oldest.exists() {
                    fs::remove_file(&oldest)?;
                }
                if previous.exists() {
                    fs::rename(&previous, &oldest)?;
                }
                if path.exists() {
                    fs::rename(&path, &previous)?;
                }
                length = 0;
            }
            if file.is_none() {
                file = Some(OpenOptions::new().create(true).append(true).open(&path)?);
            }
            file.as_mut().unwrap().write_all(line)?;
            length += line.len() as u64;
        }
        Ok(())
    }
    pub fn shutdown(&mut self) {
        self.flush();
        self.stopped = true;
    }
}
impl Drop for Diagnostics {
    fn drop(&mut self) {
        self.shutdown();
    }
}
