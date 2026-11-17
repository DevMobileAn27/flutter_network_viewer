//! Desktop renderer shared by the native host and input-driven UI tests.
use crate::desktop_support::{NoticeKind, Notifications, SupportUi, tr};
use crate::{
    capture::{CaptureStore, ConsoleRecord, NetworkRecord},
    discovery::Target,
    settings::AppSettings,
};
use eframe::egui::{self, Rect, Response};
use std::collections::{BTreeMap, HashSet};
#[path = "desktop_network.rs"]
mod network_table;
#[path = "desktop_shell.rs"]
mod shell;

const TABS: [&str; 6] = [
    "Overview",
    "Request header",
    "Request body",
    "Response header",
    "Response body",
    "Raw",
];
struct MessageTag {
    text: String,
    needle: String,
}

const CONSOLE_DEFAULT_WIDTHS: [f32; 3] = [100., 86., 120.];
const CONSOLE_MIN_WIDTHS: [f32; 4] = [64., 64., 64., 80.];

fn console_column_layout(width: f32, preferred: [f32; 3]) -> ([f32; 4], [f32; 4]) {
    let min_total = CONSOLE_MIN_WIDTHS.iter().sum::<f32>();
    let minimum = CONSOLE_MIN_WIDTHS.map(|w| w * (width / min_total).min(1.));
    let spare = (width - minimum.iter().sum::<f32>()).max(0.);
    let preferred_extra: [f32; 3] = std::array::from_fn(|i| (preferred[i] - minimum[i]).max(0.));
    let extra_total = preferred_extra.iter().sum::<f32>();
    let ratio = if extra_total > spare {
        spare / extra_total
    } else {
        1.
    };
    let mut widths = minimum;
    for i in 0..3 {
        widths[i] += ratio * preferred_extra[i];
    }
    widths[3] = (width - widths[..3].iter().sum::<f32>()).max(0.);
    (widths, minimum)
}
const CONSOLE_CELL_X_PADDING: f32 = 8.;
const CONSOLE_MESSAGE_Y_PADDING: f32 = 6.;
const CONSOLE_MIN_ROW_HEIGHT: f32 = 36.;

fn console_message_layout(
    log: &ConsoleRecord,
    font: &egui::FontId,
    width: f32,
) -> egui::text::LayoutJob {
    egui::text::LayoutJob::simple(
        log.display_message(),
        font.clone(),
        egui::Color32::PLACEHOLDER,
        width,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    CheckUpdates,
    RetryMcp,
    StartScan,
    StopScan,
    SelectTarget(String),
    Connect(String),
    SetPaused(bool),
    Clear,
}
#[derive(Default)]
pub struct ShellStatus {
    pub searching: bool,
    pub busy: bool,
    pub targets: Vec<Target>,
    pub online: HashSet<String>,
    pub error: Option<String>,
}
/// Geometry and displayed values recorded from actual widgets, for UI automation.
pub struct UiProbe {
    pub resource_text: String,
    pub support_text: String,
    pub notification_title: String,
    pub notification_color: egui::Color32,
    pub support_scroll: f32,
    pub body_kind: String,
    pub body_preview: String,
    pub image_textures: usize,
    pub image_zoom: f32,
    pub image_pan: egui::Vec2,
    pub retained_detail_bytes: usize,
    pub network_rows_rendered: usize,
    pub console_rows_rendered: usize,
    pub json: crate::desktop_json::JsonUiProbe,
    pub widgets: BTreeMap<String, Rect>,
    pub requests: Vec<(String, String)>,
    pub logs: Vec<u64>,
    pub console_scroll: f32,
    pub inspector_text: String,
    pub sidebar_width: f32,
    pub list_rect: Rect,
    pub table_content_width: f32,
}
impl Default for UiProbe {
    fn default() -> Self {
        Self {
            resource_text: String::new(),
            notification_title: String::new(),
            notification_color: egui::Color32::TRANSPARENT,
            support_text: String::new(),
            support_scroll: 0.,
            body_kind: String::new(),
            body_preview: String::new(),
            image_textures: 0,
            image_zoom: 1.,
            image_pan: egui::Vec2::ZERO,
            retained_detail_bytes: 0,
            network_rows_rendered: 0,
            console_rows_rendered: 0,
            json: Default::default(),
            widgets: BTreeMap::new(),
            requests: vec![],
            logs: vec![],
            console_scroll: 0.,
            inspector_text: String::new(),
            sidebar_width: 0.,
            list_rect: Rect::NOTHING,
            table_content_width: 0.,
        }
    }
}
impl UiProbe {
    pub(crate) fn record(&mut self, key: impl Into<String>, response: Response) -> Response {
        self.widgets.insert(key.into(), response.rect);
        response
    }
}
type DetailCache = ((String, String, u64, u64, usize), std::sync::Arc<str>);
pub struct DesktopUi {
    pub vintage_open: bool,
    pub vintage: crate::vintage::VintageDemo,
    pub memory_bytes: Option<u64>,
    pub notifications: Notifications,
    pub support: SupportUi,
    last_status_error: Option<String>,
    last_language: String,
    pub detail_preparations: u64,
    pub console_group_records_prepared: u64,
    pub body_preparations: u64,
    pub network_raw_serializations: u64,
    console_detail_cache: Option<(ConsoleRecord, Vec<(String, String)>)>,
    console_json: crate::desktop_json::JsonViewer,
    detail_cache: Option<DetailCache>,
    bodies: [crate::desktop_body::BodyViewer; 2],
    capture_epoch: u64,
    json_workspace: crate::desktop_json::JsonWorkspace,
    body_json: [crate::desktop_json::JsonViewer; 2],
    body_json_key: Option<(String, String)>,
    raw_json: crate::desktop_json::JsonViewer,
    pub settings: AppSettings,
    pub selected_request: Option<(String, String)>,
    pub selected_log: Option<u64>,
    /// Counts actual raw encodes, allowing performance regression checks.
    pub console_raw_serializations: u64,
    pub export_path: String,
    pub probe: UiProbe,
    /// Programmatic accessibility navigation to a tab, also used by UI tests.
    pub reveal_inspector_tab: Option<usize>,
    grouped_logs: Vec<ConsoleRecord>,
    console_row_heights: BTreeMap<u64, f32>,
    console_layout_key: Option<(f32, f32, f32)>,
    console_column_widths: [f32; 3],
    log_bounds: Option<(u64, u64)>,
    log_source: String,
    log_level: String,
    message_tag_input: String,
    message_tags: Vec<MessageTag>,
    console_raw: bool,
    raw_cache: Option<(ConsoleRecord, String)>,
    app: Option<String>,
    page: usize,
    query: String,
    method: String,
    status: String,
    connect_open: bool,
    export_open: bool,
    export_json: bool,
    manual_was_open: bool,
    vm_uri: String,
    message: Option<String>,
}
impl Default for DesktopUi {
    fn default() -> Self {
        Self::new(AppSettings::default())
    }
}
impl DesktopUi {
    pub fn new(settings: AppSettings) -> Self {
        let vm_uri = settings.values().vm_service_uri.clone();
        Self {
            vintage_open: false,
            vintage: Default::default(),
            memory_bytes: None,
            notifications: Default::default(),
            support: Default::default(),
            last_status_error: None,
            last_language: String::new(),
            detail_preparations: 0,
            console_group_records_prepared: 0,
            body_preparations: 0,
            network_raw_serializations: 0,
            console_detail_cache: None,
            console_json: Default::default(),
            detail_cache: None,
            bodies: Default::default(),
            capture_epoch: 0,
            json_workspace: Default::default(),
            body_json: Default::default(),
            body_json_key: None,
            raw_json: Default::default(),
            settings,
            selected_request: None,
            selected_log: None,
            console_raw_serializations: 0,
            grouped_logs: vec![],
            console_row_heights: BTreeMap::new(),
            console_layout_key: None,
            console_column_widths: CONSOLE_DEFAULT_WIDTHS,
            log_bounds: None,
            log_source: "All log sources".into(),
            log_level: "All levels".into(),
            message_tag_input: String::new(),
            message_tags: Vec::new(),
            console_raw: false,
            raw_cache: None,
            export_path: "flutter-network-viewer.json".into(),
            probe: UiProbe::default(),
            reveal_inspector_tab: None,
            app: None,
            page: 0,
            query: String::new(),
            method: "All methods".into(),
            status: "All statuses".into(),
            connect_open: false,
            export_open: false,
            export_json: false,
            manual_was_open: false,
            vm_uri,
            message: None,
        }
    }
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        store: &CaptureStore,
        state: &ShellStatus,
    ) -> Vec<Action> {
        if !crate::desktop_support::apply(ctx, &self.settings) {
            // set_fonts takes effect on the next egui pass. Do not request a
            // named weight family until the renderer has installed it.
            ctx.layer_painter(egui::LayerId::background()).rect_filled(
                ctx.content_rect(),
                0,
                ctx.style().visuals.panel_fill,
            );
            ctx.request_discard("install bundled font faces");
            return vec![];
        }
        if self.vintage_open {
            self.probe = UiProbe::default();
            if self.vintage.show(ctx, &mut self.settings, &mut self.probe) {
                self.vintage_open = false;
            }
            return vec![];
        }
        // Panels leave spacing and resize gutters outside their own frames.
        // Cover the complete surface so those gaps never expose GL clear black.
        ctx.layer_painter(egui::LayerId::background()).rect_filled(
            ctx.content_rect(),
            0,
            ctx.style().visuals.panel_fill,
        );
        if self.last_language != self.settings.values().language_code {
            self.last_language = self.settings.values().language_code.clone();
            self.detail_cache = None;
            self.console_detail_cache = None;
        }
        self.probe = UiProbe::default();
        // Clear/removal epochs invalidate even if identical keys are reintroduced
        // before the next paint. Per-record revisions restart for a new capture.
        if self.capture_epoch != store.revision {
            self.capture_epoch = store.revision;
            self.release_detail();
            self.selected_request = None;
            self.selected_log = None;
            self.raw_cache = None;
            self.console_detail_cache = None;
            self.console_json = Default::default();
            self.grouped_logs.clear();
            self.console_row_heights.clear();
            self.log_bounds = None;
        }
        let mut actions = vec![];
        if self.support.settings_open {
            self.support.show(
                ctx,
                &mut self.settings,
                &mut self.notifications,
                &mut self.probe,
                &mut actions,
            );
            self.render_notices(ctx, state);
            return actions;
        }
        let width = ctx.content_rect().width();
        let bounds = store
            .logs()
            .front()
            .zip(store.logs().back())
            .map(|(a, b)| (a.id, b.id));
        if bounds != self.log_bounds {
            // A possible unfinished block spans at most 200 raw records. Revisit
            // the group touching that tail, preserving every earlier allocation.
            let split = self
                .log_bounds
                .filter(|old| bounds.is_some_and(|new| old.0 == new.0 && new.1 >= old.1))
                .map(|old| {
                    let tail = old.1.saturating_sub(199);
                    self.grouped_logs
                        .iter()
                        .rposition(|r| r.id <= tail)
                        .unwrap_or(0)
                })
                .unwrap_or(0);
            let from = self.grouped_logs.get(split).map_or(0, |r| r.id);
            // The rebuilt tail can change a grouped message without changing its ID.
            self.console_row_heights.split_off(&from);
            self.grouped_logs.truncate(split);
            let tail: Vec<_> = store.logs().iter().filter(|r| r.id >= from).collect();
            self.console_group_records_prepared += tail.len() as u64;
            self.grouped_logs.extend(ConsoleRecord::grouped_refs(&tail));
            self.log_bounds = bounds;
        }
        if self
            .selected_log
            .is_some_and(|id| !self.grouped_logs.iter().any(|r| r.id == id))
        {
            self.selected_log = None;
            self.raw_cache = None;
            self.console_detail_cache = None;
            self.console_json = Default::default();
        }
        if self
            .app
            .as_ref()
            .is_some_and(|id| !store.sessions().contains_key(id))
        {
            self.app = None;
        }
        if self
            .selected_request
            .as_ref()
            .is_some_and(|key| !store.network().contains_key(key))
        {
            self.selected_request = None;
        }
        if self.selected_request.is_none() && self.body_json_key.is_some() {
            self.release_detail();
        }
        self.shell(ctx, store, state, &mut actions);
        let selected = self
            .selected_request
            .as_ref()
            .and_then(|key| store.network().get(key));
        if self.page == 0 && width >= 900. {
            let max_width = (ctx.available_rect().width() - 368.).max(320.);
            let panel = egui::SidePanel::right("network-inspector")
                .resizable(true)
                .default_width(
                    self.settings
                        .values()
                        .network_inspector_width
                        .unwrap_or((width as f64 - 8.) / 2.) as f32,
                )
                .min_width(320.)
                .max_width(max_width)
                .frame(egui::Frame::NONE.fill(crate::desktop_style::Palette::of(ctx).surface))
                .show(ctx, |ui| {
                    let rect = ui.available_rect_before_wrap();
                    // Panel width belongs to the divider, never to intrinsic content size.
                    let mut content = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                    content.set_clip_rect(rect.intersect(ui.clip_rect()));
                    if let Some(record) = selected {
                        self.inspector(&mut content, record);
                    } else {
                        self.empty_state(&mut content, true);
                    }
                    ui.allocate_rect(rect, egui::Sense::hover());
                });
            if split_released(ctx, panel.response.rect) {
                let value = panel.response.rect.width() as f64;
                if let Err(e) = self.settings.set_inspector_width(value, false) {
                    self.message = Some(e.to_string());
                }
            }
        }
        if self.page == 1 && width >= 900. {
            let max_width = (ctx.available_rect().width() - 368.).max(320.);
            let panel = egui::SidePanel::right("console-inspector")
                .resizable(true)
                .default_width(self.settings.values().console_inspector_width as f32)
                .min_width(320.)
                .max_width(max_width)
                .frame(egui::Frame::NONE.fill(crate::desktop_style::Palette::of(ctx).surface))
                .show(ctx, |ui| {
                    let rect = ui.available_rect_before_wrap();
                    let mut content = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                    content.set_clip_rect(rect.intersect(ui.clip_rect()));
                    if self.selected_log.is_some() {
                        self.console_inspector(&mut content);
                    } else {
                        self.empty_state(&mut content, true);
                    }
                    ui.allocate_rect(rect, egui::Sense::hover());
                });
            self.probe
                .widgets
                .insert("console-panel".into(), panel.response.rect);
            if split_released(ctx, panel.response.rect)
                && let Err(e) = self
                    .settings
                    .set_inspector_width(panel.response.rect.width() as f64, true)
            {
                self.message = Some(e.to_string());
            }
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(crate::desktop_style::Palette::of(ctx).surface))
            .show(ctx, |ui| {
                if self.page == 2 {
                    self.json_workspace
                        .show(ui, &mut self.probe, &mut self.settings);
                    return;
                }
                self.probe.list_rect = ui.available_rect_before_wrap();
                if self.page == 0 {
                    self.network(ui, store);
                } else {
                    self.console(ui, store);
                }
            });
        if self.page == 0
            && width < 900.
            && let Some(record) = selected
        {
            let response = egui::Modal::new(egui::Id::new("request-modal")).show(ctx, |ui| {
                ui.set_width((width - 80.).clamp(240., 820.));
                ui.set_max_height(ctx.content_rect().height() - 120.);
                self.inspector(ui, record);
            });
            if response.should_close() {
                self.selected_request = None;
            }
        }
        if self.page == 1 && width < 900. && self.selected_log.is_some() {
            let response = egui::Modal::new(egui::Id::new("console-modal")).show(ctx, |ui| {
                ui.set_width((width - 80.).clamp(240., 820.));
                ui.set_max_height(ctx.content_rect().height() - 120.);
                self.console_inspector(ui);
            });
            if response.should_close() {
                self.selected_log = None;
                self.raw_cache = None;
            }
        }
        if self.connect_open {
            let modal = egui::Modal::new(egui::Id::new("manual-url-popup")).show(ctx, |ui| {
                ui.set_width((width - 64.).clamp(240., 440.));
                ui.spacing_mut().button_padding = egui::vec2(8., 3.);
                ui.spacing_mut().interact_size.y = 24.;
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(tr(ctx, "Manual URL")).font(
                        crate::desktop_style::semibold(crate::desktop_style::font_size(ui.ctx())),
                    ));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self
                            .probe
                            .record(
                                "manual-close",
                                crate::desktop_style::square_icon(ui, 0xe16a, &tr(ctx, "Close")),
                            )
                            .clicked()
                        {
                            self.connect_open = false;
                        }
                    });
                });
                ui.label("URL");
                let input = self.probe.record(
                    "vm-uri",
                    crate::desktop_style::framed_text_edit(
                        ui,
                        egui::TextEdit::singleline(&mut self.vm_uri)
                            .vertical_align(egui::Align::Center)
                            .id_salt("manual-vm-uri")
                            .hint_text(tr(ctx, "Enter URL")),
                        Some(egui::vec2(ui.available_width(), 28.)),
                    ),
                );
                if !self.manual_was_open {
                    input.request_focus();
                }
                let mut submit = false;
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    submit = self
                        .probe
                        .record(
                            "connect-submit",
                            crate::desktop_style::button(ui, tr(ctx, "Find")),
                        )
                        .clicked();
                });
                if submit || (input.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))) {
                    match crate::vm::normalize_uri(self.vm_uri.trim()) {
                        Ok(uri) => {
                            actions.push(Action::Connect(uri));
                            self.connect_open = false;
                        }
                        Err(_) => {
                            self.notifications.show(
                                NoticeKind::Error,
                                tr(ctx, "Invalid VM Service URL"),
                                false,
                            );
                        }
                    }
                }
            });
            if modal.should_close() {
                self.connect_open = false;
            }
        }
        self.manual_was_open = self.connect_open;
        if !state.targets.is_empty() {
            let response = egui::Modal::new(egui::Id::new("discovery-picker")).show(ctx, |ui| {
                ui.set_width((width - 80.).clamp(240., 480.));
                ui.heading(tr(ui.ctx(), "Choose a Flutter app"));
                egui::ScrollArea::vertical()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .max_height(350.)
                    .show(ui, |ui| {
                        for target in &state.targets {
                            if self
                                .probe
                                .record(
                                    format!("target:{}", target.vm_key),
                                    crate::vintage::hover_action(ui, |ui, factor| {
                                        ui.add_sized(
                                            [ui.available_width().min(480. * factor), 40. * factor],
                                            egui::Button::new(&target.name),
                                        )
                                    })
                                    .on_hover_text(&target.uri),
                                )
                                .clicked()
                            {
                                actions.push(Action::SelectTarget(target.vm_key.clone()));
                            }
                        }
                    });
                if self
                    .probe
                    .record(
                        "picker-cancel",
                        crate::desktop_style::button(ui, tr(ui.ctx(), "Cancel scan")),
                    )
                    .clicked()
                {
                    actions.push(Action::StopScan);
                }
            });
            if response.should_close() {
                actions.push(Action::StopScan);
            }
        }
        if self.export_open {
            let response = egui::Modal::new(egui::Id::new("export-modal")).show(ctx, |ui| {
                ui.set_width((width - 80.).clamp(240., 550.));
                ui.heading(tr(
                    ui.ctx(),
                    if self.export_json {
                        "Download JSON"
                    } else {
                        "Export capture"
                    },
                ));
                ui.label(tr(
                    ui.ctx(),
                    if self.export_json {
                        "Destination JSON file"
                    } else {
                        "Destination JSON file (all apps and records)"
                    },
                ));
                self.probe.record(
                    "export-path",
                    crate::desktop_style::framed_text_edit(
                        ui,
                        egui::TextEdit::singleline(&mut self.export_path),
                        None,
                    ),
                );
                if self
                    .probe
                    .record(
                        "save-export",
                        crate::desktop_style::button(ui, tr(ui.ctx(), "Save export")),
                    )
                    .clicked()
                {
                    let content = if self.export_json {
                        Ok(self.json_workspace.export_text().into_bytes())
                    } else {
                        serde_json::to_vec_pretty(&store.export()).map_err(std::io::Error::other)
                    };
                    match content.and_then(|bytes| std::fs::write(&self.export_path, bytes)) {
                        Ok(()) => {
                            self.export_open = false;
                            self.notifications.show(
                                NoticeKind::Success,
                                format!("{} {}", tr(ctx, "Exported to"), self.export_path),
                                false,
                            );
                        }
                        Err(e) => self.message = Some(format!("Export failed: {e}")),
                    }
                }
            });
            if response.should_close() {
                self.export_open = false;
            }
        }
        if !self.support.settings_open {
            self.support.show(
                ctx,
                &mut self.settings,
                &mut self.notifications,
                &mut self.probe,
                &mut actions,
            );
        } else {
            ctx.request_repaint();
        }
        self.render_notices(ctx, state);
        actions
    }
    fn render_notices(&mut self, ctx: &egui::Context, state: &ShellStatus) {
        for (kind, message) in crate::desktop_support::take_notifications(ctx) {
            self.notifications.show(kind, message, false);
        }
        if state.error != self.last_status_error {
            self.last_status_error = state.error.clone();
            if let Some(error) = &state.error {
                self.notifications
                    .show(NoticeKind::Error, error.clone(), false);
            }
        }
        if let Some(message) = self.message.take() {
            self.notifications.show(NoticeKind::Error, message, false);
        }
        if crate::desktop_support::copied(ctx) {
            self.notifications
                .show(NoticeKind::Success, tr(ctx, "Copied to clipboard"), false);
        }
        self.notifications.render(ctx, &mut self.probe);
    }
    fn release_detail(&mut self) {
        self.body_json = Default::default();
        self.raw_json = Default::default();
        self.body_json_key = None;
        self.bodies = Default::default();
        self.detail_cache = None;
    }
    fn network(&mut self, ui: &mut egui::Ui, store: &CaptureStore) {
        let query = self.query.to_lowercase();
        let records: Vec<_> = store
            .network()
            .values()
            .rev()
            .filter(|r| {
                (self.app.as_ref().is_none_or(|id| id == &r.session_id))
                    && (self.method == "All methods"
                        || r.request_event["request"]["method"] == self.method)
                    && match self.status.as_str() {
                        "Errors" => r.failed(),
                        "Pending" => r.pending(),
                        "Success" => r.status().is_some_and(|s| (200..400).contains(&s)),
                        _ => true,
                    }
                    && (query.is_empty()
                        || format!(
                            "{} {} {} {}",
                            text(&r.request_event["request"]["method"]),
                            text(&r.request_event["request"]["url"]),
                            record_status(r),
                            text(&r.request_event["source"])
                        )
                        .to_lowercase()
                        .contains(&query))
            })
            .collect();
        self.probe.requests = records
            .iter()
            .map(|r| (r.session_id.clone(), r.request_id.clone()))
            .collect();
        let mut columns = self
            .settings
            .values()
            .network_column_widths
            .map(|v| v as f32);
        self.network_table(ui, store, &records, &mut columns);
        if columns.map(f64::from) != self.settings.values().network_column_widths
            && let Err(e) = self
                .settings
                .set_network_column_widths(&columns.map(f64::from))
        {
            self.message = Some(e.to_string());
        }
    }
    fn console(&mut self, ui: &mut egui::Ui, _store: &CaptureStore) {
        ui.scope(|ui| {
            // The header and body share one grid, with no layout gap between them.
            ui.spacing_mut().item_spacing.y = 0.;
            self.console_table(ui);
        });
    }

    fn console_table(&mut self, ui: &mut egui::Ui) {
        let (header, edges) = self.console_header(ui);
        let [source_right, level_right, message_left] = edges;
        if self.grouped_logs.is_empty() {
            self.empty_state(ui, false);
            return;
        }

        let query = self.query.to_lowercase();
        let logs: Vec<_> = self
            .grouped_logs
            .iter()
            .filter(|r| {
                self.app.as_ref().is_none_or(|id| id == &r.session_id)
                    && (self.log_source == "All log sources" || r.source() == self.log_source)
                    && (self.log_level == "All levels" || r.level() == self.log_level)
                    && (query.is_empty() || r.search_text().to_lowercase().contains(&query))
                    && (self.message_tags.is_empty() || {
                        let message = r.display_message().to_lowercase();
                        self.message_tags
                            .iter()
                            .any(|tag| message.contains(&tag.needle))
                    })
            })
            .collect();
        self.probe.logs = logs.iter().map(|r| r.id).collect();
        if logs.is_empty() {
            self.no_results(ui);
            return;
        }
        let font_size = crate::desktop_style::font_size(ui.ctx());
        let font = egui::FontId::proportional(font_size);
        let message_width = (header.width() - message_left - 2. * CONSOLE_CELL_X_PADDING).max(1.);
        let layout_key = (message_width, font_size, ui.pixels_per_point());
        if self.console_layout_key != Some(layout_key) {
            self.console_row_heights.clear();
            self.console_layout_key = Some(layout_key);
        }
        // Measure changed rows once; prefix offsets let the scroll area paint
        // only visible rows even when every row has a different height.
        let mut offsets = Vec::with_capacity(logs.len() + 1);
        offsets.push(0.);
        ui.fonts_mut(|fonts| {
            let mut y = 0.;
            for log in &logs {
                y += *self.console_row_heights.entry(log.id).or_insert_with(|| {
                    let galley =
                        fonts.layout_job(console_message_layout(log, &font, message_width));
                    (galley.size().y + 2. * CONSOLE_MESSAGE_Y_PADDING)
                        .ceil()
                        .max(CONSOLE_MIN_ROW_HEIGHT)
                });
                offsets.push(y);
            }
        });
        let output = egui::ScrollArea::vertical()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .id_salt("console-list")
            .auto_shrink([false, false])
            .show_viewport(ui, |ui, viewport| {
                let mut clip = ui.clip_rect();
                clip.min.y = clip.min.y.max(header.bottom());
                ui.set_clip_rect(clip);
                let p = crate::desktop_style::Palette::of(ui.ctx());
                let start = offsets
                    .partition_point(|&y| y <= viewport.min.y)
                    .saturating_sub(1);
                let end = offsets
                    .partition_point(|&y| y < viewport.max.y)
                    .min(logs.len());
                ui.add_space(offsets[start]);
                ui.skip_ahead_auto_ids(start);
                for index in start..end {
                    let log = logs[index];
                    self.probe.console_rows_rendered += 1;
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(header.width(), offsets[index + 1] - offsets[index]),
                        egui::Sense::click(),
                    );
                    let selected = self.selected_log == Some(log.id);
                    ui.painter()
                        .rect_filled(rect, 0, if selected { p.accent } else { p.surface });
                    ui.painter()
                        .rect_stroke(rect, 0, (1., p.line), egui::StrokeKind::Inside);
                    for offset in edges {
                        ui.painter()
                            .vline(rect.left() + offset, rect.y_range(), (1., p.line));
                    }
                    ui.painter()
                        .hline(rect.x_range(), rect.bottom(), (1., p.line));
                    ui.painter()
                        .with_clip_rect(
                            Rect::from_min_size(
                                rect.min,
                                egui::vec2(source_right, CONSOLE_MIN_ROW_HEIGHT),
                            )
                            .intersect(ui.clip_rect()),
                        )
                        .text(
                            rect.min + egui::vec2(8., CONSOLE_MIN_ROW_HEIGHT / 2.),
                            egui::Align2::LEFT_CENTER,
                            log.source_label(),
                            egui::FontId::proportional(crate::desktop_style::font_size(ui.ctx())),
                            if selected { p.on_accent } else { p.muted },
                        );
                    let level = log.level().to_uppercase();
                    ui.painter()
                        .with_clip_rect(
                            Rect::from_min_max(
                                rect.min + egui::vec2(source_right + CONSOLE_CELL_X_PADDING, 0.),
                                egui::pos2(rect.left() + level_right, rect.bottom()),
                            )
                            .intersect(ui.clip_rect()),
                        )
                        .text(
                            rect.min
                                + egui::vec2(
                                    source_right + CONSOLE_CELL_X_PADDING,
                                    CONSOLE_MIN_ROW_HEIGHT / 2.,
                                ),
                            egui::Align2::LEFT_CENTER,
                            level,
                            egui::FontId::proportional(crate::desktop_style::font_size(ui.ctx())),
                            if selected { p.on_accent } else { p.text },
                        );
                    let time = chrono::DateTime::parse_from_rfc3339(text(&log.event["timestamp"]))
                        .ok()
                        .map(|t| {
                            t.with_timezone(&chrono::Local)
                                .format("%H:%M:%S%.3f")
                                .to_string()
                        })
                        .unwrap_or_else(|| "—".into());
                    let time_pos = egui::pos2(
                        rect.left() + level_right + CONSOLE_CELL_X_PADDING,
                        rect.top() + 14.,
                    );
                    let time_clip = Rect::from_min_max(
                        rect.min + egui::vec2(level_right, 0.),
                        egui::pos2(rect.left() + message_left, rect.bottom()),
                    );
                    ui.painter()
                        .with_clip_rect(time_clip.intersect(ui.clip_rect()))
                        .text(
                            time_pos,
                            egui::Align2::LEFT_CENTER,
                            time,
                            font.clone(),
                            if selected { p.on_accent } else { p.muted },
                        );
                    let message_rect = Rect::from_min_max(
                        rect.min
                            + egui::vec2(
                                message_left + CONSOLE_CELL_X_PADDING,
                                CONSOLE_MESSAGE_Y_PADDING,
                            ),
                        rect.max - egui::vec2(CONSOLE_CELL_X_PADDING, CONSOLE_MESSAGE_Y_PADDING),
                    );
                    let galley = ui.fonts_mut(|fonts| {
                        fonts.layout_job(console_message_layout(log, &font, message_width))
                    });
                    let color = if selected {
                        p.on_accent
                    } else if log.level() == "error" {
                        if ui.visuals().dark_mode {
                            crate::app_colors::NOTICE_DARK[3]
                        } else {
                            crate::app_colors::NOTICE_LIGHT[3]
                        }
                    } else {
                        p.text
                    };
                    ui.painter()
                        .with_clip_rect(message_rect.intersect(ui.clip_rect()))
                        .galley(message_rect.left_top(), galley, color);
                    if self
                        .probe
                        .record(
                            format!("log:{}", log.id),
                            response.on_hover_cursor(egui::CursorIcon::PointingHand),
                        )
                        .clicked()
                    {
                        self.selected_log = Some(log.id);
                    }
                }
                ui.set_min_height(*offsets.last().unwrap_or(&0.));
            });
        self.probe.console_scroll = output.state.offset.y;
    }
    fn console_header(&mut self, ui: &mut egui::Ui) -> (Rect, [f32; 3]) {
        let p = crate::desktop_style::Palette::of(ui.ctx());
        let (r, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 30.), egui::Sense::hover());
        ui.painter().rect_filled(r, 0, p.surface);
        ui.painter()
            .rect_stroke(r, 0, (1., p.line), egui::StrokeKind::Inside);
        let (widths, minimum) = console_column_layout(r.width(), self.console_column_widths);
        let edges = [widths[0], widths[0] + widths[1], widths[..3].iter().sum()];
        let offsets = [0., edges[0], edges[1], edges[2]];
        for (i, (key, title)) in [
            ("source", "Source"),
            ("level", "Level"),
            ("time", "Time"),
            ("message", "Message"),
        ]
        .into_iter()
        .enumerate()
        {
            let (offset, width) = (offsets[i], widths[i]);
            let cell = Rect::from_min_size(r.min + egui::vec2(offset, 0.), egui::vec2(width, 30.));
            self.probe
                .widgets
                .insert(format!("console-header:{key}"), cell);
            let mut title_clip = cell;
            if matches!(key, "source" | "level" | "message") {
                title_clip.max.x -= crate::desktop_style::ICON_BUTTON_SIDE + 6.;
            }
            ui.painter()
                .with_clip_rect(ui.clip_rect().intersect(title_clip))
                .text(
                    cell.left_center() + egui::vec2(6., 0.),
                    egui::Align2::LEFT_CENTER,
                    tr(ui.ctx(), title),
                    egui::FontId::proportional(crate::desktop_style::font_size(ui.ctx())),
                    p.muted,
                );
            ui.painter()
                .vline(cell.right(), cell.y_range(), (1., p.line));
            if key == "message" {
                self.console_message_filter(ui, cell);
            }
            let filter = match key {
                "source" => Some((
                    &mut self.log_source,
                    &[
                        "All log sources",
                        "dart.stdout",
                        "dart.stderr",
                        "dart.logging",
                    ][..],
                )),
                "level" => Some((
                    &mut self.log_level,
                    &[
                        "All levels",
                        "log",
                        "info",
                        "warn",
                        "error",
                        "debug",
                        "warning",
                    ][..],
                )),
                _ => None,
            };
            if let Some((selected, choices)) = filter {
                let response = crate::desktop_style::table_filter_button(
                    ui,
                    cell,
                    ui.id().with(("console-filter", key)),
                    tr(ui.ctx(), selected),
                );
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
                                    tr(ui.ctx(), value),
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
        }
        ui.painter().hline(r.x_range(), r.bottom(), (1., p.line));
        for (i, offset) in edges.iter().enumerate() {
            let x = r.left() + offset;
            let hit =
                Rect::from_center_size(egui::pos2(x, r.center().y), egui::vec2(6., r.height()));
            let grip = ui
                .interact(
                    hit,
                    ui.id().with(("console-column", i)),
                    egui::Sense::drag(),
                )
                .on_hover_and_drag_cursor(egui::CursorIcon::ResizeHorizontal);
            if grip.hovered() || grip.dragged() {
                ui.painter().vline(x, r.y_range(), (2., p.accent));
            }
            if grip.dragged() && !ui.input(|input| input.pointer.any_pressed()) {
                let delta = grip.drag_delta().x.clamp(
                    -(widths[i] - minimum[i]).max(0.),
                    (widths[i + 1] - minimum[i + 1]).max(0.),
                );
                if delta != 0. {
                    self.console_column_widths.copy_from_slice(&widths[..3]);
                    self.console_column_widths[i] += delta;
                    if i + 1 < self.console_column_widths.len() {
                        self.console_column_widths[i + 1] -= delta;
                    }
                    ui.ctx().request_repaint();
                }
            }
            self.probe.record(format!("console-column:{i}"), grip);
        }
        (r, edges)
    }
    fn console_message_filter(&mut self, ui: &mut egui::Ui, cell: Rect) {
        use crate::desktop_style::{ICON_BUTTON_SIDE, Palette, framed_text_edit, square_icon};

        let p = Palette::of(ui.ctx());
        let button = crate::desktop_style::table_filter_button(
            ui,
            cell,
            ui.id().with("console-message-filter"),
            tr(ui.ctx(), "Filter Message by tags"),
        );
        if !self.message_tags.is_empty() {
            ui.painter()
                .rect_stroke(button.rect, 0, (1., p.accent), egui::StrokeKind::Inside);
        }
        let popup = egui::Popup::menu(&button)
            .align(egui::RectAlign::BOTTOM_END)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .width(300.)
            .frame(egui::Frame::NONE.fill(p.surface).stroke((1., p.line)))
            .show(|ui| {
                ui.set_width(300.);
                ui.set_height(240.);
                ui.spacing_mut().item_spacing.y = 0.;
                let header = egui::Frame::NONE
                    .fill(p.background)
                    .inner_margin(6)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let gap = crate::desktop_style::ICON_TEXT_GAP;
                            ui.spacing_mut().item_spacing.x = gap;
                            let width = ui.available_width() - ICON_BUTTON_SIDE - gap;
                            let edit = egui::TextEdit::singleline(&mut self.message_tag_input)
                                .hint_text(tr(ui.ctx(), "Enter a message tag"));
                            let input = self.probe.record(
                                "message-tag-input",
                                framed_text_edit(
                                    ui,
                                    edit,
                                    Some(egui::vec2(width, ICON_BUTTON_SIDE)),
                                ),
                            );
                            let enter =
                                input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let add_label = tr(ui.ctx(), "Add tag");
                            let add = self
                                .probe
                                .record("message-tag-add", square_icon(ui, 0xe047, &add_label));
                            if enter || add.clicked() {
                                let text = self.message_tag_input.trim();
                                if !text.is_empty() {
                                    let needle = text.to_lowercase();
                                    if !self.message_tags.iter().any(|tag| tag.needle == needle) {
                                        self.message_tags.push(MessageTag {
                                            text: text.to_owned(),
                                            needle,
                                        });
                                    }
                                    self.message_tag_input.clear();
                                }
                                input.request_focus();
                            }
                        });
                    });
                let rect = header.response.rect;
                ui.painter()
                    .hline(rect.x_range(), rect.bottom(), (1., p.line));

                let remove = egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        egui::Frame::NONE
                            .inner_margin(6)
                            .show(ui, |ui| self.console_tag_cards(ui))
                            .inner
                    })
                    .inner;
                if let Some(i) = remove {
                    self.message_tags.remove(i);
                }
            });
        self.probe.record("console-message-filter", button);
        if let Some(popup) = popup {
            self.probe.record("message-filter-popup", popup.response);
        }
    }

    fn console_tag_cards(&mut self, ui: &mut egui::Ui) -> Option<usize> {
        use crate::desktop_style::{ICON_BUTTON_SIDE, ICON_TEXT_GAP, Palette, square_icon};

        let p = Palette::of(ui.ctx());
        if self.message_tags.is_empty() {
            ui.label(egui::RichText::new(crate::vintage::empty_text(ui.ctx())).color(p.muted));
            return None;
        }

        let font = egui::TextStyle::Body.resolve(ui.style());
        let text_height = ui.fonts_mut(|fonts| fonts.row_height(&font));
        let inset = 5.;
        let chrome_width = 2. * inset + ICON_TEXT_GAP + ICON_BUTTON_SIDE;
        let height = text_height.max(ICON_BUTTON_SIDE) + 2. * inset;
        let max_width = ui.available_width();
        let remove_label = tr(ui.ctx(), "Remove tag");
        let mut remove = None;

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::splat(8.);
            for (i, tag) in self.message_tags.iter().enumerate() {
                let text_width = ui
                    .painter()
                    .layout_no_wrap(tag.text.clone(), font.clone(), p.text)
                    .size()
                    .x
                    .ceil();
                // Give the wrapper the full card width before placing any contents.
                let width = (text_width + chrome_width).min(max_width);
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
                ui.painter()
                    .rect_stroke(rect, 0, (1., p.line), egui::StrokeKind::Inside);
                self.probe
                    .widgets
                    .insert(format!("message-tag-card:{i}"), rect);
                let mut card = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt(("message-tag-card", i))
                        .max_rect(rect.shrink(inset))
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                card.spacing_mut().item_spacing.x = ICON_TEXT_GAP;
                card.spacing_mut().interact_size = egui::Vec2::splat(ICON_BUTTON_SIDE);
                card.add_sized(
                    [
                        (width - chrome_width).max(0.),
                        text_height.max(ICON_BUTTON_SIDE),
                    ],
                    egui::Label::new(egui::RichText::new(&tag.text).font(font.clone())).truncate(),
                )
                .on_hover_text(&tag.text);
                if self
                    .probe
                    .record(
                        format!("message-tag-remove:{i}"),
                        square_icon(&mut card, 0xe16a, &remove_label),
                    )
                    .clicked()
                {
                    remove = Some(i);
                }
            }
        });
        remove
    }

    fn console_inspector(&mut self, ui: &mut egui::Ui) {
        inspector_frame().show(ui, |ui| self.console_inspector_content(ui));
    }

    fn console_inspector_content(&mut self, ui: &mut egui::Ui) {
        let Some(record) = self
            .grouped_logs
            .iter()
            .find(|r| Some(r.id) == self.selected_log)
        else {
            return;
        };
        let tab = inspector_tabs(
            ui,
            &mut self.probe,
            &["Detail", "Raw"],
            usize::from(self.console_raw),
            None,
            |i| ["console-detail", "console-raw"][i].into(),
        );
        self.console_raw = tab == 1;

        if !self.console_raw {
            if self
                .console_detail_cache
                .as_ref()
                .is_none_or(|(old, _)| old.id != record.id || old.event != record.event)
            {
                self.console_detail_cache =
                    Some((record.clone(), console_detail(ui.ctx(), record)));
            }
            let fields = &self
                .console_detail_cache
                .as_ref()
                .expect("console detail cache")
                .1;
            self.probe.inspector_text = fields
                .iter()
                .map(|(name, value)| format!("{name}: {value}"))
                .collect::<Vec<_>>()
                .join("\n")
                .chars()
                .take(8192)
                .collect();
            metadata_viewport(ui, &mut self.probe, "console-detail", fields, false);
            return;
        }
        if self
            .raw_cache
            .as_ref()
            .is_none_or(|(old, _)| old.id != record.id || old.event != record.event)
        {
            self.console_raw_serializations += 1;
            let content = pretty(&serde_json::to_value(record).expect("console record"));
            self.console_json.load(&content);
            self.raw_cache = Some((record.clone(), content));
        }
        let content = &self.raw_cache.as_ref().expect("console raw cache").1;
        self.probe.inspector_text = content.chars().take(8192).collect();
        ui.push_id("console-json-raw", |ui| {
            self.console_json.show(ui, &mut self.probe)
        });
    }

    fn inspector(&mut self, ui: &mut egui::Ui, record: &NetworkRecord) {
        inspector_frame().show(ui, |ui| self.inspector_content(ui, record));
    }

    fn inspector_content(&mut self, ui: &mut egui::Ui, record: &NetworkRecord) {
        inspector_heading(ui, &mut self.probe, |ui, probe| {
            if probe
                .record(
                    "curl",
                    crate::desktop_style::copy_button(ui, &tr(ui.ctx(), "Copy cURL")),
                )
                .clicked()
            {
                crate::desktop_support::copy(ui.ctx(), record.curl());
            }
            let url_rect = Rect::from_min_size(
                ui.available_rect_before_wrap().min,
                egui::vec2(ui.available_width(), 32.),
            );
            ui.allocate_ui_with_layout(
                url_rect.size(),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    let scroll = egui::ScrollArea::horizontal()
                        .id_salt((
                            "inspector-url-scroll",
                            &record.session_id,
                            &record.request_id,
                        ))
                        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                        .auto_shrink([false, true])
                        .max_height(32.)
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(crate::desktop_style::mono(
                                    ui.ctx(),
                                    text(&record.request_event["request"]["url"]),
                                ))
                                .wrap_mode(egui::TextWrapMode::Extend)
                                .halign(egui::Align::Min),
                            );
                        });
                    probe
                        .widgets
                        .insert("inspector-url".into(), scroll.inner_rect);
                },
            );
        });
        let tab = inspector_tabs(
            ui,
            &mut self.probe,
            &TABS,
            self.settings.values().request_detail_tab as usize,
            self.reveal_inspector_tab,
            |i| format!("tab:{i}"),
        );
        self.reveal_inspector_tab = None;
        if let Err(e) = self.settings.set_request_detail_tab(tab as i64) {
            self.message = Some(e.to_string());
        }
        let req = &record.request_event["request"];
        let res = &record.completion_event["response"];
        let key = (record.session_id.clone(), record.request_id.clone());
        if self.body_json_key.as_ref() != Some(&key) {
            self.release_detail();
            self.body_json_key = Some(key.clone());
        }
        let cache_key = (
            key.0,
            key.1,
            record.request_revision,
            record.completion_revision,
            tab,
        );
        if self
            .detail_cache
            .as_ref()
            .is_none_or(|(old, _)| old != &cache_key)
        {
            self.detail_preparations += 1;
            if tab == 5 {
                self.network_raw_serializations += 1;
            }
            let content = match tab {
                0 => overview(ui.ctx(), record),
                1 => pretty(&req["headers"]),
                2 => captured_body(req),
                3 => pretty(&res["headers"]),
                4 => captured_body(res),
                _ => pretty(&serde_json::to_value(record).expect("capture record")),
            };
            self.detail_cache = Some((cache_key, content.into()));
        }
        let content = std::sync::Arc::clone(&self.detail_cache.as_ref().expect("detail cache").1);
        // Keep automation bounded: full text is copied only by an explicit action.
        self.probe.inspector_text = content.chars().take(8192).collect();
        self.probe.retained_detail_bytes = content.len();
        let data = match tab {
            1 => &req["headers"],
            3 => &res["headers"],
            2 => req,
            _ => res,
        };
        let empty = if tab == 1 || tab == 3 {
            data.is_null() || data.as_object().is_some_and(|v| v.is_empty())
        } else if tab == 2 || tab == 4 {
            data["bodyOmitted"].is_null()
                && ((data["body"].is_null() && data["bodyPreview"].is_null()) || content.is_empty())
        } else {
            false
        };
        if empty {
            ui.add_space(12.);
            let rect = ui.available_rect_before_wrap();
            let response = ui.allocate_rect(rect, egui::Sense::hover());
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                crate::vintage::empty_text(ui.ctx()),
                egui::FontId::proportional(crate::desktop_style::font_size(ui.ctx())),
                crate::desktop_style::Palette::of(ui.ctx()).muted,
            );
            self.probe.record("detail-empty", response);
            return;
        }
        if tab == 5 {
            self.raw_json.load(&content);
            ui.push_id("raw-request", |ui| self.raw_json.show(ui, &mut self.probe));
            return;
        }
        if tab == 0 || tab == 1 || tab == 3 {
            let fields = if tab == 0 {
                overview_fields(ui.ctx(), record)
            } else {
                data.as_object()
                    .map(|headers| {
                        headers
                            .iter()
                            .map(|(name, value)| {
                                (
                                    name.clone(),
                                    value
                                        .as_str()
                                        .map(str::to_owned)
                                        .unwrap_or_else(|| pretty(value)),
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default()
            };
            metadata_viewport(
                ui,
                &mut self.probe,
                tab,
                &fields,
                tab == 0 && raw_url_query(text(&req["url"])).is_some(),
            );
            return;
        }
        if tab == 2 || tab == 4 {
            let index = usize::from(tab == 4);
            let revision = if index == 0 {
                record.request_revision
            } else {
                record.completion_revision
            };
            let data = if index == 0 { req } else { res };
            if self.bodies[index].load(data, revision) {
                self.body_preparations += 1;
            }
            if self.bodies[index].show(ui, &mut self.probe, &content) {
                self.body_json[index] = Default::default();
                if self
                    .probe
                    .record(
                        "copy-detail",
                        crate::desktop_style::copy_button(ui, &tr(ui.ctx(), "Copy detail")),
                    )
                    .clicked()
                {
                    crate::desktop_support::copy(ui.ctx(), content.to_string());
                }
                return;
            }
            let viewer = &mut self.body_json[index];
            if viewer.load(&content) {
                ui.push_id(("body-json", tab), |ui| viewer.show(ui, &mut self.probe));
                return;
            }
        }
        ui.add_space(12.);
        if (tab == 2 || tab == 4)
            && self
                .probe
                .record(
                    "copy-detail",
                    crate::desktop_style::copy_button(ui, &tr(ui.ctx(), "Copy detail")),
                )
                .clicked()
        {
            crate::desktop_support::copy(ui.ctx(), content.to_string());
        }
        if (tab == 2 || tab == 4)
            && let Some(argb) = crate::body_color::parse_body_color(&content)
        {
            crate::desktop_body::color_swatch(ui, &mut self.probe, "body-color", &content, argb);
        }
        egui::ScrollArea::both()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .id_salt(("detail-content", tab))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(crate::desktop_style::mono(ui.ctx(), content.as_ref()))
                        .selectable(true)
                        .wrap_mode(egui::TextWrapMode::Extend),
                );
            });
    }
}
fn inspector_frame() -> egui::Frame {
    egui::Frame::NONE.inner_margin(egui::Margin::symmetric(18, 8))
}

fn inspector_heading(
    ui: &mut egui::Ui,
    probe: &mut UiProbe,
    content: impl FnOnce(&mut egui::Ui, &mut UiProbe),
) {
    let heading = ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 33.),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| content(ui, probe),
    );
    probe.record("inspector-heading", heading.response);
}

fn inspector_tabs(
    ui: &mut egui::Ui,
    probe: &mut UiProbe,
    labels: &[&str],
    mut tab: usize,
    reveal: Option<usize>,
    key: impl Fn(usize) -> String,
) -> usize {
    ui.scope(|ui| {
        let content_style = ui.style().clone();
        crate::desktop_style::ventura_scrollbar(ui);
        // The floating thumb grows on hover; reserve its full height below the tabs.
        let scrollbar_gutter = ui.spacing().scroll.bar_width as i8 + 2;
        let scroll = egui::ScrollArea::horizontal()
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
            .id_salt("inspector-tabs")
            .show(ui, |ui| {
                ui.set_style(content_style.clone());
                let p = crate::desktop_style::Palette::of(ui.ctx());
                egui::Frame::NONE
                    .fill(p.surface)
                    .corner_radius(0)
                    .inner_margin(egui::Margin {
                        bottom: scrollbar_gutter,
                        ..egui::Margin::same(3)
                    })
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.;
                            for (i, label) in labels.iter().copied().enumerate() {
                                let label = tr(ui.ctx(), label);
                                let width = ui
                                    .painter()
                                    .layout_no_wrap(
                                        label.clone(),
                                        egui::FontId::proportional(
                                            crate::desktop_style::font_size(ui.ctx()),
                                        ),
                                        p.text,
                                    )
                                    .size()
                                    .x
                                    + 36.;
                                let response = probe.record(
                                    key(i),
                                    crate::desktop_style::pill_with_size(
                                        ui,
                                        &label,
                                        None,
                                        tab == i,
                                        width,
                                        12.,
                                    ),
                                );
                                if reveal == Some(i) {
                                    response.scroll_to_me(Some(egui::Align::Center));
                                }
                                if response.clicked() {
                                    tab = i;
                                }
                            }
                        });
                    });
            });
        probe.widgets.insert(
            "inspector-tabs".into(),
            Rect::from_min_size(
                scroll.inner_rect.min,
                egui::vec2(
                    scroll.inner_rect.width(),
                    scroll.content_size.y.min(scroll.inner_rect.height()),
                ),
            ),
        );
    });
    tab
}

fn text(value: &serde_json::Value) -> &str {
    value.as_str().unwrap_or("—")
}
fn pretty(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).expect("JSON value")
}
fn body(value: &serde_json::Value) -> String {
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| pretty(value))
}
fn record_status(record: &NetworkRecord) -> String {
    if record.pending() {
        "Pending".into()
    } else if let Some(status) = record.status() {
        status.to_string()
    } else if record.failed() {
        "Error".into()
    } else {
        "Complete".into()
    }
}

fn overview(ctx: &egui::Context, record: &NetworkRecord) -> String {
    overview_fields(ctx, record)
        .into_iter()
        .map(|(key, value)| format!("{key}: {value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

// Slice the captured URL without normalizing encoding, ordering, or duplicates.
fn raw_url_query(url: &str) -> Option<&str> {
    let url = url.split_once('#').map_or(url, |(url, _)| url);
    let (_, query) = url.split_once('?')?;
    (!query.is_empty()).then_some(query)
}

// Keep the captured scheme, explicit port and path; omit credentials.
fn request_domain_and_endpoint(url: &str) -> (String, &str) {
    let base = url.split(['?', '#']).next().unwrap_or_default();
    let absolute = url::Url::parse(base).is_ok_and(|url| url.has_host());
    let authority_and_path = if absolute {
        base.split_once("://")
            .map(|(scheme, rest)| (Some(scheme), rest))
    } else {
        base.strip_prefix("//").map(|rest| (None, rest))
    };
    let Some((scheme, rest)) = authority_and_path else {
        return ("—".into(), if base.is_empty() { "—" } else { base });
    };
    let path_start = rest.find('/').unwrap_or(rest.len());
    let (authority, path) = rest.split_at(path_start);
    let host = authority.rsplit('@').next().unwrap_or_default();
    let domain = match (scheme, host) {
        (_, "") => "—".into(),
        (Some(scheme), host) => format!("{scheme}://{host}"),
        (None, host) => host.into(),
    };
    (domain, if path.is_empty() { "/" } else { path })
}

fn overview_fields(ctx: &egui::Context, record: &NetworkRecord) -> Vec<(String, String)> {
    use base64::Engine;
    let req = &record.request_event["request"];
    let res = &record.completion_event["response"];
    let bytes = if let Some(size) = res["bodyByteLength"].as_u64() {
        size as usize
    } else if res["body"].is_null() {
        res["bodyPreview"].as_str().unwrap_or("").len()
    } else if res["bodyEncoding"] == "base64" {
        base64::engine::general_purpose::STANDARD
            .decode(res["body"].as_str().unwrap_or(""))
            .map(|v| v.len())
            .unwrap_or(0)
    } else {
        body(&res["body"]).len()
    };
    let url = text(&req["url"]);
    let (domain, endpoint) = request_domain_and_endpoint(url);
    let mut fields = vec![
        (tr(ctx, "Method"), text(&req["method"]).into()),
        (tr(ctx, "Domain"), domain),
        (tr(ctx, "Endpoint"), endpoint.into()),
        (
            tr(ctx, "Query params"),
            raw_url_query(url)
                .map(str::to_owned)
                .unwrap_or_else(|| crate::vintage::empty_text(ctx)),
        ),
        (tr(ctx, "Request ID"), record.request_id.clone()),
        (
            tr(ctx, "Source"),
            text(&record.request_event["source"]).into(),
        ),
        (
            tr(ctx, "Started"),
            text(&record.request_event["timestamp"]).into(),
        ),
        (
            tr(ctx, "Status"),
            format!(
                "{} {}",
                tr(ctx, &record_status(record)),
                res["statusText"].as_str().unwrap_or("")
            ),
        ),
        (
            tr(ctx, "Response size"),
            format!("{} {}", bytes, tr(ctx, "captured bytes")),
        ),
    ];
    if !record.completion_event["error"].is_null() {
        fields.push((tr(ctx, "Error"), pretty(&record.completion_event["error"])));
    }
    fields
}

fn console_detail(ctx: &egui::Context, record: &ConsoleRecord) -> Vec<(String, String)> {
    let mut fields: Vec<_> = [
        ("Level", record.level().to_uppercase()),
        ("Source", record.source_label().to_string()),
        ("Logger", record.logger_name().to_string()),
        ("Isolate", text(&record.event["isolateName"]).to_string()),
        ("Timestamp", text(&record.event["timestamp"]).to_string()),
        ("Message", record.display_message()),
    ]
    .into_iter()
    .map(|(name, value)| (tr(ctx, name), value))
    .collect();
    if let Some(value) = record.severity() {
        fields.push((tr(ctx, "Severity"), value.to_string()));
    }
    if let Some(value) = record.error() {
        fields.push((tr(ctx, "Error"), value.to_string()));
    }
    if let Some(value) = record.stack_trace() {
        fields.push((tr(ctx, "Stack trace"), value.to_string()));
    }
    if let Some(value) = record.event.get("args") {
        fields.push((tr(ctx, "Arguments"), pretty(value)));
    }
    fields
}

fn captured_body(data: &serde_json::Value) -> String {
    body(if data["body"].is_null() {
        &data["bodyPreview"]
    } else {
        &data["body"]
    })
}

// egui SidePanel exposes its whole area response, rather than the resize grip.
// Persist only a release at that grip; toolbar/dialog clicks are unrelated.
fn split_released(ctx: &egui::Context, rect: Rect) -> bool {
    ctx.input(|i| {
        i.pointer.any_released()
            && i.pointer.interact_pos().is_some_and(|p| {
                (p.x - rect.left()).abs() <= 8. && p.y >= rect.top() && p.y <= rect.bottom()
            })
    })
}

fn metadata_rows(
    ui: &mut egui::Ui,
    probe: &mut UiProbe,
    fields: &[(String, String)],
    highlight_query: bool,
) {
    let entries: Vec<_> = fields
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    let query_title = tr(ui.ctx(), "Query params");
    let query_row = if highlight_query {
        fields.iter().position(|(title, _)| title == &query_title)
    } else {
        None
    };
    crate::vintage::key_value_table_with_query(ui, probe, "metadata", &entries, true, query_row);
}

fn metadata_viewport(
    ui: &mut egui::Ui,
    probe: &mut UiProbe,
    id: impl std::hash::Hash,
    fields: &[(String, String)],
    highlight_query: bool,
) {
    ui.add_space(12.);
    let viewport = egui::ScrollArea::vertical()
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
        .id_salt(("metadata", id))
        .auto_shrink([false, false])
        .show(ui, |ui| metadata_rows(ui, probe, fields, highlight_query));
    probe
        .widgets
        .insert("metadata-viewport".into(), viewport.inner_rect);
}
