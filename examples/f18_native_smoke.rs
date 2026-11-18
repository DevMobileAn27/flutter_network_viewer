//! Native F18 acceptance uses the production host, input, persisted settings,
//! OS clipboard and screenshots. It exits unsuccessfully if any step fails.
use eframe::egui::{self, Event, Modifiers, PointerButton};
use flutter_network_viewer::{desktop_host::DesktopApp, settings::AppSettings};
use std::{
    collections::VecDeque,
    path::PathBuf,
    time::{Duration, Instant},
};
struct Smoke {
    app: DesktopApp,
    events: VecDeque<Vec<Event>>,
    step: usize,
    frames: usize,
    start: Instant,
    stage: Instant,
    output: PathBuf,
    path: PathBuf,
    uri: String,
    occupied: Option<std::net::TcpListener>,
    port: u16,
    rpc: Option<std::thread::JoinHandle<serde_json::Value>>,
    success: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl Smoke {
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
        self.events.push_back(vec![Event::PointerMoved(pos)]);
        for pressed in [true, false] {
            self.events.push_back(vec![Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            }]);
        }
        self.events.push_back(vec![]);
        self.events.push_back(vec![]);
    }
    fn scroll(&mut self, key: &str) {
        let pos = self.app.ui.probe.widgets[key].center();
        self.events.push_back(vec![Event::PointerMoved(pos)]);
        self.events.push_back(vec![Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0., -10000.),
            modifiers: Modifiers::NONE,
        }]);
        for _ in 0..20 {
            self.events.push_back(vec![]);
        }
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
            let screenshots: Vec<_> = input
                .events
                .iter()
                .filter(|e| matches!(e, Event::Screenshot { .. }))
                .cloned()
                .collect();
            input.events = events;
            input.events.extend(screenshots);
        }
    }
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        assert!(
            self.start.elapsed() < Duration::from_secs(60),
            "timeout at {}",
            self.step
        );
        self.app.update(ctx, frame);
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
        self.frames += 1;
        ctx.request_repaint_after(Duration::from_millis(16));
        if self.frames < 6 || !self.events.is_empty() {
            return;
        }
        match self.step {
            0 => {
                assert!(!self.app.mcp.running());
                self.app.ui.support.settings_page = 1;
                self.click("settings");
            }
            1 => self.click("settings-menu:mcp"),
            2 => self.click("mcp-enabled"),
            3 => {
                assert!(!self.app.mcp.running());
                assert!(self.app.mcp.status().error.is_some());
                self.occupied.take();
                self.app.ui.notifications.dismiss(false);
                self.screenshot(ctx, "conflict-en-light.png");
                self.click("mcp-retry");
            }
            4 => {
                assert!(self.app.mcp.running());
                assert_eq!(self.app.mcp.port(), self.port);
                self.click("mcp-copy-codex");
            }
            5 => {
                let text = arboard::Clipboard::new().unwrap().get_text().unwrap();
                assert_eq!(text, flutter_network_viewer::mcp::setup(self.port, false));
                self.app.ui.notifications.dismiss(false);
                self.click("mcp-copy-claude");
            }
            6 => {
                assert_eq!(
                    arboard::Clipboard::new().unwrap().get_text().unwrap(),
                    flutter_network_viewer::mcp::setup(self.port, true)
                );
                self.app.ui.notifications.dismiss(false);
                self.click("mcp-port");
            }
            7 => {
                self.occupied = Some(std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap());
                self.port = self.occupied.as_ref().unwrap().local_addr().unwrap().port();
                self.occupied.take();
                self.events.push_back(vec![
                    Event::Key {
                        key: egui::Key::A,
                        physical_key: None,
                        pressed: true,
                        repeat: false,
                        modifiers: Modifiers {
                            ctrl: true,
                            command: true,
                            ..Default::default()
                        },
                    },
                    Event::Text(self.port.to_string()),
                ]);
            }
            8 => self.events.push_back(vec![Event::Key {
                key: egui::Key::Enter,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }]),
            9 => {
                assert_eq!(self.app.ui.settings.values().mcp_port, self.port as i64);
                assert_eq!(self.app.mcp.port(), self.port);
                assert!(self.app.mcp.running());
                self.app.ui.notifications.dismiss(false);
                self.click("settings-close");
            }
            10 => {
                let settings = AppSettings::load(&self.path).unwrap();
                assert!(settings.values().mcp_enabled);
                assert_eq!(settings.values().mcp_port, self.port as i64);
                self.app.ui = flutter_network_viewer::desktop::DesktopUi::new(settings);
                self.app.capture.connect_manual(&self.uri, 0).unwrap();
            }
            11 => {
                if self.app.capture.store().network().len() < 3 {
                    return;
                }
                let port = self.port;
                self.rpc = Some(std::thread::spawn(move || {
                    use std::io::{Read, Write};
                    let mut socket = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
                    socket
                        .set_read_timeout(Some(Duration::from_secs(5)))
                        .unwrap();
                    let b=serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_capture_summary","arguments":{}}}).to_string();
                    write!(socket,"POST /mcp HTTP/1.1\r\nHost: localhost:{port}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{b}",b.len()).unwrap();
                    let mut data = String::new();
                    socket.read_to_string(&mut data).unwrap();
                    serde_json::from_str(data.split_once("\r\n\r\n").unwrap().1).unwrap()
                }));
            }
            12 => {
                if !self.rpc.as_ref().unwrap().is_finished() {
                    return;
                }
                let data = self.rpc.take().unwrap().join().unwrap();
                assert_eq!(data["result"]["structuredContent"]["request_count"], 3);
                self.app.ui.support.settings_page = 0;
                self.click("settings");
            }
            13 => self.click("language"),
            14 => self.click("language:vi"),
            15 => self.click("theme"),
            16 => self.click("theme:dark"),
            17 => {
                self.app.ui.notifications.dismiss(false);
                self.click("settings-menu:mcp");
            }
            18 => {
                assert!(ctx.style().visuals.dark_mode);
                assert!(
                    self.app
                        .ui
                        .probe
                        .support_text
                        .contains("MCP đang lắng nghe")
                );
                self.screenshot(ctx, "mcp-vi-dark.png");
                self.click("mcp-copy-codex");
            }
            19 => {
                assert_eq!(
                    arboard::Clipboard::new().unwrap().get_text().unwrap(),
                    flutter_network_viewer::mcp::setup(self.port, false)
                );
                self.app.ui.notifications.dismiss(false);
                self.click("mcp-enabled");
            }
            20 => {
                self.app.ui.notifications.dismiss(false);
                assert!(!self.app.mcp.running());
                assert!(std::net::TcpListener::bind(("127.0.0.1", self.port)).is_ok());
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(360., 300.)));
                self.stage = Instant::now();
            }
            21 => {
                if self.stage.elapsed() < Duration::from_millis(400) {
                    return;
                }
                self.scroll("settings-scroll");
            }
            22 => {
                assert!(self.app.ui.probe.support_scroll > 0.);
                self.screenshot(ctx, "mcp-narrow.png");
                self.click("mcp-copy-claude");
            }
            23 => {
                assert_eq!(
                    arboard::Clipboard::new().unwrap().get_text().unwrap(),
                    flutter_network_viewer::mcp::setup(self.port, true)
                );
                assert!(!AppSettings::load(&self.path).unwrap().values().mcp_enabled);
                if !self.output.join("mcp-narrow.png").exists() {
                    return;
                }
                std::fs::write(self.output.join("native-result.json"),serde_json::to_vec_pretty(&serde_json::json!({"result":"pass","production_host":true,"clipboard_os":true,"port_edit_restart":true,"settings_reload":true,"conflict_retry":true,"stop_released_port":true,"fresh_vm_http":true,"vi_dark":true,"narrow_scroll":true})).unwrap()).unwrap();
                self.success
                    .store(true, std::sync::atomic::Ordering::Release);
                println!(
                    "PASS F18 native: settings input, conflict retry, port restart/reload, Codex+Claude OS clipboard, fresh VM HTTP, VI/dark/narrow, stop releases port"
                );
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            _ => return,
        }
        self.step += 1;
    }
}
fn main() -> eframe::Result {
    let output = PathBuf::from(std::env::args().nth(1).expect("artifact directory"));
    std::fs::create_dir_all(&output).unwrap();
    let uri = std::env::args().nth(2).expect("fake VM URI");
    let path = output.join("settings.json");
    for name in [
        "settings.json",
        "conflict-en-light.png",
        "mcp-vi-dark.png",
        "mcp-narrow.png",
        "native-result.json",
    ] {
        let _ = std::fs::remove_file(output.join(name));
    }
    struct Restore(Option<String>);
    impl Drop for Restore {
        fn drop(&mut self) {
            if let Some(text) = self.0.take() {
                let _ = arboard::Clipboard::new().and_then(|mut c| c.set_text(text));
            }
        }
    }
    let _restore = Restore(
        arboard::Clipboard::new()
            .ok()
            .and_then(|mut c| c.get_text().ok()),
    );
    let mut settings = AppSettings::load(&path).unwrap();
    settings.set_auto_detect_vm_service(false).unwrap();
    let occupied = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = occupied.local_addr().unwrap().port();
    settings.set_mcp_port(port as i64).unwrap();
    let success = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let check = success.clone();
    eframe::run_native(
        "F18 Native Acceptance",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1200., 800.])
                .with_min_inner_size([280., 240.]),
            ..Default::default()
        },
        Box::new(move |_| {
            Ok(Box::new(Smoke {
                app: DesktopApp::new(settings),
                events: VecDeque::new(),
                step: 0,
                frames: 0,
                start: Instant::now(),
                stage: Instant::now(),
                output,
                path,
                uri,
                occupied: Some(occupied),
                port,
                rpc: None,
                success,
            }))
        }),
    )?;
    assert!(
        check.load(std::sync::atomic::Ordering::Acquire),
        "native window closed before completion"
    );
    Ok(())
}
