//! Form presentation metadata. Attachment bytes remain solely in the captured body.
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormBodyPart {
    pub name: String,
    pub value: Option<String>,
    pub filename: Option<String>,
    pub content_type: Option<String>,
    pub byte_length: usize,
}
impl FormBodyPart {
    pub fn is_file(&self) -> bool {
        self.filename.is_some()
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormBody {
    pub parts: Vec<FormBodyPart>,
    pub incomplete: bool,
}
impl FormBody {
    fn invalid() -> Self {
        Self {
            parts: vec![],
            incomplete: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OmittedBodyReason {
    SizeLimit { limit_bytes: u64 },
    ReadFailed,
}
/// Gives the UI a reason even when no body value was captured. Localization is
/// deliberately left to the desktop view.
pub fn omitted_body_reason(data: &Value) -> Option<OmittedBodyReason> {
    let reason = data.get("bodyOmitted")?.as_str()?;
    if reason.is_empty() {
        return None;
    }
    Some(if reason == "profile-size-limit" {
        OmittedBodyReason::SizeLimit {
            limit_bytes: data["bodyCaptureLimit"].as_u64().unwrap_or(0),
        }
    } else {
        OmittedBodyReason::ReadFailed
    })
}

pub fn body_content_type(headers: &Value) -> String {
    let Some(headers) = headers.as_object() else {
        return String::new();
    };
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| match v {
            Value::Array(a) => a.iter().map(display_value).collect::<Vec<_>>().join("; "),
            _ => display_value(v),
        })
        .unwrap_or_default()
}
fn display_value(v: &Value) -> String {
    v.as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| v.to_string())
}
pub fn is_form_body(content_type: &str) -> bool {
    let mime = content_type.split(';').next().unwrap_or("").trim();
    mime.eq_ignore_ascii_case("multipart/form-data")
        || mime.eq_ignore_ascii_case("application/x-www-form-urlencoded")
}

/// Parses the Flutter compute entry-point envelope: body, contentType, encoding.
/// Malformed URL forms discard all fields; truncated multipart retains parsed
/// fields and the final captured remainder, matching the source contract.
pub fn parse_form_body(args: &Value) -> FormBody {
    let (Some(raw), Some(content_type)) = (args["body"].as_str(), args["contentType"].as_str())
    else {
        return FormBody::invalid();
    };
    if !is_form_body(content_type) {
        return FormBody::invalid();
    }
    let Some(parameters) = parameters(content_type) else {
        return FormBody::invalid();
    };
    let decoded;
    let bytes = if args["encoding"] == "base64" {
        let Some(data) = decode_base64(raw) else {
            return FormBody::invalid();
        };
        decoded = data;
        decoded.as_slice()
    } else {
        raw.as_bytes()
    };
    let mime = content_type.split(';').next().unwrap_or("").trim();
    if mime.eq_ignore_ascii_case("application/x-www-form-urlencoded") {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return FormBody::invalid();
        };
        let mut form = FormBody {
            parts: vec![],
            incomplete: false,
        };
        for item in text.split('&').filter(|s| !s.is_empty()) {
            if form.parts.len() >= 1000 {
                form.incomplete = true;
                return form;
            }
            let (name, value) = item.split_once('=').unwrap_or((item, ""));
            let (Some(name), Some(value)) =
                (percent_decode(name, true), percent_decode(value, true))
            else {
                return FormBody::invalid();
            };
            form.parts.push(FormBodyPart {
                name,
                byte_length: value.len(),
                value: Some(value),
                filename: None,
                content_type: None,
            });
        }
        return form;
    }
    let Some(boundary) = parameters.get("boundary") else {
        return FormBody::invalid();
    };
    if boundary.is_empty()
        || boundary.encode_utf16().count() > 200
        || boundary.contains(['\r', '\n'])
    {
        return FormBody::invalid();
    }
    parse_multipart(bytes, format!("--{boundary}").as_bytes())
}

fn parse_multipart(bytes: &[u8], delimiter: &[u8]) -> FormBody {
    let starts =
        |at: usize, pattern: &[u8]| bytes.get(at..).is_some_and(|b| b.starts_with(pattern));
    let next_boundary = |from: usize| -> Option<usize> {
        (from..bytes.len()).find(|&i| {
            if i != 0 && (i < 2 || !starts(i - 2, b"\r\n")) {
                return false;
            }
            if !starts(i, delimiter) {
                return false;
            }
            let end = i + delimiter.len();
            starts(end, b"\r\n")
                || (starts(end, b"--") && (end + 2 == bytes.len() || starts(end + 2, b"\r\n")))
        })
    };
    let mut form = FormBody {
        parts: vec![],
        incomplete: false,
    };
    let mut position = next_boundary(0);
    while let Some(pos) = position {
        let after = pos + delimiter.len();
        if starts(after, b"--") {
            return form;
        }
        let header_start = after + 2;
        let header_end = (header_start..bytes.len().min(header_start.saturating_add(65536)))
            .find(|&i| starts(i, b"\r\n\r\n"));
        let Some(header_end) = header_end else {
            form.incomplete = true;
            return form;
        };
        if form.parts.len() >= 1000 {
            form.incomplete = true;
            return form;
        }
        let header_text = String::from_utf8_lossy(&bytes[header_start..header_end]);
        let mut headers = HashMap::new();
        for line in header_text.split("\r\n") {
            if let Some((key, value)) = line.split_once(':')
                && !key.is_empty()
            {
                headers.insert(key.trim().to_ascii_lowercase(), value.trim());
            }
        }
        let start = header_end + 4;
        let next = next_boundary(start);
        let end = next.map(|n| n.saturating_sub(2)).unwrap_or(bytes.len());
        if end < start {
            form.incomplete = true;
            return form;
        }
        let disposition = parameters(headers.get("content-disposition").copied().unwrap_or(""))
            .unwrap_or_default();
        if let Some(name) = disposition.get("name") {
            let filename = disposition
                .get("filename*")
                .and_then(|s| extended_filename(s))
                .or_else(|| disposition.get("filename").cloned());
            let data = &bytes[start..end];
            let content_type = headers.get("content-type").map(|s| s.to_string());
            let charset = content_type
                .as_deref()
                .and_then(parameters)
                .and_then(|p| p.get("charset").cloned())
                .unwrap_or_default();
            let value = if filename.is_some() {
                None
            } else if charset.eq_ignore_ascii_case("iso-8859-1")
                || charset.eq_ignore_ascii_case("latin1")
            {
                Some(data.iter().map(|&b| char::from(b)).collect())
            } else {
                Some(String::from_utf8_lossy(data).into_owned())
            };
            form.parts.push(FormBodyPart {
                name: name.clone(),
                value,
                filename,
                content_type,
                byte_length: data.len(),
            });
        } else {
            form.incomplete = true;
        }
        if next.is_none() {
            form.incomplete = true;
            return form;
        }
        position = next;
    }
    form.incomplete = true;
    form
}

/// Semicolons inside quoted strings and backslash escapes do not split params.
fn parameters(text: &str) -> Option<HashMap<String, String>> {
    let mut segments = Vec::new();
    let (mut quoted, mut escaped, mut start) = (false, false, 0);
    for (i, c) in text.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if quoted && c == '\\' {
            escaped = true;
            continue;
        }
        if c == '"' {
            quoted = !quoted;
        }
        if c == ';' && !quoted {
            segments.push(&text[start..i]);
            start = i + 1;
        }
    }
    if quoted || escaped {
        return None;
    }
    segments.push(&text[start..]);
    let mut result = HashMap::new();
    for segment in segments {
        let Some((key, value)) = segment.trim().split_once('=') else {
            continue;
        };
        let key = key.trim().to_ascii_lowercase();
        let value = value.trim();
        let value = if let Some(inner) = value.strip_prefix('"') {
            let inner = inner.strip_suffix('"')?;
            let mut result = String::new();
            let mut chars = inner.chars();
            while let Some(c) = chars.next() {
                result.push(if c == '\\' { chars.next()? } else { c });
            }
            result
        } else {
            value.to_owned()
        };
        result.insert(key, value);
    }
    Some(result)
}
fn extended_filename(value: &str) -> Option<String> {
    let mut pieces = value.splitn(3, '\'');
    if !pieces.next()?.eq_ignore_ascii_case("utf-8") {
        return None;
    }
    pieces.next()?;
    percent_decode(pieces.next()?, false)
}
fn percent_decode(text: &str, plus: bool) -> Option<String> {
    let mut out = Vec::with_capacity(text.len());
    let mut iter = text.bytes();
    while let Some(b) = iter.next() {
        out.push(match b {
            b'%' => hex(iter.next()?)? * 16 + hex(iter.next()?)?,
            b'+' if plus => b' ',
            _ => b,
        });
    }
    String::from_utf8(out).ok()
}
fn hex(b: u8) -> Option<u8> {
    (b as char).to_digit(16).map(|n| n as u8)
}

pub(crate) fn decode_base64(raw: &str) -> Option<Vec<u8>> {
    // Dart accepts URL-safe alphabet and normalized missing padding.
    let normalized = percent_decode(raw, false)?;
    let data: Vec<_> = normalized
        .bytes()
        .filter(|b| !b.is_ascii_whitespace())
        .collect();
    let end = data.iter().position(|&b| b == b'=').unwrap_or(data.len());
    if data[end..].iter().any(|&b| b != b'=') || data.len() - end > 2 || end % 4 == 1 {
        return None;
    }
    if end != data.len() && (!data.len().is_multiple_of(4) || (4 - end % 4) % 4 != data.len() - end)
    {
        return None;
    }
    let mut out = Vec::with_capacity(end * 3 / 4);
    let (mut buffer, mut bits) = (0u32, 0);
    for &b in &data[..end] {
        let n = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        };
        buffer = (buffer << 6) | n as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buffer >> bits) as u8);
        }
        buffer &= (1 << bits) - 1;
    }
    if buffer != 0 {
        return None;
    }
    Some(out)
}
