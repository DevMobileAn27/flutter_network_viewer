use flutter_network_viewer::json::{JsonDocument, PathPart};
use flutter_network_viewer::json_find::{FindOptions, JsonFindController, ViewMode};
use serde_json::json;
use std::{collections::HashSet, sync::Arc};

fn key(s: &str) -> PathPart {
    PathPart::Key(s.into())
}
fn controller(value: serde_json::Value) -> JsonFindController {
    JsonFindController::new(Arc::new(JsonDocument::from_value(value)))
}
fn slice(text: &str, start: usize, end: usize) -> String {
    String::from_utf16(&text.encode_utf16().collect::<Vec<_>>()[start..end]).unwrap()
}

#[test]
fn case_word_regex_navigation_and_errors_match_flutter() {
    let mut c = controller(json!({"name":"Chi chi child CHI", "nested":{"value":"chi"}}));
    c.set_query("chi");
    assert_eq!(c.matches().len(), 5);
    assert_eq!(c.active().unwrap().path, vec![key("name")]);
    c.move_by(-1);
    assert_eq!(c.index(), Some(4));
    assert_eq!(c.active().unwrap().path, vec![key("nested"), key("value")]);
    c.move_by(1);
    assert_eq!(c.index(), Some(0));
    c.set_options(FindOptions {
        match_case: true,
        ..Default::default()
    });
    assert_eq!(c.matches().len(), 3);
    c.set_options(FindOptions {
        match_case: true,
        whole_word: true,
        regex: false,
    });
    assert_eq!(c.matches().len(), 2);
    c.set_options(FindOptions {
        whole_word: true,
        regex: true,
        match_case: false,
    });
    c.set_query("ch[iI]");
    assert_eq!(c.matches().len(), 4);
    c.set_query("[");
    assert!(c.error().is_some());
    assert!(c.active().is_none());
    c.move_by(1);
    assert_eq!(c.index(), None);
    c.set_query("^");
    assert!(c.error().is_none());
    assert!(c.matches().is_empty());
    c.set_query("");
    assert!(c.error().is_none());
}

#[test]
fn literals_escape_regex_metacharacters_and_no_results_clear_active() {
    let mut c = controller(json!({"a.b":"[x] a.b"}));
    c.set_query("a.b");
    assert_eq!(c.matches().len(), 2);
    c.set_query("[x]");
    assert_eq!(c.matches().len(), 1);
    c.set_query("absent");
    assert!(c.matches().is_empty());
    assert!(c.active().is_none());
    c.move_by(isize::MIN);
    assert!(c.active().is_none());
}

#[test]
fn unicode_utf16_offsets_and_escaped_keys_map_to_typed_paths() {
    let mut c = controller(json!({"💙quoted\"key":[{"chi":"💙 Cà phê 💙"}]}));
    c.set_query("💙");
    assert_eq!(c.matches().len(), 3);
    let text = c.document().text();
    for m in c.matches() {
        assert_eq!(m.end - m.start, 2);
        assert_eq!(slice(text, m.start, m.end), "💙");
    }
    assert_eq!(c.matches()[0].path, vec![key("💙quoted\"key")]);
    assert_eq!(
        c.matches()[2].path,
        vec![key("💙quoted\"key"), PathPart::Index(0), key("chi")]
    );
    c.set_query("\\\"");
    assert_eq!(c.matches().len(), 1);
    assert_eq!(c.active().unwrap().path, vec![key("💙quoted\"key")]);
    c.set_query("CÀ PHÊ");
    assert_eq!(c.matches().len(), 1);
    let m = c.active().unwrap();
    assert_eq!(slice(c.document().text(), m.start, m.end), "Cà phê");
}

#[test]
fn whole_word_uses_unicode_letters_numbers_and_underscore() {
    let mut c = controller(json!([
        "chi chí chi_ xchi chi2 chié chi中 chi💙 chi! 𝒜chi chi𝒜"
    ]));
    c.set_options(FindOptions {
        whole_word: true,
        ..Default::default()
    });
    c.set_query("chi");
    assert_eq!(c.matches().len(), 5); // Flutter checks one UTF-16 unit: surrogate halves are boundaries
}

#[test]
fn regex_supports_multiline_lookaround_backreferences_and_skips_zero_width() {
    let mut c = controller(json!(["chi chi", "💙", "CHi"]));
    c.set_options(FindOptions {
        regex: true,
        ..Default::default()
    });
    c.set_query(r"(chi) \1");
    assert_eq!(c.matches().len(), 1);
    c.set_query(r#"(?<=")chi(?= chi)"#);
    assert_eq!(c.matches().len(), 1);
    c.set_query(r#"^  ""#);
    assert_eq!(c.matches().len(), 3);
    c.set_query(r"(?=chi)|chi");
    assert!(c.matches().is_empty());
    c.set_query("💙");
    assert_eq!(c.active().unwrap().end - c.active().unwrap().start, 2);
}

#[test]
fn punctuation_and_container_closings_return_to_nearest_ancestor() {
    let mut c = controller(json!({"items":[{"name":"chi"}],"tail":true}));
    c.set_options(FindOptions {
        regex: true,
        ..Default::default()
    });
    c.set_query(r"[}\],]");
    let paths: Vec<_> = c.matches().iter().map(|m| m.path.clone()).collect();
    assert_eq!(
        paths,
        vec![
            vec![key("items"), PathPart::Index(0)],
            vec![key("items")],
            vec![],
            vec![]
        ]
    );
}

#[test]
fn navigation_wraps_arbitrary_signed_deltas() {
    let mut c = controller(json!(["x", "x", "x"]));
    c.set_query("x");
    c.move_by(-7);
    assert_eq!(c.index(), Some(2));
    c.move_by(7);
    assert_eq!(c.index(), Some(0));
    c.move_by(isize::MAX);
    assert_eq!(c.index(), Some((isize::MAX as usize) % 3));
    c.move_by(isize::MIN);
    assert_eq!(c.index(), Some(2));
}

#[test]
fn value_refresh_keeps_query_options_and_mode_but_resets_active() {
    let mut c = controller(json!(["needle", "needle"]));
    let options = FindOptions {
        match_case: true,
        whole_word: true,
        regex: false,
    };
    c.set_options(options);
    c.set_query("needle");
    c.move_by(1);
    c.set_mode(ViewMode::Tree);
    assert_eq!(c.index(), Some(1));
    c.set_mode(ViewMode::Code);
    assert_eq!(c.index(), Some(1));
    c.set_mode(ViewMode::Tree);
    c.set_document(Arc::new(JsonDocument::from_value(
        json!({"updated":"needle"}),
    )));
    assert_eq!(c.query(), "needle");
    assert_eq!(c.options(), options);
    assert_eq!(c.mode(), ViewMode::Tree);
    assert_eq!(c.index(), Some(0));
    assert_eq!(c.active().unwrap().path, vec![key("updated")]);
    c.set_document(Arc::new(JsonDocument::from_value(json!(null))));
    assert!(c.active().is_none());
    assert_eq!(c.query(), "needle");
}

#[test]
fn reveal_unfolds_only_active_ancestors_in_both_views() {
    let mut c = controller(json!({"nested":{"items":[{"name":"target"}]},"other":{"x":1}}));
    c.set_query("target");
    let p = c.document().presentation();
    let mut tree: HashSet<_> = p
        .tree
        .iter()
        .enumerate()
        .filter(|(i, r)| r.fold > *i)
        .map(|(i, _)| i)
        .collect();
    let other = p
        .tree
        .iter()
        .position(|r| r.path == vec![key("other")])
        .unwrap();
    c.reveal_tree(&mut tree);
    assert_eq!(tree, HashSet::from([other]));
    let target = p
        .tree
        .iter()
        .position(|r| r.path == c.active().unwrap().path)
        .unwrap();
    assert!(p.visible_tree_rows(&tree).contains(&target));
    let mut code: HashSet<_> = p.ranges.keys().copied().collect();
    let other_line = p
        .code
        .iter()
        .position(|r| r.text.contains("\"other\""))
        .unwrap();
    c.reveal_code(&mut code);
    assert_eq!(code, HashSet::from([other_line]));
    c.set_query("absent");
    c.reveal_code(&mut code);
    assert_eq!(code, HashSet::from([other_line]));
}

#[test]
fn empty_query_and_mode_changes_keep_document_completely_lazy() {
    let d = Arc::new(JsonDocument::from_value(json!({"big":[1,2,3]})));
    let mut c = JsonFindController::new(d.clone());
    c.set_query("");
    c.set_options(FindOptions {
        regex: true,
        ..Default::default()
    });
    c.set_mode(ViewMode::Tree);
    c.reveal_tree(&mut HashSet::from([0]));
    c.reveal_code(&mut HashSet::from([0]));
    assert!(!d.is_prepared());
    assert!(!d.is_text_prepared());
    c.set_query("big");
    assert!(d.is_prepared());
    assert!(d.is_text_prepared());
}

#[test]
fn scalar_empty_and_nested_documents_share_the_existing_format() {
    for value in [
        json!(null),
        json!(false),
        json!(12.5),
        json!("hello"),
        json!({}),
        json!([]),
        json!({"quoted\"key":["Cà phê",true,null,{},[],{"emoji":"💙\nline"}],"tail":42}),
    ] {
        let expected = serde_json::to_string_pretty(&value).unwrap();
        let c = controller(value);
        assert_eq!(c.document().text(), expected);
    }
}

#[test]
fn large_document_maps_every_result_and_reveals_last_folded_descendant() {
    let values: Vec<_> = (0..5000)
        .map(|i| json!({"name":format!("item {i}")}))
        .collect();
    let mut c = controller(json!({"items":values}));
    c.set_query("item");
    assert_eq!(c.matches().len(), 5001); // items key plus each value
    c.move_by(-1);
    assert_eq!(
        c.active().unwrap().path,
        vec![key("items"), PathPart::Index(4999), key("name")]
    );
    let p = c.document().presentation();
    let mut closed: HashSet<_> = p
        .tree
        .iter()
        .enumerate()
        .filter(|(i, r)| r.fold > *i)
        .map(|(i, _)| i)
        .collect();
    c.reveal_tree(&mut closed);
    let target = p
        .tree
        .iter()
        .position(|r| r.path == c.active().unwrap().path)
        .unwrap();
    assert!(p.visible_tree_rows(&closed).contains(&target));
    assert_eq!(closed.len(), 4999);
}

#[test]
fn invalid_regex_is_recoverable_without_preparing_document() {
    let document = Arc::new(JsonDocument::from_value(json!(["needle"])));
    let mut c = JsonFindController::new(document.clone());
    c.set_options(FindOptions {
        regex: true,
        ..Default::default()
    });
    c.set_query("[");
    assert!(c.error().is_some());
    assert!(!document.is_text_prepared());
    assert!(!document.is_prepared());
    c.set_query("needle");
    assert!(c.error().is_none());
    assert_eq!(c.matches().len(), 1);
    c.set_query("");
    assert!(c.matches().is_empty());
    assert!(c.active().is_none());
}
