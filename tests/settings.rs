use flutter_network_viewer::settings::{AppSettings, SettingsError, ThemeMode};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "fnv-settings-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self) -> PathBuf {
        self.0.join("settings.json")
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn defaults_are_light_english_and_mcp_opt_in() {
    let s = AppSettings::default();
    let v = s.values();
    assert_eq!(v.theme_mode, ThemeMode::Light);
    assert_eq!(v.language_code, "en");
    assert!(!v.mcp_enabled);
    assert_eq!(v.mcp_port, 38943);
    assert_eq!(v.network_inspector_width, None);
    assert_eq!(v.console_inspector_width, 400.0);
    assert_eq!(v.json_input_fraction, 0.5);
    assert_eq!(v.request_detail_tab, 0);
    assert_eq!(v.network_column_widths, [64.0, 56.0, 108.0]);
    assert_eq!(v.vm_service_uri, "");
    assert!(!v.auto_detect_vm_service);
}

#[test]
fn language_round_trip_and_unknown_locale_fallback() {
    let t = Temp::new();
    fs::write(t.file(), r#"{"languageCode":"fr"}"#).unwrap();
    let mut s = AppSettings::load(t.file()).unwrap();
    assert_eq!(s.values().language_code, "en");
    for code in ["vi", "en"] {
        s.set_language(code).unwrap();
        assert_eq!(
            AppSettings::load(t.file()).unwrap().values().language_code,
            code
        );
    }
    for code in ["fr", "VI", "en-US", ""] {
        assert!(s.set_language(code).is_err());
    }
    assert_eq!(s.values().language_code, "en");
}

#[test]
fn theme_round_trip_and_system_is_rejected() {
    let t = Temp::new();
    let mut s = AppSettings::load(t.file()).unwrap();
    for theme in [ThemeMode::Dark, ThemeMode::Light] {
        s.set_theme_mode(theme).unwrap();
        assert_eq!(
            AppSettings::load(t.file()).unwrap().values().theme_mode,
            theme
        );
    }
    assert!(s.set_theme_mode(ThemeMode::System).is_err());
    assert_eq!(s.values().theme_mode, ThemeMode::Light);
    for mode in ["system", "DARK", "unknown"] {
        assert_eq!(
            AppSettings::from_preferences(json!({"themeMode": mode}))
                .values()
                .theme_mode,
            ThemeMode::Light
        );
    }
}

#[test]
fn mcp_persists_opt_in_disable_and_port_boundaries() {
    let t = Temp::new();
    let mut s = AppSettings::load(t.file()).unwrap();
    for port in [1, 39941, 65535] {
        s.set_mcp_port(port).unwrap();
        s.set_mcp_enabled(true).unwrap();
        let restored = AppSettings::load(t.file()).unwrap();
        assert!(restored.values().mcp_enabled);
        assert_eq!(restored.values().mcp_port, port);
    }
    s.set_mcp_enabled(false).unwrap();
    let restored = AppSettings::load(t.file()).unwrap();
    assert!(!restored.values().mcp_enabled);
    assert_eq!(restored.values().mcp_port, 65535);
}

#[test]
fn invalid_ports_do_not_mutate_memory_or_file() {
    let t = Temp::new();
    let mut s = AppSettings::load(t.file()).unwrap();
    s.set_mcp_port(40000).unwrap();
    let before = fs::read(t.file()).unwrap();
    for port in [i64::MIN, -1, 0, 65536, i64::MAX] {
        assert!(matches!(
            s.set_mcp_port(port),
            Err(SettingsError::InvalidValue(_))
        ));
        assert_eq!(s.values().mcp_port, 40000);
        assert_eq!(fs::read(t.file()).unwrap(), before);
    }
    for port in [
        json!(0),
        json!(65536),
        json!("38943"),
        json!(1.5),
        json!(null),
    ] {
        assert_eq!(
            AppSettings::from_preferences(json!({"mcpPort":port}))
                .values()
                .mcp_port,
            38943
        );
    }
}

#[test]
fn uri_and_discovery_round_trip_preserve_auth_and_unicode() {
    let t = Temp::new();
    let mut s = AppSettings::load(t.file()).unwrap();
    let uri = "ws://127.0.0.1:8181/a%2Bb=/ws?name=Tiếng Việt";
    s.set_manual_vm_service_uri(uri).unwrap();
    let r = AppSettings::load(t.file()).unwrap();
    assert_eq!(r.values().vm_service_uri, uri);
    assert!(!r.values().auto_detect_vm_service);
    s.set_auto_detect_vm_service(true).unwrap();
    let r = AppSettings::load(t.file()).unwrap();
    assert!(r.values().auto_detect_vm_service);
    assert_eq!(r.values().vm_service_uri, uri);
}

#[test]
fn invalid_saved_layouts_fall_back_independently() {
    let s = AppSettings::from_preferences(
        json!({"networkInspectorWidth":319.9,"consoleInspectorWidth":-100,"jsonInputFraction":1.5,"requestDetailTab":4,"languageCode":"vi"}),
    );
    assert_eq!(s.values().network_inspector_width, None);
    assert_eq!(s.values().console_inspector_width, 400.0);
    assert_eq!(s.values().json_input_fraction, 0.5);
    assert_eq!(s.values().request_detail_tab, 0);
    assert_eq!(s.values().language_code, "vi");
}

#[test]
fn legacy_four_tabs_migrate_without_reinterpreting_invalid_indexes() {
    for (old, expected) in [(0, 0), (1, 2), (2, 4), (3, 5), (-1, 0), (4, 0), (999, 0)] {
        assert_eq!(
            AppSettings::from_preferences(json!({"requestDetailTab":old}))
                .values()
                .request_detail_tab,
            expected
        );
    }
}

#[test]
fn named_tab_takes_precedence_and_all_six_persist_by_name() {
    let t = Temp::new();
    let names = [
        "overview",
        "requestHeader",
        "requestBody",
        "responseHeader",
        "responseBody",
        "raw",
    ];
    fs::write(
        t.file(),
        r#"{"requestDetailTab":1,"requestDetailTabName":"bogus"}"#,
    )
    .unwrap();
    let mut s = AppSettings::load(t.file()).unwrap();
    assert_eq!(s.values().request_detail_tab, 0);
    for i in [1, 2, 3, 4, 5, 0] {
        s.set_request_detail_tab(i).unwrap();
        assert_eq!(
            AppSettings::load(t.file())
                .unwrap()
                .values()
                .request_detail_tab,
            i
        );
        let raw: serde_json::Value = serde_json::from_slice(&fs::read(t.file()).unwrap()).unwrap();
        assert_eq!(raw["requestDetailTabName"], names[i as usize]);
    }
    for i in [-1, 6, i64::MAX] {
        assert!(s.set_request_detail_tab(i).is_err());
    }
    assert_eq!(s.values().request_detail_tab, 0);
}

#[test]
fn independent_split_and_column_values_survive_real_file_restart() {
    let t = Temp::new();
    let mut s = AppSettings::load(t.file()).unwrap();
    s.set_inspector_width(720.0, false).unwrap();
    s.set_inspector_width(450.0, true).unwrap();
    s.set_json_input_fraction(0.63).unwrap();
    s.set_network_column_widths(&[320.0, 180.0, 200.0]).unwrap();
    assert_eq!(AppSettings::load(t.file()).unwrap().values(), s.values());
    s.set_inspector_width(320.0, true).unwrap();
    s.set_json_input_fraction(0.5).unwrap();
    assert_eq!(s.values().network_inspector_width, Some(720.0));
    assert_eq!(s.values().network_column_widths, [320.0, 180.0, 200.0]);
}

#[test]
fn layout_validation_rejects_nonfinite_and_out_of_range_without_mutation() {
    let mut s = AppSettings::default();
    let before = s.values().clone();
    for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0, 319.999] {
        for console in [false, true] {
            assert!(s.set_inspector_width(x, console).is_err());
        }
    }
    for x in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1.0,
        0.0,
        1.0,
        2.0,
    ] {
        assert!(s.set_json_input_fraction(x).is_err());
    }
    assert_eq!(s.values(), &before);
    s.set_inspector_width(100000.0, false).unwrap(); // Source has no upper bound.
    s.set_json_input_fraction(0.00001).unwrap();
    s.set_json_input_fraction(0.99999).unwrap();
}

#[test]
fn invalid_persisted_column_layouts_fall_back() {
    for saved in [
        "bad json",
        "null",
        "[80,90]",
        "[-1,90,100]",
        "[88,\"bad\",108]",
        "[9000,90,108]",
        "[64,56,95]",
        "[1e999,56,108]",
    ] {
        assert_eq!(
            AppSettings::from_preferences(json!({"networkColumnWidths":saved}))
                .values()
                .network_column_widths,
            [64.0, 56.0, 108.0]
        );
    }
}

#[test]
fn column_validation_and_copy_ownership() {
    let mut s = AppSettings::default();
    for widths in [
        vec![],
        vec![64.0, 56.0],
        vec![64.0, 56.0, 108.0, 1.0],
        vec![63.0, 56.0, 108.0],
        vec![64.0, 55.0, 108.0],
        vec![64.0, 56.0, 95.0],
        vec![1601.0, 56.0, 108.0],
        vec![f64::NAN, 56.0, 108.0],
        vec![64.0, f64::INFINITY, 108.0],
    ] {
        assert!(s.set_network_column_widths(&widths).is_err());
        assert_eq!(s.values().network_column_widths, [64.0, 56.0, 108.0]);
    }
    let mut widths = [1600.0, 1600.0, 1600.0];
    s.set_network_column_widths(&widths).unwrap();
    widths[0] = 0.0;
    assert_eq!(widths[0], 0.0);
    assert_eq!(s.values().network_column_widths[0], 1600.0);
    s.set_network_column_widths(&[64.0, 56.0, 96.0]).unwrap();
}

#[test]
fn corrupt_missing_and_wrong_typed_settings_restore_defaults() {
    let t = Temp::new();
    assert_eq!(
        AppSettings::load(t.file()).unwrap().values(),
        AppSettings::default().values()
    );
    assert!(!t.file().exists());
    for bytes in [b"{".as_slice(), b"null", b"[]", b"42", b"\xff\xfe"] {
        fs::write(t.file(), bytes).unwrap();
        assert_eq!(
            AppSettings::load(t.file()).unwrap().values(),
            AppSettings::default().values()
        );
        assert_eq!(fs::read(t.file()).unwrap(), bytes); // Reading doesn't clobber corrupt input.
    }
    let s = AppSettings::from_preferences(
        json!({"languageCode":true,"themeMode":2,"mcpEnabled":"true","mcpPort":[],"networkInspectorWidth":"500","consoleInspectorWidth":false,"jsonInputFraction":[],"networkColumnWidths":[64,56,108],"vmServiceUri":false,"autoDetectVmService":"false"}),
    );
    assert_eq!(s.values(), AppSettings::default().values());
}

#[test]
fn failed_preference_writes_keep_committed_state_and_return_io_error() {
    let t = Temp::new();
    let mut s = AppSettings::load(t.file()).unwrap();
    fs::create_dir(t.file()).unwrap(); // Deterministic rename failure, even as privileged user.
    assert!(matches!(
        s.set_theme_mode(ThemeMode::Dark),
        Err(SettingsError::Io(_))
    ));
    assert!(s.set_language("vi").is_err());
    assert!(s.set_mcp_port(40000).is_err());
    assert!(s.set_mcp_enabled(true).is_err());
    assert!(
        s.set_manual_vm_service_uri("ws://localhost:123/ws")
            .is_err()
    );
    assert!(s.set_auto_detect_vm_service(true).is_err());
    assert_eq!(s.values(), AppSettings::default().values());
    assert_eq!(fs::read_dir(&t.0).unwrap().count(), 1); // No abandoned temp files.
    fs::remove_dir(t.file()).unwrap();
    s.set_language("vi").unwrap();
    assert_eq!(
        AppSettings::load(t.file()).unwrap().values().language_code,
        "vi"
    );
}

#[test]
fn layout_stays_current_in_memory_when_disk_write_fails() {
    let t = Temp::new();
    let mut s = AppSettings::load(t.file()).unwrap();
    fs::create_dir(t.file()).unwrap();
    assert!(s.set_inspector_width(800.0, false).is_err());
    assert!(s.set_inspector_width(500.0, true).is_err());
    assert!(s.set_json_input_fraction(0.7).is_err());
    assert!(s.set_request_detail_tab(4).is_err());
    assert!(s.set_network_column_widths(&[100.0, 90.0, 160.0]).is_err());
    assert_eq!(s.values().network_inspector_width, Some(800.0));
    assert_eq!(s.values().console_inspector_width, 500.0);
    assert_eq!(s.values().json_input_fraction, 0.7);
    assert_eq!(s.values().request_detail_tab, 4);
    assert_eq!(s.values().network_column_widths, [100.0, 90.0, 160.0]);
}

#[test]
fn nonmissing_read_errors_and_missing_parent_writes_are_reported() {
    let t = Temp::new();
    fs::create_dir(t.file()).unwrap();
    assert!(matches!(
        AppSettings::load(t.file()),
        Err(SettingsError::Io(_))
    ));
    let mut s = AppSettings::load(t.0.join("absent/settings.json")).unwrap();
    assert!(matches!(s.set_language("vi"), Err(SettingsError::Io(_))));
    assert_eq!(s.values().language_code, "en");
}

#[test]
fn unknown_preferences_survive_writes_and_file_contains_no_capture_data() {
    let t = Temp::new();
    fs::write(
        t.file(),
        r#"{"futurePreference":{"enabled":true},"requestDetailTab":2}"#,
    )
    .unwrap();
    let mut s = AppSettings::load(t.file()).unwrap();
    s.set_language("vi").unwrap();
    let raw: serde_json::Value = serde_json::from_slice(&fs::read(t.file()).unwrap()).unwrap();
    assert_eq!(raw["futurePreference"], json!({"enabled":true}));
    assert!(raw.get("captures").is_none());
    assert_eq!(
        AppSettings::load(t.file())
            .unwrap()
            .values()
            .request_detail_tab,
        4
    );
}

#[test]
fn font_size_defaults_validates_and_survives_restart() {
    let t = Temp::new();
    let mut s = AppSettings::load(t.file()).unwrap();
    assert_eq!(s.values().font_size, 14.);
    for size in [10., 15., 24., 14.] {
        s.set_font_size(size).unwrap();
        assert_eq!(
            AppSettings::load(t.file()).unwrap().values().font_size,
            size
        );
    }
    for size in [9., 25., f32::NAN, f32::INFINITY] {
        assert!(s.set_font_size(size).is_err());
        assert_eq!(s.values().font_size, 14.);
    }
    for value in [json!(0), json!(100), json!("16"), serde_json::Value::Null] {
        assert_eq!(
            AppSettings::from_preferences(json!({"fontSize":value}))
                .values()
                .font_size,
            14.
        );
    }
}
