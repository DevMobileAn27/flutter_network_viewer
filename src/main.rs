use eframe::egui;
use flutter_network_viewer::{desktop_host::DesktopApp, settings::AppSettings};
use std::path::PathBuf;

fn settings_path() -> Option<PathBuf> {
    let base = if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|v| PathBuf::from(v).join("Library/Application Support"))
    } else if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|v| PathBuf::from(v).join(".config")))
    }?;
    let directory = base.join("flutter-network-viewer-rust");
    if let Err(e) = std::fs::create_dir_all(&directory) {
        eprintln!("Settings directory: {e}");
        return None;
    }
    Some(directory.join("settings.json"))
}
fn main() -> eframe::Result {
    eframe::run_native(
        "Flutter Network Viewer",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1360., 860.])
                .with_min_inner_size([480., 360.]),
            ..Default::default()
        },
        Box::new(|cc| {
            cc.egui_ctx.set_visuals(egui::Visuals::light());
            let settings = settings_path()
                .and_then(|p| AppSettings::load(p).map_err(|e| eprintln!("{e}")).ok())
                .unwrap_or_default();
            Ok(Box::new(DesktopApp::new(settings)))
        }),
    )
}
