//! Native F17 acceptance uses the production host, input, persisted settings,
//! OS clipboard and screenshots. It exits unsuccessfully if any step fails.
use eframe::egui::{self, Event, Modifiers, PointerButton};
use flutter_network_viewer::{
    desktop_host::DesktopApp,
    desktop_support::{DISCUSSION_URL, NoticeKind, NoticeReceipt},
    settings::AppSettings,
};
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
    receipt: Option<NoticeReceipt>,
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
            0 => self.click("help"),
            1 => self.click("discussion-copy"),
            2 => {
                assert_eq!(
                    arboard::Clipboard::new().unwrap().get_text().unwrap(),
                    DISCUSSION_URL
                );
                assert_eq!(self.app.ui.probe.notification_title, "Success");
                self.screenshot(ctx, "help-en-light.png");
            }
            3 => {
                if !self.output.join("help-en-light.png").exists() {
                    return;
                }
                self.click("help-close");
            }
            4 => self.click("settings"),
            5 => self.click("language"),
            6 => self.click("language:vi"),
            7 => self.click("theme"),
            8 => self.click("theme:dark"),
            9 => {
                assert!(ctx.style().visuals.dark_mode);
                assert_eq!(self.app.ui.settings.values().language_code, "vi");
                self.screenshot(ctx, "settings-vi-dark.png");
            }
            10 => {
                if !self.output.join("settings-vi-dark.png").exists() {
                    return;
                }
                self.click("settings-close");
            }
            11 => {
                let settings = AppSettings::load(&self.path).unwrap();
                assert_eq!(settings.values().language_code, "vi");
                assert_eq!(
                    settings.values().theme_mode,
                    flutter_network_viewer::settings::ThemeMode::Dark
                );
                self.app.ui = flutter_network_viewer::desktop::DesktopUi::new(settings);
            }
            12 => self.click("help"),
            13 => {
                assert!(self.app.ui.probe.support_text.contains("Hướng dẫn sử dụng"));
                self.screenshot(ctx, "help-vi-dark.png");
            }
            14 => {
                if !self.output.join("help-vi-dark.png").exists() {
                    return;
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(360., 300.)));
                for _ in 0..20 {
                    self.events.push_back(vec![]);
                }
            }
            15 => {
                if ctx.content_rect().width() > 400. {
                    return;
                }
                self.scroll("help-scroll");
            }
            16 => {
                assert!(self.app.ui.probe.support_scroll > 0.);
                self.screenshot(ctx, "help-narrow.png");
            }
            17 => {
                if !self.output.join("help-narrow.png").exists() {
                    return;
                }
                self.click("help-close");
            }
            18 => {
                self.receipt = Some(self.app.ui.notifications.show(
                    NoticeKind::Warning,
                    "Mô tả thông báo dài. ".repeat(100),
                    true,
                ));
            }
            19 => self.scroll("notification"),
            20 => {
                assert!(self.app.ui.probe.widgets["notification"].width() <= 320.);
                self.screenshot(ctx, "notification-narrow.png");
            }
            21 => {
                if !self.output.join("notification-narrow.png").exists() {
                    return;
                }
                self.click("notification-confirm");
            }
            22 => {
                assert_eq!(self.receipt.take().unwrap().result(), Some(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(1100., 800.)));
                self.receipt = Some(self.app.ui.notifications.show(
                    NoticeKind::Info,
                    "Thông tin tự đóng sau 3 giây".into(),
                    false,
                ));
                self.stage = Instant::now();
            }
            23 => {
                if self.stage.elapsed() < Duration::from_millis(3200) {
                    return;
                }
                assert_eq!(self.receipt.take().unwrap().result(), Some(false));
                self.receipt = Some(self.app.ui.notifications.show(
                    NoticeKind::Success,
                    "Chờ xác nhận".into(),
                    true,
                ));
                self.stage = Instant::now();
            }
            24 => {
                if self.stage.elapsed() < Duration::from_millis(3200) {
                    return;
                }
                assert_eq!(self.receipt.as_ref().unwrap().result(), None);
                self.click("notification-cancel");
            }
            25 => {
                assert_eq!(self.receipt.take().unwrap().result(), Some(false));
                self.receipt = Some(self.app.ui.notifications.show(
                    NoticeKind::Error,
                    "Lỗi cần xác nhận".into(),
                    false,
                ));
                self.stage = Instant::now();
            }
            26 => {
                if self.stage.elapsed() < Duration::from_millis(3200) {
                    return;
                }
                assert_eq!(self.receipt.as_ref().unwrap().result(), None);
                self.click("notification-confirm");
            }
            27 => self.click("manual-url"),
            28 => {
                self.click("vm-uri");
                self.events.push_back(vec![Event::Text(self.uri.clone())]);
                self.events.push_back(vec![]);
            }
            29 => self.click("connect-submit"),
            30 => {
                if self.app.capture.store().network().len() != 3 {
                    return;
                }
                assert_eq!(self.app.ui.settings.values().vm_service_uri, self.uri);
                assert!(!self.app.ui.settings.values().auto_detect_vm_service);
                self.click("notification-confirm");
                self.click("settings");
            }
            31 => {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(360., 300.)));
                for _ in 0..20 {
                    self.events.push_back(vec![]);
                }
            }
            32 => {
                if ctx.content_rect().width() > 400. {
                    return;
                }
                self.scroll("settings-scroll");
            }
            33 => {
                assert!(self.app.ui.probe.support_scroll > 0.);
                self.screenshot(ctx, "settings-narrow.png");
            }
            34 => {
                if !self.output.join("settings-narrow.png").exists() {
                    return;
                }
                assert_eq!(self.receipt.take().unwrap().result(), Some(true));
                std::fs::write(self.output.join("native-result.json"),serde_json::to_vec_pretty(&serde_json::json!({"result":"pass","native_host":"DesktopApp","clipboard":DISCUSSION_URL,"manual_connection":true,"settings_reload":true,"narrow_scroll":true,"timers_wall_clock":true,"frames":self.frames})).unwrap()).unwrap();
                println!(
                    "PASS native F17: production DesktopApp, real OS clipboard, settings reload VI/dark, help/settings/toast narrow scrolling, wall-clock deadlines/confirm/cancel, fresh VM manual connection"
                );
                self.success
                    .store(true, std::sync::atomic::Ordering::Release);
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
        "help-en-light.png",
        "help-vi-dark.png",
        "settings-vi-dark.png",
        "help-narrow.png",
        "notification-narrow.png",
        "settings-narrow.png",
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
    let success = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let check = success.clone();
    eframe::run_native(
        "F17 Native Acceptance",
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
                receipt: None,
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
