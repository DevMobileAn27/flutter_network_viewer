//! Whole-value color parsing. CSS #RGBA differs from Flutter 0xAARRGGBB.
use crate::json::{JsonDocument, JsonPath, PathPart};
use serde_json::Value;

/// A string value's encoded JSON token, in UTF-16 document coordinates.
/// Swatches are separate metadata: never insert them into selectable/copy text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorPreview {
    pub path: JsonPath,
    pub start: usize,
    pub end: usize,
    pub value: String,
    pub argb: u32,
}

fn dart_space(c: char) -> bool {
    matches!(c, '\u{9}'..='\u{d}' | '\u{20}' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}
fn number(text: &str) -> Option<f64> {
    let n: f64 = text.parse().ok()?;
    n.is_finite().then_some(n)
}
fn channel(text: &str, scale: f64) -> Option<f64> {
    let n = if let Some(percent) = text.strip_suffix('%') {
        number(percent)? * scale / 100.0
    } else {
        number(text)?
    };
    (n >= 0.0 && n <= scale).then_some(n)
}
fn argb(alpha: f64, r: f64, g: f64, b: f64) -> u32 {
    ((alpha * 255.0).round() as u32) << 24
        | (r.round() as u32) << 16
        | (g.round() as u32) << 8
        | b.round() as u32
}
/// Match Flutter's supported names, separators, ranges and channel rounding.
/// Returns packed ARGB, or None without guessing IDs or substrings.
pub fn parse_body_color(value: &str) -> Option<u32> {
    let text = value.trim_matches(dart_space).to_lowercase();
    if let Some(hex) = text.strip_prefix('#') {
        if !hex.bytes().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let expanded;
        let hex = if matches!(hex.len(), 3 | 4) {
            expanded = hex.chars().flat_map(|c| [c, c]).collect::<String>();
            expanded.as_str()
        } else {
            hex
        };
        return match hex.len() {
            6 => u32::from_str_radix(hex, 16).ok().map(|n| n | 0xff000000),
            8 => u32::from_str_radix(hex, 16).ok().map(|n| n.rotate_right(8)),
            _ => None,
        };
    }
    if let Some(hex) = text.strip_prefix("0x") {
        return (hex.len() == 8 && hex.bytes().all(|c| c.is_ascii_hexdigit()))
            .then(|| u32::from_str_radix(hex, 16).ok())
            .flatten();
    }
    if let Some((name, body)) = text.split_once('(') {
        if !matches!(name, "rgb" | "rgba" | "hsl" | "hsla") {
            return None;
        }
        let body = body.strip_suffix(')')?;
        if body.contains(['(', ')']) {
            return None;
        }
        let parts: Vec<_> = body
            .split(|c| dart_space(c) || c == ',' || c == '/')
            .filter(|s| !s.is_empty())
            .collect();
        if !matches!(parts.len(), 3 | 4) {
            return None;
        }
        let alpha = if parts.len() == 4 {
            channel(parts[3], 1.0)?
        } else {
            1.0
        };
        if name.starts_with("rgb") {
            return Some(argb(
                alpha,
                channel(parts[0], 255.0)?,
                channel(parts[1], 255.0)?,
                channel(parts[2], 255.0)?,
            ));
        }
        let hue = number(parts[0].strip_suffix("deg").unwrap_or(parts[0]))?.rem_euclid(360.0);
        if !parts[1].ends_with('%') || !parts[2].ends_with('%') {
            return None;
        }
        let saturation = channel(parts[1], 1.0)?;
        let lightness = channel(parts[2], 1.0)?;
        let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
        let secondary = chroma * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
        let m = lightness - chroma / 2.0;
        let (r, g, b) = match hue {
            h if h < 60.0 => (chroma, secondary, 0.0),
            h if h < 120.0 => (secondary, chroma, 0.0),
            h if h < 180.0 => (0.0, chroma, secondary),
            h if h < 240.0 => (0.0, secondary, chroma),
            h if h < 300.0 => (secondary, 0.0, chroma),
            _ => (chroma, 0.0, secondary),
        };
        return Some(argb(
            alpha,
            (r + m) * 255.0,
            (g + m) * 255.0,
            (b + m) * 255.0,
        ));
    }
    Some(match text.as_str() {
        "black" => 0xff000000,
        "white" => 0xffffffff,
        "red" => 0xffff0000,
        "green" => 0xff008000,
        "blue" => 0xff0000ff,
        "yellow" => 0xffffff00,
        "gray" | "grey" => 0xff808080,
        "silver" => 0xffc0c0c0,
        "maroon" => 0xff800000,
        "purple" => 0xff800080,
        "fuchsia" => 0xffff00ff,
        "lime" => 0xff00ff00,
        "olive" => 0xff808000,
        "navy" => 0xff000080,
        "teal" => 0xff008080,
        "aqua" => 0xff00ffff,
        "orange" => 0xffffa500,
        "transparent" => 0,
        _ => return None,
    })
}

pub(crate) fn previews(document: &JsonDocument) -> Vec<ColorPreview> {
    fn visit(
        document: &JsonDocument,
        value: &Value,
        path: &mut JsonPath,
        result: &mut Vec<ColorPreview>,
    ) {
        match value {
            Value::String(value) => {
                if let Some(argb) = parse_body_color(value) {
                    let node = &document.presentation().nodes[path];
                    result.push(ColorPreview {
                        path: path.clone(),
                        start: node.value_start,
                        end: node.end,
                        value: value.clone(),
                        argb,
                    });
                }
            }
            Value::Array(values) => {
                for (i, value) in values.iter().enumerate() {
                    path.push(PathPart::Index(i));
                    visit(document, value, path, result);
                    path.pop();
                }
            }
            Value::Object(values) => {
                for (key, value) in values {
                    path.push(PathPart::Key(key.clone()));
                    visit(document, value, path, result);
                    path.pop();
                }
            }
            _ => {}
        }
    }
    let mut result = Vec::new();
    visit(document, document.value(), &mut Vec::new(), &mut result);
    result
}
