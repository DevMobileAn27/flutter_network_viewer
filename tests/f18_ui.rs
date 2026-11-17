use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, Rect};
use flutter_network_viewer::{
    capture::CaptureStore,
    desktop::{DesktopUi, ShellStatus},
    settings::ThemeMode,
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
        if self.ui.support.settings_open && !key.contains(':') && key != "settings-close" {
            for _ in 0..20 {
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
                for _ in 0..8 {
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

fn bottom(h: &mut Harness) {
    let p = h.ui.probe.widgets["settings-scroll"].center();
    h.frame(vec![
        Event::PointerMoved(p),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0., -10000.),
            modifiers: Modifiers::NONE,
        },
    ]);
    for _ in 0..30 {
        h.time += 0.02;
        h.frame(vec![]);
    }
}
fn replace(h: &mut Harness, key: &str, text: &str) {
    h.click(key);
    h.frame(vec![
        Event::Key {
            key: egui::Key::A,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers {
                ctrl: true,
                command: true,
                ..Default::default()
            },
        },
        Event::Text(text.into()),
    ]);
    h.frame(vec![]);
}
fn enter(h: &mut Harness) {
    h.frame(vec![Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    h.frame(vec![]);
}

#[test]
fn mcp_inputs_validate_persist_retry_and_copy() {
    use flutter_network_viewer::{desktop::Action, settings::AppSettings};
    let path = std::env::temp_dir().join(format!("fnv-mcp-ui-{}.json", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut h = Harness::new();
    h.ui.settings = AppSettings::load(&path).unwrap();
    h.click("settings");
    bottom(&mut h);
    for removed in ["mcp-save", "mcp-client-menu", "mcp-copy-setup"] {
        assert!(
            !h.ui.probe.widgets.contains_key(removed),
            "obsolete control {removed}"
        );
    }
    assert!(!h.ui.settings.values().mcp_enabled);
    replace(&mut h, "mcp-port", "0");
    h.click("mcp-enabled");
    assert_eq!(h.ui.settings.values().mcp_port, 38943);
    assert!(!h.ui.settings.values().mcp_enabled);
    assert!(h.actions.is_empty());
    assert_eq!(h.ui.probe.notification_title, "Error");
    h.ui.notifications.dismiss(false);

    replace(&mut h, "mcp-port", "40123");
    h.click("mcp-enabled");
    let saved = AppSettings::load(&path).unwrap();
    assert!(saved.values().mcp_enabled);
    assert_eq!(saved.values().mcp_port, 40123);
    assert_eq!(std::mem::take(&mut h.actions), vec![Action::RetryMcp]);

    replace(&mut h, "mcp-port", "40124");
    enter(&mut h);
    assert_eq!(AppSettings::load(&path).unwrap().values().mcp_port, 40124);
    assert_eq!(std::mem::take(&mut h.actions), vec![Action::RetryMcp]);
    h.ui.notifications.dismiss(false);
    h.ui.support.mcp_status = flutter_network_viewer::mcp::McpStatus {
        port: 40124,
        error: Some("address occupied".into()),
        running: false,
    };
    h.frame(vec![]);
    bottom(&mut h);
    assert!(!h.ui.probe.support_text.contains("address occupied"));
    assert_eq!(h.ui.probe.notification_title, "Error");
    let output = h.frame(vec![]);
    let popup = h.ui.probe.widgets["notification"];
    let messages: Vec<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == "address occupied" => Some(t),
            _ => None,
        })
        .collect();
    assert_eq!(messages.len(), 1);
    assert!(popup.contains(messages[0].pos));
    h.ui.notifications.dismiss(false);
    h.frame(vec![]);
    assert!(!h.ui.probe.widgets.contains_key("notification"));
    h.click("mcp-retry");
    assert_eq!(std::mem::take(&mut h.actions), vec![Action::RetryMcp]);
    for (key, claude) in [("mcp-copy-codex", false), ("mcp-copy-claude", true)] {
        let o = h.click(key);
        let expected = flutter_network_viewer::mcp::setup(40124, claude);
        assert!(
            o.platform_output
                .commands
                .iter()
                .any(|c| matches!(c,egui::OutputCommand::CopyText(t) if *t == expected))
        );
        h.ui.notifications.dismiss(false);
    }
    replace(&mut h, "mcp-port", "0");
    h.click("mcp-enabled");
    let saved = AppSettings::load(&path).unwrap();
    assert!(!saved.values().mcp_enabled);
    assert_eq!(saved.values().mcp_port, 40124);
    assert_eq!(std::mem::take(&mut h.actions), vec![Action::RetryMcp]);
    let _ = std::fs::remove_file(path);
}
#[test]
fn mcp_narrow_locales_themes_scroll() {
    for lang in ["en", "vi"] {
        for dark in [false, true] {
            let mut h = Harness::new();
            h.ui.settings.set_language(lang).unwrap();
            h.ui.settings
                .set_theme_mode(if dark {
                    ThemeMode::Dark
                } else {
                    ThemeMode::Light
                })
                .unwrap();
            h.click("settings");
            h.size = egui::vec2(360., 300.);
            for _ in 0..5 {
                h.frame(vec![]);
            }
            bottom(&mut h);
            for key in [
                "settings-appearance",
                "settings-mcp",
                "settings-updates",
                "mcp-enabled",
            ] {
                let r = h.ui.probe.widgets[key];
                assert!(
                    r.left() >= 0. && r.right() <= h.size.x,
                    "{lang}/{dark} {key}: {r:?}"
                );
            }
            assert!(h.ui.probe.support_scroll > 0.);
            for key in ["mcp-copy-codex", "mcp-copy-claude"] {
                let r = h.ui.probe.widgets[key];
                assert!(r.left() >= 0. && r.right() <= 360., "{key}: {r:?}");
                let o = h.click(key);
                assert!(o.platform_output.commands.iter().any(
                    |c| matches!(c,egui::OutputCommand::CopyText(t) if t.contains(":38943/mcp"))
                ));
                h.ui.notifications.dismiss(false);
            }
            assert!(h.ui.probe.support_text.contains(if lang == "vi" {
                "Tắt MCP"
            } else {
                "MCP disabled"
            }));
            assert_eq!(h.ctx.style().visuals.dark_mode, dark);
        }
    }
}
#[test]
fn opening_settings_keeps_language_controls_visible_and_clickable() {
    let mut h = Harness::new();
    h.click("settings");
    let r = h.ui.probe.widgets["language"];
    assert!(
        Rect::from_min_size(Pos2::ZERO, h.size).contains_rect(r),
        "language={r:?} scroll={} clip={:?}",
        h.ui.probe.support_scroll,
        h.ui.probe.widgets["settings-scroll"]
    );
    h.click("language");
    assert!(h.ui.probe.widgets.contains_key("language:vi"));
}

#[test]
fn mcp_display_and_input_share_field_style_and_copy_buttons_include_icon_and_label() {
    let mut h = Harness::new();
    h.click("settings");
    h.size = egui::vec2(1200., 1800.);
    for size in [14., 24.] {
        h.ui.settings.set_font_size(size).unwrap();
        for _ in 0..4 {
            h.frame(vec![]);
        }
        let out = h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
        let port = h.ui.probe.widgets["mcp-port"];
        let endpoint = h.ui.probe.widgets["mcp-endpoint"];
        assert!((endpoint.height() - port.height()).abs() <= 0.5);
        let get_fill = |field: Rect| {
            out.shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Rect(r)
                        if (r.rect.min - field.min).length() < 2.
                            && (r.rect.max - field.max).length() < 2.
                            && r.fill != egui::Color32::TRANSPARENT =>
                    {
                        Some(r.fill)
                    }
                    _ => None,
                })
                .expect("field background")
        };
        assert_ne!(get_fill(port), get_fill(endpoint));
        for (rect, value) in [(port, "38943"), (endpoint, "http://127.0.0.1:38943/mcp")] {
            let text = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == value => Some(t),
                    _ => None,
                })
                .expect("painted field text");
            assert!((text.pos.x - rect.left() - 12.).abs() < 0.5);
            assert!((text.pos.y - rect.top() - 12.).abs() < 0.5);
            assert!(
                text.galley
                    .job
                    .sections
                    .iter()
                    .all(|s| (s.format.font_id.size - size).abs() < 0.1)
            );
        }
        for (key, label) in [
            ("mcp-copy-codex", "Copy manual MCP for Codex"),
            ("mcp-copy-claude", "Copy manual MCP for Claude"),
        ] {
            let r = h.ui.probe.widgets[key];
            assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if r.contains(t.pos) && t.galley.text().contains(label) && t.galley.text().contains('\u{ef7f}'))));
        }
    }
}

#[test]
fn settings_dropdown_widths_are_bounded_and_endpoint_hugs_its_text() {
    for (width, size, lang) in [
        (1360., 14., "en"),
        (1360., 24., "vi"),
        (360., 14., "vi"),
        (360., 24., "en"),
    ] {
        let mut h = Harness::new();
        h.size = egui::vec2(width, 1800.);
        h.ui.settings.set_language(lang).unwrap();
        h.click("settings");
        h.ui.settings.set_font_size(size).unwrap();
        for _ in 0..4 {
            h.frame(vec![]);
        }
        let out = h.frame(vec![Event::PointerMoved(egui::pos2(width - 1., 1700.))]);
        let theme = h.ui.probe.widgets["theme"];
        let language = h.ui.probe.widgets["language"];
        assert!((theme.width() - language.width()).abs() < 1.);
        assert!((theme.width() - 220.).abs() < 1., "{theme:?}");
        let field = h.ui.probe.widgets["mcp-endpoint"];
        let text = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == "http://127.0.0.1:38943/mcp" => Some(t),
                _ => None,
            })
            .unwrap();
        assert!(
            (field.width() - (text.galley.size().x + 24.).min(width - 34. - 42.)).abs() < 1.,
            "field={field:?} text={:?}",
            text.galley.size()
        );
        assert!(field.right() <= width - 8.);
        let copy = h.ui.probe.widgets["mcp-endpoint-copy"];
        assert!(copy.right() <= width - 8., "{copy:?}");
        h.click("theme");
        h.click("theme:dark");
        assert!(h.ui.settings.values().theme_mode == ThemeMode::Dark);
        h.ui.notifications.dismiss(false);
        h.click("language");
        h.click(if lang == "en" {
            "language:vi"
        } else {
            "language:en"
        });
        assert_ne!(h.ui.settings.values().language_code, lang);
    }
}

#[test]
fn font_size_and_mcp_endpoint_use_matching_display_text_style() {
    for size in [14., 24.] {
        let mut h = Harness::new();
        h.size = egui::vec2(1360., 1800.);
        h.click("settings");
        h.ui.settings.set_font_size(size).unwrap();
        for _ in 0..4 {
            h.frame(vec![]);
        }
        let out = h.frame(vec![Event::PointerMoved(egui::pos2(1300., 1700.))]);
        let mut styles = Vec::new();
        for (key, value) in [
            ("font-size", format!("{size:.0}")),
            ("mcp-endpoint", "http://127.0.0.1:38943/mcp".into()),
        ] {
            let rect = *h.ui.probe.widgets.get(key).unwrap_or_else(|| {
                panic!(
                    "missing {key} size={size} keys={:?}",
                    h.ui.probe.widgets.keys()
                )
            });
            let fill = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Rect(r)
                        if r.rect == rect && r.fill != egui::Color32::TRANSPARENT =>
                    {
                        Some(r.fill)
                    }
                    _ => None,
                })
                .unwrap();
            let text = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == value && rect.contains(t.pos) => {
                        Some(t)
                    }
                    _ => None,
                })
                .unwrap();
            assert!((rect.width() - text.galley.size().x - 24.).abs() < 1.);
            assert!((text.pos.y - rect.top() - 12.).abs() < 1.);
            styles.push((rect.height(), fill));
        }
        assert_eq!(styles[0], styles[1]);
    }
}

#[test]
fn mcp_endpoint_copy_action_copies_only_the_endpoint_and_is_square() {
    let mut h = Harness::new();
    h.click("settings");
    let out = h.click("mcp-endpoint-copy");
    assert!(out.platform_output.commands.iter().any(
        |c| matches!(c,egui::OutputCommand::CopyText(t) if t == "http://127.0.0.1:38943/mcp")
    ));
    let field = h.ui.probe.widgets["mcp-endpoint"];
    let button = h.ui.probe.widgets["mcp-endpoint-copy"];
    assert!((button.width() - button.height()).abs() < 0.5);
    assert!(button.left() >= field.right() && button.left() - field.right() <= 12.);
    assert!((button.center().y - field.center().y).abs() < 1.);
}

#[test]
fn settings_sections_have_equal_top_and_bottom_content_padding() {
    for (width, size) in [(360., 14.), (1200., 14.), (1200., 24.)] {
        let mut h = Harness::new();
        h.size = egui::vec2(width, 2000.);
        h.ui.settings.set_font_size(size).unwrap();
        h.click("settings");
        for _ in 0..5 {
            h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
        }
        let out = h.frame(vec![]);
        for key in ["settings-appearance", "settings-mcp", "settings-updates"] {
            let section = h.ui.probe.widgets[key];
            // Measure real painted content, not just the declared Frame margin.
            let content = out
                .shapes
                .iter()
                .filter_map(|shape| {
                    let rect = match &shape.shape {
                        egui::Shape::Text(t) => Rect::from_min_size(t.pos, t.galley.size()),
                        egui::Shape::Rect(r) if r.stroke.width > 0. && r.rect != section => r.rect,
                        _ => return None,
                    };
                    section.contains_rect(rect).then_some(rect)
                })
                .fold(Rect::NOTHING, |bounds, rect| bounds.union(rect));
            let top = content.top() - section.top();
            let bottom = section.bottom() - content.bottom();
            assert!(
                (top - 9.).abs() < 0.6 && (bottom - 9.).abs() < 0.6,
                "{key} at width {width}, font {size}: top={top}, bottom={bottom}"
            );
        }
    }
}

#[test]
fn settings_heading_icons_match_title_height_and_vertical_center() {
    for size in [14., 24.] {
        let mut h = Harness::new();
        h.size = egui::vec2(1200., 2000.);
        h.ui.settings.set_font_size(size).unwrap();
        h.click("settings");
        for _ in 0..4 {
            h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
        }
        let out = h.frame(vec![]);
        for (key, title, code) in [
            ("settings-appearance", "Interface", '\u{e683}'),
            ("settings-mcp", "MCP", '\u{f0616}'),
            ("settings-updates", "App updates", '\u{e637}'),
        ] {
            let section = h.ui.probe.widgets[key];
            let text = |value: &str| {
                out.shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Text(t)
                            if section.contains(t.pos) && t.galley.text() == value =>
                        {
                            Some(t)
                        }
                        _ => None,
                    })
                    .unwrap()
            };
            let title = text(title);
            let icon = text(&code.to_string());
            assert!(
                (icon.galley.size().y - title.galley.size().y).abs() < 0.6,
                "{key} at {size}: icon height {}, title height {}",
                icon.galley.size().y,
                title.galley.size().y
            );
            let icon_y = icon.pos.y + icon.galley.size().y / 2.;
            let title_y = title.pos.y + title.galley.size().y / 2.;
            assert!(
                (icon_y - title_y).abs() < 0.6,
                "{key}: icon/title vertical centers differ"
            );
        }
    }
}
