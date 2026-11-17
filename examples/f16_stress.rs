//! Executable renderer stress with process RSS and measured frame percentiles.
use eframe::egui;
use flutter_network_viewer::{
    capture::CaptureStore,
    desktop::{DesktopUi, ShellStatus},
};
use serde_json::json;
use std::time::Instant;
fn rss() -> u64 {
    let out = std::process::Command::new("/bin/ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}
fn frame(ctx: &egui::Context, ui: &mut DesktopUi, store: &CaptureStore) -> f64 {
    let start = Instant::now();
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1440., 900.),
            )),
            ..Default::default()
        },
        |ctx| {
            ui.show(ctx, store, &ShellStatus::default());
        },
    );
    start.elapsed().as_secs_f64() * 1000.
}
fn main() {
    let output = std::env::args().nth(1).expect("measurement output path");
    let ctx = egui::Context::default();
    let mut ui = DesktopUi::default();
    let mut store = CaptureStore::default();
    let baseline = rss();
    let mut cycles = vec![];
    for cycle in 0..4 {
        let body = json!(
            (0..20000)
                .map(|i| format!("row-{i}-😀-#ff0000"))
                .collect::<Vec<_>>()
        )
        .to_string();
        store.ingest(&json!({"sdk":{"schemaVersion":1},"session":{"id":"stress"},"events":(0..4500).map(|i|json!({"requestId":format!("r{i}"),"source":"js.fetch","phase":"request","request":{"method":"POST","url":format!("https://local/{i}"),"body":if i==4499{&body}else{"small"}}})).collect::<Vec<_>>()})).unwrap();
        ui.selected_request = Some(("stress".into(), "r4499".into()));
        ui.settings.set_request_detail_tab(2).unwrap();
        let cold = frame(&ctx, &mut ui, &store);
        for _ in 0..20 {
            frame(&ctx, &mut ui, &store);
        }
        let loaded = rss();
        let preparations = ui.detail_preparations;
        let mut times = (0..200)
            .map(|_| frame(&ctx, &mut ui, &store))
            .collect::<Vec<_>>();
        times.sort_by(f64::total_cmp);
        assert_eq!(preparations, ui.detail_preparations);
        assert!(ui.probe.network_rows_rendered < 60 && ui.probe.json.rows.len() < 60);
        assert!(ui.probe.json.total_rows > 20000);
        assert!(
            times[190] < 50.,
            "warm frame p95 exceeded 50ms: {}",
            times[190]
        );
        let warm = rss();
        assert!(warm < loaded + 64 * 1024, "warm frames grew RSS by >64MiB");
        let rendered = ui.probe.json.rows.len();
        store.clear();
        frame(&ctx, &mut ui, &store);
        assert!(ui.selected_request.is_none());
        assert_eq!(ui.probe.retained_detail_bytes, 0);
        cycles.push(json!({"cycle":cycle,"cold_frame_ms":cold,"warm_p50_ms":times[100],"warm_p95_ms":times[190],"warm_p99_ms":times[198],"rss_loaded_kib":loaded,"rss_warm_kib":warm,"rss_after_clear_kib":rss(),"rendered_json_rows":rendered,"total_json_rows":20002,"detail_preparations":preparations}));
    }
    let after = rss();
    assert!(
        after < baseline + 256 * 1024,
        "repeated select/Clear retained >256MiB"
    );
    let report = json!({"platform":std::env::consts::OS,"build":"debug","baseline_rss_kib":baseline,"final_rss_kib":after,"cycles":cycles,"limits":{"warm_p95_ms":50,"warm_rss_growth_mib":64,"four_cycles_rss_growth_mib":256},"note":"RSS includes allocator/font/galley caches; Clear resource ownership assertions are separate from RSS reclamation."});
    std::fs::write(output, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    println!("PASS F16 renderer stress: {report}");
}
