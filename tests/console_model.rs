use flutter_network_viewer::capture::{CaptureStore, ConsoleRecord};
use serde_json::{Value, json};

fn line(id: u64, text: &str) -> ConsoleRecord {
    ConsoleRecord {
        id,
        session_id: "app".into(),
        event: json!({
            "source":"dart.stdout", "message":text, "isolateId":"main",
            "timestamp":"2026-10-15T08:20:58.000Z"
        }),
    }
}
fn grouped(records: &[ConsoleRecord]) -> Vec<ConsoleRecord> {
    ConsoleRecord::grouped(records)
}

#[test]
fn display_cleanup_preserves_raw_and_export() {
    let raw =
        "\x1b[31mflutter: Error\x1b[0m\r\nflutter:   detail\n flutter: keep\nflutter:no space";
    let r = line(0, raw);
    assert_eq!(
        r.display_message(),
        "Error\n  detail\n flutter: keep\nno space"
    );
    assert_eq!(r.message(), raw);
    assert_eq!(r.line_count(), 4);
    let mut store = CaptureStore::default();
    store
        .ingest(&json!({"session":{"id":"app"},"sdk":{"schemaVersion":1},"events":[r.event]}))
        .unwrap();
    assert_eq!(
        store.export_at("now")["console"][0]["event"]["message"],
        raw
    );
}
#[test]
fn csi_cleanup_accepts_parameters_intermediates_and_multiple_lines() {
    assert_eq!(
        line(
            0,
            "\x1b[?25lflutter: hi\x1b[2J\nflutter: \x1b[1;32mgreen\x1b[0m"
        )
        .display_message(),
        "hi\ngreen"
    );
    assert_eq!(line(0, "\x1b[broken").display_message(), "roken");
    assert_eq!(line(0, "\x1b[").display_message(), "\x1b[");
}
#[test]
fn groups_json_in_order_without_mutating_input() {
    let records = vec![
        line(0, "before"),
        line(1, "flutter: {"),
        line(2, "flutter:   \"name\": \"Cà phê\","),
        line(3, "flutter:   \"items\": [1, 2]"),
        line(4, "flutter: }"),
        line(5, "after"),
    ];
    let entries = grouped(&records);
    assert_eq!(entries.len(), 3);
    assert_eq!(entries[1].id, 1);
    assert_eq!(
        entries[1].display_message(),
        "{\n  \"name\": \"Cà phê\",\n  \"items\": [1, 2]\n}"
    );
    assert_eq!(entries[1].line_count(), 4);
    assert_eq!(
        entries[1].event["rawEvents"],
        json!(records[1..5].iter().map(|r| &r.event).collect::<Vec<_>>())
    );
    assert_eq!(records[1].message(), "flutter: {");
    assert!(records[1].event.get("rawEvents").is_none());
}
#[test]
fn nested_array_closes_only_when_json_is_complete() {
    let r = grouped(&[
        line(0, "["),
        line(1, "{\"a\": [1, 2]}"),
        line(2, "]"),
        line(3, "next"),
    ]);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].line_count(), 3);
}
#[test]
fn incomplete_invalid_and_ordinary_logs_remain_separate() {
    for texts in [
        vec!["{", "unrelated", "}"],
        vec!["{", "\"a\": 1"],
        vec!["first", "second"],
        vec!["{\"a\":1}", "{}"],
        vec!["══╡ EXCEPTION", "no terminator"],
    ] {
        let input: Vec<_> = texts
            .iter()
            .enumerate()
            .map(|(i, s)| line(i as u64, s))
            .collect();
        assert_eq!(grouped(&input).len(), input.len());
    }
}
#[test]
fn attribution_boundaries_prevent_grouping() {
    for field in ["session", "isolateId", "source"] {
        let mut end = line(1, "}");
        if field == "session" {
            end.session_id = "other".into();
        } else {
            end.event[field] = json!("other");
        }
        assert_eq!(grouped(&[line(0, "{"), end]).len(), 2);
    }
    for source in ["dart.logging", "js.console"] {
        let mut r = vec![line(0, "{"), line(1, "}")];
        for x in &mut r {
            x.event["source"] = json!(source);
        }
        assert_eq!(grouped(&r).len(), 2);
    }
}
#[test]
fn time_window_uses_first_record_and_absolute_milliseconds() {
    for (timestamp, count) in [
        ("2026-10-15T08:21:00.000Z", 1),
        ("2026-10-15T08:21:00.001Z", 2),
        ("2026-10-15T08:20:55.999Z", 2),
    ] {
        let mut end = line(1, "}");
        end.event["timestamp"] = json!(timestamp);
        assert_eq!(grouped(&[line(0, "{"), end]).len(), count);
    }
    let mut middle = line(1, "\"a\":1");
    middle.event["timestamp"] = json!("2026-10-15T08:20:59.500Z");
    let mut end = line(2, "}");
    end.event["timestamp"] = json!("2026-10-15T08:21:01Z");
    assert_eq!(grouped(&[line(0, "{"), middle, end]).len(), 3);
}
#[test]
fn missing_or_invalid_time_does_not_block_groups() {
    for timestamp in [Value::Null, json!("bad")] {
        let mut end = line(1, "}");
        end.event["timestamp"] = timestamp;
        assert_eq!(grouped(&[line(0, "{"), end]).len(), 1);
    }
}
#[test]
fn exception_promotes_level_and_stops_at_terminator() {
    let input = vec![
        line(0, "flutter: ══╡ EXCEPTION CAUGHT BY WIDGETS LIBRARY ╞══"),
        line(1, "flutter: A RenderFlex overflowed"),
        line(2, "flutter: #0 Widget.build"),
        line(3, "flutter: ════════"),
        line(4, "next"),
    ];
    let r = grouped(&input);
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].level(), "error");
    assert_eq!(r[0].event["rawEvents"].as_array().unwrap().len(), 4);
    assert_eq!(input[0].level(), "log");
    assert_eq!(r[1].message(), "next");
}
#[test]
fn grouping_is_bounded_at_200_records() {
    for (n, expected) in [(200, 1), (201, 201)] {
        let mut r = vec![line(0, "[")];
        for i in 1..n - 1 {
            r.push(line(i, " "));
        }
        r.push(line(n - 1, "]"));
        assert_eq!(grouped(&r).len(), expected);
    }
}
#[test]
fn grouping_budget_counts_utf16_including_newlines() {
    for (units, count) in [(65532, 1), (65533, 3)] {
        let content = format!(
            "\"{}{}\"",
            "😀".repeat((units - 2) / 2),
            if units % 2 == 1 { "x" } else { "" }
        );
        assert_eq!(content.encode_utf16().count(), units);
        let r = grouped(&[line(0, "["), line(1, &content), line(2, "]")]);
        assert_eq!(r.len(), count);
    }
}
#[test]
fn details_use_explicit_values_then_typed_args() {
    let mut r = line(0, "hello");
    r.event["args"] = json!(["ignored",{"type":"error","value":"fallback error"},{"type":"stackTrace","value":"#0 fallback"},{"value":"arg needle"}]);
    assert_eq!(r.error().as_deref(), Some("fallback error"));
    assert_eq!(r.stack_trace().as_deref(), Some("#0 fallback"));
    for empty in [Value::Null, json!(""), json!("null")] {
        r.event["error"] = empty;
        assert_eq!(r.error().as_deref(), Some("fallback error"));
    }
    r.event["error"] = json!("explicit");
    assert_eq!(r.error().as_deref(), Some("explicit"));
    r.event["args"] = json!({"wrong":"shape"});
    r.event["stackTrace"] = Value::Null;
    assert_eq!(r.stack_trace(), None);
}
#[test]
fn search_includes_logger_error_stack_args_and_source() {
    let mut r = line(0, "flutter: hello");
    r.event["loggerName"] = json!("ProfileRepository");
    r.event["error"] = json!("missing user");
    r.event["stackTrace"] = json!("profile.dart:24");
    r.event["args"] = json!([{"value":"needle"}]);
    let text = r.search_text();
    for needle in [
        "hello",
        "stdout",
        "ProfileRepository",
        "missing user",
        "profile.dart:24",
        "needle",
    ] {
        assert!(text.contains(needle));
    }
    assert!(!text.contains("flutter:"));
}
#[test]
fn metadata_defaults_severity_and_source_labels() {
    let mut r = line(0, "");
    r.event = json!({});
    assert_eq!(r.level(), "log");
    assert_eq!(r.message(), "");
    assert_eq!(r.severity(), None);
    assert_eq!(r.line_count(), 1);
    for (source, label) in [
        ("dart.stdout", "stdout"),
        ("dart.stderr", "stderr"),
        ("dart.logging", "dart:developer"),
        ("js.console", "js.console"),
    ] {
        r.event = json!({"source":source,"level":"warning","severity":900});
        assert_eq!(r.source_label(), label);
        assert_eq!(r.level(), "warning");
        assert_eq!(r.severity(), Some(900.0));
    }
}
