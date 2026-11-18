//! Visual tokens and bundled Google Material Icons (https://fonts.google.com/icons).
use eframe::egui::{self, Color32, FontId, Response};
pub const ICON_TEXT_GAP: f32 = 4.;
pub const BUTTON_RADIUS: u8 = 4;
pub(crate) const ICON_BUTTON_SIDE: f32 = 24.;
pub(crate) const ICON_BUTTON_GLYPH_SIZE: f32 = 14.;

#[derive(Clone, Copy)]
pub struct Palette {
    pub background: Color32,
    pub surface: Color32,
    pub control: Color32,
    pub muted: Color32,
    pub accent: Color32,
    pub line: Color32,
    pub text: Color32,
    pub on_accent: Color32,
}
impl Palette {
    pub fn new(dark: bool) -> Self {
        if dark {
            crate::app_colors::DARK
        } else {
            crate::app_colors::LIGHT
        }
    }
    pub fn of(ctx: &egui::Context) -> Self {
        Self::new(ctx.style().visuals.dark_mode)
    }
}
pub fn glyph(code: u32, size: f32, color: Color32) -> egui::RichText {
    egui::RichText::new(char::from_u32(code).unwrap().to_string())
        .font(FontId::new(
            size,
            egui::FontFamily::Name("MaterialIcons".into()),
        ))
        .color(color)
}
pub fn paint_icon(painter: &egui::Painter, pos: egui::Pos2, code: u32, size: f32, color: Color32) {
    let icon = painter.layout_no_wrap(
        char::from_u32(code).unwrap().to_string(),
        FontId::new(size, egui::FontFamily::Name("MaterialIcons".into())),
        color,
    );
    // The actual glyph can have different bearings on each side of its font box.
    let ink_center = if icon.mesh_bounds.is_positive() {
        icon.mesh_bounds.center()
    } else {
        (icon.size() / 2.).to_pos2()
    };
    painter.galley(pos - ink_center.to_vec2(), icon, color);
}
pub fn icon_button(ui: &mut egui::Ui, code: u32, tooltip: &str, color: Color32) -> Response {
    let _ = color;
    crate::vintage::icon(ui, code, tooltip)
}
pub fn pill(
    ui: &mut egui::Ui,
    label: &str,
    icon: Option<u32>,
    selected: bool,
    width: f32,
) -> Response {
    pill_with_size(ui, label, icon, selected, width, 13.)
}
pub fn pill_with_size(
    ui: &mut egui::Ui,
    label: &str,
    icon: Option<u32>,
    selected: bool,
    width: f32,
    _font_size: f32,
) -> Response {
    crate::vintage::hover_action(ui, |ui, factor| {
        let font_size = local_font_size(ui);
        let p = Palette::of(ui.ctx());
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(width * factor, 32. * factor),
            egui::Sense::click(),
        );
        if selected {
            ui.painter().rect_filled(rect, BUTTON_RADIUS, p.accent);
        }
        let color = if selected { p.on_accent } else { p.text };
        let center = rect.center();
        let text_width = ui
            .painter()
            .layout_no_wrap(
                label.into(),
                if selected {
                    semibold(font_size)
                } else {
                    FontId::proportional(font_size)
                },
                color,
            )
            .size()
            .x;
        let icon_side = 17. * factor;
        let total = text_width
            + if icon.is_some() {
                icon_side + ICON_TEXT_GAP
            } else {
                0.
            };
        let mut x = center.x - total / 2.;
        if let Some(code) = icon {
            paint_icon(
                ui.painter(),
                egui::pos2(x + icon_side / 2., center.y),
                code,
                icon_side,
                color,
            );
            x += icon_side + ICON_TEXT_GAP;
        }
        ui.painter().text(
            egui::pos2(x, center.y),
            egui::Align2::LEFT_CENTER,
            label,
            if selected {
                semibold(font_size)
            } else {
                FontId::proportional(font_size)
            },
            color,
        );
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                ui.is_enabled(),
                selected,
                label,
            )
        });
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    })
}

/// UI text uses Geist; JSON/Tree content keeps the Cascadia Mono family.
pub fn semibold(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name("Geist".into()))
}
pub fn medium(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name("Geist".into()))
}
pub fn bold(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name("Geist".into()))
}
pub fn mono(ctx: &egui::Context, text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text)
        .font(FontId::proportional(font_size(ctx)))
        .line_height(Some(font_size(ctx) + 8.))
}

/// Google Material Icons content_copy. Keep the tooltip and accessible name
/// while displaying only the glyph, including the compact JSON toolbar.
pub fn copy_button(ui: &mut egui::Ui, tooltip: &str) -> Response {
    crate::vintage::icon(ui, 0xef7f, tooltip)
}

/// Shared by native TextEdit frames and composite inputs with a custom frame.
pub(crate) fn input_stroke(p: Palette, focused: bool) -> egui::Stroke {
    egui::Stroke::new(1.0_f32, if focused { p.accent } else { p.line })
}

/// egui uses the selection stroke for a focused TextEdit frame. Scope that
/// override to this field so selected menu items keep their own text color.
pub(crate) fn framed_text_edit(
    ui: &mut egui::Ui,
    edit: egui::TextEdit<'_>,
    size: Option<egui::Vec2>,
) -> Response {
    let stroke = input_stroke(Palette::of(ui.ctx()), true);
    let previous = ui.visuals().selection.stroke;
    ui.visuals_mut().selection.stroke = stroke;
    let response = match size {
        Some(size) => ui.add_sized(size, edit),
        None => ui.add(edit),
    };
    ui.visuals_mut().selection.stroke = previous;
    response
}

pub fn input_border(ui: &egui::Ui, rect: egui::Rect, focused: bool) {
    ui.painter().rect_stroke(
        rect,
        0,
        input_stroke(Palette::of(ui.ctx()), focused),
        egui::StrokeKind::Inside,
    );
}

/// One composite search field; callers own the width and clear behavior.
pub fn search_bar(
    ui: &mut egui::Ui,
    probe: &mut crate::desktop::UiProbe,
    key: &str,
    clear_key: &str,
    query: &mut String,
    hint: &str,
    width: f32,
) -> (Response, bool) {
    let p = Palette::of(ui.ctx());
    let font = FontId::proportional(font_size(ui.ctx()));
    let height = ui.fonts_mut(|f| f.row_height(&font).ceil());
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height + 8.), egui::Sense::hover());
    ui.painter().rect_filled(rect, BUTTON_RADIUS, p.control);
    let icon = egui::Rect::from_min_size(rect.min + egui::vec2(4., 4.), egui::Vec2::splat(height));
    paint_icon(ui.painter(), icon.center(), 0xe567, height, p.muted);
    let clear_rect = egui::Rect::from_center_size(
        rect.right_center() - egui::vec2(4. + height / 2., 0.),
        egui::Vec2::splat(height),
    );
    let field = egui::Rect::from_min_max(
        egui::pos2(icon.right() + 4., rect.top() + 4.),
        egui::pos2(clear_rect.left() - 4., rect.bottom() - 4.),
    );
    let input = ui
        .new_child(egui::UiBuilder::new().id_salt(key).max_rect(field))
        .add_sized(
            field.size(),
            egui::TextEdit::singleline(query)
                .id_salt(key)
                .font(font)
                .frame(false)
                .margin(egui::Margin::ZERO)
                .vertical_align(egui::Align::Center)
                .hint_text(egui::RichText::new(hint).color(p.muted)),
        );
    let cleared = if query.is_empty() {
        false
    } else {
        let clear = ui
            .interact(clear_rect, ui.id().with(clear_key), egui::Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        paint_icon(ui.painter(), clear_rect.center(), 0xe16a, height, p.muted);
        probe.record(clear_key, clear).clicked()
    };
    ui.painter().rect_stroke(
        rect,
        BUTTON_RADIUS,
        input_stroke(p, input.has_focus()),
        egui::StrokeKind::Inside,
    );
    probe.widgets.insert(format!("{key}-frame"), rect);
    probe.widgets.insert(format!("{key}-icon"), icon);
    (probe.record(key, input), cleared)
}

pub fn search_bar_height(ctx: &egui::Context) -> f32 {
    let font = FontId::proportional(font_size(ctx));
    ctx.fonts_mut(|f| f.row_height(&font).ceil()) + 8.
}

/// Single source for every text style, including painter-based tables/editors.
pub fn font_size(ctx: &egui::Context) -> f32 {
    ctx.style().text_styles[&egui::TextStyle::Body].size
}

/// Neutral buttons inherit a fill appropriate to their containing surface.
pub fn neutral_button_fill(ui: &egui::Ui) -> Color32 {
    ui.visuals().widgets.inactive.weak_bg_fill
}
pub fn style_neutral_buttons(ui: &mut egui::Ui, on_gray: bool) {
    let fill = match (ui.visuals().dark_mode, on_gray) {
        (false, false) => crate::app_colors::BUTTON_ON_WHITE_LIGHT,
        (false, true) => crate::app_colors::BUTTON_ON_GRAY_LIGHT,
        (true, false) => crate::app_colors::BUTTON_ON_WHITE_DARK,
        (true, true) => crate::app_colors::BUTTON_ON_GRAY_DARK,
    };
    let widgets = &mut ui.visuals_mut().widgets;
    for state in [
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
        &mut widgets.open,
    ] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
    }
}

/// Action widgets keep their normal dimensions and font size on hover.
pub fn button(ui: &mut egui::Ui, text: impl Into<egui::WidgetText>) -> Response {
    let text = text.into();
    crate::vintage::hover_action(ui, |ui, factor| {
        let text = match text {
            egui::WidgetText::RichText(t) => {
                egui::WidgetText::from((*t).clone().size(local_font_size(ui)))
            }
            egui::WidgetText::LayoutJob(mut job) => {
                for section in &mut std::sync::Arc::make_mut(&mut job).sections {
                    section.format.font_id.size *= factor;
                }
                egui::WidgetText::LayoutJob(job)
            }
            other => other,
        };
        ui.add(egui::Button::new(text).corner_radius(BUTTON_RADIUS))
    })
}
/// Menu options have no hover growth, occupy the full row, and own their padding.
pub fn option(ui: &mut egui::Ui, selected: bool, text: impl Into<egui::WidgetText>) -> Response {
    ui.spacing_mut().item_spacing.y = 0.;
    ui.spacing_mut().button_padding = egui::vec2(12., 8.);
    ui.add_sized(
        [
            ui.available_width(),
            ui.spacing().interact_size.y.max(font_size(ui.ctx()) + 16.),
        ],
        egui::Button::selectable(selected, text),
    )
    .on_hover_cursor(egui::CursorIcon::PointingHand)
}
pub fn option_value<T: PartialEq>(
    ui: &mut egui::Ui,
    current: &mut T,
    value: T,
    text: impl Into<egui::WidgetText>,
) -> Response {
    let mut response = option(ui, *current == value, text);
    if response.clicked() && *current != value {
        *current = value;
        response.mark_changed();
    }
    response
}
pub fn selectable_value<T: PartialEq>(
    ui: &mut egui::Ui,
    current: &mut T,
    value: T,
    text: impl Into<egui::WidgetText>,
) -> Response {
    let text = text.into();
    crate::vintage::hover_action(ui, |ui, _| ui.selectable_value(current, value, text))
}

pub fn local_font_size(ui: &egui::Ui) -> f32 {
    ui.style().text_styles[&egui::TextStyle::Body].size
}

pub fn square_icon(ui: &mut egui::Ui, code: u32, tooltip: &str) -> Response {
    icon_control(ui, code).on_hover_text(tooltip)
}

/// Layout primitive shared by glyph icons and short-label actions such as +/-.
/// Owns both axes even inside forms with a larger minimum interaction height.
fn square_action(
    ui: &mut egui::Ui,
    side: f32,
    draw: impl FnOnce(&mut egui::Ui, f32, egui::Vec2) -> Response,
) -> Response {
    crate::vintage::hover_action(ui, |ui, factor| {
        ui.scope(|ui| {
            let size = egui::Vec2::splat(side * factor);
            ui.spacing_mut().interact_size = size;
            let response = draw(ui, factor, size);
            if !ui.is_enabled() && ui.is_visible() {
                // Keep the outline legible while the disabled icon/text dims.
                let mut border = ui.painter().clone();
                border.set_opacity(1.);
                border.rect_stroke(
                    response.rect,
                    BUTTON_RADIUS,
                    egui::Stroke::new(1.0_f32, Palette::of(ui.ctx()).line),
                    egui::StrokeKind::Inside,
                );
            }
            response
        })
        .inner
    })
}

pub(crate) fn icon_control(ui: &mut egui::Ui, code: u32) -> Response {
    square_action(ui, ICON_BUTTON_SIDE, |ui, factor, size| {
        let p = Palette::of(ui.ctx());
        ui.spacing_mut().button_padding = egui::Vec2::ZERO;
        let response = square_surface(ui, size, neutral_button_fill(ui));
        paint_icon(
            ui.painter(),
            response.rect.center(),
            code,
            ICON_BUTTON_GLYPH_SIZE * factor,
            p.text,
        );
        response
    })
}

fn square_surface(ui: &mut egui::Ui, size: egui::Vec2, fill: Color32) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    ui.painter().rect(
        rect,
        BUTTON_RADIUS,
        fill,
        egui::Stroke::new(1.0_f32, Palette::of(ui.ctx()).line),
        egui::StrokeKind::Inside,
    );
    response
}

/// Place the shared filter button inside a fixed table header cell.
pub(crate) fn table_filter_button(
    ui: &mut egui::Ui,
    cell: egui::Rect,
    id: egui::Id,
    tooltip: String,
) -> Response {
    let rect = egui::Rect::from_center_size(
        cell.right_center() - egui::vec2(4. + ICON_BUTTON_SIDE / 2., 0.),
        egui::Vec2::splat(ICON_BUTTON_SIDE),
    );
    let p = Palette::of(ui.ctx());
    ui.painter().rect(
        rect,
        BUTTON_RADIUS,
        p.control,
        egui::Stroke::new(1.0_f32, p.line),
        egui::StrokeKind::Inside,
    );
    paint_icon(ui.painter(), rect.center(), 0xe098, 20., p.accent);
    ui.interact(rect, id, egui::Sense::click())
        .on_hover_text(tooltip)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Square text actions are painted inside an exact slot, independent of font line height.
pub fn square_toggle(ui: &mut egui::Ui, text: &str, selected: bool, side: f32) -> Response {
    square_action(ui, side, |ui, _, size| {
        let p = Palette::of(ui.ctx());
        let response = square_surface(ui, size, if selected { p.accent } else { p.control });
        ui.painter().text(
            response.rect.center(),
            egui::Align2::CENTER_CENTER,
            text,
            FontId::proportional(local_font_size(ui)),
            if selected { p.on_accent } else { p.text },
        );
        response.widget_info(|| {
            egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), selected, text)
        });
        response
    })
}

/// Two exclusive choices in one square frame, with a single shared divider.
pub fn segmented_pair(ui: &mut egui::Ui, labels: [&str; 2], selected: &mut bool) -> [Response; 2] {
    let palette = Palette::of(ui.ctx());
    let font = FontId::proportional(local_font_size(ui));
    let labels = labels.map(|label| {
        ui.painter()
            .layout_no_wrap(label.to_owned(), font.clone(), palette.text)
    });
    let widths = labels.each_ref().map(|label| label.size().x + 16.);
    let height = (labels[0].size().y.max(labels[1].size().y) + 2. * ui.spacing().button_padding.y)
        .max(ui.spacing().interact_size.y);
    let (rect, group) = ui.allocate_exact_size(
        egui::vec2(widths[0] + widths[1], height),
        egui::Sense::hover(),
    );
    let divider = rect.left() + widths[0];
    let segments = [
        egui::Rect::from_min_max(rect.min, egui::pos2(divider, rect.bottom())),
        egui::Rect::from_min_max(egui::pos2(divider, rect.top()), rect.max),
    ];
    let responses = std::array::from_fn(|i| {
        let response = ui.interact(segments[i], group.id.with(i), egui::Sense::click());
        if response.clicked() {
            *selected = i == 1;
        }
        if response.enabled() {
            response.on_hover_cursor(egui::CursorIcon::PointingHand)
        } else {
            response
        }
    });
    let stroke = egui::Stroke::new(1.0_f32, palette.accent);
    for (i, response) in responses.iter().enumerate() {
        let active = *selected == (i == 1);
        let segment = segments[i];
        response.widget_info(|| {
            egui::WidgetInfo::selected(
                egui::WidgetType::SelectableLabel,
                response.enabled(),
                active,
                labels[i].text(),
            )
        });
        let radius = if i == 0 {
            egui::CornerRadius {
                nw: BUTTON_RADIUS,
                sw: BUTTON_RADIUS,
                ..egui::CornerRadius::ZERO
            }
        } else {
            egui::CornerRadius {
                ne: BUTTON_RADIUS,
                se: BUTTON_RADIUS,
                ..egui::CornerRadius::ZERO
            }
        };
        ui.painter().rect_filled(
            segment,
            radius,
            if active {
                palette.accent
            } else {
                palette.control
            },
        );
        ui.painter().galley_with_override_text_color(
            segment.center() - labels[i].size() / 2.,
            labels[i].clone(),
            if active {
                palette.on_accent
            } else {
                palette.text
            },
        );
    }
    ui.painter()
        .rect_stroke(rect, BUTTON_RADIUS, stroke, egui::StrokeKind::Inside);
    ui.painter().line_segment(
        [
            egui::pos2(divider, rect.top()),
            egui::pos2(divider, rect.bottom()),
        ],
        stroke,
    );
    responses
}

/// Local overlay scrollbar; callers restore the content style inside the scroll area.
pub fn ventura_scrollbar(ui: &mut egui::Ui) {
    let dark = ui.visuals().dark_mode;
    let style = ui.style_mut();
    style.spacing.scroll = egui::style::ScrollStyle {
        floating: true,
        floating_width: 5.,
        bar_width: 9.,
        floating_allocated_width: 0.,
        handle_min_length: 32.,
        foreground_color: true,
        dormant_background_opacity: 0.,
        active_background_opacity: 0.,
        interact_background_opacity: 0.06,
        dormant_handle_opacity: 0.,
        active_handle_opacity: 0.5,
        interact_handle_opacity: 0.7,
        ..egui::style::ScrollStyle::floating()
    };
    for visual in [
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
    ] {
        visual.corner_radius = egui::CornerRadius::same(5);
        visual.fg_stroke.color = if dark { Color32::WHITE } else { Color32::BLACK };
    }
}
