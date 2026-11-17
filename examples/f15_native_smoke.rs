//! Executable native acceptance: run with a URI printed by f13_fake_vm.
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
            let screenshots = input
                .events
                .iter()
                .filter(|e| matches!(e, Event::Screenshot { .. }))
                .cloned()
                .collect::<Vec<_>>();
            input.events = events;
            input.events.extend(screenshots);
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
                    self.output.join(if self.step < 20 {
                        "native-beautify.png"
                    } else {
                        "native-json.png"
                    }),
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
                if self.app.capture.store().network().len() != 2 {
                    return;
                }
                self.click("JSON");
            }
            4 => {
                self.click("json-input");
                let value = json!({"group":{"name":"needle","other":"needle"},"color":"#ff0000",
                    "items": (0..100).map(|i|format!("item-{i}-{}", "long".repeat(50))).collect::<Vec<_>>()});
                self.events.push_back(vec![Event::Text(value.to_string())]);
                self.events.push_back(vec![]);
            }
            5 => self.click("json-format"),
            6 => {
                assert!(self.app.ui.probe.json.text.contains("item-99"));
                self.click("json-copy");
            }
            7 => {
                assert_eq!(clipboard(), self.app.ui.probe.json.text);
                self.click("json-fold:1");
            }
            8 => {
                assert!(!self.app.ui.probe.json.rows.contains(&2));
                self.click("json-input");
                self.events.push_back(vec![key(egui::Key::F)]);
                self.events.push_back(vec![Event::Text("needle".into())]);
                self.events.push_back(vec![]);
            }
            9 => {
                assert_eq!(self.app.ui.probe.json.matches, 2);
                assert!(self.app.ui.probe.json.highlighted > 0);
                assert!(self.app.ui.probe.json.rows.contains(&2));
                self.click("json-next");
            }
            10 => {
                assert_eq!(self.app.ui.probe.json.active, Some(1));
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                self.click("json-tree");
            }
            11 => {
                assert_eq!(self.app.ui.probe.json.active, Some(1));
                self.click("Network");
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(860., 760.)));
            }
            12 => self.click("JSON"),
            13 => {
                assert!(self.app.ui.probe.json.tree);
                assert_eq!(self.app.ui.probe.json.query, "needle");
                assert_eq!(self.app.ui.probe.json.active, Some(1));
                self.click("json-code");
            }
            14 => {
                let key = self.app.ui.probe.json.swatches[0].clone();
                self.click(&key);
            }
            15 => {
                assert_eq!(clipboard(), "#ff0000");
                let pos = self.app.ui.probe.widgets["json-output"].center();
                self.events.push_back(vec![
                    Event::PointerMoved(pos),
                    Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::vec2(-450., -650.),
                        modifiers: Modifiers::NONE,
                    },
                ]);
                self.events.push_back(vec![]);
            }
            16 => {
                assert!(
                    self.app.ui.probe.json.scroll.x > 0. && self.app.ui.probe.json.scroll.y > 0.
                );
                self.click("json-output");
                self.events.push_back(vec![key(egui::Key::A)]);
                self.events.push_back(vec![Event::Copy]);
                self.events.push_back(vec![]);
            }
            17 => {
                assert_eq!(clipboard(), self.app.ui.probe.json.text);
                self.click("json-input");
                self.events
                    .push_back(vec![key(egui::Key::A), Event::Text("{\n \"x\": }".into())]);
                self.events.push_back(vec![key(egui::Key::Enter)]);
                self.events.push_back(vec![]);
            }
            18 => {
                assert!(
                    self.app.ui.probe.json.error.contains("line 2"),
                    "error={:?} draft={:?}",
                    self.app.ui.probe.json.error,
                    self.app.ui.probe.json.draft
                );
                assert!(self.app.ui.probe.json.text.contains("item-99"));
                self.click("json-input");
                self.events
                    .push_back(vec![key(egui::Key::A), Event::Text("null".into())]);
                self.events.push_back(vec![key(egui::Key::Enter)]);
                self.events.push_back(vec![]);
            }
            19 => {
                assert_eq!(self.app.ui.probe.json.text, "null");
                assert!(self.app.ui.probe.json.error.is_empty());
                self.click("Network");
            }
            20 => {
                let record = self
                    .app
                    .ui
                    .probe
                    .requests
                    .iter()
                    .find(|(_, id)| id.contains("orders"))
                    .unwrap();
                self.click(&format!("request:{}:{}", record.0, record.1));
            }
            21 => {
                self.app.ui.reveal_inspector_tab = Some(4);
            }
            22 => self.click("tab:4"),
            23 => {
                assert!(
                    self.app
                        .ui
                        .probe
                        .json
                        .text
                        .contains("\"status\": \"created\"")
                );
                self.click("json-tree");
            }
            24 => {
                assert!(self.app.ui.probe.json.tree);
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                self.click("json-copy");
            }
            25 => {
                let value: Value = serde_json::from_str(&clipboard()).unwrap();
                assert_eq!(value["id"], 42);
                if !self.output.join("native-json.png").exists()
                    || !self.output.join("native-beautify.png").exists()
                {
                    return;
                }
                self.app.capture.stop();
                self.success
                    .store(true, std::sync::atomic::Ordering::Release);
                println!(
                    "PASS native F15: production host + fresh WebSocket, Format/Ctrl+Enter, error line/column, draft retained, JSON/Tree/find/fold/reveal, active result retained through tab/resize, swatch OS clipboard, two-axis scroll, Ctrl+A Copy complete document without gutter, JSON body inspector, screenshot"
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
fn clipboard() -> String {
    arboard::Clipboard::new().unwrap().get_text().unwrap()
}
fn key(key: egui::Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers {
            ctrl: true,
            command: true,
            mac_cmd: true,
            ..Modifiers::NONE
        },
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
        for name in ["native-json.png", "native-beautify.png"] {
            match std::fs::remove_file(output.join(name)) {
                Ok(()) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => panic!("cannot clear old smoke artifact {name}: {e}"),
            }
        }
    }
    let success = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let check = success.clone();
    eframe::run_native(
        "F15 Native Acceptance",
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
