//! Console presentation and bounded block grouping. Capture events remain immutable.
use chrono::{DateTime, FixedOffset};
use serde_json::Value;

use crate::capture::ConsoleRecord;

fn text(value: &Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
}

/// Strip terminal CSI sequences and per-line Flutter prefixes for display only.
pub fn clean_message(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut cleaned = String::with_capacity(value.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && bytes.get(i + 1) == Some(&b'[') {
            let mut end = i + 2;
            while bytes.get(end).is_some_and(|b| (0x30..=0x3f).contains(b)) {
                end += 1;
            }
            while bytes.get(end).is_some_and(|b| (0x20..=0x2f).contains(b)) {
                end += 1;
            }
            if bytes.get(end).is_some_and(|b| (0x40..=0x7e).contains(b)) {
                i = end + 1;
                continue;
            }
        }
        let ch = value[i..].chars().next().expect("character boundary");
        cleaned.push(ch);
        i += ch.len_utf8();
    }
    cleaned
        .split('\n')
        .map(|line| {
            let line = line
                .strip_prefix("flutter:")
                .map(|s| s.strip_prefix(' ').unwrap_or(s))
                .unwrap_or(line);
            line.replace('\r', "")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

impl ConsoleRecord {
    pub fn level(&self) -> String {
        self.field("level", "log")
    }
    pub fn message(&self) -> String {
        self.field("message", "")
    }
    pub fn source(&self) -> String {
        self.field("source", "")
    }
    pub fn logger_name(&self) -> String {
        self.field("loggerName", "")
    }
    /// Numeric VM severity is retained independently of the supplied level.
    pub fn severity(&self) -> Option<f64> {
        self.event["severity"].as_f64()
    }
    fn field(&self, name: &str, default: &str) -> String {
        match &self.event[name] {
            Value::Null => default.into(),
            value => text(value),
        }
    }
    pub fn display_message(&self) -> String {
        clean_message(&self.message())
    }
    pub fn source_label(&self) -> String {
        match self.source().as_str() {
            "dart.stdout" => "stdout".into(),
            "dart.stderr" => "stderr".into(),
            "dart.logging" => "dart:developer".into(),
            source => source.into(),
        }
    }
    pub fn error(&self) -> Option<String> {
        self.detail("error")
    }
    pub fn stack_trace(&self) -> Option<String> {
        self.detail("stackTrace")
    }
    fn detail(&self, name: &str) -> Option<String> {
        if !self.event[name].is_null() {
            let value = text(&self.event[name]);
            if !value.is_empty() && value != "null" {
                return Some(value);
            }
        }
        self.event["args"]
            .as_array()?
            .iter()
            .find(|arg| arg["type"] == name)
            .map(|arg| text(&arg["value"]))
    }
    pub fn line_count(&self) -> usize {
        self.display_message().split('\n').count()
    }
    pub fn search_text(&self) -> String {
        let args = self
            .event
            .get("args")
            .filter(|v| !v.is_null())
            .cloned()
            .unwrap_or_else(|| Value::Array(vec![]));
        format!(
            "{} {} {} {} {} {}",
            self.display_message(),
            self.source_label(),
            self.logger_name(),
            self.error().unwrap_or_default(),
            self.stack_trace().unwrap_or_default(),
            args
        )
    }
    pub fn time(&self) -> Option<DateTime<FixedOffset>> {
        DateTime::parse_from_rfc3339(self.event["timestamp"].as_str()?).ok()
    }

    /// Group only complete JSON/Flutter exception blocks, with original events
    /// attached to the synthetic entry. Failed candidates consume no input.
    /// Limits match Dart: 200 records and 65,536 UTF-16 units including separators.
    pub fn grouped(records: &[Self]) -> Vec<Self> {
        Self::grouped_refs(&records.iter().collect::<Vec<_>>())
    }
    /// Borrow history so incremental desktop grouping never clones the input.
    pub fn grouped_refs(records: &[&Self]) -> Vec<Self> {
        let mut result = Vec::with_capacity(records.len());
        let mut i = 0;
        while i < records.len() {
            let first = records[i];
            let mut message = first.display_message();
            let start = message.trim();
            let json = start == "{" || start == "[";
            let exception = start.starts_with("══╡ EXCEPTION");
            let mut end = None;
            if (json || exception)
                && matches!(first.source().as_str(), "dart.stdout" | "dart.stderr")
            {
                let mut units = message.encode_utf16().count();
                let first_time = first.time();
                for (j, next) in records.iter().enumerate().skip(i + 1).take(199) {
                    if next.session_id != first.session_id
                        || next.event["source"] != first.event["source"]
                        || next.event["isolateId"] != first.event["isolateId"]
                        || first_time
                            .zip(next.time())
                            .is_some_and(|(a, b)| (b - a).num_milliseconds().unsigned_abs() > 2000)
                    {
                        break;
                    }
                    let display = next.display_message();
                    units += 1 + display.encode_utf16().count();
                    if units > 65536 {
                        break;
                    }
                    message.push('\n');
                    message.push_str(&display);
                    let trimmed = display.trim();
                    let complete = if exception {
                        trimmed.chars().count() >= 8 && trimmed.chars().all(|c| c == '═')
                    } else {
                        (trimmed.ends_with('}') || trimmed.ends_with(']'))
                            && serde_json::from_str::<Value>(&message)
                                .is_ok_and(|v| v.is_object() || v.is_array())
                    };
                    if complete {
                        end = Some(j);
                        break;
                    }
                }
            }
            if let Some(end) = end {
                let mut grouped = first.clone();
                grouped.event["message"] = Value::String(message);
                grouped.event["rawEvents"] =
                    Value::Array(records[i..=end].iter().map(|r| r.event.clone()).collect());
                if exception {
                    grouped.event["level"] = Value::String("error".into());
                }
                result.push(grouped);
                i = end + 1;
            } else {
                result.push(first.clone());
                i += 1;
            }
        }
        result
    }
}
