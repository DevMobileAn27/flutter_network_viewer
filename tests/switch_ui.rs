use eframe::egui::{self, Event, Modifiers};
use flutter_network_viewer::vintage::toggle;

fn draw(
    ctx: &egui::Context,
    value: &mut bool,
    enabled: bool,
    events: Vec<Event>,
    time: f64,
) -> (egui::FullOutput, egui::Response) {
    let mut response = None;
    let output = ctx.run(
        egui::RawInput {
            time: Some(time),
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(200., 100.),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                response = Some(ui.add_enabled_ui(enabled, |ui| toggle(ui, value)).inner);
            });
        },
    );
    (output, response.unwrap())
}

#[test]
fn switch_keyboard_focus_disabled_and_dpi_use_the_same_vector_control() {
    for dark in [false, true] {
        for dpi in [1., 2.] {
            let ctx = egui::Context::default();
            ctx.set_visuals(if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            ctx.set_pixels_per_point(dpi);
            let mut value = false;
            let (_, response) = draw(&ctx, &mut value, true, vec![], 0.);
            assert_eq!(response.rect.size(), egui::vec2(44., 22.));
            response.request_focus();
            let press = || {
                vec![Event::Key {
                    key: egui::Key::Space,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Modifiers::NONE,
                }]
            };
            let (_, response) = draw(&ctx, &mut value, true, press(), 0.2);
            assert!(
                value && response.changed(),
                "Space must toggle the focused switch"
            );
            let (output, response) = draw(&ctx, &mut value, true, vec![], 0.4);
            assert!(response.has_focus());
            assert!(
                output
                    .shapes
                    .iter()
                    .any(|s| matches!(&s.shape, egui::Shape::Rect(r)
                if r.rect == response.rect.expand(2.) && r.stroke.width > 0.))
            );
            let (_, disabled) = draw(&ctx, &mut value, false, vec![], 0.6);
            let (_, disabled) = draw(
                &ctx,
                &mut value,
                false,
                vec![
                    Event::PointerMoved(disabled.rect.center()),
                    Event::PointerButton {
                        pos: disabled.rect.center(),
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: Modifiers::NONE,
                    },
                    Event::PointerButton {
                        pos: disabled.rect.center(),
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: Modifiers::NONE,
                    },
                ],
                0.8,
            );
            assert!(value && !disabled.changed() && !disabled.enabled());
            assert_eq!(disabled.rect.size(), egui::vec2(44., 22.));
        }
    }
}
