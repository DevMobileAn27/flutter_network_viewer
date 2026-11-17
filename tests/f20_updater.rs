use flutter_network_viewer::updater::{Backend, Event, Status, Updater, feed_for};
use std::{cell::RefCell, rc::Rc};
#[derive(Default)]
struct Calls {
    feeds: Vec<String>,
    checks: Vec<bool>,
    events: Vec<Event>,
    closed: usize,
    fail: bool,
}
struct Fake(Rc<RefCell<Calls>>);
impl Backend for Fake {
    fn configure(&mut self, feed: &str) -> Result<(), String> {
        self.0.borrow_mut().feeds.push(feed.into());
        Ok(())
    }
    fn check(&mut self, background: bool) -> Result<(), String> {
        let mut c = self.0.borrow_mut();
        if c.fail {
            return Err("offline".into());
        }
        c.checks.push(background);
        Ok(())
    }
    fn poll(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.0.borrow_mut().events)
    }
    fn close(&mut self) {
        self.0.borrow_mut().closed += 1;
    }
}
fn host(windows: bool) -> (Updater<Fake>, Rc<RefCell<Calls>>) {
    let c = Rc::new(RefCell::new(Calls::default()));
    (
        Updater::new(
            Fake(c.clone()),
            feed_for(
                if windows { "windows" } else { "macos" },
                if windows { "x86_64" } else { "aarch64" },
            ),
            windows,
        ),
        c,
    )
}
fn emit(u: &mut Updater<Fake>, c: &Rc<RefCell<Calls>>, e: Event) {
    c.borrow_mut().events.push(e);
    u.tick(1);
}
#[test]
fn feeds_are_exact_and_unsupported_makes_no_native_calls() {
    for (os, cpu, suffix) in [
        ("macos", "aarch64", "macos-arm64"),
        ("macos", "x86_64", "macos-x86_64"),
        ("windows", "x86_64", "windows-x64"),
    ] {
        assert_eq!(
            feed_for(os, cpu).unwrap(),
            format!(
                "https://github.com/DevMobileAn27/flutter_network_viewer/releases/download/rust-updates/rust-appcast-{suffix}.xml"
            )
        );
    }
    for (os, cpu) in [
        ("linux", "x86_64"),
        ("windows", "aarch64"),
        ("macos", "unknown"),
    ] {
        let c = Rc::new(RefCell::new(Calls::default()));
        let mut u = Updater::new(Fake(c.clone()), feed_for(os, cpu), false);
        u.check(false, 0);
        drop(u);
        let c = c.borrow();
        assert!(c.feeds.is_empty() && c.checks.is_empty());
        assert_eq!(c.closed, 0);
    }
}
#[test]
fn serialized_retry_timeout_and_cleanup() {
    let (mut u, c) = host(false);
    u.check(true, 0);
    u.check(false, 0);
    assert_eq!(c.borrow().checks, [true]);
    assert_eq!(u.status, Status::Checking);
    emit(&mut u, &c, Event::Failed);
    assert_eq!(u.status, Status::Failed);
    u.check(false, 2);
    emit(&mut u, &c, Event::Current);
    emit(&mut u, &c, Event::Failed);
    assert_eq!(u.status, Status::Current);
    u.check(false, 3);
    emit(&mut u, &c, Event::Failed);
    assert_eq!(u.status, Status::Failed);
    u.check(false, 4);
    u.tick(60_004);
    assert_eq!(u.status, Status::Failed);
    u.check(false, 60_005);
    assert_eq!(u.status, Status::Checking);
    drop(u);
    assert!(c.borrow().closed >= 1);
}
#[test]
fn background_availability_waits_for_manual_action_before_native_progress() {
    let (mut u, c) = host(true);
    u.check(true, 0);
    emit(&mut u, &c, Event::Available);
    emit(&mut u, &c, Event::Available);
    assert_eq!(c.borrow().checks, [true]);
    assert_eq!(u.view().status, Status::Available);
    assert!(!u.busy());
    u.check(false, 1);
    assert_eq!(c.borrow().checks, [true, false]);
    emit(&mut u, &c, Event::Downloading);
    assert!(u.busy());
    emit(&mut u, &c, Event::Downloaded);
    emit(&mut u, &c, Event::Installing);
    assert_eq!(u.status, Status::Installing);
    u.check(false, 2);
    assert_eq!(c.borrow().checks, [true, false]);
    let (mut u, c) = host(true);
    u.check(false, 0);
    emit(&mut u, &c, Event::Current);
    emit(&mut u, &c, Event::Failed);
    assert_eq!(u.status, Status::Failed);
}
#[test]
fn backend_errors_are_contained_and_macos_does_not_duplicate_confirmation() {
    let (mut u, c) = host(false);
    c.borrow_mut().fail = true;
    u.check(false, 0);
    assert_eq!(u.status, Status::Failed);
    c.borrow_mut().fail = false;
    u.check(true, 1);
    emit(&mut u, &c, Event::Available);
    assert_eq!(c.borrow().checks, [true]);
    emit(&mut u, &c, Event::Finished);
    u.check(false, 2);
    assert_eq!(c.borrow().checks, [true, false]);
    drop(u);
    assert_eq!(c.borrow().closed, 1);
}

#[test]
fn finished_download_or_cancel_clears_busy_and_allows_retry() {
    let (mut u, c) = host(false);
    u.check(false, 0);
    emit(&mut u, &c, Event::Available);
    emit(&mut u, &c, Event::Downloading);
    emit(&mut u, &c, Event::Finished);
    assert!(!u.busy());
    u.check(false, 2);
    assert_eq!(c.borrow().checks, [false, false]);
}
