//! Measured contracts from the original Flutter shell, before Rust UI changes.
use eframe::egui::{self, Color32, FontId, Pos2, Rect, Vec2};
use flutter_network_viewer::{
    capture::CaptureStore,
    desktop::{DesktopUi, ShellStatus},
    settings::{AppSettings, ThemeMode},
};
fn render(width: f32, dark: bool) -> (egui::Context, DesktopUi) {
    let ctx = egui::Context::default();
    let mut settings = AppSettings::default();
    settings
        .set_theme_mode(if dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        })
        .unwrap();
    let mut ui = DesktopUi::new(settings);
    for _ in 0..3 {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 808.))),
                ..Default::default()
            },
            |ctx| {
                ui.show(ctx, &CaptureStore::default(), &ShellStatus::default());
            },
        );
    }
    (ctx, ui)
}
#[test]
fn ventura_palette_and_cascadia_typography_are_used_in_both_themes() {
    for (dark, background, line) in [
        (
            false,
            Color32::from_rgb(236, 236, 236),
            Color32::from_rgb(210, 210, 215),
        ),
        (
            true,
            Color32::from_rgb(40, 40, 40),
            Color32::from_rgb(72, 72, 74),
        ),
    ] {
        let (ctx, _) = render(1360., dark);
        let style = ctx.style();
        assert_eq!(style.visuals.panel_fill, background);
        assert_eq!(style.visuals.widgets.noninteractive.bg_stroke.color, line);
        assert_eq!(
            style.text_styles[&egui::TextStyle::Body],
            FontId::proportional(14.)
        );
    }
}
#[test]
fn original_header_horizontal_pills_filters_footer_and_empty_split_geometry() {
    let (ctx, ui) = render(1360., false);
    let p = &ui.probe;
    assert_eq!(
        p.sidebar_width, 0.,
        "Original Flutter has no left navigation sidebar"
    );
    assert!((p.widgets["header"].height() - 32.).abs() < 1.);
    assert!(p.widgets["header"].contains_rect(p.widgets["app:all"]));
    let nav = p.widgets["nav-toolbar"];
    for key in ["pause", "clear", "export"] {
        assert!(nav.contains_rect(p.widgets[key]), "{key}");
    }
    assert!(!p.widgets.contains_key("filters"));
    assert_eq!(nav.top(), p.widgets["header"].bottom());
    assert_eq!(
        nav.height(),
        flutter_network_viewer::desktop_style::search_bar_height(&ctx) + 8.
    );
    assert!(nav.contains_rect(p.widgets["search-frame"]));
    let right = p.widgets["empty-inspector"];
    assert!((right.left() - 684.).abs() < 6., "{right:?}");
    assert!(p.list_rect.left() < 2.);
    assert_eq!(p.widgets["footer"].height(), 26.);
}
#[test]
fn split_breakpoint_is_900_and_connect_remains_in_header() {
    for (width, split) in [(899., false), (900., true), (1360., true)] {
        let (_, ui) = render(width, false);
        assert_eq!(
            ui.probe.widgets.contains_key("empty-inspector"),
            split,
            "width={width}"
        );
        assert!(!ui.probe.widgets.contains_key("empty-connect"));
        assert!(ui.probe.widgets.contains_key("manual-url"));
    }
}

#[test]
fn connection_error_is_popup_while_paused_status_precedes_navigation() {
    let ctx = egui::Context::default();
    let mut ui = DesktopUi::default();
    let mut store = CaptureStore::default();
    store.paused = true;
    let state = ShellStatus {
        error: Some("Connection refused".into()),
        ..Default::default()
    };
    for _ in 0..3 {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1360., 808.))),
                ..Default::default()
            },
            |ctx| {
                ui.show(ctx, &store, &state);
            },
        );
    }
    let p = &ui.probe.widgets;
    assert!(!p.contains_key("error-banner"));
    assert!(p.contains_key("notification"));
    assert!(p["paused-banner"].top() >= p["header"].bottom());
    assert!(p["nav-toolbar"].top() >= p["paused-banner"].bottom());
}

#[test]
fn every_page_paints_an_opaque_background_in_both_themes() {
    // Native GL clears to black. Transparent frames must never expose it in
    // blank areas of Network, Console or either half of the JSON workspace.
    for dark in [false, true] {
        for width in [700., 1360.] {
            let (ctx, mut ui) = render(width, dark);
            let store = CaptureStore::default();
            let state = ShellStatus::default();
            let frame = |ui: &mut DesktopUi, events| {
                ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 808.))),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        ui.show(ctx, &store, &state);
                    },
                )
            };
            for page in ["Network", "Console", "JSON"] {
                let pos = ui.probe.widgets[page].center();
                frame(
                    &mut ui,
                    vec![
                        egui::Event::PointerMoved(pos),
                        egui::Event::PointerButton {
                            pos,
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
                frame(
                    &mut ui,
                    vec![egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                );
                let output = frame(&mut ui, vec![]);
                for x in [width * 0.25, width * 0.75] {
                    let point = egui::pos2(x, 750.);
                    assert!(output.shapes.iter().any(|s| matches!(&s.shape,egui::Shape::Rect(r) if r.fill==ctx.style().visuals.panel_fill && r.rect.contains(point) && s.clip_rect.contains(point))),"Unpainted background: page={page}, dark={dark}, width={width}, point={point:?}");
                }
            }
        }
    }
}

#[test]
fn viewport_background_covers_panel_gaps_and_outer_edges() {
    for dark in [false, true] {
        let (ctx, mut ui) = render(1360., dark);
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1360., 808.))),
                ..Default::default()
            },
            |ctx| {
                ui.show(ctx, &CaptureStore::default(), &ShellStatus::default());
            },
        );
        let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1360., 808.));
        assert!(output.shapes.iter().any(|s| matches!(&s.shape,egui::Shape::Rect(r) if r.rect.contains_rect(screen) && r.fill==ctx.style().visuals.panel_fill)),"Panel gaps have no full-window background, dark={dark}");
    }
}

#[test]
fn header_does_not_render_brand_logo_or_title() {
    for width in [480., 1360.] {
        let (ctx, mut ui) = render(width, false);
        let out = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 808.))),
                ..Default::default()
            },
            |ctx| {
                ui.show(ctx, &CaptureStore::default(), &ShellStatus::default());
            },
        );
        let header = ui.probe.widgets["header"];
        assert!(!out.shapes.iter().any(|s| matches!(&s.shape,
            egui::Shape::Text(t) if header.contains(t.pos)
                && (t.galley.text() == "Flutter Network Viewer" || t.galley.text() == "\u{f0616}"))));
    }
}

#[test]
fn header_rows_have_eight_by_four_outer_padding() {
    for width in [480., 1360.] {
        let (_, ui) = render(width, false);
        for (outer, inner) in [("header", "header-content"), ("nav-toolbar", "nav-content")] {
            let a = ui.probe.widgets[outer];
            let b = ui.probe.widgets[inner];
            assert_eq!(
                b.min - a.min,
                egui::vec2(8., if outer == "header" { 2. } else { 4. }),
                "{outer} top-left at {width}"
            );
            assert_eq!(
                a.max - b.max,
                egui::vec2(8., if outer == "header" { 2. } else { 4. }),
                "{outer} bottom-right at {width}"
            );
        }
        let search = ui.probe.widgets["search-frame"];
        assert!((search.width() - 350.).abs() < 1.);
        assert!(ui.probe.widgets["nav-toolbar"].contains_rect(search));
        assert!(!ui.probe.widgets.contains_key("filters"));
    }
}
