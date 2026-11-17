//! Desktop help, settings and owned notification sessions. Deadlines are driven
//! by the UI clock; replacing or dropping a session leaves no detached timer.
use crate::{
    desktop::{Action, UiProbe},
    settings::{AppSettings, ThemeMode},
};
use eframe::egui;
use std::sync::{Arc, Mutex, OnceLock};
pub const DISCUSSION_URL: &str =
    "https://github.com/DevMobileAn27/flutter_network_viewer/discussions";
fn locale_id() -> egui::Id {
    egui::Id::new("desktop-locale")
}
pub fn apply(ctx: &egui::Context, settings: &AppSettings) -> bool {
    let font_id = egui::Id::new("desktop-fonts-installed");
    let ready = ctx.data(|d| d.get_temp::<bool>(font_id).unwrap_or(false));
    if !ready {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "Geist".into(),
            Arc::new(egui::FontData::from_static(include_bytes!(
                "../assets/fonts/geist/Geist-Variable.ttf"
            ))),
        );
        fonts
            .families
            .insert(egui::FontFamily::Name("Geist".into()), vec!["Geist".into()]);
        let regular = Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/cascadia_mono/CascadiaMono-Variable.ttf"
        )));
        let italic = Arc::new(egui::FontData::from_static(include_bytes!(
            "../assets/fonts/cascadia_mono/CascadiaMono-Italic-Variable.ttf"
        )));
        fonts.font_data.insert("Cascadia Mono".into(), regular);
        fonts
            .font_data
            .insert("Cascadia Mono Italic".into(), italic);
        for name in ["Cascadia Mono", "Cascadia Mono Italic"] {
            fonts
                .families
                .insert(egui::FontFamily::Name(name.into()), vec![name.into()]);
        }
        fonts.font_data.insert(
            "MaterialIcons".into(),
            Arc::new(egui::FontData::from_static(include_bytes!(
                "../assets/fonts/material_icons/MaterialIcons-Regular.otf"
            ))),
        );
        fonts.families.insert(
            egui::FontFamily::Name("MaterialIcons".into()),
            vec!["MaterialIcons".into()],
        );
        for (family, name) in [
            (egui::FontFamily::Proportional, "Geist"),
            (egui::FontFamily::Monospace, "Cascadia Mono"),
        ] {
            fonts
                .families
                .entry(family.clone())
                .or_default()
                .insert(0, name.into());
            fonts
                .families
                .entry(family)
                .or_default()
                .insert(1, "MaterialIcons".into());
        }
        ctx.set_fonts(fonts);
        ctx.data_mut(|d| d.insert_temp(font_id, true));
    }

    ctx.data_mut(|d| d.insert_temp(locale_id(), settings.values().language_code == "vi"));
    let dark = settings.values().theme_mode == ThemeMode::Dark;
    let palette = crate::desktop_style::Palette::new(dark);
    ctx.style_mut(|style| {
        style.visuals = if dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        let v = &mut style.visuals;
        v.panel_fill = palette.background;
        v.window_fill = palette.surface;
        v.menu_corner_radius = egui::CornerRadius::ZERO;
        v.text_edit_bg_color = Some(palette.control);
        v.extreme_bg_color = palette.surface;
        v.faint_bg_color = palette.surface;
        v.override_text_color = Some(palette.text);
        v.selection.bg_fill = palette.accent.gamma_multiply(0.18);
        v.selection.stroke = crate::desktop_style::input_stroke(palette, true);
        v.window_stroke = egui::Stroke::new(1.0_f32, palette.line);
        v.window_corner_radius = egui::CornerRadius::ZERO;

        for widget in [
            &mut v.widgets.noninteractive,
            &mut v.widgets.inactive,
            &mut v.widgets.hovered,
            &mut v.widgets.active,
            &mut v.widgets.open,
        ] {
            widget.bg_stroke = egui::Stroke::new(1.0_f32, palette.line);
            widget.corner_radius = egui::CornerRadius::ZERO;
            widget.fg_stroke = egui::Stroke::new(1.0_f32, palette.text);
        }
        v.widgets.inactive.bg_fill = palette.control;
        v.widgets.inactive.weak_bg_fill = palette.control;
        v.widgets.hovered.bg_fill = palette.control;
        v.widgets.active.bg_fill = palette.control;
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::proportional(settings.values().font_size),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            crate::desktop_style::medium(settings.values().font_size),
        );
        style.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::proportional(settings.values().font_size),
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            crate::desktop_style::semibold(settings.values().font_size),
        );
        style.text_styles.insert(
            egui::TextStyle::Monospace,
            egui::FontId::monospace(settings.values().font_size),
        );
        style.interaction.tooltip_delay = 0.4;
        style.spacing.window_margin = egui::Margin::same(24);
        style.spacing.item_spacing = egui::vec2(8., 6.);
        style.spacing.button_padding = egui::vec2(12., 8.);
        style.spacing.interact_size = egui::vec2(40., 32.);
    });
    crate::vintage::apply(ctx);
    ctx.style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8., 6.);
        s.spacing.window_margin = egui::Margin::same(24);
        s.spacing.interact_size = egui::vec2(32., 32.);
    });
    ready
}
pub fn tr(ctx: &egui::Context, text: &str) -> String {
    if !ctx.data(|d| d.get_temp::<bool>(locale_id()).unwrap_or(false)) {
        return text.into();
    }
    static TABLE: OnceLock<std::collections::HashMap<String, String>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        serde_json::from_str(include_str!("desktop_vi.json")).expect("translation table")
    });
    if let Some(value) = table.get(text) {
        return value.clone();
    }
    if let Some(limit) = text.strip_prefix("Body omitted: capture limit ") {
        return format!("Nội dung bị bỏ qua: giới hạn thu thập {limit}").replace("bytes", "byte");
    }
    text.into()
}
fn help(ctx: &egui::Context, key: &str) -> String {
    static TABLE: OnceLock<serde_json::Value> = OnceLock::new();
    let lang = if ctx.data(|d| d.get_temp::<bool>(locale_id()).unwrap_or(false)) {
        "vi"
    } else {
        "en"
    };
    TABLE.get_or_init(|| {
        serde_json::from_str(include_str!("desktop_help.json")).expect("help translations")
    })[key][lang]
        .as_str()
        .unwrap_or(key)
        .into()
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeKind {
    Info,
    Success,
    Warning,
    Error,
}
/// UI components report operation results here; only Notifications paints them.
pub fn notify(ctx: &egui::Context, kind: NoticeKind, message: String) {
    ctx.data_mut(|data| {
        let events = data.get_temp_mut_or_default::<Vec<(NoticeKind, String)>>(egui::Id::new(
            "notification-events",
        ));
        events.push((kind, message));
    });
}
/// Persistent async/validation state emits once per transition, not per frame.
pub fn notify_changed(
    ctx: &egui::Context,
    id: egui::Id,
    kind: NoticeKind,
    message: Option<String>,
) {
    let changed = ctx.data_mut(|data| {
        let previous = data.get_temp_mut_or_default::<Option<String>>(id);
        if *previous == message {
            false
        } else {
            *previous = message.clone();
            true
        }
    });
    if changed && let Some(message) = message {
        notify(ctx, kind, message);
    }
}
pub fn take_notifications(ctx: &egui::Context) -> Vec<(NoticeKind, String)> {
    ctx.data_mut(|data| {
        data.remove_temp::<Vec<(NoticeKind, String)>>(egui::Id::new("notification-events"))
    })
    .unwrap_or_default()
}
#[derive(Clone)]
pub struct NoticeReceipt(Arc<Mutex<Option<bool>>>);
impl NoticeReceipt {
    pub fn result(&self) -> Option<bool> {
        *self.0.lock().expect("notice result")
    }
}
struct Notice {
    kind: NoticeKind,
    description: String,
    confirmation: bool,
    mounted: Option<f64>,
    auto_hide: bool,
    delay: f64,
    receipt: NoticeReceipt,
}
impl Drop for Notice {
    fn drop(&mut self) {
        let mut r = self.receipt.0.lock().expect("notice result");
        if r.is_none() {
            *r = Some(false);
        }
    }
}
#[derive(Default)]
pub struct Notifications {
    active: Option<Notice>,
}
impl Notifications {
    pub fn show(
        &mut self,
        kind: NoticeKind,
        description: String,
        confirmation: bool,
    ) -> NoticeReceipt {
        self.show_with_options(kind, description, confirmation, None, 3000)
            .expect("default delay")
    }
    pub fn show_with_options(
        &mut self,
        kind: NoticeKind,
        description: String,
        confirmation: bool,
        auto_hide: Option<bool>,
        delay_ms: i64,
    ) -> Result<NoticeReceipt, &'static str> {
        if delay_ms < 0 {
            return Err("notification delay must be non-negative");
        }
        let receipt = NoticeReceipt(Arc::new(Mutex::new(None)));
        self.active = Some(Notice {
            kind,
            description,
            confirmation,
            mounted: None,
            auto_hide: auto_hide
                .unwrap_or(!confirmation && matches!(kind, NoticeKind::Info | NoticeKind::Success)),
            delay: delay_ms as f64 / 1000.,
            receipt: receipt.clone(),
        });
        Ok(receipt)
    }
    pub fn dismiss(&mut self, confirmed: bool) {
        if let Some(n) = self.active.take() {
            *n.receipt.0.lock().expect("notice result") = Some(confirmed);
        }
    }
    pub fn render(&mut self, ctx: &egui::Context, probe: &mut UiProbe) {
        let Some(n) = &mut self.active else {
            return;
        };
        let now = ctx.input(|i| i.time);
        let first_layout = n.mounted.is_none();
        let mounted = *n.mounted.get_or_insert(now);
        if first_layout {
            ctx.request_discard("measure notification layout");
        }
        if n.auto_hide {
            if now - mounted >= n.delay - 1e-9 {
                self.dismiss(false);
                return;
            }
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(
                (n.delay - (now - mounted)).max(0.),
            ));
        }
        let c = if ctx.style().visuals.dark_mode {
            crate::app_colors::NOTICE_DARK
        } else {
            crate::app_colors::NOTICE_LIGHT
        };
        let (title, color, icon) = match n.kind {
            NoticeKind::Info => ("Information", c[0], "ℹ"),
            NoticeKind::Success => ("Success", c[1], "✓"),
            NoticeKind::Warning => ("Warning", c[2], "⚠"),
            NoticeKind::Error => ("Error", c[3], "!"),
        };
        probe.notification_title = tr(ctx, title);
        probe.notification_color = color;
        let mut result = None;
        let area = egui::Area::new(egui::Id::new("desktop-notice"))
            .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(20., -20.))
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                let width = (ctx.content_rect().width() - 40.).clamp(40., 380.);
                egui::Frame::popup(ui.style())
                    .inner_margin(18.)
                    .show(ui, |ui| {
                        ui.set_width((width - 36. - 2. * ui.visuals().window_stroke.width).max(1.));
                        egui::ScrollArea::vertical()
                            .scroll_bar_visibility(
                                egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                            )
                            .id_salt("notice-scroll")
                            .max_height((ctx.content_rect().height() - 180.).max(20.))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x =
                                        crate::desktop_style::ICON_TEXT_GAP;
                                    let (tile, _) = ui.allocate_exact_size(
                                        egui::vec2(28., 28.),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(
                                        tile,
                                        0,
                                        crate::desktop_style::Palette::of(ui.ctx()).accent,
                                    );
                                    ui.painter().text(
                                        tile.center(),
                                        egui::Align2::CENTER_CENTER,
                                        icon,
                                        egui::FontId::proportional(
                                            crate::desktop_style::font_size(ctx),
                                        ),
                                        crate::desktop_style::Palette::of(ui.ctx()).on_accent,
                                    );
                                    probe.widgets.insert("notification-icon".into(), tile);
                                    probe.record("notification-title", ui.label(tr(ctx, title)));
                                });
                                ui.add_space(10.);
                                probe.record(
                                    "notification-description",
                                    ui.add(
                                        egui::Label::new(&n.description).selectable(true).wrap(),
                                    ),
                                );
                            });
                        ui.add_space(16.);
                        ui.horizontal(|ui| {
                            if n.confirmation
                                && probe
                                    .record(
                                        "notification-cancel",
                                        crate::desktop_style::button(ui, tr(ctx, "Cancel")),
                                    )
                                    .clicked()
                            {
                                result = Some(false);
                            }
                            if probe
                                .record(
                                    "notification-confirm",
                                    crate::desktop_style::button(
                                        ui,
                                        tr(ctx, if n.confirmation { "Confirm" } else { "OK" }),
                                    ),
                                )
                                .clicked()
                            {
                                result = Some(true);
                            }
                        });
                    });
            });
        probe
            .widgets
            .insert("notification".into(), area.response.rect);
        if let Some(result) = result {
            self.dismiss(result);
        }
    }
}
#[derive(Default)]
pub struct SupportUi {
    pub updates: crate::updater::View,
    pub help_open: bool,
    pub settings_open: bool,
    mcp_port: String,
    pub mcp_status: crate::mcp::McpStatus,
    initialized: bool,
}
impl SupportUi {
    fn show_updates(&self, ui: &mut egui::Ui, probe: &mut UiProbe, actions: &mut Vec<Action>) {
        use crate::updater::Status;
        let version = format!("Version {} - Nguyễn Quốc An", env!("CARGO_PKG_VERSION"));
        ui.colored_label(crate::desktop_style::Palette::of(ui.ctx()).muted, &version);
        probe.support_text.push_str(&version);
        let status = if !self.updates.supported {
            Some("Updates are not supported on this platform.")
        } else {
            match self.updates.status {
                Status::Checking => Some("Checking for updates…"),
                Status::Downloading => Some("Downloading update…"),
                Status::Installing => Some("Installing update…"),
                _ => None,
            }
        };
        if let Some(status) = status {
            let status = tr(ui.ctx(), status);
            probe.support_text.push_str(&status);
            probe.record("version-update-status", ui.label(status));
        }
        if self.updates.busy {
            probe.record("update-progress", crate::vintage::loading(ui, 3).response);
        }
        let label = tr(
            ui.ctx(),
            if self.updates.status == Status::Available {
                "Update lastest version"
            } else {
                "Check for updates"
            },
        );
        ui.add_space(16.);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if probe
                .record(
                    "check-app-updates",
                    ui.add_enabled_ui(self.updates.supported && !self.updates.busy, |ui| {
                        settings_button(ui, &label, false)
                    })
                    .inner,
                )
                .clicked()
            {
                actions.push(Action::CheckUpdates);
            }
        });
    }
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        settings: &mut AppSettings,
        notices: &mut Notifications,
        probe: &mut UiProbe,
        actions: &mut Vec<Action>,
    ) {
        let update_result = if self.updates.supported {
            match self.updates.status {
                crate::updater::Status::Current => {
                    Some((NoticeKind::Success, "You’re up to date."))
                }
                crate::updater::Status::Failed => Some((
                    NoticeKind::Error,
                    "Could not check for updates. Check your connection and try again.",
                )),
                crate::updater::Status::Downloaded => {
                    Some((NoticeKind::Success, "Update downloaded"))
                }
                _ => None,
            }
        } else {
            None
        };
        notify_changed(
            ctx,
            egui::Id::new("update-result-notice"),
            update_result.map_or(NoticeKind::Info, |(kind, _)| kind),
            update_result.map(|(_, text)| tr(ctx, text)),
        );
        notify_changed(
            ctx,
            egui::Id::new("mcp-error-notice"),
            NoticeKind::Error,
            self.mcp_status.error.clone(),
        );
        if self.help_open {
            let mut close = false;
            let palette = crate::desktop_style::Palette::of(ctx);
            let modal = egui::Modal::new(egui::Id::new("basic-help"))
                .frame(
                    egui::Frame::popup(&ctx.style())
                        .fill(palette.surface)
                        .inner_margin(16)
                        .corner_radius(0),
                )
                .show(ctx, |ui| {
                    ui.set_width((ctx.content_rect().width() - 64.).clamp(100., 600.));
                    ui.spacing_mut().item_spacing = egui::vec2(8., 4.);
                    ui.spacing_mut().button_padding = egui::vec2(8., 4.);
                    ui.spacing_mut().interact_size.y = 28.;
                    let title = help(ctx, "helpTitle");
                    probe.support_text = title.clone();
                    ui.horizontal(|ui| {
                        probe.record(
                            "help-title",
                            ui.label(egui::RichText::new(title).font(
                                crate::desktop_style::semibold(crate::desktop_style::font_size(
                                    ui.ctx(),
                                )),
                            )),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if probe
                                .record(
                                    "help-dismiss",
                                    crate::desktop_style::square_icon(
                                        ui,
                                        0xe16a,
                                        &tr(ctx, "Close"),
                                    ),
                                )
                                .on_hover_text(tr(ctx, "Close"))
                                .clicked()
                            {
                                close = true;
                            }
                        });
                    });
                    ui.add_space(8.);
                    ui.separator();
                    ui.add_space(4.);
                    let scroll = egui::ScrollArea::vertical()
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .id_salt("help-scroll")
                        .max_height((ctx.content_rect().height() - 180.).max(40.))
                        .show(ui, |ui| {
                            for section in ["Connect", "Network", "Console", "Json", "Capture"] {
                                let title = help(ctx, &format!("help{section}Title"));
                                let body = help(ctx, &format!("help{section}Body"));
                                egui::Frame::NONE
                                    .inner_margin(egui::Margin::symmetric(0, 8))
                                    .show(ui, |ui| {
                                        ui.horizontal_top(|ui| {
                                            ui.spacing_mut().item_spacing.x =
                                                crate::desktop_style::ICON_TEXT_GAP;
                                            let icon = match section {
                                                "Connect" => 0xe380,
                                                "Network" => 0xe627,
                                                "Console" => 0xf0589,
                                                "Json" => 0xf02ff,
                                                _ => 0xf05b,
                                            };
                                            let (rect, _) = ui.allocate_exact_size(
                                                egui::vec2(24., 24.),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().rect_filled(
                                                rect,
                                                0,
                                                crate::desktop_style::Palette::of(ui.ctx()).accent,
                                            );
                                            crate::desktop_style::paint_icon(
                                                ui.painter(),
                                                rect.center(),
                                                icon,
                                                16.,
                                                crate::desktop_style::Palette::of(ui.ctx())
                                                    .on_accent,
                                            );
                                            ui.vertical(|ui| {
                                                ui.label(egui::RichText::new(title).font(
                                                    crate::desktop_style::semibold(
                                                        crate::desktop_style::font_size(ui.ctx()),
                                                    ),
                                                ));
                                                ui.add(
                                                    egui::Label::new(
                                                        egui::RichText::new(&body)
                                                            .font(egui::FontId::proportional(
                                                                crate::desktop_style::font_size(
                                                                    ui.ctx(),
                                                                ),
                                                            ))
                                                            .color(palette.muted),
                                                    )
                                                    .wrap(),
                                                );
                                            });
                                        });
                                    });
                                probe.support_text.push_str(&body);
                            }
                        });
                    probe
                        .widgets
                        .insert("help-scroll".into(), scroll.inner_rect);
                    probe.support_scroll = scroll.state.offset.y;
                    ui.add_space(4.);
                    ui.separator();
                    ui.add_space(8.);
                    ui.horizontal(|ui| {
                        if probe
                            .record(
                                "discussion",
                                crate::desktop_style::button(
                                    ui,
                                    egui::RichText::new(tr(ctx, "Go to Discussion")).font(
                                        egui::FontId::proportional(
                                            crate::desktop_style::font_size(ui.ctx()),
                                        ),
                                    ),
                                ),
                            )
                            .on_hover_text(DISCUSSION_URL)
                            .clicked()
                        {
                            ctx.open_url(egui::OpenUrl::new_tab(DISCUSSION_URL));
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if probe
                                .record(
                                    "help-close",
                                    crate::desktop_style::button(
                                        ui,
                                        egui::RichText::new(tr(ctx, "Close")).font(
                                            egui::FontId::proportional(
                                                crate::desktop_style::font_size(ui.ctx()),
                                            ),
                                        ),
                                    ),
                                )
                                .clicked()
                            {
                                close = true;
                            }
                        });
                    });
                });
            probe
                .widgets
                .insert("help-modal".into(), modal.response.rect);
            if close || modal.should_close() {
                self.help_open = false;
            }
        }
        if self.settings_open {
            if !self.initialized {
                self.mcp_port = settings.values().mcp_port.to_string();
                self.initialized = true;
            }
            let mut close = false;
            let popup_was_open = egui::Popup::is_any_open(ctx);
            let palette = crate::desktop_style::Palette::of(ctx);
            egui::TopBottomPanel::top("settings-appbar")
                .frame(egui::Frame::NONE.fill(palette.surface).inner_margin(8))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.;
                        let start = ui.cursor().min.x;
                        let back = crate::desktop_style::button(ui, tr(ctx, "Back"));
                        let side = ui.cursor().min.x - start;
                        // The action reserves its resting layout slot while its painted
                        // response grows on hover. Size siblings from that slot so the
                        // content-driven header does not follow the enlarged hit rect.
                        let row_height = ui.min_rect().height();
                        if probe.record("settings-close", back).clicked() {
                            close = true;
                        }
                        let title = tr(ctx, "Setting");
                        probe.support_text = title.clone();
                        ui.allocate_ui_with_layout(
                            egui::vec2((ui.available_width() - side).max(0.), row_height),
                            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                            |ui| {
                                probe.record(
                                    "settings-title",
                                    ui.label(egui::RichText::new(title).font(
                                        crate::desktop_style::semibold(
                                            crate::desktop_style::font_size(ui.ctx()),
                                        ),
                                    )),
                                );
                            },
                        );
                        let (spacer, _) = ui.allocate_exact_size(
                            egui::vec2(side, row_height),
                            egui::Sense::hover(),
                        );
                        probe
                            .widgets
                            .insert("settings-header-spacer".into(), spacer);
                    });
                });
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.fill(palette.background))
                .show(ctx, |ui| {
                    let scroll = egui::ScrollArea::vertical()
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .id_salt("settings-scroll")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            let width = ui.available_width();
                            let padding = 8.;
                            ui.add_space(8.);
                            let origin = ui.cursor().min + egui::vec2(padding, 0.);
                            let content = ui.scope_builder(
                                egui::UiBuilder::new().max_rect(egui::Rect::from_min_size(
                                    origin,
                                    egui::vec2(width - 2. * padding, 0.),
                                )),
                                |ui| {
                                    ui.set_width(width - 2. * padding);
                                    ui.spacing_mut().item_spacing.y = 0.;
                                    ui.spacing_mut().interact_size.y = 44.;
                                    let appearance =
                                        settings_section(ui, &tr(ctx, "Interface"), |ui| {
                                            field_label(ui, &tr(ctx, "Theme"));
                                            let response = egui::ComboBox::from_id_salt("theme")
                                                .width(220_f32.min(ui.available_width()))
                                                .selected_text(dropdown_text(
                                                    ctx,
                                                    tr(
                                                        ctx,
                                                        if settings.values().theme_mode
                                                            == ThemeMode::Dark
                                                        {
                                                            "Dark"
                                                        } else {
                                                            "Light"
                                                        },
                                                    ),
                                                ))
                                                .popup_style(egui::style::StyleModifier::from(
                                                    |s: &mut egui::Style| {
                                                        s.spacing.menu_margin = egui::Margin::ZERO;
                                                    },
                                                ))
                                                .show_ui(ui, |ui| {
                                                    for (key, mode, label) in [
                                                        ("light", ThemeMode::Light, "Light"),
                                                        ("dark", ThemeMode::Dark, "Dark"),
                                                    ] {
                                                        if probe
                                                            .record(
                                                                format!("theme:{key}"),
                                                                crate::desktop_style::option(
                                                                    ui,
                                                                    settings.values().theme_mode
                                                                        == mode,
                                                                    tr(ctx, label),
                                                                ),
                                                            )
                                                            .clicked()
                                                        {
                                                            save_result(
                                                                settings.set_theme_mode(mode),
                                                                ctx,
                                                                notices,
                                                            );
                                                        }
                                                    }
                                                });
                                            probe.record("theme", response.response);
                                            ui.add_space(12.);
                                            field_label(ui, &tr(ctx, "Language"));
                                            let response = egui::ComboBox::from_id_salt("language")
                                                .width(220_f32.min(ui.available_width()))
                                                .selected_text(dropdown_text(
                                                    ctx,
                                                    if settings.values().language_code == "vi" {
                                                        "Tiếng Việt"
                                                    } else {
                                                        "English"
                                                    },
                                                ))
                                                .popup_style(egui::style::StyleModifier::from(
                                                    |s: &mut egui::Style| {
                                                        s.spacing.menu_margin = egui::Margin::ZERO;
                                                    },
                                                ))
                                                .show_ui(ui, |ui| {
                                                    for (code, label) in
                                                        [("en", "English"), ("vi", "Tiếng Việt")]
                                                    {
                                                        if probe
                                                            .record(
                                                                format!("language:{code}"),
                                                                crate::desktop_style::option(
                                                                    ui,
                                                                    settings.values().language_code
                                                                        == code,
                                                                    label,
                                                                ),
                                                            )
                                                            .clicked()
                                                        {
                                                            save_result(
                                                                settings.set_language(code),
                                                                ctx,
                                                                notices,
                                                            );
                                                        }
                                                    }
                                                });
                                            probe.record("language", response.response);
                                            ui.add_space(12.);
                                            field_label(ui, &tr(ctx, "Font size"));
                                            let mut size = settings.values().font_size;
                                            crate::vintage::font_size_stepper(
                                                ui,
                                                &mut size,
                                                probe,
                                                [
                                                    "font-size-decrease",
                                                    "font-size",
                                                    "font-size-increase",
                                                ],
                                            );
                                            if size != settings.values().font_size {
                                                save_result(
                                                    settings.set_font_size(size),
                                                    ctx,
                                                    notices,
                                                );
                                            }
                                            ui.add_space(0.);
                                        });
                                    ui.add_space(12.);
                                    probe.record("settings-appearance", appearance);
                                    let mcp = settings_section(ui, "MCP", |ui| {
                                        ui.scope(|ui| {
                                            ui.spacing_mut().interact_size.y = 32.;
                                            ui.horizontal(|ui| {
                                                section_heading(ui, "MCP");
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        let mut enabled =
                                                            settings.values().mcp_enabled;
                                                        if probe
                                                            .record(
                                                                "mcp-enabled",
                                                                settings_switch(
                                                                    ui,
                                                                    &mut enabled,
                                                                    None,
                                                                ),
                                                            )
                                                            .changed()
                                                        {
                                                            apply_mcp(
                                                                settings,
                                                                &self.mcp_port,
                                                                enabled,
                                                                ctx,
                                                                notices,
                                                                actions,
                                                            );
                                                        }
                                                        let status = tr(
                                                            ctx,
                                                            if !settings.values().mcp_enabled {
                                                                "MCP disabled"
                                                            } else if self.mcp_status.running {
                                                                "MCP listening"
                                                            } else {
                                                                "Server stopped"
                                                            },
                                                        );
                                                        ui.label(
                                                            egui::RichText::new(&status)
                                                                .size(
                                                                    crate::desktop_style::font_size(
                                                                        ui.ctx(),
                                                                    ),
                                                                )
                                                                .color(palette.muted),
                                                        );
                                                        probe.support_text.push_str(&status);
                                                    },
                                                );
                                            });
                                        });
                                        ui.add_space(12.);
                                        if ui.available_width() >= 680. {
                                            ui.horizontal_top(|ui| {
                                                ui.spacing_mut().item_spacing.x = 12.;
                                                ui.allocate_ui_with_layout(
                                                    egui::vec2(160., 76.),
                                                    egui::Layout::top_down(egui::Align::Min),
                                                    |ui| {
                                                        field_label(ui, &tr(ctx, "MCP port"));
                                                        let response = probe.record(
                                                            "mcp-port",
                                                            outlined_input(
                                                                ui,
                                                                &mut self.mcp_port,
                                                                134.,
                                                            ),
                                                        );
                                                        if response.lost_focus()
                                                            && ui.input(|i| {
                                                                i.key_pressed(egui::Key::Enter)
                                                            })
                                                        {
                                                            apply_mcp(
                                                                settings,
                                                                &self.mcp_port,
                                                                settings.values().mcp_enabled,
                                                                ctx,
                                                                notices,
                                                                actions,
                                                            );
                                                        }
                                                    },
                                                );
                                                ui.allocate_ui_with_layout(
                                                    egui::vec2(ui.available_width(), 76.),
                                                    egui::Layout::top_down(egui::Align::Min),
                                                    |ui| {
                                                        endpoint_field(
                                                            ui,
                                                            settings.values().mcp_port as u16,
                                                            probe,
                                                        )
                                                    },
                                                );
                                            });
                                        } else {
                                            field_label(ui, &tr(ctx, "MCP port"));
                                            let response = probe.record(
                                                "mcp-port",
                                                outlined_input(
                                                    ui,
                                                    &mut self.mcp_port,
                                                    ui.available_width() - 26.,
                                                ),
                                            );
                                            if response.lost_focus()
                                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                                            {
                                                apply_mcp(
                                                    settings,
                                                    &self.mcp_port,
                                                    settings.values().mcp_enabled,
                                                    ctx,
                                                    notices,
                                                    actions,
                                                );
                                            }
                                            ui.add_space(8.);
                                            endpoint_field(
                                                ui,
                                                settings.values().mcp_port as u16,
                                                probe,
                                            );
                                        }
                                        ui.add_space(12.);
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center)
                                                .with_main_wrap(true),
                                            |ui| {
                                                ui.spacing_mut().item_spacing =
                                                    egui::vec2(12., 12.);
                                                for (key, label, claude) in [
                                                    (
                                                        "mcp-copy-codex",
                                                        "Copy manual MCP for Codex",
                                                        false,
                                                    ),
                                                    (
                                                        "mcp-copy-claude",
                                                        "Copy manual MCP for Claude",
                                                        true,
                                                    ),
                                                ] {
                                                    if probe
                                                        .record(
                                                            key,
                                                            mcp_copy_button(ui, &tr(ctx, label)),
                                                        )
                                                        .clicked()
                                                    {
                                                        copy(
                                                            ctx,
                                                            crate::mcp::setup(
                                                                settings.values().mcp_port as u16,
                                                                claude,
                                                            ),
                                                        );
                                                    }
                                                }
                                                if self.mcp_status.error.is_some()
                                                    && probe
                                                        .record(
                                                            "mcp-retry",
                                                            crate::desktop_style::button(
                                                                ui,
                                                                tr(ctx, "Retry"),
                                                            ),
                                                        )
                                                        .clicked()
                                                {
                                                    actions.push(Action::RetryMcp);
                                                }
                                            },
                                        );
                                    });
                                    probe.record("settings-mcp", mcp);
                                    ui.add_space(12.);
                                    let updates =
                                        settings_section(ui, &tr(ctx, "App updates"), |ui| {
                                            self.show_updates(ui, probe, actions)
                                        });
                                    probe.record("settings-updates", updates);
                                },
                            );
                            ui.advance_cursor_after_rect(
                                content.response.rect.expand2(egui::vec2(0., 8.)),
                            );
                        });
                    probe
                        .widgets
                        .insert("settings-scroll".into(), scroll.inner_rect);
                    probe.support_scroll = scroll.state.offset.y;
                });
            if close || (!popup_was_open && ctx.input(|i| i.key_pressed(egui::Key::Escape))) {
                self.settings_open = false;
                self.initialized = false;
            }
        }
    }
}
fn save_result(
    result: Result<(), crate::settings::SettingsError>,
    ctx: &egui::Context,
    notices: &mut Notifications,
) -> bool {
    match result {
        Ok(()) => {
            notices.show(NoticeKind::Success, tr(ctx, "Settings saved"), false);
            true
        }
        Err(e) => {
            notices.show(
                NoticeKind::Error,
                format!("{}: {e}", tr(ctx, "Settings save failed")),
                false,
            );
            false
        }
    }
}

/// All copy entry points use eframe's native platform output and one success toast.
pub fn copy(ctx: &egui::Context, text: String) {
    ctx.copy_text(text);
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("desktop-copied"), true));
}
pub fn copied(ctx: &egui::Context) -> bool {
    ctx.data_mut(|d| {
        d.remove_temp::<bool>(egui::Id::new("desktop-copied"))
            .unwrap_or(false)
    })
}

fn field_label(ui: &mut egui::Ui, text: &str) {
    let p = crate::desktop_style::Palette::of(ui.ctx());
    ui.label(
        egui::RichText::new(text)
            .size(crate::desktop_style::font_size(ui.ctx()))
            .color(p.muted),
    );
    ui.add_space(8.);
}
fn settings_section(
    ui: &mut egui::Ui,
    title: &str,
    body: impl FnOnce(&mut egui::Ui),
) -> egui::Response {
    let p = crate::desktop_style::Palette::of(ui.ctx());
    egui::Frame::NONE
        .fill(p.surface)
        .stroke(egui::Stroke::new(1.0_f32, p.line))
        .corner_radius(0)
        .inner_margin(8)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            if title != "MCP" {
                section_heading(ui, title);
                ui.add_space(12.);
            }
            body(ui);
        })
        .response
}

pub fn outlined_input(ui: &mut egui::Ui, value: &mut String, width: f32) -> egui::Response {
    input_with_hint(ui, value, width, "")
}

pub fn input_with_hint(
    ui: &mut egui::Ui,
    value: &mut String,
    width: f32,
    hint: &str,
) -> egui::Response {
    let p = crate::desktop_style::Palette::of(ui.ctx());
    outlined_text_edit(
        ui,
        egui::TextEdit::singleline(value)
            .desired_width(width)
            .hint_text(hint),
        p.control,
    )
}

/// Content-sized read-only field shared by value displays. Selection is optional;
/// non-selectable values never take focus or respond to editing and dragging.
pub fn display_text(
    ui: &mut egui::Ui,
    text: &str,
    align: egui::Align,
    selectable: bool,
) -> egui::Response {
    let p = crate::desktop_style::Palette::of(ui.ctx());
    let font = egui::FontId::proportional(crate::desktop_style::font_size(ui.ctx()));
    let width = ui
        .fonts_mut(|fonts| fonts.layout_no_wrap(text.to_owned(), font, p.text))
        .size()
        .x;
    let mut value = text;
    outlined_text_edit(
        ui,
        egui::TextEdit::singleline(&mut value)
            .desired_width(width.min((ui.available_width() - 24.).max(1.)))
            .horizontal_align(align)
            .interactive(selectable),
        p.background,
    )
}

fn outlined_text_edit(
    ui: &mut egui::Ui,
    edit: egui::TextEdit<'_>,
    background: egui::Color32,
) -> egui::Response {
    // Draw one frame for editable and immutable buffers alike.
    let backing = ui.painter().add(egui::Shape::Noop);
    let response = ui.add(
        edit.frame(false)
            .font(egui::FontId::proportional(crate::desktop_style::font_size(
                ui.ctx(),
            )))
            .margin(egui::Margin::symmetric(12, 12))
            .vertical_align(egui::Align::Center),
    );
    ui.painter().set(
        backing,
        egui::Shape::rect_filled(response.rect, 0, background),
    );
    crate::desktop_style::input_border(ui, response.rect, response.has_focus());
    response
}
pub fn dropdown_text(ctx: &egui::Context, text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text).font(crate::desktop_style::medium(
        crate::desktop_style::font_size(ctx),
    ))
}

fn settings_button(ui: &mut egui::Ui, label: &str, filled: bool) -> egui::Response {
    crate::vintage::hover_action(ui, |ui, _| {
        let p = crate::desktop_style::Palette::of(ui.ctx());
        ui.add(
            egui::Button::new(egui::RichText::new(label).color(if filled {
                p.on_accent
            } else {
                p.text
            }))
            .fill(if filled { p.accent } else { p.control })
            .min_size(egui::vec2(0., 44.))
            .corner_radius(0),
        )
    })
}
fn settings_switch(ui: &mut egui::Ui, value: &mut bool, label: Option<&str>) -> egui::Response {
    ui.horizontal(|ui| {
        if label.is_none() {
            ui.set_width(52.);
        }
        if let Some(label) = label {
            ui.allocate_ui_with_layout(
                egui::vec2((ui.available_width() - 64.).max(20.), 48.),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(label)
                                .size(crate::desktop_style::font_size(ui.ctx())),
                        )
                        .wrap()
                        .halign(egui::Align::Min),
                    );
                },
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            crate::vintage::toggle(ui, value)
        })
        .inner
    })
    .inner
}
fn apply_mcp(
    settings: &mut AppSettings,
    draft: &str,
    enabled: bool,
    ctx: &egui::Context,
    notices: &mut Notifications,
    actions: &mut Vec<Action>,
) {
    let port = match draft.trim().parse::<i64>() {
        Ok(port) if (1..=65535).contains(&port) => port,
        _ if !enabled => settings.values().mcp_port,
        _ => {
            notices.show(
                NoticeKind::Error,
                tr(ctx, "Enter a port from 1 to 65535"),
                false,
            );
            return;
        }
    };
    let changed = settings.values().mcp_enabled != enabled || settings.values().mcp_port != port;
    if changed && save_result(settings.set_mcp_config(enabled, port), ctx, notices) {
        actions.push(Action::RetryMcp);
    }
}

fn mcp_copy_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    crate::vintage::hover_action(ui, |ui, factor| {
        let size = crate::desktop_style::font_size(ui.ctx()) * factor;
        let color = crate::desktop_style::Palette::of(ui.ctx()).text;
        let mut job = egui::text::LayoutJob::default();
        job.append(
            "\u{ef7f}",
            0.,
            egui::TextFormat {
                font_id: egui::FontId::new(size, egui::FontFamily::Name("MaterialIcons".into())),
                color,
                ..Default::default()
            },
        );
        job.append(
            label,
            crate::desktop_style::ICON_TEXT_GAP,
            egui::TextFormat {
                font_id: egui::FontId::proportional(size),
                color,
                ..Default::default()
            },
        );
        ui.add(
            egui::Button::new(job)
                .wrap()
                .fill(crate::desktop_style::Palette::of(ui.ctx()).control)
                .min_size(egui::vec2(0., 44.)),
        )
    })
}

/// Reusable read-only display with an adjacent square copy action.
pub fn display_text_with_copy(
    ui: &mut egui::Ui,
    text: &str,
    probe: &mut UiProbe,
    field_key: &str,
    copy_key: &str,
) {
    ui.horizontal(|ui| {
        let max = egui::Rect::from_min_size(
            ui.cursor().min,
            egui::vec2(
                (ui.available_width() - 34. - ui.spacing().item_spacing.x).max(24.),
                ui.available_height(),
            ),
        );
        ui.scope_builder(egui::UiBuilder::new().max_rect(max), |ui| {
            probe.record(field_key, display_text(ui, text, egui::Align::Min, true));
        });
        if probe
            .record(
                copy_key,
                crate::desktop_style::square_icon(ui, 0xef7f, &tr(ui.ctx(), "Copy")),
            )
            .clicked()
        {
            copy(ui.ctx(), text.to_owned());
        }
    });
}

fn endpoint_field(ui: &mut egui::Ui, port: u16, probe: &mut UiProbe) {
    field_label(ui, &tr(ui.ctx(), "MCP endpoint"));
    display_text_with_copy(
        ui,
        &crate::mcp::endpoint(port),
        probe,
        "mcp-endpoint",
        "mcp-endpoint-copy",
    );
}

fn section_heading(ui: &mut egui::Ui, title: &str) {
    let p = crate::desktop_style::Palette::of(ui.ctx());
    ui.scope(|ui| {
        let font = crate::desktop_style::semibold(crate::desktop_style::font_size(ui.ctx()));
        let height = ui
            .painter()
            .layout_no_wrap(title.into(), font, p.text)
            .size()
            .y;
        ui.spacing_mut().interact_size.y = height;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = crate::desktop_style::ICON_TEXT_GAP;
            let icon = match title {
                "MCP" => 0xf0616,
                "Connection" | "Kết nối" => 0xf062c,
                "App updates" | "Cập nhật ứng dụng" => 0xe637,
                _ => 0xe683,
            };
            ui.label(crate::desktop_style::glyph(icon, height, p.accent));
            ui.label(
                egui::RichText::new(title).font(crate::desktop_style::semibold(
                    crate::desktop_style::font_size(ui.ctx()),
                )),
            );
        });
    });
}
