//! Executable native acceptance: run with a URI printed by f13_fake_vm.
//! Opens a real eframe window, injects egui pointer/key input, reads OS clipboard,
//! writes and verifies an export, captures a screenshot and exits successfully.
use eframe::egui::{self, Event, Modifiers, PointerButton};
use flutter_network_viewer::{desktop_host::DesktopApp, settings::AppSettings};
use serde_json::json;
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
    fn click_record(&mut self, id: &str) {
        let record = self
            .app
            .capture
            .store()
            .network()
            .values()
            .find(|r| r.request_event["vmRequestId"] == id)
            .unwrap();
        let key = format!("request:{}:{}", record.session_id, record.request_id);
        self.click(&key);
    }
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
                    self.output.join("native-network.png"),
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
                if self.app.ui.probe.requests.len() != 2 {
                    return;
                }
                assert!(
                    self.app
                        .capture
                        .store()
                        .network()
                        .values()
                        .any(|r| r.pending())
                );
                self.click("Network");
            }
            4 => self.click_record("feed"),
            5 => {
                assert!(self.app.ui.probe.inspector_text.contains("Pending"));
                let mut client = flutter_network_viewer::vm::VmClient::connect(
                    &self.uri,
                    Duration::from_secs(2),
                )
                .unwrap();
                client.call("f13.complete", json!({})).unwrap();
            }
            6 => {
                if !self.app.ui.probe.inspector_text.contains("201") {
                    return;
                }
            }
            7 => self.click_record("orders"),
            8 => self.click("curl"),
            9 => {
                let text = arboard::Clipboard::new().unwrap().get_text().unwrap();
                assert!(
                    text.contains("curl -X POST 'https://api.example.com/orders'"),
                    "clipboard: {text}"
                );
                self.app.ui.reveal_inspector_tab = Some(4);
            }
            10 => self.click("tab:4"),
            11 => self.click("copy-detail"),
            12 => {
                let text = arboard::Clipboard::new().unwrap().get_text().unwrap();
                assert!(text.contains("created"));
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            13 => self.click("Console"),
            14 => {
                assert_eq!(self.app.ui.probe.logs.len(), 1);
                self.click("Network");
            }
            15 => self.click("pause"),
            16 => {
                assert!(self.app.capture.store().paused);
                self.click("pause");
            }
            17 => {
                assert!(!self.app.capture.store().paused);
                self.click("export");
            }
            18 => {
                self.app.ui.export_path = self
                    .output
                    .join("native-export.json")
                    .to_string_lossy()
                    .into_owned();
                self.click("save-export");
            }
            19 => {
                let v: serde_json::Value = serde_json::from_slice(
                    &std::fs::read(self.output.join("native-export.json")).unwrap(),
                )
                .unwrap();
                assert_eq!(v["network"].as_array().unwrap().len(), 2);
                self.click("clear");
            }
            20 => {
                assert!(self.app.capture.store().network().is_empty());
                assert!(self.app.ui.selected_request.is_none());
                assert!(self.output.join("native-network.png").exists());
                self.app.capture.stop();
                self.success
                    .store(true, std::sync::atomic::Ordering::Release);
                println!(
                    "PASS native F13: real WebSocket capture, pointer/key input, inspector, OS clipboard, console scope, pause/resume, export, clear, screenshot"
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
    for name in ["native-export.json", "native-network.png"] {
        match std::fs::remove_file(output.join(name)) {
            Ok(()) => (),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => panic!("cannot clear old smoke artifact {name}: {e}"),
        }
    }
    let success = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let check = success.clone();
    eframe::run_native(
        "F13 Native Acceptance",
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
