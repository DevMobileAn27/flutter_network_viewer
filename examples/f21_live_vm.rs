//! Executable acceptance against the real SDK; requires an explicit VM URI.
use flutter_network_viewer::{desktop::Action, desktop_host::DesktopApp, settings::AppSettings};
use serde_json::json;
use std::time::{Duration, Instant};
fn main() {
    let uri = std::env::args().nth(1).expect("SDK VM URI");
    let output = std::path::PathBuf::from(std::env::args().nth(2).expect("output directory"));
    let mut app = DesktopApp::with_diagnostics(
        AppSettings::from_preferences(json!({"autoDetectVmService":false})),
        output.join("diagnostics"),
    );
    app.dispatch(Action::Connect(uri), 0);
    let start = Instant::now();
    loop {
        app.tick(start.elapsed().as_millis() as u64);
        let store = app.capture.store();
        let complete = store.network().values().any(|r| {
            r.status() == Some(201)
                && r.request_event.to_string().contains("quantity")
                && r.completion_event.to_string().contains("Cà phê")
        });
        let logs = store
            .logs()
            .iter()
            .map(|r| r.event.to_string())
            .collect::<Vec<_>>();
        if complete
            && logs.iter().any(|s| s.contains("dart.stdout"))
            && logs.iter().any(|s| s.contains("dart.logging"))
        {
            std::fs::write(
                output.join("live-export.json"),
                serde_json::to_vec_pretty(&store.export_at("SDK acceptance")).unwrap(),
            )
            .unwrap();
            app.capture.stop();
            println!(
                "PASS F21 real Dart SDK HTTP 201 + request/response Unicode bodies + stdout + developer.log"
            );
            return;
        }
        assert!(
            start.elapsed() < Duration::from_secs(25),
            "SDK acceptance timeout; network={}, logs={}, error={:?}",
            store.network().len(),
            logs.len(),
            app.capture.last_error()
        );
        std::thread::sleep(Duration::from_millis(10));
    }
}
