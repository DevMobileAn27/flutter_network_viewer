//! Real Sparkle callbacks and production host/settings input. No installer exists.
use eframe::egui::{self, Event, Modifiers, PointerButton};
use flutter_network_viewer::{
    desktop_host::DesktopApp,
    settings::AppSettings,
    updater::{NativeBackend, Status, Updater},
};
use std::{
    collections::VecDeque,
    path::PathBuf,
    time::{Duration, Instant},
};
// Observe our own AppKit process; never dismiss a modal to make a test pass.
#[cfg(target_os = "macos")]
fn native_modal_is_open() -> bool {
    use std::ffi::{c_char, c_void};
    #[link(name = "objc")]
    unsafe extern "C" {
        fn objc_getClass(name: *const c_char) -> *mut c_void;
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        fn objc_msgSend(receiver: *mut c_void, selector: *mut c_void) -> *mut c_void;
    }
    // Both selectors have zero arguments and return Objective-C object pointers.
    // The smoke renderer executes this on AppKit's main thread.
    unsafe {
        let app = objc_msgSend(
            objc_getClass(c"NSApplication".as_ptr()),
            sel_registerName(c"sharedApplication".as_ptr()),
        );
        !objc_msgSend(app, sel_registerName(c"modalWindow".as_ptr())).is_null()
    }
}

struct Smoke {
    app: DesktopApp,
    output: PathBuf,
    events: VecDeque<Vec<Event>>,
    start: Instant,
    stage: Instant,
    step: u8,
    available_only: bool,
    localized_result_verified: bool,
    done: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl Smoke {
    fn click(&mut self, key: &str) {
        let pos = self.app.ui.probe.widgets[key].center();
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
    }
    fn shot(&self, ctx: &egui::Context, name: &str) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::new(
            name.to_owned(),
        )));
    }
}
impl eframe::App for Smoke {
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        if let Some(mut events) = self.events.pop_front() {
            events.extend(
                input
                    .events
                    .iter()
                    .filter(|e| matches!(e, Event::Screenshot { .. }))
                    .cloned(),
            );
            input.events = events;
        }
    }
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        assert!(
            self.start.elapsed() < Duration::from_secs(40),
            "native timeout stage {} status {:?}",
            self.step,
            self.app.updater.status
        );
        self.app.update(ctx, frame);
        #[cfg(target_os = "macos")]
        assert!(
            !native_modal_is_open(),
            "manual update check opened a blocking AppKit modal instead of the app notification"
        );
        ctx.request_repaint_after(Duration::from_millis(16));
        for e in ctx.input(|i| i.events.clone()) {
            if let Event::Screenshot {
                image, user_data, ..
            } = e
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
        if !self.events.is_empty() {
            return;
        }
        if self.available_only {
            if self.app.updater.status != Status::Available {
                return;
            }
            if self.step == 0 {
                self.shot(ctx, "native-available.png");
                self.click("settings");
                self.stage = Instant::now();
                self.step = 1;
                return;
            }
            if self.step == 1 {
                assert!(self.app.ui.support.settings_open);
                // A confirmed update must still open Sparkle's native update UI
                // from the Settings action. Never press its Install button.
                self.click("check-app-updates");
                self.stage = Instant::now();
                self.step = 2;
                return;
            }
            if self.stage.elapsed() < Duration::from_millis(700) {
                return;
            }
            assert!(self.output.join("native-available.png").exists());
            self.app.updater = Updater::new(NativeBackend::default(), None, false);
            self.done.store(true, std::sync::atomic::Ordering::Release);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        match self.step {
            0 => {
                if self.app.updater.status != Status::Failed {
                    return;
                } // malformed feed, actual Sparkle error
                std::fs::write(
                    self.output.join("feed.xml"),
                    include_str!("../tests/fixtures/f20_current.xml"),
                )
                .unwrap();
                self.app.updater.check(true, 100);
                self.stage = Instant::now();
            }
            1 => {
                if self.app.updater.status != Status::Current {
                    return;
                }
                assert!(self.stage.elapsed() < Duration::from_secs(20));
                self.click("settings");
            }
            2 => {
                assert!(self.app.ui.support.settings_open);
                assert!(
                    self.app
                        .ui
                        .probe
                        .support_text
                        .contains(env!("CARGO_PKG_VERSION"))
                );
                self.shot(ctx, "settings-current-en.png");
                self.click("check-app-updates");
            }
            3 => {
                if self.app.updater.status == Status::Checking {
                    self.shot(ctx, "settings-checking.png");
                }
                if self.app.updater.status != Status::Current {
                    return;
                }
                self.app.ui.settings.set_language("vi").unwrap();
                self.app
                    .ui
                    .settings
                    .set_theme_mode(flutter_network_viewer::settings::ThemeMode::Dark)
                    .unwrap();
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(480., 360.)));
                self.stage = Instant::now();
            }
            4 => {
                if self.stage.elapsed() < Duration::from_millis(500) {
                    return;
                }
                assert_eq!(self.app.updater.status, Status::Current);
                // Update results now appear as localized notifications, not
                // inline status text in the Settings section.
                if !self.localized_result_verified {
                    assert_eq!(
                        self.app.ui.probe.notification_title,
                        flutter_network_viewer::desktop_support::tr(ctx, "Success")
                    );
                    assert!(self.app.ui.probe.widgets.contains_key("notification"));
                    self.localized_result_verified = true;
                }
                if !ctx
                    .content_rect()
                    .contains_rect(self.app.ui.probe.widgets["check-app-updates"])
                {
                    // On a narrow window the updates section is below the fold.
                    let pos = self.app.ui.probe.widgets["settings-scroll"].center();
                    self.events.push_back(vec![Event::PointerMoved(pos)]);
                    self.events.push_back(vec![Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::vec2(0., -10000.),
                        modifiers: Modifiers::NONE,
                    }]);
                    for _ in 0..12 {
                        self.events.push_back(vec![]);
                    }
                    return;
                }
                assert!(self.app.ui.probe.support_scroll > 0.);
                self.shot(ctx, "settings-current-vi-narrow.png");
                self.stage = Instant::now();
            }
            5 => {
                if self.stage.elapsed() < Duration::from_millis(500)
                    || !self.output.join("settings-current-vi-narrow.png").exists()
                {
                    return;
                }
                // Dropping the real native owner removes its weak delegate; late work cannot
                // reference the Rust host. Replacing it also exercises stop before app close.
                self.app.updater = Updater::new(NativeBackend::default(), None, false);
                self.done.store(true, std::sync::atomic::Ordering::Release);
                println!("F20 assertions complete; waiting for the native event loop to return");
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            _ => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        }
        self.step += 1;
    }
}
fn main() -> eframe::Result {
    let output = PathBuf::from(std::env::args().nth(1).unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let feed = std::env::args().nth(2).unwrap();
    let available_only = std::env::args().nth(3).as_deref() == Some("available");
    let mut settings = AppSettings::load(output.join("settings.json")).unwrap();
    settings.set_auto_detect_vm_service(false).unwrap();
    let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let result = done.clone();
    let completed_output = output.clone();
    eframe::run_native(
        "F20 Updater Native Acceptance",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1100., 800.])
                .with_min_inner_size([360., 300.]),
            ..Default::default()
        },
        Box::new(move |_| {
            let mut app = DesktopApp::with_diagnostics(settings, output.join("diagnostics"));
            app.updater = Updater::new(NativeBackend::default(), Some(feed), false);
            Ok(Box::new(Smoke {
                app,
                output,
                events: VecDeque::new(),
                start: Instant::now(),
                stage: Instant::now(),
                step: 0,
                available_only,
                localized_result_verified: false,
                done,
            }))
        }),
    )?;
    assert!(
        result.load(std::sync::atomic::Ordering::Acquire),
        "native closed before success"
    );
    let (filename, evidence) = if available_only {
        (
            "available-result.json",
            serde_json::json!({"result":"pass","real_sparkle_available":true,"manual_available_action":true,"installer_requested":false,"listener_drop":true,"event_loop_returned":true}),
        )
    } else {
        (
            "native-result.json",
            serde_json::json!({"result":"pass","real_sparkle":true,"native_error_retry_current":true,"production_host":true,"settings_input":true,"version":true,"vi_dark_narrow":true,"listener_drop":true,"installer_requested":false,"event_loop_returned":true}),
        )
    };
    std::fs::write(
        completed_output.join(filename),
        serde_json::to_vec_pretty(&evidence).unwrap(),
    )
    .unwrap();
    println!("PASS F20: assertions, updater disposal and native event-loop return ({filename})");
    Ok(())
}
