//! Capture the real desktop renderer with isolated, synthetic data for the README.
use base64::{Engine, engine::general_purpose::STANDARD};
use eframe::egui::{self, Event, Modifiers, PointerButton};
use flutter_network_viewer::{
    capture::CaptureStore,
    desktop::{Action, DesktopUi, ShellStatus},
    settings::AppSettings,
};
use serde_json::{Value, json};
use std::{collections::VecDeque, path::PathBuf, time::Instant};

enum Step {
    Reset(bool),
    Click(&'static str),
    Text(&'static str),
    Snap(&'static str),
}
use Step::*;

const URL: &str = "http://127.0.0.1:43210/demo=/";
const DRAFT: &str = r##"{"store":"Demo Shop","order":{"id":42,"status":"created","items":[{"name":"Coffee","quantity":2},{"name":"Tea","quantity":1}]},"theme":{"accent":"#2563EB"}}"##;
const ORDER: &str = "request:ios-session-1:fetch-1";
const PHOTO: &str = "request:ios-session-1:demo-image";

fn script() -> VecDeque<Step> {
    [
        Reset(false),
        Snap("connection-1"),
        Click("manual-url"),
        Click("vm-uri"),
        Text(URL),
        Snap("connection-2"),
        Click("connect-submit"),
        Snap("connection-3"),
        Reset(true),
        Snap("network-1"),
        Click(ORDER),
        Snap("network-2"),
        Click("tab:4"),
        Snap("network-3"),
        Reset(true),
        Click(PHOTO),
        Click("tab:4"),
        Snap("image-preview-1"),
        Click("image-open"),
        Snap("image-preview-2"),
        Reset(true),
        Click("Console"),
        Snap("console-tags-1"),
        Click("console-message-filter"),
        Click("message-tag-input"),
        Text("order"),
        Snap("console-tags-2"),
        Click("message-tag-add"),
        Click("message-tag-input"),
        Text("payment"),
        Click("message-tag-add"),
        Snap("console-tags-3"),
        Click("console-message-filter"),
        Snap("console-tags-4"),
        Reset(true),
        Click("Console"),
        Snap("console-detail-1"),
        Click("demo-log"),
        Snap("console-detail-2"),
        Click("console-raw"),
        Snap("console-detail-3"),
        Reset(true),
        Click("JSON"),
        Click("json-input"),
        Text(DRAFT),
        Snap("json-beautify-1"),
        Click("json-format"),
        Snap("json-beautify-2"),
        Click("json-tree"),
        Snap("json-beautify-3"),
        Click("json-code"),
        Snap("json-search-1"),
        Click("json-workspace-find"),
        Text("Coffee"),
        Snap("json-search-2"),
        Click("json-tree"),
        Snap("json-search-3"),
        Reset(true),
        Snap("pause-resume-1"),
        Click("pause"),
        Snap("pause-resume-2"),
        Click("pause"),
        Snap("pause-resume-3"),
        Reset(true),
        Snap("export-1"),
        Click("export"),
        Snap("export-2"),
        Reset(true),
        Snap("clear-1"),
        Click("clear"),
        Snap("clear-2"),
        Reset(true),
        Click("settings"),
        Snap("settings-mcp-1"),
        Click("mcp-enabled"),
        Snap("settings-mcp-2"),
    ]
    .into()
}

fn demo(path: &std::path::Path) -> Value {
    let mut demo: Value =
        serde_json::from_str(include_str!("../tests/fixtures/sdk_envelope.json")).unwrap();
    demo["session"]["appName"] = json!("Demo Shop");
    let events = demo["events"].as_array_mut().unwrap();
    events.retain(|e| e["source"] != "js.console");
    for (i, (source, level, message)) in [
        ("dart.stdout", "log", "App started"),
        ("dart.logging", "info", "Order #42 created"),
        ("dart.stdout", "log", "Refreshing product catalog"),
        ("dart.logging", "info", "Order #42 payment confirmed"),
        ("dart.stderr", "error", "Profile request timed out"),
    ]
    .into_iter()
    .enumerate()
    {
        events.push(json!({
            "type":"console","source":source,"level":level,"message":message,"loggerName":"demo.shop",
            "timestamp":format!("2026-10-14T08:00:0{i}Z")
        }));
    }
    let body = STANDARD.encode(std::fs::read(path).expect("synthetic product image"));
    events.extend([
        json!({"requestId":"demo-image","phase":"request","request":{"method":"GET","url":"https://api.example.com/products/coffee.png"}}),
        json!({"requestId":"demo-image","phase":"response","durationMs":86,"response":{
            "statusCode":200,"headers":{"content-type":"image/png"},"body":body,"bodyEncoding":"base64"
        }}),
    ]);
    demo
}

struct Recorder {
    steps: VecDeque<Step>,
    events: VecDeque<Vec<Event>>,
    output: PathBuf,
    demo: Value,
    ui: DesktopUi,
    store: CaptureStore,
    status: ShellStatus,
    pending: Option<&'static str>,
    settle: usize,
    deadline: Instant,
}

impl Recorder {
    fn load_demo(&mut self) {
        let session = self.store.ingest(&self.demo).unwrap();
        self.status.online.insert(session);
    }
}

impl eframe::App for Recorder {
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        if let Some(events) = self.events.pop_front() {
            input.events = events;
        }
    }

    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        assert!(self.deadline.elapsed().as_secs() < 65, "capture timed out");
        for action in self.ui.show(ctx, &self.store, &self.status) {
            match action {
                Action::Connect(_) => self.load_demo(),
                Action::SetPaused(paused) => self.store.paused = paused,
                Action::Clear => self.store.clear(),
                _ => (),
            }
        }
        if let Some(id) = self.ui.probe.logs.get(1)
            && let Some(rect) = self.ui.probe.widgets.get(&format!("log:{id}")).copied()
        {
            self.ui.probe.widgets.insert("demo-log".into(), rect);
        }
        for event in ctx.input(|i| i.events.clone()) {
            if let Event::Screenshot { image, .. } = event
                && let Some(name) = self.pending.take()
            {
                image::save_buffer(
                    self.output.join(format!("{name}.png")),
                    image.as_raw(),
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                let widgets: serde_json::Map<_, _> = self
                    .ui
                    .probe
                    .widgets
                    .iter()
                    .map(|(k, r)| (k.clone(), json!([r.left(), r.top(), r.right(), r.bottom()])))
                    .collect();
                let size = ctx.content_rect().size();
                std::fs::write(
                    self.output.join(format!("{name}.json")),
                    serde_json::to_vec(&json!({"size":[size.x,size.y],"widgets":widgets})).unwrap(),
                )
                .unwrap();
                println!("Captured {name}");
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(12));
        if !self.events.is_empty() || self.pending.is_some() {
            return;
        }
        if self.settle > 0 {
            self.settle -= 1;
            return;
        }
        match self.steps.pop_front() {
            Some(Reset(loaded)) => {
                let mut settings = AppSettings::from_preferences(json!({"themeMode":"light"}));
                settings.set_language("en").unwrap();
                self.ui = DesktopUi::new(settings);
                self.store = CaptureStore::default();
                self.status = ShellStatus::default();
                if loaded {
                    self.load_demo();
                }
            }
            Some(Click(key)) => {
                if key == "tab:4" {
                    self.ui.reveal_inspector_tab = Some(4);
                }
                let Some(rect) = self.ui.probe.widgets.get(key).copied() else {
                    self.steps.push_front(Click(key));
                    return;
                };
                let pos = rect.center();
                let button = |pressed| Event::PointerButton {
                    pos,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                };
                self.events.push_back(vec![Event::PointerMoved(pos)]);
                self.events.push_back(vec![button(true)]);
                self.events.push_back(vec![button(false)]);
                self.events.push_back(vec![Event::PointerGone]);
            }
            Some(Text(text)) => self.events.push_back(vec![Event::Text(text.into())]),
            Some(Snap(name)) => {
                self.pending = Some(name);
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            None => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
        }
        self.settle = 5;
    }
}

fn main() -> eframe::Result {
    let mut args = std::env::args_os().skip(1);
    let output = PathBuf::from(args.next().expect("screenshot output directory"));
    let demo = demo(&PathBuf::from(
        args.next().expect("synthetic product image PNG"),
    ));
    std::fs::create_dir_all(&output).unwrap();
    eframe::run_native(
        "Flutter Network Viewer · Demo",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1350., 720.]),
            ..Default::default()
        },
        Box::new(move |_| {
            Ok(Box::new(Recorder {
                steps: script(),
                events: Default::default(),
                output,
                demo,
                ui: DesktopUi::default(),
                store: CaptureStore::default(),
                status: ShellStatus::default(),
                pending: None,
                settle: 8,
                deadline: Instant::now(),
            }))
        }),
    )
}
