use eframe::egui::{self, Event};
use flutter_network_viewer::{desktop_host::DesktopApp, settings::AppSettings};
use serde_json::json;
struct Capture {
    app: DesktopApp,
    output: std::path::PathBuf,
    frames: usize,
}
impl eframe::App for Capture {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.app.update(ctx, frame);
        self.frames += 1;
        if self.frames == 12 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        for event in ctx.input(|i| i.events.clone()) {
            if let Event::Screenshot { image, .. } = event {
                image::save_buffer(
                    self.output.join("screen.png"),
                    image.as_raw(),
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                let probes = self
                    .app
                    .ui
                    .probe
                    .widgets
                    .iter()
                    .map(|(key, rect)| {
                        (
                            key.clone(),
                            json!([rect.left(), rect.top(), rect.right(), rect.bottom()]),
                        )
                    })
                    .collect::<serde_json::Map<_, _>>();
                std::fs::write(
                    self.output.join("geometry.json"),
                    serde_json::to_vec_pretty(&probes).unwrap(),
                )
                .unwrap();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        assert!(self.frames < 200, "Screenshot did not arrive");
        ctx.request_repaint();
    }
}
fn main() -> eframe::Result {
    let output = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mode = std::env::args().nth(2).unwrap_or_default();
    let settings = AppSettings::from_preferences(
        json!({"autoDetectVmService":false,"themeMode":if mode.contains("dark"){"dark"}else{"light"}}),
    );
    let mut app = DesktopApp::with_diagnostics(settings, output.join("diagnostics"));
    if mode.contains("captured") {
        app.dispatch(
            flutter_network_viewer::desktop::Action::Connect(
                std::env::args().nth(3).expect("VM URL"),
            ),
            0,
        );
    }

    eframe::run_native(
        "Flutter Network Viewer",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1360., 808.]),
            ..Default::default()
        },
        Box::new(move |_| {
            Ok(Box::new(Capture {
                app,
                output,
                frames: 0,
            }))
        }),
    )
}
