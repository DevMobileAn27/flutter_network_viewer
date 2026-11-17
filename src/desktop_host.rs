//! Native host: nonblocking discovery tick and renderer action dispatch.
use crate::diagnostics::{Diagnostics, Event};
use crate::resources::{ResourceUsage, sample_process_memory};
use crate::{
    coordinator::{CoordinatorConfig, ThreadConnector},
    desktop::{Action, DesktopUi, ShellStatus},
    discovery::{AppDiscovery, ThreadDiscovery},
    settings::AppSettings,
};
use eframe::egui;
use std::time::{Duration, Instant};
pub struct DesktopApp {
    pub ui: DesktopUi,
    pub capture: AppDiscovery<ThreadConnector, ThreadDiscovery>,
    pub mcp: crate::mcp::McpServer,
    pub diagnostics: Diagnostics,
    pub updater: crate::updater::Updater<crate::updater::NativeBackend>,
    pub resources: ResourceUsage<fn() -> std::io::Result<u64>>,
    last_state: Option<(usize, bool, bool, bool)>,
    started: Instant,
    updates_started: bool,
}
impl DesktopApp {
    pub fn new(settings: AppSettings) -> Self {
        Self::with_diagnostics(settings, Diagnostics::desktop_directory())
    }
    pub fn with_diagnostics(settings: AppSettings, directory: std::path::PathBuf) -> Self {
        let mut diagnostics = Diagnostics::new(directory, crate::diagnostics::FILE_BYTES);
        diagnostics.record(Event::AppStarted, 0);
        let capture = AppDiscovery::new(
            ThreadConnector::default(),
            ThreadDiscovery::default(),
            CoordinatorConfig::production(),
        );
        let mut resources =
            ResourceUsage::new(sample_process_memory as fn() -> std::io::Result<u64>);
        resources.tick(0);
        let ui = DesktopUi::new(settings);
        Self {
            updater: crate::updater::Updater::desktop(),
            diagnostics,
            resources,
            last_state: None,
            ui,
            capture,
            mcp: Default::default(),
            started: Instant::now(),
            updates_started: false,
        }
    }
    pub fn tick(&mut self, now: u64) {
        self.updater.tick(now);
        self.ui.support.updates = self.updater.view();
        self.capture.tick(now);
        self.resources.tick(now);
        self.ui.memory_bytes = self.resources.memory_bytes();
        let state = (
            self.capture
                .store()
                .sessions()
                .keys()
                .filter(|id| self.capture.is_online(id))
                .count(),
            self.capture.busy(),
            self.capture.searching(),
            self.capture.last_error().is_some(),
        );
        if self.last_state != Some(state) {
            self.diagnostics.record(
                Event::ConnectionState {
                    online: state.0,
                    busy: state.1,
                    searching: state.2,
                    failed: state.3,
                },
                now,
            );
            self.last_state = Some(state);
        }
        self.diagnostics.poll(now);
    }
    pub fn dispatch(&mut self, action: Action, now: u64) {
        let event = match &action {
            Action::StartScan => Some(Event::CaptureStarted { automatic: false }),
            Action::StopScan => Some(Event::DiscoveryStopped),
            Action::Connect(_) | Action::SelectTarget(_) => Some(Event::ConnectionAttempt),
            Action::SetPaused(paused) => Some(Event::PauseChanged { paused: *paused }),
            Action::Clear => Some(Event::CaptureCleared),
            Action::RetryMcp | Action::CheckUpdates => None,
        };
        if let Some(event) = event {
            self.diagnostics.record(event, now);
        }
        match action {
            Action::CheckUpdates => self.updater.check(false, now),
            Action::RetryMcp => self.mcp.configure(
                self.ui.settings.values().mcp_enabled,
                self.ui.settings.values().mcp_port as u16,
                true,
            ),
            Action::StartScan => self.capture.start_scan(now),
            Action::StopScan => self.capture.stop_discovery(),
            Action::SelectTarget(key) => {
                self.capture.select(&key, now);
            }
            Action::Connect(uri) => match self.capture.connect_manual(&uri, now) {
                Ok(()) => {
                    if let Some(endpoint) = self.capture.endpoint()
                        && let Err(e) = self.ui.settings.set_manual_vm_service_uri(endpoint)
                    {
                        eprintln!("{e}");
                    }
                }
                Err(_) => self.diagnostics.record(Event::ConnectionFailed, now),
            },
            Action::SetPaused(paused) => self.capture.set_paused(paused),
            Action::Clear => self.capture.clear(now),
        }
    }
}
impl Drop for DesktopApp {
    fn drop(&mut self) {
        self.resources.stop();
        self.diagnostics.record(
            Event::AppStopped,
            self.started.elapsed().as_millis().min(u64::MAX as u128) as u64,
        );
        self.diagnostics.shutdown();
    }
}
impl eframe::App for DesktopApp {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let now = self.started.elapsed().as_millis().min(u64::MAX as u128) as u64;
        if !self.updates_started {
            self.updates_started = true;
            self.updater.check(true, now);
        }
        self.tick(now);
        if cfg!(windows) && self.updater.status == crate::updater::Status::Installing {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        let status = ShellStatus {
            searching: self.capture.searching(),
            busy: self.capture.busy(),
            targets: self.capture.targets().to_vec(),
            online: self
                .capture
                .store()
                .sessions()
                .keys()
                .filter(|id| self.capture.is_online(id))
                .cloned()
                .collect(),
            error: self.capture.last_error().map(ToString::to_string),
        };
        self.mcp.configure(
            self.ui.settings.values().mcp_enabled,
            self.ui.settings.values().mcp_port as u16,
            false,
        );
        self.mcp.poll(self.capture.store(), &status.online);
        self.ui.support.mcp_status = self.mcp.status();
        for action in self.ui.show(ctx, self.capture.store(), &status) {
            self.dispatch(action, now);
        }
        ctx.request_repaint_after(Duration::from_millis(50));
    }
}
