//! Search state shared by future code and Tree views. Ranges are UTF-16 offsets.
use crate::json::{JsonDocument, JsonPath};
use regress::{Flags, Regex};
use std::{
    collections::HashSet,
    sync::{Arc, OnceLock},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FindOptions {
    pub match_case: bool,
    pub whole_word: bool,
    pub regex: bool,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ViewMode {
    #[default]
    Code,
    Tree,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonFindMatch {
    pub start: usize,
    pub end: usize,
    pub path: JsonPath,
}

#[derive(Debug)]
pub struct JsonFindController {
    document: Arc<JsonDocument>,
    query: String,
    options: FindOptions,
    mode: ViewMode,
    error: Option<String>,
    matches: Vec<JsonFindMatch>,
    index: Option<usize>,
}
impl JsonFindController {
    pub fn new(document: Arc<JsonDocument>) -> Self {
        Self {
            document,
            query: String::new(),
            options: FindOptions::default(),
            mode: ViewMode::Code,
            error: None,
            matches: Vec::new(),
            index: None,
        }
    }
    pub fn document(&self) -> &Arc<JsonDocument> {
        &self.document
    }
    pub fn query(&self) -> &str {
        &self.query
    }
    pub fn options(&self) -> FindOptions {
        self.options
    }
    pub fn mode(&self) -> ViewMode {
        self.mode
    }
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
    pub fn matches(&self) -> &[JsonFindMatch] {
        &self.matches
    }
    pub fn index(&self) -> Option<usize> {
        self.index
    }
    pub fn active(&self) -> Option<&JsonFindMatch> {
        self.index.map(|i| &self.matches[i])
    }
    pub fn set_document(&mut self, document: Arc<JsonDocument>) {
        self.document = document;
        self.search();
    }
    pub fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
        self.search();
    }
    pub fn set_options(&mut self, options: FindOptions) {
        self.options = options;
        self.search();
    }
    /// Mode changes leave the query, options and current result intact.
    pub fn set_mode(&mut self, mode: ViewMode) {
        self.mode = mode;
    }
    pub fn move_by(&mut self, delta: isize) {
        if let Some(index) = self.index {
            // Widen before adding so even MIN/MAX deltas cannot overflow.
            self.index = Some(
                (index as i128 + delta as i128).rem_euclid(self.matches.len() as i128) as usize,
            );
        }
    }
    fn search(&mut self) {
        self.error = None;
        self.matches.clear();
        self.index = None;
        // Do not serialize or build indexes when no search was requested.
        if self.query.is_empty() {
            return;
        }
        let pattern = if self.options.regex {
            self.query.clone()
        } else {
            escape(&self.query)
        };
        let expression = match Regex::with_flags(
            &pattern,
            Flags {
                icase: !self.options.match_case,
                multiline: true,
                unicode: true,
                ..Default::default()
            },
        ) {
            Ok(expression) => expression,
            Err(error) => {
                self.error = Some(error.to_string());
                return;
            }
        };
        let text: Vec<u16> = self.document.text().encode_utf16().collect();
        let nodes = &self.document.presentation().nodes;
        let mut ordered: Vec<_> = nodes.values().collect();
        ordered.sort_unstable_by_key(|n| n.key_start.unwrap_or(n.value_start));
        let mut cursor = 0;
        for found in expression.find_from_utf16(&text, 0) {
            let range = found.range();
            if range.is_empty() {
                continue;
            }
            if self.options.whole_word
                && ((range.start > 0 && word_unit(text[range.start - 1]))
                    || (range.end < text.len() && word_unit(text[range.end])))
            {
                continue;
            }
            while cursor + 1 < ordered.len()
                && ordered[cursor + 1]
                    .key_start
                    .unwrap_or(ordered[cursor + 1].value_start)
                    <= range.start
            {
                cursor += 1;
            }
            let mut node = ordered[cursor];
            while !node.path.is_empty() && range.start >= node.end {
                node = &nodes[&node.path[..node.path.len() - 1]];
            }
            self.matches.push(JsonFindMatch {
                start: range.start,
                end: range.end,
                path: node.path.clone(),
            });
        }
        if !self.matches.is_empty() {
            self.index = Some(0);
        }
    }
    /// Remove folds containing the active position; unrelated folds stay closed.
    pub fn reveal_code(&self, collapsed: &mut HashSet<usize>) {
        let Some(active) = self.active() else {
            return;
        };
        let p = self.document.presentation();
        let line = p
            .code
            .partition_point(|r| r.start <= active.start)
            .saturating_sub(1);
        collapsed.retain(|i| {
            !p.ranges
                .get(i)
                .is_some_and(|end| *i <= line && line <= *end)
        });
    }
    /// Tree keys reveal their ancestors, retaining the matched node's own fold.
    pub fn reveal_tree(&self, collapsed: &mut HashSet<usize>) {
        let Some(active) = self.active() else {
            return;
        };
        let p = self.document.presentation();
        collapsed.retain(|i| {
            !p.tree.get(*i).is_some_and(|r| {
                r.path.len() < active.path.len() && active.path.starts_with(&r.path)
            })
        });
    }
}
fn escape(query: &str) -> String {
    let mut result = String::with_capacity(query.len());
    for c in query.chars() {
        if "\\^$.|?*+()[]{}".contains(c) {
            result.push('\\');
        }
        result.push(c);
    }
    result
}
// Match Dart's one-code-unit substring boundaries, including surrogate halves.
fn word_unit(unit: u16) -> bool {
    static WORD: OnceLock<Regex> = OnceLock::new();
    let Some(c) = char::from_u32(unit as u32) else {
        return false;
    };
    let mut buffer = [0; 4];
    WORD.get_or_init(|| Regex::with_flags(r"[\p{L}\p{N}_]", "u").expect("static word expression"))
        .find(c.encode_utf8(&mut buffer))
        .is_some()
}
