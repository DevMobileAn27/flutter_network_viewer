# UI style — đối chiếu code Flutter / Rust

Đợt 2026-11-13. Chỉ sửa Rust; dùng source Flutter làm chuẩn.

| Hạng mục | Nguồn Flutter | Thay đổi Rust |
|---|---|---|
| Font faces | pubspec.yaml | Bundle Regular/Medium/SemiBold/Bold/Italic; chọn FontFamily thật thay vì egui strong |
| Header | viewer_shell.dart _header | SemiBold16, logo38, gap12, căn trái title |
| Navigation | viewer_shell.dart _toolbar | Selected SemiBold13, unselected Regular13; Material icon font riêng |
| Badge | components.dart StatusChip | Bold11, fill method/status, radius5, padding ngang8 |
| Network | network_request_table.dart | Header9, URL12, metadata10 gồm milliseconds; Time11 |
| Console | viewer_shell.dart _flutterConsoleList | Row36, padding8/6, badge11Bold, clock10, message12; nền chọn cả hàng |
| Inspector | inspector.dart | Margin18, title Console13SemiBold; URL12 mono; tabs pill38/label12, background input/radius30 |
| Mono text | components.dart mono | 12px, line height19.8 (1.65), áp dụng nội dung inspector và editor draft JSON |
| JSON workspace | json_beautify_screen.dart | Lề20/4/20/20; toolbar44; gap8; editor padding16/radius7/viền accent1; divider16 với line1/hover3; output placeholder surface/radius6/viền line1 |
| Settings sections | settings_screen.dart _section/_field | Width tối đa764, section padding24/radius12, title16SemiBold, label12muted/gap8; field16 với padding12 và viền accent1/focus1.5 |
| Dropdown label | viewer_dropdown.dart | Medium12; menu radius12 |
| Help | help_dialog.dart _step | Icon20/gap12/title13SemiBold/body13 description/gap6/section gap20 |
| Dialog/tooltip | Flutter Material3 / viewer_theme.dart | Dialog radius28/margin24; tooltip delay400ms |

Kiểm chứng: tests/ui_parity.rs kiểm font thực trong paint output và căn trái;
tests/desktop_ui.rs kiểm JSON insets/divider; tests/f17_ui.rs kiểm glyph tiếng Việt
ở mọi face, font Medium12 của dropdown và chiều cao ô nhập. Harness Settings
cuộn tới control trước khi click sau khi section có padding thực. Không bỏ assert.

Giới hạn: đây là đồng bộ style theo source; chưa kiểm screenshot native do quyền
Computer Use. Settings đã chuyển sang route riêng trong đợt sửa tiếp theo bên dưới.
Console summary vẫn hiển thị dòng đầu; wrap toàn message như Flutter chưa được
chuyển trong đợt style. Một số nội dung inspector vẫn dùng khối text thay DataSection.
Không tuyên bố toàn app đạt pixel parity. Native Windows vẫn pending.


## Settings route — 2026-11-13

Đối chiếu trực tiếp `lib/ui/settings_screen.dart` và l10n/viewer_dropdown Flutter.
Rust hiển thị Settings toàn cửa sổ, AppBar56 có Back/Escape; shell chính không
render phía dưới. Body cuộn toàn chiều rộng, trên/dưới28, nội dung764 khi width>820,
lề ngang28 khi hẹp. Thứ tự Interface (Theme trước Language), Connection, MCP,
App updates. Card padding24/radius12, header icon20/gap12/title16/gap24;
label12/gap8, dropdown Medium12/full width, input16/padding12, action44/radius10.
Connection switch bên phải và nhãn wrap khi hẹp; bỏ separator/subheading thừa.
MCP switch/status ở header; port160 và endpoint cùng hàng khi inner width>=680,
stack khi hẹp; copy setup với menu chọn Codex/Claude thay hai nút copy riêng.
Save/retry/clipboard, theme/language persistence và capture state giữ nguyên.

TDD: test full-page fail trước sửa, pass sau sửa; kiểm rect/layout và pointer/key,
Back/Escape, Escape menu không đóng route, 360px/EN/VI/light/dark không tràn ngang,
MCP menu/client/clipboard và updater sau khi cuộn. 282 tests pass; fmt/clippy/build pass.
Logs `artifacts/ui-parity/settings-{red,regression,clippy,fmt,build}.log`.
Bundle debug mới đã đồng bộ SHA256. Chưa kiểm screenshot/native trong đợt này;
không dùng bằng chứng native của smoke cũ để kết luận giao diện mới đạt pixel parity.

Response body: DataSection toolbar 30, JSON/Tree pill 128, copy icon; surface,
mono 12 và hàng 22 theo NativeJsonViewer. Code có gutter và nút gập riêng;
Tree có connector, số phần tử và palette theo loại token. Panel detail clip
nội dung theo chiều rộng divider, kể cả URL dài. Kiểm renderer trên macOS;
chưa so ảnh native từng pixel.


### Scrollbar và Copy (2026-11-13)

Theo yêu cầu riêng cho Rust: mọi ScrollArea đặt AlwaysHidden, giữ input cuộn.
Copy dùng Google Material Icons content_copy U+EF7F từ font bundled
assets/fonts/material_icons/MaterialIcons-Regular.otf (bộ icon đã dùng trong
shell). Nút chỉ paint glyph, giữ tooltip/accessibility và clipboard; áp dụng
cURL, request/console detail, JSON, Help và MCP. UI tests kiểm paint glyph,
không có text Copy trong nút, không paint scrollbar và cuộn vẫn đổi offset.
287 regression tests pass; fmt/clippy/build pass. App debug FNV Rust.app đã
cập nhật. Không đổi các icon vector Back/find đã có trong phạm vi sửa này.


### Request inspector theo phản hồi ảnh (2026-11-13)

Top URL căn trái, giữ cURL icon bên phải. Overview/Header dùng card surface,
viền line/radius7, lề12, title SemiBold13 và key Medium12/muted; giá trị mono12,
line-height19.8, selectable/wrap. Bảng stack khi inner width<420. Raw dùng JSON
viewer chung. Empty header/body có nhãn bản địa hóa No data/Không có dữ liệu.
List và inspector dùng nền surface xuyên suốt. Row chọn dùng xanh #2563eb alpha
26/255 (10%), giữ màu text/muted; marker trái và badges giữ phân biệt trạng thái.
UI regression 290 pass; bằng chứng inspector-* trong docs/artifacts/ui-parity.


## Giao diện hiện tại

Bộ Vintage đã thay thế các màu, font và góc bo cũ. Xem
[`vintage-widget-inventory.md`](vintage-widget-inventory.md) cho các thành phần
production, gallery và palette. Bảng đối chiếu cũ bên trên là lịch sử migration.
