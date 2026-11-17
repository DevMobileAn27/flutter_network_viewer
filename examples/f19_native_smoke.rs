//! F19 production host + fresh VM + native renderer/input acceptance.
use eframe::egui::{self, Event, Modifiers, PointerButton};
use flutter_network_viewer::{
    desktop_host::DesktopApp,
    settings::{AppSettings, ThemeMode},
};
use std::{
    collections::VecDeque,
    path::PathBuf,
    time::{Duration, Instant},
};
struct Smoke {
    app: Option<DesktopApp>,
    output: PathBuf,
    events: VecDeque<Vec<Event>>,
    start: Instant,
    stage: Instant,
    step: u8,
    success: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl Smoke {
    fn click(&mut self, key: &str) {
        let p = self.app.as_ref().unwrap().ui.probe.widgets[key].center();
        self.events.push_back(vec![Event::PointerMoved(p)]);
        for pressed in [true, false] {
            self.events.push_back(vec![Event::PointerButton {
                pos: p,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            }]);
        }
        self.events.push_back(vec![]);
    }
    fn screenshot(&self, ctx: &egui::Context, name: &str) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
            name.to_owned(),
        )));
    }
}
impl eframe::App for Smoke {
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        if let Some(events) = self.events.pop_front() {
            let shots: Vec<_> = input
                .events
                .iter()
                .filter(|e| matches!(e, Event::Screenshot { .. }))
                .cloned()
                .collect();
            input.events = events;
            input.events.extend(shots);
        }
    }
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        assert!(
            self.start.elapsed() < Duration::from_secs(35),
            "native timeout step {}",
            self.step
        );
        let Some(app) = self.app.as_mut() else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        };
        app.update(ctx, frame);
        for event in ctx.input(|i| i.events.clone()) {
            if let Event::Screenshot {
                image, user_data, ..
            } = event
            {
                let name = user_data
                    .data
                    .as_ref()
                    .unwrap()
                    .downcast_ref::<String>()
                    .unwrap();
                image::save_buffer(
                    self.output.join(name),
                    image.as_raw(),
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
        ctx.request_repaint_after(Duration::from_millis(20));
        if !self.events.is_empty() {
            return;
        }
        let app = self.app.as_ref().unwrap();
        match self.step {
            0 => {
                if app.capture.store().network().len() < 3 || app.resources.samples < 2 {
                    return;
                }
                assert!(app.resources.memory_bytes().unwrap() > 0);
                assert_eq!(app.ui.probe.resource_text, app.resources.label());
                assert!(!self.output.join("blocker/nested/viewer.log").exists());
                // Independent OS RSS validates that a real native process is running; macOS
                // UI uses physical footprint rather than RSS, so equality is not expected.
                let rss = std::process::Command::new("/bin/ps")
                    .args(["-o", "rss=", "-p", &std::process::id().to_string()])
                    .output()
                    .unwrap();
                assert!(rss.status.success());
                let rss_kib = String::from_utf8(rss.stdout)
                    .unwrap()
                    .trim()
                    .parse::<u64>()
                    .unwrap();
                assert!(rss_kib > 0);
                std::fs::write(self.output.join("memory.json"),serde_json::to_vec_pretty(&serde_json::json!({"memory_bytes":app.resources.memory_bytes(),"rss_kib":rss_kib,"samples":app.resources.samples,"requests":app.capture.store().network().len()})).unwrap()).unwrap();
                self.screenshot(ctx, "ram-en-light.png");
                self.click("pause");
            }
            1 => {
                assert!(app.capture.store().paused);
                std::fs::remove_file(self.output.join("blocker")).unwrap();
                self.click("pause");
                self.stage = Instant::now();
            }
            2 => {
                if self.stage.elapsed() < Duration::from_millis(400) {
                    return;
                }
                assert!(!app.capture.store().paused);
                assert!(app.capture.store().network().len() >= 3);
                assert!(self.output.join("blocker/nested/viewer.log").exists());
                let app = self.app.as_mut().unwrap();
                app.ui.settings.set_language("vi").unwrap();
                app.ui.settings.set_theme_mode(ThemeMode::Dark).unwrap();
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(480., 360.)));
                self.stage = Instant::now();
            }
            3 => {
                if self.stage.elapsed() < Duration::from_millis(500) {
                    return;
                }
                let r = app.ui.probe.widgets["resource-usage"];
                assert!(ctx.content_rect().contains_rect(r));
                assert_eq!(app.ui.probe.resource_text, app.resources.label());
                self.screenshot(ctx, "ram-vi-dark-narrow.png");
                self.click("settings");
            }
            4 => {
                assert!(app.ui.support.settings_open);
                self.stage = Instant::now();
            }
            5 => {
                if self.stage.elapsed() < Duration::from_millis(400)
                    || !self.output.join("ram-vi-dark-narrow.png").exists()
                {
                    return;
                }
                drop(self.app.take());
                let log =
                    std::fs::read_to_string(self.output.join("blocker/nested/viewer.log")).unwrap();
                assert!(log.contains("app_stopped"));
                assert!(log.contains("pause_changed"));
                for line in log.lines() {
                    let v: serde_json::Value = serde_json::from_str(line).unwrap();
                    for k in v.as_object().unwrap().keys() {
                        assert!(
                            [
                                "event",
                                "time",
                                "pid",
                                "online",
                                "busy",
                                "searching",
                                "failed",
                                "automatic",
                                "paused"
                            ]
                            .contains(&k.as_str()),
                            "unexpected diagnostics field: {k}"
                        );
                    }
                }
                for forbidden in [
                    "ws://",
                    "Authorization",
                    "quantity",
                    "customer",
                    "base64",
                    "body",
                    "header",
                    "console",
                ] {
                    assert!(!log.contains(forbidden));
                }
                std::fs::write(self.output.join("native-result.json"),r#"{"result":"pass","production_host":true,"fresh_vm":true,"path_failure_capture_continues":true,"path_recovers":true,"periodic_native_memory":true,"pause_input":true,"narrow_vi_dark":true,"shutdown_flush":true}"#).unwrap();
                self.success
                    .store(true, std::sync::atomic::Ordering::Release);
                println!(
                    "PASS F19 native: live capture during log path failure, recovery, real periodic RAM, native input, narrow VI/dark, disposal flush"
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            _ => return,
        }
        self.step += 1;
    }
}
fn main() -> eframe::Result {
    let output = PathBuf::from(std::env::args().nth(1).expect("output directory"));
    std::fs::create_dir_all(&output).unwrap();
    let uri = std::env::args().nth(2).expect("VM URI");
    let blocker = output.join("blocker");
    assert!(!blocker.exists(), "use a fresh output directory");
    std::fs::write(&blocker, "not a directory").unwrap();
    let mut settings = AppSettings::load(output.join("settings.json")).unwrap();
    settings.set_auto_detect_vm_service(false).unwrap();
    settings.set_manual_vm_service_uri(&uri).unwrap();
    let success = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let check = success.clone();
    eframe::run_native(
        "F19 Native Acceptance",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1200., 800.])
                .with_min_inner_size([360., 300.]),
            ..Default::default()
        },
        Box::new(move |_| {
            Ok(Box::new(Smoke {
                app: Some(DesktopApp::with_diagnostics(
                    settings,
                    blocker.join("nested"),
                )),
                output,
                events: VecDeque::new(),
                start: Instant::now(),
                stage: Instant::now(),
                step: 0,
                success,
            }))
        }),
    )?;
    assert!(
        check.load(std::sync::atomic::Ordering::Acquire),
        "native closed before completion"
    );
    Ok(())
}
