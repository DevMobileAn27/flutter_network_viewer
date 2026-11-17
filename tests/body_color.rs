use flutter_network_viewer::body_color::parse_body_color;
use flutter_network_viewer::json::{JsonDocument, PathPart};
use flutter_network_viewer::json_find::JsonFindController;
use serde_json::json;
use std::{collections::HashSet, sync::Arc};

fn check(cases: &[(&str, u32)]) {
    for (text, expected) in cases {
        assert_eq!(parse_body_color(text), Some(*expected), "{text}");
    }
}
#[test]
fn css_hex_expands_and_uses_trailing_alpha() {
    check(&[
        ("#B45309", 0xffb45309),
        ("#abc", 0xffaabbcc),
        ("#abcd", 0xddaabbcc),
        ("#11223380", 0x80112233),
        ("#0000", 0),
        ("#fff", 0xffffffff),
    ]);
}
#[test]
fn flutter_hex_uses_leading_alpha() {
    check(&[
        ("0x80112233", 0x80112233),
        (" 0XFFABCDEF ", 0xffabcdef),
        ("0x00112233", 0x00112233),
    ]);
}
#[test]
fn rgb_numbers_percentages_and_rounding() {
    check(&[
        ("rgb(180, 83, 9)", 0xffb45309),
        ("rgba(255,0,0,.5)", 0x80ff0000),
        ("rgb(100% 0% 0% / 50%)", 0x80ff0000),
        ("RGB(1.5 2.5 3.5)", 0xff020304),
        ("rgba(1e2,0,0)", 0xff640000),
    ]);
}
#[test]
fn source_accepts_mixed_separators_and_optional_alpha_on_either_name() {
    check(&[
        ("rgb(255 / 0, 0, 50%)", 0x80ff0000),
        ("hsla(120 100% 50%)", 0xff00ff00),
        ("hsl(240 100% 50% / .5)", 0x800000ff),
    ]);
}
#[test]
fn hsl_wraps_hue_and_converts_achromatic_values() {
    check(&[
        ("hsl(120,100%,50%)", 0xff00ff00),
        ("hsla(240deg,100%,50%,.5)", 0x800000ff),
        ("hsl(-120,100%,50%)", 0xff0000ff),
        ("hsl(720,100%,50%)", 0xffff0000),
        ("hsl(25,0%,50%)", 0xff808080),
        ("hsl(33 100% 0%)", 0xff000000),
        ("hsl(33 100% 100%)", 0xffffffff),
        ("hsl(30 100% 50%)", 0xffff8000),
    ]);
}
#[test]
fn all_source_names_and_case_whitespace() {
    check(&[
        ("black", 0xff000000),
        ("white", 0xffffffff),
        ("red", 0xffff0000),
        ("green", 0xff008000),
        ("blue", 0xff0000ff),
        ("yellow", 0xffffff00),
        ("gray", 0xff808080),
        ("grey", 0xff808080),
        ("silver", 0xffc0c0c0),
        ("maroon", 0xff800000),
        ("purple", 0xff800080),
        ("fuchsia", 0xffff00ff),
        ("lime", 0xff00ff00),
        ("olive", 0xff808000),
        ("navy", 0xff000080),
        ("teal", 0xff008080),
        ("aqua", 0xff00ffff),
        ("orange", 0xffffa500),
        (" \u{feff}TRANSPARENT\u{a0}", 0),
    ]);
}
#[test]
fn rejects_ids_substrings_and_unsupported_names() {
    for text in [
        "B45309",
        "123456",
        "id #abcdef",
        "use #fff here",
        "url(https://example.com/#abc)",
        "red blue",
        "red\nblue",
        "rebeccapurple",
        "#fff\nextra",
        "0xff11223344",
    ] {
        assert_eq!(parse_body_color(text), None, "{text}");
    }
}
#[test]
fn rejects_malformed_hex_and_functions() {
    for text in [
        "",
        "#12",
        "#12345",
        "#1234567",
        "#xyz",
        "#123456789",
        "0xabcdef",
        "rgb()",
        "rgb(1 2)",
        "rgb(1 2 3 4 5)",
        "rgb((1),2,3)",
        "hsl(0,5,2)",
        "hsl(1rad 100% 50%)",
        "rgb(1px 2 3)",
        "rgb(1 2 3) trailing",
    ] {
        assert_eq!(parse_body_color(text), None, "{text}");
    }
}
#[test]
fn rejects_non_finite_and_out_of_range_channels() {
    for text in [
        "rgb(300,0,0)",
        "rgb(-1 0 0)",
        "rgba(0,0,0,2)",
        "rgba(0 0 0 / -1%)",
        "rgb(NaN,0,0)",
        "rgb(inf 0 0)",
        "rgb(1e999 0 0)",
        "rgb(101% 0 0)",
        "hsl(NaN 0% 0%)",
        "hsl(0 101% 50%)",
        "hsl(0 50% -1%)",
    ] {
        assert_eq!(parse_body_color(text), None, "{text}");
    }
}
#[test]
fn previews_only_string_values_including_arrays_and_root() {
    let d = JsonDocument::from_value(
        json!({"#ff0000":"key only","textColor":"#B45309","background":"rgba(255, 0, 0, 0.5)","id":"B45309","message":"use #fff here","list":["blue",123,null,true,{"x":"transparent"}]}),
    );
    let previews = d.color_previews();
    assert_eq!(previews.len(), 4);
    assert_eq!(
        previews.iter().map(|p| p.argb).collect::<Vec<_>>(),
        [0xffb45309, 0x80ff0000, 0xff0000ff, 0]
    );
    assert_eq!(
        previews[2].path,
        vec![PathPart::Key("list".into()), PathPart::Index(0)]
    );
    let root = JsonDocument::from_value(json!("#abc"));
    assert!(root.color_previews()[0].path.is_empty());
    for value in [json!(null), json!(12), json!(false), json!({}), json!([])] {
        assert!(JsonDocument::from_value(value).color_previews().is_empty());
    }
}
#[test]
fn offsets_are_utf16_and_cover_encoded_token_with_escaped_whitespace() {
    let d = JsonDocument::parse(r##"{"💙\"key":"\n#abc\t","before":"💙","tail":"red"}"##).unwrap();
    let units: Vec<_> = d.text().encode_utf16().collect();
    for p in d.color_previews() {
        let token = String::from_utf16(&units[p.start..p.end]).unwrap();
        assert_eq!(serde_json::from_str::<String>(&token).unwrap(), p.value);
        let n = &d.presentation().nodes[&p.path];
        assert_eq!((p.start, p.end), (n.value_start, n.end));
    }
    assert_eq!(d.color_previews().len(), 2);
    assert_eq!(d.color_previews()[0].value, "\n#abc\t");
    assert!(d.color_previews()[1].start < d.text().find("\"red\"").unwrap());
}
#[test]
fn metadata_is_lazy_cached_and_response_specific() {
    let d = JsonDocument::from_value(json!(["red"]));
    assert!(!d.is_prepared());
    let first = d.color_previews();
    assert!(std::ptr::eq(first, d.color_previews()));
    let replacement = JsonDocument::from_value(json!(["blue"]));
    assert_eq!(replacement.color_previews()[0].argb, 0xff0000ff);
    assert_eq!(first[0].argb, 0xffff0000);
}
#[test]
fn search_copy_data_and_folding_remain_unchanged() {
    let raw = r##"{"💙":{"color":"#B45309","other":"rgba(255, 0, 0, 0.5)"}}"##;
    let d = Arc::new(JsonDocument::parse(raw).unwrap());
    let value = d.value().clone();
    let text = d.text().to_owned();
    assert_eq!(d.color_previews().len(), 2);
    let mut find = JsonFindController::new(d.clone());
    find.set_query("#B45309");
    assert_eq!(find.matches().len(), 1);
    let p = d.presentation();
    let closed = HashSet::from([0]);
    assert_eq!(p.visible_tree_rows(&closed), [0]);
    assert!(d.color_previews().iter().all(|c| {
        !p.visible_tree_rows(&closed)
            .iter()
            .any(|i| p.tree[*i].path == c.path)
    }));
    assert!(d.color_previews().iter().all(|c| {
        p.visible_tree_rows(&HashSet::new())
            .iter()
            .any(|i| p.tree[*i].path == c.path)
    }));
    assert_eq!(d.text(), text);
    assert_eq!(d.value(), &value);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&text).unwrap(),
        value
    );
    assert_eq!(
        raw,
        r##"{"💙":{"color":"#B45309","other":"rgba(255, 0, 0, 0.5)"}}"##
    );
}

#[test]
fn escaped_json_color_and_code_folds_keep_original_token_offsets() {
    let d = JsonDocument::parse(r##"{"nested":["\u0023abc","red"],"last":"blue"}"##).unwrap();
    let p = d.presentation();
    let nested_line = p
        .code
        .iter()
        .position(|r| r.text.contains("\"nested\""))
        .unwrap();
    let end = p.ranges[&nested_line];
    let visible: Vec<_> = d
        .color_previews()
        .iter()
        .filter(|c| {
            let line = p.code.partition_point(|r| r.start <= c.start) - 1;
            !(nested_line < line && line <= end)
        })
        .collect();
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].value, "blue");
    assert_eq!(d.color_previews()[0].value, "#abc");
    assert_eq!(d.color_previews()[0].argb, 0xffaabbcc);
    assert_eq!(d.color_previews().len(), 3);
}

#[test]
fn large_document_shares_one_metadata_cache_across_threads() {
    let d = Arc::new(JsonDocument::from_value(json!(
        (0..5000).map(|_| "#abcd").collect::<Vec<_>>()
    )));
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let d = d.clone();
            std::thread::spawn(move || {
                let previews = d.color_previews();
                assert_eq!(previews.len(), 5000);
                assert!(previews.windows(2).all(|p| p[0].end < p[1].start));
                previews.as_ptr() as usize
            })
        })
        .collect();
    let pointers: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(pointers.iter().all(|p| *p == pointers[0]));
}
