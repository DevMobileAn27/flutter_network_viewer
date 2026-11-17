use flutter_network_viewer::{
    diagnostics::{Diagnostics, Event},
    resources::ResourceUsage,
};
use std::{cell::Cell, path::PathBuf, rc::Rc};
fn dir(name: &str) -> PathBuf {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("docs/artifacts/f19/test-data")
        .join(name);
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(&p).unwrap();
    p
}
#[test]
fn structured_rotation_and_queue_have_fixed_byte_budgets() {
    let p = dir("rotation");
    let mut d = Diagnostics::new(p.clone(), 300);
    for i in 0..20 {
        d.record(
            Event::ConnectionState {
                online: i,
                busy: false,
                searching: false,
                failed: false,
            },
            0,
        );
    }
    assert!(!p.join("viewer.log").exists());
    d.poll(249);
    assert!(!p.join("viewer.log").exists());
    d.poll(250);
    let files: Vec<_> = std::fs::read_dir(&p).unwrap().map(Result::unwrap).collect();
    assert_eq!(files.len(), 3);
    for f in files {
        let bytes = std::fs::read(f.path()).unwrap();
        assert!(bytes.len() <= 300);
        for line in String::from_utf8(bytes).unwrap().lines() {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            assert_eq!(v["pid"], std::process::id());
            assert!(v["time"].as_str().unwrap().ends_with('Z'));
            assert_eq!(v["event"], "connection_state");
        }
    }
    let last = std::fs::read_to_string(p.join("viewer.log")).unwrap();
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(last.lines().last().unwrap()).unwrap()["online"],
        19
    );
    for _ in 0..10000 {
        d.record(Event::AppStarted, 300);
    }
    assert!(d.pending_bytes() <= 128 * 1024);
    d.shutdown();
    assert_eq!(d.pending_bytes(), 0);
    let before = std::fs::read(p.join("viewer.log")).unwrap();
    d.record(Event::AppStarted, 500);
    d.poll(10000);
    assert_eq!(before, std::fs::read(p.join("viewer.log")).unwrap());
}
#[test]
fn unavailable_path_recovers_and_capture_continues_without_sensitive_data() {
    use flutter_network_viewer::{
        desktop::Action, desktop_host::DesktopApp, settings::AppSettings,
    };
    let p = dir("blocked");
    let blocker = p.join("blocker");
    std::fs::write(&blocker, "file").unwrap();
    let mut settings = AppSettings::default();
    settings.set_auto_detect_vm_service(false).unwrap();
    let mut app = DesktopApp::with_diagnostics(settings, blocker.join("nested"));
    app.dispatch(
        Action::Connect("ws://127.0.0.1:bad/SECRET_VM_AUTH/ws?token=SECRET".into()),
        0,
    );
    app.tick(250);
    assert!(!app.capture.store().paused);
    assert_eq!(std::fs::read_to_string(&blocker).unwrap(), "file");
    std::fs::remove_file(&blocker).unwrap();
    app.dispatch(Action::SetPaused(true), 300);
    // Exercise an actual sensitive caller value with a writable log path too.
    app.dispatch(
        Action::Connect("ws://127.0.0.1:bad/SECRET_VM_AUTH/ws?token=SECRET".into()),
        301,
    );
    app.tick(550);
    assert!(app.capture.store().paused);
    drop(app);
    let text = std::fs::read_to_string(blocker.join("nested/viewer.log")).unwrap();
    assert!(text.contains("pause_changed"));
    assert!(text.contains("connection_attempt") && text.contains("connection_failed"));
    for forbidden in [
        "SECRET", "ws://", "body", "header", "console", "quantity", "customer",
    ] {
        assert!(!text.contains(forbidden), "{text}");
    }
    assert!(text.contains("app_stopped"));
}
#[test]
fn samples_immediately_every_two_seconds_clear_failure_recover_and_stop() {
    let calls = Rc::new(Cell::new(0));
    let c = calls.clone();
    let mut r = ResourceUsage::new(move || {
        c.set(c.get() + 1);
        if c.get() == 2 {
            Err(std::io::Error::other("sample failed"))
        } else {
            Ok(171_000_000 + c.get() * 1_000_000)
        }
    });
    r.tick(0);
    assert_eq!(r.label(), "RAM 172 MB");
    r.tick(1999);
    assert_eq!(calls.get(), 1);
    r.tick(2000);
    assert_eq!(r.label(), "RAM —");
    assert_eq!(r.memory_bytes(), None);
    r.tick(4000);
    assert_eq!(r.label(), "RAM 174 MB");
    r.stop();
    r.tick(99999);
    assert_eq!(calls.get(), 3);
    drop(r);
    assert_eq!(Rc::strong_count(&calls), 1);
}
#[test]
fn process_sample_is_real_and_positive() {
    assert!(flutter_network_viewer::resources::sample_process_memory().unwrap() > 0);
}

#[test]
fn desktop_startup_never_scans_or_connects_saved_settings() {
    use flutter_network_viewer::{desktop_host::DesktopApp, settings::AppSettings};
    let p = dir("manual-startup");
    let settings = AppSettings::from_preferences(
        serde_json::json!({"autoDetectVmService":true,"vmServiceUri":"ws://127.0.0.1:12345/ws"}),
    );
    let mut app = DesktopApp::with_diagnostics(settings, p.clone());
    app.tick(0);
    assert!(!app.capture.searching());
    assert!(!app.capture.busy());
    assert_eq!(app.capture.connections(), 0);
    drop(app);
    let _ = std::fs::remove_dir_all(p);
}
