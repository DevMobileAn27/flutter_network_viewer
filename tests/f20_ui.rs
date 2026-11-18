use eframe::egui::{self, Event, Modifiers, PointerButton, Pos2, Rect};
use flutter_network_viewer::{
    capture::CaptureStore,
    desktop::Action,
    desktop::{DesktopUi, ShellStatus},
    updater::{Status, View},
};
struct Harness {
    ctx: egui::Context,
    ui: DesktopUi,
    store: CaptureStore,
    status: ShellStatus,
    time: f64,
    size: egui::Vec2,
    actions: Vec<flutter_network_viewer::desktop::Action>,
}
impl Harness {
    fn new() -> Self {
        let mut h = Self {
            ctx: Default::default(),
            ui: Default::default(),
            store: Default::default(),
            status: Default::default(),
            time: 0.,
            size: egui::vec2(1200., 800.),
            actions: vec![],
        };
        h.frame(vec![]);
        h.frame(vec![]);
        h
    }
    fn frame(&mut self, events: Vec<Event>) -> egui::FullOutput {
        self.ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)),
                time: Some(self.time),
                events,
                ..Default::default()
            },
            |ctx| {
                self.actions
                    .extend(self.ui.show(ctx, &self.store, &self.status));
            },
        )
    }
    fn click(&mut self, key: &str) -> egui::FullOutput {
        if self.ui.support.settings_open && !key.starts_with("settings-menu:") {
            let page = if key.starts_with("mcp-") || key == "settings-mcp" {
                Some("mcp")
            } else if key == "check-app-updates" || key == "settings-updates" {
                Some("about")
            } else if key == "theme" || key == "language" || key == "font-size" {
                Some("interface")
            } else {
                None
            };
            if let Some(page) = page
                && !self.ui.probe.widgets.contains_key(key)
            {
                self.click(&format!("settings-menu:{page}"));
            }
        }

        for _ in 0..4 {
            self.frame(vec![]);
        }
        if self.ui.support.settings_open
            && !key.contains(':')
            && key != "settings-close"
            && !key.starts_with("settings-menu:")
        {
            for _ in 0..20 {
                let pane = self.ui.probe.widgets["settings-scroll"];
                let target = self.ui.probe.widgets[key];
                if pane.contains_rect(target) {
                    break;
                }
                self.frame(vec![
                    Event::PointerMoved(pane.center()),
                    Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::vec2(
                            0.,
                            (pane.center().y - target.center().y).clamp(-350., 350.),
                        ),
                        modifiers: Modifiers::NONE,
                    },
                ]);
                for _ in 0..8 {
                    self.time += 0.02;
                    self.frame(vec![]);
                }
            }
        }
        let pos = self
            .ui
            .probe
            .widgets
            .get(key)
            .unwrap_or_else(|| panic!("missing {key}: {:?}", self.ui.probe.widgets.keys()))
            .center();
        self.frame(vec![Event::PointerMoved(pos)]);
        self.frame(vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: true,
            modifiers: Modifiers::NONE,
        }]);
        let o = self.frame(vec![Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed: false,
            modifiers: Modifiers::NONE,
        }]);
        self.frame(vec![]);
        o
    }
}

#[test]
fn updates_version_progress_retry_and_input_in_both_languages() {
    for lang in ["en", "vi"] {
        let mut h = Harness::new();
        h.ui.settings.set_language(lang).unwrap();
        h.ui.support.updates = View {
            supported: true,
            status: Status::Idle,
            busy: false,
        };
        h.click("settings");
        h.click("settings-menu:about");
        assert!(h.ui.probe.support_text.contains(env!("CARGO_PKG_VERSION")));
        h.click("check-app-updates");
        assert_eq!(h.actions, vec![Action::CheckUpdates]);
        h.actions.clear();
        for status in [Status::Checking, Status::Downloading, Status::Installing] {
            h.ui.support.updates.status = status;
            h.ui.support.updates.busy = true;
            h.frame(vec![]);
            h.click("check-app-updates");
            assert!(h.actions.is_empty());
            assert!(h.ui.probe.widgets.contains_key("update-progress"));
        }
        for status in [Status::Failed, Status::Current, Status::Available] {
            h.ui.support.updates.status = status;
            h.ui.support.updates.busy = false;
            h.frame(vec![]);
            let output = h.frame(vec![]);
            if status == Status::Failed {
                let expected = if lang == "vi" {
                    "Không thể kiểm tra cập nhật. Kiểm tra kết nối rồi thử lại."
                } else {
                    "Could not check for updates. Check your connection and try again."
                };
                assert!(!h.ui.probe.support_text.contains(expected));
                assert!(h.ui.probe.widgets.contains_key("notification"));
                assert!(
                    output.shapes.iter().any(
                        |s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==expected)
                    )
                );
            }
            assert!(!h.ui.probe.widgets.contains_key("update-progress"));
            h.ui.notifications.dismiss(false);
            h.frame(vec![]);
            assert!(!h.ui.probe.widgets.contains_key("notification"));
            h.click("check-app-updates");
            assert_eq!(h.actions, vec![Action::CheckUpdates]);
            h.actions.clear();
        }
        h.ui.support.updates.supported = false;
        h.frame(vec![]);
        h.click("check-app-updates");
        assert!(h.actions.is_empty());
        h.size = egui::vec2(360., 300.);
        for _ in 0..5 {
            h.frame(vec![]);
        }
        h.click("check-app-updates");
        assert!(h.actions.is_empty());
        assert!(
            Rect::from_min_size(Pos2::ZERO, h.size)
                .contains_rect(h.ui.probe.widgets["check-app-updates"])
        );
    }
}

#[test]
fn startup_update_failure_is_silent_but_manual_retry_reports_errors() {
    use flutter_network_viewer::updater::{Backend, Event as UpdateEvent, Updater};
    use std::{cell::RefCell, rc::Rc};
    #[derive(Default)]
    struct State {
        configure_fails: bool,
        check_fails: bool,
        events: Vec<UpdateEvent>,
        checks: Vec<bool>,
    }
    struct Fake(Rc<RefCell<State>>);
    impl Backend for Fake {
        fn configure(&mut self, _: &str) -> Result<(), String> {
            if self.0.borrow().configure_fails {
                Err("invalid bundle".into())
            } else {
                Ok(())
            }
        }
        fn check(&mut self, background: bool) -> Result<(), String> {
            let mut state = self.0.borrow_mut();
            state.checks.push(background);
            if state.check_fails {
                Err("offline".into())
            } else {
                Ok(())
            }
        }
        fn poll(&mut self) -> Vec<UpdateEvent> {
            std::mem::take(&mut self.0.borrow_mut().events)
        }
        fn close(&mut self) {}
    }
    for windows in [false, true] {
        for failure in ["configure", "check", "async", "timeout", "current"] {
            let state = Rc::new(RefCell::new(State::default()));
            state.borrow_mut().configure_fails = failure == "configure";
            state.borrow_mut().check_fails = failure == "check";
            let mut updater = Updater::new(
                Fake(state.clone()),
                Some("https://example.com/feed".into()),
                windows,
            );
            let mut h = Harness::new();
            updater.check(true, 0);
            if failure == "async" {
                state.borrow_mut().events.push(UpdateEvent::Failed);
            }
            if failure == "current" {
                state.borrow_mut().events.push(UpdateEvent::Current);
            }
            updater.tick(if failure == "timeout" { 60_000 } else { 1 });
            h.ui.support.updates = updater.view();
            for _ in 0..3 {
                h.frame(vec![]);
            }
            assert!(
                !h.ui.probe.widgets.contains_key("notification"),
                "startup {failure}, windows={windows}"
            );
            h.click("settings");
            h.click("settings-menu:about");
            assert!(!h.ui.probe.widgets.contains_key("notification"));
            h.click("check-app-updates");
            assert_eq!(h.actions, [Action::CheckUpdates]);
            state.borrow_mut().configure_fails = false;
            state.borrow_mut().check_fails = true;
            updater.check(false, 60_001);
            h.ui.support.updates = updater.view();
            h.frame(vec![]);
            let output = h.frame(vec![]);
            assert_eq!(updater.status, Status::Failed);
            assert!(h.ui.probe.widgets.contains_key("notification"));
            assert!(output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text().contains("Could not check for updates"))));
            h.click("notification-confirm");
            h.frame(vec![]);
            assert!(!h.ui.probe.widgets.contains_key("notification"));
        }
    }
}

#[test]
fn updates_hide_duplicate_status_and_offer_latest_version_in_both_languages() {
    for lang in ["en", "vi"] {
        let mut h = Harness::new();
        h.size.y = 1600.;
        h.ui.settings.set_language(lang).unwrap();
        h.ui.support.updates.supported = true;
        h.click("settings");
        h.click("settings-menu:about");
        for status in [Status::Idle, Status::Current, Status::Available] {
            h.ui.support.updates.status = status;
            h.ui.support.updates.busy = false;
            for _ in 0..3 {
                h.frame(vec![Event::PointerMoved(Pos2::ZERO)]);
            }
            let out = h.frame(vec![]);
            if status == Status::Available {
                assert!(!h.ui.probe.widgets.contains_key("notification"));
            }
            assert!(!h.ui.probe.widgets.contains_key("version-update-status"));
            let expected = match (lang, status) {
                ("vi", Status::Available) => "Cập nhật phiên bản mới nhất",
                (_, Status::Available) => "Update lastest version",
                ("vi", _) => "Kiểm tra cập nhật",
                _ => "Check for updates",
            };
            let button = h.ui.probe.widgets["check-app-updates"];
            let labels: Vec<_> = out
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    egui::Shape::Text(t) if t.galley.text() == expected => Some(t.pos),
                    _ => None,
                })
                .collect();
            assert_eq!(labels.len(), 1, "{lang}: {status:?}");
            assert!(button.contains(labels[0]));
            h.click("check-app-updates");
            assert_eq!(std::mem::take(&mut h.actions), [Action::CheckUpdates]);
            h.ui.notifications.dismiss(false);
        }
    }
}
