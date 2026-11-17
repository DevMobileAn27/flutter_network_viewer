//! Settings core. JSON files use the Flutter preference keys (column widths are
//! themselves a JSON string). VM URL normalization belongs to the VM adapter;
//! callers pass its normalized URL to `set_manual_vm_service_uri`.
//!
//! Missing or corrupt files restore defaults without rewriting the file. Other
//! read/write failures are returned. Writes replace the destination via a unique
//! sibling temporary file; the parent must already exist. This is a single-owner
//! store, not a multi-process merge or a power-loss durability guarantee.
use serde_json::{Map, Value, json};
use std::{
    fmt,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

const TABS: [&str; 6] = [
    "overview",
    "requestHeader",
    "requestBody",
    "responseHeader",
    "responseBody",
    "raw",
];
const COLUMN_MIN: [f64; 3] = [64.0, 56.0, 96.0];
const COLUMN_DEFAULT: [f64; 3] = [64.0, 56.0, 108.0];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeMode {
    Light,
    Dark,
    /// Accepted as an input only so unsupported system mode can be rejected.
    System,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SettingsValues {
    pub language_code: String,
    pub font_size: f32,
    pub theme_mode: ThemeMode,
    pub mcp_enabled: bool,
    pub mcp_port: i64,
    /// None lets the future GUI center the network split at its current size.
    pub network_inspector_width: Option<f64>,
    pub console_inspector_width: f64,
    pub json_input_fraction: f64,
    pub request_detail_tab: i64,
    pub network_column_widths: [f64; 3],
    pub vm_service_uri: String,
    pub auto_detect_vm_service: bool,
}

#[derive(Debug)]
pub enum SettingsError {
    InvalidValue(&'static str),
    Io(io::Error),
}
impl fmt::Display for SettingsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidValue(key) => write!(f, "Invalid setting: {key}"),
            Self::Io(error) => write!(f, "Could not persist or read settings: {error}"),
        }
    }
}
impl std::error::Error for SettingsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}
impl From<io::Error> for SettingsError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug)]
pub struct AppSettings {
    values: SettingsValues,
    preferences: Map<String, Value>,
    path: Option<PathBuf>,
}
impl Default for AppSettings {
    fn default() -> Self {
        Self::from_preferences(Value::Null)
    }
}
impl AppSettings {
    /// In-memory store, also useful for importing a Flutter preference map.
    /// Unknown keys are retained on subsequent writes.
    pub fn from_preferences(value: Value) -> Self {
        let preferences = value.as_object().cloned().unwrap_or_default();
        Self {
            values: restore(&preferences),
            preferences,
            path: None,
        }
    }

    pub fn load(path: impl AsRef<Path>) -> Result<Self, SettingsError> {
        let path = path.as_ref();
        let value = match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).unwrap_or(Value::Null),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Value::Null,
            Err(error) => return Err(error.into()),
        };
        let mut settings = Self::from_preferences(value);
        settings.path = Some(path.to_owned());
        Ok(settings)
    }

    pub fn values(&self) -> &SettingsValues {
        &self.values
    }

    // Layout changes update memory before I/O like Flutter. Other preferences
    // commit memory only after the file replacement succeeds. On failure the
    // committed preference map stays unchanged in both cases.
    fn update(&mut self, key: &str, value: Value) -> Result<(), SettingsError> {
        let mut next = self.preferences.clone();
        next.insert(key.into(), value);
        self.commit(next)
    }
    fn commit(&mut self, next: Map<String, Value>) -> Result<(), SettingsError> {
        let restored = restore(&next);
        if let Some(path) = &self.path {
            write_preferences(path, &next)?;
        }
        // Preserve any local layout whose earlier write failed when committing
        // an unrelated preference, just as local Flutter widget state does.
        self.values.language_code = restored.language_code;
        self.values.font_size = restored.font_size;
        self.values.theme_mode = restored.theme_mode;
        self.values.mcp_enabled = restored.mcp_enabled;
        self.values.mcp_port = restored.mcp_port;
        self.values.vm_service_uri = restored.vm_service_uri;
        self.values.auto_detect_vm_service = restored.auto_detect_vm_service;
        self.preferences = next;
        Ok(())
    }

    pub fn set_font_size(&mut self, size: f32) -> Result<(), SettingsError> {
        if !size.is_finite() || !(10.0..=24.0).contains(&size) {
            return Err(SettingsError::InvalidValue("fontSize"));
        }
        if self.values.font_size == size {
            return Ok(());
        }
        self.update("fontSize", json!(size))
    }
    pub fn set_language(&mut self, code: &str) -> Result<(), SettingsError> {
        if !matches!(code, "en" | "vi") {
            return Err(SettingsError::InvalidValue("languageCode"));
        }
        if self.values.language_code == code {
            return Ok(());
        }
        self.update("languageCode", json!(code))
    }
    pub fn set_theme_mode(&mut self, mode: ThemeMode) -> Result<(), SettingsError> {
        let name = match mode {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
            ThemeMode::System => return Err(SettingsError::InvalidValue("themeMode")),
        };
        if self.values.theme_mode == mode {
            return Ok(());
        }
        self.update("themeMode", json!(name))
    }
    /// Persist enablement and port together before restarting the service.
    pub fn set_mcp_config(&mut self, enabled: bool, port: i64) -> Result<(), SettingsError> {
        if !(1..=65535).contains(&port) {
            return Err(SettingsError::InvalidValue("mcpPort"));
        }
        let mut next = self.preferences.clone();
        next.insert("mcpEnabled".into(), json!(enabled));
        next.insert("mcpPort".into(), json!(port));
        self.commit(next)
    }
    pub fn set_mcp_enabled(&mut self, enabled: bool) -> Result<(), SettingsError> {
        if self.values.mcp_enabled == enabled {
            return Ok(());
        }
        self.update("mcpEnabled", json!(enabled))
    }
    pub fn set_mcp_port(&mut self, port: i64) -> Result<(), SettingsError> {
        if !(1..=65535).contains(&port) {
            return Err(SettingsError::InvalidValue("mcpPort"));
        }
        if self.values.mcp_port == port {
            return Ok(());
        }
        self.update("mcpPort", json!(port))
    }
    /// Persists an adapter-normalized URL and disables automatic discovery in
    /// one file replacement. Does not start, stop, or normalize a VM connection.
    pub fn set_manual_vm_service_uri(&mut self, uri: &str) -> Result<(), SettingsError> {
        let mut next = self.preferences.clone();
        next.insert("vmServiceUri".into(), json!(uri));
        next.insert("autoDetectVmService".into(), json!(false));
        self.commit(next)
    }
    pub fn set_auto_detect_vm_service(&mut self, enabled: bool) -> Result<(), SettingsError> {
        if self.values.auto_detect_vm_service == enabled {
            return Ok(());
        }
        self.update("autoDetectVmService", json!(enabled))
    }
    pub fn set_inspector_width(&mut self, width: f64, console: bool) -> Result<(), SettingsError> {
        if valid_width(width).is_none() {
            return Err(SettingsError::InvalidValue("inspectorWidth"));
        }
        let key = if console {
            "consoleInspectorWidth"
        } else {
            "networkInspectorWidth"
        };
        let current = if console {
            Some(self.values.console_inspector_width)
        } else {
            self.values.network_inspector_width
        };
        if current == Some(width) {
            return Ok(());
        }
        self.update_layout(key, json!(width))
    }
    pub fn set_json_input_fraction(&mut self, fraction: f64) -> Result<(), SettingsError> {
        if !valid_fraction(fraction) {
            return Err(SettingsError::InvalidValue("jsonInputFraction"));
        }
        if self.values.json_input_fraction == fraction {
            return Ok(());
        }
        self.update_layout("jsonInputFraction", json!(fraction))
    }
    pub fn set_request_detail_tab(&mut self, index: i64) -> Result<(), SettingsError> {
        if !(0..6).contains(&index) {
            return Err(SettingsError::InvalidValue("requestDetailTab"));
        }
        if self.values.request_detail_tab == index {
            return Ok(());
        }
        self.update_layout("requestDetailTabName", json!(TABS[index as usize]))
    }
    pub fn set_network_column_widths(&mut self, widths: &[f64]) -> Result<(), SettingsError> {
        if !valid_columns(widths) {
            return Err(SettingsError::InvalidValue("networkColumnWidths"));
        }
        self.update_layout(
            "networkColumnWidths",
            json!(serde_json::to_string(widths).expect("finite columns")),
        )
    }
    fn update_layout(&mut self, key: &str, value: Value) -> Result<(), SettingsError> {
        // Change only the targeted layout field; failed previous writes must not
        // reset unrelated local split/column values on a later layout change.
        let mut local = self.preferences.clone();
        local.insert(key.into(), value.clone());
        let restored = restore(&local);
        match key {
            "networkInspectorWidth" => {
                self.values.network_inspector_width = restored.network_inspector_width
            }
            "consoleInspectorWidth" => {
                self.values.console_inspector_width = restored.console_inspector_width
            }
            "jsonInputFraction" => self.values.json_input_fraction = restored.json_input_fraction,
            "requestDetailTabName" => self.values.request_detail_tab = restored.request_detail_tab,
            "networkColumnWidths" => {
                self.values.network_column_widths = restored.network_column_widths
            }
            _ => unreachable!(),
        }
        // commit preserves layout values and commits the backing map only
        // when I/O succeeds.
        self.update(key, value)
    }
}

fn valid_width(value: f64) -> Option<f64> {
    (value.is_finite() && value >= 320.0).then_some(value)
}
fn valid_fraction(value: f64) -> bool {
    value.is_finite() && value > 0.0 && value < 1.0
}
fn valid_columns(widths: &[f64]) -> bool {
    widths.len() == 3
        && widths
            .iter()
            .zip(COLUMN_MIN)
            .all(|(&w, min)| w.is_finite() && w >= min && w <= 1600.0)
}
fn restore(p: &Map<String, Value>) -> SettingsValues {
    let string = |key| p.get(key).and_then(Value::as_str);
    let number = |key| p.get(key).and_then(Value::as_f64);
    let integer = |key| p.get(key).and_then(Value::as_i64);
    let boolean = |key| p.get(key).and_then(Value::as_bool);
    let tab = if let Some(name) = string("requestDetailTabName") {
        TABS.iter().position(|&t| t == name).unwrap_or(0) as i64
    } else {
        match integer("requestDetailTab") {
            Some(1) => 2,
            Some(2) => 4,
            Some(3) => 5,
            _ => 0,
        }
    };
    let columns = string("networkColumnWidths")
        .and_then(|s| serde_json::from_str::<Vec<f64>>(s).ok())
        .filter(|w| valid_columns(w))
        .map(|w| [w[0], w[1], w[2]])
        .unwrap_or(COLUMN_DEFAULT);
    SettingsValues {
        font_size: number("fontSize")
            .filter(|v| (10.0..=24.0).contains(v))
            .unwrap_or(14.0) as f32,
        language_code: if string("languageCode") == Some("vi") {
            "vi"
        } else {
            "en"
        }
        .into(),
        theme_mode: if string("themeMode") == Some("dark") {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        },
        mcp_enabled: boolean("mcpEnabled").unwrap_or(false),
        mcp_port: integer("mcpPort")
            .filter(|p| (1..=65535).contains(p))
            .unwrap_or(38943),
        network_inspector_width: number("networkInspectorWidth").and_then(valid_width),
        console_inspector_width: number("consoleInspectorWidth")
            .and_then(valid_width)
            .unwrap_or(400.0),
        json_input_fraction: number("jsonInputFraction")
            .filter(|&v| valid_fraction(v))
            .unwrap_or(0.5),
        request_detail_tab: tab,
        network_column_widths: columns,
        vm_service_uri: string("vmServiceUri").unwrap_or("").into(),
        auto_detect_vm_service: boolean("autoDetectVmService").unwrap_or(false),
    }
}

fn write_preferences(path: &Path, preferences: &Map<String, Value>) -> io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let bytes = serde_json::to_vec_pretty(preferences)?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let (temp, mut file) = loop {
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let temp = parent.join(format!(".fnv-settings-{}-{id}.tmp", std::process::id()));
        match OpenOptions::new().write(true).create_new(true).open(&temp) {
            Ok(file) => break (temp, file),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    };
    let result = (|| {
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temp, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
