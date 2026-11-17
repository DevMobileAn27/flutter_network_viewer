//! Executable native acceptance: run with a URI printed by f14_fake_vm.
//! Opens a real eframe window, injects egui pointer/key input, reads OS clipboard,
//! writes and verifies an export, captures a screenshot and exits successfully.
use eframe::egui::{self, Event, Modifiers, PointerButton};
use flutter_network_viewer::{desktop_host::DesktopApp, settings::AppSettings};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};
struct Smoke {
    app: DesktopApp,
    events: VecDeque<Vec<Event>>,
    step: usize,
    frames: usize,
    start: Instant,
    uri: String,
    output: std::path::PathBuf,
    success: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl Smoke {
    fn click(&mut self, key: &str) {
        let pos = self.app.ui.probe.widgets[key].center();
        self.events.push_back(vec![
            Event::PointerMoved(pos),
            Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed: true,
                modifiers: Modifiers::NONE,
            },
        ]);
        self.events.push_back(vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
        self.events.push_back(vec![]);
        self.events.push_back(vec![]);
    }
}
impl eframe::App for Smoke {
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        if let Some(events) = self.events.pop_front() {
            input.events = events;
        }
    }
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        assert!(
            self.start.elapsed() < Duration::from_secs(30),
            "native smoke timeout at step {}",
            self.step
        );
        self.app.update(ctx, frame);
        for event in ctx.input(|i| i.events.clone()) {
            if let Event::Screenshot { image, .. } = event {
                image::save_buffer(
                    self.output.join("native-console.png"),
                    image.as_raw(),
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                )
                .unwrap();
            }
        }
        self.frames += 1;
        if self.frames < 4 || !self.events.is_empty() {
            ctx.request_repaint();
            return;
        }
        match self.step {
            0 => self.click("connect"),
            1 => {
                self.click("vm-uri");
                self.events.push_back(vec![Event::Text(self.uri.clone())]);
                self.events.push_back(vec![]);
            }
            2 => self.click("connect-submit"),
            3 => {
                if self.app.capture.store().logs().len() != 8 {
                    return;
                }
                // All logs arrived at streamListen, before any getVM/getIsolate/profile.
                assert!(
                    self.app
                        .capture
                        .store()
                        .logs()
                        .iter()
                        .any(|r| r.message() == "Native early console")
                );
                self.click("Console");
            }
            4 => {
                assert_eq!(self.app.ui.probe.logs.len(), 4);
                self.click("source");
            }
            5 => self.click("source:dart.stderr"),
            6 => self.click("level"),
            7 => self.click("level:error"),
            8 => {
                self.click("search");
                self.events.push_back(vec![Event::Text("broken".into())]);
                self.events.push_back(vec![]);
            }
            9 => {
                assert_eq!(self.app.ui.probe.logs.len(), 1);
                let id = self.app.ui.probe.logs[0];
                self.click(&format!("log:{id}"));
            }
            10 => {
                assert!(self.app.ui.probe.inspector_text.contains("broken layout"));
                assert_eq!(self.app.ui.console_raw_serializations, 0);
                self.click("console-copy");
            }
            11 => {
                assert!(
                    arboard::Clipboard::new()
                        .unwrap()
                        .get_text()
                        .unwrap()
                        .contains("broken layout")
                );
                self.click("console-raw");
            }
            12 => {
                let raw: Value = serde_json::from_str(&self.app.ui.probe.inspector_text).unwrap();
                assert_eq!(raw["event"]["rawEvents"].as_array().unwrap().len(), 3);
                self.click("console-copy");
            }
            13 => {
                let raw: Value =
                    serde_json::from_str(&arboard::Clipboard::new().unwrap().get_text().unwrap())
                        .unwrap();
                assert_eq!(raw["event"]["source"], "dart.stderr");
                assert_eq!(self.app.ui.console_raw_serializations, 1);
                self.click("console-detail");
            }
            14 => self.click("reset"),
            15 => {
                assert_eq!(self.app.ui.probe.logs.len(), 4);
                let id = self.app.ui.probe.logs[1];
                self.click(&format!("log:{id}"));
            }
            16 => {
                assert!(self.app.ui.probe.inspector_text.contains("\"answer\":42"));
                assert_eq!(self.app.ui.console_raw_serializations, 1);
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            17 => {
                let id = self.app.ui.probe.logs[3];
                self.click(&format!("log:{id}"));
            }
            18 => {
                for value in ["orders", "native-error", "native-stack", "1000"] {
                    assert!(self.app.ui.probe.inspector_text.contains(value));
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(860., 760.)));
            }
            19 => {
                assert!(ctx.content_rect().width() < 1100.);
                assert!(self.app.ui.probe.inspector_text.contains("native-error"));
                self.events.push_back(vec![Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }]);
                self.events.push_back(vec![]);
            }
            20 => {
                assert!(self.app.ui.selected_log.is_none());
                self.click("clear");
            }
            21 => {
                assert!(self.app.capture.store().logs().is_empty());
                assert!(self.app.ui.selected_log.is_none());
                assert!(self.output.join("native-console.png").exists());
                self.app.capture.stop();
                self.success
                    .store(true, std::sync::atomic::Ordering::Release);
                println!(
                    "PASS native F14: early Stdout/Stderr/Logging before bootstrap, grouped JSON/exception, source+level+search, detail/raw, OS clipboard readback, lazy raw cache, narrow modal Escape, Clear, screenshot"
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            _ => (),
        }
        self.step += 1;
        self.frames = 0;
        ctx.request_repaint();
    }
}
struct ClipboardRestore(Option<String>);
impl Drop for ClipboardRestore {
    fn drop(&mut self) {
        if let Some(text) = self.0.take()
            && let Ok(mut clipboard) = arboard::Clipboard::new()
        {
            let _ = clipboard.set_text(text);
        }
    }
}
fn main() -> eframe::Result {
    let _clipboard_restore = ClipboardRestore(
        arboard::Clipboard::new()
            .ok()
            .and_then(|mut c| c.get_text().ok()),
    );
    let mut args = std::env::args().skip(1);
    let uri = args.next().expect("fake VM URI");
    let output = std::path::PathBuf::from(args.next().expect("artifact directory"));
    std::fs::create_dir_all(&output).unwrap();
    // A failed clipboard/export/screenshot path must never pass on stale output.
    {
        let name = "native-console.png";
        match std::fs::remove_file(output.join(name)) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => panic!("cannot clear old smoke artifact {name}: {e}"),
        }
    }
    let success = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let check = success.clone();
    eframe::run_native(
        "F14 Native Acceptance",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1360., 860.]),
            ..Default::default()
        },
        Box::new(move |cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::light());
            Ok(Box::new(Smoke {
                app: DesktopApp::new(AppSettings::from_preferences(
                    json!({"autoDetectVmService":false}),
                )),
                events: VecDeque::new(),
                step: 0,
                frames: 0,
                start: Instant::now(),
                uri,
                output,
                success,
            }))
        }),
    )?;
    assert!(
        check.load(std::sync::atomic::Ordering::Acquire),
        "native acceptance window closed before completion"
    );
    Ok(())
}
