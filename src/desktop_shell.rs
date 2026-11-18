//! Compact desktop header, contextual tools and connection controls.
use super::*;
use crate::desktop_style::{ICON_BUTTON_GLYPH_SIZE, ICON_BUTTON_SIDE, Palette, paint_icon};
impl DesktopUi {
    pub(super) fn shell(
        &mut self,
        ctx: &egui::Context,
        store: &CaptureStore,
        state: &ShellStatus,
        actions: &mut Vec<Action>,
    ) {
        let p = Palette::of(ctx);
        let extra = (crate::desktop_style::font_size(ctx) - 14.).max(0.);
        let compact = ctx.content_rect().width() < 1000. + extra * 30.;
        egui::TopBottomPanel::top("flutter-header")
            .exact_height(32. + extra)
            .frame(
                egui::Frame::NONE
                    .fill(p.surface)
                    .inner_margin(egui::Margin::symmetric(8, 2)),
            )
            .show(ctx, |ui| {
                self.probe
                    .widgets
                    .insert("header".into(), ui.max_rect().expand2(egui::vec2(8., 2.)));
                self.probe
                    .widgets
                    .insert("header-content".into(), ui.max_rect());
                ui.spacing_mut().item_spacing.x = 6.;
                ui.spacing_mut().interact_size.y = 24.;
                ui.horizontal_centered(|ui| {
                    let tab_width = if ctx.content_rect().width() < 440. {
                        36.
                    } else if compact {
                        60. + extra * 3.
                    } else {
                        150. + extra * 8.
                    };
                    let tabs = egui::Frame::NONE
                        .fill(p.surface)
                        .corner_radius(0)
                        .inner_margin(egui::Margin::symmetric(0, 2))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 0.;
                                for (index, key, label, code) in [
                                    (
                                        0,
                                        "Network",
                                        format!("{} {}", tr(ctx, "Network"), store.network().len()),
                                        0xe627,
                                    ),
                                    (
                                        1,
                                        "Console",
                                        format!(
                                            "{} {}",
                                            tr(ctx, "Console"),
                                            self.grouped_logs.len()
                                        ),
                                        0xf0589,
                                    ),
                                    (2, "JSON", "Json beautify".into(), 0xf02ff),
                                ] {
                                    let label = if compact {
                                        match index {
                                            0 => "Net",
                                            1 => "Logs",
                                            _ => "JSON",
                                        }
                                        .into()
                                    } else {
                                        label
                                    };
                                    if self
                                        .probe
                                        .record(
                                            key,
                                            compact_tab(
                                                ui,
                                                &label,
                                                code,
                                                self.page == index,
                                                tab_width,
                                            ),
                                        )
                                        .clicked()
                                    {
                                        self.page = index;
                                    }
                                }
                            });
                        });
                    self.probe
                        .widgets
                        .insert("page-tabs".into(), tabs.response.rect);
                    group_separator(ui);
                    let label = tr(
                        ctx,
                        if state.searching {
                            "Stop scan"
                        } else {
                            "Find & connect"
                        },
                    );
                    let scan_code = if state.searching { 0xf3e9 } else { 0xe501 };
                    let manual = tr(ctx, "Manual URL");
                    let action_width = |code, label| {
                        if compact {
                            ICON_BUTTON_SIDE
                        } else {
                            let job = header_action_job(ui, code, Some(label));
                            ui.fonts_mut(|f| f.layout_job(job)).size().x + 12.
                        }
                    };
                    let scan_width = action_width(scan_code, &label);
                    let manual_width = action_width(0xe380, &manual);
                    let app_width = (ui.available_width()
                        - 26.
                        - scan_width.max(24.)
                        - manual_width.max(24.)
                        - 48.
                        - 36.)
                        .max(40.);
                    self.header_apps(ui, store, state, app_width);
                    group_separator(ui);
                    if self
                        .probe
                        .record(
                            if state.searching { "stop-scan" } else { "scan" },
                            header_action(ui, scan_code, if compact { None } else { Some(&label) })
                                .on_hover_text(&label),
                        )
                        .clicked()
                    {
                        actions.push(if state.searching {
                            Action::StopScan
                        } else {
                            Action::StartScan
                        });
                    }
                    if self
                        .probe
                        .record(
                            "manual-url",
                            header_action(ui, 0xe380, if compact { None } else { Some(&manual) })
                                .on_hover_text(&manual),
                        )
                        .clicked()
                    {
                        self.connect_open = true;
                    }
                    group_separator(ui);
                    if self
                        .probe
                        .record(
                            "settings",
                            header_action(ui, 0xf36e, None).on_hover_text(tr(ctx, "Settings")),
                        )
                        .clicked()
                    {
                        self.support.settings_open = true;
                    }
                    if self
                        .probe
                        .record(
                            "help",
                            header_action(ui, 0xe30b, None).on_hover_text(tr(ctx, "Help")),
                        )
                        .clicked()
                    {
                        self.support.help_open = true;
                    }
                });
            });
        if store.paused {
            self.status_banner(
                ctx,
                "paused-banner",
                &tr(ctx, "Capture paused · Incoming events discarded"),
                p.muted,
            );
        }
        let json_page = self.page == 2;
        let text_control_side = crate::desktop_json::json_text_control_side(ctx);
        let row_height = (if json_page {
            text_control_side + 4.
        } else {
            24.
        })
        .max(crate::desktop_style::search_bar_height(ctx));
        egui::TopBottomPanel::top("flutter-navigation")
            .exact_height(row_height + 8.)
            .frame(
                egui::Frame::NONE
                    .fill(p.background)
                    .inner_margin(egui::Margin::symmetric(8, 4)),
            )
            .show(ctx, |ui| {
                self.probe.widgets.insert(
                    "nav-toolbar".into(),
                    ui.max_rect().expand2(egui::vec2(8., 4.)),
                );
                self.probe
                    .widgets
                    .insert("nav-content".into(), ui.max_rect());
                ui.spacing_mut().item_spacing = egui::vec2(6., 0.);
                ui.spacing_mut().interact_size.y = 24.;
                // Keep the measured width of the JSON controls, excluding the
                // search field whose width can change as the window resizes.
                let json_width_id = ui
                    .id()
                    .with(("json-tools-width", text_control_side.to_bits()));
                let previous_width = ctx.data(|d| d.get_temp::<f32>(json_width_id));
                let json_fixed_width = previous_width.unwrap_or_else(|| {
                    3. * text_control_side
                        + 5. * ICON_BUTTON_SIDE
                        + 9. * crate::desktop_style::font_size(ctx)
                        + 202.
                });
                let search_width = (ui.available_width() - json_fixed_width).clamp(160., 350.);
                let tools_width = if json_page {
                    json_fixed_width + search_width
                } else {
                    350. + 13. + 3. * ICON_BUTTON_SIDE + 4. * ui.spacing().item_spacing.x
                };
                egui::ScrollArea::horizontal()
                    .id_salt("capture-toolbar-scroll")
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .auto_shrink([false, true])
                    .max_height(row_height)
                    .show(ui, |ui| {
                        ui.horizontal_centered(|ui| {
                            ui.add_space((ui.available_width() - tools_width).max(0.));
                            if self.page == 2 {
                                let start = ui.next_widget_position().x;
                                if self
                                    .json_workspace
                                    .toolbar(ui, &mut self.probe, search_width)
                                {
                                    self.export_open = true;
                                    self.export_json = true;
                                    self.export_path = "formatted.json".into();
                                }
                                let width = self.probe.widgets["json-download"].right()
                                    - start
                                    - search_width;
                                if previous_width.is_none_or(|old| (old - width).abs() > 0.1) {
                                    ctx.data_mut(|d| d.insert_temp(json_width_id, width));
                                    ctx.request_repaint();
                                }
                            } else {
                                self.capture_search(ui);
                                group_separator(ui);
                                if self
                                    .probe
                                    .record(
                                        "pause",
                                        header_action(
                                            ui,
                                            if store.paused { 0xe4cb } else { 0xe47c },
                                            None,
                                        )
                                        .on_hover_text(tr(
                                            ctx,
                                            if store.paused { "Resume" } else { "Pause" },
                                        )),
                                    )
                                    .clicked()
                                {
                                    actions.push(Action::SetPaused(!store.paused));
                                }
                                if self
                                    .probe
                                    .record(
                                        "clear",
                                        header_action(ui, 0xefab, None)
                                            .on_hover_text(tr(ctx, "Clear")),
                                    )
                                    .clicked()
                                {
                                    self.release_detail();
                                    self.selected_request = None;
                                    self.selected_log = None;
                                    self.raw_cache = None;
                                    self.console_detail_cache = None;
                                    actions.push(Action::Clear);
                                }
                                if self
                                    .probe
                                    .record(
                                        "export",
                                        header_action(ui, 0xf05b, None)
                                            .on_hover_text(tr(ctx, "Export")),
                                    )
                                    .clicked()
                                {
                                    self.export_open = true;
                                    self.export_json = false;
                                }
                            }
                        });
                    });
            });
        egui::TopBottomPanel::bottom("resource-status")
            .exact_height(26. + extra)
            .frame(
                egui::Frame::NONE
                    .fill(p.surface)
                    .inner_margin(egui::Margin::symmetric(18, 4)),
            )
            .show(ctx, |ui| {
                self.probe
                    .widgets
                    .insert("footer".into(), ui.max_rect().expand2(egui::vec2(18., 4.)));
                let text = crate::resources::memory_label(self.memory_bytes);
                self.probe.resource_text = text.clone();
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    self.probe.record(
                        "resource-usage",
                        ui.label(
                            egui::RichText::new(text)
                                .size(crate::desktop_style::font_size(ui.ctx()))
                                .color(p.muted),
                        ),
                    );
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(format!(
                                "● {} connections   {} visible · {} evicted · {} skipped",
                                state.online.len(),
                                store.network().len() + store.logs().len(),
                                store.statistics.evicted,
                                store.statistics.ignored_while_paused
                            ))
                            .size(crate::desktop_style::font_size(ui.ctx()))
                            .color(p.muted),
                        )
                        .truncate()
                        .show_tooltip_when_elided(false),
                    );
                });
            });
    }
    fn capture_search(&mut self, ui: &mut egui::Ui) {
        let hint = tr(
            ui.ctx(),
            if self.page == 0 {
                "Filter URL, method, status…"
            } else {
                "Search message, logger, error, stack, args"
            },
        );
        let (_, cleared) = crate::desktop_style::search_bar(
            ui,
            &mut self.probe,
            "search",
            "reset",
            &mut self.query,
            &hint,
            350.,
        );
        if cleared {
            self.reset_capture_filters();
        }
    }

    fn status_banner(
        &mut self,
        ctx: &egui::Context,
        key: &'static str,
        text: &str,
        color: egui::Color32,
    ) {
        let panel = egui::TopBottomPanel::top(key)
            .frame(
                egui::Frame::NONE
                    .fill(color.gamma_multiply(0.1))
                    .inner_margin(egui::Margin::symmetric(22, 10)),
            )
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(text)
                        .size(crate::desktop_style::font_size(ui.ctx()))
                        .color(color),
                );
            });
        self.probe.widgets.insert(key.into(), panel.response.rect);
    }
    pub(super) fn reset_capture_filters(&mut self) {
        self.query.clear();
        self.method = "All methods".into();
        self.status = "All statuses".into();
        self.log_source = "All log sources".into();
        self.log_level = "All levels".into();
        self.message_tag_input.clear();
        self.message_tags.clear();
        self.app = None;
    }
    pub(super) fn empty_state(&mut self, ui: &mut egui::Ui, inspector: bool) {
        self.render_empty_state(ui, inspector);
    }

    pub(super) fn no_results(&mut self, ui: &mut egui::Ui) {
        self.render_empty_state(ui, false);
    }

    fn render_empty_state(&mut self, ui: &mut egui::Ui, inspector: bool) {
        let p = Palette::of(ui.ctx());
        let rect = ui.available_rect_before_wrap();
        let key = if inspector {
            "empty-inspector"
        } else {
            "empty-list"
        };
        self.probe.widgets.insert(key.into(), rect);
        let title = crate::vintage::empty_text(ui.ctx());
        if !inspector {
            ui.painter()
                .rect_stroke(rect, 0, (1.0_f32, p.line), egui::StrokeKind::Inside);
        }
        ui.painter()
            .with_clip_rect(ui.clip_rect().intersect(rect))
            .text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                title,
                crate::desktop_style::medium(crate::desktop_style::font_size(ui.ctx())),
                p.text,
            );
    }
}

impl DesktopUi {
    fn header_apps(
        &mut self,
        ui: &mut egui::Ui,
        store: &CaptureStore,
        state: &ShellStatus,
        app_width: f32,
    ) {
        let p = Palette::of(ui.ctx());
        ui.allocate_ui_with_layout(
            egui::vec2(
                app_width,
                project_size(ui, &tr(ui.ctx(), "All"), self.app.is_none()).y,
            ),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let frame = egui::Frame::NONE
                    .fill(p.surface)
                    .corner_radius(0)
                    .inner_margin(0)
                    .show(ui, |ui| {
                        ui.set_width(app_width);
                        ui.spacing_mut().interact_size.y =
                            project_size(ui, &tr(ui.ctx(), "All"), self.app.is_none()).y;

                        egui::ScrollArea::horizontal()
                            .scroll_bar_visibility(
                                egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                            )
                            .id_salt("apps-scroll")
                            .max_width(app_width)
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = 0.;
                                    if self
                                        .probe
                                        .record(
                                            "app:all",
                                            project_chip(
                                                ui,
                                                &tr(ui.ctx(), "All"),
                                                self.app.is_none(),
                                                app_width,
                                            ),
                                        )
                                        .clicked()
                                    {
                                        self.app = None;
                                        self.selected_request = None;
                                        self.selected_log = None;
                                        self.raw_cache = None;
                                    }
                                    for (id, session) in store.sessions() {
                                        let label = project_label(
                                            id,
                                            session.session["appName"].as_str(),
                                            state,
                                        );
                                        if self
                                            .probe
                                            .record(
                                                format!("app:{id}"),
                                                project_chip(
                                                    ui,
                                                    &label,
                                                    self.app.as_ref() == Some(id),
                                                    app_width,
                                                )
                                                .on_hover_text(id),
                                            )
                                            .clicked()
                                        {
                                            self.app = Some(id.clone());
                                            self.selected_request = None;
                                            self.selected_log = None;
                                            self.raw_cache = None;
                                        }
                                    }
                                });
                            });
                    });
                self.probe
                    .widgets
                    .insert("project-list".into(), frame.response.rect);
            },
        );
    }
}
fn header_action(ui: &mut egui::Ui, code: u32, label: Option<&str>) -> egui::Response {
    let Some(label) = label else {
        return crate::desktop_style::icon_control(ui, code);
    };
    crate::vintage::hover_action(ui, |ui, factor| {
        let p = Palette::of(ui.ctx());
        let mut job = header_action_job(ui, code, Some(label));
        for section in &mut job.sections {
            if section.format.font_id.family == egui::FontFamily::Name("MaterialIcons".into()) {
                section.format.font_id.size *= factor;
            }
        }
        let size = egui::vec2(0., 24.) * factor;
        ui.scope(|ui| {
            let font = egui::FontId::proportional(crate::desktop_style::local_font_size(ui));
            let line_height = ui.fonts_mut(|f| f.row_height(&font));
            let target_height = 24. + (font.size - 14.).max(0.);
            let padding_y = ((target_height - line_height) / 2.).clamp(0., 3.);
            ui.spacing_mut().button_padding = egui::vec2(6. * factor, padding_y);
            ui.add(
                egui::Button::new(job)
                    .fill(p.control)
                    .stroke(egui::Stroke::new(1.0_f32, p.line))
                    .corner_radius(crate::desktop_style::BUTTON_RADIUS)
                    .min_size(size),
            )
        })
        .inner
    })
}
fn header_action_job(ui: &egui::Ui, code: u32, label: Option<&str>) -> egui::text::LayoutJob {
    let p = Palette::of(ui.ctx());
    let mut job = egui::text::LayoutJob::default();
    job.append(
        &char::from_u32(code).unwrap().to_string(),
        0.,
        egui::TextFormat {
            font_id: egui::FontId::new(14., egui::FontFamily::Name("MaterialIcons".into())),
            color: p.accent,
            valign: egui::Align::Center,
            ..Default::default()
        },
    );
    if let Some(label) = label {
        job.append(
            label,
            crate::desktop_style::ICON_TEXT_GAP,
            egui::TextFormat {
                font_id: egui::FontId::proportional(crate::desktop_style::local_font_size(ui)),
                color: p.accent,
                valign: egui::Align::Center,
                ..Default::default()
            },
        );
    }
    job
}

fn project_label(id: &str, name: Option<&str>, _state: &ShellStatus) -> String {
    name.unwrap_or(id).to_owned()
}
fn project_size(ui: &egui::Ui, label: &str, selected: bool) -> egui::Vec2 {
    ui.painter()
        .layout_no_wrap(
            label.into(),
            if selected {
                crate::desktop_style::semibold(crate::desktop_style::local_font_size(ui))
            } else {
                egui::FontId::proportional(crate::desktop_style::local_font_size(ui))
            },
            Palette::of(ui.ctx()).text,
        )
        .size()
        + egui::vec2(16., 8.)
}
fn project_chip(ui: &mut egui::Ui, label: &str, selected: bool, max_width: f32) -> egui::Response {
    crate::vintage::hover_action(ui, |ui, _| {
        let p = Palette::of(ui.ctx());
        let size = project_size(ui, label, selected);
        let (rect, response) = ui.allocate_exact_size(
            egui::vec2(size.x.min(max_width), size.y),
            egui::Sense::click(),
        );
        if selected {
            ui.painter()
                .rect_filled(rect, crate::desktop_style::BUTTON_RADIUS, p.accent);
        }
        let color = if selected { p.on_accent } else { p.muted };
        ui.painter()
            .with_clip_rect(rect.shrink2(egui::vec2(8., 4.)).intersect(ui.clip_rect()))
            .text(
                rect.min + egui::vec2(8., 4.),
                egui::Align2::LEFT_TOP,
                label,
                if selected {
                    crate::desktop_style::semibold(crate::desktop_style::local_font_size(ui))
                } else {
                    egui::FontId::proportional(crate::desktop_style::local_font_size(ui))
                },
                color,
            );
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::SelectableLabel, ui.is_enabled(), label)
        });
        response
    })
}

fn group_separator(ui: &mut egui::Ui) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(13., 24.), egui::Sense::hover());
    ui.painter().vline(
        r.center().x,
        r.y_range().shrink(4.),
        (1., Palette::of(ui.ctx()).line),
    );
}
fn compact_tab(
    ui: &mut egui::Ui,
    label: &str,
    code: u32,
    selected: bool,
    width: f32,
) -> egui::Response {
    crate::vintage::hover_action(ui, |ui, factor| {
        let p = Palette::of(ui.ctx());
        let (r, response) = ui.allocate_exact_size(
            egui::vec2(
                if width < 44. {
                    ICON_BUTTON_SIDE * factor
                } else {
                    width * factor
                },
                if width < 44. {
                    ICON_BUTTON_SIDE * factor
                } else {
                    (24. + (crate::desktop_style::local_font_size(ui) / factor - 14.).max(0.))
                        * factor
                },
            ),
            egui::Sense::click(),
        );
        if width < 44. {
            if selected {
                ui.painter()
                    .rect_filled(r, crate::desktop_style::BUTTON_RADIUS, p.accent);
            }
            paint_icon(
                ui.painter(),
                r.center(),
                code,
                ICON_BUTTON_GLYPH_SIZE * factor,
                if selected { p.on_accent } else { p.muted },
            );
            return response.on_hover_text(label);
        }
        if selected {
            ui.painter()
                .rect_filled(r, crate::desktop_style::BUTTON_RADIUS, p.accent);
        }
        let color = if selected { p.on_accent } else { p.muted };
        let font = if selected {
            crate::desktop_style::semibold(crate::desktop_style::local_font_size(ui))
        } else {
            egui::FontId::proportional(crate::desktop_style::local_font_size(ui))
        };
        let text = ui.painter().layout_no_wrap(label.into(), font, color);
        let icon_side = 14. * factor;
        let prefix = icon_side + crate::desktop_style::ICON_TEXT_GAP;
        let x = r.center().x - (text.size().x + prefix) / 2.;
        let painter = ui.painter().with_clip_rect(r.intersect(ui.clip_rect()));
        paint_icon(
            &painter,
            egui::pos2(x + icon_side / 2., r.center().y),
            code,
            14. * factor,
            color,
        );
        painter.galley(
            egui::pos2(x + prefix, r.center().y - text.size().y / 2.),
            text,
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
        response
    })
}
