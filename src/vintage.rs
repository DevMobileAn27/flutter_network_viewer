//! Approved Vintage design kit and interactive component gallery.
use crate::{desktop::UiProbe, desktop_style, settings::AppSettings};
use eframe::egui::{self, Color32, Response, Stroke};

pub fn empty_text(ctx: &egui::Context) -> String {
    crate::desktop_support::tr(ctx, "No data")
}

// Compatibility aliases for the widget gallery; the palette lives in app_colors.
pub const BACKGROUND: Color32 = crate::app_colors::LIGHT.background;
pub const PRIMARY: Color32 = crate::app_colors::LIGHT.text;
pub const CONTROL: Color32 = crate::app_colors::LIGHT.control;
pub const MUTED: Color32 = crate::app_colors::LIGHT.muted;
pub const LINE: Color32 = crate::app_colors::LIGHT.line;
pub const CATEGORIES: [&str; 7] = [
    "Overview",
    "Controls",
    "Navigation",
    "Tables",
    "JSON",
    "Bodies",
    "Feedback",
];
pub struct InventoryItem {
    pub name: &'static str,
    pub source: &'static str,
    pub example: &'static str,
    pub category: &'static str,
}
macro_rules! inventory {
    ($(($name:literal, $source:literal, $example:literal, $category:literal)),* $(,)?) => {
        pub const INVENTORY: &[InventoryItem] = &[$(InventoryItem {name:$name,source:$source,example:$example,category:$category}),*];
    };
}
inventory![
    (
        "Primary / outline / quiet / disabled button",
        "desktop_shell.rs; desktop_support.rs",
        "vintage-primary",
        "Controls"
    ),
    (
        "Icon action / tooltip / copy",
        "desktop_style.rs",
        "vintage-copy",
        "Controls"
    ),
    (
        "Single-line input / focus / hint",
        "desktop_support.rs",
        "vintage-input",
        "Controls"
    ),
    (
        "Search / clear / validation",
        "desktop_shell.rs",
        "vintage-search",
        "Controls"
    ),
    (
        "Dropdown / menu selection",
        "desktop_support.rs",
        "vintage-dropdown",
        "Controls"
    ),
    (
        "Display-only value / square font-size stepper",
        "desktop_support.rs",
        "vintage-font-value",
        "Controls"
    ),
    (
        "Toggle / checkbox / radio",
        "desktop_support.rs",
        "vintage-switch",
        "Controls"
    ),
    (
        "Multiline / selectable text",
        "desktop_json.rs",
        "vintage-multiline",
        "Controls"
    ),
    (
        "Page tabs",
        "desktop_shell.rs",
        "vintage-tab:Network",
        "Navigation"
    ),
    (
        "Project selector / connection indicator",
        "desktop_shell.rs",
        "vintage-project:All",
        "Navigation"
    ),
    (
        "Inspector tabs",
        "desktop.rs",
        "vintage-detail:Overview",
        "Navigation"
    ),
    (
        "Toolbar / contextual actions",
        "desktop_shell.rs; desktop_json.rs",
        "vintage-pause",
        "Navigation"
    ),
    (
        "Breadcrumb / back / external link",
        "desktop_support.rs",
        "vintage-link",
        "Navigation"
    ),
    (
        "Network table / selection / grid",
        "desktop_network.rs",
        "vintage-row:0",
        "Tables"
    ),
    (
        "Header filters / popup",
        "desktop_network.rs",
        "vintage-table-filter",
        "Tables"
    ),
    (
        "Console source / level / time / message",
        "desktop.rs",
        "vintage-console",
        "Tables"
    ),
    (
        "Metadata card / key-value rows",
        "desktop.rs",
        "vintage-metadata",
        "Tables"
    ),
    (
        "Resizable split / inspector",
        "desktop.rs",
        "vintage-splitter",
        "Tables"
    ),
    (
        "Settings section / field labels",
        "desktop_support.rs",
        "vintage-settings",
        "Controls"
    ),
    (
        "JSON code / line numbers / syntax",
        "desktop_json.rs",
        "vintage-json-code",
        "JSON"
    ),
    (
        "JSON Tree / fold / expand",
        "desktop_json.rs",
        "vintage-json-fold",
        "JSON"
    ),
    (
        "JSON find / case / word / regex / navigation",
        "desktop_json.rs",
        "vintage-json-find",
        "JSON"
    ),
    (
        "JSON editor / format / copy / download",
        "desktop_json.rs",
        "vintage-json-copy",
        "JSON"
    ),
    (
        "Color swatch / value copy",
        "desktop_body.rs",
        "vintage-swatch",
        "JSON"
    ),
    (
        "Raw / Form / Image / body modes",
        "desktop_body.rs",
        "vintage-body:Form",
        "Bodies"
    ),
    (
        "Form fields / file metadata / omitted body",
        "desktop_body.rs",
        "vintage-form",
        "Bodies"
    ),
    (
        "Image preview / fit / zoom / enlarged",
        "desktop_body.rs",
        "vintage-image-open",
        "Bodies"
    ),
    (
        "Modal / manual connection / export",
        "desktop.rs",
        "vintage-dialog-open",
        "Feedback"
    ),
    (
        "Confirmation / cancel / dismiss",
        "desktop_support.rs",
        "vintage-dialog-cancel",
        "Feedback"
    ),
    (
        "Info / success / warning / error notice",
        "desktop_support.rs",
        "vintage-toast-open",
        "Feedback"
    ),
    (
        "Help sections / link / copy",
        "desktop_support.rs",
        "vintage-help",
        "Feedback"
    ),
    (
        "Loading / progress / retry",
        "desktop_support.rs; desktop_body.rs",
        "vintage-progress",
        "Feedback"
    ),
    (
        "Empty / no results text",
        "desktop_shell.rs",
        "vintage-empty",
        "Feedback"
    ),
    (
        "Footer / RAM / capture counters",
        "desktop_shell.rs",
        "vintage-footer",
        "Navigation"
    ),
    (
        "Card / separator / scroll / responsive layout",
        "desktop_support.rs; desktop.rs",
        "vintage-scroll",
        "Overview"
    ),
];

/// Shared Vintage theme for the production renderer and gallery.
pub fn apply(ctx: &egui::Context) {
    let p = desktop_style::Palette::of(ctx);
    ctx.style_mut(|s| {
        s.visuals = if s.visuals.dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        s.visuals.override_text_color = None;
        s.visuals.hyperlink_color = p.accent;
        s.visuals.panel_fill = p.background;
        s.visuals.window_fill = p.surface;
        s.visuals.extreme_bg_color = p.surface;
        s.visuals.text_edit_bg_color = Some(p.control);
        s.visuals.faint_bg_color = p.surface;
        s.visuals.selection.bg_fill = p.accent;
        s.visuals.selection.stroke = Stroke::new(1.0_f32, p.on_accent);
        s.visuals.window_stroke = Stroke::new(1.0_f32, p.line);
        s.visuals.window_corner_radius = egui::CornerRadius::ZERO;
        s.visuals.menu_corner_radius = egui::CornerRadius::ZERO;
        s.visuals.window_shadow = egui::epaint::Shadow::NONE;
        s.visuals.popup_shadow = egui::epaint::Shadow::NONE;
        for w in [
            &mut s.visuals.widgets.noninteractive,
            &mut s.visuals.widgets.inactive,
            &mut s.visuals.widgets.hovered,
            &mut s.visuals.widgets.active,
            &mut s.visuals.widgets.open,
        ] {
            w.corner_radius = egui::CornerRadius::ZERO;
            w.expansion = 0.;
            w.bg_fill = p.control;
            w.weak_bg_fill = p.control;
            w.bg_stroke = desktop_style::input_stroke(p, false);
            w.fg_stroke = Stroke::new(1.0_f32, p.text);
        }
        s.visuals.widgets.noninteractive.bg_fill = p.surface;
        s.visuals.widgets.noninteractive.weak_bg_fill = p.surface;
        s.visuals.clip_rect_margin = 0.;
        s.visuals.interact_cursor = Some(egui::CursorIcon::PointingHand);
        s.spacing.icon_spacing = desktop_style::ICON_TEXT_GAP;
        s.spacing.menu_margin = egui::Margin::ZERO;
        s.spacing.item_spacing = egui::vec2(10., 10.);
        s.spacing.button_padding = egui::vec2(12., 8.);
        s.spacing.interact_size = egui::vec2(32., 34.);
        s.spacing.window_margin = egui::Margin::same(20);
    });
}
pub fn card(ctx: &egui::Context) -> egui::Frame {
    let p = desktop_style::Palette::of(ctx);
    egui::Frame::NONE
        .fill(p.background)
        .stroke(Stroke::new(1.0_f32, p.line))
        .inner_margin(18)
        .corner_radius(0)
}
/// Render an opaque action at its normal size and retain its clickable cursor.
/// Hover never changes layout, widget dimensions, font sizes or icon sizes.
pub fn hover_action(
    ui: &mut egui::Ui,
    show: impl FnOnce(&mut egui::Ui, f32) -> Response,
) -> Response {
    action_control(ui, show)
}
fn fixed_action(ui: &mut egui::Ui, show: impl FnOnce(&mut egui::Ui) -> Response) -> Response {
    action_control(ui, |ui, _| show(ui))
}
fn dropdown_option(ui: &mut egui::Ui, selected: &mut usize, value: usize, label: &str) -> Response {
    fixed_action(ui, |ui| {
        ui.spacing_mut().button_padding = egui::vec2(12., 8.);
        let mut response = ui.add_sized(
            egui::vec2(ui.available_width(), ui.spacing().interact_size.y),
            egui::Button::selectable(*selected == value, label),
        );
        if response.clicked() && *selected != value {
            *selected = value;
            response.mark_changed();
        }
        response
    })
}
fn action_control(
    ui: &mut egui::Ui,
    show: impl FnOnce(&mut egui::Ui, f32) -> Response,
) -> Response {
    let cache_id = ui.next_auto_id().with((
        "vintage-action-metrics",
        desktop_style::font_size(ui.ctx()).to_bits(),
        ui.ctx().content_rect().width().to_bits(),
    ));
    let previous = ui.ctx().data(|d| d.get_temp::<egui::Vec2>(cache_id));
    let available = ui.available_rect_before_wrap();
    let (_, slot) = ui.allocate_space(previous.unwrap_or(egui::Vec2::ZERO));
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .id_salt(cache_id)
            .max_rect(if previous.is_some() { slot } else { available })
            .layout(if previous.is_none() {
                *ui.layout()
            } else {
                egui::Layout::top_down(egui::Align::Min)
            }),
    );
    child.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
    let backing = child.painter().add(egui::Shape::Noop);
    let response = show(&mut child, 1.);
    child.painter().set(
        backing,
        egui::Shape::rect_filled(
            response.rect,
            0,
            desktop_style::Palette::of(ui.ctx()).control,
        ),
    );
    ui.advance_cursor_after_rect(response.rect);
    ui.ctx()
        .data_mut(|d| d.insert_temp(cache_id, response.rect.size()));
    if response.enabled() && response.sense.senses_click() && !response.sense.senses_drag() {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}
pub fn button(ui: &mut egui::Ui, text: &str, primary: bool) -> Response {
    hover_action(ui, |ui, _| {
        let p = desktop_style::Palette::of(ui.ctx());
        ui.add(
            egui::Button::new(egui::RichText::new(text).color(if primary {
                p.on_accent
            } else {
                p.text
            }))
            .fill(if primary { p.accent } else { p.control })
            .stroke(Stroke::new(1.0_f32, p.text))
            .corner_radius(0),
        )
    })
}
/// Shared font-size control used by production Settings and the widget gallery.
/// Only the square decrement/increment actions can change the displayed value.
pub fn font_size_stepper(ui: &mut egui::Ui, size: &mut f32, probe: &mut UiProbe, keys: [&str; 3]) {
    ui.scope(|ui| {
        let font = egui::FontId::proportional(desktop_style::font_size(ui.ctx()));
        let display_height = ui.fonts_mut(|fonts| fonts.row_height(&font)) + 24.;
        ui.spacing_mut().interact_size.y = display_height.max(34.);
        ui.horizontal(|ui| {
            if probe
                .record(
                    keys[0],
                    ui.add_enabled_ui(*size > 10., |ui| {
                        desktop_style::square_icon(ui, 0xe516, "−")
                    })
                    .inner,
                )
                .clicked()
            {
                *size = (*size - 1.).max(10.);
            }
            probe.record(
                keys[1],
                crate::desktop_support::display_text(
                    ui,
                    &format!("{size:.0}"),
                    egui::Align::Center,
                    false,
                ),
            );
            if probe
                .record(
                    keys[2],
                    ui.add_enabled_ui(*size < 24., |ui| {
                        desktop_style::square_icon(ui, 0xe047, "+")
                    })
                    .inner,
                )
                .clicked()
            {
                *size = (*size + 1.).min(24.);
            }
        });
    });
}

pub fn icon(ui: &mut egui::Ui, code: u32, tip: &str) -> Response {
    desktop_style::square_icon(ui, code, tip)
}
pub fn tabs(
    ui: &mut egui::Ui,
    probe: &mut UiProbe,
    prefix: &str,
    options: &[&str],
    selected: &mut usize,
) {
    ui.horizontal_wrapped(|ui| {
        for (i, label) in options.iter().enumerate() {
            if probe
                .record(
                    format!("{prefix}:{label}"),
                    button(ui, label, *selected == i),
                )
                .clicked()
            {
                *selected = i;
            }
        }
    });
}
pub fn input(ui: &mut egui::Ui, value: &mut String, hint: &str) -> Response {
    crate::desktop_support::input_with_hint(ui, value, (ui.available_width() - 24.).max(1.), hint)
}
/// Shared square tile for information, success, warning, error and dialogs.
fn notice_heading(ui: &mut egui::Ui, icon: &str, title: &str, probe: &mut UiProbe) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = desktop_style::ICON_TEXT_GAP;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(28., 28.), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 0, desktop_style::Palette::of(ui.ctx()).accent);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(desktop_style::font_size(ui.ctx())),
            desktop_style::Palette::of(ui.ctx()).on_accent,
        );
        probe.widgets.insert("vintage-notice-icon".into(), rect);
        ui.label(title);
    });
}
pub struct LoadingResponse {
    pub response: Response,
    pub cells: Vec<egui::Rect>,
    pub active: usize,
}
/// Indeterminate loading: one inset ink square advances every 200ms and wraps.
/// Border is 1px plus 2px inner padding; count is always in the supported 3..=10.
pub fn loading(ui: &mut egui::Ui, count: usize) -> LoadingResponse {
    let p = desktop_style::Palette::of(ui.ctx());
    let count = count.clamp(3, 10);
    let time = ui.input(|i| i.time).max(0.);
    let active = ((time * 5. + 1e-6).floor() as usize) % count;
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(count as f32 * 16. + (count - 1) as f32 * 4., 16.),
        egui::Sense::hover(),
    );
    let mut cells = Vec::with_capacity(count);
    for i in 0..count {
        let cell = egui::Rect::from_min_size(
            rect.min + egui::vec2(i as f32 * 20., 0.),
            egui::vec2(16., 16.),
        );
        ui.painter().rect(
            cell,
            0,
            p.background,
            Stroke::new(1.0_f32, p.accent),
            egui::StrokeKind::Inside,
        );
        if i == active {
            ui.painter().rect_filled(cell.shrink(3.), 0, p.accent);
        }
        cells.push(cell);
    }
    ui.ctx()
        .request_repaint_after(std::time::Duration::from_millis(200));
    LoadingResponse {
        response,
        cells,
        active,
    }
}
pub fn toggle(ui: &mut egui::Ui, value: &mut bool) -> Response {
    hover_action(ui, |ui, factor| {
        let (rect, mut response) =
            ui.allocate_exact_size(egui::vec2(52., 32.) * factor, egui::Sense::click());
        let p = desktop_style::Palette::of(ui.ctx());
        if response.clicked() {
            *value = !*value;
            response.mark_changed();
        }
        ui.painter().rect(
            rect,
            0,
            if *value { p.accent } else { p.control },
            egui::Stroke::new(1.0_f32, p.text),
            egui::StrokeKind::Inside,
        );
        let knob = egui::Rect::from_center_size(
            egui::pos2(
                if *value {
                    rect.right() - 16. * factor
                } else {
                    rect.left() + 16. * factor
                },
                rect.center().y,
            ),
            egui::Vec2::splat(20. * factor),
        );
        ui.painter()
            .rect_filled(knob, 0, if *value { p.on_accent } else { p.text });
        response
    })
}

fn heading(ui: &mut egui::Ui, index: &str, title: &str, subtitle: &str) {
    ui.horizontal_wrapped(|ui| {
        ui.label(index);
        ui.label(
            egui::RichText::new(title)
                .font(desktop_style::semibold(desktop_style::font_size(ui.ctx()))),
        );
    });
    ui.label(egui::RichText::new(subtitle).color(desktop_style::Palette::of(ui.ctx()).muted));
    ui.add_space(10.);
}
const JSON: &str = "{\n  \"timestamp\": \"2026-11-16T10:30:00\",\n  \"success\": true,\n  \"duration\": 128,\n  \"missing\": null,\n  \"data\": {\n    \"color\": \"#52634B\",\n    \"items\": [1, 2, 3]\n  }\n}";
/// Seven distinct roles, matching legacy JsonSyntax: punctuation, key, string,
/// number, boolean, collection accent, null/muted. Search adds a background only.
pub const JSON_COLORS: [Color32; 7] = crate::app_colors::JSON_LIGHT;

pub struct VintageDemo {
    pub section: usize,
    pub enabled: bool,
    pub folded: bool,
    search: String,
    value: String,
    draft: String,
    choice: usize,
    tab: usize,
    project: usize,
    detail: usize,
    selected_row: usize,
    selected_console_row: usize,
    filter: usize,
    body: usize,
    paused: bool,
    check: bool,
    radio: usize,
    dialog: bool,
    toast: bool,
    notice_kind: usize,
    image_open: bool,
    zoom: f32,
    split: f32,
    code_view: crate::desktop_json::JsonViewer,
    tree_view: crate::desktop_json::JsonViewer,
}
impl Default for VintageDemo {
    fn default() -> Self {
        Self {
            section: 0,
            enabled: false,
            folded: false,
            search: String::new(),
            value: "http://127.0.0.1:8181/".into(),
            draft: JSON.into(),
            choice: 0,
            tab: 0,
            project: 0,
            detail: 0,
            selected_row: 0,
            selected_console_row: 0,
            filter: 0,
            body: 0,
            paused: false,
            check: true,
            radio: 0,
            dialog: false,
            toast: false,
            notice_kind: 0,
            image_open: false,
            zoom: 1.,
            split: 0.55,
            code_view: Default::default(),
            tree_view: Default::default(),
        }
    }
}
impl VintageDemo {
    /// Returns true only when the reviewer explicitly opens the original app.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        settings: &mut AppSettings,
        probe: &mut UiProbe,
    ) -> bool {
        apply(ctx);
        let mut legacy = false;
        egui::TopBottomPanel::top("vintage-header")
            .frame(
                egui::Frame::NONE
                    .fill(desktop_style::Palette::of(ctx).background)
                    .inner_margin(18),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new("FNV / VINTAGE WIDGET LAB")
                            .font(desktop_style::semibold(desktop_style::font_size(ctx))),
                    );
                    ui.label(
                        egui::RichText::new("VINTAGE · COMPONENT LIBRARY")
                            .color(desktop_style::Palette::of(ctx).muted),
                    );
                    if probe
                        .record("vintage-legacy", button(ui, "Mở app hiện tại →", true))
                        .clicked()
                    {
                        legacy = true;
                    }
                });
                ui.add_space(8.);
                tabs(ui, probe, "vintage-nav", &CATEGORIES, &mut self.section);
            });
        egui::TopBottomPanel::bottom("vintage-foot")
            .frame(
                egui::Frame::NONE
                    .fill(desktop_style::Palette::of(ctx).background)
                    .inner_margin(10),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(format!("{} widget families", INVENTORY.len()));
                    ui.label("14 mặc định / một cỡ chữ / không bo góc");
                    ui.label("Bộ widget dùng chung với app");
                });
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(desktop_style::Palette::of(ctx).background)
                    .inner_margin(24),
            )
            .show(ctx, |ui| {
                let scroll = egui::ScrollArea::vertical()
                    .id_salt(("vintage-gallery", self.section))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.set_max_width(ui.available_width().min(1400.));
                        match self.section {
                            0 => self.overview(ui, probe),
                            1 => self.controls(ui, settings, probe),
                            2 => self.navigation(ui, probe),
                            3 => self.tables(ui, probe),
                            4 => self.json(ui, probe),
                            5 => self.bodies(ui, probe),
                            _ => self.feedback(ui, probe),
                        }
                    });
                probe
                    .widgets
                    .insert("vintage-scroll".into(), scroll.inner_rect);
            });
        if self.dialog {
            let mut close = false;
            let response = egui::Modal::new(egui::Id::new("vintage-confirmation"))
                .frame(card(ctx))
                .show(ctx, |ui| {
                    ui.set_max_width((ctx.content_rect().width() - 80.).min(480.));
                    notice_heading(ui, "?", "Kết nối thủ công", probe);
                    ui.label("Popup mẫu dùng chung cho Manual URL, Export và Confirmation.");
                    probe.record(
                        "vintage-dialog-input",
                        input(ui, &mut self.value, "VM Service URL"),
                    );
                    ui.label("Thao tác này chỉ cập nhật bản xem trước.");
                    ui.horizontal_wrapped(|ui| {
                        if probe
                            .record("vintage-dialog-cancel", button(ui, "Hủy", false))
                            .clicked()
                        {
                            close = true;
                        }
                        if probe
                            .record("vintage-dialog-confirm", button(ui, "Xác nhận", true))
                            .clicked()
                        {
                            close = true;
                            self.toast = true;
                        }
                    });
                });
            probe
                .widgets
                .insert("vintage-dialog".into(), response.response.rect);
            if close || response.should_close() {
                self.dialog = false;
            }
        }
        if self.image_open {
            let mut close = false;
            egui::Modal::new(egui::Id::new("vintage-image-modal"))
                .frame(card(ctx))
                .show(ctx, |ui| {
                    ui.set_width((ctx.content_rect().width() - 100.).min(600.));
                    self.image(ui, 200.);
                    if probe
                        .record("vintage-image-close", button(ui, "Đóng ảnh", false))
                        .clicked()
                    {
                        close = true;
                    }
                });
            if close {
                self.image_open = false;
            }
        }
        if self.toast {
            egui::Area::new(egui::Id::new("vintage-notice"))
                .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(18., -50.))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    let frame = card(ui.ctx()).show(ui, |ui| {
                        ui.set_width((ctx.content_rect().width() - 100.).clamp(140., 380.));
                        let (icon, title) = [
                            ("i", "Thông tin"),
                            ("✓", "Thành công"),
                            ("!", "Cảnh báo"),
                            ("×", "Có lỗi"),
                        ][self.notice_kind];
                        notice_heading(ui, icon, title, probe);
                        ui.label("Thông báo mẫu. Mọi thao tác demo đều ở dữ liệu giả lập.");
                        if probe
                            .record("vintage-toast-close", button(ui, "Đóng", false))
                            .clicked()
                        {
                            self.toast = false;
                        }
                    });
                    probe
                        .widgets
                        .insert("vintage-toast".into(), frame.response.rect);
                });
        }
        legacy
    }
    fn overview(&mut self, ui: &mut egui::Ui, probe: &mut UiProbe) {
        heading(
            ui,
            "01 / DESIGN DIRECTION",
            "Bộ màu giao diện hệ thống macOS Ventura",
            "Nền xám, vùng nội dung trắng, primary xanh. Giữ widget vuông và không đổ bóng.",
        );
        card(ui.ctx()).show(ui, |ui| {
            ui.label("PALETTE / MÀU NỀN & PRIMARY");
            ui.add_space(10.);
            ui.horizontal_wrapped(|ui| {
                for (label, color) in [
                    (
                        "BACKGROUND",
                        desktop_style::Palette::of(ui.ctx()).background,
                    ),
                    ("TEXT", desktop_style::Palette::of(ui.ctx()).text),
                    (
                        "PRIMARY / ACCENT",
                        desktop_style::Palette::of(ui.ctx()).accent,
                    ),
                    ("CONTROL", desktop_style::Palette::of(ui.ctx()).control),
                    ("LINE", desktop_style::Palette::of(ui.ctx()).line),
                ] {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(42., 32.), egui::Sense::hover());
                    ui.painter().rect(
                        r,
                        0,
                        color,
                        Stroke::new(1.0_f32, desktop_style::Palette::of(ui.ctx()).line),
                        egui::StrokeKind::Inside,
                    );
                    ui.label(label);
                }
            });
            ui.add_space(12.);
            ui.label("TYPOGRAPHY / Geist UI · Cascadia Mono JSON/Tree · một cỡ chữ");
            ui.label("SHAPE / radius 0 · border 1 px · không gradient · không shadow");
            ui.label("SPACING / 4, 8, 12, 18, 24 · phân nhóm bằng đường kẻ và khoảng trắng");
        });
        ui.add_space(18.);
        card(ui.ctx()).show(ui, |ui| {
            ui.label("WIDGET INVENTORY / ĐỐI CHIẾU APP HIỆN TẠI");
            ui.label("Mỗi mục có mẫu tương tác trong tab tương ứng. Đây là bước duyệt bộ style.");
            ui.add_space(8.);
            for item in INVENTORY {
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    if button(ui, item.category, false).clicked() {
                        self.section = CATEGORIES
                            .iter()
                            .position(|c| *c == item.category)
                            .unwrap_or(0);
                    }
                    ui.label(item.name);
                });
                ui.label(
                    egui::RichText::new(item.source)
                        .color(desktop_style::Palette::of(ui.ctx()).muted),
                );
            }
        });
        probe.support_text = INVENTORY
            .iter()
            .map(|i| i.name)
            .collect::<Vec<_>>()
            .join("\n");
    }
    fn controls(&mut self, ui: &mut egui::Ui, settings: &mut AppSettings, probe: &mut UiProbe) {
        heading(
            ui,
            "02 / CONTROLS",
            "Nút, input và thiết lập",
            "Giữ trạng thái rõ ràng bằng mực, nền và đường viền; góc vuông ở mọi biến thể.",
        );
        card(ui.ctx()).show(ui, |ui| {
            ui.label("BUTTONS / ACTIONS");
            ui.horizontal_wrapped(|ui| {
                if probe
                    .record("vintage-primary", button(ui, "Kết nối", true))
                    .clicked()
                {
                    self.toast = true;
                }
                button(ui, "Outline", false);
                probe.record(
                    "vintage-quiet",
                    hover_action(ui, |ui, _| {
                        ui.add(egui::Button::new("Quiet action").frame(false))
                    }),
                );
                probe.record(
                    "vintage-disabled",
                    hover_action(ui, |ui, _| {
                        ui.add_enabled(false, egui::Button::new("Disabled").corner_radius(0))
                    }),
                );
                if probe
                    .record("vintage-copy", icon(ui, 0xef7f, "Copy mẫu"))
                    .clicked()
                {
                    ui.ctx().copy_text(self.value.clone());
                }
                icon(ui, 0xe5d5, "Refresh");
                icon(ui, 0xe8b8, "Settings");
            });
            ui.separator();
            ui.label("INPUT / DEFAULT · FOCUS · SEARCH · MULTILINE");
            probe.record(
                "vintage-input",
                input(ui, &mut self.value, "VM Service URL"),
            );
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = desktop_style::ICON_TEXT_GAP;
                ui.label("⌕");
                probe.record(
                    "vintage-search",
                    input(ui, &mut self.search, "Tìm URL, method, status…"),
                );
                if button(ui, "×", false).clicked() {
                    self.search.clear();
                }
            });
            let multiline = desktop_style::framed_text_edit(
                ui,
                egui::TextEdit::multiline(&mut self.draft)
                    .desired_rows(3)
                    .desired_width(f32::INFINITY)
                    .font(egui::FontId::proportional(desktop_style::font_size(
                        ui.ctx(),
                    )))
                    .margin(8.),
                None,
            );
            probe.record("vintage-multiline", multiline);
        });
        ui.add_space(16.);
        let r = card(ui.ctx()).show(ui, |ui| {
            ui.label("INTERFACE / SETTINGS");
            ui.horizontal_wrapped(|ui| {
                ui.label("Theme");
                let r = fixed_action(ui, |ui| {
                    egui::ComboBox::from_id_salt("vintage-theme")
                        .selected_text(["Vintage paper", "Vintage ink"][self.choice])
                        .popup_style(egui::style::StyleModifier::from(
                            |style: &mut egui::Style| {
                                style.spacing.menu_margin = egui::Margin::ZERO;
                            },
                        ))
                        .show_ui(ui, |ui| {
                            ui.set_style(ui.ctx().style());
                            ui.spacing_mut().item_spacing.y = 0.;
                            for (i, name) in ["Vintage paper", "Vintage ink"].iter().enumerate() {
                                probe.record(
                                    format!("vintage-theme-option:{i}"),
                                    dropdown_option(ui, &mut self.choice, i, name),
                                );
                            }
                        })
                        .response
                });
                probe.record("vintage-dropdown", r);
                ui.label("Dropdown state demo; palette preview stays paper.");
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Cỡ chữ toàn app");
                let mut size = settings.values().font_size;
                font_size_stepper(
                    ui,
                    &mut size,
                    probe,
                    [
                        "vintage-font-minus",
                        "vintage-font-value",
                        "vintage-font-plus",
                    ],
                );
                if size != settings.values().font_size
                    && let Err(e) = settings.set_font_size(size)
                {
                    ui.label(e.to_string());
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("MCP / capture toggle");
                probe.record("vintage-switch", toggle(ui, &mut self.enabled));
                probe.record(
                    "vintage-checkbox",
                    hover_action(ui, |ui, _| ui.checkbox(&mut self.check, "Checkbox")),
                );
                probe.record(
                    "vintage-radio:0",
                    hover_action(ui, |ui, _| ui.radio_value(&mut self.radio, 0, "All")),
                );
                probe.record(
                    "vintage-radio:1",
                    hover_action(ui, |ui, _| ui.radio_value(&mut self.radio, 1, "Selected")),
                );
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Validation sample");
                ui.colored_label(
                    if ui.visuals().dark_mode {
                        crate::app_colors::NOTICE_DARK[3]
                    } else {
                        crate::app_colors::NOTICE_LIGHT[3]
                    },
                    "URL không hợp lệ · kiểm tra lại địa chỉ",
                );
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Read-only / selectable");
                ui.add(egui::Label::new("http://127.0.0.1:38943/mcp").selectable(true));
            });
        });
        probe
            .widgets
            .insert("vintage-settings".into(), r.response.rect);
    }
    fn navigation(&mut self, ui: &mut egui::Ui, probe: &mut UiProbe) {
        heading(
            ui,
            "03 / NAVIGATION",
            "Tab vuông, nhóm chọn và toolbar",
            "Cùng bộ token cho header, project selector, inspector và footer.",
        );
        card(ui.ctx()).show(ui, |ui| {
            ui.label("PAGE TABS");
            tabs(
                ui,
                probe,
                "vintage-tab",
                &["Network", "Console", "JSON"],
                &mut self.tab,
            );
            ui.separator();
            ui.label("PROJECT SELECTOR");
            tabs(
                ui,
                probe,
                "vintage-project",
                &["All", "pgapp", "staging"],
                &mut self.project,
            );
            ui.separator();
            ui.label("INSPECTOR TABS");
            tabs(
                ui,
                probe,
                "vintage-detail",
                &[
                    "Overview",
                    "Request header",
                    "Request body",
                    "Response header",
                    "Response body",
                    "Raw",
                ],
                &mut self.detail,
            );
            ui.separator();
            ui.label("CONTEXT TOOLBAR");
            ui.horizontal_wrapped(|ui| {
                if probe
                    .record(
                        "vintage-pause",
                        button(
                            ui,
                            if self.paused {
                                "▶ Resume"
                            } else {
                                "Ⅱ Pause"
                            },
                            self.paused,
                        ),
                    )
                    .clicked()
                {
                    self.paused = !self.paused;
                }
                if button(ui, "Clear", false).clicked() {
                    self.toast = true;
                }
                icon(ui, 0xef7f, "Copy");
                icon(ui, 0xf090, "Download");
                icon(ui, 0xe567, "Find");
                ui.label("Format / Raw / JSON / Tree");
            });
            ui.separator();
            probe.record(
                "vintage-link",
                hover_action(ui, |ui, _| {
                    ui.hyperlink_to(
                        "GitHub Discussions ↗",
                        crate::desktop_support::DISCUSSION_URL,
                    )
                }),
            );
            ui.horizontal_wrapped(|ui| {
                button(ui, "← Back", false);
                ui.label("Network / request / response");
            });
        });
        ui.add_space(16.);
        let r = card(ui.ctx()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("● 1 connection");
                ui.label("128 visible · 0 evicted · 0 skipped");
                ui.label("RAM 135 MB");
            });
        });
        probe
            .widgets
            .insert("vintage-footer".into(), r.response.rect);
    }
    fn tables(&mut self, ui: &mut egui::Ui, probe: &mut UiProbe) {
        heading(
            ui,
            "04 / DATA DISPLAY",
            "Bảng dữ liệu & inspector",
            "Method và status là chữ thuần; hàng được chọn có nền mực. Header cố định khi cuộn.",
        );
        card(ui.ctx()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label("NETWORK");
                let r = fixed_action(ui, |ui| {
                    egui::ComboBox::from_id_salt("vintage-filter")
                        .selected_text(["All methods", "GET", "POST"][self.filter])
                        .popup_style(egui::style::StyleModifier::from(
                            |style: &mut egui::Style| {
                                style.spacing.menu_margin = egui::Margin::ZERO;
                            },
                        ))
                        .show_ui(ui, |ui| {
                            ui.set_style(ui.ctx().style());
                            ui.spacing_mut().item_spacing.y = 0.;
                            for (i, v) in ["All methods", "GET", "POST"].iter().enumerate() {
                                probe.record(
                                    format!("vintage-filter-option:{i}"),
                                    dropdown_option(ui, &mut self.filter, i, v),
                                );
                            }
                        })
                        .response
                });
                probe.record("vintage-table-filter", r);
            });
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.;
                ui.set_min_width(640.);
                let widths = [80., ui.available_width() - 270., 90., 100.];
                let (header, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 36.),
                    egui::Sense::hover(),
                );
                probe.widgets.insert("vintage-table-header".into(), header);
                paint_row(
                    ui,
                    header,
                    &["Method", "URL", "Status", "Time"],
                    &widths,
                    false,
                    true,
                );
                let rows = egui::ScrollArea::vertical()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .max_height(185.)
                    .id_salt("vintage-table-rows")
                    .show(ui, |ui| {
                        for i in 0..20 {
                            let method = if i % 3 == 0 { "POST" } else { "GET" };
                            if self.filter == 1 && method != "GET"
                                || self.filter == 2 && method != "POST"
                            {
                                continue;
                            }
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 38.),
                                egui::Sense::click(),
                            );
                            paint_row(
                                ui,
                                rect,
                                &[
                                    method,
                                    if i % 2 == 0 {
                                        "/api/v1/user/account-info"
                                    } else {
                                        "/api/v1/notifications"
                                    },
                                    if i == 4 { "404" } else { "200" },
                                    "128 ms",
                                ],
                                &widths,
                                self.selected_row == i,
                                false,
                            );
                            if probe
                                .record(
                                    format!("vintage-row:{i}"),
                                    response.on_hover_cursor(egui::CursorIcon::PointingHand),
                                )
                                .clicked()
                            {
                                self.selected_row = i;
                            }
                        }
                    });
                let grid = egui::Rect::from_min_max(
                    header.min,
                    egui::pos2(header.right(), rows.inner_rect.bottom()),
                );
                ui.painter().rect_stroke(
                    grid,
                    0,
                    Stroke::new(1.0_f32, desktop_style::Palette::of(ui.ctx()).line),
                    egui::StrokeKind::Inside,
                );
                probe.widgets.insert("vintage-table-grid".into(), grid);
            });
        });
        ui.add_space(16.);
        let metadata = card(ui.ctx()).show(ui, |ui| {
            ui.label("OVERVIEW / KEY–VALUE / REQUEST HEADER");
            key_value_table(
                ui,
                probe,
                "vintage-metadata-table",
                &[
                    ("URL", "https://example.com/api/v1/account-info"),
                    ("Method", "GET"),
                    ("Status", "200 OK"),
                    ("Content-Type", "application/json"),
                    ("Elapsed", "128 ms"),
                ],
                false,
            );
        });
        probe
            .widgets
            .insert("vintage-metadata".into(), metadata.response.rect);
        ui.add_space(16.);
        let console = card(ui.ctx()).show(ui, |ui| {
            ui.label("CONSOLE");
            egui::ScrollArea::horizontal()
                .id_salt("vintage-console-horizontal")
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.;
                    let size = desktop_style::font_size(ui.ctx());
                    let source_width = (size * 7. + 16.).ceil();
                    let level_width = (size * 5. + 16.).ceil();
                    let time_width = (size * 5. + 32.).ceil();
                    let fixed = source_width + level_width + time_width;
                    ui.set_min_width((fixed + 320.).max(640.));
                    let widths = [
                        source_width,
                        level_width,
                        time_width,
                        ui.available_width() - fixed,
                    ];
                    let row_height = (size + 20.).max(38.);
                    let (header, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), row_height),
                        egui::Sense::hover(),
                    );
                    paint_row(
                        ui,
                        header,
                        &["Source", "Level", "Time", "Message"],
                        &widths,
                        false,
                        true,
                    );
                    probe
                        .widgets
                        .insert("vintage-console-header".into(), header);
                    let mut x = header.left();
                    for (i, width) in widths.iter().enumerate() {
                        probe.widgets.insert(
                            format!("vintage-console-head:{i}"),
                            egui::Rect::from_min_size(
                                egui::pos2(x, header.top()),
                                egui::vec2(*width, row_height),
                            ),
                        );
                        x += width;
                    }
                    let entries = [
                        ["stdout", "INFO", "10:06:20", "Connected to pgapp"],
                        ["logger", "WARN", "10:06:21", "Slow response: 980 ms"],
                        ["stderr", "ERROR", "10:06:22", "Sample exception"],
                    ];
                    let rows = egui::ScrollArea::vertical()
                        .id_salt("vintage-console-rows")
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .max_height(160.)
                        .show(ui, |ui| {
                            let mut clip = ui.clip_rect();
                            clip.min.y = clip.min.y.max(header.bottom());
                            ui.set_clip_rect(clip);
                            for i in 0..12 {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::vec2(ui.available_width(), row_height),
                                    egui::Sense::click(),
                                );
                                let values = &entries[i % entries.len()];
                                paint_row(
                                    ui,
                                    rect,
                                    values,
                                    &widths,
                                    self.selected_console_row == i,
                                    false,
                                );
                                if probe
                                    .record(
                                        format!("vintage-console-row:{i}"),
                                        response.on_hover_cursor(egui::CursorIcon::PointingHand),
                                    )
                                    .clicked()
                                {
                                    self.selected_console_row = i;
                                }
                            }
                        });
                    let grid = egui::Rect::from_min_max(
                        header.min,
                        egui::pos2(header.right(), rows.inner_rect.bottom()),
                    );
                    ui.painter().rect_stroke(
                        grid,
                        0,
                        Stroke::new(1.0_f32, desktop_style::Palette::of(ui.ctx()).line),
                        egui::StrokeKind::Inside,
                    );
                    probe.widgets.insert("vintage-console-grid".into(), grid);
                });
        });
        probe
            .widgets
            .insert("vintage-console".into(), console.response.rect);
        ui.add_space(12.);
        ui.label("RESIZABLE SPLIT / kéo đường chia");
        let w = ui.available_width();
        let row_h = 64.;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(w, row_h), egui::Sense::hover());
        let x = rect.left() + w * self.split;
        let divider =
            egui::Rect::from_center_size(egui::pos2(x, rect.center().y), egui::vec2(8., row_h));
        let response = ui
            .interact(
                divider,
                ui.id().with("vintage-splitter"),
                egui::Sense::drag(),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        if response.dragged() {
            self.split =
                ((response.interact_pointer_pos().unwrap().x - rect.left()) / w).clamp(0.2, 0.8);
        }
        ui.painter().rect_stroke(
            rect,
            0,
            Stroke::new(1.0_f32, desktop_style::Palette::of(ui.ctx()).line),
            egui::StrokeKind::Inside,
        );
        ui.painter().vline(
            x,
            rect.y_range(),
            Stroke::new(1.0_f32, desktop_style::Palette::of(ui.ctx()).text),
        );
        let font = egui::FontId::proportional(desktop_style::font_size(ui.ctx()));
        ui.painter().text(
            rect.left_center() + egui::vec2(12., 0.),
            egui::Align2::LEFT_CENTER,
            "List",
            font.clone(),
            desktop_style::Palette::of(ui.ctx()).text,
        );
        ui.painter().text(
            egui::pos2(x + 12., rect.center().y),
            egui::Align2::LEFT_CENTER,
            "Inspector",
            font,
            desktop_style::Palette::of(ui.ctx()).text,
        );
        probe.record("vintage-splitter", response);
    }
    fn json(&mut self, ui: &mut egui::Ui, probe: &mut UiProbe) {
        heading(
            ui,
            "05 / JSON & CODE",
            "Giữ đủ 7 vai trò màu cú pháp",
            "Key xanh lam, string xanh lá, number đất nung, boolean tím, collection đỏ đất, null xám, punctuation màu mực.",
        );
        self.code_view.load(JSON);
        self.tree_view.load(JSON);
        self.tree_view.set_tree(true);
        let surface = card(ui.ctx())
            .fill(desktop_style::Palette::of(ui.ctx()).background)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if probe
                        .record(
                            "vintage-json-fold",
                            button(
                                ui,
                                if self.folded {
                                    "Expand all"
                                } else {
                                    "Fold all"
                                },
                                false,
                            ),
                        )
                        .clicked()
                    {
                        self.folded = !self.folded;
                        self.code_view.fold_all(self.folded);
                        self.tree_view.fold_all(self.folded);
                    }
                    if probe
                        .record("vintage-json-copy", button(ui, "Copy JSON", false))
                        .clicked()
                    {
                        ui.ctx().copy_text(JSON.into());
                    }
                });
                for (name, viewer) in [("code", &mut self.code_view), ("tree", &mut self.tree_view)]
                {
                    ui.label(if name == "code" { "CODE" } else { "TREE" });
                    let mut local = UiProbe::default();
                    let r = ui.push_id(name, |ui| {
                        ui.allocate_ui(egui::vec2(ui.available_width(), 360.), |ui| {
                            ui.set_max_height(360.);
                            viewer.show(ui, &mut local);
                        });
                    });
                    probe
                        .widgets
                        .insert(format!("vintage-json-{name}"), r.response.rect);
                    if name == "code"
                        && let Some(rect) = local.widgets.get("json-find")
                    {
                        probe.widgets.insert("vintage-json-find".into(), *rect);
                    }
                    for (key, rect) in local.widgets {
                        probe.widgets.insert(format!("vintage-{name}:{key}"), rect);
                    }
                }
                ui.horizontal_wrapped(|ui| {
                    for (i, name) in [
                        "Punctuation",
                        "Key",
                        "String",
                        "Number",
                        "Boolean",
                        "Collection",
                        "Null",
                    ]
                    .iter()
                    .enumerate()
                    {
                        ui.colored_label(
                            if ui.visuals().dark_mode {
                                crate::app_colors::JSON_DARK[i]
                            } else {
                                crate::app_colors::JSON_LIGHT[i]
                            },
                            *name,
                        );
                    }
                });
                if probe
                    .record("vintage-swatch", button(ui, "#52634B · Copy color", false))
                    .clicked()
                {
                    ui.ctx().copy_text("#52634B".into());
                }
            });
        probe
            .widgets
            .insert("vintage-json-surface".into(), surface.response.rect);
    }
    fn bodies(&mut self, ui: &mut egui::Ui, probe: &mut UiProbe) {
        heading(
            ui,
            "06 / BODY PREVIEWS",
            "Form, file, ảnh và raw",
            "Ảnh và swatch giữ màu của nội dung; nền, đường viền và thao tác dùng chung bộ style.",
        );
        card(ui.ctx()).show(ui,|ui|{
            tabs(ui,probe,"vintage-body",&["Form","Image","Raw"],&mut self.body);
            ui.separator();
            match self.body {
                0 => {
                    let grid = key_value_table(ui,probe,"vintage-form-table",&[
                        ("name","Nguyễn Quốc An"), ("email","demo@example.com"),
                        ("attachment","sample.png · image/png · 24 KB"),
                    ],true);
                    probe.widgets.insert("vintage-form".into(),grid);
                },
                1=>{ui.horizontal_wrapped(|ui|{if button(ui,"−",false).clicked(){self.zoom=(self.zoom-0.25).max(0.25);}ui.label(format!("{:.0}%",self.zoom*100.));if button(ui,"+",false).clicked(){self.zoom=(self.zoom+0.25).min(3.);}
if button(ui,"Fit",false).clicked(){self.zoom=1.;}
if probe.record("vintage-image-open",button(ui,"Enlarge ↗",false)).clicked(){self.image_open=true;}});self.image(ui,180.);},
                _=>{ui.add(egui::Label::new("POST /api/v1/upload\nContent-Type: multipart/form-data\n\nname=Nguyễn Quốc An&file=sample.png").selectable(true));}
            }
            ui.separator();ui.colored_label(desktop_style::Palette::of(ui.ctx()).muted,"Body omitted: capture size limit · 1 MB / EN–VI notice");
        });
    }
    fn image(&self, ui: &mut egui::Ui, height: f32) {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width().min(580.), height),
            egui::Sense::hover(),
        );
        ui.painter()
            .rect_filled(rect, 0, crate::app_colors::PREVIEW_BACKGROUND);
        let p = ui.painter().with_clip_rect(rect);
        let center = rect.center();
        let scale = self.zoom;
        for i in 0..13 {
            let x = center.x + (i as f32 - 6.) * 24. * scale;
            p.line_segment(
                [
                    egui::pos2(x, rect.bottom()),
                    egui::pos2(x, rect.top() + 30. + (i % 3) as f32 * 20.),
                ],
                Stroke::new(1.0_f32, desktop_style::Palette::of(ui.ctx()).text),
            );
        }
        p.line_segment(
            [rect.left_bottom(), rect.right_top()],
            Stroke::new(2.0_f32, desktop_style::Palette::of(ui.ctx()).text),
        );
        p.text(
            rect.left_top() + egui::vec2(12., 12.),
            egui::Align2::LEFT_TOP,
            "SAMPLE.PNG / 640 × 360",
            egui::FontId::proportional(desktop_style::font_size(ui.ctx())),
            desktop_style::Palette::of(ui.ctx()).text,
        );
    }
    fn feedback(&mut self, ui: &mut egui::Ui, probe: &mut UiProbe) {
        heading(
            ui,
            "07 / FEEDBACK",
            "Thông báo, popup và trạng thái",
            "Giữ biểu tượng và nhãn rõ ràng; trạng thái hiển thị bằng chữ và đường kẻ.",
        );
        card(ui.ctx()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (i, name) in ["Information", "Success", "Warning", "Error"]
                    .iter()
                    .enumerate()
                {
                    if button(ui, name, self.notice_kind == i).clicked() {
                        self.notice_kind = i;
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                if probe
                    .record("vintage-toast-open", button(ui, "Hiện thông báo", true))
                    .clicked()
                {
                    self.toast = true;
                }
                if probe
                    .record(
                        "vintage-dialog-open",
                        button(ui, "Mở dialog / confirmation", false),
                    )
                    .clicked()
                {
                    self.dialog = true;
                }
            });
            ui.separator();
            ui.label("PROGRESS / LOADING / RETRY");
            probe.record(
                "vintage-progress",
                ui.add(
                    egui::ProgressBar::new(0.68)
                        .text("Downloading · 68%")
                        .corner_radius(0),
                ),
            );
            ui.horizontal_wrapped(|ui| {
                let loader = loading(ui, 5);
                probe.record("vintage-loading", loader.response);
                ui.label("Đang quét project…");
                if button(ui, "Retry", false).clicked() {
                    self.toast = true;
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label("Compact / 3 ô");
                let compact = loading(ui, 3);
                probe.record("vintage-loading-compact", compact.response);
                ui.label("Wide / 10 ô");
                let wide = loading(ui, 10);
                probe.record("vintage-loading-wide", wide.response);
            });
        });
        ui.add_space(16.);
        let empty = card(ui.ctx()).show(ui, |ui| {
            ui.label("EMPTY / NO RESULTS");
            ui.label(empty_text(ui.ctx()));
            ui.separator();
            ui.label(empty_text(ui.ctx()));
        });
        probe
            .widgets
            .insert("vintage-empty".into(), empty.response.rect);
        ui.add_space(16.);
        let help=card(ui.ctx()).show(ui,|ui|{ui.label("HELP / HƯỚNG DẪN");ui.label("01  Find & connect\n02  Chọn project hoặc nhập Manual URL\n03  Lọc request và kiểm tra response\n04  Copy / Export dữ liệu");ui.horizontal_wrapped(|ui|{hover_action(ui, |ui, _| ui.hyperlink_to("Discussion ↗",crate::desktop_support::DISCUSSION_URL));if button(ui,"Copy guide",false).clicked(){ui.ctx().copy_text("Scan → Select project → Inspect → Export".into());}});});
        probe
            .widgets
            .insert("vintage-help".into(), help.response.rect);
    }
}
pub(crate) fn paint_row(
    ui: &egui::Ui,
    rect: egui::Rect,
    values: &[&str],
    widths: &[f32],
    selected: bool,
    header: bool,
) {
    let painter = ui.painter();
    let palette = desktop_style::Palette::of(ui.ctx());
    painter.rect_filled(
        rect,
        0,
        if selected {
            palette.accent
        } else if header {
            palette.background
        } else {
            palette.surface
        },
    );
    let fg = if selected {
        palette.on_accent
    } else {
        palette.text
    };
    let font = egui::FontId::proportional(desktop_style::font_size(ui.ctx()));
    let mut x = rect.left();
    for (i, value) in values.iter().enumerate() {
        let cell = egui::Rect::from_min_size(
            egui::pos2(x, rect.top()),
            egui::vec2(widths[i], rect.height()),
        );
        painter
            .with_clip_rect(cell.shrink2(egui::vec2(6., 0.)).intersect(ui.clip_rect()))
            .text(
                cell.left_center() + egui::vec2(8., 0.),
                egui::Align2::LEFT_CENTER,
                *value,
                font.clone(),
                fg,
            );
        x += widths[i];
        painter.vline(x, rect.y_range(), Stroke::new(1.0_f32, palette.line));
    }
    painter.vline(
        rect.left(),
        rect.y_range(),
        Stroke::new(1.0_f32, palette.line),
    );
    painter.hline(
        rect.x_range(),
        rect.top(),
        Stroke::new(1.0_f32, palette.line),
    );
    painter.hline(
        rect.x_range(),
        rect.bottom(),
        Stroke::new(1.0_f32, palette.line),
    );
}

/// Fixed title width; narrow panes may shrink it to retain space for values.
pub const KEY_VALUE_TITLE_WIDTH: f32 = 160.;

/// Shared metadata/header/form table. Values wrap; only the enclosing pane scrolls vertically.
pub fn key_value_table(
    ui: &mut egui::Ui,
    probe: &mut UiProbe,
    id: &str,
    entries: &[(&str, &str)],
    copy: bool,
) -> egui::Rect {
    key_value_table_with_query(ui, probe, id, entries, copy, None)
}

/// Query highlighting changes text formats only; every source character is retained.
fn query_value_job(
    ui: &egui::Ui,
    raw: &str,
    font: &egui::FontId,
    width: f32,
) -> egui::text::LayoutJob {
    let colors = if ui.visuals().dark_mode {
        crate::app_colors::JSON_DARK
    } else {
        crate::app_colors::JSON_LIGHT
    };
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = width.max(1.);
    job.wrap.break_anywhere = true;
    let mut append = |text: &str, color| {
        job.append(
            text,
            0.,
            egui::TextFormat {
                font_id: font.clone(),
                color,
                ..Default::default()
            },
        );
    };
    for (index, pair) in raw.split('&').enumerate() {
        if index > 0 {
            append("&", colors[0]);
        }
        if let Some((key, value)) = pair.split_once('=') {
            append(key, colors[1]);
            append("=", colors[0]);
            let role = match value {
                "true" | "false" => 4,
                "null" => 6,
                _ if value.parse::<serde_json::Number>().is_ok() => 3,
                _ => 2,
            };
            append(value, colors[role]);
        } else {
            append(pair, colors[1]);
        }
    }
    job
}

pub(crate) fn key_value_table_with_query(
    ui: &mut egui::Ui,
    probe: &mut UiProbe,
    id: &str,
    entries: &[(&str, &str)],
    copy: bool,
    query_row: Option<usize>,
) -> egui::Rect {
    let table_width = ui
        .available_width()
        .min(ui.clip_rect().right() - ui.next_widget_position().x)
        .max(1.);
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 0.;
        let palette = desktop_style::Palette::of(ui.ctx());
        let font = egui::FontId::proportional(desktop_style::font_size(ui.ctx()));
        let action_width = if copy {
            64_f32.min(table_width * 0.3)
        } else {
            0.
        };
        let title_width = KEY_VALUE_TITLE_WIDTH.min((table_width - action_width - 48.).max(0.));
        let mut widths = vec![title_width, table_width - title_width - action_width];
        let mut headings = vec!["Title", "Value"];
        if copy {
            widths.push(action_width);
            headings.push("Action");
        }
        let min_height = (font.size + 20.).max(38.);
        let layout_text = |ui: &egui::Ui, text: &str, width: f32, align: egui::Align| {
            let mut job = egui::text::LayoutJob::simple(
                text.to_owned(),
                font.clone(),
                palette.text,
                (width - 16.).max(1.),
            );
            job.halign = align;
            job.wrap.break_anywhere = true;
            ui.fonts_mut(|f| f.layout_job(job))
        };
        let heading_galleys: Vec<_> = headings
            .iter()
            .enumerate()
            .map(|(column, text)| {
                layout_text(
                    ui,
                    text,
                    widths[column],
                    if column == 2 {
                        egui::Align::Center
                    } else {
                        egui::Align::Min
                    },
                )
            })
            .collect();
        let header_height = heading_galleys.iter().fold(min_height, |height, galley| {
            height.max(galley.size().y + 8.)
        });
        let (header, _) =
            ui.allocate_exact_size(egui::vec2(table_width, header_height), egui::Sense::hover());
        paint_row(ui, header, &[""; 3][..widths.len()], &widths, false, true);
        let mut x = header.left();
        for (column, galley) in heading_galleys.into_iter().enumerate() {
            let cell = egui::Rect::from_min_size(
                egui::pos2(x, header.top()),
                egui::vec2(widths[column], header.height()),
            );
            let mut cell_ui = ui.new_child(
                egui::UiBuilder::new()
                    .id_salt((id, "heading", column))
                    .max_rect(cell.shrink2(egui::vec2(8., 4.)))
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            cell_ui.add(egui::Label::new(galley).selectable(false));
            probe.widgets.insert(format!("{id}:head:{column}"), cell);
            x += widths[column];
        }
        probe.widgets.insert(format!("{id}:header"), header);
        let mut grid = header;
        for (i, (title, value)) in entries.iter().enumerate() {
            let title_galley = layout_text(ui, title, title_width, egui::Align::Min);
            let value_galley = if query_row == Some(i) {
                let job = query_value_job(ui, value, &font, widths[1] - 16.);
                ui.fonts_mut(|f| f.layout_job(job))
            } else {
                layout_text(ui, value, widths[1], egui::Align::Min)
            };
            let row_height = min_height
                .max(title_galley.size().y + 8.)
                .max(value_galley.size().y + 8.);
            let (row, _) =
                ui.allocate_exact_size(egui::vec2(table_width, row_height), egui::Sense::hover());
            paint_row(ui, row, &[""; 3][..widths.len()], &widths, false, false);
            let mut x = row.left();
            for (column, width) in widths.iter().enumerate() {
                let cell = egui::Rect::from_min_size(
                    egui::pos2(x, row.top()),
                    egui::vec2(*width, row_height),
                );
                probe
                    .widgets
                    .insert(format!("{id}:cell:{i}:{column}"), cell);
                let side = desktop_style::ICON_BUTTON_SIDE;
                let mut cell_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt((id, i, column))
                        .max_rect(if column == 2 {
                            egui::Rect::from_center_size(cell.center(), egui::Vec2::splat(side))
                        } else {
                            cell.shrink2(egui::vec2(8., 4.))
                        })
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                match column {
                    0 => {
                        cell_ui.add(egui::Label::new(title_galley.clone()).selectable(false));
                    }
                    1 => {
                        cell_ui.add(egui::Label::new(value_galley.clone()).selectable(true));
                    }
                    _ => {
                        let response =
                            desktop_style::square_icon(&mut cell_ui, 0xef7f, "Copy value");
                        if probe.record(format!("{id}:copy:{i}"), response).clicked() {
                            ui.ctx().copy_text((*value).into());
                        }
                    }
                }
                x += width;
            }
            grid = grid.union(row);
            probe.widgets.insert(format!("{id}:row:{i}"), row);
        }
        ui.painter().rect_stroke(
            grid,
            0,
            Stroke::new(1.0_f32, palette.line),
            egui::StrokeKind::Inside,
        );
        probe.widgets.insert(format!("{id}:grid"), grid);
        grid
    })
    .inner
}
