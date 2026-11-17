//! These tests drive the production egui renderer with pointer/key events.
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, Rect, Vec2};
use flutter_network_viewer::{
    capture::CaptureStore,
    desktop::{Action, DesktopUi, ShellStatus},
};
use serde_json::{Value, json};

struct Harness {
    ctx: egui::Context,
    ui: DesktopUi,
    store: CaptureStore,
    size: Vec2,
    status: ShellStatus,
    time: f64,
}
impl Harness {
    fn new(width: f32) -> Self {
        let mut store = CaptureStore::default();
        store.ingest(&fixture()).unwrap();
        let mut h = Self {
            ctx: egui::Context::default(),
            ui: DesktopUi::default(),
            store,
            size: Vec2::new(width, 800.),
            status: ShellStatus::default(),
            time: 0.,
        };
        h.frame(vec![]);
        h.frame(vec![]);
        h
    }
    fn frame(&mut self, events: Vec<Event>) -> (egui::FullOutput, Vec<Action>) {
        self.time += 0.02;
        let mut actions = vec![];
        let output = self.ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ctx| {
                actions = self.ui.show(ctx, &self.store, &self.status);
            },
        );
        assert!(!output.shapes.is_empty());
        (output, actions)
    }
    fn click(&mut self, key: &str) -> (egui::FullOutput, Vec<Action>) {
        self.frame(vec![]);
        if key == "vm-uri" && !self.ui.probe.widgets.contains_key(key) {
            self.click("manual-url");
        }
        if let Some(index) = key.strip_prefix("tab:").and_then(|s| s.parse().ok()) {
            self.ui.reveal_inspector_tab = Some(index);
            for _ in 0..12 {
                self.frame(vec![]);
            }
        }
        let pos = self
            .ui
            .probe
            .widgets
            .get(key)
            .unwrap_or_else(|| panic!("missing {key}: {:?}", self.ui.probe.widgets.keys()))
            .center();
        self.at(pos)
    }
    fn at(&mut self, pos: Pos2) -> (egui::FullOutput, Vec<Action>) {
        self.frame(vec![Event::PointerMoved(pos)]);
        self.frame(vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ]);
        let result = self.frame(vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
        self.frame(vec![]);
        result
    }
    fn escape(&mut self) {
        self.frame(vec![Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }]);
        self.frame(vec![]);
    }
    fn text(&mut self, key: &str, text: &str) {
        self.click(key);
        self.frame(vec![Event::Text(text.into())]);
        self.frame(vec![]);
    }
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/sdk_envelope.json")).unwrap()
}
fn copied(out: &egui::FullOutput) -> Option<&str> {
    out.platform_output.commands.iter().find_map(|v| {
        if let egui::OutputCommand::CopyText(s) = v {
            Some(s.as_str())
        } else {
            None
        }
    })
}
#[test]
fn app_tabs_scope_network_and_console_and_all_restores() {
    let mut h = Harness::new(1440.);
    let mut other = fixture();
    other["session"]["id"] = json!("second");
    other["session"]["appName"] = json!("Second app");
    h.store.ingest(&other).unwrap();
    h.frame(vec![]);
    assert_eq!(h.ui.probe.requests.len(), 6);
    h.click("app:second");
    assert_eq!(h.ui.probe.requests.len(), 3);
    h.click("Console");
    assert_eq!(h.ui.probe.logs.len(), 1);
    h.click("app:all");
    assert_eq!(h.ui.probe.logs.len(), 2);
    h.click("Network");
    assert_eq!(h.ui.probe.requests.len(), 6);
}
#[test]
fn method_status_search_combine_and_menus_dismiss() {
    let mut h = Harness::new(1440.);
    h.click("method");
    h.click("method:POST");
    assert_eq!(h.ui.probe.requests.len(), 1);
    h.click("status");
    h.click("status:Success");
    h.text("search", "201");
    assert_eq!(h.ui.probe.requests.len(), 1);
    h.frame(vec![Event::Text("missing".into())]);
    assert!(h.ui.probe.requests.is_empty());
    h.click("reset");
    assert_eq!(h.ui.probe.requests.len(), 3);
    h.text("search", "get");
    assert_eq!(h.ui.probe.requests.len(), 2);
    h.click("reset");
    h.text("search", "orders");
    assert_eq!(h.ui.probe.requests.len(), 1);
    h.click("method");
    assert!(h.ui.probe.widgets.contains_key("method:OPTIONS"));
    h.escape();
    assert!(!h.ui.probe.widgets.contains_key("method:OPTIONS"));
    h.click("method");
    h.at(Pos2::new(4., 790.));
    assert!(!h.ui.probe.widgets.contains_key("method:OPTIONS"));
}
#[test]
fn pending_modal_stays_live_and_all_six_inspector_tabs_render() {
    let mut h = Harness::new(860.);
    h.click("request:ios-session-1:fetch-3");
    assert!(h.ui.probe.inspector_text.contains("Pending"));
    let mut completion = fixture();
    completion["events"] = json!([{ "requestId":"fetch-3", "source":"js.fetch", "phase":"response", "response":{"statusCode":201,"headers":{"x-test":"yes"},"body":"live-body"} }]);
    h.store.ingest(&completion).unwrap();
    h.frame(vec![]);
    assert!(h.ui.probe.inspector_text.contains("201"));
    // Tabs scroll horizontally; expose each via the same egui scroll API as keyboard focus.
    for index in 0..6 {
        h.ui.reveal_inspector_tab = Some(index);
        h.frame(vec![]);
        h.frame(vec![]);
        h.click(&format!("tab:{index}"));
        assert_eq!(h.ui.settings.values().request_detail_tab, index as i64);
        assert!(!h.ui.probe.inspector_text.is_empty());
    }
    assert!(h.ui.probe.inspector_text.contains("live-body"));
    h.escape();
    assert!(h.ui.selected_request.is_none());
    h.click("request:ios-session-1:fetch-3");
    h.at(Pos2::new(2., 790.));
    assert!(h.ui.selected_request.is_none());
}
#[test]
fn clipboard_commands_and_export_file_use_actual_selected_data() {
    let mut h = Harness::new(1440.);
    h.click("request:ios-session-1:fetch-1");
    let (out, _) = h.click("curl");
    assert!(
        copied(&out)
            .unwrap()
            .contains("curl -X POST 'https://api.example.com/orders?draft=true'")
    );
    h.click("tab:4");
    let (out, _) = h.click("json-copy");
    assert!(copied(&out).unwrap().contains("created"));
    let path = std::env::temp_dir().join(format!("fnv-ui-export-{}.json", std::process::id()));
    h.click("export");
    h.ui.export_path = path.to_string_lossy().into_owned();
    h.click("save-export");
    let value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(value["format"], "flutter_network_viewer");
    assert_eq!(value["network"].as_array().unwrap().len(), 3);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn layouts_fit_narrow_sidebar_and_columns_scroll_and_resize() {
    for width in [480., 860., 1100., 1360.] {
        let mut h = Harness::new(width);
        assert!(h.ui.probe.list_rect.right() <= width);
        assert!(h.ui.probe.sidebar_width < width / 3.);
        // Force overflow; Flutter otherwise lets URL fill the viewport.
        h.ui.settings
            .set_network_column_widths(&[220., 180., 240.])
            .unwrap();
        h.frame(vec![]);
        assert!(h.ui.probe.table_content_width >= 798.);
        let before = h.ui.probe.widgets["request:ios-session-1:fetch-1"].left();
        let center = h.ui.probe.list_rect.center();
        h.frame(vec![
            Event::PointerMoved(center),
            Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: Vec2::new(-400., 0.),
                modifiers: Modifiers::NONE,
            },
        ]);
        h.frame(vec![]);
        if width < 860. {
            assert!(h.ui.probe.widgets["request:ios-session-1:fetch-1"].left() < before);
        }
    }
}
#[test]
fn discovery_picker_and_capture_actions_are_wired_and_dismissible() {
    let mut h = Harness::new(1440.);
    let (_, actions) = h.click("scan");
    assert_eq!(actions, vec![Action::StartScan]);
    h.status.searching = true;
    h.frame(vec![]);
    let (_, actions) = h.click("stop-scan");
    assert_eq!(actions, vec![Action::StopScan]);
    h.status.targets = vec![
        flutter_network_viewer::discovery::Target {
            name: "Shop".into(),
            uri: "ws://127.0.0.1:8181/ws".into(),
            vm_key: "one".into(),
            project_key: "shop".into(),
        },
        flutter_network_viewer::discovery::Target {
            name: "Other".into(),
            uri: "ws://127.0.0.1:8182/ws".into(),
            vm_key: "two".into(),
            project_key: "other".into(),
        },
    ];
    h.frame(vec![]);
    h.frame(vec![]);
    let (_, actions) = h.click("target:two");
    assert_eq!(actions, vec![Action::SelectTarget("two".into())]);
    h.status.targets.clear();
    h.status.searching = false;
    h.frame(vec![]);
    let (_, actions) = h.click("pause");
    assert_eq!(actions, vec![Action::SetPaused(true)]);
    let (_, actions) = h.click("clear");
    assert_eq!(actions, vec![Action::Clear]);
    h.text("vm-uri", "ws://127.0.0.1:8181/ws");
    let (_, actions) = h.click("connect-submit");
    assert_eq!(
        actions,
        vec![Action::Connect("ws://127.0.0.1:8181/ws".into())]
    );
    h.escape();
    assert!(!h.ui.probe.widgets.contains_key("vm-uri"));
}

#[test]
fn three_column_dividers_align_with_rows_and_persist_across_restart() {
    let mut h = Harness::new(1440.);
    let path = std::env::temp_dir().join(format!("fnv-ui-layout-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    h.ui.settings = flutter_network_viewer::settings::AppSettings::load(&path).unwrap();
    h.frame(vec![]);
    assert!(
        (h.ui.probe.widgets["header:url"].left()
            - h.ui.probe.widgets["request:ios-session-1:fetch-1"].left())
        .abs()
            < 1.
    );
    for i in 0..3 {
        let before = h.ui.settings.values().network_column_widths[i];
        let pos = h.ui.probe.widgets[&format!("column:{i}")].center();
        h.frame(vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ]);
        let moved = pos
            + Vec2::new(
                match i {
                    1 => -40.,
                    2 => 10.,
                    _ => 40.,
                },
                0.,
            );
        h.frame(vec![Event::PointerMoved(moved)]);
        h.frame(vec![Event::PointerButton {
            pos: moved,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
        h.frame(vec![]);
        assert_eq!(
            h.ui.settings.values().network_column_widths[i],
            before + if i == 2 { -10. } else { 40. }
        );
    }
    let saved = h.ui.settings.values().network_column_widths;
    h.click("Console");
    h.click("Network");
    h.size.x = 480.;
    h.frame(vec![]);
    assert_eq!(h.ui.settings.values().network_column_widths, saved);
    h.ui = DesktopUi::new(flutter_network_viewer::settings::AppSettings::load(&path).unwrap());
    h.frame(vec![]);
    assert_eq!(h.ui.settings.values().network_column_widths, saved);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn errors_pending_success_filters_and_picker_escape_outside_cancel() {
    let mut h = Harness::new(1440.);
    for (status, id) in [
        ("Errors", "xhr-2"),
        ("Pending", "fetch-3"),
        ("Success", "fetch-1"),
    ] {
        h.click("status");
        h.click(&format!("status:{status}"));
        assert_eq!(
            h.ui.probe.requests,
            vec![("ios-session-1".into(), id.into())]
        );
    }
    h.status
        .targets
        .push(flutter_network_viewer::discovery::Target {
            name: "Shop".into(),
            uri: "ws://127.0.0.1:8181/ws".into(),
            vm_key: "one".into(),
            project_key: "shop".into(),
        });
    h.frame(vec![]);
    h.frame(vec![]);
    let (_, actions) = h.frame(vec![Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    assert_eq!(actions, vec![Action::StopScan]);
    h.status.targets.clear();
    h.frame(vec![]);
    h.status
        .targets
        .push(flutter_network_viewer::discovery::Target {
            name: "Shop".into(),
            uri: "ws://127.0.0.1:8181/ws".into(),
            vm_key: "one".into(),
            project_key: "shop".into(),
        });
    h.frame(vec![]);
    h.frame(vec![]);
    let (_, actions) = h.at(Pos2::new(2., 790.));
    assert_eq!(actions, vec![Action::StopScan]);
}

#[test]
fn many_app_tabs_scroll_at_narrow_width_and_filter_both_pages() {
    let mut h = Harness::new(480.);
    for i in 0..14 {
        let mut payload = fixture();
        payload["session"]["id"] = json!(format!("app{i}"));
        payload["session"]["appName"] = json!(format!("Application number {i}"));
        h.store.ingest(&payload).unwrap();
    }
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.ui.probe.widgets["app:app13"].left() > 480.);
    let pos = h.ui.probe.widgets["app:all"].center();
    h.frame(vec![
        Event::PointerMoved(pos),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: Vec2::new(-4000., 0.),
            modifiers: Modifiers::NONE,
        },
    ]);
    for _ in 0..30 {
        h.frame(vec![]);
    }
    assert!(h.ui.probe.widgets["app:app13"].right() <= 480.);
    h.click("app:app13");
    assert_eq!(h.ui.probe.requests.len(), 3);
    h.click("Console");
    assert_eq!(h.ui.probe.logs.len(), 1);
}

#[test]
fn inspector_tab_follows_new_selection_and_file_restart_wide_and_narrow() {
    for width in [860., 1440.] {
        let mut h = Harness::new(width);
        let path =
            std::env::temp_dir().join(format!("fnv-tab-{}-{width}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        h.ui = DesktopUi::new(flutter_network_viewer::settings::AppSettings::load(&path).unwrap());
        h.frame(vec![]);
        h.click("request:ios-session-1:fetch-1");
        h.ui.reveal_inspector_tab = Some(4);
        h.frame(vec![]);
        h.frame(vec![]);
        h.click("tab:4");
        assert!(h.ui.probe.inspector_text.contains("created"));
        if width < 1100. {
            h.escape();
        }
        h.click("request:ios-session-1:fetch-3");
        assert_eq!(h.ui.settings.values().request_detail_tab, 4);
        assert_eq!(h.ui.probe.inspector_text, "null");
        h.ui = DesktopUi::new(flutter_network_viewer::settings::AppSettings::load(&path).unwrap());
        h.frame(vec![]);
        h.frame(vec![]);
        h.click("request:ios-session-1:fetch-1");
        assert_eq!(h.ui.settings.values().request_detail_tab, 4);
        assert!(h.ui.probe.inspector_text.contains("created"));
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn overview_metadata_query_and_raw_match_the_capture_record() {
    let mut h = Harness::new(1440.);
    h.click("request:ios-session-1:fetch-1");
    for value in [
        "fetch-1",
        "js.fetch",
        "2026-10-14",
        "201",
        "Response size",
        "draft",
        "true",
    ] {
        assert!(h.ui.probe.inspector_text.contains(value), "missing {value}");
    }
    h.ui.reveal_inspector_tab = Some(5);
    h.frame(vec![]);
    h.frame(vec![]);
    h.click("tab:5");
    let shown: Value = serde_json::from_str(&h.ui.probe.inspector_text).unwrap();
    assert_eq!(
        shown,
        serde_json::to_value(
            h.store
                .network()
                .get(&("ios-session-1".into(), "fetch-1".into()))
                .unwrap()
        )
        .unwrap()
    );
}

fn console_event(h: &mut Harness, session: &str, events: Value) {
    let mut envelope = fixture();
    envelope["session"]["id"] = json!(session);
    envelope["events"] = events;
    h.store.ingest(&envelope).unwrap();
}
#[test]
fn console_source_level_search_and_app_filters_combine() {
    let mut h = Harness::new(1440.);
    console_event(
        &mut h,
        "second",
        json!([
            {"type":"console","source":"dart.logging","level":"error","message":"failure","loggerName":"orders","error":"needle","stackTrace":"stack-42","args":[{"value":"arg-42"}]},
            {"type":"console","source":"dart.stdout","level":"log","message":"noise"}
        ]),
    );
    h.frame(vec![]);
    h.click("Console");
    h.click("app:second");
    h.click("source");
    h.click("source:dart.logging");
    h.click("level");
    h.click("level:error");
    for term in ["needle", "orders", "stack-42", "arg-42"] {
        h.text("search", term);
        assert_eq!(h.ui.probe.logs.len(), 1, "{term}");
        h.click("reset");
        h.click("source");
        h.click("source:dart.logging");
        h.click("level");
        h.click("level:error");
    }
    h.click("app:ios-session-1");
    assert!(h.ui.probe.logs.is_empty());
    h.click("app:all");
    assert_eq!(h.ui.probe.logs.len(), 1);
    h.click("reset");
    assert_eq!(h.ui.probe.logs.len(), 3);
}
#[test]
fn console_groups_json_exception_inspects_copies_and_lazily_opens_raw() {
    for width in [860., 1440.] {
        let mut h = Harness::new(width);
        console_event(
            &mut h,
            "ios-session-1",
            json!([
                {"type":"console","source":"dart.stdout","message":"flutter: {"},
                {"type":"console","source":"dart.stdout","message":"flutter: \"answer\":42"},
                {"type":"console","source":"dart.stdout","message":"flutter: }"},
                {"type":"console","source":"dart.stderr","message":"══╡ EXCEPTION caught"},
                {"type":"console","source":"dart.stderr","message":"broken layout"},
                {"type":"console","source":"dart.stderr","message":"════════════"}
            ]),
        );
        h.click("Console");
        assert_eq!(h.ui.probe.logs.len(), 3);
        let id = h.ui.probe.logs[1];
        h.click(&format!("log:{id}"));
        assert!(h.ui.probe.widgets.contains_key("metadata:row:0"));
        assert!(!h.ui.probe.widgets.contains_key("json-toolbar"));
        assert!(!h.ui.probe.widgets.contains_key("inspector-heading"));
        assert_eq!(h.ui.console_raw_serializations, 0);
        let (out, _) = h.click("metadata:copy:5");
        assert_eq!(copied(&out), Some("{\n\"answer\":42\n}"));
        h.click("console-raw");
        let raw: Value = serde_json::from_str(&h.ui.probe.inspector_text).unwrap();
        assert_eq!(raw["event"]["rawEvents"].as_array().unwrap().len(), 3);
        h.click("json-tree");
        assert!(h.ui.probe.json.tree);
        h.click("json-code");
        let (out, _) = h.click("json-copy");
        assert_eq!(
            serde_json::from_str::<Value>(copied(&out).unwrap()).unwrap(),
            raw
        );
        assert!(raw.to_string().contains("flutter:"));
        let count = h.ui.console_raw_serializations;
        for _ in 0..4 {
            h.frame(vec![]);
        }
        assert_eq!(h.ui.console_raw_serializations, count);
        h.click("console-detail");
        if width < 1100. {
            h.escape();
            assert!(h.ui.selected_log.is_none());
        }
        let id = h.ui.probe.logs[2];
        h.click(&format!("log:{id}"));
        assert!(h.ui.probe.inspector_text.contains("ERROR"));
        assert!(h.ui.probe.inspector_text.contains("broken layout"));
        assert_eq!(h.ui.console_raw_serializations, count);
        if width < 1100. {
            h.at(Pos2::new(2., 790.));
            assert!(h.ui.selected_log.is_none());
        }
    }
}
#[test]
fn console_live_append_keeps_search_focus_scroll_selection_and_unopened_raw_lazy() {
    let mut h = Harness::new(1440.);
    console_event(&mut h, "ios-session-1", json!((0..100).map(|i| json!({"type":"console","source":"dart.stdout","message":format!("message {i}")})).collect::<Vec<_>>()));
    h.click("Console");
    h.click("log:2");
    h.click("search");
    let search_id = h.ctx.memory(|m| m.focused());
    let pos = h.ui.probe.list_rect.center();
    h.frame(vec![
        Event::PointerMoved(pos),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: Vec2::new(0., -500.),
            modifiers: Modifiers::NONE,
        },
    ]);
    for _ in 0..20 {
        h.frame(vec![]);
    }
    let offset = h.ui.probe.console_scroll;
    assert!(offset > 0.);
    console_event(
        &mut h,
        "ios-session-1",
        json!([{"type":"console","source":"dart.stdout","message":"new message"}]),
    );
    h.frame(vec![]);
    assert_eq!(h.ctx.memory(|m| m.focused()), search_id);
    assert_eq!(h.ui.probe.console_scroll, offset);
    assert_eq!(h.ui.selected_log, Some(2));
    assert_eq!(h.ui.console_raw_serializations, 0);
    h.frame(vec![Event::Text("message".into())]);
    h.frame(vec![]);
    assert_eq!(h.ui.probe.logs.len(), 101);
    h.click("clear");
    h.store.clear();
    h.frame(vec![]);
    assert!(h.ui.selected_log.is_none());
    assert!(h.ui.probe.inspector_text.is_empty());
}

#[test]
fn console_split_persists_independently_of_network_and_survives_restart() {
    let mut h = Harness::new(1440.);
    let path = std::env::temp_dir().join(format!("fnv-console-split-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    h.ui = DesktopUi::new(flutter_network_viewer::settings::AppSettings::load(&path).unwrap());
    h.ui.settings.set_inspector_width(500., false).unwrap();
    h.frame(vec![]);
    h.click("Console");
    h.click("log:0");
    let before = h.ui.probe.widgets["console-panel"].width();
    let pos = h.ui.probe.widgets["console-panel"].left_center();
    h.frame(vec![
        Event::PointerMoved(pos),
        Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
    ]);
    let moved = pos - Vec2::new(70., 0.);
    h.frame(vec![Event::PointerMoved(moved)]);
    h.frame(vec![Event::PointerButton {
        pos: moved,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![]);
    let saved = h.ui.settings.values().console_inspector_width;
    assert!(saved > before as f64 + 40.);
    assert_eq!(h.ui.settings.values().network_inspector_width, Some(500.));
    h.click("Network");
    h.click("request:ios-session-1:fetch-1");
    h.click("Console");
    assert!((h.ui.probe.widgets["console-panel"].width() as f64 - saved).abs() < 2.);
    h.ui = DesktopUi::new(flutter_network_viewer::settings::AppSettings::load(&path).unwrap());
    h.frame(vec![]);
    h.click("Console");
    h.click("log:0");
    assert_eq!(h.ui.settings.values().console_inspector_width, saved);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn console_warn_level_from_vm_is_filterable_and_raw_tracks_live_group_completion() {
    let mut h = Harness::new(1440.);
    console_event(
        &mut h,
        "ios-session-1",
        json!([
            {"type":"console","source":"dart.logging","level":"warn","message":"warning from VM"},
            {"type":"console","source":"dart.stdout","message":"{"}
        ]),
    );
    h.click("Console");
    h.click("level");
    h.click("level:warn");
    assert_eq!(h.ui.probe.logs.len(), 1);
    h.click("reset");
    let id = h.ui.probe.logs[2];
    h.click(&format!("log:{id}"));
    h.click("console-raw");
    let first = h.ui.console_raw_serializations;
    console_event(
        &mut h,
        "ios-session-1",
        json!([
            {"type":"console","source":"dart.stdout","message":"\"live\":true"},
            {"type":"console","source":"dart.stdout","message":"}"}
        ]),
    );
    h.frame(vec![]);
    assert_eq!(h.ui.selected_log, Some(id));
    let shown: Value = serde_json::from_str(&h.ui.probe.inspector_text).unwrap();
    assert_eq!(shown["event"]["rawEvents"].as_array().unwrap().len(), 3);
    assert_eq!(h.ui.console_raw_serializations, first + 1);
    h.frame(vec![]);
    assert_eq!(h.ui.console_raw_serializations, first + 1);
}

fn shortcut(key: egui::Key, shift: bool) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers {
            ctrl: true,
            command: true,
            shift,
            ..Modifiers::NONE
        },
    }
}
fn json_harness(source: &str) -> Harness {
    let mut h = Harness::new(1440.);
    h.click("JSON");
    h.text("json-input", source);
    h.click("json-format");
    h
}
#[test]
fn json_format_shortcut_error_preserves_draft_and_previous_output() {
    let mut h = json_harness(r##"{"name":"Việt 😀","array":[1,null]}"##);
    let expected = "{\n  \"name\": \"Việt 😀\",\n  \"array\": [\n    1,\n    null\n  ]\n}";
    assert_eq!(h.ui.probe.json.text, expected);
    let (out, _) = h.click("json-copy");
    assert_eq!(copied(&out), Some(expected));
    h.click("json-input");
    h.frame(vec![
        shortcut(egui::Key::A, false),
        Event::Text("{\n  \"x\": }".into()),
    ]);
    h.frame(vec![shortcut(egui::Key::Enter, false)]);
    h.frame(vec![]);
    assert!(h.ui.probe.json.error.contains("line 2"));
    assert!(h.ui.probe.json.error.contains("column"));
    assert_eq!(h.ui.probe.json.draft, "{\n  \"x\": }");
    assert_eq!(h.ui.probe.json.text, expected);
    h.click("json-input");
    h.frame(vec![
        shortcut(egui::Key::A, false),
        Event::Text("null".into()),
    ]);
    h.frame(vec![shortcut(egui::Key::Enter, false)]);
    h.frame(vec![]);
    assert_eq!(h.ui.probe.json.text, "null");
    assert!(h.ui.probe.json.error.is_empty());
}
#[test]
fn json_find_focus_options_highlights_navigation_and_fold_reveal() {
    let mut h =
        json_harness(r##"{"group":{"name":"red","other":"RED redder"},"color":"#ff0000"}"##);
    h.click("json-fold:1");
    assert!(!h.ui.probe.json.rows.contains(&2));
    h.frame(vec![shortcut(egui::Key::F, false)]);
    h.frame(vec![Event::Text("red".into())]);
    h.frame(vec![]);
    assert_eq!(h.ui.probe.json.matches, 3);
    assert!(h.ui.probe.json.rows.contains(&2));
    assert!(h.ui.probe.json.highlighted > 0);
    h.click("json-word");
    assert_eq!(h.ui.probe.json.matches, 2);
    h.click("json-case");
    assert_eq!(h.ui.probe.json.matches, 1);
    h.click("json-case");
    h.click("json-next");
    assert_eq!(h.ui.probe.json.active, Some(1));
    h.click("json-next");
    assert_eq!(h.ui.probe.json.active, Some(0));
    h.click("json-prev");
    assert_eq!(h.ui.probe.json.active, Some(1));
    h.click("json-tree");
    assert_eq!(h.ui.probe.json.query, "red");
    h.click("json-find");
    h.frame(vec![shortcut(egui::Key::A, false), Event::Text("[".into())]);
    h.click("json-regex");
    assert!(!h.ui.probe.json.find_error.is_empty());
    let (out, _) = h.frame(vec![]);
    let error = &h.ui.probe.json.find_error;
    let popup = h.ui.probe.widgets["notification"];
    let messages: Vec<_> = out
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == error => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(messages.len(), 1);
    assert!(popup.contains(messages[0].pos));
}
#[test]
fn json_draft_folds_query_mode_and_split_survive_navigation_resize() {
    let mut h = json_harness(r##"{"a":{"child":1},"b":2}"##);
    h.click("json-tree");
    h.click("json-fold:1");
    h.text("json-find", "absent");
    h.click("json-input");
    h.frame(vec![Event::Text(" draft".into())]);
    let draft = h.ui.probe.json.draft.clone();
    let rows = h.ui.probe.json.rows.clone();
    h.click("Network");
    h.size = Vec2::new(860., 600.);
    h.frame(vec![]);
    h.click("JSON");
    assert_eq!(h.ui.probe.json.draft, draft);
    assert_eq!(h.ui.probe.json.rows, rows);
    assert!(h.ui.probe.json.tree);
    assert_eq!(h.ui.probe.json.query, "absent");
    let divider = h.ui.probe.widgets["json-divider"].center();
    h.frame(vec![
        Event::PointerMoved(divider),
        Event::PointerButton {
            pos: divider,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
    ]);
    let end = divider + Vec2::new(60., 0.);
    h.frame(vec![Event::PointerMoved(end)]);
    h.frame(vec![Event::PointerButton {
        pos: end,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    assert!((h.ui.settings.values().json_input_fraction - 0.5).abs() > 0.03);
}
#[test]
fn json_output_scrolls_both_axes_and_select_all_copies_hidden_rows_without_gutter() {
    let value = json!({"items": (0..140).map(|i| format!("row-{i}-{}", "long".repeat(70))).collect::<Vec<_>>(), "paint":"#ff0000"});
    let mut h = json_harness(&value.to_string());
    let expected = h.ui.probe.json.text.clone();
    let pos = h.ui.probe.widgets["json-output"].center();
    h.frame(vec![
        Event::PointerMoved(pos),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: Vec2::new(-500., -800.),
            modifiers: Modifiers::NONE,
        },
    ]);
    h.frame(vec![]);
    assert!(h.ui.probe.json.scroll.x > 0.);
    assert!(h.ui.probe.json.scroll.y > 0.);
    h.click("json-output");
    h.frame(vec![shortcut(egui::Key::A, false)]);
    let (out, _) = h.frame(vec![Event::Copy]);
    assert_eq!(copied(&out), Some(expected.as_str()));
    h.click("json-tree");
    h.click("json-fold:0");
    let (out, _) = h.click("json-copy");
    assert_eq!(copied(&out), Some(expected.as_str()));
}
#[test]
fn json_colors_are_separate_clickable_swatches_and_copy_keeps_unicode() {
    let mut h =
        json_harness(r##"{"màu":"#ff0000","other":"rgba(0, 255, 0, .5)","id":"abc-red-def"}"##);
    assert_eq!(h.ui.probe.json.swatches.len(), 2);
    let key = h.ui.probe.json.swatches[0].clone();
    let (out, _) = h.click(&key);
    assert_eq!(copied(&out), Some("#ff0000"));
    let (out, _) = h.click("json-copy");
    assert_eq!(copied(&out), Some(h.ui.probe.json.text.as_str()));
}
#[test]
fn json_network_body_uses_shared_find_tree_copy_and_live_invalidation() {
    let mut h = Harness::new(1440.);
    h.click("request:ios-session-1:fetch-3");
    let mut completion = fixture();
    completion["events"] = json!([{ "requestId":"fetch-3", "source":"js.fetch", "phase":"response", "response":{"statusCode":200,"body":"{\"deep\":{\"color\":\"#ff0000\"}}"}}]);
    h.store.ingest(&completion).unwrap();
    h.ui.reveal_inspector_tab = Some(4);
    h.frame(vec![]);
    h.frame(vec![]);
    h.click("tab:4");
    assert!(h.ui.probe.json.text.contains("\"deep\""));
    h.click("json-tree");
    h.text("json-find", "color");
    assert_eq!(h.ui.probe.json.matches, 1);
    let (out, _) = h.click("json-copy");
    assert!(copied(&out).unwrap().contains("#ff0000"));
    h.click("tab:0");
    h.click("tab:4");
    assert!(h.ui.probe.json.tree);
    assert_eq!(h.ui.probe.json.query, "color");
    completion["events"][0]["response"]["body"] = json!("{\"color\":\"blue\"}");
    h.store.ingest(&completion).unwrap();
    h.frame(vec![]);
    assert!(h.ui.probe.json.text.contains("blue"));
    assert!(!h.ui.probe.json.text.contains("deep"));
}

#[test]
fn json_find_retains_active_match_across_views_tabs_and_resize_and_focuses_from_draft() {
    let mut h =
        json_harness(r##"{"items":[{"name":"needle"},{"name":"needle"},{"name":"needle"}]}"##);
    h.click("json-input");
    h.frame(vec![shortcut(egui::Key::F, false)]);
    h.frame(vec![Event::Text("needle".into())]);
    h.frame(vec![]);
    assert_eq!(h.ui.probe.json.matches, 3);
    h.click("json-next");
    assert_eq!(h.ui.probe.json.active, Some(1));
    h.click("json-tree");
    h.click("Console");
    h.size = Vec2::new(860., 600.);
    h.click("JSON");
    assert_eq!(h.ui.probe.json.active, Some(1));
    h.frame(vec![Event::Key {
        key: egui::Key::F3,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers {
            shift: true,
            ..Modifiers::NONE
        },
    }]);
    h.frame(vec![]);
    assert_eq!(h.ui.probe.json.active, Some(0));
    h.click("json-code");
    assert_eq!(h.ui.probe.json.active, Some(0));
    h.click("json-input");
    h.frame(vec![
        shortcut(egui::Key::A, false),
        Event::Text("[]".into()),
    ]);
    h.frame(vec![shortcut(egui::Key::Enter, false)]);
    h.frame(vec![]);
    assert_eq!(h.ui.probe.json.text, "[]");
    assert_eq!(h.ui.probe.json.query, "needle");
    assert_eq!(h.ui.probe.json.matches, 0);
}

#[test]
fn json_find_reveals_horizontal_match_on_long_unicode_line() {
    let value = json!({"line":format!("{}😀needle","prefix".repeat(90))});
    let mut h = json_harness(&value.to_string());
    h.text("json-find", "needle");
    h.frame(vec![]);
    h.frame(vec![]);
    assert_eq!(h.ui.probe.json.matches, 1);
    assert!(
        h.ui.probe.json.scroll.x > 100.,
        "active match must scroll horizontally"
    );
}
#[test]
fn json_body_search_is_local_and_only_valid_json_bodies_offer_find() {
    let mut h = Harness::new(1440.);
    let mut event = fixture();
    event["events"] = json!([{"requestId":"fetch-3","source":"js.fetch","phase":"request","request":{"method":"GET","url":"https://x/","body":"{\"needle\":42}"}},
        {"requestId":"fetch-3","source":"js.fetch","phase":"response","response":{"statusCode":200,"body":"plain needle"}}]);
    h.store.ingest(&event).unwrap();
    h.frame(vec![]);
    h.click("request:ios-session-1:fetch-3");
    h.click("tab:2");
    h.frame(vec![shortcut(egui::Key::F, false)]);
    h.frame(vec![Event::Text("needle".into())]);
    h.frame(vec![]);
    assert_eq!(h.ui.probe.json.matches, 1);
    h.click("tab:4");
    assert!(!h.ui.probe.widgets.contains_key("json-find"));
    h.click("tab:1");
    assert!(!h.ui.probe.widgets.contains_key("json-find"));
    h.click("tab:2");
    assert_eq!(h.ui.probe.json.query, "needle");
}

fn f16_body(h: &mut Harness, data: Value) {
    let mut envelope = fixture();
    envelope["events"] =
        json!([{"requestId":"fetch-3","source":"js.fetch","phase":"response","response":data}]);
    h.store.ingest(&envelope).unwrap();
    h.frame(vec![]);
}
fn f16_select_body(h: &mut Harness) {
    h.click("request:ios-session-1:fetch-3");
    h.ui.reveal_inspector_tab = Some(4);
    h.frame(vec![]);
    h.frame(vec![]);
    h.click("tab:4");
}
#[test]
fn f16_form_raw_live_update_and_omitted_notice() {
    let mut h = Harness::new(1440.);
    f16_body(
        &mut h,
        json!({"statusCode":200,"headers":{"CONTENT-TYPE":"application/x-www-form-urlencoded"},"body":"a=hello+world&a=%F0%9F%98%80"}),
    );
    f16_select_body(&mut h);
    assert_eq!(h.ui.probe.body_kind, "form");
    assert!(h.ui.probe.body_preview.contains("hello world"));
    h.click("body-raw");
    let (out, _) = h.click("copy-detail");
    assert_eq!(copied(&out), Some("a=hello+world&a=%F0%9F%98%80"));
    h.click("body-preview");
    f16_body(
        &mut h,
        json!({"statusCode":200,"headers":{"content-type":"application/x-www-form-urlencoded"},"body":"new=value"}),
    );
    assert!(h.ui.probe.body_preview.contains("value"));
    assert!(!h.ui.probe.body_preview.contains("hello"));
    f16_body(
        &mut h,
        json!({"statusCode":200,"bodyOmitted":"profile-size-limit","bodyCaptureLimit":1024}),
    );
    assert!(h.ui.probe.body_preview.contains("1024"));
    assert!(!h.ui.probe.widgets.contains_key("json-find"));
}
#[test]
fn f16_image_shares_texture_zoom_modal_clips_and_clear_releases() {
    use base64::Engine;
    for width in [860., 1440.] {
        let mut h = Harness::new(width);
        let encoded = base64::engine::general_purpose::STANDARD
            .encode(include_bytes!("fixtures/preview.png"));
        f16_body(
            &mut h,
            json!({"statusCode":200,"body":encoded,"bodyEncoding":"base64","headers":{"content-type":"image/png"}}),
        );
        f16_select_body(&mut h);
        assert_eq!(h.ui.probe.body_kind, "image");
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while h.ui.probe.image_textures == 0 && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(2));
            h.frame(vec![]);
        }
        assert_eq!(h.ui.probe.image_textures, 1);
        let prepared = h.ui.body_preparations;
        h.click("image-open");
        assert!(h.ui.probe.widgets.contains_key("image-modal"));
        h.click("image-zoom-in");
        assert!(h.ui.probe.image_zoom > 1.);
        assert_eq!(h.ui.probe.image_textures, 1);
        assert_eq!(h.ui.body_preparations, prepared);
        let a = h.ui.probe.widgets["image-modal"].center();
        let b = a + Vec2::new(45., 30.);
        h.frame(vec![
            Event::PointerMoved(a),
            Event::PointerButton {
                pos: a,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ]);
        h.frame(vec![Event::PointerMoved(b)]);
        h.frame(vec![Event::PointerButton {
            pos: b,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
        assert!(h.ui.probe.image_pan.length() > 10.);
        h.click("image-fit");
        assert_eq!(h.ui.probe.image_zoom, 1.);
        assert_eq!(h.ui.probe.image_pan, Vec2::ZERO);
        assert_eq!(h.ui.body_preparations, prepared);
        assert_eq!(h.ui.probe.image_textures, 1);
        h.escape();
        assert!(!h.ui.probe.widgets.contains_key("image-modal"));
        h.click("body-raw");
        let (out, _) = h.click("copy-detail");
        assert_eq!(copied(&out), Some(encoded.as_str()));
        h.click("body-preview");
        f16_body(
            &mut h,
            json!({"statusCode":200,"body":"broken","headers":{"content-type":"image/png"}}),
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while h.ui.probe.body_preview.is_empty() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(2));
            h.frame(vec![]);
        }
        assert!(h.ui.probe.body_preview.contains("unavailable"));
        assert_eq!(h.ui.probe.image_textures, 0);
        if width >= 1100. {
            h.click("clear");
        }
        h.store.clear();
        h.frame(vec![]);
        assert_eq!(h.ui.probe.retained_detail_bytes, 0);
        assert_eq!(h.ui.probe.image_textures, 0);
        assert!(h.ui.selected_request.is_none());
    }
}

#[test]
fn f16_lists_and_json_build_only_viewport_and_detail_cache_ignores_other_events() {
    let mut h = Harness::new(1440.);
    let mut envelope = fixture();
    envelope["events"] = json!((0..3000).map(|i| json!({"requestId":format!("stress-{i}"),"source":"js.fetch","phase":"request","request":{"method":"GET","url":format!("https://x/{i}")}})).collect::<Vec<_>>());
    h.store.ingest(&envelope).unwrap();
    h.frame(vec![]);
    assert!(h.ui.probe.network_rows_rendered < 60);
    h.click("request:ios-session-1:stress-2999");
    let key = h.ui.selected_request.clone();
    h.ui.settings.set_request_detail_tab(4).unwrap();
    envelope["events"] = json!([{"requestId":"stress-2999","source":"js.fetch","phase":"response","response":{"statusCode":200,"body":json!((0..4000).collect::<Vec<_>>()).to_string()}}]);
    h.store.ingest(&envelope).unwrap();
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.ui.probe.json.rows.len() < 60);
    assert!(h.ui.probe.json.total_rows > 4000);
    let preparations = h.ui.detail_preparations;
    let body_preparations = h.ui.body_preparations;
    envelope["events"] = json!([{"source":"js.console","level":"info","message":"unrelated"}]);
    h.store.ingest(&envelope).unwrap();
    for _ in 0..20 {
        h.frame(vec![]);
    }
    assert_eq!(h.ui.selected_request, key);
    assert_eq!(h.ui.detail_preparations, preparations);
    assert_eq!(h.ui.body_preparations, body_preparations);
    h.click("Console");
    envelope["events"] = json!(
        (0..3000)
            .map(|i| json!({"source":"js.console","level":"info","message":format!("line-{i}")}))
            .collect::<Vec<_>>()
    );
    h.store.ingest(&envelope).unwrap();
    h.frame(vec![]);
    assert!(h.ui.probe.console_rows_rendered < 60);
}

#[test]
fn f16_virtual_json_cross_line_unicode_selection_and_offscreen_find() {
    let value = json!({"first":"😀 tiếng Việt", "second":"line two", "items":(0..1000).map(|i|format!("row-{i}")).collect::<Vec<_>>()});
    let mut h = json_harness(&value.to_string());
    let rect = h.ui.probe.widgets["json-output"];
    // Start at the end of the opening brace and drag into the third row.
    let a = egui::pos2(rect.left() + 61., rect.top() + 5.);
    let b = egui::pos2(rect.left() + 220., rect.top() + 55.);
    h.frame(vec![
        Event::PointerMoved(a),
        Event::PointerButton {
            pos: a,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
    ]);
    h.frame(vec![Event::PointerMoved(b)]);
    h.frame(vec![Event::PointerButton {
        pos: b,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    let (out, _) = h.frame(vec![Event::Copy]);
    let selection = copied(&out).expect("cross-line selection clipboard");
    assert!(selection.contains("😀 tiếng Việt"), "{selection:?}");
    assert!(selection.contains('\n'));
    assert!(!selection.contains("1 -"));
    h.text("json-find", "row-999");
    for _ in 0..5 {
        h.frame(vec![]);
    }
    assert!(h.ui.probe.json.scroll.y > 1000.);
    assert!(h.ui.probe.json.highlighted > 0);
    assert!(h.ui.probe.json.rows.len() < 50);
}
#[test]
fn f16_clear_external_narrow_and_wide_drops_json_and_image_inspectors() {
    for width in [860., 1440.] {
        let mut h = Harness::new(width);
        f16_body(&mut h, json!({"statusCode":200,"body":"{\"private\":42}"}));
        f16_select_body(&mut h);
        assert!(h.ui.probe.retained_detail_bytes > 0);
        h.store.clear();
        h.frame(vec![]);
        assert!(h.ui.selected_request.is_none());
        assert_eq!(h.ui.probe.retained_detail_bytes, 0);
        assert!(!h.ui.probe.widgets.contains_key("json-output"));
        assert!(!h.ui.probe.widgets.contains_key("image-modal"));
    }
}

#[test]
fn f16_body_scalar_and_form_colors_copy_raw_value() {
    let mut h = Harness::new(1440.);
    f16_body(
        &mut h,
        json!({"statusCode":200,"headers":{"content-type":"application/x-www-form-urlencoded"},"body":"paint=%23ff0000"}),
    );
    f16_select_body(&mut h);
    let (out, _) = h.click("body-color:0");
    assert_eq!(copied(&out), Some("#ff0000"));
    f16_body(
        &mut h,
        json!({"statusCode":200,"body":"rgba(255, 0, 0, .5)"}),
    );
    let (out, _) = h.click("body-color");
    assert_eq!(copied(&out), Some("rgba(255, 0, 0, .5)"));
}
#[test]
fn f16_raw_network_is_lazy_stable_and_updates_only_selected_record() {
    let mut h = Harness::new(1440.);
    f16_select_body(&mut h);
    let before = h.ui.network_raw_serializations;
    for _ in 0..10 {
        h.frame(vec![]);
    }
    assert_eq!(h.ui.network_raw_serializations, before);
    h.ui.reveal_inspector_tab = Some(5);
    h.frame(vec![]);
    h.frame(vec![]);
    h.click("tab:5");
    assert_eq!(h.ui.network_raw_serializations, before + 1);
    for _ in 0..10 {
        h.frame(vec![]);
    }
    assert_eq!(h.ui.network_raw_serializations, before + 1);
    let mut envelope = fixture();
    envelope["events"] = json!([{"requestId":"fetch-1","source":"js.fetch","phase":"response","response":{"statusCode":204}}]);
    h.store.ingest(&envelope).unwrap();
    h.frame(vec![]);
    assert_eq!(h.ui.network_raw_serializations, before + 1);
    f16_body(&mut h, json!({"statusCode":201,"body":"new"}));
    assert_eq!(h.ui.network_raw_serializations, before + 2);
}

#[test]
fn f16_multipart_files_request_response_and_body_identity_invalidation() {
    let mut h = Harness::new(1440.);
    let raw = "--b\r\nContent-Disposition: form-data; name=\"name\"\r\n\r\nAn\r\n--b\r\nContent-Disposition: form-data; name=\"file\"; filename=\"a.bin\"\r\nContent-Type: application/octet-stream\r\n\r\nABC\r\n--b--\r\n";
    let data = json!({"statusCode":200,"headers":{"Content-Type":"multipart/form-data; boundary=b"},"body":raw});
    f16_body(&mut h, data.clone());
    f16_select_body(&mut h);
    assert!(h.ui.probe.body_preview.contains("name: An"));
    assert!(
        h.ui.probe
            .body_preview
            .contains("a.bin · application/octet-stream · 3 bytes")
    );
    let prepared = h.ui.body_preparations;
    let mut metadata = data.clone();
    metadata["statusCode"] = json!(201);
    f16_body(&mut h, metadata);
    assert_eq!(
        h.ui.body_preparations, prepared,
        "metadata-only updates must retain prepared body"
    );
    let mut envelope = fixture();
    envelope["events"] = json!([{"requestId":"fetch-3","source":"js.fetch","phase":"request","request":{"method":"POST","url":"https://x/","headers":data["headers"],"body":raw}}]);
    h.store.ingest(&envelope).unwrap();
    h.click("tab:2");
    assert_eq!(h.ui.probe.body_kind, "form");
    assert!(h.ui.probe.body_preview.contains("name: An"));
    h.click("tab:4");
    let changed =
        json!({"statusCode":201,"headers":{"content-type":"text/plain"},"body":"ordinary"});
    f16_body(&mut h, changed);
    assert!(h.ui.probe.body_kind.is_empty());
    assert!(!h.ui.probe.widgets.contains_key("body-preview"));
}

#[test]
fn f16_console_append_revisits_bounded_tail_and_keeps_open_raw_cache() {
    let mut h = Harness::new(1440.);
    console_event(&mut h,"ios-session-1",json!((0..3000).map(|i|json!({"type":"console","source":"dart.stdout","message":format!("message {i}")})).collect::<Vec<_>>()));
    h.click("Console");
    h.click("log:2");
    h.click("console-raw");
    let encodes = h.ui.console_raw_serializations;
    let prepared = h.ui.console_group_records_prepared;
    console_event(
        &mut h,
        "ios-session-1",
        json!([{"type":"console","source":"dart.stdout","message":"append"}]),
    );
    assert!(h.ui.console_group_records_prepared - prepared <= 401);
    assert_eq!(h.ui.console_raw_serializations, encodes);
    // Also exercise an open raw cache that lies inside the regrouped tail.
    let mut small = Harness::new(1440.);
    small.click("Console");
    small.click("log:0");
    small.click("console-raw");
    let encodes = small.ui.console_raw_serializations;
    console_event(
        &mut small,
        "ios-session-1",
        json!([{"type":"console","source":"dart.stdout","message":"append"}]),
    );
    assert_eq!(small.ui.console_raw_serializations, encodes);
}
#[test]
fn f16_discarded_image_worker_never_replaces_new_body_after_clear() {
    use base64::Engine;
    let mut h = Harness::new(1440.);
    f16_select_body(&mut h);
    let encoded =
        base64::engine::general_purpose::STANDARD.encode(include_bytes!("fixtures/preview.png"));
    for _ in 0..12 {
        f16_body(
            &mut h,
            json!({"statusCode":200,"body":encoded,"bodyEncoding":"base64","headers":{"content-type":"image/png"}}),
        );
        f16_body(
            &mut h,
            json!({"statusCode":200,"body":"plain current body"}),
        );
    }
    std::thread::sleep(std::time::Duration::from_millis(30));
    h.frame(vec![]);
    assert_eq!(h.ui.probe.inspector_text, "plain current body");
    assert_eq!(h.ui.probe.image_textures, 0);
    h.store.clear();
    h.frame(vec![]);
    std::thread::sleep(std::time::Duration::from_millis(30));
    h.frame(vec![]);
    assert_eq!(h.ui.probe.image_textures, 0);
    assert_eq!(h.ui.probe.retained_detail_bytes, 0);
}

#[test]
fn f16_clear_then_same_ids_before_next_frame_cannot_reuse_stale_detail() {
    let mut h = Harness::new(1440.);
    f16_body(&mut h, json!({"statusCode":200,"body":"private old body"}));
    f16_select_body(&mut h);
    assert!(h.ui.probe.inspector_text.contains("private old"));
    h.store.clear();
    h.store.ingest(&fixture()).unwrap();
    h.frame(vec![]);
    assert!(h.ui.selected_request.is_none());
    assert_eq!(h.ui.probe.retained_detail_bytes, 0);
}
#[test]
fn f16_select_all_copies_full_large_document_beyond_probe_budget() {
    let value = json!(
        (0..20000)
            .map(|i| format!("😀 row-{i}"))
            .collect::<Vec<_>>()
    );
    let mut h = json_harness(&value.to_string());
    assert!(h.ui.probe.json.text.len() < value.to_string().len());
    h.click("json-output");
    h.frame(vec![shortcut(egui::Key::A, false)]);
    let (out, _) = h.frame(vec![Event::Copy]);
    assert_eq!(
        copied(&out),
        Some(serde_json::to_string_pretty(&value).unwrap().as_str())
    );
}

#[test]
fn empty_filter_results_are_text_only_and_toolbar_can_reset() {
    let mut h = Harness::new(1360.);
    h.text("search", "no-such-entry-parity");
    assert!(h.ui.probe.requests.is_empty());
    assert!(!h.ui.probe.widgets.contains_key("empty-reset"));
    assert!(!h.ui.probe.widgets.contains_key("empty-connect"));
    h.click("reset");
    assert_eq!(h.ui.probe.requests.len(), 3);
    h.click("Console");
    h.text("search", "no-such-entry-parity");
    assert!(h.ui.probe.logs.is_empty());
    assert!(!h.ui.probe.widgets.contains_key("empty-reset"));
    h.click("reset");
    assert_eq!(h.ui.probe.logs.len(), 1);
}

#[test]
fn network_columns_follow_flutter_order_with_two_line_url_rows() {
    let h = Harness::new(1360.);
    let p = &h.ui.probe.widgets;
    assert!(p["header:method"].left() < p["header:url"].left());
    assert!(p["header:url"].left() < p["header:status"].left());
    assert!(p["header:status"].left() < p["header:time"].left());
    assert!(p["request:ios-session-1:fetch-1"].height() >= 48.);
}

#[test]
fn json_workspace_has_uniform_outer_padding_and_sixteen_pixel_divider() {
    for width in [700., 1360.] {
        let mut h = Harness::new(width);
        h.click("JSON");
        let p = &h.ui.probe.widgets;
        assert_eq!(p["json-divider"].width(), 16.);
        assert!((p["json-input"].top() - p["nav-toolbar"].bottom() - 8.).abs() < 1.);
        assert!((p["json-output-pane"].right() - (width - 8.)).abs() < 1.);
        assert!((p["json-output-pane"].top() - p["json-input"].top()).abs() < 1.);
        assert!((p["json-output-pane"].bottom() - p["json-input"].bottom()).abs() < 1.);

        assert!((p["json-input"].left() - 8.).abs() < 1.);
        assert!((p["json-input"].bottom() - (p["footer"].top() - 8.)).abs() < 3.);
    }
}

#[test]
fn request_selection_and_body_never_resize_the_inspector() {
    let mut h = Harness::new(1360.);
    h.ui.settings.set_inspector_width(580., false).unwrap();
    h.ctx = egui::Context::default();
    let mut data = fixture();
    data["events"][0]["request"]["url"] =
        json!(format!("https://example.com/{}", "long-path/".repeat(300)));
    h.store.ingest(&data).unwrap();
    h.frame(vec![]);
    let left = h.ui.probe.list_rect.right();
    assert!((left - (1360. - 580.)).abs() < 1., "{left}");
    for key in [
        "request:ios-session-1:fetch-1",
        "request:ios-session-1:xhr-2",
        "request:ios-session-1:fetch-1",
    ] {
        h.click(key);
        for tab in [0, 4, 2, 5] {
            h.click(&format!("tab:{tab}"));
            assert_eq!(h.ui.settings.values().request_detail_tab, tab);
            for _ in 0..4 {
                h.frame(vec![]);
            }
            assert!(
                (h.ui.probe.list_rect.right() - left).abs() < 1.,
                "selection/tab {tab} moved split: {left} -> {}",
                h.ui.probe.list_rect.right()
            );
        }
    }
    assert_eq!(h.ui.settings.values().network_inspector_width, Some(580.));
    let edge = Pos2::new(left, h.ui.probe.list_rect.center().y);
    h.frame(vec![
        Event::PointerMoved(edge),
        Event::PointerButton {
            pos: edge,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
    ]);
    let moved = edge - Vec2::new(56., 0.);
    h.frame(vec![Event::PointerMoved(moved)]);
    h.frame(vec![Event::PointerButton {
        pos: moved,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![]);
    assert!((h.ui.probe.list_rect.right() - moved.x).abs() < 1.);
    let saved = h.ui.settings.values().network_inspector_width.unwrap();
    assert!((saved - 636.).abs() < 1.);
    h.click("request:ios-session-1:xhr-2");
    assert!((h.ui.probe.list_rect.right() - moved.x).abs() < 1.);
}

#[test]
fn json_body_uses_plain_line_numbers_compact_toolbar_and_terminal_tree() {
    let mut h = Harness::new(1440.);
    f16_body(
        &mut h,
        json!({"statusCode":200,"body":r#"{"group":{"id":"123-blue","flag":true,"count":42,"empty":null}}"#}),
    );
    f16_select_body(&mut h);
    let (out, _) = h.frame(vec![]);
    assert!(!out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "1 -" || t.galley.text() == "2 -")));
    let toolbar = h.ui.probe.widgets["json-toolbar"];
    assert!((toolbar.height() - 30.).abs() < 1.);
    let output = h.ui.probe.widgets["json-output"];
    assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "1" && t.galley.job.sections[0].format.font_id.size == 14.)));
    assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text().contains("123-blue") && t.galley.job.sections.iter().all(|part| part.format.font_id.size == 14. && part.format.line_height == Some(22.)))));
    let value_color = flutter_network_viewer::app_colors::JSON_LIGHT[2];
    assert!(out.shapes.iter().any(|s| {
        let egui::Shape::Text(t) = &s.shape else {
            return false;
        };
        let rendered = t.galley.text();
        let Some(start) = rendered.find("123-blue") else {
            return false;
        };
        let end = start + "123-blue".len();
        t.galley
            .job
            .sections
            .iter()
            .filter(|part| part.byte_range.start < end && part.byte_range.end > start)
            .all(|part| part.format.color == value_color)
    }));
    let tabs = h.ui.probe.widgets["inspector-tabs"];
    assert_eq!(toolbar.top() - tabs.bottom(), 6., "tabs to JSON toolbar");
    assert_eq!(
        output.top() - toolbar.bottom(),
        12.,
        "toolbar to first JSON row"
    );
    h.click("json-tree");
    let (out, _) = h.frame(vec![]);
    let widgets = &h.ui.probe.widgets;
    assert_eq!(
        widgets["json-toolbar"].top() - widgets["inspector-tabs"].bottom(),
        6.
    );
    assert_eq!(
        widgets["json-output"].top() - widgets["json-toolbar"].bottom(),
        12.
    );
    assert!(!out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "1" || t.galley.text() == "2")));
    assert!(
        out.shapes
            .iter()
            .any(|s| matches!(&s.shape, egui::Shape::LineSegment { .. }))
    );
    assert!(
        out.shapes
            .iter()
            .any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text().contains("items")))
    );
}

#[test]
fn json_scrollbars_are_hidden_but_wheel_still_scrolls() {
    let value = json!({"items": (0..140).map(|i| format!("row-{i}-{}", "long".repeat(70))).collect::<Vec<_>>()});
    let mut h = json_harness(&value.to_string());
    // Use solid bars with a distinctive thickness so any painted bar is observable.
    h.ctx.style_mut(|s| {
        s.spacing.scroll = egui::style::ScrollStyle::solid();
        s.spacing.scroll.bar_width = 17.;
    });
    let mut output = h.frame(vec![]).0;
    for _ in 0..8 {
        output = h.frame(vec![]).0;
    }
    assert!(
        !output
            .shapes
            .iter()
            .any(|s| matches!(&s.shape, egui::Shape::Rect(r)
        if ((r.rect.width() - 17.).abs() < 0.5 && r.rect.height() > 30.)
            || ((r.rect.height() - 17.).abs() < 0.5 && r.rect.width() > 30.))),
        "scrollbar was painted"
    );
    let pos = h.ui.probe.widgets["json-output"].center();
    h.frame(vec![
        Event::PointerMoved(pos),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: Vec2::new(-500., -800.),
            modifiers: Modifiers::NONE,
        },
    ]);
    h.frame(vec![]);
    assert!(h.ui.probe.json.scroll.x > 0. && h.ui.probe.json.scroll.y > 0.);
}

#[test]
fn request_and_json_copy_buttons_paint_material_icons_and_copy_data() {
    let mut h = Harness::new(1440.);
    h.click("request:ios-session-1:fetch-1");
    h.click("tab:4");
    let (output, _) = h.frame(vec![]);
    for key in ["curl", "json-copy"] {
        let rect = h.ui.probe.widgets[key];
        assert!(output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t)
            if t.galley.text() == "\u{ef7f}" && rect.contains(t.pos)
                && t.galley.job.sections[0].format.font_id.family == egui::FontFamily::Name("MaterialIcons".into()))),
            "{key} does not paint the Material copy icon");
        assert!(
            !output
                .shapes
                .iter()
                .any(|s| matches!(&s.shape, egui::Shape::Text(t)
            if rect.contains(t.pos) && t.galley.text().starts_with("Copy")))
        );
        let (out, _) = h.click(key);
        let text = copied(&out).expect("clipboard command");
        assert!(text.contains(if key == "curl" {
            "curl -X POST"
        } else {
            "created"
        }));
    }
}

#[test]
fn inspector_url_is_left_aligned_and_metadata_tables_fill_the_viewport() {
    let mut h = Harness::new(1440.);
    h.click("request:ios-session-1:fetch-1");
    h.click("tab:0");
    let (out, _) = h.frame(vec![]);
    let url = h.ui.probe.widgets["inspector-url"];
    assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t)
        if t.galley.text().starts_with("https://api.example.com/") && (t.pos.x-url.left()).abs()<1.)));
    for tab in [0, 1, 3, 5] {
        h.click(&format!("tab:{tab}"));
        let (out, _) = h.frame(vec![]);
        if tab == 5 {
            assert!(h.ui.probe.json.total_rows > 5);
            assert!(h.ui.probe.widgets.contains_key("json-output"));
            let (copy_out, _) = h.click("json-copy");
            let copied_record: Value =
                serde_json::from_str(copied(&copy_out).expect("raw clipboard")).unwrap();
            assert_eq!(copied_record["requestId"], "fetch-1");
        } else {
            let widgets = &h.ui.probe.widgets;
            let grid = widgets["metadata:grid"];
            let viewport = widgets["metadata-viewport"];
            assert!((grid.left() - viewport.left()).abs() < 0.5);
            assert!((grid.right() - viewport.right()).abs() < 0.5);
            assert!((grid.top() - viewport.top()).abs() < 0.5);
            let frames: Vec<_> = out
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Rect(r)
                        if r.stroke.width > 0.
                            && r.rect.contains_rect(grid)
                            && grid.expand(16.).contains_rect(r.rect) =>
                    {
                        Some(r.rect)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(frames, [grid], "the table is the only bordered frame");
            assert!(!h.ui.probe.widgets.contains_key("copy-detail"));
            assert!(!out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t)
                if grid.contains(t.pos) && ["Overview", "Request header", "Response header"].contains(&t.galley.text()))));
            assert!(
                out.shapes
                    .iter()
                    .any(|s| matches!(&s.shape, egui::Shape::Text(t)
                if grid.contains(t.pos) && t.galley.text() == "Title"))
            );
        }
    }
}

#[test]
fn empty_headers_and_bodies_show_localized_no_data() {
    for lang in ["en", "vi"] {
        let mut h = Harness::new(1440.);
        h.ui.settings.set_language(lang).unwrap();
        h.click("request:ios-session-1:fetch-3");
        let message = if lang == "vi" {
            "Không có dữ liệu"
        } else {
            "No data"
        };
        for tab in [1, 2, 3, 4] {
            h.click(&format!("tab:{tab}"));
            let (out, _) = h.frame(vec![]);
            assert!(
                out.shapes
                    .iter()
                    .any(|s| matches!(&s.shape, egui::Shape::Text(t)
                if t.galley.text() == message)),
                "empty tab {tab} in {lang}"
            );
            assert!(!h.ui.probe.widgets.contains_key("copy-detail"));
        }
    }
}

#[test]
fn request_table_uses_paper_surface_and_opaque_ink_selection_with_readable_text() {
    let mut h = Harness::new(1440.);
    h.click("request:ios-session-1:fetch-1");
    // Move away so hover cannot mask the base surface.
    let (out, _) = h.frame(vec![Event::PointerMoved(Pos2::new(0., 0.))]);
    let list = h.ui.probe.list_rect;
    assert!(
        out.shapes
            .iter()
            .any(|s| matches!(&s.shape, egui::Shape::Rect(r)
        if r.fill == flutter_network_viewer::vintage::BACKGROUND && r.rect.contains(list.center())))
    );
    let row = h.ui.probe.widgets["row:ios-session-1:fetch-1"];
    let blue = flutter_network_viewer::app_colors::LIGHT.accent;
    assert!(
        out.shapes
            .iter()
            .any(|s| matches!(&s.shape, egui::Shape::Rect(r)
        if r.rect == row && r.fill == blue))
    );
    assert!(
        out.shapes
            .iter()
            .any(|s| matches!(&s.shape, egui::Shape::Text(t)
        if row.contains(t.pos) && t.galley.text().starts_with("/orders")
            && t.galley.job.sections[0].format.color == egui::Color32::WHITE))
    );
}

#[test]
fn inspector_url_scroll_reveals_end_keeps_copy_fixed_and_resets_per_request() {
    let mut h = Harness::new(1440.);
    let long_url = format!(
        "https://api.example.com/{}?last=visible-end",
        "long-path/".repeat(80)
    );
    let mut envelope = fixture();
    envelope["events"] = json!([{"requestId":"long-url", "source":"js.fetch", "phase":"request", "request":{"method":"GET", "url":long_url}}]);
    h.store.ingest(&envelope).unwrap();
    h.frame(vec![]);
    h.click("request:ios-session-1:long-url");
    let (out, _) = h.frame(vec![]);
    let viewport = h.ui.probe.widgets["inspector-url"];
    let copy_rect = h.ui.probe.widgets["curl"];
    let text_end = |out: &egui::FullOutput| {
        out.shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t)
                    if t.galley.text() == long_url
                        && s.clip_rect.intersects(viewport)
                        && t.pos.y < viewport.bottom() =>
                {
                    Some(t.pos.x + t.galley.size().x)
                }
                _ => None,
            })
            .expect("URL must render fully without ellipsis")
    };
    assert!(text_end(&out) > viewport.right());
    h.frame(vec![
        Event::PointerMoved(viewport.center()),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: Vec2::new(-100_000., 0.),
            modifiers: Modifiers::NONE,
        },
    ]);
    let mut out = h.frame(vec![]).0;
    for _ in 0..20 {
        out = h.frame(vec![]).0;
    }
    assert!(
        text_end(&out) <= viewport.right() + 1.,
        "URL tail must be visible"
    );
    assert!(text_end(&out) > viewport.left());
    assert_eq!(h.ui.probe.widgets["curl"], copy_rect);
    assert!(viewport.right() < copy_rect.left());
    let (out, _) = h.click("curl");
    assert!(copied(&out).unwrap().contains(&long_url));
    h.click("request:ios-session-1:fetch-1");
    let (out, _) = h.frame(vec![]);
    let viewport = h.ui.probe.widgets["inspector-url"];
    assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t)
        if t.galley.text() == "https://api.example.com/orders?draft=true" && (t.pos.x-viewport.left()).abs()<1.)));
}

#[test]
fn json_find_clear_uses_same_material_icon_size_as_navigation() {
    let mut h = json_harness(r#"{"value":"needle"}"#);
    h.click("json-find");
    h.frame(vec![Event::Text("needle".into())]);
    let (out, _) = h.frame(vec![]);
    for (key, code) in [
        ("json-prev", '\u{e0a0}'),
        ("json-next", '\u{e097}'),
        ("json-clear", '\u{e16a}'),
    ] {
        let rect = h.ui.probe.widgets[key];
        assert_eq!(rect.size(), Vec2::splat(24.));
        assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t)
            if rect.contains(t.pos) && t.galley.text() == code.to_string()
                && t.galley.job.sections[0].format.font_id == egui::FontId::new(14., egui::FontFamily::Name("MaterialIcons".into())))), "icon {key}");
    }
    h.click("json-clear");
    assert_eq!(h.ui.probe.json.query, "");
}

#[test]
fn collapsed_json_objects_and_arrays_show_closing_delimiter_and_keep_copy_complete() {
    for (source, row, expected) in [
        (
            r#"{"object":{"child":1},"tail":2}"#,
            1,
            r#"  "object": { ... },"#,
        ),
        (r#"{"array":[1,2]}"#, 1, r#"  "array": [ ... ]"#),
        (r#"{"object":{"child":1}}"#, 0, "{ ... }"),
        ("[1,2]", 0, "[ ... ]"),
    ] {
        let mut h = json_harness(source);
        let full = h.ui.probe.json.text.clone();
        h.click(&format!("json-fold:{row}"));
        let (out, _) = h.frame(vec![]);
        assert!(
            out.shapes
                .iter()
                .any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == expected)),
            "missing {expected}"
        );
        let (out, _) = h.click("json-copy");
        assert_eq!(copied(&out), Some(full.as_str()));
        h.click(&format!("json-fold:{row}"));
        assert!(h.ui.probe.json.rows.len() > 1);
    }
}

#[test]
fn network_filters_live_in_header_and_search_has_compact_padding() {
    let mut h = Harness::new(1440.);
    let (out, _) = h.frame(vec![]);
    for (key, header) in [("method", "header:method"), ("status", "header:status")] {
        let rect = h.ui.probe.widgets[key];
        let cell = h.ui.probe.widgets[header];
        assert!(cell.contains_rect(rect));
        assert!((cell.right() - rect.right() - 4.).abs() < 1.);
        assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Rect(r) if r.rect==rect && r.corner_radius==egui::CornerRadius::ZERO && r.fill==flutter_network_viewer::vintage::CONTROL && r.stroke.width==1.)));
    }
    let frame = h.ui.probe.widgets["search-frame"];
    let icon = h.ui.probe.widgets["search-icon"];
    let input = h.ui.probe.widgets["search"];
    assert!((icon.left() - frame.left() - 8.).abs() < 1.);
    assert!((input.left() - icon.right() - 4.).abs() < 1.);
    assert!((input.top() - frame.top() - 3.).abs() < 1.);
    assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t)
        if t.galley.text()=="Filter URL, method, status…" && t.galley.job.sections[0].format.font_id.size==14.
            && t.galley.job.sections[0].format.color==flutter_network_viewer::app_colors::LIGHT.muted)));
    h.click("method");
    h.click("method:GET");
    h.click("status");
    h.click("status:Pending");
    assert_eq!(
        h.ui.probe.requests,
        vec![("ios-session-1".into(), "fetch-3".into())]
    );
}

#[test]
fn header_filter_can_recover_from_empty_result() {
    let mut h = Harness::new(1440.);
    h.click("method");
    h.click("method:DELETE");
    assert!(h.ui.probe.requests.is_empty());
    assert!(h.ui.probe.widgets["header:method"].contains_rect(h.ui.probe.widgets["method"]));
    h.click("method");
    h.click("method:All methods");
    assert_eq!(h.ui.probe.requests.len(), 3);
}

#[test]
fn network_filter_menus_show_all_and_restore_unfiltered_requests() {
    for language in ["en", "vi"] {
        let mut h = Harness::new(1440.);
        h.ui.settings.set_language(language).unwrap();
        h.frame(vec![]);
        let total = h.ui.probe.requests.len();
        assert_eq!(total, 3);
        for (filter, restricted, all) in [
            ("method", "method:DELETE", "method:All methods"),
            ("status", "status:Pending", "status:All statuses"),
        ] {
            h.click(filter);
            h.click(restricted);
            assert!(h.ui.probe.requests.len() < total);
            h.click(filter);
            let output = h.frame(vec![]).0;
            let option = h.ui.probe.widgets[all];
            assert!(
                output
                    .shapes
                    .iter()
                    .any(|s| matches!(&s.shape, egui::Shape::Text(t)
                    if t.galley.text() == "ALL" && option.contains(t.pos))),
                "{filter} must show ALL in {language}"
            );
            h.click(all);
            assert_eq!(h.ui.probe.requests.len(), total);
        }
    }
}

#[test]
fn manual_connection_controls_and_app_list_live_in_first_header_row() {
    let mut h = Harness::new(1440.);
    let header = h.ui.probe.widgets["header"];
    for key in [
        "app:all",
        "app:ios-session-1",
        "manual-url",
        "scan",
        "settings",
        "help",
    ] {
        assert!(
            header.contains_rect(h.ui.probe.widgets[key]),
            "{key} outside header1"
        );
    }
    assert!(!h.ui.probe.widgets.contains_key("connect"));
    assert!(h.ui.probe.widgets["scan"].right() < h.ui.probe.widgets["manual-url"].left());
    h.text("vm-uri", "http://127.0.0.1:8181/token=/");
    let (_, actions) = h.click("connect-submit");
    assert_eq!(
        actions,
        vec![Action::Connect("ws://127.0.0.1:8181/token=/ws".into())]
    );
    let (_, actions) = h.click("scan");
    assert_eq!(actions, vec![Action::StartScan]);
    h.status.targets = vec![
        flutter_network_viewer::discovery::Target {
            name: "A".into(),
            uri: "ws://127.0.0.1:1/ws".into(),
            vm_key: "a".into(),
            project_key: "a".into(),
        },
        flutter_network_viewer::discovery::Target {
            name: "B".into(),
            uri: "ws://127.0.0.1:2/ws".into(),
            vm_key: "b".into(),
            project_key: "b".into(),
        },
    ];
    h.frame(vec![]);
    h.frame(vec![]);
    assert!(h.ui.probe.widgets["target:a"].bottom() < h.ui.probe.widgets["target:b"].top());
    let (_, actions) = h.click("target:b");
    assert_eq!(actions, vec![Action::SelectTarget("b".into())]);
}

#[test]
fn narrow_header_keeps_scan_and_manual_buttons_visible() {
    let mut h = Harness::new(480.);
    let header = h.ui.probe.widgets["header"];
    for key in [
        "Network",
        "Console",
        "JSON",
        "project-list",
        "scan",
        "manual-url",
        "settings",
        "help",
    ] {
        assert!(header.contains_rect(h.ui.probe.widgets[key]), "{key}");
    }
    let (_, actions) = h.click("scan");
    assert_eq!(actions, vec![Action::StartScan]);
    h.click("manual-url");
    assert!(h.ui.probe.widgets.contains_key("vm-uri"));
    h.escape();
    assert!(!h.ui.probe.widgets.contains_key("vm-uri"));
}

#[test]
fn header_connection_controls_align_right_with_manual_popup_button() {
    for width in [1360., 1600.] {
        let mut h = Harness::new(width);
        let (out, _) = h.frame(vec![]);
        let scan = h.ui.probe.widgets["scan"];
        assert!(
            out.shapes
                .iter()
                .any(|shape| matches!(&shape.shape, egui::Shape::Text(t)
            if scan.contains(t.pos) && t.galley.text().contains("Find & connect")))
        );
        let manual = h.ui.probe.widgets["manual-url"];
        let help = h.ui.probe.widgets["help"];
        assert!((scan.top() - manual.top()).abs() < 0.5);
        assert_eq!(scan.height(), 24.);
        assert_eq!(manual.height(), 24.);
        assert!((manual.left() - scan.right() - 6.).abs() < 1.);
        assert!((h.ui.probe.widgets["header-content"].right() - help.right()).abs() < 1.);
        assert!(!out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if h.ui.probe.widgets["header"].contains(t.pos) && ["Use","Use URL","URL"].contains(&t.galley.text()))));
    }
}

#[test]
fn project_list_is_right_grouped_with_zero_outer_and_eight_by_four_chip_padding() {
    for width in [1360., 1600.] {
        let mut h = Harness::new(width);
        let (out, _) = h.frame(vec![]);
        let list = h.ui.probe.widgets["project-list"];
        assert!(list.width() >= 360.);
        let all = h.ui.probe.widgets["app:all"];
        let scan = h.ui.probe.widgets["scan"];
        assert!((scan.left() - list.right() - 25.).abs() < 1.);
        assert!((all.left() - list.left()).abs() <= 0.51);
        assert!(
            (all.top() - list.top()).abs() <= 0.51,
            "all {all:?}, list {list:?}"
        );
        assert!((all.bottom() - list.bottom()).abs() <= 0.51);
        for key in ["app:all", "app:ios-session-1"] {
            let chip = h.ui.probe.widgets[key];
            let text = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if chip.contains(t.pos) => Some(t),
                    _ => None,
                })
                .unwrap();
            assert!((text.pos.x - chip.left() - 8.).abs() <= 0.51);
            assert!((text.pos.y - chip.top() - 4.).abs() <= 0.51);
            assert!((chip.height() - text.galley.size().y - 8.).abs() <= 0.51);
            assert!((chip.width() - text.galley.size().x - 16.).abs() <= 0.51);
        }
    }
}

#[test]
fn search_is_350_wide_in_navigation_before_capture_actions() {
    let mut h = Harness::new(1360.);
    for page in ["Network", "Console"] {
        h.click(page);
        let search = h.ui.probe.widgets["search-frame"];
        let nav = h.ui.probe.widgets["nav-toolbar"];
        let pause = h.ui.probe.widgets["pause"];
        assert!((search.width() - 350.).abs() < 1.);
        assert!(nav.contains_rect(search));
        assert!(
            (pause.left() - search.right() - 25.).abs() < 1.,
            "search {search:?} pause {pause:?}"
        );
        assert!(
            (pause.center().y - search.center().y).abs() < 1.,
            "pause {pause:?}, search {search:?}"
        );
        if page == "Network" {
            assert!(!h.ui.probe.widgets.contains_key("filters"));
        }
    }
}

#[test]
fn compact_header_manual_popup_and_console_header_filters_work() {
    let mut h = Harness::new(1360.);
    let header = h.ui.probe.widgets["header"];
    assert_eq!(header.height(), 32.);
    assert_eq!(h.ui.probe.widgets["nav-toolbar"].height(), 32.);
    for key in [
        "Network",
        "Console",
        "JSON",
        "project-list",
        "scan",
        "manual-url",
        "settings",
        "help",
    ] {
        assert!(header.contains_rect(h.ui.probe.widgets[key]), "{key}");
    }
    assert_eq!(
        h.ui.probe.widgets["Network"].width(),
        h.ui.probe.widgets["JSON"].width()
    );
    assert!(!h.ui.probe.widgets.contains_key("vm-uri"));
    h.click("manual-url");
    h.text("vm-uri", "bad URL");
    let (_, actions) = h.click("connect-submit");
    assert!(actions.is_empty());
    assert!(h.ui.probe.widgets.contains_key("vm-uri"));
    h.click("vm-uri");
    h.frame(vec![Event::Key {
        key: egui::Key::A,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::COMMAND,
    }]);
    h.frame(vec![Event::Text("http://127.0.0.1:8181/token=/".into())]);
    let (_, actions) = h.click("connect-submit");
    assert_eq!(
        actions,
        vec![Action::Connect("ws://127.0.0.1:8181/token=/ws".into())]
    );
    assert!(!h.ui.probe.widgets.contains_key("vm-uri"));
    h.click("Console");
    assert!(!h.ui.probe.widgets.contains_key("filters"));
    for (key, header) in [
        ("source", "console-header:source"),
        ("level", "console-header:level"),
    ] {
        let rect = h.ui.probe.widgets[key];
        let cell = h.ui.probe.widgets[header];
        assert_eq!(rect.size(), Vec2::splat(24.));
        assert!(cell.contains_rect(rect));
        assert_eq!(rect.center().y, cell.center().y);
    }
    h.click("level");
    h.click("level:error");
    assert!(h.ui.probe.logs.is_empty());
    h.click("level");
    h.click("level:All levels");
    assert_eq!(h.ui.probe.logs.len(), 1);
}

fn assert_input_border(out: &egui::FullOutput, rect: Rect, color: egui::Color32) {
    assert!(
        out.shapes
            .iter()
            .any(|s| matches!(&s.shape, egui::Shape::Rect(r)
        if r.rect == rect && s.clip_rect.contains_rect(rect)
            && r.stroke.width>=1. && r.stroke.color==color)),
        "missing border {color:?} at {rect:?}; nearby shapes: {:?}",
        out.shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Rect(r) if (r.rect.center() - rect.center()).length() < 3. =>
                    Some((r.rect, r.stroke, s.clip_rect)),
                _ => None,
            })
            .collect::<Vec<_>>()
    );
}
#[test]
fn text_inputs_keep_vintage_default_and_focused_borders() {
    fn check_find(h: &mut Harness, key: &str, p: flutter_network_viewer::desktop_style::Palette) {
        assert_input_border(&h.frame(vec![]).0, h.ui.probe.widgets[key], p.line);
        h.text(key, "created");
        assert_eq!(h.ui.probe.json.query, "created");
        assert_input_border(&h.frame(vec![]).0, h.ui.probe.widgets[key], p.accent);
        h.click("json-tree");
        assert_input_border(&h.frame(vec![]).0, h.ui.probe.widgets[key], p.line);
    }
    fn tab_away(h: &mut Harness) -> egui::FullOutput {
        h.frame(vec![Event::Key {
            key: egui::Key::Tab,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }]);
        h.frame(vec![]).0
    }
    for dark in [false, true] {
        let mut h = Harness::new(1360.);
        h.ui.settings
            .set_theme_mode(if dark {
                flutter_network_viewer::settings::ThemeMode::Dark
            } else {
                flutter_network_viewer::settings::ThemeMode::Light
            })
            .unwrap();
        let p = flutter_network_viewer::desktop_style::Palette::new(dark);
        let (out, _) = h.frame(vec![]);
        assert_input_border(&out, h.ui.probe.widgets["search-frame"], p.line);
        let (out, _) = h.click("search");
        assert_input_border(&out, h.ui.probe.widgets["search-frame"], p.accent);
        let (out, _) = h.frame(vec![Event::Text("orders".into())]);
        assert_input_border(&out, h.ui.probe.widgets["search-frame"], p.accent);
        h.click("reset");
        h.click("request:ios-session-1:fetch-1");
        h.click("tab:4");
        check_find(&mut h, "json-find", p);

        h.click("Console");
        let (out, _) = h.frame(vec![]);
        assert_input_border(&out, h.ui.probe.widgets["search-frame"], p.line);
        h.text("search", "log");
        assert_input_border(
            &h.frame(vec![]).0,
            h.ui.probe.widgets["search-frame"],
            p.accent,
        );

        h.click("manual-url");
        h.text("vm-uri", "ws://localhost:8181/ws");
        assert_input_border(&h.frame(vec![]).0, h.ui.probe.widgets["vm-uri"], p.accent);
        let out = tab_away(&mut h);
        assert_input_border(&out, h.ui.probe.widgets["vm-uri"], p.line);
        h.escape();
        h.click("export");
        // Wait for the modal fade so the border has its final palette color.
        for _ in 0..12 {
            h.frame(vec![]);
        }
        assert_input_border(
            &h.frame(vec![]).0,
            h.ui.probe.widgets["export-path"],
            p.line,
        );
        h.text("export-path", "capture.json");
        assert_input_border(
            &h.frame(vec![]).0,
            h.ui.probe.widgets["export-path"],
            p.accent,
        );
        h.escape();
        h.click("JSON");
        let (out, _) = h.frame(vec![]);
        assert_input_border(&out, h.ui.probe.widgets["json-input"], p.line);
        h.text("json-input", r#"{"created":true}"#);
        assert_input_border(
            &h.frame(vec![]).0,
            h.ui.probe.widgets["json-input"],
            p.accent,
        );
        h.click("json-format");
        check_find(&mut h, "json-workspace-find", p);
        h.click("json-download");
        h.click("export-path");
        assert_input_border(
            &h.frame(vec![]).0,
            h.ui.probe.widgets["export-path"],
            p.accent,
        );
    }
}

#[test]
fn workspace_header_formats_finds_copies_and_downloads_json() {
    let mut h = Harness::new(1360.);
    h.click("JSON");
    let nav = h.ui.probe.widgets["nav-toolbar"];
    for key in [
        "json-format",
        "json-workspace-find",
        "json-workspace-copy",
        "json-download",
    ] {
        assert!(nav.contains_rect(h.ui.probe.widgets[key]), "{key}");
    }
    h.text("json-input", r#"{"needle": 1, "other": 2}"#);
    h.click("json-format");
    h.text("json-workspace-find", "needle");
    assert_eq!(h.ui.probe.json.matches, 1);
    let (out, _) = h.click("json-workspace-copy");
    let expected = copied(&out).unwrap().to_owned();
    assert!(expected.contains("needle"));
    h.click("json-download");
    let path = std::env::temp_dir().join(format!("fnv-header-json-{}.json", std::process::id()));
    h.ui.export_path = path.to_string_lossy().into();
    h.click("save-export");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), expected);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn singleline_inputs_center_text_and_multiline_keeps_text_at_top() {
    fn text_rect(out: &egui::FullOutput, text: &str) -> Rect {
        out.shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == text => {
                    Some(Rect::from_min_size(t.pos, t.galley.size()))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing painted text {text}"))
    }
    let mut h = Harness::new(1360.);
    h.click("manual-url");
    let (out, _) = h.frame(vec![]);
    let field = h.ui.probe.widgets["vm-uri"];
    let text = text_rect(&out, "Enter URL");
    assert!(
        (text.center().y - field.center().y).abs() < 1.,
        "{text:?} vs {field:?}"
    );
    h.escape();
    h.click("JSON");
    let (out, _) = h.frame(vec![]);
    let field = h.ui.probe.widgets["json-workspace-find"];
    let text = text_rect(&out, "Find in JSON…");
    assert!(
        (text.center().y - field.center().y).abs() < 1.,
        "{text:?} vs {field:?}"
    );
    let field = h.ui.probe.widgets["json-input"];
    let text = text_rect(
        &out,
        "Enter your json input here. Example: {\"name\": \"value\"}",
    );
    assert!(text.top() >= field.top() && text.top() < field.top() + 30.);
    assert!(text.bottom() < field.center().y);
}

#[test]
fn failures_paint_only_in_notification_popup_and_do_not_repeat_each_frame() {
    fn only_popup(out: &egui::FullOutput, h: &Harness, message: &str) {
        let popup = h.ui.probe.widgets["notification"];
        let texts: Vec<_> = out
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text().contains(message) => Some(t),
                _ => None,
            })
            .collect();
        assert_eq!(texts.len(), 1, "duplicate or missing {message}");
        assert!(
            popup.contains(texts[0].pos),
            "message painted outside popup"
        );
        assert!(!h.ui.probe.widgets.contains_key("error-banner"));
    }
    let mut h = Harness::new(1360.);
    h.status.error = Some("VM transport failure".into());
    h.frame(vec![]);
    let (out, _) = h.frame(vec![]);
    only_popup(&out, &h, "VM transport failure");
    h.click("notification-confirm");
    h.frame(vec![]);
    assert!(
        !h.ui.probe.widgets.contains_key("notification"),
        "notice {}",
        h.ui.probe.notification_title
    );
    h.click("manual-url");
    h.text("vm-uri", "bad URL");
    h.click("connect-submit");
    let (out, _) = h.frame(vec![]);
    only_popup(&out, &h, "Invalid VM Service URL");
    h.click("notification-confirm");
    h.escape();
    h.click("JSON");
    h.text("json-input", "invalid");
    h.click("json-format");
    let (out, _) = h.frame(vec![]);
    only_popup(&out, &h, "Invalid JSON");
    h.click("notification-confirm");
    let (out, _) = h.frame(vec![]);
    assert!(
        !h.ui.probe.widgets.contains_key("notification"),
        "notice {}",
        h.ui.probe.notification_title
    );
    assert!(!out.shapes.iter().any(
        |s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text().contains("Invalid JSON"))
    ));
    h.click("json-format");
    h.frame(vec![]);
    assert!(
        h.ui.probe.widgets.contains_key("notification"),
        "retry reports failure again"
    );
}

#[test]
fn unified_font_setting_updates_all_painted_text_and_plain_network_grid() {
    fn check(out: &egui::FullOutput, size: f32) {
        for shape in &out.shapes {
            if let egui::Shape::Text(t) = &shape.shape {
                for section in &t.galley.job.sections {
                    if section.format.font_id.family
                        != egui::FontFamily::Name("MaterialIcons".into())
                    {
                        assert_eq!(section.format.font_id.size, size, "{}", t.galley.text());
                    }
                }
            }
        }
    }
    let mut h = Harness::new(1440.);
    let out = h.frame(vec![]).0;
    check(&out, 14.);
    let row =
        h.ui.probe
            .widgets
            .iter()
            .find(|(key, _)| key.starts_with("row:"))
            .unwrap()
            .1;
    for i in 0..3 {
        let x = h.ui.probe.widgets[&format!("column:{i}")].center().x;
        assert!(out.shapes.iter().any(
            |s| matches!(&s.shape, egui::Shape::LineSegment { points, .. }
            if (points[0].x-x).abs()<1. && points[0].y <= row.top() && points[1].y >= row.bottom())
        ));
    }
    assert!(
        !out.shapes
            .iter()
            .any(|s| matches!(&s.shape, egui::Shape::Rect(r)
        if row.contains_rect(r.rect) && r.corner_radius.nw > 0))
    );
    for page in ["Console", "JSON", "Network"] {
        h.click(page);
        h.frame(vec![Event::PointerMoved(Pos2::new(0., 799.))]);
        for _ in 0..3 {
            h.frame(vec![]);
        }
        check(&h.frame(vec![]).0, 14.);
    }
    for size in [10., 24., 14.] {
        h.ui.settings.set_font_size(size).unwrap();
        for dark in [true, false] {
            h.ui.settings
                .set_theme_mode(if dark {
                    flutter_network_viewer::settings::ThemeMode::Dark
                } else {
                    flutter_network_viewer::settings::ThemeMode::Light
                })
                .unwrap();
            f16_body(
                &mut h,
                json!({"statusCode":200,"body":r#"{"nested":{"text":"font test"}}"#}),
            );
            f16_select_body(&mut h);
            h.frame(vec![Event::PointerMoved(Pos2::new(0., 799.))]);
            for _ in 0..3 {
                h.frame(vec![]);
            }
            check(&h.frame(vec![]).0, size);
            for tab in [0, 1, 2, 3, 4, 5] {
                h.click(&format!("tab:{tab}"));
                h.frame(vec![Event::PointerMoved(Pos2::new(0., 799.))]);
                for _ in 0..3 {
                    h.frame(vec![]);
                }
                check(&h.frame(vec![]).0, size);
            }
        }
    }
    h.click("settings");
    h.frame(vec![Event::PointerMoved(Pos2::new(0., 799.))]);
    for _ in 0..3 {
        h.frame(vec![]);
    }
    check(&h.frame(vec![]).0, 14.);
    h.click("font-size-increase");
    h.frame(vec![Event::PointerMoved(Pos2::new(0., 799.))]);
    for _ in 0..3 {
        h.frame(vec![]);
    }
    check(&h.frame(vec![]).0, 15.);
    h.click("font-size-decrease");
    h.frame(vec![Event::PointerMoved(Pos2::new(0., 799.))]);
    for _ in 0..3 {
        h.frame(vec![]);
    }
    check(&h.frame(vec![]).0, 14.);
    h.click("settings-close");
    h.click("help");
    h.frame(vec![Event::PointerMoved(Pos2::new(0., 799.))]);
    for _ in 0..3 {
        h.frame(vec![]);
    }
    check(&h.frame(vec![]).0, 14.);
}

#[test]
fn scrolling_network_rows_are_clipped_below_fixed_header() {
    for dark in [false, true] {
        for size in [14., 24.] {
            let mut h = Harness::new(1000.);
            h.size.y = 360.;
            h.ui.settings.set_font_size(size).unwrap();
            h.ui.settings
                .set_theme_mode(if dark {
                    flutter_network_viewer::settings::ThemeMode::Dark
                } else {
                    flutter_network_viewer::settings::ThemeMode::Light
                })
                .unwrap();
            let mut data = fixture();
            for i in 0..40 {
                data["session"]["id"] = json!(format!("scroll-{i}"));
                h.store.ingest(&data).unwrap();
            }
            h.frame(vec![]);
            let header = h.ui.probe.widgets["header:url"];
            let pos = Pos2::new(header.center().x, header.bottom() + 70.);
            h.frame(vec![
                Event::PointerMoved(pos),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: Vec2::new(0., -23.),
                    modifiers: Modifiers::NONE,
                },
            ]);
            for _ in 0..30 {
                h.frame(vec![]);
            }
            let out = h.frame(vec![]).0;
            assert_eq!(h.ui.probe.widgets["header:url"], header);
            let mut partial_row = false;
            for (key, row) in &h.ui.probe.widgets {
                if key.starts_with("row:")
                    && row.top() < header.bottom()
                    && row.bottom() > header.bottom()
                {
                    partial_row = true;
                    for shape in &out.shapes {
                        if let egui::Shape::Text(text) = &shape.shape {
                            let bounds = Rect::from_min_size(text.pos, text.galley.size());
                            if bounds.intersects(*row)
                                && text.pos.y >= row.top()
                                && text.pos.x < h.ui.probe.list_rect.right()
                                && !["Method", "URL", "Status", "Time"]
                                    .contains(&text.galley.text())
                                && !["method", "status"].iter().any(|key| {
                                    text.galley.text() == "\u{e098}"
                                        && h.ui.probe.widgets[*key].contains(text.pos)
                                })
                            {
                                assert!(
                                    shape.clip_rect.top() >= header.bottom(),
                                    "row text {:?} clips at {}, header ends {}",
                                    text.galley.text(),
                                    shape.clip_rect.top(),
                                    header.bottom()
                                );
                            }
                        }
                    }
                }
            }
            assert!(partial_row, "scroll must stop within a row");
        }
    }
}

#[test]
fn network_columns_shrink_to_small_title_filter_gap() {
    for language in ["en", "vi"] {
        for font_size in [14., 24.] {
            let mut h = Harness::new(1440.);
            h.ui.settings.set_language(language).unwrap();
            h.ui.settings.set_font_size(font_size).unwrap();
            h.frame(vec![]);
            for (column, filter, title) in [
                (0, "method", "Method"),
                (
                    2,
                    "status",
                    if language == "vi" {
                        "Trạng thái"
                    } else {
                        "Status"
                    },
                ),
            ] {
                let pos = h.ui.probe.widgets[&format!("column:{column}")].center();
                let moved = pos - Vec2::new(220., 0.);
                h.frame(vec![
                    Event::PointerMoved(pos),
                    Event::PointerButton {
                        pos,
                        button: PointerButton::Primary,
                        pressed: true,
                        modifiers: Modifiers::NONE,
                    },
                ]);
                h.frame(vec![Event::PointerMoved(moved)]);
                h.frame(vec![Event::PointerButton {
                    pos: moved,
                    button: PointerButton::Primary,
                    pressed: false,
                    modifiers: Modifiers::NONE,
                }]);
                let out = h.frame(vec![]).0;
                let cell = h.ui.probe.widgets[&format!("header:{filter}")];
                let button = h.ui.probe.widgets[filter];
                assert_eq!(button.size(), Vec2::splat(24.));
                assert!(
                    out.shapes
                        .iter()
                        .any(|s| matches!(&s.shape, egui::Shape::Text(t)
                    if button.contains(t.pos) && t.galley.text() == "\u{e098}"
                        && t.galley.job.sections[0].format.font_id.size == 20.))
                );
                let text = out
                    .shapes
                    .iter()
                    .find_map(|shape| match &shape.shape {
                        egui::Shape::Text(t)
                            if t.galley.text() == title
                                && cell.intersects(Rect::from_min_size(t.pos, t.galley.size())) =>
                        {
                            Some(t)
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| panic!("missing title {title}"));
                let gap = button.left() - (text.pos.x + text.galley.size().x);
                assert!(
                    (1.5..=3.5).contains(&gap),
                    "{language} {font_size} {title}: gap={gap}"
                );
                assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text()==title && s.clip_rect.right() >= t.pos.x + t.galley.size().x - 0.5)));
                h.click(filter);
                h.click(&format!(
                    "{filter}:{}",
                    if filter == "method" { "GET" } else { "Success" }
                ));
            }
        }
    }
}

#[test]
fn json_root_connectors_reach_nested_arrows_and_folding_is_independent() {
    let mut h = json_harness(r#"{"branch":{"list":[{"leaf":1}]},"tail":2}"#);
    for tree in [false, true] {
        h.click(if tree { "json-tree" } else { "json-code" });
        h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
        let (out, _) = h.frame(vec![]);
        let root = h.ui.probe.widgets["json-fold:0"];
        let child = h.ui.probe.widgets["json-fold:1"];
        assert!(
            child.center().x > root.center().x,
            "nested arrows must follow indentation"
        );
        let body = h.ui.probe.widgets["json-content"];
        let lines: Vec<_> = out
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::LineSegment { points, .. }
                    if body.contains(points[0]) && body.contains(points[1]) =>
                {
                    Some(*points)
                }
                _ => None,
            })
            .collect();
        if tree {
            assert!(!lines.is_empty());
            assert!(
                lines.iter().any(|p| (p[1] - p[0]).length() > 10.),
                "Tree connectors must use solid row segments"
            );
            assert!(lines.iter().any(|p| (p[0].x - root.center().x).abs() < 0.6
                && p[0].y >= root.bottom() - 0.5
                && p[0].y < child.bottom()));
        } else {
            assert!(lines.is_empty(), "JSON code has no branch guides");
        }
        h.click("json-fold:1");
        assert!(!h.ui.probe.widgets.contains_key("json-fold:2"));
        h.click("json-fold:1");
        assert!(h.ui.probe.widgets.contains_key("json-fold:2"));
        h.click("json-fold:0");
        assert_eq!(h.ui.probe.json.rows, vec![0]);
        h.click("json-fold:0");
        assert!(h.ui.probe.json.rows.len() > 3);
    }
}

#[test]
fn approved_metadata_and_form_tables_keep_copy_actions_and_full_values() {
    let mut h = Harness::new(1440.);
    h.click("request:ios-session-1:fetch-1");
    h.click("tab:0");
    assert!(h.ui.probe.widgets.contains_key("metadata:header"));
    let row = h.ui.probe.widgets["metadata:row:0"];
    let action = h.ui.probe.widgets["metadata:cell:0:2"];
    let copy = h.ui.probe.widgets["metadata:copy:0"];
    assert_eq!(copy.width(), copy.height());
    assert!((copy.center().x - action.center().x).abs() < 0.6);
    assert!((copy.center().y - row.center().y).abs() < 0.6);
    let (out, _) = h.click("metadata:copy:0");
    assert!(!copied(&out).unwrap().is_empty());
    f16_body(
        &mut h,
        json!({"statusCode":200,"headers":{"content-type":"application/x-www-form-urlencoded"},"body":"name=Ti%E1%BA%BFng+Vi%E1%BB%87t&long=full+value"}),
    );
    f16_select_body(&mut h);
    let header = h.ui.probe.widgets["body-form-header"];
    let row = h.ui.probe.widgets["body-form-row:0"];
    assert!((header.bottom() - row.top()).abs() < 0.6);
    let (out, _) = h.click("body-form-copy:0");
    assert_eq!(copied(&out), Some("Tiếng Việt"));
}

#[test]
fn empty_capture_panes_only_paint_centered_messages() {
    let mut h = Harness::new(1360.);
    h.store = CaptureStore::default();
    for (page, message) in [("Network", "No data"), ("Console", "No data")] {
        h.click(page);
        let (out, _) = h.frame(vec![Event::PointerMoved(Pos2::new(0., 799.))]);
        let list = h.ui.probe.widgets["empty-list"];
        let header = h.ui.probe.widgets[if page == "Network" {
            "header:method"
        } else {
            "console-header:source"
        }];
        assert!(header.bottom() <= list.top());
        assert!(out.shapes.iter().any(
            |s| matches!(&s.shape, egui::Shape::Rect(r) if r.rect == list && r.stroke.width > 0.)
        ));

        assert!(h.ui.probe.list_rect.contains_rect(list));
        for (pane, message) in [
            (list, message),
            (h.ui.probe.widgets["empty-inspector"], "No data"),
        ] {
            let content: Vec<_> = out
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::Shape::Text(t) if pane.contains(t.pos) => Some(t),
                    _ => None,
                })
                .collect();
            assert_eq!(
                content.iter().map(|t| t.galley.text()).collect::<Vec<_>>(),
                vec![message]
            );
            assert!(!out.shapes.iter().any(|shape| matches!(&shape.shape,
                egui::Shape::Rect(r) if pane.contains_rect(r.rect) && r.rect.size() == Vec2::splat(72.))));
            let t = content[0];
            assert!((t.pos + t.galley.size() / 2. - pane.center()).length() <= 1.);
        }
        assert!(!h.ui.probe.widgets.contains_key("empty-connect"));
    }
}

#[test]
fn json_workspace_panes_have_four_pixel_insets_and_top_left_placeholders() {
    const INPUT: &str = r#"Enter your json input here. Example: {"name": "value"}"#;
    const OUTPUT: &str = "Your formatted json input will be display here";
    for width in [700., 1360.] {
        let mut h = Harness::new(width);
        h.click("JSON");
        let (out, _) = h.frame(vec![]);
        let input = h.ui.probe.widgets["json-input"];
        let divider = h.ui.probe.widgets["json-divider"];
        assert!((input.top() - divider.top()).abs() <= 1.);
        let hint = |text: &str| {
            out.shapes.iter().find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == text => {
                    Some(Rect::from_min_size(t.pos, t.galley.size()))
                }
                _ => None,
            })
        };
        assert!(hint("JSON input").is_none());
        assert!(hint("Formatted JSON").is_none());
        let left = hint(INPUT).expect("left placeholder");
        let right = hint(OUTPUT).expect("right placeholder");
        let inset = Vec2::splat(4.);
        assert!(
            (left.min - input.min - inset).length() <= 0.6,
            "{left:?} inside {input:?}"
        );
        let right_origin = egui::pos2(divider.right(), divider.top());
        assert!(
            (right.min - right_origin - inset).length() <= 0.6,
            "{right:?} from {right_origin:?}"
        );
        assert!(right.right() <= width - 12.);
        h.text("json-input", "4");
        let (typed, _) = h.frame(vec![]);
        assert!(
            typed
                .shapes
                .iter()
                .any(|shape| matches!(&shape.shape, egui::Shape::Text(t)
            if t.galley.text() == "4" && (t.pos - input.min - inset).length() <= 0.6))
        );
        h.click("json-format");
        assert_eq!(h.ui.probe.json.text, "4");
        let output = h.ui.probe.widgets["json-content"];
        assert!((output.min - right_origin - inset).length() <= 0.6);
    }
}

#[test]
fn toolbar_icon_ink_is_centered_on_both_axes_at_rest_and_on_hover() {
    let mut h = Harness::new(1360.);
    for (key, code) in [
        ("settings", '\u{f36e}'),
        ("help", '\u{e30b}'),
        ("pause", '\u{e47c}'),
        ("clear", '\u{efab}'),
        ("export", '\u{f05b}'),
    ] {
        h.frame(vec![Event::PointerMoved(Pos2::new(0., 799.))]);
        let (rest, _) = h.frame(vec![]);
        let rect = h.ui.probe.widgets[key];
        let check = |output: &egui::FullOutput, rect: Rect| {
            assert!(
                (rect.width() - rect.height()).abs() < 0.1,
                "{key}: {rect:?}"
            );
            let ink = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(t) if t.galley.text() == code.to_string() => {
                        let bounds = t.galley.mesh_bounds.translate(t.pos.to_vec2());
                        rect.contains(bounds.center()).then_some(bounds)
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing painted glyph for {key}"));
            let offset = ink.center() - rect.center();
            assert!(
                offset.x.abs() <= 0.55 && offset.y.abs() <= 0.55,
                "{key}: icon offset {offset:?}"
            );
        };
        check(&rest, rect);
        h.frame(vec![Event::PointerMoved(rect.center())]);
        let (hover, _) = h.frame(vec![]);
        let grown = h.ui.probe.widgets[key];
        assert_eq!(grown, rect, "{key} changed dimensions on hover");
        assert!((grown.center() - rect.center()).length() <= 0.55);
        check(&hover, grown);
    }
}

#[test]
fn filtered_network_and_console_keep_headers_and_share_empty_text_in_both_locales() {
    for lang in ["en", "vi"] {
        for page in ["Network", "Console"] {
            let mut h = Harness::new(1360.);
            h.ui.settings.set_language(lang).unwrap();
            h.click(page);
            h.text("search", "does-not-match-any-capture-123456");
            let (out, _) = h.frame(vec![]);
            let message = if lang == "vi" {
                "Không có dữ liệu"
            } else {
                "No data"
            };
            let rect = h.ui.probe.widgets["empty-list"];
            let key = if page == "Network" {
                "header:method"
            } else {
                "console-header:source"
            };
            assert!(h.ui.probe.widgets[key].bottom() <= rect.top());
            assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if rect.contains(t.pos) && t.galley.text() == message)));
            assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Rect(r) if r.rect == rect && r.stroke.width > 0.)));
        }
    }
}

#[test]
fn json_toolbar_has_uniform_square_actions_padded_tabs_and_no_group_border() {
    for workspace in [false, true] {
        for size in [14., 24.] {
            let mut h = if workspace {
                json_harness(r#"{"value":"needle"}"#)
            } else {
                let mut h = Harness::new(2200.);
                h.click("request:ios-session-1:fetch-1");
                h.click("tab:5");
                h
            };
            h.ui.settings.set_font_size(size).unwrap();
            if size == 24. {
                h.ui.settings
                    .set_theme_mode(flutter_network_viewer::settings::ThemeMode::Dark)
                    .unwrap();
            }
            h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
            for _ in 0..4 {
                h.frame(vec![]);
            }
            let (out, _) = h.frame(vec![]);
            let reference = h.ui.probe.widgets["json-clear"];
            for key in [
                "json-case",
                "json-word",
                "json-regex",
                "json-prev",
                "json-next",
                "json-clear",
                "json-copy",
            ] {
                let rect = h.ui.probe.widgets[key];
                assert!(
                    (rect.width() - rect.height()).abs() < 0.5,
                    "{key}: {rect:?}"
                );
                assert!(
                    if ["json-case", "json-word", "json-regex"].contains(&key) {
                        rect.size() == h.ui.probe.widgets["json-case"].size()
                    } else {
                        rect.size() == reference.size()
                    },
                    "{key}: {rect:?} is inconsistent with its group"
                );
            }
            for (key, label) in [("json-code", "JSON"), ("json-tree", "Tree")] {
                let rect = h.ui.probe.widgets[key];
                let t = out
                    .shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Text(t)
                            if rect.contains(t.pos) && t.galley.text() == label =>
                        {
                            Some(t)
                        }
                        _ => None,
                    })
                    .unwrap();
                assert!(t.pos.x - rect.left() >= 7.5);
                assert!(rect.right() - t.pos.x - t.galley.size().x >= 7.5);
            }
            let json = h.ui.probe.widgets["json-code"];
            let tree = h.ui.probe.widgets["json-tree"];
            assert_eq!(json.right(), tree.left());
            assert_eq!(json.y_range(), tree.y_range());
            let selector = json.union(tree);
            let accent = flutter_network_viewer::desktop_style::Palette::of(&h.ctx).accent;
            let borders: Vec<_> = out
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Rect(r)
                        if [json, tree, selector].contains(&r.rect) && r.stroke.width > 0. =>
                    {
                        Some(r)
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(borders.len(), 1, "only the selector owns a border");
            assert_eq!(borders[0].rect, selector);
            assert_eq!(borders[0].stroke, egui::Stroke::new(1.0_f32, accent));
            assert_eq!(
                out.shapes
                    .iter()
                    .filter(|s| matches!(&s.shape,
                        egui::Shape::LineSegment { points, stroke }
                            if (points[0].x - tree.left()).abs() < 0.01
                            && (points[1].x - tree.left()).abs() < 0.01
                            && (points[0].y - selector.top()).abs() < 0.01
                            && (points[1].y - selector.bottom()).abs() < 0.01
                            && *stroke == egui::Stroke::new(1.0_f32, accent)
                    ))
                    .count(),
                1,
                "single blue divider"
            );
            let (hover, _) = h.frame(vec![Event::PointerMoved(tree.center())]);
            assert_eq!(
                hover.platform_output.cursor_icon,
                egui::CursorIcon::PointingHand
            );
            h.click("json-tree");
            let (out, _) = h.frame(vec![]);
            assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Rect(r)
                if r.rect == h.ui.probe.widgets["json-tree"] && r.fill == accent && r.stroke.width == 0.)));
            h.click("json-code");
            let (out, _) = h.frame(vec![]);
            assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Rect(r)
                if r.rect == h.ui.probe.widgets["json-code"] && r.fill == accent && r.stroke.width == 0.)));
            let group = h.ui.probe.widgets["json-find-group"];
            assert!(!out.shapes.iter().any(
            |s| matches!(&s.shape, egui::Shape::Rect(r) if r.rect == group && r.stroke.width > 0.)
        ));
        }
    }
}

#[test]
fn tree_connectors_match_reference_short_elbows_and_start_below_parent_rows() {
    for size in [14., 24.] {
        let mut h = json_harness(
            r#"{"timestamp":"2026-11-16","success":true,"data":{"staffId":"abc","pin":100}}"#,
        );
        h.ui.settings.set_font_size(size).unwrap();
        for _ in 0..4 {
            h.frame(vec![]);
        }
        h.click("json-tree");
        h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
        let (out, _) = h.frame(vec![]);
        let root = h.ui.probe.widgets["json-fold:0"];
        let data = h.ui.probe.widgets["json-fold:3"];
        let cell = h
            .ctx
            .fonts_mut(|f| f.glyph_width(&egui::FontId::monospace(size), ' '));
        let lines: Vec<_> = out
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::LineSegment { points, .. }
                    if h.ui.probe.widgets["json-content"].contains(points[0]) =>
                {
                    Some(*points)
                }
                _ => None,
            })
            .collect();
        assert!(
            !lines
                .iter()
                .any(|p| (p[0].x - root.center().x).abs() < 0.6 && p[0].y < root.bottom() - 0.5)
        );
        assert!(
            !lines
                .iter()
                .any(|p| (p[0].x - data.center().x).abs() < 0.6 && p[0].y < data.bottom() - 0.5)
        );
        let elbows: Vec<_> = lines
            .iter()
            .filter(|p| (p[0].y - p[1].y).abs() < 0.1)
            .collect();
        assert_eq!(elbows.len(), 5);
        for line in elbows {
            assert!(((line[1].x - line[0].x) - 2.5 * cell).abs() < 0.6);
        }
        assert!(
            lines.iter().any(|p| (p[0].x - root.center().x).abs() < 0.6
                && (p[1].y - data.center().y).abs() < 0.6)
        );
        assert!(
            !lines
                .iter()
                .any(|p| (p[0].x - root.center().x).abs() < 0.6 && p[1].y > data.center().y + 0.6)
        );
        h.click("json-fold:3");
        assert_eq!(h.ui.probe.json.rows, vec![0, 1, 2, 3]);
        h.click("json-fold:3");
        assert_eq!(h.ui.probe.json.rows.len(), 6);
    }
}

#[test]
fn json_and_tree_fold_arrows_center_on_visible_text() {
    for size in [14., 24.] {
        let mut h = json_harness(r#"{"data":{"pin":100}}"#);
        h.ui.settings.set_font_size(size).unwrap();
        for _ in 0..4 {
            h.frame(vec![]);
        }
        for mode in ["json-code", "json-tree"] {
            h.click(mode);
            for key in ["json-fold:0", "json-fold:1"] {
                let slot = h.ui.probe.widgets[key];
                for collapsed in [false, true] {
                    let (out, _) = h.frame(vec![]);
                    assert_eq!(h.ui.probe.widgets[key], slot);
                    let arrow = out
                        .shapes
                        .iter()
                        .find_map(|s| match &s.shape {
                            egui::Shape::Path(path) if path.closed && path.points.len() == 3 => {
                                let bounds = Rect::from_points(&path.points);
                                slot.contains(bounds.center()).then_some(bounds)
                            }
                            _ => None,
                        })
                        .expect("painted fold arrow");
                    assert_eq!(arrow.height() > arrow.width(), collapsed);
                    let text_y = out
                        .shapes
                        .iter()
                        .find_map(|s| match &s.shape {
                            egui::Shape::Text(t)
                                if (t.pos.y - slot.top()).abs() < 0.5
                                    && t.galley.text().contains('{')
                                    && h.ui.probe.widgets["json-content"].contains(t.pos) =>
                            {
                                let row = &t.galley.rows[0];
                                let bounds = row.visuals.mesh.vertices
                                    [row.visuals.glyph_vertex_range.clone()]
                                .iter()
                                .fold(Rect::NOTHING, |mut bounds, vertex| {
                                    bounds.extend_with(vertex.pos);
                                    bounds
                                });
                                Some(t.pos.y + row.pos.y + bounds.center().y)
                            }
                            _ => None,
                        })
                        .expect("painted fold row text");
                    assert!(
                        (arrow.center().y - text_y).abs() <= 0.55,
                        "{mode} at {size}px, {key}, collapsed={collapsed}: arrow center {}, text center {text_y}",
                        arrow.center().y
                    );
                    h.click(key);
                }
            }
        }
    }
}

#[test]
fn network_rows_do_not_show_tooltips_but_action_buttons_do() {
    let mut h = Harness::new(1360.);
    let hover = |h: &mut Harness, key: &str| {
        let pos = h.ui.probe.widgets[key].center();
        h.frame(vec![Event::PointerMoved(pos)]);
        h.time += 1.;
        h.frame(vec![]);
        h.frame(vec![]).0
    };
    let paints_text = |output: &egui::FullOutput, text: &str| {
        output
            .shapes
            .iter()
            .any(|shape| matches!(&shape.shape, egui::Shape::Text(t) if t.galley.text() == text))
    };
    let row = "request:ios-session-1:fetch-1";
    let hovered = hover(&mut h, row);
    assert_eq!(
        hovered.platform_output.cursor_icon,
        egui::CursorIcon::PointingHand
    );
    assert!(!paints_text(
        &hovered,
        "https://api.example.com/orders?draft=true"
    ));
    let hovered = hover(&mut h, "settings");
    assert!(paints_text(&hovered, "Settings"));
    h.click(row);
    assert!(h.ui.probe.widgets.contains_key("tab:0"));
}

#[test]
fn light_theme_uses_ventura_content_and_control_backgrounds() {
    let mut h = Harness::new(1440.);
    let surface = egui::Color32::WHITE;
    let control = egui::Color32::WHITE;
    let fill = |out: &egui::FullOutput, h: &Harness, key: &str, color| {
        let rect = h.ui.probe.widgets[key];
        assert!(
            out.shapes
                .iter()
                .any(|s| matches!(&s.shape, egui::Shape::Rect(r)
                if r.rect == rect && r.fill == color)),
            "{key} should paint {color:?}"
        );
    };
    let request = "row:ios-session-1:fetch-1";
    let (out, _) = h.frame(vec![]);
    fill(&out, &h, request, surface);
    fill(&out, &h, "search-frame", control);
    fill(&out, &h, "settings", control);
    let (out, _) = h.frame(vec![Event::PointerMoved(
        h.ui.probe.widgets[request].center(),
    )]);
    fill(&out, &h, request, surface);

    h.click("request:ios-session-1:fetch-1");
    h.click("tab:0");
    let (out, _) = h.frame(vec![]);
    fill(&out, &h, "metadata:row:0", surface);

    h.click("JSON");
    let (out, _) = h.frame(vec![]);
    fill(&out, &h, "json-input", control);
    fill(&out, &h, "json-output-pane", surface);

    h.text(
        "json-input",
        r##"{"sample":"rgba(0, 255, 0, .5)","white":"#ffffff"}"##,
    );
    h.click("json-format");
    let (out, _) = h.frame(vec![]);
    let swatches = &h.ui.probe.json.swatches;
    assert_eq!(swatches.len(), 2);
    fill(&out, &h, &swatches[0], control);
    fill(&out, &h, &swatches[1], egui::Color32::WHITE);

    h.click("settings");
    for _ in 0..4 {
        h.frame(vec![]);
    }
    let (out, _) = h.frame(vec![]);
    fill(&out, &h, "settings-appearance", surface);
    fill(&out, &h, "mcp-port", control);
    fill(
        &out,
        &h,
        "mcp-endpoint",
        egui::Color32::from_rgb(236, 236, 236),
    );
}

#[test]
fn json_workspace_puts_all_find_controls_and_view_tabs_in_the_header() {
    for (width, size) in [(700., 14.), (1360., 24.)] {
        let mut h = Harness::new(width);
        h.click("JSON");
        h.text("json-input", r#"{"results":["needle","needle","Needle"]}"#);
        h.click("json-format");
        h.ui.settings.set_font_size(size).unwrap();
        for _ in 0..4 {
            h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
        }
        let widgets = &h.ui.probe.widgets;
        let nav = widgets["nav-toolbar"];
        let find = widgets["json-find"];
        for key in [
            "json-format",
            "json-case",
            "json-word",
            "json-regex",
            "json-prev",
            "json-next",
            "json-clear",
            "json-code",
            "json-tree",
            "json-copy",
            "json-download",
        ] {
            let rect = widgets[key];
            assert!(
                nav.contains_rect(rect),
                "{key} at {width}/{size}: {rect:?} outside {nav:?}"
            );
            assert!(
                (rect.center().y - find.center().y).abs() < 1.,
                "{key} at {width}/{size}: {rect:?} is not aligned with search {find:?}"
            );
        }
        h.text("json-find", "needle");
        assert_eq!(h.ui.probe.json.matches, 3);
        h.click("json-case");
        assert_eq!(h.ui.probe.json.matches, 2);
        let previous = h.ui.probe.json.active;
        h.click("json-next");
        assert_ne!(h.ui.probe.json.active, previous);
        h.click("json-tree");
        assert!(h.ui.probe.json.tree);
        h.click("json-clear");
        assert!(!h.ui.probe.widgets.contains_key("json-toolbar"));
    }
}

#[test]
fn both_header_rows_keep_complete_action_borders_with_and_without_json() {
    for size in [14., 24.] {
        let mut h = Harness::new(if size == 14. { 1360. } else { 2200. });
        h.ui.settings.set_font_size(size).unwrap();
        h.click("JSON");
        for populated in [false, true] {
            if populated {
                h.text("json-input", r#"{"value":1}"#);
                h.click("json-format");
            }
            for _ in 0..4 {
                h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
            }
            let (out, _) = h.frame(vec![]);
            for (header, keys) in [
                ("header", &["scan", "manual-url", "settings", "help"][..]),
                (
                    "nav-toolbar",
                    &[
                        "json-case",
                        "json-word",
                        "json-regex",
                        "json-prev",
                        "json-next",
                        "json-clear",
                        "json-workspace-copy",
                        "json-download",
                    ][..],
                ),
            ] {
                for key in keys {
                    let rect = h.ui.probe.widgets[*key];
                    assert!(
                        h.ui.probe.widgets[header].contains_rect(rect),
                        "{key}: {rect:?}"
                    );
                    let border = out
                        .shapes
                        .iter()
                        .rev()
                        .find_map(|shape| match &shape.shape {
                            egui::Shape::Rect(r) if r.rect == rect && r.stroke.width >= 1. => {
                                Some((shape.clip_rect, r))
                            }
                            _ => None,
                        })
                        .unwrap_or_else(|| {
                            panic!("missing border: {key}, font {size}, populated {populated}")
                        });
                    assert!(
                        border.0.contains_rect(rect),
                        "clipped border: {key}: {:?} in {:?}",
                        rect,
                        border.0
                    );
                    // Disabled controls keep a visible outline; only their icon dims.
                    assert_eq!(
                        border.1.stroke.color,
                        flutter_network_viewer::vintage::LINE,
                        "faded border: {key}, font {size}, populated {populated}"
                    );
                }
            }
            if !populated {
                let (_, actions) = h.click("json-download");
                assert!(actions.is_empty());
                assert!(!h.ui.probe.widgets.contains_key("save-export"));
                let (out, _) = h.click("json-workspace-copy");
                assert!(copied(&out).is_none());
            }
        }
    }
}

#[test]
fn icon_text_gaps_are_four_pixels_in_header_search_and_settings() {
    fn job_gap(out: &egui::FullOutput, rect: Rect) {
        let text = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t)
                    if rect.contains(t.pos) && t.galley.job.sections.len() == 2 =>
                {
                    Some(t)
                }
                _ => None,
            })
            .expect("icon and label");
        let glyphs = &text.galley.rows[0].glyphs;
        assert!(
            !glyphs[1].chr.is_whitespace(),
            "spacing must use pixels, not space characters"
        );
        let gap = glyphs[1].pos.x - glyphs[0].pos.x - glyphs[0].advance_width;
        assert!((gap - 4.).abs() < 0.6, "icon/text gap {gap}");
    }
    for size in [14., 24.] {
        let mut h = Harness::new(2200.);
        h.size.y = 1800.;
        h.ui.settings.set_font_size(size).unwrap();
        for _ in 0..4 {
            h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
        }
        let (out, _) = h.frame(vec![]);
        for key in ["scan", "manual-url"] {
            job_gap(&out, h.ui.probe.widgets[key]);
        }
        for key in ["scan", "manual-url"] {
            h.frame(vec![Event::PointerMoved(h.ui.probe.widgets[key].center())]);
            let (hover, _) = h.frame(vec![]);
            job_gap(&hover, h.ui.probe.widgets[key]);
            h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
        }
        let icon = h.ui.probe.widgets["search-icon"];
        let input = h.ui.probe.widgets["search"];
        assert!((input.left() - icon.right() - 4.).abs() < 0.6);
        h.click("settings");
        h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
        let (out, _) = h.frame(vec![]);
        for key in ["mcp-copy-codex", "mcp-copy-claude"] {
            job_gap(&out, h.ui.probe.widgets[key]);
        }
        for key in ["mcp-copy-codex", "mcp-copy-claude"] {
            h.frame(vec![Event::PointerMoved(h.ui.probe.widgets[key].center())]);
            let (hover, _) = h.frame(vec![]);
            job_gap(&hover, h.ui.probe.widgets[key]);
            h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
        }
        for (key, title, code) in [
            ("settings-appearance", "Interface", '\u{e683}'),
            ("settings-mcp", "MCP", '\u{f0616}'),
            ("settings-updates", "App updates", '\u{e637}'),
        ] {
            let section = h.ui.probe.widgets[key];
            let bounds = |value: &str| {
                out.shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Text(t)
                            if section.contains(t.pos) && t.galley.text() == value =>
                        {
                            Some(Rect::from_min_size(t.pos, t.galley.size()))
                        }
                        _ => None,
                    })
                    .unwrap()
            };
            assert!(
                (bounds(title).left() - bounds(&code.to_string()).right() - 4.).abs() < 0.6,
                "{key} icon/title gap"
            );
        }
    }
}

#[test]
fn ventura_palette_is_applied_to_production_controls_selection_and_json_in_both_modes() {
    use flutter_network_viewer::settings::ThemeMode;
    for (mode, bg, surface, control, text, muted, line, accent) in [
        (
            ThemeMode::Light,
            0xECECEC,
            0xFFFFFF,
            0xFFFFFF,
            0x1D1D1F,
            0x6E6E73,
            0xD2D2D7,
            0x007AFF,
        ),
        (
            ThemeMode::Dark,
            0x282828,
            0x1E1E1E,
            0x3A3A3A,
            0xF5F5F7,
            0x98989D,
            0x48484A,
            0x0A84FF,
        ),
    ] {
        let rgb = |v: u32| egui::Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8);
        let mut h = Harness::new(1440.);
        h.ui.settings.set_theme_mode(mode).unwrap();
        let (out, _) = h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
        let p = flutter_network_viewer::desktop_style::Palette::of(&h.ctx);
        assert_eq!(
            (
                p.background,
                p.surface,
                p.control,
                p.text,
                p.muted,
                p.line,
                p.accent
            ),
            (
                rgb(bg),
                rgb(surface),
                rgb(control),
                rgb(text),
                rgb(muted),
                rgb(line),
                rgb(accent)
            )
        );
        let visuals = h.ctx.style().visuals.clone();
        assert_eq!(visuals.panel_fill, rgb(bg));
        assert_eq!(visuals.selection.bg_fill, rgb(accent));
        assert_eq!(visuals.selection.stroke.color, egui::Color32::WHITE);
        assert_eq!(visuals.hyperlink_color, rgb(accent));
        let search = h.ui.probe.widgets["search-frame"];
        assert!(out.shapes.iter().any(
            |s| matches!(&s.shape, egui::Shape::Rect(r) if r.rect==search && r.fill==rgb(control))
        ));
        h.click("request:ios-session-1:fetch-1");
        let (out, _) = h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
        let row = h.ui.probe.widgets["row:ios-session-1:fetch-1"];
        assert!(out.shapes.iter().any(
            |s| matches!(&s.shape, egui::Shape::Rect(r) if r.rect==row && r.fill==rgb(accent))
        ));
        assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if row.contains(t.pos) && t.galley.text().starts_with("/orders") && t.galley.job.sections[0].format.color==egui::Color32::WHITE)));
        use flutter_network_viewer::desktop_support::NoticeKind;
        for (kind, light, dark) in [
            (NoticeKind::Info, 0x007AFF, 0x0A84FF),
            (NoticeKind::Success, 0x34C759, 0x30D158),
            (NoticeKind::Warning, 0xFF9500, 0xFF9F0A),
            (NoticeKind::Error, 0xFF3B30, 0xFF453A),
        ] {
            h.ui.notifications
                .show(kind, "Ventura palette".into(), false);
            h.frame(vec![]);
            assert_eq!(
                h.ui.probe.notification_color,
                rgb(if mode == ThemeMode::Dark { dark } else { light })
            );
            h.ui.notifications.dismiss(false);
            h.frame(vec![]);
        }
        h.click("JSON");
        h.text(
            "json-input",
            r#"{"key":"value","number":42,"boolean":true,"null":null,"array":[1]}"#,
        );
        h.click("json-format");
        let (out, _) = h.frame(vec![]);
        let colors: std::collections::HashSet<_> = out
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.job.sections.len() > 1 => {
                    Some(t.galley.job.sections.iter().map(|s| s.format.color))
                }
                _ => None,
            })
            .flatten()
            .collect();
        let expected = if mode == ThemeMode::Dark {
            flutter_network_viewer::app_colors::JSON_DARK
        } else {
            flutter_network_viewer::app_colors::JSON_LIGHT
        };
        assert_eq!(
            expected
                .into_iter()
                .collect::<std::collections::HashSet<_>>()
                .len(),
            7
        );
        for color in expected {
            assert!(colors.contains(&color), "missing syntax role {color:?}");
        }
    }
}

#[test]
fn geist_ui_and_cascadia_json_tree_share_the_configured_font_size() {
    use flutter_network_viewer::settings::ThemeMode;
    for mode in [ThemeMode::Light, ThemeMode::Dark] {
        for size in [14., 24.] {
            let mut h = Harness::new(1600.);
            h.ui.settings.set_theme_mode(mode).unwrap();
            h.ui.settings.set_font_size(size).unwrap();
            let (out, _) = h.frame(vec![]);
            let named_ui = egui::FontFamily::Name("Geist".into());
            assert_eq!(
                flutter_network_viewer::desktop_style::medium(size).family,
                named_ui
            );
            let definitions = h.ctx.fonts(|f| f.definitions().clone());
            assert_eq!(
                definitions.families[&egui::FontFamily::Proportional][0],
                "Geist"
            );
            assert_eq!(
                definitions.families[&egui::FontFamily::Monospace][0],
                "Cascadia Mono"
            );
            assert!(h.ctx.fonts_mut(|f| f.has_glyphs(
                &egui::FontId::new(size, named_ui.clone()),
                "Tiếng Việt Cài đặt Đường dẫn ăâđêôơư"
            )));
            let scan = h.ui.probe.widgets["scan"];
            assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if scan.contains(t.pos) && t.galley.text().contains("Find & connect") && t.galley.job.sections.iter().filter(|s| s.format.font_id.family != egui::FontFamily::Name("MaterialIcons".into())).all(|s| (s.format.font_id.family == named_ui || s.format.font_id.family == egui::FontFamily::Proportional) && s.format.font_id.size == size))));
            h.click("JSON");
            h.text(
                "json-input",
                r#"{"geist_font_check":"Tiếng Việt","child":{"value":42}}"#,
            );
            h.click("json-format");
            for tree in [false, true] {
                if tree {
                    h.click("json-tree");
                }
                let (out, _) = h.frame(vec![]);
                let content = h.ui.probe.widgets["json-content"];
                let sections: Vec<_> = out
                    .shapes
                    .iter()
                    .filter_map(|s| match &s.shape {
                        egui::Shape::Text(t)
                            if content.contains(t.pos)
                                && t.galley.text().contains("geist_font_check") =>
                        {
                            Some(&t.galley.job.sections)
                        }
                        _ => None,
                    })
                    .flatten()
                    .collect();
                assert!(!sections.is_empty(), "painted JSON content tree={tree}");
                assert!(
                    sections
                        .iter()
                        .all(|s| s.format.font_id.family == egui::FontFamily::Monospace
                            && s.format.font_id.size == size)
                );
            }
        }
    }
}

#[test]
fn inspector_overview_and_header_tables_wrap_values_inside_the_viewport() {
    for size in [14., 24.] {
        let mut h = Harness::new(1000.);
        h.size.y = 1800.;
        h.ui.settings.set_font_size(size).unwrap();
        let mut data = fixture();
        let url = format!(
            "https://pgapp-feature.nst.com.vn/api/v1/dms-service/staffPosition?identifier={}",
            "abcdef0123456789".repeat(6)
        );
        let header_value = "long-header-value-".repeat(12);
        data["events"][0]["request"]["url"] = json!(url);
        data["events"][0]["request"]["headers"] = json!({"x-long-header":header_value});
        h.store = CaptureStore::default();
        h.store.ingest(&data).unwrap();
        h.frame(vec![]);
        h.click("request:ios-session-1:fetch-1");
        for tab in [0, 1] {
            h.click(&format!("tab:{tab}"));
            let (out, _) = h.frame(vec![]);
            let viewport = h.ui.probe.widgets["metadata-viewport"];
            let grid = h.ui.probe.widgets["metadata:grid"];
            assert!(
                grid.left() >= viewport.left() && grid.right() <= viewport.right() + 0.5,
                "table stays inside inspector viewport"
            );
            let title = h.ui.probe.widgets["metadata:cell:0:0"];
            assert_eq!(title.width(), 160.);
            let row = if tab == 0 { 3 } else { 0 };
            let expected = if tab == 0 {
                url.split_once('?').unwrap().1
            } else {
                &header_value
            };
            let value = h.ui.probe.widgets[&format!("metadata:cell:{row}:1")];
            let text = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t)
                        if value.contains(t.pos) && t.galley.text() == expected =>
                    {
                        Some(t)
                    }
                    _ => None,
                })
                .expect("complete wrapped value is rendered");
            assert!(text.galley.rows.len() > 1);
            assert!(text.galley.size().y <= value.height() - 7.);
            let (out, _) = h.click(&format!("metadata:copy:{row}"));
            assert_eq!(copied(&out), Some(expected));
        }
    }
}

#[test]
fn query_parameters_preserve_raw_bytes_use_json_colors_and_left_align_values() {
    use flutter_network_viewer::settings::ThemeMode;
    let queries = [
        "origin=10.797235%2C106.63440500000002&destination=10.7965826087%2C106.63197473959997&alternatives=false&key=4b1967a7588c3b73e6ab158cbd20c6bd",
        "tag=a%2Cb&tag=a%2cb&plus=a+b&empty=&flag&=value&&encoded%20key=a=b&count=42&unicode=Ti%E1%BA%BFng&nil=null",
    ];
    for query in queries {
        for mode in [ThemeMode::Light, ThemeMode::Dark] {
            for language in ["en", "vi"] {
                for size in [14., 24.] {
                    let mut h = Harness::new(1000.);
                    h.size.y = 2200.;
                    h.ui.settings.set_theme_mode(mode).unwrap();
                    h.ui.settings.set_language(language).unwrap();
                    h.ui.settings.set_font_size(size).unwrap();
                    let mut data = fixture();
                    data["events"][0]["request"]["url"] = json!(format!(
                        "https://example.com/path?{query}#fragment?ignored=yes"
                    ));
                    h.store = CaptureStore::default();
                    h.store.ingest(&data).unwrap();
                    h.frame(vec![]);
                    h.click("request:ios-session-1:fetch-1");
                    let (out, _) = h.frame(vec![]);
                    let cell = h.ui.probe.widgets["metadata:cell:3:1"];
                    let painted = out
                        .shapes
                        .iter()
                        .find_map(|s| match &s.shape {
                            egui::Shape::Text(t)
                                if cell.contains(t.pos) && t.galley.text() == query =>
                            {
                                Some(t)
                            }
                            _ => None,
                        })
                        .expect("original encoded query is rendered verbatim");
                    let bounds = painted.galley.rect.translate(painted.pos.to_vec2());
                    assert!((bounds.left() - cell.left() - 8.).abs() < 0.6);
                    assert!(bounds.right() <= cell.right() - 7.);
                    assert!(painted.galley.rows.len() > 1);
                    let colors = if mode == ThemeMode::Dark {
                        flutter_network_viewer::app_colors::JSON_DARK
                    } else {
                        flutter_network_viewer::app_colors::JSON_LIGHT
                    };
                    let color_at = |offset| {
                        painted
                            .galley
                            .job
                            .sections
                            .iter()
                            .find(|s| s.byte_range.contains(&offset))
                            .unwrap()
                            .format
                            .color
                    };
                    for (token, role) in if query.starts_with("origin=") {
                        vec![
                            ("origin", 1),
                            ("10.797235%2C", 2),
                            ("alternatives", 1),
                            ("false", 4),
                            ("4b1967", 2),
                        ]
                    } else {
                        vec![
                            ("tag", 1),
                            ("a%2Cb", 2),
                            ("a+b", 2),
                            ("flag", 1),
                            ("encoded%20key", 1),
                            ("a=b", 2),
                            ("42", 3),
                            ("null", 6),
                        ]
                    } {
                        assert_eq!(
                            color_at(query.find(token).unwrap()),
                            colors[role],
                            "{token}"
                        );
                    }
                    for (offset, ch) in query.char_indices() {
                        if ch == '&' {
                            assert_eq!(color_at(offset), colors[0]);
                        }
                    }
                    assert!(
                        painted
                            .galley
                            .job
                            .sections
                            .iter()
                            .all(
                                |s| s.format.font_id.family == egui::FontFamily::Proportional
                                    && s.format.font_id.size == size
                            )
                    );
                    let (out, _) = h.click("metadata:copy:3");
                    assert_eq!(copied(&out), Some(query));
                }
            }
        }
    }
}

#[test]
fn overview_lists_method_domain_endpoint_and_query_params_as_first_four_rows() {
    for (url, domain, endpoint, query) in [
        (
            "https://example.com:8443/api/v1/a%2Fb?tag=a%2Cb&tag=a%2cb#fragment",
            "https://example.com:8443",
            "/api/v1/a%2Fb",
            "tag=a%2Cb&tag=a%2cb",
        ),
        (
            "http://[::1]:8080?flag#fragment",
            "http://[::1]:8080",
            "/",
            "flag",
        ),
        (
            "https://demo:password@example.com:443/path",
            "https://example.com:443",
            "/path",
            "No data",
        ),
        ("//example.com/path", "example.com", "/path", "No data"),
        (
            "/api/v1/items#fragment?ignored",
            "—",
            "/api/v1/items",
            "No data",
        ),
    ] {
        let mut h = Harness::new(1440.);
        h.size.y = 1200.;
        let mut data = fixture();
        data["events"][0]["request"]["url"] = json!(url);
        h.store = CaptureStore::default();
        h.store.ingest(&data).unwrap();
        h.frame(vec![]);
        h.click("request:ios-session-1:fetch-1");
        let (out, _) = h.frame(vec![]);
        for (row, (title, value)) in [
            ("Method", "POST"),
            ("Domain", domain),
            ("Endpoint", endpoint),
            ("Query params", query),
        ]
        .into_iter()
        .enumerate()
        {
            for (column, expected) in [title, value].into_iter().enumerate() {
                let cell = h.ui.probe.widgets[&format!("metadata:cell:{row}:{column}")];
                assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if cell.contains(t.pos) && t.galley.text()==expected)), "row {row} must show {expected:?}");
            }
            let (out, _) = h.click(&format!("metadata:copy:{row}"));
            assert_eq!(copied(&out), Some(value));
        }
    }
}

#[test]
fn inspector_ventura_scrollbar_is_overlay_and_drags_to_hidden_tabs() {
    for dark in [false, true] {
        let mut h = Harness::new(1000.);
        h.ctx.set_visuals(if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });
        h.click("request:ios-session-1:fetch-1");
        let viewport = h.ui.probe.widgets["inspector-tabs"];
        let height = viewport.height();
        let first = h.ui.probe.widgets["tab:0"].left();
        h.frame(vec![Event::PointerMoved(viewport.center())]);
        let mut output = h.frame(vec![]).0;
        for _ in 0..20 {
            output = h.frame(vec![]).0;
        }
        let thumb = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Rect(r)
                    if r.corner_radius.nw > 0
                        && r.fill.a() > 0
                        && r.rect.width() > 30.
                        && r.rect.height() <= 9.
                        && viewport.expand(1.).contains_rect(r.rect) =>
                {
                    Some(r.rect)
                }
                _ => None,
            })
            .expect("overflowing inspector tabs need a rounded Ventura thumb");
        let tabs_bottom = h.ui.probe.widgets["tab:0"].bottom();
        assert!(
            thumb.top() >= tabs_bottom + 1.,
            "scrollbar overlaps the tab buttons"
        );
        let start = thumb.center();
        h.frame(vec![Event::PointerMoved(start)]);
        let mut hovered = h.frame(vec![]).0;
        for _ in 0..20 {
            hovered = h.frame(vec![]).0;
        }
        let mut has_expanded_bar = false;
        for shape in &hovered.shapes {
            if let egui::Shape::Rect(r) = &shape.shape
                && r.corner_radius.nw > 0
                && r.fill.a() > 0
                && r.rect.width() > 30.
                && r.rect.height() > thumb.height()
                && viewport.expand(1.).contains_rect(r.rect)
            {
                has_expanded_bar = true;
                assert!(
                    r.rect.top() >= tabs_bottom + 1.,
                    "hovered scrollbar overlaps the tab buttons"
                );
            }
        }
        assert!(
            has_expanded_bar,
            "expected to exercise the wider hovered scrollbar"
        );
        h.frame(vec![Event::PointerButton {
            pos: start,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        }]);
        let end = Pos2::new(viewport.right() - 2., start.y);
        h.frame(vec![Event::PointerMoved(end)]);
        h.frame(vec![Event::PointerButton {
            pos: end,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
        assert!(h.ui.probe.widgets["tab:0"].left() < first - 20.);
        assert!(viewport.contains(h.ui.probe.widgets["tab:5"].center()));
        assert_eq!(h.ui.probe.widgets["inspector-tabs"].height(), height);
        assert_eq!(h.ui.settings.values().request_detail_tab, 0);
        let mut h = Harness::new(2200.);
        h.ctx.set_visuals(if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });
        h.click("request:ios-session-1:fetch-1");
        for _ in 0..20 {
            h.frame(vec![]);
        }
        let wide = h.ui.probe.widgets["inspector-tabs"];
        h.frame(vec![Event::PointerMoved(wide.center())]);
        let out = h.frame(vec![]).0;
        assert_eq!(wide.height(), height);
        assert!(
            !out.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Rect(r) if r.corner_radius.nw > 0 && r.fill.a() > 0
                && r.rect.width() > 30. && r.rect.height() <= 9.
                && wide.expand(1.).contains_rect(r.rect))),
            "no scrollbar when tabs fit"
        );
    }
}

#[test]
fn standalone_icon_buttons_use_24_point_frames_and_14_point_glyphs() {
    fn check(h: &mut Harness, buttons: &[(&str, char)]) {
        let (out, _) = h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
        for &(key, code) in buttons {
            let rect = h.ui.probe.widgets[key];
            assert_eq!(rect.size(), Vec2::splat(24.), "{key}");
            assert!(
                out.shapes
                    .iter()
                    .any(|s| matches!(&s.shape, egui::Shape::Text(t)
                if rect.contains(t.pos) && t.galley.text() == code.to_string()
                    && t.galley.job.sections[0].format.font_id.size == 14.)),
                "{key} glyph"
            );
        }
    }
    for size in [14., 24.] {
        let mut h = Harness::new(1800.);
        h.ui.settings.set_font_size(size).unwrap();
        h.click("request:ios-session-1:fetch-1");
        check(
            &mut h,
            &[
                ("settings", '\u{f36e}'),
                ("help", '\u{e30b}'),
                ("pause", '\u{e47c}'),
                ("clear", '\u{efab}'),
                ("export", '\u{f05b}'),
                ("curl", '\u{ef7f}'),
                ("metadata:copy:0", '\u{ef7f}'),
            ],
        );
        assert_eq!(
            h.ui.probe.widgets["metadata:copy:0"].center(),
            h.ui.probe.widgets["metadata:cell:0:2"].center()
        );
        assert!(copied(&h.click("curl").0).unwrap().starts_with("curl"));
        h.click("JSON");
        h.text("json-input", r#"{"value":1}"#);
        h.click("json-format");
        check(
            &mut h,
            &[
                ("json-clear", '\u{e16a}'),
                ("json-workspace-copy", '\u{ef7f}'),
                ("json-download", '\u{f05b}'),
            ],
        );
    }
}

#[test]
fn console_table_joins_header_and_body_and_clips_scrolling_rows() {
    let mut h = Harness::new(1440.);
    console_event(
        &mut h,
        "ios-session-1",
        json!((0..40).map(|i| json!({
            "type": "console", "source": "dart.stdout", "message": format!("seam sample {i}")
        })).collect::<Vec<_>>()),
    );
    h.click("Console");
    let header = h.ui.probe.widgets["console-header:source"]
        .union(h.ui.probe.widgets["console-header:message"]);
    let first = h.ui.probe.widgets[&format!("log:{}", h.ui.probe.logs[0])];
    assert_eq!(first.top(), header.bottom(), "gap below Console header");
    assert_eq!(first.x_range(), header.x_range());
    let next = h.ui.probe.widgets[&format!("log:{}", h.ui.probe.logs[1])];
    assert_eq!(first.bottom(), next.top());

    h.frame(vec![
        Event::PointerMoved(first.center()),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: Vec2::new(0., -21.),
            modifiers: Modifiers::NONE,
        },
    ]);
    for _ in 0..25 {
        h.frame(vec![]);
    }
    assert!(h.ui.probe.console_scroll > 0.);
    let output = h.frame(vec![]).0;
    assert_eq!(
        h.ui.probe.widgets["console-header:source"].bottom(),
        header.bottom()
    );
    let partial =
        h.ui.probe
            .widgets
            .iter()
            .find_map(|(key, rect)| {
                (key.starts_with("log:")
                    && rect.top() < header.bottom()
                    && rect.bottom() > header.bottom())
                .then_some(*rect)
            })
            .expect("a partially visible scrolling row");
    assert!(output.shapes.iter().any(|s| matches!(&s.shape,
        egui::Shape::Rect(r) if r.rect == partial
            && s.clip_rect.top() == header.bottom())));
    assert!(!output.shapes.iter().any(|s| matches!(&s.shape,
        egui::Shape::Rect(r) if r.rect == partial
            && s.clip_rect.top() < header.bottom())));
    h.text("search", "matches no console entries");
    assert_eq!(h.ui.probe.widgets["empty-list"].top(), header.bottom());
}

#[test]
fn console_message_wraps_full_text_and_grows_only_its_row() {
    let mut h = Harness::new(1200.);
    let message = format!("{}\n{}", "token".repeat(90), "last line — tiếng Việt");
    console_event(
        &mut h,
        "ios-session-1",
        json!([
            {"type":"console","source":"dart.stdout","message":message},
            {"type":"console","source":"dart.stdout","message":"short next row"}
        ]),
    );
    h.click("Console");
    let long_key = format!("log:{}", h.ui.probe.logs[h.ui.probe.logs.len() - 2]);
    let short_key = format!("log:{}", h.ui.probe.logs[h.ui.probe.logs.len() - 1]);
    let check = |h: &mut Harness| {
        let out = h.frame(vec![]).0;
        let long = h.ui.probe.widgets[&long_key];
        let short = h.ui.probe.widgets[&short_key];
        let message_header = h.ui.probe.widgets["console-header:message"];
        let painted = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == message => Some((t, s.clip_rect)),
                _ => None,
            })
            .expect("the complete Message, including the explicit newline, must be painted");
        let (text, clip) = painted;
        let bounds = Rect::from_min_size(text.pos, text.galley.size());
        assert!(
            text.galley.rows.len() > 2,
            "long unbroken tokens should wrap"
        );
        assert!(clip.contains_rect(bounds));
        assert!(bounds.left() >= message_header.left() && bounds.right() <= message_header.right());
        assert_eq!(long.bottom(), short.top());
        assert_eq!(short.height(), 36.);
        assert!(long.height() > short.height());
        long.height()
    };
    let wide = check(&mut h);
    h.size.x = 480.;
    assert!(check(&mut h) > wide);
}

#[test]
fn console_message_filter_tags_add_remove_and_match_only_message_text() {
    let mut h = Harness::new(860.);
    console_event(
        &mut h,
        "ios-session-1",
        json!([
            {"type":"console","source":"dart.stdout","message":"Auth started"},
            {"type":"console","source":"dart.stderr","message":"SYNC done"},
            {"type":"console","source":"dart.logging","message":"unrelated","loggerName":"auth"}
        ]),
    );
    h.click("Console");
    let baseline = h.ui.probe.logs.clone();
    let entries = &baseline[baseline.len() - 3..];
    h.click("console-message-filter");
    let popup_size = h.ui.probe.widgets["message-filter-popup"].size();
    assert_eq!(popup_size, Vec2::new(302., 242.));
    assert!(
        h.ui.probe.widgets["console-header:message"]
            .contains_rect(h.ui.probe.widgets["console-message-filter"])
    );
    let add = |h: &mut Harness, tag: &str| {
        h.text("message-tag-input", tag);
        h.click("message-tag-add");
        assert_eq!(
            h.ui.probe.widgets["message-filter-popup"].size(),
            popup_size
        );
    };
    let check_icon = |h: &mut Harness, key: &str, expected: &str| {
        let output = h.frame(vec![]).0;
        let rect = h.ui.probe.widgets[key];
        assert_eq!(rect.size(), Vec2::splat(24.));
        assert!(
            output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text)
                if text.galley.text() == expected
                    && text.galley.job.sections[0].format.font_id.size == 14.
                    && (text.pos + text.galley.mesh_bounds.center().to_vec2()).distance(rect.center()) < 0.5)),
            "wrong glyph or alignment for {key}"
        );
    };
    check_icon(&mut h, "message-tag-add", "\u{e047}");
    add(&mut h, "  AuTH  ");
    assert_eq!(h.ui.probe.logs, entries[..1]);
    check_icon(&mut h, "message-tag-remove:0", "\u{e16a}");
    add(&mut h, "auth");
    assert!(!h.ui.probe.widgets.contains_key("message-tag-remove:1"));
    h.text("message-tag-input", "sync");
    h.frame(vec![Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    assert_eq!(h.ui.probe.logs, entries[..2]);
    h.click("message-tag-remove:0");
    assert_eq!(h.ui.probe.logs, entries[1..2]);
    h.click("message-tag-remove:0");
    assert_eq!(h.ui.probe.logs, baseline);
    add(&mut h, "auth");
    h.click("source");
    h.click("source:dart.stderr");
    assert!(h.ui.probe.logs.is_empty());
    h.click("reset");
    assert_eq!(h.ui.probe.logs, baseline);

    h.click("console-message-filter");
    for tag in ["A", "Longer tag", "W".repeat(130).as_str()] {
        add(&mut h, tag);
    }
    let output = h.frame(vec![]).0;
    let popup = h.ui.probe.widgets["message-filter-popup"];
    let card = |i| h.ui.probe.widgets[&format!("message-tag-card:{i}")];
    let (first, second, long) = (card(0), card(1), card(2));
    assert_eq!(first.top(), second.top());
    assert_eq!(second.left() - first.right(), 8.);
    assert!(first.width() < second.width());
    assert_eq!(long.left(), first.left());
    assert_eq!(long.top() - first.bottom(), 8.);
    for (i, bounds) in [first, second, long].into_iter().enumerate() {
        assert!(popup.contains_rect(bounds));
        assert!(bounds.contains_rect(h.ui.probe.widgets[&format!("message-tag-remove:{i}")]));
        assert!(
            output
                .shapes
                .iter()
                .any(|shape| matches!(&shape.shape, egui::Shape::Rect(r)
            if r.rect == bounds && r.stroke.width == 1.
                && r.stroke.color == flutter_network_viewer::app_colors::LIGHT.line))
        );
    }
    for i in 3..30 {
        add(&mut h, &format!("tag {i}"));
    }
    h.frame(vec![
        Event::PointerMoved(popup.center()),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: Vec2::new(0., -560.),
            modifiers: Modifiers::NONE,
        },
    ]);
    for _ in 0..20 {
        h.frame(vec![]);
    }
    assert_eq!(h.ui.probe.widgets["message-filter-popup"], popup);
    assert!(popup.contains_rect(h.ui.probe.widgets["message-tag-remove:29"]));
    h.click("message-tag-remove:29");
    assert!(!h.ui.probe.widgets.contains_key("message-tag-remove:29"));
}

#[test]
fn console_column_dividers_resize_adjacent_cells_and_reflow_message() {
    let mut h = Harness::new(860.);
    let message = "unbroken-token".repeat(55);
    console_event(
        &mut h,
        "ios-session-1",
        json!([
            {"type":"console","source":"dart.stdout","message":message}
        ]),
    );
    h.click("Console");
    let header = |h: &Harness| {
        ["source", "level", "time", "message"]
            .map(|key| h.ui.probe.widgets[&format!("console-header:{key}")])
    };
    let drag = |h: &mut Harness, index, dx| {
        let from = h.ui.probe.widgets[&format!("console-column:{index}")].center();
        let to = from + Vec2::new(dx, 0.);
        let hover = h.frame(vec![Event::PointerMoved(from)]).0;
        assert_eq!(
            hover.platform_output.cursor_icon,
            egui::CursorIcon::ResizeHorizontal
        );
        let button = |pos, pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        };
        h.frame(vec![button(from, true)]);
        h.frame(vec![Event::PointerMoved(to)]);
        h.frame(vec![button(to, false)]);
        h.frame(vec![]).0
    };
    let row_key = format!("log:{}", h.ui.probe.logs.last().unwrap());
    let old_height = h.ui.probe.widgets[&row_key].height();
    for i in 0..3 {
        let before = header(&h);
        let output = drag(&mut h, i, 18.);
        let after = header(&h);
        assert_eq!(after[i].width(), before[i].width() + 18.);
        assert_eq!(after[i + 1].width(), before[i + 1].width() - 18.);
        for j in 0..4 {
            if j != i && j != i + 1 {
                assert_eq!(before[j], after[j]);
            }
        }
        let painted = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == message => Some(t),
                _ => None,
            })
            .unwrap();
        assert_eq!(painted.pos.x, after[3].left() + 8.);
        assert!(h.ui.selected_log.is_none());
    }
    assert!(h.ui.probe.widgets[&row_key].height() > old_height);
    drag(&mut h, 2, 2_000.);
    assert!(header(&h)[3].width() >= 80.);
    h.size.x = 480.;
    h.frame(vec![]);
    let narrowed = header(&h);
    assert!(narrowed[3].width() >= 80.);
    assert!(narrowed[3].right() <= h.size.x);
    h.click("source");
    assert!(h.ui.probe.widgets.contains_key("source:dart.stdout"));
}

#[test]
fn console_and_network_inspectors_share_tabs_spacing_and_json_controls() {
    for (width, font_size) in [(860., 14.), (1440., 14.), (1440., 24.)] {
        let mut h = Harness::new(width);
        h.ui.settings.set_font_size(font_size).unwrap();
        if font_size == 24. {
            h.ui.settings.set_inspector_width(320., true).unwrap();
        }
        let geometry = |h: &Harness, tab: &str| {
            let w = &h.ui.probe.widgets;
            let tabs = w["inspector-tabs"];
            let toolbar = w["json-toolbar"];
            let row = w["json-output"];
            for key in [
                "json-find",
                "json-case",
                "json-prev",
                "json-next",
                "json-clear",
                "json-code",
                "json-tree",
                "json-copy",
            ] {
                assert!(
                    toolbar.contains_rect(w[key]),
                    "{key} does not fit within {toolbar:?}"
                );
            }
            [
                w[tab].height(),
                w[tab].top() - tabs.top(),
                tabs.bottom() - w[tab].bottom(),
                toolbar.top() - tabs.bottom(),
                row.top() - toolbar.bottom(),
                w["json-copy"].height(),
            ]
        };
        h.click("request:ios-session-1:fetch-1");
        let table_geometry = |h: &Harness| {
            let w = &h.ui.probe.widgets;
            [
                w["metadata-viewport"].top() - w["inspector-tabs"].bottom(),
                w["metadata:cell:0:0"].width(),
                w["metadata:cell:0:2"].width(),
                w["metadata:copy:0"].height(),
            ]
        };
        let network_table = table_geometry(&h);
        h.click("tab:5");
        let network = geometry(&h, "tab:5");
        if width < 900. {
            h.escape();
        }
        h.click("Console");
        let id = h.ui.probe.logs[0];
        h.click(&format!("log:{id}"));
        assert_eq!(h.ui.console_raw_serializations, 0);
        let check_spacing = |h: &Harness, tab| {
            let console = geometry(h, tab);
            assert!(
                console
                    .iter()
                    .zip(network)
                    .all(|(a, b)| (a - b).abs() <= 0.5),
                "{tab}: Console {console:?}, Network {network:?}"
            );
        };
        assert!(h.ui.probe.widgets.contains_key("metadata:row:0"));
        assert!(
            table_geometry(&h)
                .iter()
                .zip(network_table)
                .all(|(a, b)| (a - b).abs() <= 0.5)
        );
        assert!(!h.ui.probe.widgets.contains_key("inspector-heading"));
        assert!(!h.ui.probe.widgets.contains_key("json-toolbar"));
        h.click("console-raw");
        check_spacing(&h, "console-raw");
        h.text("json-find", "message");
        assert!(h.ui.probe.json.matches > 0);
        h.click("console-detail");
        assert!(!h.ui.probe.widgets.contains_key("json-toolbar"));
    }
}
