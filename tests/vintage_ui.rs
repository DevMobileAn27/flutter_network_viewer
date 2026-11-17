//! Drive the review-only vintage gallery with production egui input.
use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, Rect};
use flutter_network_viewer::{
    capture::CaptureStore,
    desktop::{DesktopUi, ShellStatus},
};
struct Harness {
    ctx: egui::Context,
    ui: DesktopUi,
    time: f64,
    dpi: f32,
}
impl Harness {
    fn new() -> Self {
        let mut ui = DesktopUi::default();
        ui.vintage_open = true;
        let mut h = Self {
            ctx: Default::default(),
            ui,
            time: 0.,
            dpi: 1.,
        };
        for _ in 0..3 {
            h.frame(vec![]);
        }
        h
    }
    fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.time += 0.02;
        self.ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, egui::vec2(1360., 860.))),
                time: Some(self.time),
                viewports: std::iter::once((
                    egui::ViewportId::ROOT,
                    egui::ViewportInfo {
                        native_pixels_per_point: Some(self.dpi),
                        ..Default::default()
                    },
                ))
                .collect(),
                events,
                ..Default::default()
            },
            |ctx| {
                self.ui
                    .show(ctx, &CaptureStore::default(), &ShellStatus::default());
            },
        )
    }
    fn reveal(&mut self, key: &str) {
        self.frame(vec![]);
        for _ in 0..24 {
            let viewport = self.ui.probe.widgets["vintage-scroll"];
            let target = self.ui.probe.widgets[key];
            if viewport.contains(target.center())
                || key.starts_with("vintage-nav:")
                || key.starts_with("vintage-dialog")
                || key.starts_with("vintage-toast")
                || key == "vintage-legacy"
            {
                break;
            }
            self.frame(vec![
                Event::PointerMoved(viewport.center()),
                Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(
                        0.,
                        (viewport.center().y - target.center().y).clamp(-300., 300.),
                    ),
                    modifiers: Modifiers::NONE,
                },
            ]);
            for _ in 0..8 {
                self.frame(vec![]);
            }
        }
    }
    fn click(&mut self, key: &str) {
        self.reveal(key);
        let pos = self.ui.probe.widgets[key].center();
        self.frame(vec![Event::PointerMoved(pos)]);
        for pressed in [true, false] {
            self.frame(vec![Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            }]);
        }
        self.frame(vec![Event::PointerMoved(egui::pos2(0., 0.))]);
        self.frame(vec![]);
    }
}
#[test]
fn gallery_has_square_widgets_shared_font_and_complete_inventory() {
    use flutter_network_viewer::vintage::{CATEGORIES, INVENTORY};
    let mut h = Harness::new();
    assert!(INVENTORY.len() >= 30);
    for size in [14., 20.] {
        h.ui.settings.set_font_size(size).unwrap();
        for category in CATEGORIES {
            h.click(&format!("vintage-nav:{category}"));
            let output = h.frame(vec![]);
            for clipped in &output.shapes {
                match &clipped.shape {
                    egui::Shape::Rect(r) => assert_eq!(r.corner_radius, egui::CornerRadius::ZERO),
                    egui::Shape::Text(t) => {
                        for section in &t.galley.job.sections {
                            if section.format.font_id.family
                                != egui::FontFamily::Name("MaterialIcons".into())
                            {
                                assert_eq!(
                                    section.format.font_id.size,
                                    size,
                                    "{}",
                                    t.galley.text()
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    for item in INVENTORY {
        assert!(!item.source.is_empty() && !item.example.is_empty());
    }
    h.click("vintage-legacy");
    assert!(!h.ui.vintage_open);
    assert!(h.ui.probe.widgets.contains_key("Network"));
}
#[test]
fn gallery_controls_modals_and_json_are_interactive() {
    let mut h = Harness::new();
    h.click("vintage-nav:Controls");
    h.click("vintage-font-plus");
    assert_eq!(h.ui.settings.values().font_size, 15.);
    h.click("vintage-font-minus");
    assert_eq!(h.ui.settings.values().font_size, 14.);
    h.click("vintage-switch");
    assert!(h.ui.vintage.enabled);
    h.click("vintage-nav:JSON");
    let output = h.frame(vec![]);
    let colors: std::collections::HashSet<_> = output
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.job.sections.len() > 1 => Some(t),
            _ => None,
        })
        .flat_map(|t| t.galley.job.sections.iter().map(|s| s.format.color))
        .collect();
    assert_eq!(colors.len(), 7, "preserve seven existing JSON syntax roles");
    h.click("vintage-json-fold");
    assert!(h.ui.vintage.folded);
    h.click("vintage-json-copy");
    h.click("vintage-nav:Feedback");
    h.click("vintage-dialog-open");
    assert!(h.ui.probe.widgets.contains_key("vintage-dialog"));
    h.click("vintage-dialog-cancel");
    assert!(!h.ui.probe.widgets.contains_key("vintage-dialog"));
    h.click("vintage-toast-open");
    assert!(h.ui.probe.widgets.contains_key("vintage-toast"));
    h.click("vintage-toast-close");
    assert!(!h.ui.probe.widgets.contains_key("vintage-toast"));
}
#[test]
fn native_host_starts_in_approved_production_app() {
    assert!(!DesktopUi::default().vintage_open);
    let path = std::env::temp_dir().join(format!("vintage-host-{}", std::process::id()));
    let app = flutter_network_viewer::desktop_host::DesktopApp::with_diagnostics(
        Default::default(),
        path.clone(),
    );
    assert!(!app.ui.vintage_open);
    drop(app);
    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn inventory_examples_exist_and_narrow_gallery_scrolls_without_hiding_navigation() {
    use flutter_network_viewer::vintage::{CATEGORIES, INVENTORY};
    let mut h = Harness::new();
    let mut seen = std::collections::HashSet::new();
    for category in CATEGORIES {
        h.click(&format!("vintage-nav:{category}"));
        seen.extend(h.ui.probe.widgets.keys().cloned());
    }
    h.click("vintage-nav:Bodies");
    h.click("vintage-body:Image");
    seen.extend(h.ui.probe.widgets.keys().cloned());
    h.click("vintage-image-open");
    assert!(h.ui.probe.widgets.contains_key("vintage-image-close"));
    h.click("vintage-image-close");
    h.click("vintage-nav:Feedback");
    h.click("vintage-dialog-open");
    seen.extend(h.ui.probe.widgets.keys().cloned());
    h.click("vintage-dialog-cancel");
    for item in INVENTORY {
        assert!(
            seen.contains(item.example),
            "missing real example {} / {}",
            item.name,
            item.example
        );
    }
    for size in [egui::vec2(480., 360.), egui::vec2(1360., 860.)] {
        let bounds = Rect::from_min_size(Pos2::ZERO, size);
        for category in CATEGORIES {
            h.ui.vintage.section = CATEGORIES.iter().position(|c| *c == category).unwrap();
            for _ in 0..3 {
                let _ = h.ctx.run(
                    egui::RawInput {
                        screen_rect: Some(bounds),
                        ..Default::default()
                    },
                    |ctx| {
                        h.ui.show(ctx, &CaptureStore::default(), &ShellStatus::default());
                    },
                );
            }
            for key in ["vintage-legacy", "vintage-nav:Feedback"] {
                assert!(
                    bounds.contains_rect(h.ui.probe.widgets[key]),
                    "{category} {size:?} {key}"
                );
            }
        }
    }
}

fn has_rect(
    out: &egui::FullOutput,
    rect: Rect,
    fill: egui::Color32,
    stroke: Option<egui::Color32>,
) -> bool {
    out.shapes.iter().any(|s| matches!(&s.shape,egui::Shape::Rect(r) if (r.rect.min-rect.min).length()<0.6 && (r.rect.max-rect.max).length()<0.6 && r.fill==fill && stroke.is_none_or(|c| r.stroke.color==c && r.stroke.width>=1.)))
}
#[test]
fn selected_popup_options_use_paper_text_and_focused_inputs_keep_ink_borders() {
    use flutter_network_viewer::app_colors::LIGHT;
    let background = LIGHT.on_accent;
    let mut h = Harness::new();
    h.click("vintage-nav:Tables");
    h.click("vintage-table-filter");
    for _ in 0..12 {
        h.frame(vec![]);
    }
    let out = h.frame(vec![]);
    let selected = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t)
                if t.galley.text() == "All methods"
                    && t.pos.y > h.ui.probe.widgets["vintage-table-filter"].bottom() =>
            {
                Some(t)
            }
            _ => None,
        })
        .unwrap();
    assert!(
        selected
            .galley
            .job
            .sections
            .iter()
            .all(|s| selected.override_text_color.unwrap_or(
                if s.format.color == egui::Color32::PLACEHOLDER {
                    selected.fallback_color
                } else {
                    s.format.color
                }
            ) == background)
    );
    h.frame(vec![Event::Key {
        key: egui::Key::Escape,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }]);
    h.click("vintage-nav:Controls");
    for key in ["vintage-input", "vintage-search", "vintage-multiline"] {
        h.reveal(key);
        let out = h.frame(vec![]);
        let rect = h.ui.probe.widgets[key];
        assert!(
            out.shapes
                .iter()
                .any(|s| matches!(&s.shape, egui::Shape::Rect(r)
            if r.rect == rect && s.clip_rect.contains_rect(rect)
                && r.stroke.width >= 1. && r.stroke.color == LIGHT.line)),
            "missing idle border {key}"
        );
        h.click(key);
        let out = h.frame(vec![]);
        let rect = h.ui.probe.widgets[key];
        assert!(out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Rect(r) if (r.rect.min-rect.min).length()<1. && (r.rect.max-rect.max).length()<1. && r.stroke.width>=1. && r.stroke.color==LIGHT.accent && s.clip_rect.contains_rect(rect))),"missing focused border {key}");
    }
}
#[test]
fn notice_icon_has_ink_tile_paper_glyph_and_json_has_paper_background() {
    use flutter_network_viewer::app_colors::LIGHT;
    let background = LIGHT.on_accent;
    let primary = LIGHT.accent;
    let mut h = Harness::new();
    h.click("vintage-nav:Feedback");
    for key in ["vintage-toast-open", "vintage-dialog-open"] {
        h.click(key);
        for _ in 0..12 {
            h.frame(vec![]);
        }
        let out = h.frame(vec![]);
        let tile = h.ui.probe.widgets["vintage-notice-icon"];
        assert!(has_rect(&out, tile, primary, None));
        assert!(out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if tile.contains(t.pos) && t.galley.job.sections.iter().all(|s|s.format.color==background))));
        h.click(if key == "vintage-toast-open" {
            "vintage-toast-close"
        } else {
            "vintage-dialog-cancel"
        });
    }
    h.click("vintage-nav:JSON");
    let out = h.frame(vec![]);
    assert!(has_rect(
        &out,
        h.ui.probe.widgets["vintage-json-surface"],
        flutter_network_viewer::vintage::BACKGROUND,
        None
    ));
}
#[test]
fn vintage_table_grid_is_contiguous_and_has_outer_edges() {
    let mut h = Harness::new();
    h.click("vintage-nav:Tables");
    let out = h.frame(vec![]);
    let header = h.ui.probe.widgets["vintage-table-header"];
    assert!(has_rect(
        &out,
        h.ui.probe.widgets["vintage-table-grid"],
        egui::Color32::TRANSPARENT,
        Some(flutter_network_viewer::vintage::LINE)
    ));
    let first = h.ui.probe.widgets["vintage-row:0"];
    let second = h.ui.probe.widgets["vintage-row:1"];
    assert!((header.bottom() - first.top()).abs() < 0.1);
    assert!((first.bottom() - second.top()).abs() < 0.1);
    for rect in [header, first, second] {
        for x in [rect.left(), rect.right()] {
            assert!(out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::LineSegment{points,..} if (points[0].x-x).abs()<0.6 && (points[1].x-x).abs()<0.6 && points[0].y<=rect.top()+0.6 && points[1].y>=rect.bottom()-0.6)),"missing outside edge at {x}");
        }
    }
}
#[test]
fn square_loader_cycles_one_inset_cell_for_three_to_ten_slots() {
    use flutter_network_viewer::vintage::loading;
    let primary = flutter_network_viewer::app_colors::DARK.accent;
    let ctx = egui::Context::default();
    for count in 3..=10 {
        for step in 0..=count * 2 {
            let mut rects = vec![];
            let out = ctx.run(
                egui::RawInput {
                    time: Some(step as f64 * 0.2),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        rects = loading(ui, count).cells;
                    });
                },
            );
            assert_eq!(rects.len(), count);
            let active = out
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Rect(r)
                        if r.fill == primary
                            && rects.iter().any(|cell| *cell == r.rect.expand(3.)) =>
                    {
                        Some(r.rect)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(active.len(), 1, "only one active square");
            assert_eq!(active[0], rects[step % count].shrink(3.));
            for rect in &rects {
                assert!(has_rect(
                    &out,
                    *rect,
                    flutter_network_viewer::app_colors::DARK.background,
                    Some(primary)
                ));
            }
        }
    }
}

#[test]
fn metadata_and_form_use_aligned_grids_with_optional_copy_action_column() {
    let mut h = Harness::new();
    h.click("vintage-nav:Tables");
    let p = &h.ui.probe.widgets;
    let header = p["vintage-metadata-table:header"];
    let first = p["vintage-metadata-table:row:0"];
    assert!((header.bottom() - first.top()).abs() < 0.1);
    assert!(
        header.right() <= p["vintage-scroll"].right(),
        "header={header:?} viewport={:?}",
        p["vintage-scroll"]
    );
    let title = p["vintage-metadata-table:cell:0:0"];
    let value = p["vintage-metadata-table:cell:0:1"];
    assert_eq!(title.right(), value.left());
    assert_eq!(value.right(), first.right());
    assert!(!p.contains_key("vintage-metadata-table:cell:0:2"));
    for i in 1..5 {
        assert_eq!(
            p[&format!("vintage-metadata-table:cell:{i}:0")].right(),
            title.right()
        );
        assert_eq!(
            p[&format!("vintage-metadata-table:row:{}", i - 1)].bottom(),
            p[&format!("vintage-metadata-table:row:{i}")].top()
        );
    }
    h.click("vintage-nav:Bodies");
    h.click("vintage-form-table:copy:0");
    // Click through production egui; verify the system clipboard command carries the raw Unicode value.
    let pos = h.ui.probe.widgets["vintage-form-table:copy:0"].center();
    h.frame(vec![
        Event::PointerMoved(pos),
        Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        },
    ]);
    let out = h.frame(vec![Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    assert!(
        out.platform_output
            .commands
            .iter()
            .any(|c| matches!(c,egui::OutputCommand::CopyText(t) if t=="Nguyễn Quốc An"))
    );
    h.frame(vec![Event::PointerMoved(egui::pos2(0., 0.))]);
    h.frame(vec![]);
    let p = &h.ui.probe.widgets;
    assert!(p["vintage-form-table:header"].right() <= p["vintage-scroll"].right());
    for i in 0..3 {
        let value = p[&format!("vintage-form-table:cell:{i}:1")];
        let action = p[&format!("vintage-form-table:cell:{i}:2")];
        assert_eq!(value.right(), action.left());
        assert!(action.contains_rect(p[&format!("vintage-form-table:copy:{i}")]));
    }
}
#[test]
fn console_has_four_real_columns_grid_selection_and_fixed_header_when_scrolling() {
    use flutter_network_viewer::vintage::LINE;
    let mut h = Harness::new();
    h.click("vintage-nav:Tables");
    h.click("vintage-console-row:0");
    let out = h.frame(vec![]);
    let header = h.ui.probe.widgets["vintage-console-header"];
    let first = h.ui.probe.widgets["vintage-console-row:0"];
    assert!(
        header.right() <= h.ui.probe.widgets["vintage-scroll"].right(),
        "header={header:?} viewport={:?}",
        h.ui.probe.widgets["vintage-scroll"]
    );
    assert!(first.top() <= header.bottom() && first.bottom() > header.bottom());
    assert_eq!(
        first.bottom(),
        h.ui.probe.widgets["vintage-console-row:1"].top()
    );
    for col in 0..4 {
        let cell = h.ui.probe.widgets[&format!("vintage-console-head:{col}")];
        assert!(out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if ["Source","Level","Time","Message"].contains(&t.galley.text()) && cell.contains(t.pos))));
    }
    assert!(has_rect(
        &out,
        first,
        flutter_network_viewer::app_colors::LIGHT.accent,
        None
    ));
    assert!(out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if first.contains(t.pos) && t.galley.text()=="stdout" && t.galley.job.sections.iter().all(|s|s.format.color==egui::Color32::WHITE))));
    assert!(has_rect(
        &out,
        h.ui.probe.widgets["vintage-console-grid"],
        egui::Color32::TRANSPARENT,
        Some(LINE)
    ));
    let pos = first.center();
    h.frame(vec![
        Event::PointerMoved(pos),
        Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0., -53.),
            modifiers: Modifiers::NONE,
        },
    ]);
    for _ in 0..30 {
        h.frame(vec![]);
    }
    let out = h.frame(vec![]);
    assert_eq!(h.ui.probe.widgets["vintage-console-header"], header);
    assert!(h.ui.probe.widgets["vintage-console-row:0"].top() < first.top());
    for shape in &out.shapes {
        if let egui::Shape::Text(t) = &shape.shape
            && ["stdout", "logger", "stderr"].contains(&t.galley.text())
        {
            assert!(shape.clip_rect.top() >= header.bottom());
        }
    }
}

#[test]
fn icon_buttons_are_square_in_controls_toolbar_and_table_actions() {
    use flutter_network_viewer::vintage;
    let mut h = Harness::new();
    for size in [10., 14., 24.] {
        h.ui.settings.set_font_size(size).unwrap();
        h.click("vintage-nav:Controls");
        let copy = h.ui.probe.widgets["vintage-copy"];
        assert_eq!(copy.width(), copy.height(), "control icon at font {size}");
        h.click("vintage-nav:Bodies");
        for i in 0..3 {
            let button = h.ui.probe.widgets[&format!("vintage-form-table:copy:{i}")];
            let cell = h.ui.probe.widgets[&format!("vintage-form-table:cell:{i}:2")];
            assert_eq!(button.width(), button.height(), "table icon at font {size}");
            assert!(cell.contains_rect(button));
        }
        // All toolbar glyphs use the same production icon helper.
        let mut rects = Vec::new();
        let _ = h.ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for code in [0xef7f, 0xe5d5, 0xe8b8, 0xf090, 0xe567] {
                        rects.push(vintage::icon(ui, code, "Action").rect);
                    }
                });
            });
        });
        for rect in rects {
            assert_eq!(rect.width(), rect.height());
        }
    }
}

#[test]
fn key_value_content_aligns_title_and_value_left_and_actions_center() {
    let mut h = Harness::new();
    for size in [14., 24.] {
        h.ui.settings.set_font_size(size).unwrap();
        for (tab, id, titles, values) in [
            (
                "Tables",
                "vintage-metadata-table",
                vec!["URL", "Method", "Status", "Content-Type", "Elapsed"],
                vec![
                    "https://example.com/api/v1/account-info",
                    "GET",
                    "200 OK",
                    "application/json",
                    "128 ms",
                ],
            ),
            (
                "Bodies",
                "vintage-form-table",
                vec!["name", "email", "attachment"],
                vec![
                    "Nguyễn Quốc An",
                    "demo@example.com",
                    "sample.png · image/png · 24 KB",
                ],
            ),
        ] {
            h.click(&format!("vintage-nav:{tab}"));
            h.click(&format!("{id}:row:{}", titles.len() - 1));
            let out = h.frame(vec![]);
            for (i, (title, value)) in titles.iter().zip(values.iter()).enumerate() {
                let p = &h.ui.probe.widgets;
                let title_cell = p[&format!("{id}:cell:{i}:0")];
                let value_cell = p[&format!("{id}:cell:{i}:1")];
                for (text, cell) in [(title, title_cell), (value, value_cell)] {
                    let painted = out
                        .shapes
                        .iter()
                        .find_map(|s| match &s.shape {
                            egui::Shape::Text(t)
                                if t.galley.text() == *text
                                    && cell.contains(t.pos)
                                    && s.clip_rect.intersects(cell) =>
                            {
                                Some(t)
                            }
                            _ => None,
                        })
                        .expect("table text painted");
                    let edge = painted.pos.x + painted.galley.rect.left();
                    let expected = cell.left() + 8.;
                    assert!(
                        (edge - expected).abs() <= 1.,
                        "{id} {text}: edge={edge}, expected={expected}"
                    );
                }
                if tab == "Bodies" {
                    let action = p[&format!("{id}:cell:{i}:2")];
                    let button = p[&format!("{id}:copy:{i}")];
                    assert!((action.center().x - button.center().x).abs() <= 1.);
                    assert!((action.center().y - button.center().y).abs() <= 1.);
                    assert_eq!(button.width(), button.height());
                }
            }
        }
    }
}

#[test]
fn action_hover_preserves_widget_dimensions_and_font_size() {
    let mut h = Harness::new();
    h.click("vintage-nav:Controls");
    for key in ["vintage-primary", "vintage-copy"] {
        h.frame(vec![Event::PointerMoved(egui::pos2(0., 0.))]);
        let base = h.frame(vec![]);
        let rect = h.ui.probe.widgets[key];
        let text = base
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t)
                    if rect.contains_rect(t.galley.rect.translate(t.pos.to_vec2())) =>
                {
                    Some(t.clone())
                }
                _ => None,
            })
            .expect("action text or icon");
        h.frame(vec![Event::PointerMoved(rect.center())]);
        let hovered = h.frame(vec![]);
        let enlarged = h.ui.probe.widgets[key];
        assert_eq!(enlarged, rect, "hover keeps the action rectangle unchanged");
        assert!(
            (enlarged.width() - rect.width()).abs() <= 1.5,
            "width stays fixed for {key}"
        );
        assert!(
            (enlarged.height() - rect.height()).abs() <= 1.5,
            "height stays fixed for {key}"
        );
        assert!(hovered.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Rect(r) if (r.rect.min-enlarged.min).length()<0.7 && (r.rect.max-enlarged.max).length()<0.7)), "missing normal-size background for {key}");
        let zoomed_text = hovered
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t)
                    if t.galley.text() == text.galley.text() && enlarged.contains(t.pos) =>
                {
                    Some(t)
                }
                _ => None,
            })
            .unwrap();
        for (base, hover) in text
            .galley
            .job
            .sections
            .iter()
            .zip(&zoomed_text.galley.job.sections)
        {
            assert!(
                (hover.format.font_id.size - base.format.font_id.size).abs() < 0.01,
                "font must remain the same size for {key}"
            );
        }
        let expected_job = (*zoomed_text.galley.job).clone();
        let fresh = h.ctx.fonts_mut(|fonts| fonts.layout_job(expected_job));
        assert_eq!(
            zoomed_text.galley.rows, fresh.rows,
            "glyphs must come from fresh font layout, not a transformed mesh"
        );
        h.frame(vec![Event::PointerMoved(egui::pos2(0., 0.))]);
        let restored = h.frame(vec![]);
        assert!(
            has_rect(
                &restored,
                rect,
                if key == "vintage-primary" {
                    flutter_network_viewer::app_colors::LIGHT.accent
                } else {
                    flutter_network_viewer::vintage::CONTROL
                },
                None
            ) || key == "vintage-dropdown"
        );
    }
    // Clicking inside the original icon edge still dispatches its action.
    let rect = h.ui.probe.widgets["vintage-copy"];
    h.frame(vec![Event::PointerMoved(rect.center())]);
    h.frame(vec![]);
    let rim = egui::pos2(rect.left() + 1., rect.center().y);
    h.frame(vec![Event::PointerMoved(rim)]);
    h.frame(vec![Event::PointerButton {
        pos: rim,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    }]);
    let out = h.frame(vec![Event::PointerButton {
        pos: rim,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    }]);
    assert!(
        out.platform_output
            .commands
            .iter()
            .any(|c| matches!(c, egui::OutputCommand::CopyText(_)))
    );
}

#[test]
fn hover_keeps_action_variants_and_popup_options_at_normal_size() {
    for size in [14., 24.] {
        let mut h = Harness::new();
        h.ui.settings.set_font_size(size).unwrap();
        h.click("vintage-nav:Controls");
        for (key, label, _enabled) in [
            ("vintage-quiet", "Quiet action", true),
            ("vintage-disabled", "Disabled", false),
            ("vintage-checkbox", "Checkbox", true),
            ("vintage-radio:1", "Selected", true),
        ] {
            h.click(key);
            let base = h.frame(vec![]);
            let rect = h.ui.probe.widgets[key];
            let font_size = |out: &egui::FullOutput| {
                out.shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Text(t)
                            if t.galley.text() == label && rect.expand(30.).contains(t.pos) =>
                        {
                            Some(t.galley.job.sections[0].format.font_id.size)
                        }
                        _ => None,
                    })
                    .expect("painted action label")
            };
            let base_font = font_size(&base);
            h.frame(vec![Event::PointerMoved(rect.center())]);
            let hovered = h.frame(vec![]);
            let expected = 1.;
            assert!(
                (font_size(&hovered) / base_font - expected).abs() < 0.01,
                "{key} at font{size}"
            );
            let actual = h.ui.probe.widgets[key];
            assert!((actual.height() - rect.height() * expected).abs() <= 1.5);
        }
        h.click("vintage-dropdown");
        for _ in 0..12 {
            h.frame(vec![]);
        }
        let rect = h.ui.probe.widgets["vintage-theme-option:1"];
        let base = h.frame(vec![]);
        let popup_font_size = |out: &egui::FullOutput| {
            out.shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t)
                        if t.galley.text() == "Vintage ink" && rect.expand(30.).contains(t.pos) =>
                    {
                        Some(t.galley.job.sections[0].format.font_id.size)
                    }
                    _ => None,
                })
                .unwrap()
        };
        let base_popup_font = popup_font_size(&base);
        h.frame(vec![Event::PointerMoved(rect.center())]);
        let hovered = h.frame(vec![]);
        assert!((popup_font_size(&hovered) / base_popup_font - 1.).abs() < 0.01);
        h.click("vintage-theme-option:1");
        let out = h.frame(vec![]);
        let dropdown = h.ui.probe.widgets["vintage-dropdown"];
        assert!(out.shapes.iter().any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="Vintage ink" && dropdown.contains(t.pos))));
        // Reset theme selection for the next font-size scenario.
        h.click("vintage-dropdown");
        h.click("vintage-theme-option:0");
    }
}

#[test]
fn hovered_table_icon_keeps_center_and_rasterizes_at_current_dpi() {
    for dpi in [1., 2.] {
        for font in [14., 24.] {
            let mut h = Harness::new();
            h.dpi = dpi;
            h.ui.settings.set_font_size(font).unwrap();
            h.click("vintage-nav:Bodies");
            let base = h.ui.probe.widgets["vintage-form-table:copy:0"];
            let cell = h.ui.probe.widgets["vintage-form-table:cell:0:2"];
            h.frame(vec![Event::PointerMoved(base.center())]);
            let out = h.frame(vec![]);
            let grown = h.ui.probe.widgets["vintage-form-table:copy:0"];
            assert!((grown.width() - base.width()).abs() <= 1.);
            assert!((grown.height() - base.height()).abs() <= 1.);
            assert_eq!(grown.size(), egui::vec2(24., 24.));
            assert!(
                (grown.center() - cell.center()).length() <= 1.,
                "centered Action at dpi{dpi} font{font}: {grown:?} cell{cell:?}"
            );
            let glyph = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t)
                        if t.galley.text() == "\u{ef7f}" && grown.contains(t.pos) =>
                    {
                        Some(t)
                    }
                    _ => None,
                })
                .expect("copy icon text shape");
            let ink = glyph.galley.mesh_bounds.translate(glyph.pos.to_vec2());
            assert!((ink.center() - grown.center()).length() <= 0.55);
            assert!((glyph.galley.job.sections[0].format.font_id.size - 14.).abs() < 0.01);
            assert!((glyph.galley.pixels_per_point - dpi).abs() < 0.01);
            let fresh = h
                .ctx
                .fonts_mut(|f| f.layout_job((*glyph.galley.job).clone()));
            assert_eq!(glyph.galley.rows, fresh.rows);
        }
    }
}

#[test]
fn hover_keeps_controls_neighbors_rows_and_panels_unchanged() {
    for size in [14., 24.] {
        let mut h = Harness::new();
        h.ui.settings.set_font_size(size).unwrap();
        for (tab, key) in [
            ("Controls", "vintage-nav:Controls"),
            ("Controls", "vintage-primary"),
            ("Controls", "vintage-copy"),
            ("Controls", "vintage-dropdown"),
            ("Bodies", "vintage-form-table:copy:0"),
        ] {
            h.click(&format!("vintage-nav:{tab}"));
            h.reveal(key);
            h.frame(vec![Event::PointerMoved(egui::pos2(0., 0.))]);
            for _ in 0..30 {
                h.frame(vec![]);
            }
            let baseline = h.ui.probe.widgets.clone();
            let rect = baseline[key];
            h.frame(vec![Event::PointerMoved(rect.center())]);
            for _ in 0..5 {
                h.frame(vec![]);
            }
            let grown = h.ui.probe.widgets[key];
            let factor = 1.;
            assert!(
                (grown.width() - rect.width() * factor).abs() <= 1.5,
                "{key}"
            );
            assert!(
                (grown.height() - rect.height() * factor).abs() <= 1.5,
                "{key}"
            );
            assert!(
                (grown.center() - rect.center()).length() <= 1.,
                "hover center {key} font{size}: base{rect:?} grown{grown:?}"
            );
            for (neighbor, before) in &baseline {
                if neighbor != key {
                    assert_eq!(
                        h.ui.probe.widgets[neighbor], *before,
                        "{key} moves/resizes {neighbor} at font{size}"
                    );
                }
            }
            h.frame(vec![Event::PointerMoved(egui::pos2(0., 0.))]);
            h.frame(vec![]);
            assert_eq!(h.ui.probe.widgets, baseline, "restore {key}");
        }
    }
}

#[test]
fn action_controls_have_solid_backgrounds_at_rest_and_hover() {
    let mut h = Harness::new();
    h.click("vintage-nav:Controls");
    for key in [
        "vintage-primary",
        "vintage-quiet",
        "vintage-disabled",
        "vintage-copy",
        "vintage-dropdown",
        "vintage-checkbox",
        "vintage-radio:1",
    ] {
        h.reveal(key);
        h.frame(vec![Event::PointerMoved(egui::pos2(0., 0.))]);
        for _ in 0..30 {
            h.frame(vec![]);
        }
        let base = h.ui.probe.widgets[key];
        for hovering in [false, true] {
            h.frame(vec![Event::PointerMoved(if hovering {
                base.center()
            } else {
                egui::pos2(0., 0.)
            })]);
            let out = h.frame(vec![]);
            let rect = h.ui.probe.widgets[key];
            assert!(out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Rect(r) if (r.rect.min-rect.min).length()<0.6 && (r.rect.max-rect.max).length()<0.6 && r.fill.a()==255)), "opaque full background {key}, hovering={hovering}");
        }
    }
}

#[test]
fn hovered_dropdown_stays_opaque_without_overlapping_neighbor_labels() {
    let mut h = Harness::new();
    h.click("vintage-nav:Controls");
    h.reveal("vintage-dropdown");
    h.frame(vec![Event::PointerMoved(egui::pos2(0., 0.))]);
    for _ in 0..30 {
        h.frame(vec![]);
    }
    let base = h.ui.probe.widgets["vintage-dropdown"];
    h.frame(vec![Event::PointerMoved(base.center())]);
    let out = h.frame(vec![]);
    let rect = h.ui.probe.widgets["vintage-dropdown"];
    let neighbor = out.shapes.iter().position(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text().starts_with("Dropdown state demo"))).unwrap();
    let backing = out
        .shapes
        .iter()
        .enumerate()
        .find_map(|(i, s)| match &s.shape {
            egui::Shape::Rect(r)
                if (r.rect.min - rect.min).length() < 0.6
                    && (r.rect.max - rect.max).length() < 0.6
                    && r.fill.a() == 255 =>
            {
                Some(i)
            }
            _ => None,
        })
        .expect("opaque dropdown background");
    let egui::Shape::Text(neighbor) = &out.shapes[neighbor].shape else {
        unreachable!()
    };
    let neighbor_ink = neighbor
        .galley
        .mesh_bounds
        .translate(neighbor.pos.to_vec2());
    assert!(
        !rect.intersects(neighbor_ink),
        "normal-size dropdown does not cover neighboring text"
    );
    assert_eq!(rect, base);
    let text = out.shapes.iter().position(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()=="Vintage paper" && rect.contains(t.pos))).unwrap();
    assert!(text > backing, "own label remains above its backing");
}

#[test]
fn dropdowns_and_options_keep_dimensions_and_font_size_on_hover() {
    for font in [14., 24.] {
        let mut h = Harness::new();
        h.ui.settings.set_font_size(font).unwrap();
        for (tab, key, option, selected) in [
            (
                "Controls",
                "vintage-dropdown",
                "vintage-theme-option:1",
                "Vintage ink",
            ),
            (
                "Tables",
                "vintage-table-filter",
                "vintage-filter-option:1",
                "GET",
            ),
        ] {
            h.click(&format!("vintage-nav:{tab}"));
            h.reveal(key);
            h.frame(vec![Event::PointerMoved(egui::pos2(0., 0.))]);
            for _ in 0..30 {
                h.frame(vec![]);
            }
            let base = h.ui.probe.widgets[key];
            h.frame(vec![Event::PointerMoved(base.center())]);
            let out = h.frame(vec![]);
            assert_eq!(h.ui.probe.widgets[key], base, "closed dropdown {key}");
            for shape in &out.shapes {
                if let egui::Shape::Text(t) = &shape.shape
                    && base.contains(t.pos)
                {
                    assert!(
                        t.galley
                            .job
                            .sections
                            .iter()
                            .all(|s| (s.format.font_id.size - font).abs() < 0.01)
                    );
                }
            }
            h.click(key);
            for _ in 0..12 {
                h.frame(vec![]);
            }
            let rect = h.ui.probe.widgets[option];
            h.frame(vec![Event::PointerMoved(rect.center())]);
            let out = h.frame(vec![]);
            assert_eq!(h.ui.probe.widgets[option], rect, "option {option}");
            let text = out
                .shapes
                .iter()
                .find_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == selected && rect.contains(t.pos) => {
                        Some(t)
                    }
                    _ => None,
                })
                .unwrap();
            assert!(
                text.galley
                    .job
                    .sections
                    .iter()
                    .all(|s| (s.format.font_id.size - font).abs() < 0.01)
            );
            h.click(option);
            let out = h.frame(vec![]);
            let dropdown = h.ui.probe.widgets[key];
            assert!(out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==selected && dropdown.contains(t.pos))));
        }
    }
}

#[test]
fn clickable_widgets_and_dropdown_options_use_hand_cursor() {
    let mut h = Harness::new();
    for (tab, key) in [
        ("Controls", "vintage-nav:Controls"),
        ("Controls", "vintage-primary"),
        ("Controls", "vintage-quiet"),
        ("Controls", "vintage-copy"),
        ("Controls", "vintage-checkbox"),
        ("Controls", "vintage-radio:1"),
        ("Controls", "vintage-switch"),
        ("Controls", "vintage-dropdown"),
        ("Navigation", "vintage-link"),
        ("Navigation", "vintage-tab:Network"),
        ("Bodies", "vintage-form-table:copy:0"),
        ("Tables", "vintage-row:0"),
        ("Tables", "vintage-console-row:0"),
    ] {
        h.click(&format!("vintage-nav:{tab}"));
        h.reveal(key);
        h.frame(vec![Event::PointerMoved(egui::pos2(0., 0.))]);
        for _ in 0..30 {
            h.frame(vec![]);
        }
        let rect = h.ui.probe.widgets[key];
        h.frame(vec![Event::PointerMoved(rect.center())]);
        let out = h.frame(vec![]);
        assert_eq!(
            out.platform_output.cursor_icon,
            egui::CursorIcon::PointingHand,
            "{key}"
        );
    }
    for (tab, key, option) in [
        ("Controls", "vintage-dropdown", "vintage-theme-option:1"),
        ("Tables", "vintage-table-filter", "vintage-filter-option:1"),
    ] {
        h.click(&format!("vintage-nav:{tab}"));
        h.click(key);
        for _ in 0..12 {
            h.frame(vec![]);
        }
        let rect = h.ui.probe.widgets[option];
        h.frame(vec![Event::PointerMoved(rect.center())]);
        let out = h.frame(vec![]);
        assert_eq!(
            out.platform_output.cursor_icon,
            egui::CursorIcon::PointingHand,
            "{option}"
        );
        h.click(option);
    }
}

#[test]
fn disabled_text_and_resize_widgets_keep_appropriate_cursors() {
    let mut h = Harness::new();
    h.click("vintage-nav:Controls");
    for (key, cursor) in [
        ("vintage-disabled", egui::CursorIcon::Default),
        ("vintage-input", egui::CursorIcon::Text),
        ("vintage-font-value", egui::CursorIcon::Default),
    ] {
        h.reveal(key);
        h.frame(vec![Event::PointerMoved(egui::pos2(0., 0.))]);
        for _ in 0..30 {
            h.frame(vec![]);
        }
        let rect = h.ui.probe.widgets[key];
        h.frame(vec![Event::PointerMoved(rect.center())]);
        let out = h.frame(vec![]);
        assert_eq!(out.platform_output.cursor_icon, cursor, "{key}");
    }
    h.click("vintage-nav:Tables");
    h.reveal("vintage-splitter");
    h.frame(vec![Event::PointerMoved(egui::pos2(0., 0.))]);
    for _ in 0..30 {
        h.frame(vec![]);
    }
    let rect = h.ui.probe.widgets["vintage-splitter"];
    h.frame(vec![Event::PointerMoved(rect.center())]);
    let out = h.frame(vec![]);
    assert_eq!(
        out.platform_output.cursor_icon,
        egui::CursorIcon::ResizeHorizontal
    );
    h.frame(vec![Event::PointerMoved(egui::pos2(0., 0.))]);
    assert_eq!(
        h.frame(vec![]).platform_output.cursor_icon,
        egui::CursorIcon::Default
    );
}

#[test]
fn dropdown_list_has_no_outer_padding_and_options_keep_internal_padding() {
    use flutter_network_viewer::vintage::LINE;
    for font in [14., 24.] {
        let mut h = Harness::new();
        h.ui.settings.set_font_size(font).unwrap();
        for (tab, key, prefix, labels) in [
            (
                "Controls",
                "vintage-dropdown",
                "vintage-theme-option",
                vec!["Vintage paper", "Vintage ink"],
            ),
            (
                "Tables",
                "vintage-table-filter",
                "vintage-filter-option",
                vec!["All methods", "GET", "POST"],
            ),
        ] {
            h.click(&format!("vintage-nav:{tab}"));
            h.click(key);
            for _ in 0..12 {
                h.frame(vec![]);
            }
            let out = h.frame(vec![]);
            let first = h.ui.probe.widgets[&format!("{prefix}:0")];
            let mut list = first;
            for (i, label) in labels.iter().enumerate() {
                let row = h.ui.probe.widgets[&format!("{prefix}:{i}")];
                assert_eq!(row.left(), first.left());
                assert_eq!(row.right(), first.right(), "option fills list width");
                if i > 0 {
                    assert!(
                        (row.top() - list.bottom()).abs() < 0.1,
                        "no gaps between options"
                    );
                }
                list = list.union(row);
                let text = out
                    .shapes
                    .iter()
                    .find_map(|s| match &s.shape {
                        egui::Shape::Text(t)
                            if t.galley.text() == *label && row.contains(t.pos) =>
                        {
                            Some(t)
                        }
                        _ => None,
                    })
                    .unwrap();
                let bounds = text.galley.rect.translate(text.pos.to_vec2());
                assert!(
                    bounds.left() - row.left() >= 11.5 && row.right() - bounds.right() >= 11.5,
                    "option horizontal padding"
                );
                assert!(
                    bounds.top() - row.top() >= 7.5 && row.bottom() - bounds.bottom() >= 7.5,
                    "option vertical padding"
                );
            }
            // Only the popup's 1px border surrounds the rows; no container margin.
            assert!(out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Rect(r) if r.fill==flutter_network_viewer::app_colors::LIGHT.surface && r.stroke.color==LINE && (r.rect.min-list.expand(1.).min).length()<1. && (r.rect.max-list.expand(1.).max).length()<1.)), "popup border flush with options at font{font}");
            h.click(&format!("{prefix}:1"));
        }
    }
}

#[test]
fn project_selector_uses_plain_names_without_status_badges() {
    use flutter_network_viewer::app_colors::LIGHT;
    let background = LIGHT.on_accent;
    let primary = LIGHT.accent;
    let mut h = Harness::new();
    h.click("vintage-nav:Navigation");
    for name in ["All", "pgapp", "staging"] {
        let out = h.frame(vec![]);
        let rect =
            h.ui.probe
                .widgets
                .get(&format!("vintage-project:{name}"))
                .copied()
                .expect("plain project name");
        assert!(out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==name && rect.contains(t.pos))));
        assert!(!out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if rect.contains(t.pos) && (t.galley.text().contains('●') || t.galley.text().contains('○')))));
        h.click(&format!("vintage-project:{name}"));
        let out = h.frame(vec![]);
        let rect = h.ui.probe.widgets[&format!("vintage-project:{name}")];
        assert!(has_rect(&out, rect, primary, None));
        assert!(out.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==name && rect.contains(t.pos) && t.galley.job.sections.iter().all(|s|s.format.color==background))));
    }
}

#[test]
fn shared_json_demo_has_independent_clickable_root_and_child_folds() {
    let mut h = Harness::new();
    h.click("vintage-nav:JSON");
    for view in ["code", "tree"] {
        let root = format!("vintage-{view}:json-fold:0");
        assert!(h.ui.probe.widgets.contains_key(&root), "missing {root}");
        let child =
            h.ui.probe
                .widgets
                .keys()
                .find(|k| k.starts_with(&format!("vintage-{view}:json-fold:")) && **k != root)
                .cloned()
                .expect("nested collection arrow");
        h.click(&child);
        assert!(h.ui.probe.widgets.contains_key(&root));
        h.click(&child);
        h.click(&root);
        assert!(
            !h.ui.probe.widgets.contains_key(&child),
            "root must hide children"
        );
        let other = if view == "code" { "tree" } else { "code" };
        assert!(
            h.ui.probe.widgets.keys().any(|k| k
                .starts_with(&format!("vintage-{other}:json-fold:"))
                && !k.ends_with(":0")),
            "views fold independently"
        );
        h.click(&root);
        assert!(h.ui.probe.widgets.contains_key(&child));
    }
}

#[test]
fn production_uses_vintage_surfaces_and_square_components() {
    let mut h = Harness::new();
    h.click("vintage-legacy");
    for key in ["Network", "Console", "JSON", "settings"] {
        if let Some(rect) = h.ui.probe.widgets.get(key).copied() {
            let pos = rect.center();
            h.frame(vec![Event::PointerMoved(pos)]);
            for pressed in [true, false] {
                h.frame(vec![Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                }]);
            }
            h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
            let output = h.frame(vec![]);
            assert_eq!(
                h.ctx.style().visuals.panel_fill,
                flutter_network_viewer::vintage::BACKGROUND
            );
            for shape in output.shapes {
                if let egui::Shape::Rect(r) = shape.shape {
                    assert_eq!(
                        r.corner_radius,
                        egui::CornerRadius::ZERO,
                        "rounded component on {key}"
                    );
                }
            }
        }
    }
}

#[test]
fn gallery_font_stepper_matches_production_square_display_only_control() {
    let mut h = Harness::new();
    h.click("vintage-nav:Controls");
    h.reveal("vintage-font-value");
    for key in ["vintage-font-minus", "vintage-font-plus"] {
        let rect = h.ui.probe.widgets[key];
        assert!((rect.width() - rect.height()).abs() < 0.5, "{key} {rect:?}");
    }
    h.click("vintage-font-value");
    let out = h.frame(vec![]);
    assert_eq!(out.platform_output.cursor_icon, egui::CursorIcon::Default);
    assert_eq!(h.ui.settings.values().font_size, 14.);
}

#[test]
fn gallery_switch_previews_the_production_track_and_square_knob() {
    let mut h = Harness::new();
    h.click("vintage-nav:Controls");
    h.reveal("vintage-switch");
    h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
    for _ in 0..4 {
        h.frame(vec![]);
    }
    let out = h.frame(vec![]);
    let rect = h.ui.probe.widgets["vintage-switch"];
    assert!((rect.width() - 52.).abs() < 0.5 && (rect.height() - 32.).abs() < 0.5);
    assert!(out.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Rect(r) if rect.contains_rect(r.rect) && (r.rect.width() - 20.).abs() < 0.5 && (r.rect.height() - 20.).abs() < 0.5)));
    h.frame(vec![Event::PointerMoved(rect.center())]);
    for _ in 0..6 {
        h.frame(vec![]);
    }
    let hovered = h.ui.probe.widgets["vintage-switch"];
    assert!((hovered.width() - rect.width()).abs() < 0.5);
    assert!((hovered.height() - rect.height()).abs() < 0.5);
    assert!((hovered.center() - rect.center()).length() < 0.5);
    h.click("vintage-switch");
    assert!(h.ui.vintage.enabled);
}

#[test]
fn key_value_tables_wrap_full_values_inside_fixed_title_columns_without_horizontal_overflow() {
    use flutter_network_viewer::{
        desktop::UiProbe,
        settings::{AppSettings, ThemeMode},
    };
    fn draw(
        ctx: &egui::Context,
        settings: &AppSettings,
        width: f32,
        copy: bool,
        probe: &mut UiProbe,
        entries: &[(&str, &str)],
        events: Vec<Event>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(width + 16., 4000.),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                if !flutter_network_viewer::desktop_support::apply(ctx, settings) {
                    return;
                }
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE.inner_margin(8))
                    .show(ctx, |ui| {
                        flutter_network_viewer::vintage::key_value_table(
                            ui, probe, "wrap", entries, copy,
                        );
                    });
            },
        )
    }
    let long_value = format!(
        "https://pgapp-feature.nst.com.vn/api/v1/dms-service/staffPosition?request={}",
        "abcdef0123456789".repeat(8)
    );
    let entries = [
        ("URL", long_value.as_str()),
        (
            "A deliberately long title that also needs to wrap inside its fixed column",
            "Tiếng Việt dòng một\nDòng thứ hai",
        ),
        ("Status", "200 OK"),
    ];
    for width in [280., 520., 960.] {
        for size in [14., 24.] {
            for copy in [false, true] {
                let ctx = egui::Context::default();
                let mut settings = AppSettings::default();
                settings.set_font_size(size).unwrap();
                settings
                    .set_theme_mode(if copy {
                        ThemeMode::Dark
                    } else {
                        ThemeMode::Light
                    })
                    .unwrap();
                let mut probe = UiProbe::default();
                for _ in 0..3 {
                    draw(&ctx, &settings, width, copy, &mut probe, &entries, vec![]);
                }
                let out = draw(&ctx, &settings, width, copy, &mut probe, &entries, vec![]);
                let grid = probe.widgets["wrap:grid"];
                assert!(
                    (grid.width() - width).abs() < 0.5,
                    "table fits width {width}: {grid:?}"
                );
                assert!(grid.right() <= width + 8.5);
                assert_eq!(probe.widgets["wrap:cell:0:0"].width(), 160.);
                let long_row = probe.widgets["wrap:row:0"];
                let short_row = probe.widgets["wrap:row:2"];
                assert!(
                    long_row.height() > short_row.height(),
                    "long URL increases row height"
                );
                for (i, (title, value)) in entries.iter().enumerate() {
                    let row = probe.widgets[&format!("wrap:row:{i}")];
                    for (column, text) in [(0, *title), (1, *value)] {
                        let cell = probe.widgets[&format!("wrap:cell:{i}:{column}")];
                        assert_eq!(cell.height(), row.height());
                        let painted = out
                            .shapes
                            .iter()
                            .find_map(|s| match &s.shape {
                                egui::Shape::Text(t)
                                    if t.galley.text() == text && cell.contains(t.pos) =>
                                {
                                    Some(t)
                                }
                                _ => None,
                            })
                            .expect("full text painted without truncation");
                        let bounds = painted.galley.rect.translate(painted.pos.to_vec2());
                        assert!(
                            cell.shrink2(egui::vec2(7., 3.)).contains_rect(bounds),
                            "text stays inside its cell: {text} {bounds:?} {cell:?}"
                        );
                        assert!((bounds.left() - cell.left() - 8.).abs() < 0.6);
                        if i == 0 && column == 1 && width <= 520. {
                            assert!(painted.galley.rows.len() > 1);
                        }
                    }
                    if i > 0 {
                        assert_eq!(
                            probe.widgets[&format!("wrap:row:{}", i - 1)].bottom(),
                            row.top()
                        );
                    }
                    if copy {
                        let button = probe.widgets[&format!("wrap:copy:{i}")];
                        let action = probe.widgets[&format!("wrap:cell:{i}:2")];
                        assert!((button.center() - action.center()).length() < 0.6);
                        assert_eq!(button.width(), button.height());
                    }
                }
                let before = probe.widgets["wrap:cell:0:1"];
                draw(
                    &ctx,
                    &settings,
                    width,
                    copy,
                    &mut probe,
                    &entries,
                    vec![
                        Event::PointerMoved(before.center()),
                        Event::MouseWheel {
                            unit: egui::MouseWheelUnit::Point,
                            delta: egui::vec2(-100., 0.),
                            modifiers: Modifiers::NONE,
                        },
                    ],
                );
                assert_eq!(
                    probe.widgets["wrap:cell:0:1"], before,
                    "horizontal wheel never shifts columns"
                );
                if copy {
                    let pos = probe.widgets["wrap:copy:0"].center();
                    draw(
                        &ctx,
                        &settings,
                        width,
                        copy,
                        &mut probe,
                        &entries,
                        vec![
                            Event::PointerMoved(pos),
                            Event::PointerButton {
                                pos,
                                button: PointerButton::Primary,
                                pressed: true,
                                modifiers: Modifiers::NONE,
                            },
                        ],
                    );
                    let out = draw(
                        &ctx,
                        &settings,
                        width,
                        copy,
                        &mut probe,
                        &entries,
                        vec![Event::PointerButton {
                            pos,
                            button: PointerButton::Primary,
                            pressed: false,
                            modifiers: Modifiers::NONE,
                        }],
                    );
                    assert!(out.platform_output.commands.iter().any(
                        |c| matches!(c,egui::OutputCommand::CopyText(text) if text==&long_value)
                    ));
                }
            }
        }
    }
}
