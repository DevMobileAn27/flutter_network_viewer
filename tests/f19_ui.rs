use eframe::egui::{self, Event, Modifiers, PointerButton};
use flutter_network_viewer::{
    capture::CaptureStore,
    desktop::{DesktopUi, ShellStatus},
};
#[test]
fn ram_status_renders_changes_and_preserves_input_and_capture_caches() {
    for (size, lang) in [
        (egui::vec2(1200., 800.), "en"),
        (egui::vec2(360., 300.), "vi"),
    ] {
        let ctx = egui::Context::default();
        let mut ui = DesktopUi::default();
        ui.settings.set_language(lang).unwrap();
        let mut store = CaptureStore::default();
        store
            .ingest(&serde_json::from_str(include_str!("fixtures/sdk_envelope.json")).unwrap())
            .unwrap();
        ui.selected_request = Some(("ios-session-1".into(), "fetch-1".into()));
        ui.settings.set_request_detail_tab(2).unwrap();
        let status = ShellStatus::default();
        let render = |ui: &mut DesktopUi, events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    events,
                    ..Default::default()
                },
                |c| {
                    ui.show(c, &store, &status);
                },
            )
        };
        render(&mut ui, vec![]);
        render(&mut ui, vec![]);
        let preparations = ui.detail_preparations;
        let bodies = ui.body_preparations;
        let encodes = ui.network_raw_serializations;
        for bytes in [None, Some(171_000_000), None, Some(256_000_000)] {
            ui.memory_bytes = bytes;
            let output = render(&mut ui, vec![]);
            render(&mut ui, vec![]);
            let expected = bytes.map_or("RAM —".into(), |b| {
                format!("RAM {:.0} MB", b as f64 / 1e6)
            });
            assert_eq!(ui.probe.resource_text, expected);
            assert!(
                output
                    .shapes
                    .iter()
                    .any(|s| matches!(&s.shape,egui::Shape::Text(t) if t.galley.text()==expected))
            );
            let r = ui.probe.widgets["resource-usage"];
            assert!(egui::Rect::from_min_size(egui::Pos2::ZERO, size).contains_rect(r));
            assert_eq!(ui.detail_preparations, preparations);
            assert_eq!(ui.body_preparations, bodies);
            assert_eq!(ui.network_raw_serializations, encodes);
        }
        // Narrow request inspector is modal; dismiss it through real keyboard
        // input before exercising the navigation behind it.
        render(
            &mut ui,
            vec![Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
        render(
            &mut ui,
            vec![Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
        );
        for _ in 0..4 {
            render(&mut ui, vec![]);
        }
        let p = ui.probe.widgets["settings"].center();
        render(&mut ui, vec![Event::PointerMoved(p)]);
        for pressed in [true, false] {
            render(
                &mut ui,
                vec![Event::PointerButton {
                    pos: p,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                }],
            );
        }
        assert!(ui.support.settings_open);
    }
}
