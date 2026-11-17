//! Selected body owns its preview resources. Egui paints into the same clipped
//! surface as menus/modals, so there are no child native hosts to obscure overlays.
use crate::desktop_support::tr;
use crate::{desktop::UiProbe, form_body::*, image_body::*};
use eframe::egui::{self, TextureHandle};
use serde_json::Value;

#[derive(Default)]
pub struct BodyViewer {
    revision: Option<u64>,
    identity: Option<BodyIdentity>,
    form: Option<FormBody>,
    image: bool,
    texture: Option<TextureHandle>,
    pending: Option<std::sync::mpsc::Receiver<Result<ImagePixels, String>>>,
    wanted: Option<std::sync::Arc<[u8]>>,
    life: std::sync::Arc<()>,
    notice: String,
    error: String,
    raw: bool,
    enlarged: bool,
    zoom: f32,
    pan: egui::Vec2,
}
impl BodyViewer {
    pub fn load(&mut self, data: &Value, revision: u64) -> bool {
        if self.revision == Some(revision) {
            return false;
        }
        let mime = body_content_type(&data["headers"]);
        let body = if data["body"].is_null() {
            &data["bodyPreview"]
        } else {
            &data["body"]
        };
        if self.identity.as_ref().is_some_and(|old| {
            old.body == *body
                && old.mime == mime
                && old.encoding == data["bodyEncoding"]
                && old.omitted == data["bodyOmitted"]
                && old.limit == data["bodyCaptureLimit"]
        }) {
            self.revision = Some(revision);
            return false;
        }
        let identity = BodyIdentity {
            body: body.clone(),
            mime: mime.clone(),
            encoding: data["bodyEncoding"].clone(),
            omitted: data["bodyOmitted"].clone(),
            limit: data["bodyCaptureLimit"].clone(),
        };
        *self = Self {
            revision: Some(revision),
            identity: Some(identity),
            zoom: 1.,
            ..Default::default()
        };
        if let Some(reason) = omitted_body_reason(data) {
            self.notice = match reason {
                OmittedBodyReason::SizeLimit { limit_bytes } => {
                    format!("Body omitted: capture limit {limit_bytes} bytes")
                }
                OmittedBodyReason::ReadFailed => "Body unavailable: read failed".into(),
            };
            return true;
        }
        if is_form_body(&mime) {
            self.form = Some(parse_form_body(
                &serde_json::json!({"body": if data["body"].is_null() { &data["bodyPreview"] } else { &data["body"] }, "contentType":mime,"encoding":data["bodyEncoding"]}),
            ));
        } else {
            let image = image_body_from_capture(data);
            self.image = image.image;
            if let Some(bytes) = image.bytes {
                self.wanted = Some(bytes);
            } else if self.image {
                self.error = "Image unavailable; captured raw data is preserved".into();
            }
        }
        true
    }
    /// True when a non-JSON preview (or its raw variant) consumed the content.
    pub fn show(&mut self, ui: &mut egui::Ui, probe: &mut UiProbe, raw: &str) -> bool {
        if let Some(bytes) = self.wanted.take() {
            let (tx, rx) = std::sync::mpsc::channel();
            let task = DecodeTask {
                bytes,
                life: std::sync::Arc::downgrade(&self.life),
                tx,
            };
            match decoder().try_send(task) {
                Ok(()) => self.pending = Some(rx),
                Err(std::sync::mpsc::TrySendError::Full(task)) => {
                    self.wanted = Some(task.bytes);
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(16));
                }
                Err(_) => self.error = "Image unavailable".into(),
            }
        }
        if let Some(rx) = &self.pending {
            match rx.try_recv() {
                Ok(result) => {
                    self.pending = None;
                    match result {
                        Ok(pixels) => {
                            self.texture = Some(ui.ctx().load_texture(
                                "captured-image",
                                egui::ColorImage::from_rgba_unmultiplied(
                                    [pixels.width as usize, pixels.height as usize],
                                    &pixels.rgba,
                                ),
                                egui::TextureOptions::LINEAR,
                            ))
                        }
                        Err(_) => {
                            self.error = "Image unavailable; captured raw data is preserved".into()
                        }
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => ui
                    .ctx()
                    .request_repaint_after(std::time::Duration::from_millis(16)),
                Err(_) => {
                    self.pending = None;
                    self.error = "Image unavailable".into();
                }
            }
        }
        probe.image_textures = usize::from(self.texture.is_some());
        let issue = if !self.error.is_empty() {
            Some((
                crate::desktop_support::NoticeKind::Error,
                self.error.as_str(),
            ))
        } else if !self.notice.is_empty() {
            Some((
                crate::desktop_support::NoticeKind::Warning,
                self.notice.as_str(),
            ))
        } else if self.form.as_ref().is_some_and(|form| form.incomplete) {
            Some((
                crate::desktop_support::NoticeKind::Warning,
                "Incomplete form: captured data is truncated or malformed",
            ))
        } else {
            None
        };
        crate::desktop_support::notify_changed(
            ui.ctx(),
            ui.id().with("body-preview-notice"),
            issue.map_or(crate::desktop_support::NoticeKind::Info, |(kind, _)| kind),
            issue.map(|(_, message)| tr(ui.ctx(), message)),
        );
        if !self.notice.is_empty() {
            probe.body_kind = "omitted".into();
            probe.body_preview = tr(ui.ctx(), &self.notice);
            ui.add_space(12.);
            ui.label(crate::vintage::empty_text(ui.ctx()));
            return true;
        }
        if self.form.is_none() && !self.image {
            return false;
        }
        probe.body_kind = if self.form.is_some() { "form" } else { "image" }.into();
        ui.add_space(12.);
        ui.horizontal(|ui| {
            probe.record(
                "body-preview",
                crate::desktop_style::selectable_value(
                    ui,
                    &mut self.raw,
                    false,
                    tr(ui.ctx(), if self.image { "Image" } else { "Form" }),
                ),
            );
            probe.record(
                "body-raw",
                crate::desktop_style::selectable_value(
                    ui,
                    &mut self.raw,
                    true,
                    tr(ui.ctx(), "Raw"),
                ),
            );
            if self.texture.is_some()
                && probe
                    .record(
                        "image-open",
                        crate::desktop_style::button(ui, tr(ui.ctx(), "Enlarge")),
                    )
                    .clicked()
            {
                self.enlarged = true;
            }
        });
        if self.raw {
            egui::ScrollArea::both()
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .id_salt("body-raw-scroll")
                .show(ui, |ui| {
                    ui.add(
                        egui::Label::new(egui::RichText::new(raw).font(
                            egui::FontId::proportional(crate::desktop_style::font_size(ui.ctx())),
                        ))
                        .selectable(true)
                        .wrap_mode(egui::TextWrapMode::Extend),
                    );
                });
        } else if let Some(form) = &self.form {
            egui::ScrollArea::both()
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .id_salt("body-form-scroll")
                .show(ui, |ui| {
                    ui.set_min_width(520.);
                    ui.spacing_mut().item_spacing.y = 0.;
                    let row_height = (crate::desktop_style::font_size(ui.ctx()) + 20.).max(38.);
                    let width = ui.available_width();
                    let widths = [160., width - 224., 64.];
                    let (header, _) =
                        ui.allocate_exact_size(egui::vec2(width, row_height), egui::Sense::hover());
                    crate::vintage::paint_row(
                        ui,
                        header,
                        &["Title", "Value", "Action"],
                        &widths,
                        false,
                        true,
                    );
                    probe.widgets.insert("body-form-header".into(), header);
                    egui::ScrollArea::vertical()
                        .id_salt("body-form-rows")
                        .show_rows(ui, row_height, form.parts.len(), |ui, rows| {
                            for index in rows {
                                let part = &form.parts[index];
                                let value = part.value.clone().unwrap_or_else(|| {
                                    format!(
                                        "{} · {} · {} bytes",
                                        part.filename.as_deref().unwrap_or("file"),
                                        part.content_type.as_deref().unwrap_or("binary"),
                                        part.byte_length
                                    )
                                });
                                let text = format!("{}: {}", part.name, value);
                                probe.body_preview.push_str(&text);
                                let (row, _) = ui.allocate_exact_size(
                                    egui::vec2(width, row_height),
                                    egui::Sense::hover(),
                                );
                                crate::vintage::paint_row(
                                    ui,
                                    row,
                                    &[&part.name, "", ""],
                                    &widths,
                                    false,
                                    false,
                                );
                                probe.widgets.insert(format!("body-form-row:{index}"), row);
                                let value_cell = egui::Rect::from_min_max(
                                    row.min + egui::vec2(widths[0] + 8., 2.),
                                    row.max - egui::vec2(widths[2] + 8., 2.),
                                );
                                let mut cell = ui.new_child(
                                    egui::UiBuilder::new()
                                        .id_salt(("form-value", index))
                                        .max_rect(value_cell)
                                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                                );
                                if let Some(argb) = crate::body_color::parse_body_color(&value) {
                                    color_swatch(
                                        &mut cell,
                                        probe,
                                        &format!("body-color:{index}"),
                                        &value,
                                        argb,
                                    );
                                }
                                cell.add(
                                    egui::Label::new(&value)
                                        .selectable(true)
                                        .truncate()
                                        .show_tooltip_when_elided(false),
                                );
                                let center =
                                    egui::pos2(row.right() - widths[2] / 2., row.center().y);
                                let mut action = ui.new_child(
                                    egui::UiBuilder::new()
                                        .id_salt(("form-copy", index))
                                        .max_rect(egui::Rect::from_center_size(
                                            center,
                                            egui::vec2(34., 34.),
                                        ))
                                        .layout(egui::Layout::top_down(egui::Align::Center)),
                                );
                                if probe
                                    .record(
                                        format!("body-form-copy:{index}"),
                                        crate::desktop_style::copy_button(
                                            &mut action,
                                            "Copy value",
                                        ),
                                    )
                                    .clicked()
                                {
                                    crate::desktop_support::copy(ui.ctx(), value);
                                }
                            }
                        });
                });
        } else if let Some(texture) = &self.texture {
            let available = ui.available_size().max(egui::vec2(1., 1.));
            let scale = (available.x / texture.size_vec2().x)
                .min(available.y / texture.size_vec2().y)
                .min(1.);
            let response =
                ui.add(egui::Image::new(texture).fit_to_exact_size(texture.size_vec2() * scale));
            let mut response = response;
            response.rect = response.rect.intersect(ui.clip_rect());
            probe.record("image-preview", response);
        } else if self.pending.is_some() || self.wanted.is_some() {
            crate::vintage::loading(ui, 3);
        } else {
            probe.body_preview = tr(ui.ctx(), &self.error);
            ui.label(crate::vintage::empty_text(ui.ctx()));
        }
        if self.enlarged
            && let Some(texture) = &self.texture
        {
            let modal = egui::Modal::new(ui.id().with("image-enlarged")).show(ui.ctx(), |ui| {
                ui.set_width((ui.ctx().content_rect().width() - 100.).max(120.));
                ui.horizontal(|ui| {
                    if probe
                        .record(
                            "image-zoom-in",
                            crate::desktop_style::button(ui, tr(ui.ctx(), "+")),
                        )
                        .clicked()
                    {
                        self.zoom = (self.zoom * 1.25).min(16.);
                    }
                    if probe
                        .record(
                            "image-zoom-out",
                            crate::desktop_style::button(ui, tr(ui.ctx(), "-")),
                        )
                        .clicked()
                    {
                        self.zoom = (self.zoom / 1.25).max(0.05);
                    }
                    if probe
                        .record(
                            "image-fit",
                            crate::desktop_style::button(ui, tr(ui.ctx(), "Fit")),
                        )
                        .clicked()
                    {
                        self.zoom = 1.;
                        self.pan = egui::Vec2::ZERO;
                    }
                    if probe
                        .record(
                            "image-close",
                            crate::desktop_style::button(ui, tr(ui.ctx(), "Close")),
                        )
                        .clicked()
                    {
                        self.enlarged = false;
                    }
                });
                let available = egui::vec2(
                    ui.available_width(),
                    (ui.ctx().content_rect().height() - 180.).max(80.),
                );
                let (rect, response) = ui.allocate_exact_size(available, egui::Sense::drag());
                if response.dragged() {
                    self.pan += response.drag_delta();
                }
                if response.hovered() {
                    self.zoom = (self.zoom * ui.input(|i| i.zoom_delta())).clamp(0.05, 16.);
                }
                let fit = (rect.width() / texture.size_vec2().x)
                    .min(rect.height() / texture.size_vec2().y)
                    .min(1.);
                let target = egui::Rect::from_center_size(
                    rect.center() + self.pan,
                    texture.size_vec2() * fit * self.zoom,
                );
                ui.painter().with_clip_rect(rect).image(
                    texture.id(),
                    target,
                    egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1., 1.)),
                    egui::Color32::WHITE,
                );
                probe.record("image-modal", response);
            });
            if modal.should_close() {
                self.enlarged = false;
            }
        }
        probe.image_zoom = self.zoom;
        probe.image_pan = self.pan;
        true
    }
}

struct DecodeTask {
    bytes: std::sync::Arc<[u8]>,
    life: std::sync::Weak<()>,
    tx: std::sync::mpsc::Sender<Result<ImagePixels, String>>,
}
fn decoder() -> &'static std::sync::mpsc::SyncSender<DecodeTask> {
    static DECODER: std::sync::OnceLock<std::sync::mpsc::SyncSender<DecodeTask>> =
        std::sync::OnceLock::new();
    DECODER.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::sync_channel::<DecodeTask>(1);
        std::thread::spawn(move || {
            while let Ok(task) = rx.recv() {
                if task.life.upgrade().is_some() {
                    let result = ImageAsset::new(task.bytes).into_pixels();
                    if task.life.upgrade().is_some() {
                        let _ = task.tx.send(result);
                    }
                }
            }
        });
        tx
    })
}

pub(crate) fn color_swatch(
    ui: &mut egui::Ui,
    probe: &mut UiProbe,
    key: &str,
    value: &str,
    argb: u32,
) {
    let [a, r, g, b] = argb.to_be_bytes();
    let response = ui
        .add(egui::Button::new("   ").fill(egui::Color32::from_rgba_unmultiplied(r, g, b, a)))
        .on_hover_text(value);
    if probe.record(key, response).clicked() {
        crate::desktop_support::copy(ui.ctx(), value.to_owned());
    }
}

struct BodyIdentity {
    body: Value,
    mime: String,
    encoding: Value,
    omitted: Value,
    limit: Value,
}
