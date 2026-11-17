//! Network table geometry mirrors lib/ui/network_request_table.dart.
use super::*;
use crate::desktop_style::Palette;
impl DesktopUi {
    pub(super) fn network_table(
        &mut self,
        ui: &mut egui::Ui,
        store: &CaptureStore,
        records: &[&NetworkRecord],
        widths: &mut [f32; 3],
    ) {
        let p = Palette::of(ui.ctx());
        let font_size = crate::desktop_style::font_size(ui.ctx());
        // 4px left inset + actual title + 2px gap + 24pt filter + 4px right inset.
        let minimums = ["Method", "Status"].map(|title| {
            ui.painter()
                .layout_no_wrap(
                    tr(ui.ctx(), title),
                    egui::FontId::proportional(font_size),
                    p.muted,
                )
                .size()
                .x
                .ceil()
                + 10.
                + crate::desktop_style::ICON_BUTTON_SIDE
        });
        let method_min = minimums[0].max(64.);
        let status_min = minimums[1].max(56.);
        widths[0] = widths[0].max(method_min);
        widths[1] = widths[1].max(status_min);
        let row_height = (font_size * 2. + 24.).max(52.);
        let total = ui.available_width().max(widths.iter().sum::<f32>() + 158.);
        self.probe.table_content_width = total;
        egui::ScrollArea::horizontal()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .id_salt("network-table")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_min_width(total);
                ui.spacing_mut().item_spacing.y = 0.;
                let x = ui.cursor().left();
                let y = ui.cursor().top();
                let sizes = [
                    widths[0],
                    total - widths.iter().sum::<f32>() - 18.,
                    widths[1],
                    widths[2],
                ];
                let mut edges = [x; 5];
                for i in 0..4 {
                    edges[i + 1] = edges[i] + sizes[i] + if i < 3 { 6. } else { 0. };
                }
                let (header, _) =
                    ui.allocate_exact_size(egui::vec2(total, 30.), egui::Sense::hover());
                ui.painter().rect_filled(header, 0, p.surface);
                ui.painter()
                    .rect_stroke(header, 0, (1., p.line), egui::StrokeKind::Inside);
                for (i, (key, title)) in [
                    ("method", "Method"),
                    ("url", "URL"),
                    ("status", "Status"),
                    ("time", "Time"),
                ]
                .iter()
                .enumerate()
                {
                    let cell =
                        Rect::from_min_size(egui::pos2(edges[i], y), egui::vec2(sizes[i], 30.));
                    self.probe.widgets.insert(format!("header:{key}"), cell);
                    let filter = match i {
                        0 => Some((
                            "method",
                            &mut self.method,
                            &[
                                "All methods",
                                "GET",
                                "POST",
                                "PUT",
                                "PATCH",
                                "DELETE",
                                "HEAD",
                                "OPTIONS",
                            ][..],
                        )),
                        2 => Some((
                            "status",
                            &mut self.status,
                            &["All statuses", "Success", "Errors", "Pending"][..],
                        )),
                        _ => None,
                    };
                    let mut title_clip = cell;
                    if let Some((key, selected, choices)) = filter {
                        let label = |ctx: &egui::Context, value: &str| match value {
                            "All methods" | "All statuses" => "ALL".into(),
                            _ => tr(ctx, value),
                        };
                        let response = crate::desktop_style::table_filter_button(
                            ui,
                            cell,
                            ui.id().with(key),
                            label(ui.ctx(), selected),
                        );
                        title_clip.max.x = response.rect.left() - 2.;
                        egui::Popup::menu(&response).show(|ui| {
                            for &value in choices {
                                if self
                                    .probe
                                    .record(
                                        format!("{key}:{value}"),
                                        crate::desktop_style::option_value(
                                            ui,
                                            selected,
                                            value.into(),
                                            label(ui.ctx(), value),
                                        ),
                                    )
                                    .clicked()
                                {
                                    ui.close();
                                }
                            }
                        });
                        self.probe.record(key, response);
                    }
                    ui.painter()
                        .with_clip_rect(title_clip.intersect(ui.clip_rect()))
                        .text(
                            cell.left_center()
                                + egui::vec2(if i == 0 || i == 2 { 4. } else { 8. }, 0.),
                            egui::Align2::LEFT_CENTER,
                            tr(ui.ctx(), title),
                            egui::FontId::proportional(crate::desktop_style::font_size(ui.ctx())),
                            p.muted,
                        );
                }
                ui.painter()
                    .hline(header.x_range(), header.bottom(), (1., p.line));
                for i in 0..3 {
                    let hit =
                        Rect::from_min_size(egui::pos2(edges[i + 1] - 6., y), egui::vec2(6., 30.));
                    let response = ui
                        .interact(hit, ui.id().with(("column", i)), egui::Sense::drag())
                        .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                    ui.painter()
                        .vline(hit.center().x, header.y_range(), (1., p.line));
                    if response.dragged() && !ui.input(|i| i.pointer.any_pressed()) {
                        let delta = response.drag_delta().x;
                        let spare = (sizes[1] - 140.).max(0.);
                        match i {
                            0 => {
                                widths[0] = (widths[0] + delta)
                                    .clamp(method_min, (widths[0] + spare).min(1600.))
                            }
                            1 => {
                                widths[1] = (widths[1] - delta)
                                    .clamp(status_min, (widths[1] + spare).min(1600.))
                            }
                            _ => {
                                let sum = widths[1] + widths[2];
                                widths[1] = (widths[1] + delta)
                                    .clamp(status_min.max(sum - 1600.), 1600_f32.min(sum - 96.));
                                widths[2] = sum - widths[1];
                            }
                        }
                    }
                    self.probe.record(format!("column:{i}"), response);
                }
                if records.is_empty() {
                    self.no_results(ui);
                    return;
                }
                egui::ScrollArea::vertical()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .id_salt("network-rows")
                    .auto_shrink([false, false])
                    .show_viewport(ui, |ui, viewport| {
                        // ScrollArea expands its clip by the visual margin (3px).
                        // Rows must never paint or receive clicks over the fixed header.
                        let mut clip = ui.clip_rect();
                        clip.min.y = clip.min.y.max(header.bottom());
                        ui.set_clip_rect(clip);
                        let start = ((viewport.min.y.max(0.) / row_height).floor() as usize)
                            .min(records.len());
                        let end =
                            ((viewport.max.y / row_height).ceil() as usize + 2).min(records.len());
                        ui.add_space(start as f32 * row_height);
                        for record in records.iter().take(end).skip(start) {
                            self.probe.network_rows_rendered += 1;
                            let (row, response) = ui.allocate_exact_size(
                                egui::vec2(total, row_height),
                                egui::Sense::click(),
                            );
                            let key = (record.session_id.clone(), record.request_id.clone());
                            let selected = self.selected_request.as_ref() == Some(&key);
                            ui.painter().rect_filled(
                                row,
                                0,
                                if selected { p.accent } else { p.surface },
                            );
                            ui.painter().rect_stroke(
                                row,
                                0,
                                (1., p.line),
                                egui::StrokeKind::Inside,
                            );
                            ui.painter()
                                .hline(row.x_range(), row.bottom(), (1., p.line));
                            let cells: Vec<_> = (0..4)
                                .map(|i| {
                                    Rect::from_min_size(
                                        egui::pos2(edges[i], row.top()),
                                        egui::vec2(sizes[i], row_height),
                                    )
                                })
                                .collect();
                            let fg = if selected { p.on_accent } else { p.text };
                            let muted = if selected { p.on_accent } else { p.muted };
                            let method = text(&record.request_event["request"]["method"]);
                            let status = tr(ui.ctx(), &record_status(record));
                            paint_cell(ui, cells[0], method, fg);
                            paint_cell(ui, cells[2], &status, fg);
                            for edge in edges.iter().take(4).skip(1) {
                                ui.painter().vline(*edge - 3., row.y_range(), (1., p.line));
                            }
                            let raw = text(&record.request_event["request"]["url"]);
                            let uri = url::Url::parse(raw).ok();
                            let path = uri
                                .as_ref()
                                .map(|u| {
                                    format!(
                                        "{}{}",
                                        if u.path().is_empty() { "/" } else { u.path() },
                                        u.query().map(|q| format!("?{q}")).unwrap_or_default()
                                    )
                                })
                                .unwrap_or_else(|| raw.to_string());
                            let app = store
                                .sessions()
                                .get(&record.session_id)
                                .and_then(|s| s.session["appName"].as_str())
                                .unwrap_or("");
                            let time = chrono::DateTime::parse_from_rfc3339(text(
                                &record.request_event["timestamp"],
                            ))
                            .ok()
                            .map(|t| {
                                t.with_timezone(&chrono::Local)
                                    .format("%H:%M:%S%.3f")
                                    .to_string()
                            })
                            .unwrap_or_else(|| "—".into());
                            let meta = format!(
                                "{} · {} · {}",
                                uri.as_ref().and_then(|u| u.host_str()).unwrap_or(""),
                                app,
                                time
                            );
                            let painter = ui.painter().with_clip_rect(
                                cells[1]
                                    .shrink2(egui::vec2(12., 0.))
                                    .intersect(ui.clip_rect()),
                            );
                            painter.text(
                                cells[1].min + egui::vec2(12., row_height * 0.32),
                                egui::Align2::LEFT_CENTER,
                                path,
                                egui::FontId::proportional(crate::desktop_style::font_size(
                                    ui.ctx(),
                                )),
                                fg,
                            );
                            painter.text(
                                cells[1].min + egui::vec2(12., row_height * 0.69),
                                egui::Align2::LEFT_CENTER,
                                meta,
                                egui::FontId::proportional(crate::desktop_style::font_size(
                                    ui.ctx(),
                                )),
                                muted,
                            );
                            let duration = record.completion_event["durationMs"]
                                .as_f64()
                                .map(|v| format!("{v:.0} ms"))
                                .unwrap_or_else(|| "—".into());
                            ui.painter()
                                .with_clip_rect(cells[3].intersect(ui.clip_rect()))
                                .text(
                                    cells[3].right_center() - egui::vec2(12., 0.),
                                    egui::Align2::RIGHT_CENTER,
                                    duration,
                                    egui::FontId::proportional(crate::desktop_style::font_size(
                                        ui.ctx(),
                                    )),
                                    muted,
                                );
                            self.probe
                                .widgets
                                .insert(format!("request:{}:{}", key.0, key.1), cells[1]);
                            self.probe
                                .widgets
                                .insert(format!("row:{}:{}", key.0, key.1), row);
                            if response
                                .on_hover_cursor(egui::CursorIcon::PointingHand)
                                .clicked()
                            {
                                self.selected_request = Some(key);
                            }
                        }
                        ui.add_space(records.len().saturating_sub(end) as f32 * row_height);
                    });
            });
    }
}
fn paint_cell(ui: &egui::Ui, cell: Rect, text: &str, color: egui::Color32) {
    ui.painter()
        .with_clip_rect(cell.intersect(ui.clip_rect()))
        .text(
            cell.left_center() + egui::vec2(8., 0.),
            egui::Align2::LEFT_CENTER,
            text,
            egui::FontId::proportional(crate::desktop_style::font_size(ui.ctx())),
            color,
        );
}
