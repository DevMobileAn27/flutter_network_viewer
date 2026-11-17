use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, Rect};
use flutter_network_viewer::{
    capture::CaptureStore,
    desktop::{DesktopUi, ShellStatus},
    desktop_support::{DISCUSSION_URL, NoticeKind},
};
struct Harness {
    ctx: egui::Context,
    ui: DesktopUi,
    store: CaptureStore,
    status: ShellStatus,
    time: f64,
    size: egui::Vec2,
    actions: Vec<flutter_network_viewer::desktop::Action>,
}
impl Harness {
    fn new() -> Self {
        let mut h = Self {
            ctx: Default::default(),
            ui: Default::default(),
            store: Default::default(),
            status: Default::default(),
            time: 0.,
            size: egui::vec2(1200., 800.),
            actions: vec![],
        };
        h.frame(vec![]);
        h.frame(vec![]);
        h
    }
    fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ctx| {
                self.actions
                    .extend(self.ui.show(ctx, &self.store, &self.status));
            },
        )
    }
    fn click(&mut self, key: &str) -> egui::FullOutput {
        for _ in 0..4 {
            self.frame(vec![]);
        }
        if self.ui.support.settings_open
            && ["mcp-port", "language", "theme", "auto-detect"].contains(&key)
        {
            for _ in 0..12 {
                let pane = self.ui.probe.widgets["settings-scroll"];
                let target = self.ui.probe.widgets[key];
                if pane.contains_rect(target) {
                    break;
                }
                self.frame(vec![
                    Event::PointerMoved(pane.center()),
                    Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::vec2(
                            0.,
                            (pane.center().y - target.center().y).clamp(-350., 350.),
                        ),
                        modifiers: Modifiers::NONE,
                    },
                ]);
                for _ in 0..6 {
                    self.time += 0.02;
                    self.frame(vec![]);
                }
            }
        }
        let pos = self
            .ui
            .probe
            .widgets
            .get(key)
            .unwrap_or_else(|| panic!("missing {key}: {:?}", self.ui.probe.widgets.keys()))
            .center();
        self.frame(vec![Event::PointerMoved(pos)]);
        self.frame(vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        }]);
        let o = self.frame(vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
        self.frame(vec![]);
        o
    }
}
#[test]
fn help_keeps_discussion_link_without_copy_action() {
    let mut h = Harness::new();
    h.click("help");
    assert!(h.ui.probe.widgets.contains_key("help-scroll"));
    assert!(!h.ui.probe.widgets.contains_key("vm-uri"));
    let o = h.click("discussion");
    assert!(
        o.platform_output
            .commands
            .iter()
            .any(|c| matches!(c,egui::OutputCommand::OpenUrl(u) if u.url==DISCUSSION_URL))
    );
    assert!(!h.ui.probe.widgets.contains_key("discussion-copy"));
    assert!(
        !o.platform_output
            .commands
            .iter()
            .any(|command| matches!(command, egui::OutputCommand::CopyText(_)))
    );
    h.click("help-close");
    assert!(!h.ui.support.help_open, "help did not close");
    assert!(!h.ui.probe.widgets.contains_key("vm-uri"));
}
#[test]
fn settings_dropdown_inputs_theme_locale_and_discovery() {
    let mut h = Harness::new();
    assert!(!h.ctx.style().visuals.dark_mode);
    h.click("settings");
    h.click("language");
    h.click("language:vi");
    assert_eq!(h.ui.settings.values().language_code, "vi");
    h.click("theme");
    h.click("theme:dark");
    h.frame(vec![]);
    assert!(h.ctx.style().visuals.dark_mode);
    assert!(h.ui.probe.support_text.contains("Cài đặt"));
    assert!(!h.ui.probe.widgets.contains_key("auto-detect"));
    assert!(!h.ui.settings.values().auto_detect_vm_service);
    h.click("theme");
    h.click("theme:light");
    h.click("language");
    h.click("language:en");
    h.frame(vec![]);
    assert!(!h.ctx.style().visuals.dark_mode);
    assert!(h.ui.probe.support_text.contains("Setting"));
}
#[test]
fn toast_variants_position_rows_and_exact_mounted_deadlines() {
    let mut h = Harness::new();
    for kind in [
        NoticeKind::Info,
        NoticeKind::Success,
        NoticeKind::Warning,
        NoticeKind::Error,
    ] {
        let r = h.ui.notifications.show(kind, "details".into(), false);
        h.frame(vec![]);
        h.frame(vec![]);
        let expected_color = match kind {
            NoticeKind::Info => flutter_network_viewer::app_colors::NOTICE_LIGHT[0],
            NoticeKind::Success => flutter_network_viewer::app_colors::NOTICE_LIGHT[1],
            NoticeKind::Warning => flutter_network_viewer::app_colors::NOTICE_LIGHT[2],
            NoticeKind::Error => flutter_network_viewer::app_colors::NOTICE_LIGHT[3],
        };
        assert_eq!(h.ui.probe.notification_color, expected_color);
        let p = &h.ui.probe.widgets;
        let rect = p["notification"];
        assert!((rect.left() - 20.).abs() < 1.);
        assert!(
            (rect.bottom() - 780.).abs() < 1.,
            "notice rect {rect:?} at {:?}",
            kind
        );
        assert!(p["notification-title"].top() < p["notification-description"].top());
        assert!(p["notification-description"].top() < p["notification-confirm"].top());
        h.time += 2.999;
        h.frame(vec![]);
        assert_eq!(r.result(), None);
        h.time += 0.001;
        h.frame(vec![]);
        if matches!(kind, NoticeKind::Info | NoticeKind::Success) {
            assert_eq!(r.result(), Some(false));
        } else {
            assert_eq!(r.result(), None);
            h.click("notification-confirm");
            assert_eq!(r.result(), Some(true));
        }
    }
}
#[test]
fn confirmation_replace_and_dispose_cancel_pending_deadlines() {
    let mut h = Harness::new();
    let old =
        h.ui.notifications
            .show(NoticeKind::Info, "old".into(), false);
    h.frame(vec![]);
    h.time = 2.;
    let r =
        h.ui.notifications
            .show(NoticeKind::Success, "confirm".into(), true);
    assert_eq!(old.result(), Some(false));
    h.frame(vec![]);
    h.time = 10.;
    h.frame(vec![]);
    assert_eq!(r.result(), None);
    h.click("notification-cancel");
    assert_eq!(r.result(), Some(false));
    let r =
        h.ui.notifications
            .show(NoticeKind::Info, "dispose".into(), false);
    h.frame(vec![]);
    drop(h);
    assert_eq!(r.result(), Some(false));
}
#[test]
fn narrow_support_dialogs_and_toast_scroll_with_both_locales_themes() {
    for lang in ["vi", "en"] {
        for dark in [false, true] {
            let mut h = Harness::new();
            h.ui.settings.set_language(lang).unwrap();
            h.ui.settings
                .set_theme_mode(if dark {
                    flutter_network_viewer::settings::ThemeMode::Dark
                } else {
                    flutter_network_viewer::settings::ThemeMode::Light
                })
                .unwrap();
            h.click("help");
            h.size = egui::vec2(360., 300.);
            h.frame(vec![]);
            h.frame(vec![]);
            let r = h.ui.probe.widgets["help-scroll"];
            assert!(r.width() <= 340.);
            assert!(r.height() < 260.);
            let before = h.ui.probe.support_scroll;
            h.frame(vec![
                Event::PointerMoved(r.center()),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0., -500.),
                    modifiers: Modifiers::NONE,
                },
            ]);
            for _ in 0..10 {
                h.time += 0.02;
                h.frame(vec![]);
            }
            assert!(h.ui.probe.support_scroll > before);
            h.click("help-close");
            let receipt =
                h.ui.notifications
                    .show(NoticeKind::Warning, "long description ".repeat(200), true);
            h.frame(vec![]);
            h.frame(vec![]);
            assert!(h.ui.probe.widgets["notification"].width() <= 320.);
            assert_eq!(receipt.result(), None);
        }
    }
}
#[test]
fn custom_delay_disable_zero_negative_and_nonmodal_interaction() {
    let mut h = Harness::new();
    assert!(
        h.ui.notifications
            .show_with_options(NoticeKind::Info, "invalid".into(), true, Some(true), -1)
            .is_err()
    );
    let r =
        h.ui.notifications
            .show_with_options(NoticeKind::Info, "immediate".into(), false, Some(true), 0)
            .unwrap();
    h.frame(vec![]);
    assert_eq!(r.result(), Some(false));
    let r =
        h.ui.notifications
            .show_with_options(NoticeKind::Success, "manual".into(), false, Some(false), 1)
            .unwrap();
    h.frame(vec![]);
    h.time = 10.;
    h.frame(vec![]);
    assert_eq!(r.result(), None);
    h.click("settings");
    assert!(h.ui.probe.widgets.contains_key("language"));
    h.click("settings-close");
    h.click("notification-confirm");
    assert_eq!(r.result(), Some(true));
    let r =
        h.ui.notifications
            .show_with_options(
                NoticeKind::Warning,
                "timed confirmation".into(),
                true,
                Some(true),
                1500,
            )
            .unwrap();
    h.time = 100.;
    h.frame(vec![]);
    h.time = 101.499;
    h.frame(vec![]);
    assert_eq!(r.result(), None);
    h.time = 101.5;
    h.frame(vec![]);
    assert_eq!(r.result(), Some(false));
}
#[test]
fn ui_settings_persist_reload_preserving_layout_and_manual_validation() {
    use flutter_network_viewer::settings::AppSettings;
    let path = std::env::temp_dir().join(format!("f17-settings-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut settings = AppSettings::load(&path).unwrap();
    settings.set_request_detail_tab(4).unwrap();
    settings.set_inspector_width(432., false).unwrap();
    let mut h = Harness::new();
    h.ui = DesktopUi::new(settings);
    h.frame(vec![]);
    h.frame(vec![]);
    h.click("settings");
    h.click("language");
    h.click("language:vi");
    h.click("theme");
    h.click("theme:dark");
    assert!(!h.ui.probe.widgets.contains_key("auto-detect"));
    let settings = AppSettings::load(&path).unwrap();
    assert_eq!(settings.values().language_code, "vi");
    assert_eq!(settings.values().request_detail_tab, 4);
    assert_eq!(settings.values().network_inspector_width, Some(432.));
    assert!(!settings.values().auto_detect_vm_service);
    h.ui = DesktopUi::new(settings);
    h.frame(vec![]);
    assert!(h.ctx.style().visuals.dark_mode);
    h.click("manual-url");
    h.click("vm-uri");
    h.frame(vec![Event::Text("not a URL".into())]);
    h.click("connect-submit");
    assert_eq!(h.ui.settings.values().vm_service_uri, "");
    assert!(h.ui.probe.widgets.contains_key("notification"));
    std::fs::remove_file(path).unwrap();
}
#[test]
fn localized_body_notices_update_without_redecode_or_payload_translation() {
    use flutter_network_viewer::desktop_body::BodyViewer;
    use serde_json::json;
    let ctx = egui::Context::default();
    let mut settings = flutter_network_viewer::settings::AppSettings::default();
    let mut body = BodyViewer::default();
    let mut probe = flutter_network_viewer::desktop::UiProbe::default();
    for (rev, data, en, vi) in [
        (
            1,
            json!({"bodyOmitted":"profile-size-limit","bodyCaptureLimit":1024}),
            "capture limit",
            "giới hạn",
        ),
        (
            2,
            json!({"bodyOmitted":"read-failed"}),
            "read failed",
            "Không thể đọc",
        ),
        (
            3,
            json!({"headers":{"content-type":"image/png"},"body":"!!!","bodyEncoding":"base64"}),
            "Image unavailable",
            "Không thể xem ảnh",
        ),
    ] {
        assert!(body.load(&data, rev));
        for lang in ["en", "vi"] {
            settings.set_language(lang).unwrap();
            let _ = ctx.run(Default::default(), |ctx| {
                flutter_network_viewer::desktop_support::apply(ctx, &settings);
                egui::CentralPanel::default().show(ctx, |ui| {
                    assert!(body.show(ui, &mut probe, "corrupt"));
                });
            });
            assert!(
                probe
                    .body_preview
                    .contains(if lang == "en" { en } else { vi }),
                "rev={rev} lang={lang} actual={}",
                probe.body_preview
            );
            assert!(!body.load(&data, rev));
        }
    }
}
#[test]
fn vietnamese_ui_glyphs_exist_in_both_font_families() {
    let mut h = Harness::new();
    h.ui.settings.set_language("vi").unwrap();
    h.frame(vec![]);
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        assert!(h.ctx.fonts_mut(|f| f.has_glyphs(
            &egui::FontId::new(14., family),
            "Cài đặt Ngôn ngữ Kết nối Hướng dẫn sử dụng ăâđêôơư Ắấậặằẵẳ ếềễểệ ốồỗổộ ớờỡởợ ứừữửự"
        )));
    }
}

#[test]
fn manual_connection_dispatches_normalized_uri_without_settings_section() {
    let mut h = Harness::new();
    h.click("settings");
    assert!(!h.ui.probe.widgets.contains_key("auto-detect"));
    assert!(
        !h.actions
            .contains(&flutter_network_viewer::desktop::Action::StartScan)
    );
    h.click("settings-close");
    h.click("manual-url");
    h.click("vm-uri");
    h.frame(vec![Event::Text("http://127.0.0.1:8181/auth=/".into())]);
    h.click("connect-submit");
    assert!(
        h.actions
            .contains(&flutter_network_viewer::desktop::Action::Connect(
                "ws://127.0.0.1:8181/auth=/ws".into()
            ))
    );
    // DesktopHost persists the normalized URI after successful connection.
    assert_eq!(h.ui.settings.values().vm_service_uri, "");
    assert!(!h.ui.settings.values().auto_detect_vm_service);
}
#[test]
fn dropdown_escape_and_outside_restore_closed_geometry_without_change() {
    let mut h = Harness::new();
    h.click("settings");
    let closed = h.ui.probe.widgets["language"];
    h.click("language");
    assert!(h.ui.probe.widgets.contains_key("language:vi"));
    h.frame(vec![Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![]);
    assert!(!h.ui.probe.widgets.contains_key("language:vi"));
    assert!(h.ui.probe.widgets.contains_key("theme"));
    assert_eq!(h.ui.settings.values().language_code, "en");
    assert_eq!(h.ui.probe.widgets["language"], closed);
}

#[test]
fn language_switch_keeps_capture_filters_and_connection_draft_during_scan_updates() {
    let mut h = Harness::new();
    h.store
        .ingest(&serde_json::from_str(include_str!("fixtures/sdk_envelope.json")).unwrap())
        .unwrap();
    h.frame(vec![]);
    h.click("method");
    h.click("method:POST");
    assert_eq!(h.ui.probe.requests.len(), 1);
    h.click("settings");
    h.click("language");
    h.click("language:vi");
    h.click("settings-close");
    h.click("manual-url");
    h.click("vm-uri");
    h.frame(vec![Event::Text("http://127.0.0.1:8181/token=/".into())]);
    h.status.searching = true;
    h.frame(vec![]);
    h.click("connect-submit");
    assert!(
        h.actions
            .contains(&flutter_network_viewer::desktop::Action::Connect(
                "ws://127.0.0.1:8181/token=/ws".into()
            ))
    );
    assert_eq!(h.ui.probe.requests.len(), 1);
    assert_eq!(h.store.network().len(), 3);
}
#[test]
fn settings_header_stays_fixed_while_narrow_content_scrolls() {
    let mut h = Harness::new();
    h.click("settings");
    h.size = egui::vec2(360., 300.);
    for _ in 0..6 {
        h.frame(vec![]);
    }
    let before = h.ui.probe.widgets["settings-close"];
    let rect = h.ui.probe.widgets["settings-scroll"];
    assert!((rect.width() - 360.).abs() < 1.);
    let back = h.ui.probe.widgets["settings-close"];
    assert!((back.top() - 8.).abs() < 1.);
    assert!((rect.top() - back.bottom() - 8.).abs() < 1.);
    assert!((rect.height() - (300. - rect.top())).abs() < 1.);
    h.frame(vec![
        Event::PointerMoved(rect.right_center() - egui::vec2(5., 0.)),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0., -2000.),
            modifiers: Modifiers::NONE,
        },
    ]);
    for _ in 0..20 {
        h.time += 0.02;
        h.frame(vec![]);
    }
    assert!(h.ui.probe.support_scroll > 0.);
    assert_eq!(h.ui.probe.widgets["settings-close"], before);
    h.click("settings-close");
    assert!(!h.ui.probe.widgets.contains_key("settings-scroll"));
}
#[test]
fn settings_write_failure_keeps_committed_value_and_shows_error() {
    let mut h = Harness::new();
    let missing = std::env::temp_dir()
        .join(format!("f17-missing-{}", std::process::id()))
        .join("settings.json");
    assert!(!missing.parent().unwrap().exists());
    h.ui = DesktopUi::new(flutter_network_viewer::settings::AppSettings::load(missing).unwrap());
    h.frame(vec![]);
    h.click("settings");
    h.click("theme");
    h.click("theme:dark");
    assert_eq!(
        h.ui.settings.values().theme_mode,
        flutter_network_viewer::settings::ThemeMode::Light
    );
    assert!(!h.ctx.style().visuals.dark_mode);
    assert_eq!(h.ui.probe.notification_title, "Error");
}
#[test]
fn large_json_draft_exposes_visible_input_hit_target_after_scroll() {
    let mut h = Harness::new();
    h.click("JSON");
    h.click("json-input");
    h.frame(vec![Event::Text("{\"x\":\"long value\"}\n".repeat(1500))]);
    for _ in 0..6 {
        h.frame(vec![]);
    }
    let rect = h.ui.probe.widgets["json-input"];
    assert!(
        Rect::from_min_size(Pos2::ZERO, h.size).contains_rect(rect),
        "input hit target must be clipped to visible surface: {rect:?}"
    );
    h.click("json-input");
    h.frame(vec![
        Event::Key {
            key: egui::Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers {
                ctrl: true,
                command: true,
                mac_cmd: true,
                ..Modifiers::NONE
            },
        },
        Event::Text("null".into()),
    ]);
    h.click("json-format");
    assert_eq!(h.ui.probe.json.text, "null");
}

#[test]
fn bundled_geist_and_cascadia_faces_keep_vietnamese_and_settings_field_styling() {
    let mut h = Harness::new();
    for name in ["Geist", "Cascadia Mono", "Cascadia Mono Italic"] {
        assert!(
            h.ctx.fonts_mut(|fonts| fonts.has_glyphs(
                &egui::FontId::new(13., egui::FontFamily::Name(name.into())),
                "Tiếng Việt Đường dẫn ứng dụng"
            )),
            "{name}"
        );
    }
    h.click("settings");
    let out = h.frame(vec![]);
    let english = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == "English" => Some(t),
            _ => None,
        })
        .unwrap();
    assert_eq!(
        english.galley.job.sections[0].format.font_id,
        egui::FontId::new(14., egui::FontFamily::Name("Geist".into()))
    );
    let uri = h.ui.probe.widgets["mcp-port"];
    assert!(
        uri.height() >= 40.,
        "text field needs the Flutter 12px vertical padding"
    );
}

#[test]
fn settings_is_a_full_page_with_compact_section_order_and_back_navigation() {
    let mut h = Harness::new();
    h.size = egui::vec2(1360., 900.);
    h.frame(vec![]);
    h.click("settings");
    assert!(
        !h.ui.probe.widgets.contains_key("settings"),
        "main shell must leave the Settings route"
    );
    let appearance = h.ui.probe.widgets["settings-appearance"];
    assert!((appearance.left() - 8.).abs() < 1., "{appearance:?}");
    assert!((appearance.width() - 1344.).abs() < 1., "{appearance:?}");
    assert!(
        (appearance.top() - h.ui.probe.widgets["settings-scroll"].top() - 8.).abs() < 1.,
        "{appearance:?}"
    );
    assert!(h.ui.probe.widgets["theme"].top() < h.ui.probe.widgets["language"].top());
    assert!(appearance.bottom() < h.ui.probe.widgets["settings-mcp"].top());
    assert!(
        h.ui.probe.widgets["settings-mcp"].bottom() < h.ui.probe.widgets["settings-updates"].top()
    );
    let mcp = h.ui.probe.widgets["settings-mcp"];
    assert!(mcp.contains_rect(h.ui.probe.widgets["mcp-enabled"]));
    assert!(h.ui.probe.widgets["mcp-enabled"].top() < h.ui.probe.widgets["mcp-port"].top());
    assert!(h.ui.probe.widgets["mcp-endpoint"].left() > h.ui.probe.widgets["mcp-port"].right());
    h.click("settings-close");
    assert!(!h.ui.support.settings_open);
    assert!(h.ui.probe.widgets.contains_key("settings"));
    h.size = egui::vec2(700., 800.);
    h.frame(vec![]);
    h.click("settings");
    assert!((h.ui.probe.widgets["settings-appearance"].left() - 8.).abs() < 1.);
    h.frame(vec![Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    assert!(!h.ui.support.settings_open);
}

#[test]
fn settings_compact_header_balances_text_back_and_removes_connection() {
    for (width, size, lang, back, title) in [
        (1360., 14., "en", "Back", "Setting"),
        (360., 14., "vi", "Quay lại", "Cài đặt"),
        (700., 24., "vi", "Quay lại", "Cài đặt"),
    ] {
        let mut h = Harness::new();
        h.size = egui::vec2(width, 900.);
        h.ui.settings.set_language(lang).unwrap();
        h.ui.settings.set_font_size(size).unwrap();
        h.click("settings");
        h.frame(vec![Event::PointerMoved(egui::pos2(width - 2., 880.))]);
        h.frame(vec![]);
        let output = h.frame(vec![]);
        for key in ["settings-connection", "settings-uri", "settings-connect"] {
            assert!(!h.ui.probe.widgets.contains_key(key), "removed {key}");
        }
        let button = h.ui.probe.widgets["settings-close"];
        let spacer = h.ui.probe.widgets["settings-header-spacer"];
        assert!((button.left() - 8.).abs() < 1.);
        assert!((button.top() - 8.).abs() < 1.);
        let header_bottom = h.ui.probe.widgets["settings-scroll"].top();
        assert!((header_bottom - button.bottom() - 8.).abs() < 1.);
        assert!((spacer.width() - button.width()).abs() < 0.5);
        for (label, center) in [
            (back, button.center()),
            (title, egui::pos2(width / 2., button.center().y)),
        ] {
            let text = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == label => Some(t),
                    _ => None,
                })
                .expect(label);
            assert!(
                (text.pos + text.galley.size() / 2. - center).length() < 1.,
                "{label}: text={:?} expected={center:?} button={button:?} spacer={spacer:?}",
                text.pos + text.galley.size() / 2.
            );
        }
        for key in ["settings-appearance", "settings-mcp", "settings-updates"] {
            let rect = h.ui.probe.widgets[key];
            assert!((rect.left() - 8.).abs() < 1., "{rect:?}");
            assert!((rect.right() - (width - 8.)).abs() < 1., "{rect:?}");
        }
        h.click("settings-close");
        assert!(!h.ui.support.settings_open);
    }
}

#[test]
fn settings_icons_labels_fields_and_actions_follow_requested_layout() {
    let mut h = Harness::new();
    h.size = egui::vec2(1440., 2000.);
    h.click("settings");
    let out = h.frame(vec![]);
    for (key, glyph) in [
        ("settings-appearance", '\u{e683}'),
        ("settings-mcp", '\u{f0616}'),
        ("settings-updates", '\u{e637}'),
    ] {
        let card = h.ui.probe.widgets[key];
        assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t)
            if card.contains(t.pos) && t.galley.text() == glyph.to_string()
                && t.galley.job.sections[0].format.font_id.family == egui::FontFamily::Name("MaterialIcons".into()))), "wrong {key} icon");
    }
    assert!(!h.ui.probe.widgets.contains_key("auto-detect"));
    for (label, key) in [("MCP port", "mcp-port"), ("MCP endpoint", "mcp-endpoint")] {
        let input = h.ui.probe.widgets[key];
        let pos = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == label => Some(t.pos),
                _ => None,
            })
            .unwrap();
        assert!(pos.y + 12. < input.top(), "{label} must be above its field");
        assert!(
            (pos.x - input.left()).abs() <= 12.,
            "{label} must align with field"
        );
    }
    for (button, section) in [
        ("mcp-copy-codex", "settings-mcp"),
        ("check-app-updates", "settings-updates"),
    ] {
        assert!(
            (h.ui.probe.widgets[section].right() - 8. - h.ui.probe.widgets[button].right()).abs()
                < 2.,
            "{button} must align right"
        );
    }
}

#[test]
fn mcp_endpoint_and_port_text_are_centered_vertically() {
    let mut h = Harness::new();
    h.size = egui::vec2(1440., 2000.);
    h.click("settings");
    let out = h.frame(vec![]);
    for (key, value) in [
        ("mcp-port", "38943"),
        ("mcp-endpoint", "http://127.0.0.1:38943/mcp"),
    ] {
        let rect = h.ui.probe.widgets[key];
        let center = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == value => {
                    Some(t.pos.y + t.galley.size().y / 2.)
                }
                _ => None,
            })
            .unwrap();
        assert!(
            (center - rect.center().y).abs() < 1.,
            "{key} is not centered"
        );
    }
}

#[test]
fn settings_text_fields_have_gray_idle_and_blue_focus_borders() {
    for dark in [false, true] {
        let mut h = Harness::new();
        h.ui.settings
            .set_theme_mode(if dark {
                flutter_network_viewer::settings::ThemeMode::Dark
            } else {
                flutter_network_viewer::settings::ThemeMode::Light
            })
            .unwrap();
        h.click("settings");
        h.click("settings-mcp");
        let p = flutter_network_viewer::desktop_style::Palette::new(dark);
        let check = |h: &mut Harness, key, color| {
            let out = h.frame(vec![]);
            let rect = h.ui.probe.widgets[key];
            assert!(
                out.shapes
                    .iter()
                    .any(|s| matches!(&s.shape, egui::Shape::Rect(r)
                if r.rect == rect && s.clip_rect.contains_rect(rect)
                    && r.stroke.color == color && r.stroke.width >= 1.)),
                "missing {key} border at {rect:?}"
            );
        };
        check(&mut h, "mcp-port", p.line);
        check(&mut h, "mcp-endpoint", p.line);
        h.click("mcp-port");
        h.frame(vec![Event::Text("4".into())]);
        check(&mut h, "mcp-port", p.accent);
        h.click("mcp-endpoint");
        check(&mut h, "mcp-port", p.line);
        check(&mut h, "mcp-endpoint", p.accent);
    }
}

#[test]
fn help_guide_has_compact_header_padding_and_current_connection_steps() {
    for (width, height, lang) in [(1200., 800., "en"), (480., 420., "vi")] {
        let mut h = Harness::new();
        h.size = egui::vec2(width, height);
        h.ui.settings.set_language(lang).unwrap();
        h.click("help");
        let bounds = h.ui.probe.widgets["help-modal"];
        let title = h.ui.probe.widgets["help-title"];
        let dismiss = h.ui.probe.widgets["help-dismiss"];
        let close = h.ui.probe.widgets["help-close"];
        let viewport = Rect::from_min_size(Pos2::ZERO, h.size);
        assert!(viewport.contains_rect(bounds));
        assert!(title.left() >= bounds.left() + 15.);
        assert!(dismiss.right() <= bounds.right() - 15.);
        assert!(dismiss.center().x > title.center().x);
        assert!(close.right() <= bounds.right() - 15.);
        assert!(close.center().x > bounds.center().x);
        assert!(h.ui.probe.support_text.contains("Manual URL"));
        assert!(!h.ui.probe.support_text.contains("Settings → Connection"));
        assert!(!h.ui.probe.support_text.contains("Find apps discovers"));
        h.click("help-dismiss");
        assert!(!h.ui.support.help_open);
    }
}

#[test]
fn settings_back_hover_grows_in_place_without_resizing_header() {
    for (width, size, lang) in [(1360., 14., "en"), (360., 14., "vi"), (700., 24., "vi")] {
        let mut h = Harness::new();
        h.size = egui::vec2(width, 900.);
        h.ui.settings.set_font_size(size).unwrap();
        h.ui.settings.set_language(lang).unwrap();
        h.click("settings");
        h.frame(vec![Event::PointerMoved(egui::pos2(width - 2., 880.))]);
        for _ in 0..4 {
            h.frame(vec![]);
        }
        let back = h.ui.probe.widgets["settings-close"];
        let keys = [
            "settings-scroll",
            "settings-title",
            "settings-header-spacer",
            "settings-appearance",
            "settings-mcp",
            "settings-updates",
        ];
        let before: Vec<_> = keys.iter().map(|k| h.ui.probe.widgets[*k]).collect();
        for hovered in [true, false, true, false] {
            for _ in 0..6 {
                let out = h.frame(vec![Event::PointerMoved(if hovered {
                    back.center()
                } else {
                    egui::pos2(width - 2., 880.)
                })]);
                for (key, expected) in keys.iter().zip(&before) {
                    let actual = h.ui.probe.widgets[*key];
                    assert!(
                        (actual.min - expected.min).length() < 0.5
                            && (actual.max - expected.max).length() < 0.5,
                        "{key}: {actual:?} vs {expected:?}"
                    );
                }
                if hovered {
                    assert_eq!(
                        out.platform_output.cursor_icon,
                        egui::CursorIcon::PointingHand
                    );
                }
            }
            let current = h.ui.probe.widgets["settings-close"];
            let factor = 1.;
            assert!((current.width() / back.width() - factor).abs() < 0.02);
            assert!(
                (current.height() - back.height() * factor).abs() <= 1.,
                "current={current:?} back={back:?} factor={factor}"
            );
            assert!((current.center() - back.center()).length() < 1.);
        }
        h.click("settings-close");
        assert!(!h.ui.support.settings_open);
    }
}

#[test]
fn font_size_value_is_display_only_and_buttons_change_and_persist_size() {
    use flutter_network_viewer::settings::AppSettings;
    let path = std::env::temp_dir().join(format!("fnv-font-display-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut h = Harness::new();
    h.ui = DesktopUi::new(AppSettings::load(&path).unwrap());
    h.click("settings");
    h.click("font-size");
    let rect = h.ui.probe.widgets["font-size"];
    let out = h.frame(vec![Event::PointerMoved(rect.center())]);
    assert_eq!(out.platform_output.cursor_icon, egui::CursorIcon::Default);
    assert!(
        h.ctx.memory(|m| m.focused()).is_none(),
        "display must not receive keyboard focus"
    );
    h.frame(vec![
        Event::Text("22".into()),
        Event::Key {
            key: egui::Key::ArrowUp,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        },
    ]);
    h.frame(vec![Event::PointerButton {
        pos: rect.center(),
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![Event::PointerMoved(
        rect.center() + egui::vec2(80., 0.),
    )]);
    h.frame(vec![Event::PointerButton {
        pos: rect.center() + egui::vec2(80., 0.),
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    assert_eq!(h.ui.settings.values().font_size, 14.);
    assert_eq!(AppSettings::load(&path).unwrap().values().font_size, 14.);
    for (key, expected) in [("font-size-increase", 15.), ("font-size-decrease", 14.)] {
        h.click(key);
        assert_eq!(h.ui.settings.values().font_size, expected);
        assert_eq!(
            AppSettings::load(&path).unwrap().values().font_size,
            expected
        );
        h.ui.notifications.dismiss(false);
        let out = h.frame(vec![]);
        let rect = h.ui.probe.widgets["font-size"];
        let text = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t)
                    if t.galley.text() == format!("{expected:.0}") && rect.contains(t.pos) =>
                {
                    Some(t)
                }
                _ => None,
            })
            .expect("displayed size");
        assert!((text.pos + text.galley.size() / 2. - rect.center()).length() < 1.);
    }
    for (size, key) in [(10., "font-size-decrease"), (24., "font-size-increase")] {
        h.ui.settings.set_font_size(size).unwrap();
        h.click(key);
        assert_eq!(h.ui.settings.values().font_size, size);
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn font_stepper_actions_are_square_at_rest_hover_and_disabled() {
    for size in [10., 14., 24.] {
        let mut h = Harness::new();
        h.size = egui::vec2(1360., 1800.);
        h.click("settings");
        h.ui.settings.set_font_size(size).unwrap();
        h.frame(vec![Event::PointerMoved(egui::pos2(1300., 1700.))]);
        for _ in 0..4 {
            h.frame(vec![]);
        }
        let display = h.ui.probe.widgets["font-size"];
        let header = h.ui.probe.widgets["settings-scroll"];
        for key in ["font-size-decrease", "font-size-increase"] {
            let before = h.ui.probe.widgets[key];
            assert!(before.size() == egui::Vec2::splat(24.), "{key} {before:?}");
            h.frame(vec![Event::PointerMoved(before.center())]);
            for _ in 0..6 {
                h.frame(vec![]);
            }
            let current = h.ui.probe.widgets[key];
            assert!(
                (current.width() - current.height()).abs() < 0.5,
                "hover {key}: {current:?}"
            );
            let expected = 1.;
            assert!((current.width() / before.width() - expected).abs() < 0.02);
            let out = h.frame(vec![]);
            let symbol = if key == "font-size-decrease" {
                "\u{e516}"
            } else {
                "\u{e047}"
            };
            let text = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t)
                        if t.galley.text() == symbol && current.contains(t.pos) =>
                    {
                        Some(t)
                    }
                    _ => None,
                })
                .expect("painted stepper symbol");
            assert_eq!(text.galley.job.sections[0].format.font_id.size, 14.);
            assert!(
                (text.pos + text.galley.mesh_bounds.center().to_vec2() - current.center()).length()
                    < 1.
            );
            assert_eq!(display, h.ui.probe.widgets["font-size"]);
            assert_eq!(header, h.ui.probe.widgets["settings-scroll"]);
            h.frame(vec![Event::PointerMoved(egui::pos2(1300., 1700.))]);
            for _ in 0..6 {
                h.frame(vec![]);
            }
        }
    }
}
