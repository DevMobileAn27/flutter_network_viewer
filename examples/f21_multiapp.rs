//! Real DDS HTTP resolution, VM probes, picker and multi-app capture sockets.
use flutter_network_viewer::{
    coordinator::{CoordinatorConfig, ThreadConnector},
    discovery::{AppDiscovery, DiscoveryFactory, Target, discover_processes},
    vm::{ConnectionCancel, RpcError},
};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};
struct Processes(String);
impl DiscoveryFactory for Processes {
    fn begin(&mut self, cancel: ConnectionCancel) -> mpsc::Receiver<Result<Vec<Target>, RpcError>> {
        let text = self.0.clone();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = discover_processes(
                &text,
                &cancel,
                Duration::from_secs(3),
                std::process::id(),
                std::env::consts::OS,
            );
            eprintln!("DDS result: {result:?}");
            let _ = tx.send(result);
        });
        rx
    }
}
fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let text = args[..2]
        .iter()
        .map(|uri| format!("dart development-service --vm-service-uri={uri}\n"))
        .collect();
    eprintln!("process list: {text}");
    let mut app = AppDiscovery::new(
        ThreadConnector::default(),
        Processes(text),
        CoordinatorConfig::production(),
    );
    let start = Instant::now();
    let now = || start.elapsed().as_millis() as u64;
    app.start_scan(now());
    loop {
        app.tick(now());
        if app.targets().len() == 2 {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(10),
            "discover two VMs: {:?}",
            app.last_error()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(app.store().network().is_empty());
    let key = app.targets()[0].vm_key.clone();
    assert!(app.select(&key, now()));
    loop {
        app.tick(now());
        if app.store().network().len() == 2 && !app.searching() {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(15));
        std::thread::sleep(Duration::from_millis(5));
    }
    let first = app.store().network().keys().cloned().collect::<Vec<_>>();
    app.start_scan(now());
    loop {
        app.tick(now());
        if app.store().network().len() == 4 && !app.searching() && app.store().logs().len() >= 2 {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(25),
            "second app requests={} logs={} error={:?}",
            app.store().network().len(),
            app.store().logs().len(),
            app.last_error()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(app.store().sessions().len(), 2);
    for key in first {
        assert!(app.store().network().contains_key(&key));
    }
    app.set_paused(true);
    assert!(app.store().paused);
    app.clear(now());
    for _ in 0..250 {
        app.tick(now());
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(app.store().network().is_empty() && app.store().logs().is_empty());
    std::fs::write(&args[2],serde_json::to_vec_pretty(&serde_json::json!({"result":"pass","dds_http_and_vm_probe":true,"picker":true,"two_live_capture_sockets":true,"shared_console":true,"preserved_first_app":true,"global_pause_clear":true})).unwrap()).unwrap();
    app.stop();
    println!(
        "PASS F21 combined DDS -> choice -> first app -> scan second app -> shared captures/console -> pause/clear"
    );
}
