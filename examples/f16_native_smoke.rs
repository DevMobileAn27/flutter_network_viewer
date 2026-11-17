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
    samples: Vec<f64>,
    success: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl Smoke {
    fn request(&mut self, name: &str) {
        let key = self
            .app
            .capture
            .store()
            .network()
            .iter()
            .find(|(_, r)| {
                r.request_event["request"]["url"]
                    .as_str()
                    .is_some_and(|url| url.ends_with(name))
            })
            .unwrap()
            .0
            .clone();
        self.click(&format!("request:{}:{}", key.0, key.1));
    }
    fn click(&mut self, key: &str) {
        let pos = self
            .app
            .ui
            .probe
            .widgets
            .get(key)
            .unwrap_or_else(|| {
                panic!(
                    "step {} missing {key}: {:?}",
                    self.step,
                    self.app.ui.probe.widgets.keys()
                )
            })
            .center();
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
            self.start.elapsed() < Duration::from_secs(60),
            "native smoke timeout at step {}",
            self.step
        );
        let measured = Instant::now();
        self.app.update(ctx, frame);
        if self.step == 18 {
            self.samples.push(measured.elapsed().as_secs_f64() * 1000.);
        }
        for event in ctx.input(|i| i.events.clone()) {
            if let Event::Screenshot { image, .. } = event {
                image::save_buffer(
                    self.output.join(if self.step < 15 {
                        "native-body.png"
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
                if self.app.capture.store().network().len() != 3 {
                    ctx.request_repaint();
                    return;
                }
                self.request("image");
                self.app.ui.reveal_inspector_tab = Some(4);
            }
            4 => {
                self.click("tab:4");
            }
            5 => {
                if self.app.ui.probe.image_textures != 1 {
                    ctx.request_repaint();
                    return;
                }
                self.click("image-open");
            }
            6 => {
                assert!(self.app.ui.probe.widgets.contains_key("image-modal"));
                self.click("image-zoom-in");
            }
            7 => {
                assert!(self.app.ui.probe.image_zoom > 1.);
                assert_eq!(self.app.ui.probe.image_textures, 1);
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            }
            8 => {
                if !self.output.join("native-body.png").exists() {
                    ctx.request_repaint();
                    return;
                }
                self.click("image-close");
            }
            9 => {
                self.click("body-raw");
            }
            10 => {
                self.click("copy-detail");
            }
            11 => {
                use base64::Engine;
                assert_eq!(
                    clipboard(),
                    base64::engine::general_purpose::STANDARD
                        .encode(include_bytes!("../tests/fixtures/preview.png"))
                );
                self.request("form");
            }
            12 => {
                assert_eq!(self.app.ui.probe.body_kind, "form");
                assert!(self.app.ui.probe.body_preview.contains("hello world"));
                self.click("body-raw");
            }
            13 => {
                self.click("copy-detail");
            }
            14 => {
                assert_eq!(clipboard(), "a=hello+world&a=%F0%9F%98%80");
                self.request("large-json");
            }
            15 => {
                assert!(self.app.ui.probe.json.total_rows > 20000);
                assert!(self.app.ui.probe.json.rows.len() < 70);
                self.click("json-find");
                self.events.push_back(vec![Event::Text("row-19999".into())]);
                self.events.push_back(vec![]);
            }
            16 => {
                assert_eq!(self.app.ui.probe.json.matches, 1);
                assert!(self.app.ui.probe.json.scroll.y > 10000.);
                assert!(self.app.ui.probe.json.highlighted > 0);
                self.click("json-copy");
            }
            17 => {
                assert!(clipboard().contains("row-19999-😀"));
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            }
            18 => {
                if !self.output.join("native-json.png").exists() {
                    ctx.request_repaint();
                    return;
                }
                // Settle 180 real native frames and measure the production update.
                if self.frames < 180 {
                    ctx.request_repaint();
                    return;
                }
                let mut samples = self.samples.clone();
                samples.sort_by(f64::total_cmp);
                let report = json!({"native_frames":samples.len(),"update_ms_p50":samples[samples.len()/2],"update_ms_p95":samples[samples.len()*95/100],"rss_loaded_kib":rss(),"json_rows_total":self.app.ui.probe.json.total_rows,"json_rows_rendered":self.app.ui.probe.json.rows.len(),"detail_preparations":self.app.ui.detail_preparations,"body_preparations":self.app.ui.body_preparations});
                std::fs::write(
                    self.output.join("native-measurement.json"),
                    serde_json::to_vec_pretty(&report).unwrap(),
                )
                .unwrap();
                self.request("form");
            }
            19 => {
                assert!(self.app.ui.probe.body_preview.contains("hello world"));
                let (mut ws, _) = tungstenite::connect(self.uri.as_str()).unwrap();
                ws.send(tungstenite::Message::Text(
                    json!({"jsonrpc":"2.0","id":1,"method":"f13.complete"})
                        .to_string()
                        .into(),
                ))
                .unwrap();
                ws.read().unwrap();
                ws.close(None).unwrap();
            }
            20 => {
                if !self.app.ui.probe.body_preview.contains("live: updated") {
                    ctx.request_repaint();
                    return;
                }
                assert!(!self.app.ui.probe.body_preview.contains("hello"));
                self.request("image");
            }
            21 => {
                if !self.app.ui.probe.body_preview.contains("unavailable") {
                    ctx.request_repaint();
                    return;
                }
                assert_eq!(self.app.ui.probe.image_textures, 0);
                self.click("clear");
            }
            22 => {
                assert!(self.app.ui.selected_request.is_none());
                assert_eq!(self.app.ui.probe.retained_detail_bytes, 0);
                assert_eq!(self.app.ui.probe.image_textures, 0);
                assert!(self.app.capture.store().network().is_empty());
                self.success
                    .store(true, std::sync::atomic::Ordering::Release);
                println!(
                    "PASS native F16: fresh WebSocket + production host, Form/Raw OS clipboard, image shared texture/enlarge/zoom/clipping, 20k-row JSON viewport/find/Unicode clipboard, Clear resources, screenshots and frame/RSS measurement"
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
        for name in ["native-json.png", "native-body.png"] {
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
        "F16 Native Acceptance",
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
                samples: vec![],
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

fn rss() -> u64 {
    let output = std::process::Command::new("/bin/ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .unwrap();
    String::from_utf8(output.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}
