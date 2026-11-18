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
        if self.ui.support.settings_open && !key.starts_with("settings-menu:") {
            let page = if key.starts_with("mcp-") || key == "settings-mcp" {
                Some("mcp")
            } else if key == "check-app-updates" || key == "settings-updates" {
                Some("about")
            } else if key == "theme" || key == "language" || key == "font-size" {
                Some("interface")
            } else {
                None
            };
            if let Some(page) = page
                && !self.ui.probe.widgets.contains_key(key)
            {
                self.click(&format!("settings-menu:{page}"));
            }
        }

        for _ in 0..4 {
            self.frame(vec![]);
        }
        if self.ui.support.settings_open
            && !key.contains(':')
            && key != "settings-close"
            && !key.starts_with("settings-menu:")
        {
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
    h.click("settings-menu:mcp");
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
            h.click("settings-menu:mcp");
            h.size = egui::vec2(360., 300.);
            for _ in 0..5 {
                h.frame(vec![]);
            }
            bottom(&mut h);
            for key in [
                "settings-sidebar",
                "settings-mcp",
                "settings-content",
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
                assert!(
                    r.width() >= 100. && r.height() < 160.,
                    "copy actions must wrap as words, not a column of characters: {key} {r:?}"
                );
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
    h.click("settings-menu:interface");
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
fn mcp_switch_uses_macos_capsules_in_both_themes_and_keeps_save_behavior() {
    for theme in [ThemeMode::Light, ThemeMode::Dark] {
        let mut h = Harness::new();
        h.ui.settings.set_theme_mode(theme).unwrap();
        h.click("settings");
        h.click("settings-menu:mcp");
        for enabled in [false, true, false] {
            if h.ui.settings.values().mcp_enabled != enabled {
                h.click("mcp-enabled");
            }
            for _ in 0..12 {
                h.time += 0.02;
                h.frame(vec![]);
            }
            let output = h.frame(vec![]);
            let rect = h.ui.probe.widgets["mcp-enabled"];
            assert_eq!(rect.size(), egui::vec2(44., 22.));
            let track = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Rect(r)
                        if r.rect == rect && r.corner_radius == egui::CornerRadius::same(11) =>
                    {
                        Some(r)
                    }
                    _ => None,
                })
                .expect("rounded track in production Settings");
            assert!(track.stroke.is_empty());
            let palette = flutter_network_viewer::desktop_style::Palette::of(&h.ctx);
            if enabled {
                assert_eq!(track.fill, palette.accent);
            }
            let thumb = output
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Rect(r)
                        if rect.contains_rect(r.rect)
                            && r.fill == egui::Color32::WHITE
                            && r.rect.size() == egui::vec2(26., 18.) =>
                    {
                        Some(r)
                    }
                    _ => None,
                })
                .expect("white capsule thumb");
            assert_eq!(thumb.corner_radius, egui::CornerRadius::same(9));
            assert!((thumb.rect.center().y - rect.center().y).abs() < 0.1);
            if enabled {
                assert!(thumb.rect.center().x > rect.center().x);
            } else {
                assert!(thumb.rect.center().x < rect.center().x);
            }
            assert_eq!(h.ui.settings.values().mcp_enabled, enabled);
        }
    }
}

#[test]
fn mcp_display_and_input_share_field_style_and_copy_buttons_include_icon_and_label() {
    let mut h = Harness::new();
    h.click("settings");
    h.click("settings-menu:mcp");
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
        h.click("settings-menu:mcp");
        h.ui.settings.set_font_size(size).unwrap();
        for _ in 0..4 {
            h.frame(vec![]);
        }
        h.frame(vec![Event::PointerMoved(egui::pos2(width - 1., 1700.))]);
        h.click("settings-menu:interface");
        let theme = h.ui.probe.widgets["theme"];
        let language = h.ui.probe.widgets["language"];
        assert!((theme.right() - language.right()).abs() < 1.);
        assert!(theme.width() < 220. && language.width() < 220.);
        h.click("settings-menu:mcp");
        let out = h.frame(vec![]);
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
            (field.width()
                - (text.galley.size().x + 24.)
                    .min(h.ui.probe.widgets["settings-content"].width() - 34. - 42.))
            .abs()
                < 1.,
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
fn mcp_endpoint_retains_display_text_style() {
    for size in [14., 24.] {
        let mut h = Harness::new();
        h.size = egui::vec2(1360., 1800.);
        h.click("settings");
        h.click("settings-menu:mcp");
        h.ui.settings.set_font_size(size).unwrap();
        for _ in 0..4 {
            h.frame(vec![]);
        }
        let out = h.frame(vec![Event::PointerMoved(egui::pos2(1300., 1700.))]);
        let (key, value) = ("mcp-endpoint", "http://127.0.0.1:38943/mcp");
        {
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
            assert_eq!(fill, flutter_network_viewer::app_colors::LIGHT.background);
        }
    }
}

#[test]
fn mcp_endpoint_copy_action_copies_only_the_endpoint_and_is_square() {
    let mut h = Harness::new();
    h.click("settings");
    h.click("settings-menu:mcp");
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
        h.click("settings-menu:mcp");
        for _ in 0..5 {
            h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
        }
        h.frame(vec![]);
        for (key, page) in [
            ("settings-appearance", "interface"),
            ("settings-mcp", "mcp"),
            ("settings-updates", "about"),
        ] {
            h.click(&format!("settings-menu:{page}"));
            h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
            let out = h.frame(vec![]);
            let section = h.ui.probe.widgets[key];
            // Measure real painted content, not just the declared Frame margin.
            let content = out
                .shapes
                .iter()
                .filter_map(|shape| {
                    let rect = match &shape.shape {
                        egui::Shape::Text(t) => Rect::from_min_size(t.pos, t.galley.size()),
                        egui::Shape::Rect(r)
                            if r.rect != section
                                && (r.stroke.width > 0.
                                    || h.ui.probe.widgets.get("mcp-enabled") == Some(&r.rect)) =>
                        {
                            r.rect
                        }
                        _ => return None,
                    };
                    section.contains_rect(rect).then_some(rect)
                })
                .fold(Rect::NOTHING, |bounds, rect| bounds.union(rect));
            // Include transparent control rows; their layout also reserves inner padding.
            let content = ["theme", "language", "font-size", "mcp-status-row"]
                .iter()
                .fold(content, |bounds, key| {
                    let Some(rect) = h.ui.probe.widgets.get(*key).copied() else {
                        return bounds;
                    };
                    if section.contains_rect(rect) {
                        bounds.union(rect)
                    } else {
                        bounds
                    }
                });
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
fn settings_sidebar_icons_use_eighty_percent_of_centered_square_tiles() {
    for size in [14., 24.] {
        let mut h = Harness::new();
        h.size = egui::vec2(1200., 900.);
        h.ui.settings.set_font_size(size).unwrap();
        h.click("settings");
        let out = h.frame(vec![]);
        for page in ["interface", "mcp", "about"] {
            let tile = h.ui.probe.widgets[&format!("settings-menu:{page}-tile")];
            let icon = h.ui.probe.widgets[&format!("settings-menu:{page}-icon")];
            assert_eq!(tile.width(), tile.height());
            assert_eq!(tile.center(), icon.center());
            assert!((icon.width() / tile.width() - 0.8).abs() < 0.001);
            let painted = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t)
                        if t.galley.job.sections[0].format.font_id.family
                            == egui::FontFamily::Name("MaterialIcons".into())
                            && tile.contains(t.pos + t.galley.mesh_bounds.center().to_vec2()) =>
                    {
                        Some(t)
                    }
                    _ => None,
                })
                .unwrap();
            assert!(
                (painted.galley.job.sections[0].format.font_id.size - icon.height()).abs() < 0.01
            );
            assert!(
                (painted.pos + painted.galley.mesh_bounds.center().to_vec2() - tile.center())
                    .length()
                    < 0.6
            );
        }
    }
}
