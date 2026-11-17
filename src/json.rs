//! Immutable JSON documents and lazy indexes shared by future JSON/Tree views.
//! All public source offsets use UTF-16 code units, matching Dart and native hosts.
use indexmap::IndexMap;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, OnceLock};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathPart {
    Key(String),
    Index(usize),
}
pub type JsonPath = Vec<PathPart>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub raw: String,
    pub message: String,
    /// One-based line and UTF-16 column; raw is available for fallback display.
    pub line: usize,
    pub column: usize,
}
impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for ParseError {}

#[derive(Debug)]
pub struct JsonDocument {
    value: Value,
    text: OnceLock<String>,
    prepared: OnceLock<Presentation>,
    colors: OnceLock<Vec<crate::body_color::ColorPreview>>,
}
impl JsonDocument {
    pub fn parse(source: &str) -> Result<Self, ParseError> {
        serde_json::from_str(source)
            .map(Self::from_value)
            .map_err(|error| {
                let line = error.line().max(1);
                let byte_column = error.column().max(1);
                let column = source.lines().nth(line - 1).map_or(1, |text| {
                    // serde reports byte columns; convert the prefix before the error.
                    text.char_indices()
                        .take_while(|(i, _)| *i < byte_column - 1)
                        .map(|(_, c)| c.len_utf16())
                        .sum::<usize>()
                        + 1
                });
                ParseError {
                    raw: source.into(),
                    message: error.to_string(),
                    line,
                    column,
                }
            })
    }
    pub fn from_value(value: Value) -> Self {
        Self {
            value,
            text: OnceLock::new(),
            prepared: OnceLock::new(),
            colors: OnceLock::new(),
        }
    }
    /// Lazy swatch metadata; raw values, text, search and fold indexes stay immutable.
    pub fn color_previews(&self) -> &[crate::body_color::ColorPreview] {
        self.colors
            .get_or_init(|| crate::body_color::previews(self))
    }
    pub fn value(&self) -> &Value {
        &self.value
    }
    pub fn text(&self) -> &str {
        self.text.get_or_init(|| {
            serde_json::to_string_pretty(&self.value).expect("JSON values serialize")
        })
    }
    pub fn is_text_prepared(&self) -> bool {
        self.text.get().is_some()
    }
    pub fn is_prepared(&self) -> bool {
        self.prepared.get().is_some()
    }
    pub fn presentation(&self) -> &Presentation {
        self.prepared.get_or_init(|| Presentation::build(self))
    }
}

#[derive(Debug, Clone)]
pub struct JsonNode {
    pub path: JsonPath,
    pub key_start: Option<usize>,
    pub value_start: usize,
    pub end: usize,
}
#[derive(Debug, Clone)]
pub struct CodeRow {
    pub text: String,
    pub columns: usize,
    pub start: usize,
    pub end: usize,
    /// Inclusive closing line, or this row if it cannot fold.
    pub fold: usize,
}
#[derive(Debug, Clone)]
pub struct TextMapping {
    pub local_start: usize,
    pub len: usize,
    pub document_start: usize,
}
#[derive(Debug, Clone)]
pub struct TreeRow {
    pub path: JsonPath,
    pub text: String,
    pub columns: usize,
    pub start: usize,
    pub end: usize,
    /// Inclusive last descendant. Folding never removes indexed descendants.
    pub fold: usize,
    pub depth: usize,
    pub branches: Vec<bool>,
    pub last: bool,
    pub count: Option<usize>,
    pub maps: Vec<TextMapping>,
}
#[derive(Debug)]
pub struct Presentation {
    pub code: Vec<CodeRow>,
    pub tree: Vec<TreeRow>,
    pub nodes: HashMap<JsonPath, JsonNode>,
    pub ranges: BTreeMap<usize, usize>,
    pub columns: usize,
}
fn units(text: &str) -> usize {
    text.encode_utf16().count()
}
fn columns(text: &str) -> usize {
    text.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum()
}
fn encode(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).expect("JSON values serialize")
}

// Write the same two-space format while recording exact key/value boundaries.
#[derive(Default)]
struct NodeWriter {
    offset: usize,
    nodes: HashMap<JsonPath, JsonNode>,
}
impl NodeWriter {
    fn write(&mut self, value: &Value, path: JsonPath, depth: usize, key_start: Option<usize>) {
        let start = self.offset;
        let entries: Vec<(PathPart, &Value)> = match value {
            Value::Object(map) => map
                .iter()
                .map(|(k, v)| (PathPart::Key(k.clone()), v))
                .collect(),
            Value::Array(array) => array
                .iter()
                .enumerate()
                .map(|(i, v)| (PathPart::Index(i), v))
                .collect(),
            _ => Vec::new(),
        };
        if value.is_object() || value.is_array() {
            self.offset += 1;
            for (i, (part, child)) in entries.iter().enumerate() {
                self.offset += 1 + 2 * (depth + 1);
                let key = if let PathPart::Key(k) = part {
                    let key = self.offset;
                    self.offset += units(&encode(k)) + 2;
                    Some(key)
                } else {
                    None
                };
                let mut child_path = path.clone();
                child_path.push(part.clone());
                self.write(child, child_path, depth + 1, key);
                if i + 1 < entries.len() {
                    self.offset += 1;
                }
            }
            if !entries.is_empty() {
                self.offset += 1 + 2 * depth;
            }
            self.offset += 1;
        } else {
            self.offset += units(&encode(value));
        }
        self.nodes.insert(
            path.clone(),
            JsonNode {
                path,
                key_start,
                value_start: start,
                end: self.offset,
            },
        );
    }
}
impl Presentation {
    fn build(document: &JsonDocument) -> Self {
        let text = document.text();
        let mut ranges = BTreeMap::new();
        let mut openings = Vec::new();
        let (mut line, mut in_string, mut escaped) = (0, false, false);
        for c in text.chars() {
            if c == '\n' {
                line += 1;
            }
            if in_string {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == '"' {
                    in_string = false;
                }
            } else {
                match c {
                    '"' => in_string = true,
                    '{' | '[' => openings.push(line),
                    '}' | ']' => {
                        if let Some(start) = openings.pop()
                            && start < line
                        {
                            ranges.insert(start, line);
                        }
                    }
                    _ => {}
                }
            }
        }
        let mut offset = 0;
        let code: Vec<_> = text
            .split('\n')
            .enumerate()
            .map(|(i, text)| {
                let start = offset;
                offset += units(text) + 1;
                CodeRow {
                    text: text.into(),
                    columns: columns(text),
                    start,
                    end: offset - 1,
                    fold: *ranges.get(&i).unwrap_or(&i),
                }
            })
            .collect();
        let mut writer = NodeWriter::default();
        writer.write(document.value(), Vec::new(), 0, None);
        debug_assert_eq!(writer.offset, units(text));
        let mut result = Self {
            columns: code.iter().map(|r| r.columns).max().unwrap_or(0),
            code,
            tree: Vec::new(),
            nodes: writer.nodes,
            ranges,
        };
        result.visit(document.value(), Vec::new(), None, Vec::new(), true);
        result
    }
    fn visit(
        &mut self,
        value: &Value,
        path: JsonPath,
        name: Option<String>,
        ancestors: Vec<bool>,
        last: bool,
    ) {
        let node = &self.nodes[&path];
        let collection = value.is_object() || value.is_array();
        let count = match value {
            Value::Object(m) => m.len(),
            Value::Array(a) => a.len(),
            _ => 0,
        };
        let prefix = name.as_ref().map_or_else(String::new, |s| format!("{s}: "));
        let body = if value.is_object() {
            "{}".into()
        } else if value.is_array() {
            "[]".into()
        } else {
            encode(value)
        };
        let mut maps = Vec::new();
        if let Some(key_start) = node.key_start {
            maps.push(TextMapping {
                local_start: 0,
                len: units(name.as_deref().unwrap_or_default()),
                document_start: key_start + 1,
            });
        }
        maps.push(TextMapping {
            local_start: units(&prefix),
            len: if collection { 1 } else { units(&body) },
            document_start: node.value_start,
        });
        if collection {
            maps.push(TextMapping {
                local_start: units(&prefix) + 1,
                len: 1,
                document_start: node.end - 1,
            });
        }
        let text = format!("{prefix}{body}");
        let index = self.tree.len();
        self.tree.push(TreeRow {
            path: path.clone(),
            columns: columns(&text),
            text,
            start: node.key_start.unwrap_or(node.value_start),
            end: node.end,
            fold: index,
            depth: path.len(),
            branches: ancestors.clone(),
            last,
            count: (count > 0).then_some(count),
            maps,
        });
        let mut children_ancestors = ancestors;
        if name.is_some() {
            children_ancestors.push(!last);
        }
        match value {
            Value::Object(map) => {
                for (i, (key, child)) in map.iter().enumerate() {
                    let escaped = encode(key);
                    let mut child_path = path.clone();
                    child_path.push(PathPart::Key(key.clone()));
                    self.visit(
                        child,
                        child_path,
                        Some(escaped[1..escaped.len() - 1].into()),
                        children_ancestors.clone(),
                        i + 1 == count,
                    );
                }
            }
            Value::Array(array) => {
                for (i, child) in array.iter().enumerate() {
                    let mut child_path = path.clone();
                    child_path.push(PathPart::Index(i));
                    self.visit(
                        child,
                        child_path,
                        Some(format!("[{i}]")),
                        children_ancestors.clone(),
                        i + 1 == count,
                    );
                }
            }
            _ => {}
        }
        self.tree[index].fold = self.tree.len() - 1;
    }
    /// Return visible indexes without cloning or changing the immutable document.
    pub fn visible_tree_rows(&self, collapsed: &HashSet<usize>) -> Vec<usize> {
        let mut visible = Vec::new();
        let mut i = 0;
        while i < self.tree.len() {
            visible.push(i);
            i = if collapsed.contains(&i) {
                self.tree[i].fold + 1
            } else {
                i + 1
            };
        }
        visible
    }
}

/// LRU keyed by the exact immutable captured source; estimates match Flutter.
#[derive(Debug)]
pub struct JsonDocumentCache {
    max_bytes: usize,
    max_entries: usize,
    estimated_bytes: usize,
    entries: IndexMap<String, Arc<JsonDocument>>,
}
impl Default for JsonDocumentCache {
    fn default() -> Self {
        Self::new(32 * 1024 * 1024, 24)
    }
}
impl JsonDocumentCache {
    pub fn new(max_bytes: usize, max_entries: usize) -> Self {
        Self {
            max_bytes,
            max_entries,
            estimated_bytes: 0,
            entries: IndexMap::new(),
        }
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn estimated_bytes(&self) -> usize {
        self.estimated_bytes
    }
    pub fn clear(&mut self) {
        self.entries.clear();
        self.estimated_bytes = 0;
    }
    fn cost(source: &str) -> usize {
        units(source).saturating_mul(64).saturating_add(4096)
    }
    pub fn decode(&mut self, source: &str) -> Result<Arc<JsonDocument>, ParseError> {
        if let Some((key, document)) = self.entries.shift_remove_entry(source) {
            self.entries.insert(key, Arc::clone(&document));
            return Ok(document);
        }
        let document = Arc::new(JsonDocument::parse(source)?);
        let cost = Self::cost(source);
        if cost <= self.max_bytes && self.max_entries > 0 {
            while !self.entries.is_empty()
                && (self.estimated_bytes > self.max_bytes - cost
                    || self.entries.len() >= self.max_entries)
            {
                if let Some((key, _)) = self.entries.shift_remove_index(0) {
                    self.estimated_bytes -= Self::cost(&key);
                }
            }
            self.entries.insert(source.into(), Arc::clone(&document));
            self.estimated_bytes += cost;
        }
        Ok(document)
    }
}
