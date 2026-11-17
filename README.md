# Flutter Network Viewer — Rust

Port từ `/Users/quocan/Github/flutter_network_viewer` (Flutter 0.1.8+9).
Desktop Rust dùng egui/eframe và OpenGL, không cần Flutter runtime.
F01–F12 core, F13 shell/Network, F14 Console/inspector, F15 JSON/beautify, F16 body previews/performance, F17 help/settings/notifications F18 MCP read-only HTTP F19 diagnostics/RAM và F20 updater đã triển khai. Đã chạy native trên macOS ARM64;
F21 đã build/package macOS ARM64 và Intel (startup Intel qua Rosetta); Windows cross-check/clippy pass nhưng linked/native còn bị chặn bởi MSVC. Acceptance toàn app vẫn partial.

Cửa sổ có app tabs All/từng app, Network/Console/JSON navigation, discovery picker,
manual VM connect, Pause/Clear, URL/method/status search và filters, bảng Network
cuộn hai chiều với ba cột kéo/lưu độ rộng, inspector sáu tab, Copy cURL/detail và
export JSON thật. Inspector là split pane khi rộng, modal khi hẹp; pending request
cập nhật từ store hiện tại. Console có source/level/search kết hợp app, nhóm JSON/exception, detail/raw/copy
và split riêng được lưu; raw chỉ encode khi mở và cache theo record. JSON beautify có
Format/Ctrl+Enter, JSON/Tree, fold, Find/Ctrl+F/F3, highlight, swatch màu và copy;
giữ draft/query/kết quả qua đổi trang và resize. Body JSON dùng cùng viewer.
Form/Image/Raw preview có live update, ảnh phóng to dùng chung texture, zoom/pan/Fit và fallback. Network/Console/JSON chỉ dựng hàng trong viewport. Settings có VI/EN, light/dark, discovery và URL thủ công; help riêng và thông báo bốn loại, clipboard native. Các phần này chưa được coi là UI parity toàn app.

## Chạy và kiểm thử

Các đường dẫn trong `docs/artifacts/` trỏ tới log, ảnh và gói build cục bộ do các lệnh kiểm thử hoặc đóng gói tạo ra. Git không lưu các đầu ra này.

Máy hiện tại dùng toolchain tạm, không sửa PATH hệ thống:

```sh
export CARGO_HOME=/private/tmp/fnv-cargo
export RUSTUP_HOME=/private/tmp/fnv-rustup
export PATH="$CARGO_HOME/bin:$PATH"
cargo run --locked --offline
```

Nếu toolchain/cache tạm bị xóa, cần khôi phục trước khi chạy. App lưu settings ở
`~/Library/Application Support/flutter-network-viewer-rust/settings.json` trên
macOS hoặc `%APPDATA%/flutter-network-viewer-rust/settings.json` trên Windows.
Capture/body chỉ giữ trong RAM. Export ghi toàn bộ app và record vào đường dẫn
người dùng nhập trong dialog; lỗi I/O được hiển thị, không báo export thành công.

```sh
cargo test --locked --offline
cargo fmt --check
cargo clippy --all-targets --locked --offline -- -D warnings
python3 tools/f13_native_smoke.py
python3 tools/f14_native_smoke.py
python3 tools/f15_native_smoke.py
python3 tools/f16_native_smoke.py
python3 tools/f17_native_smoke.py
python3 tools/f18_native_smoke.py
cargo run --locked --offline --example f16_stress -- docs/artifacts/f16/stress-measurement.json
```

Test thường: 270 pass (213 core/HTTP + 57 UI/renderer tests), không ignored. UI tests gửi
pointer/key/wheel vào chính renderer sản phẩm; kiểm tra popup/modal, sáu tab,
live update, app filters, scrolling, file export và persisted column layout.
Native smoke là executable riêng: mở cửa sổ thật, dùng production DesktopApp +
WebSocket fake VM, kiểm tra pending→201, OS clipboard đọc lại cURL/detail,
Console, Pause/Resume, export/Clear, rồi tự đóng. Cần GUI session và quyền loopback;
trong Codex sandbox cần escalation cho socket. Smoke không thay live Dart SDK.
Bằng chứng macOS: [log](docs/artifacts/f13/native-smoke.log),
[screenshot](docs/artifacts/f13/native-network.png), [export](docs/artifacts/f13/native-export.json).

F14 smoke kiểm tra Stdout/Stderr/Logging gửi tại streamListen trước bootstrap,
nhóm JSON/exception, bộ lọc phối hợp, detail/raw, OS clipboard, modal hẹp/Escape
và Clear. [Log F14](docs/artifacts/f14/native-smoke.log),
[screenshot Console](docs/artifacts/f14/native-console.png),
[regression](docs/artifacts/f14/regression.log). Early notifications được giữ trong
hàng đợi có giới hạn; Clear/Pause/Stop xóa hàng đợi này. Không thay đổi F13 artifacts.

F15 smoke dùng host thật và fresh WebSocket fixture; kiểm tra beautify, lỗi dòng/cột,
giữ draft/fold/find qua đổi tab/resize, swatch màu, cuộn ngang/dọc, Ctrl+A/Copy
toàn document không gutter và request body JSON. [Log F15](docs/artifacts/f15/native-smoke.log),
[beautify](docs/artifacts/f15/native-beautify.png), [body Tree](docs/artifacts/f15/native-json.png),
[regression 222 pass](docs/artifacts/f15/regression-rerun.log).
Một full run vẫn gặp bootstrap timeout trong coordinator_socket cũ; targeted và
full rerun pass không thay timeout/fixture. Xem progress để biết giới hạn kiểm thử.

## Phạm vi và cấu trúc

- [Ma trận tính năng và bài kiểm thử toàn app](docs/feature-matrix.md).
- [Danh mục test Flutter gốc và mapping Rust](docs/flutter-test-catalog.json).
- [Trạng thái và handoff](docs/progress.md).
- [Hash source Flutter](docs/source-snapshot.json).

`src/desktop.rs` render shell/inspectors, `src/desktop_json.rs` render JSON; `src/desktop_host.rs::DesktopApp` gọi
`AppDiscovery<ThreadConnector, ThreadDiscovery>::tick(now_ms)` mỗi frame và dispatch
actions. Workers chạy ngoài UI thread. `src/main.rs` chọn đường dẫn settings và
mở cửa sổ. Các inspector cache theo revision record đang chọn, Raw encode khi mở; Network/Console/JSON chỉ dựng viewport. Console append chỉ regroup tail có giới hạn. Settings layout, locale/theme, discovery và URL thủ công nối AppSettings F08.
`src/desktop_support.rs` quản lý help/settings, localization và notification sessions.
Font Cascadia Mono được bundle kèm OFL để hiển thị đầy đủ dấu tiếng Việt.

F12 scan thêm tối đa một app; Stop scan giữ app cũ, manual connect thay tất cả app.
Các app dùng chung capture budget/history, Pause/Clear/cursors toàn cục. F11
reconnect/bootstrap và worker stream đã có fake WebSocket tests; live SDK, Windows,
WSS certificate và combined discovery→multi-app real socket acceptance còn pending.
System DNS/TCP trước khi có socket handle chưa tuyệt đối cancellable/deadline.
Async Logging object/truncated-string resolution vẫn partial. F06 thumbnail nguồn
ảnh vượt 64 MiB dùng ImageIO trên macOS; Windows vẫn giữ decode guard và cần backend thumbnail F21. Updater F20 và local packaging F21 đã làm; release còn các gate bên dưới.
Không publish.

F16 native kiểm tra Form/Raw, live Form/image replacement qua WebSocket thật,
Image/Raw/enlarge/zoom và OS clipboard, JSON 20.002 hàng/find/copy/clear. Xem
[log](docs/artifacts/f16/native-smoke-final.log), [ảnh modal](docs/artifacts/f16/native-body.png),
[JSON viewport](docs/artifacts/f16/native-json.png), [đo native](docs/artifacts/f16/native-measurement.json)
và [stress/RSS](docs/artifacts/f16/stress-measurement.json). Probe text có giới hạn;
Copy vẫn lấy toàn bộ document. Parse/index JSON đầu chạy đồng bộ; số đo cold/warm
được tách riêng trong artifacts. Egui vẽ chung surface nên không có child native
host/IPC cần tạo hoặc fallback. Giới hạn và handoff chi tiết trong progress.

F17 native kiểm tra help riêng, OS clipboard URL Discussion, VI/dark qua lưu và tải
lại settings, help/settings/thông báo cuộn ở 360×300, timeout 3 giây theo đồng hồ
thật, confirmation/error chờ lựa chọn và kết nối VM mới qua Settings.
[Log F17](docs/artifacts/f17/f17-native-final.log),
[kết quả native](docs/artifacts/f17/native-result.json),
[help tiếng Việt](docs/artifacts/f17/help-vi-dark.png),
[settings hẹp](docs/artifacts/f17/settings-narrow.png),
[regression](docs/artifacts/f17/regression-final.log).
Success/info tự đóng sau 3 giây; warning/error/confirmation chờ người dùng. Các
phiên bị thay hoặc drop trả false, không giữ timer nền. API hỗ trợ override delay
và auto-hide; reject delay âm. MCP controls/server đã triển khai F18 trên macOS; chưa tuyên bố toàn app parity.


F18 MCP mặc định tắt; bật trong Settings, cổng mặc định 38943. Server chỉ bind
`127.0.0.1`, endpoint `/mcp`, Host chỉ nhận loopback/localhost và từ chối mọi
Origin. Năm tools read-only: `list_sessions`, `list_requests`, `get_request`,
`list_logs`, `get_capture_summary`; dữ liệu đọc trực tiếp từ store hiện tại,
filter/sort/pagination và body/log chunks có giới hạn. Settings có trạng thái/lỗi,
retry, validate/lưu cổng và Copy Codex/Claude setup (HTTP + bridge stdio Claude).
MCP JSON responses hỗ trợ protocol 2024-11-05, 2025-03-26, 2025-06-18, 2025-11-25;
GET trả 405 vì server không cần luồng notification SSE. HTTP owner poll nonblocking,
16 connection, timeout 5s, request tối đa 1MiB; stop/drop đóng port và socket.
[Regression 259 pass](docs/artifacts/f18/regression-final.log),
[native production HTTP/clipboard](docs/artifacts/f18/native-final-source.log),
[ảnh MCP VI/dark](docs/artifacts/f18/native-final-source/mcp-vi-dark.png).


F19 diagnostics ghi JSONL metadata cố định (UTC, PID, lifecycle/connection state,
pause/clear); API không nhận URL/token, header/body/console hay error text.
Queue tối đa 128 KiB; flush sau 250 ms theo clock owner và khi đóng app; xoay
`viewer.log`, `.1`, `.2`, mỗi file tối đa 512 KiB. Lỗi disk/path bỏ batch và capture
vẫn hoạt động; batch mới phục hồi khi đường dẫn dùng được. Log mặc định ở
`~/Library/Logs/Flutter Network Viewer` (macOS), `%LOCALAPPDATA%/Flutter Network Viewer`
(Windows), `$XDG_STATE_HOME/Flutter Network Viewer` hoặc `~/.local/state/Flutter Network Viewer`
(Linux). Có thể đặt `FNV_DIAGNOSTICS_DIR` để dùng thư mục khác.

Footer RAM dùng native physical footprint trên macOS, working set trên Windows,
VmRSS trên Linux; đơn vị MB thập phân như Flutter. Lấy mẫu ngay khi mở rồi mỗi
2 giây theo owner tick; lỗi hiển thị `RAM —` và lần sau thử lại. Không spawn process,
thread hay timer nền; đóng app dừng lấy mẫu và flush diagnostics. Sampling và I/O
batch chạy trên owner thread; cadence phụ thuộc event loop và log I/O vẫn đồng bộ.
Windows/Linux backends chưa chạy native trong F19.
[Regression F19](docs/artifacts/f19/regression-final.log),
[native F19](docs/artifacts/f19/native-final.log),
[RAM thật](docs/artifacts/f19/native-final/memory.json),
[ảnh VI/dark hẹp](docs/artifacts/f19/native-final/ram-vi-dark-narrow.png).
Chạy native với thư mục artifact mới:
`python3 tools/f19_native_smoke.py docs/artifacts/f19/native-next`.


## F20 — Updater

Production host kiểm tra nền một lần khi chạy; Settings hiển thị version Cargo,
trạng thái, progress và nút kiểm tra/retry bằng EN/VI. Feed tách macOS ARM64,
macOS x86_64 và Windows x64; nền tảng khác không gọi native. Check owner-serialized,
timeout60s cho fetch, hủy/finish cho phép retry, listener được cleanup khi drop.
Sparkle giữ kết quả no-update sau abort callback; Windows background discovery
mở native confirmation một lần. Native frameworks quản lý xác nhận, download,
progress và cài đặt; Rust không gọi silent installer.

macOS dùng Sparkle2.9.6 vendored và dynamic-load từ app bundle Frameworks.
Cần `SUPublicEDKey` Ed25519 và ba key `SUEnableAutomaticChecks`,
`SUAutomaticallyUpdate`, `SUAllowsAutomaticUpdates` đặt false rõ ràng.
`cargo run` standalone không có bundle/framework/trust anchor nên updater báo lỗi
và cho retry. Windows adapter dynamic-load WinSparkle0.8.1 x64 bên cạnh exe,
đặt DSA public key và callbacks/cleanup; chưa build/chạy Windows (F21).
Các runtime, license và SHA256 pin nằm trong `vendor/`; build kiểm tra checksum
bằng `python3 -B tools/verify_updater_vendor.py`. Không có private signing key.

`python3 -B tools/test_f20_integrity.py` dùng verifier Sparkle thật trên inert bytes:
valid pass, unsigned/wrong signature/wrong key/tampered archive fail. Đây là key test
cố định, không phải release key. SHA256 pin bảo vệ runtime vendored; chữ ký archive
update được native Sparkle/WinSparkle xác minh trước khi cho cài đặt.
`python3 -B tools/f20_native_smoke.py [output_dir]` compile source, dựng app bundle
thử nghiệm và chỉ phục vụ appcast loopback; không download hoặc cài installer.
F21 đã tách Rust feed khỏi Flutter: kênh cố định `rust-updates`, tên `rust-appcast-*`,
tag `rust-v0.1.0` và archive `flutter-network-viewer-rust-*`. Kênh chưa được publish;
feed thiếu báo lỗi/retry và không fallback sang artifact Flutter. Windows native và
acceptance toàn app vẫn partial.


## F21 — Build/release và acceptance

Xem [release gates và bằng chứng](docs/release.md). Đã viết test trước tooling:
8 release tests +2 native signature/integrity tests +1 actual signing-helper test pass; 270 Rust tests, fmt và
all-targets clippy pass. Windows x64 cross-check/clippy cũng pass. CI cấu hình ba
platform để test/build và upload artifact; chưa chạy GitHub CI, không có publish job.

Rust `0.1.0`, build `1`, app ID `dev.quocan.flutterNetworkViewer.rust`; bundle chứa
icon, Cascadia Mono/OFL, license/notices và Sparkle nguyên symlink/helpers.
[ARM64 zip](docs/artifacts/f21/package-final/flutter-network-viewer-rust-0.1.0-macos-arm64.zip)
và [Intel zip](docs/artifacts/f21/intel-final/flutter-network-viewer-rust-0.1.0-macos-x86_64.zip)
đã build từ source cuối, kiểm tra SHA256/kiến trúc/layout và startup; Intel qua Rosetta.
Đây là preview chưa Developer ID signed/notarized và chưa đủ release gates.

Đã chạy native full flow fake VM→filter→inspect→pause→MCP→export→Clear,
DDS→hai app sockets/console, live Dart SDK HTTP/body tiếng Việt/stdout/developer.log,
F13–F20 native regressions và bốn vòng stress. Font Việt có glyph thật và ảnh
VI/dark/hẹp đã inspect. WSS chỉ xác minh từ chối certificate tự ký.

Còn pending: Windows linked/native và installer/resource (thiếu MSVC `link.exe`),
private update keys/signing identities (0 identities), notarization/Authenticode,
trusted WSS, live Flutter discovery/GUI, accessibility/AccessKit/screen-reader và
font acceptance mọi OS, thumbnail Windows, dependency license/source review.
Không đánh dấu acceptance toàn app hoàn thành; không tạo chat kế tiếp.

Release tooling cần Python >=3.11:
`/opt/homebrew/bin/python3 -B tools/test_f21_release.py` và
`/opt/homebrew/bin/python3 -B tools/release.py package macos-arm64 --output dist`.
Toolchain tạm vẫn phải đặt CARGO_HOME/RUSTUP_HOME/PATH như trên.
