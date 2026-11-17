use flutter_network_viewer::json::{JsonDocument, JsonDocumentCache, PathPart};
use serde_json::json;
use std::sync::Arc;

fn utf16_slice(s: &str, start: usize, end: usize) -> String {
    String::from_utf16(&s.encode_utf16().collect::<Vec<_>>()[start..end]).unwrap()
}

#[test]
fn objects_format_two_spaces_and_preserve_key_order() {
    let d = JsonDocument::parse(r#"{"z":[1,{"a":true}],"a":null}"#).unwrap();
    assert_eq!(
        d.text(),
        "{\n  \"z\": [\n    1,\n    {\n      \"a\": true\n    }\n  ],\n  \"a\": null\n}"
    );
    assert_eq!(d.value(), &json!({"z":[1,{"a":true}],"a":null}));
}

#[test]
fn scalars_and_empty_collections_have_one_row() {
    for source in ["null", "true", "false", "2.5", "-3", "\"😀\"", "{}", "[]"] {
        let d = JsonDocument::parse(source).unwrap();
        assert_eq!(d.text(), source);
        assert_eq!(d.presentation().code.len(), 1);
        assert_eq!(d.presentation().tree.len(), 1);
        assert_eq!(d.presentation().tree[0].fold, 0);
    }
}

#[test]
fn invalid_json_preserves_raw_and_reports_line_column() {
    let raw = "{\n  \"a\": @\n}";
    let e = JsonDocument::parse(raw).unwrap_err();
    assert_eq!(e.raw, raw);
    assert_eq!((e.line, e.column), (2, 8));
    assert!(!e.message.is_empty());
    assert!(JsonDocument::parse("").is_err());
    assert!(JsonDocument::parse("{} trailing").is_err());
}

#[test]
fn presentation_is_lazy_and_shared() {
    let d = JsonDocument::parse("{\"a\":1}").unwrap();
    assert!(!d.is_prepared());
    assert!(std::ptr::eq(d.presentation(), d.presentation()));
    assert!(d.is_prepared());
}

#[test]
fn unicode_keys_offsets_and_tree_mappings_match_document() {
    let key = "Tiếng 😀\n\"";
    let d = JsonDocument::from_value(json!({key:{"text":"😀hello"},"list":[null,true,2.5]}));
    let p = d.presentation();
    for row in &p.code {
        assert_eq!(utf16_slice(d.text(), row.start, row.end), row.text);
    }
    for row in &p.tree {
        for m in &row.maps {
            assert_eq!(
                utf16_slice(&row.text, m.local_start, m.local_start + m.len),
                utf16_slice(d.text(), m.document_start, m.document_start + m.len)
            );
        }
    }
    let node = &p.nodes[&vec![PathPart::Key(key.into()), PathPart::Key("text".into())]];
    assert_eq!(
        utf16_slice(d.text(), node.value_start, node.end),
        "\"😀hello\""
    );
    assert!(node.key_start.is_some());
    assert_eq!(
        p.nodes[&vec![PathPart::Key("list".into()), PathPart::Index(2)]]
            .path
            .len(),
        2
    );
}

#[test]
fn tree_branches_counts_and_array_indices_match_flutter() {
    let d = JsonDocument::parse(
        r##"{"group":{"child":{"color":"#f00"},"lastChild":1},"rows":[true],"tail":{}}"##,
    )
    .unwrap();
    let rows = &d.presentation().tree;
    assert_eq!(
        rows.iter().map(|r| r.text.as_str()).collect::<Vec<_>>(),
        [
            "{}",
            "group: {}",
            "child: {}",
            "color: \"#f00\"",
            "lastChild: 1",
            "rows: []",
            "[0]: true",
            "tail: {}"
        ]
    );
    assert_eq!(rows[2].branches, [true]);
    assert_eq!(rows[3].branches, [true, true]);
    assert!(!rows[1].last);
    assert!(rows[4].last);
    assert_eq!(rows[0].count, Some(3));
    assert_eq!(rows[7].count, None);
    assert_eq!(rows[1].fold, 4);
}

#[test]
fn folds_ignore_brackets_in_strings_and_preserve_data() {
    let d = JsonDocument::parse(r#"{"a":[{"b":"[}\\\""}],"empty":[]}"#).unwrap();
    let p = d.presentation();
    assert_eq!(p.ranges.len(), 3);
    assert_eq!(p.ranges[&0], p.code.len() - 1);
    let original = d.text().to_owned();
    assert_eq!(
        p.visible_tree_rows(&[1].into_iter().collect()),
        vec![0, 1, 4]
    );
    assert_eq!(p.visible_tree_rows(&Default::default()).len(), 5);
    assert_eq!(d.text(), original);
}

#[test]
fn width_counts_non_ascii_as_two_cells() {
    let d = JsonDocument::from_value(json!({"text":"漢字😀"}));
    let row = d
        .presentation()
        .code
        .iter()
        .find(|r| r.text.contains('漢'))
        .unwrap();
    assert_eq!(
        row.columns,
        row.text
            .chars()
            .map(|c| if c.is_ascii() { 1 } else { 2 })
            .sum::<usize>()
    );
    assert!(row.columns > row.text.encode_utf16().count());
}

#[test]
fn large_tree_keeps_folded_descendants() {
    let d = JsonDocument::from_value(
        json!({"rows":(0..5000).map(|i|json!({"id":i})).collect::<Vec<_>>()}),
    );
    let rows = &d.presentation().tree;
    assert_eq!(rows.len(), 10002);
    assert_eq!(rows[1].fold, 10001);
    assert_eq!(rows[2].fold, 3);
    assert_eq!(rows.last().unwrap().depth, 3);
}

#[test]
fn cache_reuses_and_evicts_least_recently_read() {
    let mut c = JsonDocumentCache::new(32 * 1024 * 1024, 2);
    let a = c.decode("1").unwrap();
    let b = c.decode("2").unwrap();
    assert!(Arc::ptr_eq(&a, &c.decode("1").unwrap()));
    c.decode("3").unwrap();
    assert_eq!(c.len(), 2);
    assert!(Arc::ptr_eq(&a, &c.decode("1").unwrap()));
    assert!(!Arc::ptr_eq(&b, &c.decode("2").unwrap()));
}

#[test]
fn cache_budget_oversized_disabled_and_invalid() {
    let mut c = JsonDocumentCache::new(10000, 24);
    for s in ["1", "2", "3"] {
        c.decode(s).unwrap();
    }
    assert_eq!(c.len(), 2);
    assert!(c.estimated_bytes() <= 10000);
    let large = format!("\"{}\"", "large".repeat(1000));
    let a = c.decode(&large).unwrap();
    assert!(!Arc::ptr_eq(&a, &c.decode(&large).unwrap()));
    let bytes = c.estimated_bytes();
    assert!(c.decode("invalid").is_err());
    assert_eq!(c.estimated_bytes(), bytes);
    let mut disabled = JsonDocumentCache::new(10000, 0);
    disabled.decode("null").unwrap();
    assert!(disabled.is_empty());
}

#[test]
fn cache_cost_uses_utf16_and_clear_releases_ownership() {
    let mut c = JsonDocumentCache::default();
    let a = c.decode("\"😀\"").unwrap();
    assert_eq!(c.estimated_bytes(), 4096 + 4 * 64);
    let weak = Arc::downgrade(&a);
    drop(a);
    c.clear();
    assert!(weak.upgrade().is_none());
    assert_eq!(c.estimated_bytes(), 0);
    assert!(c.is_empty());
}

#[test]
fn cache_keys_are_exact_source_strings() {
    let mut c = JsonDocumentCache::default();
    let a = c.decode("[1]").unwrap();
    let b = c.decode("[ 1 ]").unwrap();
    assert!(!Arc::ptr_eq(&a, &b));
    assert_eq!(a.text(), b.text());
}
