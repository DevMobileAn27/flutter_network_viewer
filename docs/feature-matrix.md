# Tính năng và bài test toàn app

Nguồn: README, cấu trúc CodeGraph, `lib/models/capture.dart`,
`lib/services/capture_store.dart` và danh mục test Flutter trong
`flutter-test-catalog.json`. Catalog giữ tên test gốc, kể cả tham số Dart;
đây là đặc tả đối chiếu, không phải kết quả chạy test Rust.

Mỗi hàng là một tính năng cho một chat. Bài test gồm unit/contract, integration
và UI theo phạm vi. F01–F12 core và F13–F20 desktop có test executable. Những hàng pending là acceptance
specification phải được chuyển thành test executable trong chat tương ứng.

| ID | Tính năng | Bài test bắt buộc | Test Flutter tham chiếu | Trạng thái |
|---|---|---|---|---|
| F01 | Capture store, model, Pause/Clear/export | Validate envelope trước mutation; reject từng event; ghép phase đảo thứ tự; request ID theo session; console append; FIFO chung 5.000 record/64 MiB; cập nhật byte; xóa session; metadata tối đa 100 khi không còn dữ liệu; schema export v1; cURL quote/binary | capture_store_test | Implemented, xem progress |
| F02 | JSON document, format, Tree, cache | Object/array/scalar/null/rỗng; Unicode/escaped key; format 2 space; invalid giữ raw và trả dòng/cột; node path/index; folding không mất dữ liệu; LRU theo budget; oversized không cache; Clear giải phóng | json_document, json_body, native_json_document | Core implemented (13 tests pass); JSON/Tree UI F15 implemented, viewport performance F16 implemented (macOS) |
| F03 | JSON find | Case/whole word/regex; regex sai/zero width; next/previous wrap; offsets Unicode; key/value paths; unfold match; giữ query khi đổi mode; no results; Ctrl/Cmd+F focus cục bộ | json_find | Core implemented (13 tests pass); UI focus/highlights/navigation F15 implemented, body-type/stress F16 implemented (macOS) |
| F04 | Console model và nhóm log | Bỏ ANSI/flutter prefix chỉ khi display; giữ raw; JSON block/Flutter exception; không ghép khác session/isolate/source hoặc quá 2s; giới hạn 200 dòng/64K; tìm logger/error/stack/args; severity | flutter_console | Core implemented (14 tests pass); UI F14 implemented |
| F05 | Body form-data | URL encoded lặp key/rỗng/plus/Unicode; multipart nhị phân, filename mở rộng; MIME case insensitive; truncated giữ phần còn lại; malformed boundary/base64/percent không panic; raw nguyên vẹn; omitted body báo lý do | form_body, form_body_view | Core implemented (13 tests pass); UI Raw/Form, live updates/notice F16 implemented; VI/EN F17 |
| F06 | Body image | Nhận base64/data URI/JSON string và MIME thiếu; bytes lỗi fallback raw; thay response cập nhật; chia sẻ bytes preview/enlarged; zoom/pan/fit; không reload khi zoom | image_preview, native_image_view | Core implemented (18 tests pass); GUI F16 implemented; ImageIO thumbnail >64 MiB tested macOS, Windows F21 |
| F07 | Preview màu | HEX/CSS rgb(a)/hsl(a)/Flutter AARRGGBB/tên; loại ID và substring; Unicode offset; copy/search/folding không đổi raw | body_color | Core implemented (15 tests pass); JSON GUI swatches/clipboard F15 implemented, scalar/form swatches F16 implemented |
| F08 | Settings và persistence | Light mặc định, VI/EN, locale lạ fallback; lưu URL, MCP opt-in false/port 38943; validate port/layout; migrate 6 tab; lưu split riêng và độ rộng cột; settings hỏng fallback | settings, layout_preferences, network_columns | Core implemented (18 tests pass); desktop layout F13–F16, locale/theme/discovery/manual settings F17 implemented macOS; MCP controls/lifecycle F18 implemented macOS |
| F09 | VM protocol và adapter | DevTools/encoded HTTP/WS URL; auth token; JSON-RPC correlation/error/timeout; HTTP profile timing/header/Unicode/binary/error; isolate-scoped IDs; body size guard và malformed bytes | viewer_server, live_vm_capture, support/fake_vm_service | Core implemented (21 tests pass); WebSocket fake VM + live Dart SDK HTTP/body/stdout/log F21 tested; trusted WSS/live Flutter GUI pending |
| F10 | Capture lifecycle và adaptive polling | Network 1.4s +0.2s tối đa 5s; thay đổi reset; console batch độc lập; poll không overlap; Stop hủy timers; pause bỏ events; Clear bỏ pending/in-flight và inclusive cursor; không replay; giới hạn queue | adaptive_capture, capture_cleanup, viewer_server | Core implemented (22 tests pass); single attached isolate, reconnect/bootstrap F11 và GUI pending |
| F11 | VM reconnect và nhiều isolate | Latest connect wins; socket đóng reconnect cùng URL; timeout profile giữ transport và data; isolate Exit hủy hung request; delayed extensions; hot restart một app tab; endpoint switch dọn session; disposal hủy attempts | viewer_server | Core implemented (17 tests pass: 14 deterministic + 3 WebSocket); GUI F13/live SDK pending |
| F12 | Auto discovery và chọn app | Chỉ process DDS local; giữ auth; redirect/dedupe; bỏ VM không Flutter và Viewer; chờ app start; nhiều app đợi chọn; mỗi scan thêm một app rồi tắt; giữ app cũ; Stop/manual hủy scan | vm_service_discovery, vm_service_discovery_widget | Core implemented (15 tests: 12 deterministic + 3 HTTP/WebSocket); GUI picker actions F13 tested; preferences F17 implemented; F21 combined DDS→multi-app real sockets/console pass; live Flutter discovery acceptance pending |
| F13 | Desktop shell, tabs và Network | App chạy macOS/Windows; All/app lọc cả console/network; search URL/method/status kết hợp filters; request pending cập nhật khi chọn; 6 tab inspector; cURL/copy/export thật; sidebar hẹp/cuộn cột; Escape/outside dismiss | viewer_widget, application_tabs, network_columns | Implemented: 11 egui input UI tests + native macOS smoke; regression Manual URL 12 pt inset (480/1360 pt); Windows chưa chạy (F21) |
| F14 | Desktop Console và inspector | Lọc source/level/search; group JSON/error; chọn detail/copy/raw; split riêng; log update không rebuild toolbar; selection khác không serialize raw chưa mở | flutter_console, inspector_performance, layout_preferences | Implemented: 5 Console egui input tests + native macOS smoke; early Stdout regression fixed, Windows F21 |
| F15 | Desktop JSON và beautify | Format/Ctrl+Enter; JSON/Tree/copy/find/color; draft/folds/query giữ khi đổi tab/resize; lỗi dòng/cột; scroll cả chiều ngang/dọc; Ctrl+A copy toàn bộ không gutter | json_beautify, json_body_height, json_find, virtual_json | Implemented: 9 JSON egui input tests + native macOS smoke; viewport virtualization/stress F16 implemented, Windows F21 |
| F16 | Desktop body previews và performance | Form/image/raw switch; live body invalidate đúng; danh sách/JSON chỉ vẽ visible rows; selected detail không đổi do event khác; Clear giải phóng cache và selection; overlay/clipping/disposal; tạo host lỗi có fallback nếu dùng native host; thumbnail ảnh nguồn vượt budget giải mã F06 | native_capture_list, native_json_view, native_image_view, inspector_performance | Implemented macOS: 12 new input UI tests, native smoke + frame/RSS stress; egui replaces native child hosts; Windows/thumbnail backend F21 |
| F17 | Help, notifications và desktop settings | VI/EN light/dark; help tách manual connect; discussion mở đúng URL; toast 4 loại, 3 dòng, góc trái dưới; success/info 3s; warning/error/confirm chờ; cancel=false; replace/dispose hủy timers; clipboard thật; cửa sổ hẹp vẫn scroll | help_dialog, viewer_notification, viewer_dropdown, settings | Implemented macOS: 15 renderer/input tests + native settings reload/clipboard/narrow scroll/timers/fresh VM connection; Windows F21 |
| F18 | MCP read-only HTTP | Off mặc định; 5 tools app/request list/detail/log/stats; protocol versions; loopback only, origin/host validation; unknown routes; conflict/retry/races; stop thả port; Codex/Claude config copy đúng port | viewer_mcp_server, settings | Implemented macOS: 7 core/HTTP tests +3 renderer/input tests, production native HTTP/clipboard/settings/retry/port release; Windows F21 |
| F19 | Diagnostics và resource usage | Structured logs rotate budget; không chứa body/header/console; path lỗi không phá capture; memory sample định kỳ, lỗi xóa sample và phục hồi; disposal dừng timer | viewer_diagnostics, process_resource_usage | Implemented macOS: 4 diagnostics/resource tests +1 renderer/input test; native fresh VM/path failure/recovery/periodic RAM/narrow VI-dark/disposal; Windows F21 |
| F20 | Updater | Feed theo OS/CPU; version/progress; serialize checks; retry lỗi; no-update/native callbacks; confirmation một lần; unsupported không gọi native; cleanup listener; signature/checksum; không cài thật trong test | app_updater, settings_updates, tool/test_update_feed.py, tool/test_release_encoding.py | Implemented macOS: 5 controller +1 renderer/input +2 integrity tests; real Sparkle error/current/available + production Settings native; vendor SHA256, signature tamper rejection; Windows native/release generation F21 |
| F21 | Build/release và acceptance toàn app | CI fmt/clippy/unit/integration/UI; build macOS ARM/Intel + Windows x64; metadata/icons/fonts/license; installer/feed/checksum; chưa publish; fake VM end-to-end capture → filter → inspect → pause/clear → MCP → export; live VM smoke; restart settings; stress memory | .github/workflows, tool/test_prepare_github_release.py, live_vm_capture | Partial: 270 Rust +8 release +2 integrity +1 signing-helper pass; macOS ARM/Intel zip builds/layout/startup (Intel Rosetta), native full flow/MCP/export, live Dart SDK, multi-app sockets, F13–F20 regression/stress pass. Windows cross-check/clippy pass nhưng linked/native blocked MSVC; signing/trusted WSS/accessibility/live Flutter còn pending. Xem release.md |

## Điều kiện nghiệm thu cuối

1. Toàn bộ hàng có test executable tương ứng, chạy pass; không dùng ignore/todo
   để biến tính năng chưa làm thành thành công.
2. Fixture từ Flutter giữ nguyên; export và dữ liệu gốc tương thích, khác biệt
   timestamp được chuẩn hóa. Test byte budget tính byte UTF-8.
3. Dùng fake VM và clock xác định để test timeout/reconnect/polling không flaky.
   MCP dùng loopback port ngẫu nhiên trong test; teardown luôn thả socket.
4. Chạy GUI trên macOS và Windows: thao tác thực, clipboard, export, resize,
   keyboard và accessibility. Ghi rõ nền tảng nào chưa được chạy.
5. Chạy stress large JSON/body/log; đo bounded memory và rendering. Không suy
   ra hiệu năng GUI từ unit test model.
6. Chạy live Dart VM integration khi có Flutter SDK; thiếu SDK phải báo pending.



## Bổ sung UI parity 2026-11-13

Shell/Network: sửa cấu trúc bảng theo Flutter, 52px row/30px header, URL ở giữa,
metadata dòng phụ, full-row selection, màu method/status, quy tắc ba divider.
`tests/ui_parity.rs` (4), `tests/desktop_ui.rs` (39), `tests/f17_ui.rs` (15),
`tests/f19_ui.rs` (1) kiểm chứng renderer/input và regression layout.
Toàn bộ Rust: 276 pass, fmt/clippy pass; xem artifacts/ui-parity và progress.
Console/inspector/JSON/body/settings/dialog visual parity toàn diện vẫn pending.


UI style bổ sung: 281 tests pass sau đồng bộ font faces/weights, header,
Network/Console badges, inspector tabs, JSON padding và Settings/Help controls.
Xem ui-style-parity.md cho mapping source và giới hạn chưa đạt toàn app parity.


Settings route bổ sung 2026-11-13: `tests/f17_ui.rs::settings_is_a_full_page_with_flutter_section_order_and_back_navigation`
kiểm full-page/Back/Escape, section order, responsive764/28 và MCP geometry.
F17/F18/F20 renderer tests giữ chức năng khi cập nhật bố cục, chọn client rồi copy.
282 regression tests pass; fmt/clippy/build pass. Native visual verification lượt mới pending.

Settings Back arrow: `tests/f17_ui.rs::settings_back_button_paints_a_left_arrow_independent_of_icon_font`
kiểm ba stroke paint thực và click Back; regression283 pass (2026-11-13).

F13/F15/F16: kiểm split inspector giữ nguyên khi chọn/đổi tab và vẫn lưu khi kéo;
kiểm JSON/Tree body qua paint output và thao tác pointer. Hai test regression mới
trong tests/desktop_ui.rs; 285 test toàn bộ pass, fmt/clippy/build pass.


UI refinement 2026-11-13 (F13–F18): scrollbar luôn ẩn trong Rust; Copy chỉ dùng
Google Material Icons kèm tooltip. Mapping UI regression:
`tests/desktop_ui.rs::json_scrollbars_are_hidden_but_wheel_still_scrolls` và
`request_and_json_copy_buttons_paint_material_icons_and_copy_data` kiểm paint,
cuộn hai chiều, clipboard. Các test copy console/Help/MCP hiện có giữ pass.
Kết quả 287 pass, xem progress và docs/artifacts/ui-parity/scroll-copy-*.


UI refinement request inspector (F13/F15, 2026-11-13):
- `inspector_url_is_left_aligned_and_metadata_tabs_have_cards`: URL left paint,
  Overview/headers card style, Raw JSON viewer và clipboard.
- `empty_headers_and_bodies_show_localized_no_data`: empty header/body VI.
- `request_table_is_white_and_selection_is_ten_percent_blue_with_readable_text`:
  nền surface trắng, alpha10% và chữ đậm cho row chọn.
290 regression pass; fmt/clippy/build pass. Xem progress và inspector-* logs.


F13 URL overflow: tests/desktop_ui.rs::inspector_url_scroll_reveals_end_keeps_copy_fixed_and_resets_per_request
kiểm cuộn URL dài tới cuối, nút Copy cố định/clipboard đầy đủ và URL mới căn trái.
291 regression pass, fmt/clippy/build pass; logs url-scroll-* trong ui-parity.


F15 find icons và fold formatting: UI tests
`json_find_clear_uses_same_material_icon_size_as_navigation` và
`collapsed_json_objects_and_arrays_show_closing_delimiter_and_keep_copy_complete`.
Kiểm X/material18/target24, query clear và fold object/array kèm dấu đóng/phẩy,
expand/copy toàn bộ. 293 regression tests pass khi chạy tuần tự; fmt/clippy/build
pass. Logs fold-icons-* trong docs/artifacts/ui-parity, gồm socket failures ở
các lượt parallel và lượt serial thành công.


F13 metadata card: inspector_url_is_left_aligned_and_metadata_tabs_have_cards
kiểm không còn title/Copy row lặp trong Overview/Request header/Response header,
giữ card và dữ liệu. Body/Raw toolbar giữ nguyên. 293 regression pass; xem
progress và card-header-* logs (gồm một lượt socket timeout trước lượt pass).


F17 Settings layout: test settings_icons_labels_fields_and_actions_follow_requested_layout
kiểm icon Material chính xác, text căn trái, label trên input và action căn phải.
Wide MCP fields giữ hai cột có child layout dọc; narrow stack. Các UI tests
Connection/MCP/locale/theme vẫn pass. Full regression 294 pass; fmt/clippy/build
pass; evidence settings-layout-* trong docs/artifacts/ui-parity.


F13/F17 UI refinements: network_filters_live_in_header_and_search_has_compact_padding,
header_filter_can_recover_from_empty_result và
mcp_endpoint_and_port_text_are_centered_vertically kiểm header dropdown, compact
search/hint, empty-results recovery, MCP y alignment. Regression297 pass;
fmt/clippy/build pass, evidence header-filters-* trong docs/artifacts/ui-parity.


F13 outer padding ba header rows: three_header_rows_have_eight_by_four_outer_padding
kiểm margin8x4 ở width480/1360 và search-frame đúng các cạnh row. Title x và
row heights cập nhật. Regression298 pass; fmt/clippy/build pass; logs
header-padding-* trong docs/artifacts/ui-parity.


Header manual connection / discovery refinement: app selector chuyển lên
dòng1; URL + Sử dụng + Quét dự án & kết nối; icon buttons nền trắng/border
xám/icon xanh. Startup idle kể cả saved autoDetect=true; scan one-shot,
1 target auto-connect, nhiều target chọn qua vertical overlay. Tests
manual_connection_controls_and_app_list_live_in_first_header_row,
narrow_header_scroll_reveals_manual_scan_without_scrolling_app_list,
desktop_startup_never_scans_or_connects_saved_settings cùng discovery và
app filtering regressions. 301 tests pass, fmt/clippy/build pass; evidence
manual-connect-* trong docs/artifacts/ui-parity.


Header URL refinement: nhóm input/Use/scan căn phải, bỏ nhãn URL và
border/padding wrapper. UI test header_connection_controls_align_right_without_url_wrapper
kiểm geometry/text/paint; manual/narrow-scroll tests giữ pass. Regression302
pass, fmt/clippy/build pass; evidence header-right-*; Rust app đã đồng bộ.


Header Use height/alignment: allocation32, cùng top/height với scan button.
UI regression assertion red→green; full302 tests pass; fmt/clippy/build pass.
Evidence use-height-*; Rust app bundle đã đồng bộ.


Project selector right alignment/padding: natural content width capped at
360/140, grouped12px before URL input, list margin0, each project8x4.
UI test project_list_is_right_grouped_with_zero_outer_and_eight_by_four_chip_padding
plus app-filter/narrow-scroll regressions pass. Full303 tests pass;
fmt/clippy/build pass; evidence project-padding-*; Rust app đã cập nhật.


Project list width correction: giữ fixed360 wide/140 narrow, outer padding0,
chip8x4 và căn phải. UI test width360 red→green;303 regression tests pass;
fmt/clippy/build pass. Evidence project-width-*; Rust app đã cập nhật.


Search row refinement: Network/Console search width350 trong navigation
trước pause/clear/export (gap12); Network bỏ dòng3, Console Source/Level
giữ nguyên. UI test search_is_350_wide_in_navigation_before_capture_actions
cùng search/filter/reset và geometry regressions pass. Full304 tests
pass; fmt/clippy/build pass. Evidence search-row-*; Rust app đã cập nhật.


Header branding removal: bỏ logo/title sát trái. UI test
header_does_not_render_brand_logo_or_title pass ở wide/narrow; existing
header controls/alignment regressions pass.304 tests pass, fmt/clippy/build
pass; evidence remove-brand-*; Rust app đã cập nhật.


Compact header v5 + input borders: hai dòng32px, fixed-width centered pill
tabs, project list fill, Scan/Manual URL popup, search350 và contextual actions
căn phải; Console Source/Level dropdown trong heading. Toàn bộ TextEdit và
composite search/editor giữ outline xám mặc định, xanh khi focus (light/dark).
4 UI tests mới cùng existing regression:308 pass; fmt/clippy/build pass.
Native check header/search focus/modal/Console/JSON thành công. Evidence:
compact-header-* trong docs/artifacts/ui-parity/. Rust app đã cập nhật.


## Căn dọc nội dung input (2026-11-16)

Rust: toàn bộ TextEdit singleline căn giữa theo chiều dọc (Manual URL,
JSON Find trong body/workspace; search/settings vốn đã Center). Multiline
JSON draft đặt Align::Min để giữ nội dung ở trên. Giữ viền xám/xanh theo focus.
UI test singleline_inputs_center_text_and_multiline_keeps_text_at_top đo
vị trí text được paint so với rect input, red trước sửa và green sau sửa.
Full regression309 pass/0failed/0ignored; fmt/clippy -D warnings/build pass.
Evidence docs/artifacts/ui-parity/input-alignment-*.log. App bundle atomic
replace, SHA256 trùng binary build. Không sửa Flutter.


## Làm gọn màn hình hướng dẫn (2026-11-16)

Rust: popup Help padding16, title16, section title/body12, icon Material16
trong ô24 đồng nhất. Header có nút X góc phải; footer giữ Discussion/copy
và căn phải Close. Bỏ dòng URL dài dưới footer, giữ URL ở tooltip và thao
tác copy/mở trình duyệt. Nội dung tiếng Anh/Việt cập nhật Scan chủ động,
auto-connect khi có1 project, picker khi nhiều project, Manual URL/Find,
filter tại table headings và toolbar JSON/Download hiện tại. Scroll chỉ
trong nội dung, header/footer luôn truy cập được khi cửa sổ hẹp.

UI test help_guide_has_compact_header_padding_and_current_connection_steps
red→green: kiểm viewport/padding/header/footer/dismiss và nội dung mới ở
EN/VI. Các test Help link/copy/Close và scroll narrow light/dark pass.
Full310 tests pass; fmt/clippy -D warnings/build pass. Lần regression đầu
timeout ở coordinator_socket (VM did not progress), lần chạy lại toàn bộ
pass; lưu cả hai logs để truy vết. Evidence help-refresh-* trong
docs/artifacts/ui-parity/. Native screenshot đã kiểm: modal trắng có
padding, 5 mục rõ ràng, icon đúng và các nút nằm trong popup. App bundle
atomic replace, SHA256 trùng binary build. Không sửa Flutter.


## Thông báo kết quả chỉ qua popup (2026-11-16)

Rust: bỏ error-banner lỗi kết nối (trước đây lỗi xuất hiện trùng ở banner
và popup). Manual URL validation, JSON format/regex search, MCP errors,
update results và body preview warnings/errors dùng chung Notifications.
Bỏ text lỗi/thành công inline cùng checkmark kết quả cập nhật; các dữ liệu
trạng thái vận hành vẫn có trong UI. Component gửi sự kiện qua notify;
notify_changed phát một lần khi trạng thái đổi để dismiss không tự mở lại.
JSON Format lỗi phát lại khi người dùng thử lại. Popup hiện có vẫn giữ
quy tắc timeout và confirmation. Raw capture/log/error fields vẫn là nội
dung dữ liệu, không phải thông báo thao tác.

TDD failures_paint_only_in_notification_popup_and_do_not_repeat_each_frame
red→green: kiểm lỗi transport chỉ paint1 lần trong popup, không banner;
dismiss không lặp, manual URL/JSON format chỉ hiện trong popup, retry có
thông báo mới. Mở rộng assertions regex/MCP/update cùng parity contract.
Full311 tests pass/0failed/0ignored; fmt/clippy -D warnings/build pass.
Evidence docs/artifacts/ui-parity/popup-only-*.log (selected log ban đầu
bao gồm2 test contract cũ đã được cập nhật; support/full log đều pass).
Native screenshot xác nhận popup lỗi update nằm góc dưới trái, không có
banner lỗi trong nội dung. Không hoàn tất native click Manual URL do CUA
báo noWindowsAvailable; luồng này đã pass kiểm thử UI thực bằng pointer.
App bundle atomic replace, SHA256 trùng binary build:
fab62639012906585099f1fb588f320661eef9282fd7281d5dd35047fc314e5e.
Không sửa Flutter.


## Cỡ chữ thống nhất / bảng chữ thuần (2026-11-16)

F13/F17 refinement: mọi text dùng fontSize mặc định14, field−/+ và nhập/kéo
trong Settings (10–24, persist/reload). Method/Status/Console level bỏ badge;
Network kẻ header và dọc các row. Tests:
`desktop_ui::unified_font_setting_updates_all_painted_text_and_plain_network_grid`
(paint/input, min/max, themes, inspector/JSON/help) và
`settings::font_size_defaults_validates_and_survives_restart`.
313 regression pass; fmt/clippy/build pass. Evidence unified-font-* tại
 docs/artifacts/ui-parity/. Native Settings field đã quan sát; native click+
bị noWindowsAvailable; UI pointer automation pass. F21 blockers giữ nguyên.


F13 scroll clipping fix (2026-11-16): network rows clip tại mép dưới fixed
header, loại bỏ bleed3px của ScrollArea. UI test
`desktop_ui::scrolling_network_rows_are_clipped_below_fixed_header` red→green
với wheel/partial row, light/dark, font14/24. Full314 pass; fmt/clippy/build
pass. Evidence docs/artifacts/ui-parity/header-scroll-*.log.


F13 title/filter gap (2026-11-16): đo title thực thay ước lượng minimum width;
kéo Method/Status sát filter với gap2px (rounding<1px), EN/VI và font14/24.
UI test `desktop_ui::network_columns_shrink_to_small_title_filter_gap`
red→green, giữ filter click.315 regression pass; fmt/clippy/build pass.
Evidence docs/artifacts/ui-parity/filter-gap-*.


F20 startup background check (2026-11-16): background Failed/Current không
phát popup; manual CheckUpdates vẫn báo kết quả. UI/controller test
`f20_ui::startup_update_failure_is_silent_but_manual_retry_reports_errors`
red→green cho configure/check/async/timeout/current, macOS/Windows branches.
316 regression pass (retry sau VM timeout); fmt/clippy/build pass.
Evidence docs/artifacts/ui-parity/startup-update-*.


## Vintage widget demo — chờ duyệt (2026-11-16)

Bước kit UI riêng, chưa thay style app cũ.35 nhóm widget/7 tab được đối chiếu
source trong docs/vintage-widget-inventory.md và vintage::INVENTORY. Native
startup vào demo, có nút sang app cũ/quay lại. Palette xanh ngà/mực xanh đen,
radius0, một fontSize chung, JSON giữ7 vai trò màu.4 UI/host tests mới kiểm
paint/input/completeness/narrow routing;320 full regression pass, fmt/clippy/
build pass. Evidence vintage-* trong docs/artifacts/ui-parity. Native screenshot
bị CUA timeout; chờ người dùng review/duyệt trước migrate toàn app.


Vintage review refinement (2026-11-16): selected text#EBF0E4, icon tile#282F26/
glyph#EBF0E4, persistent focused input outlines, JSON background#EBF0E4,
continuous table grid including outer edges, square loader3–10 slots/inset2px
inside1px border with200ms cyclic selection.4 new executable UI tests,
8 vintage tests and324 full regression pass; fmt/clippy/build pass.
Evidence docs/artifacts/ui-parity/vintage-refine-*. App cũ giữ nguyên.


Vintage tables review (2026-11-16): reusable Title/Value table with optional
third Action/Copy column (metadata2 cols, Form3 cols). Console now has4 aligned
Source/Level/Time/Message columns, contiguous grid/perimeter, selection and
fixed/clipped header during wheel scroll. Gallery respects viewport width.
10 vintage UI tests/326 full regression pass,0 failed/ignored; fmt/clippy/build
pass. Evidence docs/artifacts/ui-parity/vintage-tables-*. Native screenshot not
verified; production egui input/paint tested. Legacy migration remains pending approval.


Vintage square icon review (2026-11-16): Controls/toolbar/Action Copy share
34x34px icon button helper, radius0, padding4px. UI test checks square geometry
at font10/14/24 and Action containment; existing pointer Copy test still passes.
11 vintage UI tests/327 full regression pass; fmt/clippy/build pass. Evidence
`docs/artifacts/ui-parity/vintage-icons-*`. Demo only, migration pending approval.


Vintage alignment review (2026-11-16): metadata/Form content Title left8px,
Value right8px, Action centered with square34x34 button. Headers unchanged.
12 vintage UI tests/328 full regression pass,0 failed/ignored; fmt/clippy/build
pass. Production text galley edges and button centers checked at font14/24.
Evidence `docs/artifacts/ui-parity/vintage-alignment-*`; legacy migration pending.


Vintage hover review (2026-11-16): enabled action controls paint at1.2x around
center, including text/icon/border; allocation/base font unchanged, expanded
rim clickable. Shared helper covers buttons/icons/tabs/toggles, dropdowns and
options, checkbox/radio, numeric field, links and swatch. Disabled stays1.0.
14 vintage UI tests/330 full regression pass,0 failed/ignored; fmt/clippy/build
pass. Evidence `docs/artifacts/ui-parity/vintage-hover-*`. Native screenshot
not verified; legacy migration pending approval.


Vintage hover sharpness correction (2026-11-16): supersedes paint-transform
approach above. Hover builds controls with real width/height/font sizes1.2x,
including explicit icon size; no bitmap/mesh zoom. Dropdown options reset to
base font before own hover; real widget hit rectangles, centered square Action.
15 vintage UI tests/331 full regression pass,0 failed/ignored; fmt/clippy/build
pass. LayoutJob font sizes, fresh glyph layout, DPI1/2, font14/24, Action center,
clipboard and narrow navigation verified. Evidence `vintage-hover-sharp-*` in
`docs/artifacts/ui-parity`. Native screenshot not verified; demo only.


Vintage hover fixed layout (2026-11-16): normal layout slot/offset retained;
control renders1.2x around its center using real dimensions/font and grown hit
area. Sibling widgets/rows/cards/panels stay exactly unchanged; font rendering
remains native (no mesh zoom).16 vintage UI tests/332 full regression pass,
0 failed/ignored; fmt/clippy/build pass. Font14/24 full probe-map comparisons,
restore, center and existing DPI/clipboard tests pass. Evidence
`docs/artifacts/ui-parity/vintage-hover-fixed-slot-*`. Demo only, no native screenshot.


Vintage opaque controls (2026-11-16): full control backing SURFACE alpha255
including quiet/disabled/checkbox/radio/link, opaque weak/button fills. Hover
paint sublayer above parent siblings prevents later neighbor text showing over
the enlarged dropdown; original clip and interaction layers retained.18 vintage
UI tests/334 full regression pass,0 failed/ignored; fmt/clippy/build pass.
Evidence `docs/artifacts/ui-parity/vintage-opaque-*`. Demo only; native screenshot
not verified. Real font rendering, fixed slots, square icons and JSON7 colors retained.


Vintage hover factor update (2026-11-16): current factor1.1 (replaces1.2).
Real widget/font/icon sizes grow10%; fixed layout, opaque surfaces and overlay
retained.18 UI tests/334 full regression pass,0 failed/ignored; fmt/clippy/build
pass. Evidence `docs/artifacts/ui-parity/vintage-hover-110-*`. Demo only.


Vintage dropdown exception (2026-11-16): Theme/Network dropdowns and menu options
never scale on hover (rect/font1.0), retain opaque backgrounds/overlay/selection.
Other action controls retain1.1.19 UI tests/335 regression pass,0 failed/ignored;
fmt/clippy/build pass. Evidence `docs/artifacts/ui-parity/vintage-dropdown-no-scale-*`.
Demo only; native screenshot not verified.


Vintage cursor review (2026-11-16): enabled click controls and selectable
Network/Console rows use PointingHand; disabled uses Default, text uses Text,
numeric/splitter keep ResizeHorizontal. Pointer exit restores Default. Scale,
opaque surfaces and dropdown no-scale retained.21 UI tests/337 regression pass,
0 failed/ignored; fmt/clippy/build pass. Evidence
`docs/artifacts/ui-parity/vintage-cursor-*`. Demo only; native screenshot not verified.


Vintage dropdown list padding (2026-11-16): popup container margin0, row gap0;
full-width options own padding12px horizontal/8px vertical. Both Theme/Network
retain no-scale, opaque surfaces, selected colors and hand cursor.22 UI tests/
338 regression pass,0 failed/ignored; fmt/clippy/build pass. Evidence
`docs/artifacts/ui-parity/vintage-dropdown-padding-*`. Demo only; no native screenshot.


Vintage project selector review (2026-11-16): plain All/pgapp/staging labels,
no leading status glyphs; selection retained.23 UI tests/339 regression pass,
0 failed/ignored; fmt/clippy/build pass. Evidence
`docs/artifacts/ui-parity/vintage-project-plain-*`. Demo only, no native screenshot.


Vintage production: Network/Console/JSON/Settings/Help renderer, shared UI kit,
Cascadia Mono at one configurable base size, independent Code/Tree folds and
painted root/branch lines, seven syntax roles, metadata/form copy grids. Verified
by `tests/desktop_ui.rs`, `tests/vintage_ui.rs`, `tests/f17_ui.rs`, `tests/ui_parity.rs`
and the release font/license tests. 343 Rust tests and 8 packaging tests pass;
fmt/clippy/build pass. macOS native startup visually inspected on Network.
Evidence `docs/artifacts/ui-parity/vintage-production-*`.


Vintage empty states and blank JSON workspace: Network/Console and unselected inspectors show centered text only. Filter reset and connection remain in the toolbar. JSON workspace has no input heading and displays the new wrapping hints.
345 Rust tests pass (0 failed/ignored); fmt, Clippy and build pass. Verified production egui paint/input; no native screenshot for this change. Evidence `docs/artifacts/ui-parity/vintage-empty-text-*`.


JSON workspace uses 4 px padding in both panes and top-left placeholder alignment. Verified typed text, wrapped hints and formatted output inset at widths 700 and 1360. 345 Rust tests pass; fmt, Clippy and build pass. Verified production egui paint/input. Evidence `docs/artifacts/ui-parity/vintage-json-padding-*`.


Square icon controls center the painted glyph on both axes at rest and on hover. UI test reproduced the Settings glyph 2 px left, then verified Settings, Help, Pause, Clear and Export. Copy icon test measures the actual glyph at 1× and 2× DPI. 346 Rust tests pass (0 failed/ignored); fmt, Clippy and build pass. Verified production egui paint and pointer input. Evidence `docs/artifacts/ui-parity/vintage-icon-center-*`.


MCP Settings refinement (2026-11-16): shared editable/read-only field geometry
with distinct opaque backgrounds; two icon+text manual-copy actions for Codex
and Claude; switch atomically saves/reconfigures MCP, Enter saves port edits.
Four F18 UI tests, F17 alignment regression and 347 total Rust tests pass;
fmt/Clippy/build pass. Production egui paint/input verified; native screenshot
not rerun. Evidence `docs/artifacts/ui-parity/vintage-mcp-*`.


Settings layout refinement (2026-11-16): text Back/Quay lại at the left edge,
Setting/Cài đặt centered with an equal-width trailing spacer. Header 44 px;
content and section margins 8 px, section gaps 12 px, no 764 px width cap.
Removed Settings Connection section and its unused URL draft. Manual URL
continues in the main toolbar. Shared font size and MCP automatic save retained.

Test-first UI failure confirmed the obsolete Connection section, then verified
header text/spacer geometry at widths 360/700/1360, fonts 14/24 and EN/VI,
full-width section bounds, scrolling and Back/Escape. Existing UI connection
checks now exercise the toolbar modal; MCP validation/persistence/copy remain
covered. 347 Rust tests pass, 0 failed/ignored; fmt, Clippy and build pass.
Production egui paint/input tested; no native screenshot or native smoke rerun
for this revision. Updated native smoke selectors. Evidence:
`docs/artifacts/ui-parity/vintage-settings-layout-*`.


Settings field widths (2026-11-16): Theme and Language use equal 220 px
widths, bounded by available space. Read-only MCP endpoint measures its current
URL at the shared font size and adds 12 px padding on each side, capped to
available space; editable port style, selection and opaque background retained.
Test-first red reproduced full-width dropdowns. Production egui test verifies
widths, URL bounds and dropdown selection at 360/1360 px, fonts 14/24, EN/VI.
348 Rust regression tests pass (0 failed/ignored); focused F18 UI 5 pass;
fmt/Clippy/build pass. No new native screenshot. Evidence
`docs/artifacts/ui-parity/vintage-settings-width-*`.


Settings header padding (2026-11-16): removed exact height; header follows
button/title content with 12px padding on all sides. Balanced title and
Back/Quay lại remain. Test-first failure confirmed the old zero margin, then
UI tests verify 12px insets, centered title/spacer, fixed header during scroll
and back navigation at 14/24px fonts. 348 regression tests pass, 0 failed/ignored;
fmt/Clippy/build pass. Production egui paint/input verified; no native screenshot.
Evidence `docs/artifacts/ui-parity/vintage-header-padding-*`.


Settings header padding follow-up: reduced 12px to 8px on all four sides.
Content-driven height and centered title retained. Updated UI assertions first
(red against 12px), then 348 regression tests pass, 0 failed/ignored.
fmt/Clippy/build pass. Production egui geometry/back/scroll tested; no native
screenshot. Evidence `docs/artifacts/ui-parity/vintage-header-padding8-*`.


Settings Back hover fix (2026-11-16): title and trailing spacer use the
reserved resting row height rather than the enlarged Back response. Header
remains content-sized with 8px padding; hover still grows Back 1.1x in place.
Test-first UI reproduced header growth from 48 to 52px. New UI test repeatedly
enters/exits hover, verifies stable header/content/title/spacer bounds, pointing
cursor, enlarged control centered in its slot and successful Back navigation
at widths 360/700/1360, fonts 14/24 and EN/VI. 349 regression tests pass,
0 failed/ignored; fmt/Clippy/build pass. Production egui paint/input tested;
no new native screenshot. Evidence `docs/artifacts/ui-parity/vintage-back-hover-*`.


Settings font-size display-only (2026-11-16): replaced DragValue with an
opaque bordered, centered display using hover-only sense and Label semantics.
It cannot focus, edit or drag. Only +/- change/persist font size; 10–24 bounds
and shared font remain. UI test first reproduced Text cursor on the editable
control, then verifies default cursor/no focus, typing/arrows/drag leave 14
unchanged, centered displayed values, button file persistence and limits.
350 Rust regression tests pass, 0 failed/ignored; fmt/Clippy/build pass.
Production egui input/paint verified; no new native screenshot. Evidence
`docs/artifacts/ui-parity/vintage-font-display-*`.


Shared display text (2026-11-16): extracted public display_text from MCP
endpoint and reused for font-size value. Both share outlined_text_edit with
editable fields for font, 12px padding and border. Display backgrounds and
content-sized widths now match. Font size is centered/non-selectable with no
focus/edit/drag; MCP endpoint stays left-aligned/selectable and immutable.
Removed the separate font-size painter. Test-first paint failure confirmed the
old padding difference; matching style test passes at fonts14/24 alongside
button-only persistence and endpoint width tests. 351 Rust tests pass,
0 failed/ignored; fmt/Clippy/build pass. Production egui paint/input tested;
no native screenshot. Evidence `docs/artifacts/ui-parity/vintage-shared-display-*`.


Vintage widget kit audit (2026-11-16): real UI test reproduced +/- at
34x44 in the production form. Icon and short-label square controls now share
square_action, owning both interaction dimensions. +/- are34x34 resting,
1.1x hovered, centered; disabled remains square without growth. Introduced
vintage::font_size_stepper used by Settings and gallery with shared display_text,
button-only edits and disabled10/24 limits. Removed gallery DragValue. Gallery
inputs now use production input_with_hint/outlined_text_edit; gallery and MCP
use the same vintage::toggle track/knob. Updated inventory to display-only
stepper and documented owners/composition wrappers in docs/vintage-widget-kit.md.

Tests first failed on34x44 and old gallery switch shape. UI tests verify
squares at font10/14/24, hover/disabled behavior, centered symbols, stationary
display/header, gallery reuse and switch1.1x geometry. Existing icon/clipboard/
JSON/dropdown/table tests retained. 354 Rust tests pass,0 failed/ignored;
fmt/Clippy/build pass. Production egui paint/input verified; no native screenshot.
Evidence `docs/artifacts/ui-parity/vintage-kit-*`.


MCP endpoint copy and consistent empty tables (2026-11-16): added reusable
display_text_with_copy composition, used by MCP endpoint. Adjacent square34px
copy action copies the exact URL, independent of manual client setup buttons;
field reserves space for the action on narrow screens. Removed Network's
early empty-store return so the real header/filter/table always render. Empty
Network and Console body regions retain their outer border. Shared EMPTY_TEXT
is exactly “Không có dữ liệu” for lists, filtered lists, unselected inspectors,
empty detail/query/body views and gallery feedback, including English UI.
Input/output JSON hints remain input guidance rather than empty-data messages.

Test-first failure reproduced missing Network header. UI tests verify retained
headers and borders for empty/filtered Network/Console, centered text, both
locales, correct native clipboard output command, square adjacent vertically
centered copy action, and field/action bounds at360px/font24. 356 Rust tests
pass,0 failed/ignored; fmt/Clippy/build pass. Production egui paint/input
verified; no native screenshot. Evidence `docs/artifacts/ui-parity/vintage-empty-copy-*`.


JSON workspace parent padding (2026-11-16): replaced asymmetric margin
(left/right/bottom20px, top4px) with8px on all sides around both panes.
Inner pane padding4px and splitter16px retained. Added output pane geometry
probe; test-first red reproduced wrong top margin. Updated UI test verifies
left/top/right/bottom bounds and pane alignment at700/1360px; existing
placeholder/input/format padding test still passes. 356 Rust regression tests
pass,0 failed/ignored; fmt/Clippy/build pass. Production egui paint/input tested;
no native screenshot. Evidence `docs/artifacts/ui-parity/vintage-json-outer-*`.


JSON/Tree guides and toolbar (2026-11-16): Code mode omits branch guides
while keeping line numbers and independent fold arrows. Tree branch/root
connectors use2px dashes/3px gaps. Search option buttons share square_toggle
with the square action primitive; navigation, clear and compact Copy use one
font-aware side calculation (24px at font14,35px at font24). This removes the
old34px Copy versus24px navigation mismatch. JSON/Tree tabs have8px horizontal
padding. Removed only the enclosing search-group stroke, preserving input
and option button borders. Applies to shared production/gallery JSON renderer.

Test-first toolbar failure reproduced missing tab padding. Revised connector
test checks no Code guides, short dashed Tree segments and independent folds.
New toolbar UI test checks both workspace/inspector at14/24px, uniform square
actions, tab padding and absence of enclosing stroke. 357 Rust regression tests
pass,0 failed/ignored; fmt/Clippy/build pass. Production egui paint/input tested;
no new native screenshot. Evidence `docs/artifacts/ui-parity/vintage-json-tools-*`.


Tree reference correction (2026-11-16): user confirmed original screenshot
as correct. Read Flutter lib/ui/json_tree_view.dart::_TreeBranches.paint
without modifying source; restored that geometry in Rust. This supersedes the
previous dashed-line interpretation. Solid lines use column4+.5 character-cell
positions, horizontal elbows2.5 cells, ancestor continuation and last-child
half-row termination. Removed extra stems from a parent's own arrow row;
child stems start on the next row. Fold arrows align to branch columns; text
keeps the depth*4+2-cell prefix spacing and a clear gap after elbows.
Vintage colors/fonts and the latest toolbar fixes retained; Code has no guides.

Test-first failure reproduced unwanted root-row stem. New UI test checks
short elbows, no parent-row stem, last-child termination and fold/reopen at
font14/24; revised independent fold test covers both modes. 358 Rust regression
tests pass,0 failed/ignored; fmt/Clippy/build pass. Production egui paint/input
verified; no new native screenshot. Evidence
`docs/artifacts/ui-parity/vintage-tree-reference-*`.


## Vintage — center JSON and Tree fold arrows

The shared viewer aligns the painted fold triangle with the visible row glyphs,
excluding search and selection backgrounds. The expanded and collapsed arrows
keep the original hit areas and Tree connector geometry. The UI test first
reproduced a 3.25px offset in JSON at 14px, then verified both modes, both arrow
directions and root/nested rows at 14px and 24px. All 359 Rust tests passed with
zero failures or skips. Format, Clippy and build passed. Production egui
paint and input were tested; no native screenshot was taken.
Evidence: `docs/artifacts/ui-parity/vintage-json-arrow-*`.


## Vintage — reserve hover tooltips for action controls

Network rows no longer show the full URL on hover; selection and the hand
cursor are preserved. Removed hover text from read-only table values, the
gallery Console row and the MCP port field. Disabled egui's automatic
tooltip for truncated static labels in form tables, the gallery and the
status bar. Action tooltips remain available.

The UI test first reproduced the URL tooltip, then verified its absence,
the Settings button tooltip, and opening a Network request. All 360 Rust
tests passed, with zero failures or skips. Format, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/vintage-action-tooltip-*`.


## Vintage — unify light backgrounds

Light theme content surfaces use `#EBF0E4` across tables, headers, the inspector,
JSON output, Settings and the gallery. Editable fields, search bars and
ordinary buttons use `#FFFFFF`. Read-only displays share the content color.
Selected state, primary actions and JSON syntax retain their existing colors.
Dark mode continues to use dark surfaces and controls.

The production UI test reproduced the old row color and now checks the exact
fills for the Network row (including hover), search bar and button, inspector,
JSON workspace and Settings. Existing widget tests verify the revised fills
without changing table or popup geometry. All 361 Rust tests passed, with
zero failures or skips. Format, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/vintage-green-background-*`.


## Vintage — move the complete JSON workspace toolbar into the header

The JSON workspace places Format, Find, case/word/regex, match count,
previous/next, Clear, JSON/Tree, Copy and Export on one header row. The
result starts at its pane inset and uses the released space for content.
The search width adapts to the available space, and the row scrolls
horizontally when necessary. Inspectors still use their local toolbar
with the shared find controls.

A test first reproduced the case button outside the header. The production UI
test now checks row alignment and functional search, case, navigation, view and
clear actions at width 700/font 14 and width 1360/font 24. Existing tests cover
Copy/Export, shortcuts, fold state and inspector behavior. All 362 Rust tests
passed with zero failures or skips; format, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/vintage-json-header-controls-*`.


## Vintage — tint formerly white widget backgrounds

Changed the shared light control background from `#FFFFFF` to `#E6EBDF` for
inputs, search bars and ordinary buttons. JSON color preview backings use the
same tint; the data colors themselves are unchanged. The main content remains
`#EBF0E4`.

The updated production UI test failed against the old white search bar and now
verifies the new exact color on controls and JSON swatch backings. It also
checks untouched content surfaces and a literal white JSON sample. All 362
Rust tests passed with zero failures or skips; format, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/vintage-tinted-controls-*`.


## Vintage — control background #D9DED3

Replaced the shared control background #E6EBDF with #D9DED3 and updated
the gallery palette label and widget inventory. The production UI fill test
first failed against the previous color and now passes for search, buttons,
inputs and color preview backings. All 362 Rust tests passed with zero failures
or skips; format, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/vintage-d9ded3-*`.


## Vintage — control background #E0E5D9

Replaced #D9DED3 with #E0E5D9 in the shared light control palette. The UI fill test failed before the change and passed after it. All 362 tests, format, Clippy and build passed. Evidence: `docs/artifacts/ui-parity/vintage-e0e5d9-*`.


## Vintage — visible header action borders (2026-11-17)

Added a production UI test for action borders in both header rows: 14px at
1360px and 24px at 2200px, with empty and populated JSON. It first reproduced
faded disabled outlines, then caught Export's right edge extending past the
toolbar clip rectangle. Shared square controls now retain opaque outlines
while disabled icon/text remain dim. Copy/Export no longer nest disabled UI
scopes. Increased the toolbar width reservation to keep the rightmost border
inside the scroll viewport. Disabled Copy/Export still do not act.

All 363 Rust tests passed (zero failed/ignored), including all 78 desktop UI
tests. Format, Clippy and build passed. Toolchain and dependency cache were
restored after the temporary directories had been removed. Validation uses
production egui paint/input; no native screenshot was taken.
Evidence: `docs/artifacts/ui-parity/vintage-header-border-*`.


## Vintage — equal Settings section padding (2026-11-17)

Settings section headings no longer inherit the 44px input row height.
Heading rows use their actual font/icon height; the MCP title/switch row fits
the switch. The shared font-size stepper row fits the display and square
buttons instead of retaining a 44px minimum. This removes extra whitespace
above titles and below the Interface controls while preserving the shared
8px Frame padding and 1px border.

Test-first production UI measurement reproduced top 21px versus bottom 10.84px
at the Interface frame. The new test measures painted content against all
three section frames, verifying 9px including the border at width360/font14,
width1200/font14 and width1200/font24. All 364 Rust tests passed, with zero
failures or skips; format, Clippy and build passed. No native screenshot taken.
Evidence: `docs/artifacts/ui-parity/vintage-settings-section-padding-*`.


## Vintage — match Settings heading icon and title heights (2026-11-17)

The shared section heading measures the actual title galley height and uses it
for the icon font size and row height. Removed the fixed20px icon size.
Interface, MCP and App updates now have matching icon/title line heights and
vertical centers as the configured text size changes.

The UI test first reproduced icon20px versus title16px at font14. Production
paint checks now pass for all three headings at font14/24; equal section
padding tests also pass. All 365 Rust tests passed with zero failures/skips;
format, Clippy and build passed. No native screenshot was taken.
Evidence: `docs/artifacts/ui-parity/vintage-settings-heading-icon-*`.


## Vintage — remove Help copy and localize empty states (2026-11-17)

Removed the Copy address icon from the Help footer while preserving Go to
Discussion and Close. Replaced the fixed Vietnamese empty-state constant
with vintage::empty_text(ctx), using the existing No data translation key:
English displays No data; Vietnamese displays Không có dữ liệu. Shared list,
inspector, metadata, body and gallery empty states now follow the chosen locale.

Test-first failures reproduced the extra Help copy action and Vietnamese text
under English. Production UI tests verify the remaining Discussion URL action,
absence of Copy, EN/VI filtered tables and empty detail tabs, and centered
unfiltered empty panes. All 365 Rust tests passed with zero failures/skips;
format, Clippy and build passed. No native screenshot taken.
Evidence: `docs/artifacts/ui-parity/vintage-help-localized-empty-*`.


## Vintage — silent update probe and Settings update label (2026-11-17)

Removed the duplicated idle/status Check for updates line below the version.
The button keeps Check for updates until an update is available, then shows
the requested Update lastest version (Vietnamese: Cập nhật phiên bản mới nhất).
Busy/download/install statuses and progress remain available.

The native host already runs one startup check. macOS startup now uses Sparkle's
checkForUpdateInformation probe rather than the background flow that can offer
an update immediately. Windows no longer promotes background availability to
an automatic UI check. Available results update Settings without a toast;
the manual button initiates the native update UI. Existing silent offline,
current-version and timeout behavior is preserved.

Test-first failures reproduced the duplicate status and automatic Windows UI
check. Production UI tests verify EN/VI button labels, no duplicated line,
no Available toast and action dispatch; backend tests verify explicit manual
transition and progress. All 366 Rust tests passed with zero failed/ignored;
format, Clippy and build passed, including the Objective-C bridge compilation.
No live release installation or native screenshot was performed.
Evidence: `docs/artifacts/ui-parity/vintage-background-update-label-*`.


## Vintage — shared four-pixel icon/text gap (2026-11-17)

Added desktop_style::ICON_TEXT_GAP =4px and applied it to section headings,
header action LayoutJobs, navigation tabs, shared pills, MCP copy buttons,
search icon/input, Help rows, notification headings and standard checkbox/
dropdown icon spacing. Header and MCP labels now use pixel leading_space
instead of literal spaces; hover preserves the4px icon/text gap.

Test-first production UI check reproduced the space-based header layout.
The new test measures actual glyph advances and label positions at14/24px,
including hovered header/MCP controls; existing search geometry expectations
now check4px. All 367 Rust tests passed with zero failed/ignored; format,
Clippy and build passed. No native screenshot was taken.
Evidence: `docs/artifacts/ui-parity/vintage-icon-text-gap-*`.


## Vintage — Find & connect label (2026-11-17)

Renamed the connection scan action to “Find & connect” (VI: “Tìm & kết nối”)
and updated Help and the widget gallery. Action dispatch and probe IDs stay intact.
The production header UI assertion failed against the old label, then passed.
All 367 Rust tests passed with zero failed/ignored; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/vintage-find-connect-*`.


## Ventura — shared light/dark system-inspired palette (2026-11-17)

Centralized fixed sRGB theme, JSON, notices and search colors in src/app_colors.rs.
Light uses ECECEC/FFFFFF surfaces and007AFF accent; Dark uses282828/1E1E1E
surfaces,3A3A3A controls and0A84FF accent. Selected content uses white text;
read-only displays retain a distinct gray background. Square geometry, Cascadia
Mono, the shared font size and hover behavior remain unchanged. The gallery
uses the active palette, and production JSON preserves seven distinct roles.
Removed method_color/status_color; console error text now uses the shared
semantic Error color (an actual remaining caller missed in the earlier inventory).

Test-first production UI check failed on the old palette. Tests now exercise
Light/Dark controls, blue row selection with white text, all four notice colors
and all seven JSON roles. Existing geometry, focus, dropdown, field and gallery
UI tests remain active with updated palette expectations. All368 Rust tests
passed with zero failed/ignored; fmt, Clippy and build passed. No native
screenshot or OS semantic-color API integration was performed.
Evidence: `docs/artifacts/ui-parity/ventura-palette-*`.


## UI — remove hover scaling (2026-11-17)

Removed shared1.1x hover growth, scaled font/spacing changes and the overlap
paint layer. Actions now render at their resting dimensions with an opaque
backing; enabled click actions retain the hand cursor and normal behavior.
Dropdowns, controls, icons, switches and stepper actions keep their normal
rectangles/fonts. Image preview zoom remains an explicit image action.

Test-first production gallery check failed on the old growing rectangle.
Updated real renderer tests cover unchanged fonts and rectangles, centered
icons at1x/2x DPI, stable neighbors/header, Settings back and +/- controls,
opaque non-overlapping dropdowns and actual click/copy behavior. Existing
cursor and tooltip regressions pass. All368 Rust tests passed with zero
failed/ignored; fmt, Clippy and build passed. No native screenshot was taken.
Evidence: `docs/artifacts/ui-parity/vintage-no-hover-scale-*`.


## UI — Geist interface and Cascadia Mono JSON/Tree (2026-11-17)

Bundled Google Fonts Geist variable TTF and its SIL OFL license for offline UI
rendering. Proportional and named UI fonts use Geist; JSON/Tree content, line
numbers and the JSON draft editor retain Cascadia Mono. General tables, console,
raw text, controls and the gallery now use Geist. Both text families share the
existing font-size setting; icon glyphs remain in Material Icons.

Geist's taller line metrics exposed non-square text actions and header alignment
changes. Shared square actions now allocate exact rectangles and paint centered
text/icons; header button and JSON find padding fit the measured UI line height.
This preserves square controls, stable headers and aligned JSON toolbar controls
without hover scaling.

Test-first UI check failed against Cascadia as the UI family. New production
renderer tests verify bundled-family precedence, Vietnamese Geist glyphs, actual
header text and JSON/Tree families at14/24px in both themes. Existing focus,
square-action, scrolling-header, tooltip, cursor, clipboard and toolbar geometry
regressions pass. All369 Rust tests passed with zero failed/ignored; fmt, Clippy
and build passed. No native screenshot was taken.
Evidence: `docs/artifacts/ui-parity/geist-ui-cascadia-json-*`.


## UI — wrapped inspector metadata with fixed Title width (2026-11-17)

The shared key/value table used by Overview and request/response headers now
fits its viewport without an internal horizontal ScrollArea. Title is160px,
shrinking only when necessary to retain value space in very narrow panes.
Values take the remaining width and wrap, including unbroken URLs and explicit
newlines. Titles can wrap too. Row heights derive from the actual Geist galleys;
Title stays left aligned, Value right aligned, and square Copy actions centered.
Header and outer/grid borders remain continuous, values stay selectable, and
Copy retains the complete original value. Enclosing inspector scroll is vertical.

Test-first renderer check reproduced a1074px table in a280px viewport. New
shared-table UI checks cover280/520/960px, fonts14/24, both themes, with/without
Copy, full text bounds, wrapped rows, alignment, contiguous grid, horizontal
wheel stability and actual copy commands. Production inspector checks cover
long URLs and request headers in the card. All371 Rust tests passed with zero
failed/ignored; fmt, Clippy and build passed. No native screenshot was taken.
Evidence: `docs/artifacts/ui-parity/wrapped-metadata-tables-*`.


## UI — preserve and color the raw query; left-align values (2026-11-17)

Value cells in the details tables align left; the shared metadata table keeps
its 160 px Title column, wrapping, and centered copy actions. Overview extracts
the original query substring before the fragment without decoding or reordering.
Only text color changes: JSON key, string, number, boolean and null colors apply
to parameters, while delimiters use normal text. Copy preserves the exact query.

The UI test failed first with the old pretty-printed JSON, then verified rendered
text, colors, wrapping and clipboard output across both themes, English and
Vietnamese, and 14/24 px. All 372 Rust tests passed; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/raw-colored-query-*`.


## UI — split the Overview URL into four fields (2026-11-17)

Overview starts with Method, Domain (host and optional explicit port), Endpoint
(raw path), and Query params. Query colors, raw encoding and copy behavior remain.
The full URL above the table is unchanged. Added Vietnamese labels for the new fields.

The renderer test failed first on the combined URL row, then passed with the
new order, encoded paths and queries, IPv6 with port, relative paths, and
per-row clipboard output. The query syntax and wrapping regressions still
pass. All 373 Rust tests passed; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/overview-url-split-*`.


## UI — shared JSON/Tree segmented selector (2026-11-17)

The workspace and inspector now use one reusable segmented selector. It paints
a single square blue outline and one divider, with separate clickable JSON and
Tree areas, 8 px horizontal label padding, and a blue selected segment.

The renderer test failed first because both former buttons owned borders. It
now checks the single outline, divider, selection in both directions and cursor
in the workspace and inspector across light and dark themes. All 373 tests
passed; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/json-tree-segmented-control-*`.


## UI — full-width inspector metadata tables (2026-11-17)

Overview, Request header and Response header render their tables directly in
the vertical viewport. Removed the extra card border and its 12 px padding;
the table retains its grid, outer border, cell padding and copy actions.

The renderer test failed first on the inset, then verified that all three
tables reach the viewport edges and paint only one enclosing border. Wrapping
and copying still pass. All 373 tests passed; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/full-width-metadata-table-*`.


## UI — keep HTTP scheme in the Overview Domain row (2026-11-17)

Domain displays and copies the captured scheme, host and any explicit port.
Credentials are omitted and no scheme is added to scheme-relative URLs.
The UI test reproduced the missing scheme, then passed for HTTP, HTTPS, IPv6,
explicit ports, and relative URLs. All 373 tests passed; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/overview-domain-scheme-*`.


## UI — Ventura horizontal inspector scrollbar (2026-11-17)

The inspector tab strip now has a local Ventura-style overlay scrollbar:
a rounded gray 5 px thumb, expanding to 9 px on hover, with a subtle hovered
track. It fades when idle, appears only on overflow, and supports dragging to
hidden tabs. Tab controls keep their square style; metadata still wraps.

Added a renderer test before implementation (missing scrollbar reproduced).
Pointer-driven tests verify dragging to the final tab, unchanged selection and
strip height, no bar when tabs fit, and both light/dark themes.
All 374 tests passed; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/ventura-scrollbar-*`.


## UI — keep the inspector scrollbar below the tabs (2026-11-17)

Reserved a lane below the tabs for the full hover height of the Ventura-style
scrollbar. The UI test reproduced the overlap, then verified clearance for the
thumb and expanded scrollbar, dragging to the final tab, and no bar when tabs
fit. All 374 tests passed; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/ventura-scrollbar-clearance-*`.


## UI — standardize icon buttons (2026-11-17)

Standalone icon controls now use a 24 × 24 pt hit area and 14 pt glyphs,
independent of the interface font size. This includes inspector and JSON copy,
search actions, table filter buttons, dialog controls and the settings stepper.
Text controls continue to follow the interface font size.

The UI tests first reproduced the 34 pt copy and stepper buttons, then checked
fixed dimensions, glyph size and centering, hover stability, and working copy
and filter actions. All 375 tests passed; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/icon-button-size-*`.


## UI — keep text input borders visible (2026-11-17)

Native inputs and composite editors use a shared idle and focus stroke. The
native TextEdit focus override is local to the text field, preserving selected
dropdown text colors. Read-only display fields paint one opaque frame.

The UI test reproduced the missing focus border in the inspector JSON search.
Coverage verifies the rendered outline during typing and after focus changes,
including Network and Console search, JSON inspector and Beautify, Manual URL,
export paths, MCP fields, and gallery inputs. Light and dark production themes
passed. All 375 tests passed; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/input-borders-*`.


## UI — tighten inspector JSON spacing (2026-11-17)

Removed the extra space around the inspector JSON toolbar. The scrollbar lane
ends 6 pt above the toolbar, and the first JSON or Tree row starts 12 pt below
it (6 pt layout gap and 6 pt content inset). The reserved scrollbar lane and
other inspector content spacing are unchanged.

The UI test first measured the old 18 pt gap above the toolbar, then verified
both new distances in JSON and Tree views. All 375 tests passed; fmt,
Clippy and build passed. Evidence: `docs/artifacts/ui-parity/inspector-json-spacing-*`.


## UI — enlarge table header filter icons (2026-11-17)

Network and Console table header filters retain 24 × 24 pt button frames
and use centered 20 pt arrow icons. Other icon button sizes are unchanged.

UI tests verify the frame and glyph size, column title clearance, and working
filter menus. All 375 tests passed; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/table-filter-size-*`.


## UI — join Console table header and rows (2026-11-17)

Removed the 6 pt gap below the Console table header. Rows and filtered empty
results meet the header edge. Scrolling row paint stays below the fixed header.

The UI test reproduced the gap before the fix and now checks contiguous rows,
the shared header/body edge, scroll clipping and the filtered empty state.
All 376 tests passed; fmt, Clippy and build passed. Evidence: `docs/artifacts/ui-parity/console-table-seam-*`.


## UI — wrap complete Console messages (2026-11-17)

The Console Message column wraps complete log text within the cell and
preserves explicit line breaks. Each row grows as needed. Cached row heights
keep the scroll layout accurate while only visible rows are painted.

The input-driven UI test reproduced the truncated message and now verifies
long unbroken tokens, a final Unicode line, contiguous variable-height rows,
and reflow at a narrower width. All 377 tests passed; fmt, Clippy and build
passed. Evidence: `docs/artifacts/ui-parity/console-message-wrap-*`.


## UI — share the Console and Network inspector components (2026-11-17)

The inspectors share their frame, heading row, primary tabs and tab spacing.
Console Detail presents structured log fields and expandable JSON messages;
Raw renders the original record and is still serialized only on demand.
Both use the same JSON/Tree viewer, search and clipboard controls as Network.
The shared viewer wraps its toolbar when panel width is limited.

UI tests cover geometry and control access in wide, narrow and modal layouts,
JSON/Tree and clipboard behavior, and independent search state. All 378 tests
passed; fmt, Clippy and build passed. Evidence: `docs/artifacts/ui-parity/shared-inspector-*`.


## UI — Console Detail metadata table (2026-11-17)

Removed the Console inspector heading row. Detail uses the shared Network
Title / Value / Action table and metadata viewport, with full wrapped values
and per-row Copy. Raw retains the shared JSON/Tree viewer and lazy serialization.
Detail preserves cleaned message text and explicit line breaks without parsing
JSON-looking messages into a different representation.

Test-first UI failure reproduced the missing table. Tests verify heading removal,
exact copied Message, shared table dimensions and spacing at modal/desktop widths
and font 24, plus Raw JSON/Tree/search/clipboard behavior. All 378 regression tests
and 12 focused Console UI tests passed; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/console-detail-table-*`.


## UI — Resize Console columns (2026-11-17)

Drag any of the three header dividers to resize its adjacent columns. The header
and rows share the resulting edges; Message wraps to its new width. Minimum
widths keep the filters and Message usable when shrinking the list.

The input-driven UI test first failed on the missing drag handle. It now checks
the resize cursor, every divider, adjacent widths, Message reflow and the Source
filter after resizing. All 379 regression tests and 13 focused Console UI tests
passed; fmt, Clippy and build passed. Evidence: `docs/artifacts/ui-parity/console-column-resize-*`.


## UI — Network filter ALL labels (2026-11-17)

The Method and Status dropdowns display ALL for their unfiltered option in both
English and Vietnamese. Existing filter values and behavior are unchanged.

The UI test first failed on the old label, then passed for both languages and
both filters. Of 380 regression tests, one socket test timed out on the initial
run and passed when its suite was rerun. Fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/network-filter-all-*`.


## UI — Console Message tag filter (2026-11-17)

The Message header has a fixed-size popup with a gray input/Plus header
and a scrollable list of removable tag cards. Tags match case-insensitive
substrings against the displayed Message only. A match against any tag is
required when tags are present, alongside the other Console filters.

The input-driven UI test reproduced the missing control before the change and
now covers adding by click/Enter, duplicates, matching, deletion, filter reset
and scrolling a full tag list without changing the popup size. All 381 tests
passed; fmt, Clippy and build passed. Evidence: `docs/artifacts/ui-parity/console-message-tags-*`.


## UI — Correct Console tag icons (2026-11-17)

Use the bundled font's Add and Close glyphs for tag actions. The font lookup
confirmed the old codes were Cast connected and Snowboarding. The UI test
first reproduced the wrong icon and now checks both centered 14 pt glyphs in
24 × 24 pt buttons. All 381 tests passed; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/console-tag-icons-*`.


## UI — Wrap Console tag cards horizontally (2026-11-17)

Console tag cards have a gray border and size to the label and close button,
up to the width of the popup. Cards flow from the left and wrap with 8 pt
horizontal and vertical gaps. The popup stays fixed while its list scrolls.

The UI test covers shrink-to-fit widths, wrap alignment, gaps, gray borders,
and deleting an item after scrolling. It reproduced the missing card layout
before the change. Of 381 regression tests, one socket test timed out on the
initial run and passed when its suite was rerun. Fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/console-tags-wrap-*`.
