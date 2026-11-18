# Trạng thái port

## F01 — Capture store (hoàn thành core)

Đã khảo sát source và tạo ma trận 21 tính năng, catalog test gốc và snapshot hash.
Đã triển khai Rust library `src/capture.rs`, fixture copy nguyên từ Flutter và
16 integration tests `tests/capture_store.rs`.

Validation trên macOS ARM64, Rust 1.99.1:
- `cargo test --locked --offline`: 16 pass, 0 failed, 0 ignored.
- `cargo fmt --check`: pass.
- `cargo clippy --all-targets --locked --offline -- -D warnings`: pass.
- Cargo.lock đã lưu để tái lập dependency.

Toàn app chưa được
port xong; các mục F09–F21 vẫn pending, chưa có UI hoặc kết nối VM.

Tương thích hành vi cần lưu ý:
- Replay network giữ một record, nhưng console vẫn append giống Flutter.
- Clear giữ trạng thái Pause và ID log tăng dần, reset counters/data/bytes.
- Giới hạn session chỉ xóa metadata không còn record; không xóa attribution.
- Capture byte budget đo JSON UTF-8, không phải tổng heap allocation.
- revision tăng khi Clear/remove session giống nguồn; UI về sau cần cơ chế
  thông báo ingest riêng, không dùng revision như event sequence.

## F02 — JSON document/format/Tree/cache

Hoàn thành core `src/json.rs`, export qua `src/lib.rs`. Không sửa source Flutter.
Đã đọc source JSON Flutter qua CodeGraph context/explore và đọc các file cụ thể
không được trả trong output bị cắt. Tham chiếu `json_document.dart`,
`native_json_document.dart`, `json_document_test.dart`, `native_json_document_test.dart`.

TDD: viết `tests/json_document.rs` trước triển khai. Lần chạy red sau khi sửa
cú pháp raw string báo thiếu module `flutter_network_viewer::json`. Sau đó thêm
implementation và sửa annotation kiểu sum trong test; 13 test thực thi pass.
Không có ignored/placeholder tests.

Các contract đã phủ:
- Object/array/scalar/null và collection rỗng; pretty 2 spaces, giữ key order.
- Invalid JSON trả raw nguyên vẹn, message, dòng/cột 1-based; cột UTF-16.
- Text/index lazy qua OnceLock; immutable value, cache trả Arc dùng chung.
- Paths phân biệt key/index; keyStart/valueStart/end UTF-16; escaped keys,
  non-BMP Unicode, code line offsets và Tree text mappings.
- Tree unquoted keys, array indexes, branch continuation, depth/count/fold;
  folding chỉ đổi visible indexes, không mất descendants hoặc text.
- Cây 5.000 object có 10.002 rows, fold indexes đúng; width giống Flutter
  (ASCII 1 cell, mỗi Unicode code point khác ASCII 2 cells).
- Cache exact-source key, LRU theo read, entry/byte budget, oversized/disabled,
  parse lỗi không mutate budget; clear thả ownership (kiểm tra Weak).

Validation macOS ARM64 ngày 2026-11-12:
- `cargo test --locked --offline`: 29 pass (F01 16 + F02 13), 0 fail/ignored.
- `cargo fmt --check`: pass với toolchain tạm (PATH mặc định không có cargo).
- `cargo clippy --all-targets --locked --offline -- -D warnings`: pass.
- Không thêm dependency; Cargo.lock giữ nguyên.

Giới hạn/phạm vi còn lại:
- Chưa có GUI; F15/F16 vẫn pending. Styled syntax spans/native host thuộc UI;
  color swatches thuộc F07. Catalog ghi partial cho các test gốc chứa các mục này.
- Cache budget là estimate theo UTF-16 giống Flutter (`4096 + length * 64`),
  không phải tổng heap đo thực tế. Arc ngoài cache vẫn giữ document sau clear.
- Parser dùng serde_json với nesting limit mặc định; deep JSON vượt giới hạn
  trả ParseError/raw thay vì panic. Không tuyên bố full native/UI parity.

## Handoff F02 → F03 (đã thực hiện)

Đọc AGENTS.md và docs, khảo sát json_find_controller.dart/json_find_test.dart
bằng CodeGraph projectPath Flutter. Viết contract tests trước rồi implement.
Dùng JsonDocument/Presentation.nodes có UTF-16 offsets và typed JsonPath;
query rỗng phải giữ lazy (không gọi text/presentation), query không rỗng mới
prepare. Test case/whole word/regex, invalid regex/zero-width, next/prev wrap,
Unicode offsets, key/value paths, unfold ancestors và mode-state. UI keyboard
focus/Cmd+F giữ F15, không kết luận UI pass từ core tests. Cập nhật catalog,
regression/fmt/clippy và mở đúng một chat F04 nếu pass.

## F03 — JSON find (hoàn thành core)

Triển khai `src/json_find.rs`, export `json_find`; thêm getter
`JsonDocument::is_text_prepared()` để kiểm chứng serialization lazy. Chỉ sửa thư
mục Rust. Khảo sát Flutter qua CodeGraph context rồi explore; output explore
không có test file nên đọc trực tiếp `test/json_find_test.dart` để đủ contract.

TDD: viết 11 integration tests trước implementation; sửa raw-string delimiters
trong test rồi chạy red: thiếu module json_find và getter is_text_prepared.
Implementation pass 11 tests; thêm 2 regression tests cho 5.000 object và hồi
phục regex lỗi. Tổng F03 có 13 tests thực thi, không ignored/placeholder.

Phủ case insensitive/sensitive, literal escaping, whole-word, ECMAScript regex
(multiline, lookaround, backreference), regex lỗi và bỏ zero-width; next/previous
wrap với delta âm/dương và MIN/MAX; UTF-16 offsets cho non-BMP, tiếng Việt và
escaped keys; typed key/index paths, punctuation/closing container về ancestor;
reveal code/Tree chỉ mở ancestor cần thiết; giữ query/options/index khi đổi mode,
đổi document giữ query/options nhưng reset active; no results; query rỗng hoặc
regex không compile giữ cả text và presentation lazy. Large test gán đúng 5.001
matches và reveal descendant cuối trong 5.000 objects.

Thêm dependency `regress 0.12.0` (ECMAScript, feature utf16), cập nhật Cargo.lock.
Whole-word giữ hành vi Flutter: boundary xem một UTF-16 unit với nhóm L/N/_, nên
surrogate halves được coi là boundary; có regression test rõ ràng. Search trả
path theo vị trí bắt đầu của match như source; không quét toàn bộ nodes riêng
cho mỗi match (sort một lần, cursor tiến và lùi qua parent).

Validation macOS ARM64, Rust 1.99.1 ngày 2026-11-12:
- `cargo test --locked --offline`: 42 pass (16 + 13 + 13), 0 fail/ignored.
- `cargo fmt --check`: pass.
- `cargo clippy --all-targets --locked --offline -- -D warnings`: pass.
- Cập nhật mapping 8 test gốc json_find: 2 covered, 4 partial, 2 UI pending.

Chưa triển khai styled highlight spans, render/counter layout, scroll, keyboard
focus Cmd/Ctrl+F hoặc body/inspector UI. Các mục này vẫn F15/F16 (inspector F13).
Core test không chứng minh GUI parity. Không publish; Rust chưa là Git repo.

## Handoff F03 → F04 (đã thực hiện)

Đọc AGENTS.md/progress/matrix trong Rust. Khảo sát Flutter console model và
`flutter_console_test.dart` bằng CodeGraph với projectPath Flutter; context rồi
explore và chỉ đọc file cụ thể nếu output thiếu. Viết executable tests trước code.
Chỉ làm core F04: ANSI/Flutter display cleanup nhưng raw giữ nguyên, JSON block/
Flutter exception grouping, không ghép khác session/isolate/source hoặc quá 2s,
limits 200 dòng/64K, search logger/error/stack/args, severity. Xem CaptureStore
ConsoleRecord hiện có để tái sử dụng; UI console thuộc F14. Cập nhật catalog,
chạy regression/fmt/clippy rồi mở đúng một chat F05 nếu pass. Nếu bị chặn ghi rõ,
không tạo chuỗi chat lỗi. Dùng CARGO_HOME/RUSTUP_HOME/PATH toolchain tạm như trên.

## F04 — Console model và nhóm log (hoàn thành core)

Triển khai `src/console.rs` dưới dạng methods trên `capture::ConsoleRecord`,
export module qua `src/lib.rs`. Không đổi schema capture/export; không sửa Flutter.
Đã dùng CodeGraph context rồi đúng một explore cho ConsoleRecord và toàn bộ
`flutter_console_test.dart`, không cần đọc lại source Flutter.

TDD: viết 14 executable tests trong `tests/console_model.rs` trước implementation.
Lần red báo thiếu các API ConsoleRecord; sau implementation cả 14 pass, không
ignored/placeholder. Phủ CSI ANSI, Flutter prefix và CR theo từng dòng nhưng raw
export nguyên vẹn; JSON object/array/nested Unicode, exception kết thúc ít nhất
8 ký tự ═ và promote level error; ordinary/incomplete/invalid không ghép;
session/source/isolate boundaries; window tuyệt đối từ dòng đầu 2.000 ms cho
phép, 2.001 ms từ chối; invalid/missing time không chặn giống nguồn.

Giới hạn giống Dart: tối đa 200 record (không phải physical lines trong một
record) và 65.536 UTF-16 units gồm newline, có test 200/201 và emoji ở biên budget.
Synthetic entry giữ id/session/metadata đầu, chứa rawEvents nguyên gốc và không
mutate input. Search gồm display, source label, logger, explicit error/stack hoặc
typed args fallback và JSON args. Level mặc định log, numeric severity giữ độc
lập; chuyển VM numeric level sang named level thuộc adapter F09.

Validation macOS ARM64 ngày 2026-11-12, toolchain tạm:
- `cargo test --locked --offline`: 56 pass (F01 16 + F02 13 + F03 13 + F04 14),
  0 failed, 0 ignored.
- `cargo fmt --check`: pass.
- `cargo clippy --all-targets --locked --offline -- -D warnings`: pass.
- Không thêm dependency; Cargo.lock không đổi.
- Catalog 4 test Flutter core covered; test widget còn partial F14.

F14 vẫn pending: console list/filters/selection/inspector/copy/layout và GUI tests.
Không suy ra UI parity hoặc GUI performance từ các core tests này.

## Handoff F04 → F05 (đã thực hiện)

Đọc AGENTS.md/progress/matrix trong Rust; chỉ viết Rust tại
`/Users/quocan/Github/flutter_network_viewer (rust)`. Khảo sát form_body và
form_body_view cùng test Flutter qua CodeGraph projectPath Flutter: context rồi
một explore, đọc file cụ thể khi output thiếu. Test executable trước code.
URL encoded repeated/empty keys, plus/Unicode, multipart binary/extended filename,
MIME case insensitive, truncated giữ remainder, malformed boundary/base64/percent
không panic, raw nguyên vẹn và omitted reason. Chỉ core F05; UI form view F16
pending. Cập nhật catalog/docs, chạy tất cả regression/fmt/clippy, chỉ mở đúng
một chat F06 khi pass. Toolchain CARGO_HOME=/private/tmp/fnv-cargo,
RUSTUP_HOME=/private/tmp/fnv-rustup, PATH=/private/tmp/fnv-cargo/bin:$PATH;
ưu tiên --locked --offline. Không publish, Rust chưa là Git repo.

## F05 — Body form-data (hoàn thành core)

Triển khai `src/form_body.rs`, export qua `src/lib.rs`, không sửa Flutter.
Đã dùng CodeGraph context rồi một explore với projectPath Flutter. Explore
trả toàn bộ parser, view bị cắt và không có tests; đọc trực tiếp hai file
`test/form_body_test.dart`, `test/form_body_view_test.dart` và phần view còn thiếu.

TDD: viết 13 executable tests `tests/form_body.rs` trước implementation. Lần
red thiếu module form_body; sau implementation 12/13 pass, sửa fixture base64
trong test (đã encode nhầm `%2+` thay vì `%2B`), sau đó 13/13 pass. Không có
ignored hoặc placeholder tests. Không thêm dependency, Cargo.lock không đổi.

Phủ URL encoded thứ tự/key lặp/key rỗng/giá trị rỗng/flag/plus/Unicode và UTF-8
byte lengths; multipart nhị phân không decode attachment sang text, metadata
filename/MIME/size, delimiter giả trong attachment, filename UTF-8 mở rộng và
fallback, quoted semicolon/backslash, tên header/parameter/MIME không phân biệt
hoa thường nhưng boundary giữ case. Truncated giữ fields đã parse và remainder;
missing disposition bỏ part và báo incomplete. Text hỗ trợ Latin1 và UTF-8 lỗi
thay replacement; base64 chấp nhận whitespace/missing padding/URL-safe alphabet.
Parser chặn 1.000 parts và header scan 65.536 bytes giống nguồn. Malformed
boundary/base64/percent hoặc input thiếu/sai kiểu trả incomplete; URL decode lỗi
bỏ toàn bộ fields giống Flutter. Không mutate input/raw; FormBody chỉ giữ metadata
và text fields, không giữ bản sao attachment. OmittedBodyReason phân biệt size
limit (giữ limit bytes) và read failure khi không có body.

Validation macOS ARM64, Rust 1.99.1 ngày 2026-11-12, toolchain tạm:
- `cargo test --locked --offline`: 69 pass (16 + 13 + 13 + 14 + 13), 0 fail/ignored.
- `cargo fmt --check`: pass.
- `cargo clippy --all-targets --locked --offline -- -D warnings`: pass.
- Catalog 6 parser tests covered; 3 form-body view test entries partial, UI F16 pending.

F16 còn pending: Raw/Form toggle, render file metadata/fields, invalidation khi
body/MIME cập nhật và notice EN/VI. Core enum reason chưa phải thông báo UI.
Không kết luận GUI parity từ unit tests; không publish; Rust chưa là Git repo.

## Chat tiếp theo: F06 — Body image

Đọc AGENTS.md/progress/matrix trong Rust; chỉ ghi tại
`/Users/quocan/Github/flutter_network_viewer (rust)`. Khảo sát image_preview và
native_image_view cùng tests Flutter bằng CodeGraph projectPath Flutter:
context rồi một explore, đọc file cụ thể khi output thiếu. Viết executable tests
trước implementation. Phân loại base64/data URI/JSON string và MIME thiếu;
bytes lỗi fallback raw, giữ input; metadata/dimension/decode và cache ownership
nếu thuộc core. UI preview/enlarged chia sẻ bytes, zoom/pan/fit, response update
và không reload khi zoom cần GUI test, giữ F16 pending nếu chưa có GUI. Không
đánh dấu UI pass chỉ từ core tests. Cập nhật catalog/docs, chạy toàn bộ regression,
fmt/clippy rồi mới tạo đúng một chat F07 nếu pass. Không spawn subagent, không
publish. Bắt buộc CARGO_HOME=/private/tmp/fnv-cargo,
RUSTUP_HOME=/private/tmp/fnv-rustup, PATH=/private/tmp/fnv-cargo/bin:$PATH;
ưu tiên --locked --offline. Rust chưa là Git repo; không sửa source Flutter.

## F06 — Body image (hoàn thành core, GUI và large-image parity còn pending)

Triển khai `src/image_body.rs`, export qua `src/lib.rs`; tái sử dụng helper
base64 của F05 bằng visibility crate-private. Chỉ sửa Rust. Khảo sát Flutter
bằng CodeGraph context rồi đúng một explore. Đọc trực tiếp các phần output
thiếu: image_preview_test, network_body_section, native decoder Swift/WIC.

TDD: viết 15 executable tests `tests/image_body.rs` trước implementation;
chạy red báo unresolved module image_body. Sau implementation, allocation-guard
test phát hiện giới hạn max_alloc của codec chưa đủ chặn frame lớn: thêm kiểm
tra total_bytes và width*height*4 trước decode. Thêm 3 tests cho codec/frame đầu,
EXIF và concurrent owners. Test EXIF ban đầu dùng fixture có metadata nên kỳ vọng
pixel sai; thay bằng JPEG tự encode không metadata rồi chèn APP1 xác định.
Cuối cùng 18/18 pass, không ignored hoặc placeholder.

Contract đã phủ:
- Capture headers case-insensitive/list, bodyPreview fallback, bodyEncoding;
  pure isolate envelope body/image/base64, sniff prefix tối đa 128 UTF-16 units.
- PNG/JPEG/GIF87a/GIF89a/BMP/WebP signature; MIME thiếu/text MIME, base64 có
  whitespace, URL-safe/missing padding, transport bọc base64, JSON string,
  data URI base64/percent; ordinary JSON/text/binary vẫn để Raw.
- Malformed base64/data URI giữ image intent và không panic; corrupt signature
  decode lỗi được cache cục bộ. Input/raw capture không mutate.
- Pixel decode thực cho PNG/JPEG/GIF/BMP/WebP; frame đầu GIF; EXIF orientation
  sau resize; không upscale, cạnh dài tối đa 4096, RGBA8 cho texture về sau.
- ImageAsset giữ Arc bytes và OnceLock pixel result/error; nhiều owner/thread
  dùng cùng một result, không global image cache. Last owner drop giải phóng
  resource; response mới có asset độc lập. GUI về sau gọi pixels trên worker.

Thêm `image 0.25.10` chỉ bật png/jpeg/gif/bmp/webp và khóa Cargo.lock. Đã fetch
thành công dependencies sau khi network sandbox không resolve được crates.io;
validation sau đó dùng locked/offline.

Validation macOS ARM64, Rust 1.99.1 ngày 2026-11-12, toolchain tạm:
- `cargo test --locked --offline`: 87 pass (16 + 13 + 13 + 14 + 13 + 18),
  0 failed, 0 ignored.
- `cargo fmt --check`: pass.
- `cargo clippy --all-targets --locked --offline -- -D warnings`: pass.
- Copy nguyên fixtures preview.png/jpg từ Flutter; không sửa source gốc.
- Catalog 4 widget entries partial F06 core, 2 native-only entries pending F16.

Giới hạn có chủ đích, chưa đạt full parity: codec Rust có thể cấp phát frame
nguồn, nên kiểm tra decoded source bytes và RGBA size <=64 MiB trước decode.
Ảnh vượt budget trả lỗi và giữ raw; chưa có thumbnail decode ảnh nguồn rất lớn
như ImageIO/WIC. 64 MiB không phải tổng heap đo thực tế (codec scratch, bytes,
source frame và RGBA conversion có thể tồn tại cùng lúc). SVG data URI được
normalize nhưng chưa có SVG renderer, nên pixel decode trả lỗi.

F16 vẫn pending: preview/Raw toggle và copy, inline/enlarged binding, live update
revision và cancellation, zoom/pan/fit/no reload khi zoom, GUI cache release,
platform host failure/overlay/clipping. Arc/core tests không chứng minh GUI pass.
Large-source thumbnail parity cũng phải giải quyết tại F16 trước nghiệm thu app.
Không publish; Rust folder chưa là Git repo.

## Chat tiếp theo: F07 — Preview màu

Đọc AGENTS.md/progress/matrix tại `/Users/quocan/Github/flutter_network_viewer (rust)`.
Chỉ ghi trong Rust; không sửa Flutter và không spawn subagent. Khảo sát body_color
và tests bằng CodeGraph projectPath `/Users/quocan/Github/flutter_network_viewer`:
context rồi một explore, đọc file cụ thể khi output thiếu. Viết executable tests
trước code: HEX/CSS rgb(a)/hsl(a)/Flutter AARRGGBB/tên; loại ID/substrings;
Unicode UTF-16 offsets; copy/search/folding giữ raw. UI swatches/copy và renderer
thuộc F15/F16, không suy ra GUI parity từ core tests. Cập nhật catalog/docs;
regression/fmt/clippy pass rồi list_projects lấy ID thật và tạo đúng một chat
F08, yêu cầu bắt đầu thực hiện ngay. Toolchain bắt buộc
CARGO_HOME=/private/tmp/fnv-cargo, RUSTUP_HOME=/private/tmp/fnv-rustup,
PATH=/private/tmp/fnv-cargo/bin:$PATH; ưu tiên --locked --offline. Không publish.

## F07 — Preview màu (hoàn thành core; GUI F15/F16 pending)

Triển khai `src/body_color.rs`, export qua `src/lib.rs`; `JsonDocument` giữ
`OnceLock<Vec<ColorPreview>>` theo document. Chỉ ghi trong thư mục Rust.
Khảo sát CodeGraph context rồi đúng một explore với projectPath Flutter;
explore trả source không liên quan nên đọc cụ thể `body_color.dart`,
`body_color_test.dart` và các đoạn dùng parser trong code/Tree/virtual views.

TDD: viết 13 executable tests trước implementation; chạy red báo unresolved
module body_color và thiếu color_previews. Sau implementation 13/13 pass;
thêm 2 regression tests cho escaped JSON/code folds và concurrent cache 5.000
values. Tổng F07 15 tests thực thi, không ignored/placeholder.

Phủ CSS #RGB/#RGBA/#RRGGBB/#RRGGBBAA, Flutter 0xAARRGGBB; CSS rgb(a)/hsl(a),
percent/alpha/deg, hue âm và nhiều vòng, channel rounding; cả 19 tên của nguồn.
Giữ cả hành vi separator và optional alpha của nguồn, không mở rộng tên CSS.
Reject ID/substrings, số trần, hex/function sai, NaN/infinity/out-of-range.
Trim whitespace Dart kể cả BOM. Chỉ string values được nhận (không keys),
bao gồm root/array/nested; metadata chứa decoded value, ARGB, typed path và
encoded token range UTF-16, thứ tự document. Unicode non-BMP/escaped keys và
escaped color tokens có kiểm tra offsets. Metadata lazy/cache thread-safe,
response mới độc lập, không global cache. Không sửa text/value/search/fold
indexes; core tests kiểm tra copy-data JSON và metadata visibility qua indexes.

Validation macOS ARM64, Rust 1.99.1 ngày 2026-11-12, toolchain tạm:
- `cargo test --locked --offline`: 102 pass (16+13+13+14+13+18+15), 0 fail/ignored.
- `cargo fmt --check`: pass.
- `cargo clippy --all-targets --locked --offline -- -D warnings`: pass.
- Không thêm dependency; Cargo.toml/Cargo.lock không đổi.
- Catalog body_color: parser covered, widget entry partial core; native mapping
  giữ rendering pending.

Giới hạn: chưa có GUI. Light/dark swatches, checkerboard transparency,
tooltip/semantics, clipboard thật, code/Tree toggle/render và folding tương tác
vẫn F15/F16 pending; không suy ra GUI pass từ các core tests. F06 large-image
thumbnail parity vượt decoded budget 64 MiB vẫn pending F16. Không publish;
Rust chưa là Git repo.

## Handoff F07 → F08 — Settings và persistence

Chat kế tiếp chỉ làm F08. Bắt đầu thực hiện ngay, không spawn subagent.
Đọc AGENTS.md/progress/matrix trong Rust; chỉ ghi tại
`/Users/quocan/Github/flutter_network_viewer (rust)`, không sửa Flutter.
Khảo sát settings/layout_preferences/network_columns và tests bằng CodeGraph
projectPath `/Users/quocan/Github/flutter_network_viewer`: context rồi đúng một
explore, tin kết quả, đọc file cụ thể nếu thiếu. Viết executable tests trước
implementation: light mặc định, VI/EN và locale lạ fallback; lưu URL, MCP opt-in
false/port 38943; validate port/layout; migrate 6 tab; split riêng và column widths;
settings hỏng fallback; persistence round-trip thực dùng temp file và lỗi I/O nếu
thuộc contract. UI settings F17 vẫn pending, không suy ra GUI từ core tests.
Cập nhật catalog/docs; chạy regression/fmt/clippy; khi pass lấy project ID thật
bằng list_projects và mở đúng một chat F09, yêu cầu bắt đầu thực hiện ngay.
Nếu bị chặn ghi rõ và không tạo chuỗi thất bại. Toolchain bắt buộc:
CARGO_HOME=/private/tmp/fnv-cargo RUSTUP_HOME=/private/tmp/fnv-rustup
PATH=/private/tmp/fnv-cargo/bin:$PATH; ưu tiên --locked --offline. Không publish.

Đã tạo đúng một chat F08: `01a1cf2b-329e-7633-84fd-482e82e7de0d` trong project
Flutter Network, kèm đường dẫn Rust và yêu cầu bắt đầu ngay.

## F08 — Settings và persistence (hoàn thành core; GUI pending)

Triển khai `src/settings.rs`, export qua `src/lib.rs`. Chỉ ghi Rust, không sửa
Flutter; khảo sát CodeGraph context rồi đúng một explore. Đọc trực tiếp các
phần output thiếu: layout_preferences/network_columns tests, phần settings test
và settings_screen/main chứa URL/discovery persistence.

TDD: viết 18 executable integration tests `tests/settings.rs` trước code; chạy
red báo unresolved module settings. Sau implementation 18/18 pass; sửa warning
unused assignment trong test ownership trước khi chạy clippy. Không có ignored
hoặc placeholder. Không thêm dependency, Cargo.toml/Cargo.lock không đổi.

Contract đã phủ:
- Light/EN mặc định; VI/EN và dark/light round-trip; locale/theme lạ fallback;
  setter từ chối language lạ và system theme.
- MCP opt-in false/port 38943; giữ port khi disable; boundary 1..65535 và
  reject input invalid trước mutation/file write. Chưa chạy server MCP (F18).
- Lưu nguyên URL đã normalize từ adapter và discovery flag; manual save tắt
  discovery trong cùng file replacement. Mặc định URL rỗng/discovery true khi
  không có compile-time endpoint; normalize và restart VM thuộc F09/F10/F17.
- Legacy tab 0/1/2/3 migrate thành 0/2/4/5; tên tab ưu tiên kể cả tên lạ về 0;
  sáu tab lưu bằng tên. Network width None (GUI sẽ center), console 400,
  JSON fraction .5; width finite >=320, fraction finite 0<x<1.
- Ba cột mặc định [64,56,108], min [64,56,96], max 1600; finite, đúng 3 phần tử,
  copy ownership, lưu JSON string theo key Flutter. Split/cột độc lập qua reload.
- Missing/corrupt/non-object/invalid UTF-8 fallback không ghi đè file khi đọc;
  trường sai kiểu fallback riêng; unknown keys giữ nguyên. File chỉ nhận settings,
  không có capture store. Non-NotFound read errors và write failures trả lỗi.
- Temp directory/file thực cho round-trip và lỗi I/O. Theme/language/MCP giữ
  committed state khi ghi lỗi; layout cập nhật RAM trước I/O giống Flutter,
  cả nhiều lỗi layout liên tiếp. Lỗi rename dọn temp và có thể retry preference.

File store dùng flat preference JSON với key Flutter; chưa tự import file
SharedPreferences platform cũ. Caller chọn path có parent đã tồn tại. Mỗi lần
save ghi temp sibling create_new, sync file rồi rename thay destination; không
hứa power-loss durability (chưa sync parent), multi-process merge hoặc tự tạo
thư mục. Store đồng bộ single-owner; GUI sau này cần cân nhắc worker cho I/O.
Không có notifications/GUI binding trong core; không suy ra GUI parity từ tests.

Validation macOS ARM64, Rust 1.99.1 ngày 2026-11-12, toolchain tạm:
- `cargo test --locked --offline`: 120 pass (102 trước + F08 18), 0 fail/ignored.
- `cargo fmt --check`: pass.
- `cargo clippy --all-targets --locked --offline -- -D warnings`: pass.
- Catalog: 6 core entries covered; 6 widget entries partial, các widget settings
  còn lại giữ pending F17/F18. GUI F13–F17 chưa triển khai.
- Không publish; Rust chưa là Git repo.

## Handoff F08 → F09 — VM protocol và adapter

Chat tiếp theo chỉ làm F09, bắt đầu thực hiện ngay; không spawn subagent.
Đọc AGENTS.md/progress/matrix tại `/Users/quocan/Github/flutter_network_viewer (rust)`.
Chỉ ghi trong Rust, không sửa Flutter. Khảo sát viewer_server, live_vm_capture,
support/fake_vm_service bằng CodeGraph projectPath Flutter: context rồi đúng một
explore, đọc file cụ thể khi output thiếu. Viết executable tests trước code:
normalize DevTools/encoded HTTP/WS URL và auth token; JSON-RPC correlation,
error/timeout; HTTP profile timing/headers/Unicode/binary/errors; isolate-scoped
IDs; body size guard và malformed bytes. Lưu URL F08 nhận URL đã normalize,
không tự normalize hoặc restart VM. Lifecycle/polling/reconnect thuộc F10/F11.
F09 phải có integration fake VM/transport phù hợp, không dùng placeholder hay
ignored tests để báo pass; không kết luận GUI/live SDK parity từ core tests.
Cập nhật catalog/docs, regression/fmt/clippy pass rồi list_projects lấy ID thật
và tạo đúng một chat F10 yêu cầu bắt đầu thực hiện ngay. Nếu môi trường chặn,
ghi rõ và không tạo chuỗi thất bại. Bắt buộc CARGO_HOME=/private/tmp/fnv-cargo,
RUSTUP_HOME=/private/tmp/fnv-rustup, PATH=/private/tmp/fnv-cargo/bin:$PATH;
ưu tiên locked/offline, thêm dependency chỉ khi cần và xác minh được.
F06 large-source thumbnail >64MiB còn pending F16; GUI toàn bộ vẫn pending.

## F09 — VM protocol và adapter (hoàn thành core; lifecycle/live SDK pending)

Triển khai `src/vm.rs`, export `vm`; 21 executable tests trong
`tests/vm_protocol.rs`. Chỉ ghi Rust; không sửa Flutter, không spawn subagent.
Đã khảo sát Flutter bằng CodeGraph context rồi đúng một explore; đọc cụ thể
các đoạn viewer_server và viewer_server_test bị thiếu cùng live_vm_capture test.

TDD: viết 15 tests trước code; sửa cú pháp json! và getter network của test,
chạy red báo thiếu module vm/dependency tungstenite. Sau implementation thêm
4 regressions wire/timestamp; thêm numeric guard test chạy red thực sự với
contentLength 1e30 rồi sửa saturation/trim headers. Thêm contract test trước khi
đổi profile_detail sang Result để phân biệt lỗi transport với lỗi detail RPC.
Không có ignored, placeholder hoặc live test được skip để báo pass.

Contract và bằng chứng:
- Normalize DevTools uri= percent-encoded, HTTP(S)/WS(S) roots; giữ encoded auth
  path/token, IPv6; bỏ query; reject scheme/authority sai, userinfo, fragment và
  DevTools thiếu uri. F08 tiếp tục nhận URL đã normalize, không tự restart.
- RpcCore deterministic caller-clock: monotonic numeric IDs, params object,
  correlation out-of-order, remote error code/message/data, notifications,
  malformed JSON/schema, inclusive expiry, unknown/late replies ignored.
  Malformed response không lấy mất pending call.
- HTTP profile request/response/error, pending phase; duration microseconds→ms,
  ISO UTC millisecond/microsecond kể cả âm; headers list/scalar, events, Unicode
  UTF-8 strict, invalid UTF-8→base64, byte counts, malformed byte array omission.
- 4 MiB combined declared size preflight (saturating numeric lengths), fallback
  Content-Length case-insensitive/list/string; exact boundary cho phép. Oversized
  arrays omit trước byte conversion; giữ status/headers/length và không mutate
  summary. Isolate-scoped JSON tuple IDs và vmRequestId ingested vào CaptureStore.
- VmClient dùng tungstenite, WS và native TLS WSS; real loopback fake VM với
  handshake auth path, detail request params, response adapter; large body không
  gọi detail RPC; RPC error omit body và RPC sau vẫn thành công. Timeout thật,
  late response/notification rồi RPC tiếp trên cùng socket, malformed reply và
  socket close đều được kiểm tra. profile_detail trả Result: Remote/Timeout
  thành profile-read-error omission; transport/protocol errors trả Err cho caller.

Dependencies mới: url 2.5.8, base64 0.22.1, tungstenite 0.29.0 + native-tls;
Cargo.lock đã khóa dependencies/transitives. Fetch ban đầu lỗi DNS sandbox,
fetch ngoài sandbox được chấp thuận và thành công. Test loopback ban đầu bị
sandbox chặn bind EPERM; chạy lại ngoài sandbox được chấp thuận và pass thật.
Clippy large-error lint chỉ allow tại fake handshake helper vì callback của
thư viện yêu cầu concrete HTTP error response type; không suppress source lint.

Validation macOS ARM64, Rust 1.99.1 ngày 2026-11-12, toolchain tạm:
- `cargo test --locked --offline`: 141 pass (120 trước + F09 21), 0 fail/ignored.
- `cargo fmt --check`: pass.
- `cargo clippy --all-targets --locked --offline -- -D warnings`: pass.
- Catalog: 3 viewer_server core entries covered, 2 partial; live SDK giữ pending.
- Sửa status F13 bị chép nhầm từ F08 trong matrix về Pending; chưa có GUI.

Giới hạn cụ thể: client blocking, caller phải chạy trên worker; một RPC in-flight
mỗi VmClient (RpcCore độc lập hỗ trợ correlation nhiều pending calls). WSS/TLS
được compile trên macOS nhưng chưa chạy test TLS certificate hoặc Windows.
DNS resolution dùng resolver hệ thống nên không có absolute DNS deadline;
socket timeout áp dụng handshake I/O, RPC dùng remaining wall-clock budget.
Wire messages giới hạn 48 MiB, notification queue giữ tối đa 1024 entries, drop
oldest; chưa có console stream decoder hoặc byte-budget queue (F10).
Chưa có background stream pump ngoài thời gian call. F10 cần transport/worker
abstraction thích hợp cho console độc lập và cancellation; F11 xử lý reconnect,
latest-connect-wins, nhiều isolate. Không suy ra GUI/live SDK parity từ fake VM.
F06 thumbnail nguồn vượt decoded budget 64 MiB vẫn pending F16. Không publish;
Rust folder chưa là Git repository.

## Handoff F09 → F10 — Capture lifecycle và adaptive polling

Chat kế tiếp chỉ làm F10, bắt đầu ngay; không spawn subagent. Đọc AGENTS.md,
progress/matrix tại Rust; chỉ ghi `/Users/quocan/Github/flutter_network_viewer (rust)`.
Không sửa Flutter. Khảo sát adaptive_capture/capture_cleanup/viewer_server và
fake_vm_service bằng CodeGraph projectPath Flutter: context rồi đúng một explore,
đọc file cụ thể khi thiếu. Viết executable tests trước implementation: network
1.4s +0.2s tối đa 5s, thay đổi reset; console batch độc lập/in-order; poll không
overlap; Stop hủy timers; Pause bỏ events; Clear hủy pending/in-flight và inclusive
cursor không replay; bounded queue. Dùng fake clock/transport deterministic và
integration fake VM phù hợp. Dựa trên vm.rs F09, capture.rs và console.rs;
không để blocking VmClient cản console độc lập/Stop, không gọi generic RPC timeout
là transport disconnect. Reconnect/nhiều isolate F11 và GUI vẫn pending.
Cập nhật catalog/docs, regression/fmt/clippy pass rồi list_projects lấy ID thật và
mở đúng một chat F11 yêu cầu bắt đầu ngay; nếu bị chặn ghi rõ và không tạo chuỗi
thất bại. Toolchain bắt buộc CARGO_HOME=/private/tmp/fnv-cargo,
RUSTUP_HOME=/private/tmp/fnv-rustup, PATH=/private/tmp/fnv-cargo/bin:$PATH;
ưu tiên locked/offline; loopback tests cần chạy ngoài sandbox bằng escalation.

Đã tạo đúng một chat F10: `01a1cf3b-02ea-7080-9d7a-c19225b25009` trong project
Flutter Network, kèm đường dẫn Rust, handoff đầy đủ và yêu cầu bắt đầu ngay.

## F10 — Capture lifecycle và adaptive polling (hoàn thành core)

Chỉ ghi Rust, không sửa Flutter, không spawn subagent. Khảo sát Flutter bằng
CodeGraph context rồi đúng một explore; output thiếu phần lớn bodies/tests nên
đọc các phần cụ thể adaptive_capture, capture_cleanup, fake_vm_service và
viewer_server. Không dùng catalog làm bằng chứng test pass.

Thêm `src/lifecycle.rs` và `src/vm_worker.rs`, export trong lib.rs. `CaptureEngine`
sở hữu CaptureStore cho một isolate đã attach; caller cung cấp clock monotonic
qua tick(now_ms). Network/console có deadline riêng. Poll chỉ schedule sau khi
hoàn tất summary và chuỗi detail, không overlap. Base 1400 ms, idle +200 ms,
cap 5000 ms, summary thay đổi reset riêng network; batch console có dữ liệu
reset riêng console. Summary không đổi không gọi detail lại. Fingerprint,
pending summary và console queue có giới hạn count/UTF-8 bytes; queue giữ phần
mới nhất. Eviction từ raw transport queue được cộng vào store statistics.

Worker dùng VmClient trên thread riêng, subscribe Stdout/Stderr/Logging, pump
khi idle và callback notification ngay trong RPC đang chờ. Thêm call_streaming,
pump và cloned TCP shutdown handle crate-private trong vm.rs; API F09 giữ nguyên.
Một RPC outstanding, mailbox chứa tối đa một response và một terminal error;
raw stream queue count/byte bounded. Stop/Drop shutdown TCP, đánh thức blocking
read kể cả detail chưa trả. Không join/chờ RPC timeout trên owner thread.
Idle socket closure được chuyển thành lỗi terminal và xóa session/capture.
RPC Timeout/Remote giữ transport; detail lỗi trở thành omitted body summary.

Clear bỏ pending logs, stdout fragments, summary work và fingerprint; tăng
revision/capture epoch để bỏ detail trả trễ. Giữ cursor+1 vì VM filter inclusive.
Nếu Clear xảy ra trước summary reply, lấy timestamp reply+1 nhưng bỏ requests.
Pause bỏ queued/partial output, advance cursor/fingerprint, không RPC detail cho
paused summaries. Pause→Resume trong một in-flight summary/detail không replay.
Stop xóa session/capture, cursor, fingerprint và hai deadline; transport đã stop
không được reuse. Khởi động lại cần transport/engine mới do coordinator F11.

Stdout/Stderr giữ bytes qua chunks để không làm hỏng split UTF-8; newline emit
in-order, partial buffer >65536 bytes được flush. Logging inline valueAsString
được decode cùng severity/logger/error/stack. Async invoke/getObject để resolve
object error/truncated strings chưa có; full Flutter developer-log test vẫn partial,
không được coi là pass từ inline decoder test.

TDD: viết 10 executable lifecycle tests trước implementation, red thiếu module;
viết 3 worker tests trước worker, red thiếu module. Bổ sung regression tests trước
fix phát hiện và sửa Stop chưa xóa retained session, Pause→Resume summary replay,
raw queue eviction chưa cộng statistics và bounded summary giữ nhầm rows cũ.
Sửa một lỗi ownership trong test và cách tiến fake clock sau completion ở real
Clear test; không bỏ/ignore tests. Cuối cùng F10 có 15 deterministic state-machine
và 7 real loopback WebSocket fake VM tests. Fake VM dùng channels để giữ/release
RPC, không dùng sleeps để chọn thời điểm response; bounded wait chỉ để thread I/O
progress. Timeout test dùng socket timeout thật và late response trên cùng socket.

Validation macOS ARM64, Rust 1.99.1 ngày 2026-11-12, toolchain tạm:
- `cargo test --locked --offline`: 163 pass (141 trước + F10 22), 0 fail/ignored.
- `cargo fmt --check`: pass.
- `cargo clippy --all-targets --locked --offline -- -D warnings`: pass.
- Tests loopback chạy ngoài sandbox bằng approval do sandbox chặn bind.
- Không thêm dependency, Cargo.lock không đổi; Rust chưa là Git repository.

Giới hạn: factory VmWorker::connect vẫn blocking khi DNS/handshake/subscription;
caller phải chạy trên connection worker. CaptureEngine nhận sẵn session/isolate,
không tự getVM/getIsolate/enable profiler. Coordinator bootstrap, latest-connect-
wins, extension discovery, reconnect và nhiều isolate thuộc F11. Store hiện sở
hữu bởi mỗi engine; F11 cần phối hợp shared capture/application metadata và giữ
cleared cursor qua reconnect. Tick phải được host gọi; chưa có GUI event loop.
Budget đo serialized JSON, không phải tổng heap; wire cap vẫn 48 MiB. TLS compile
macOS, chưa cert integration/Windows. Chưa live Dart SDK/GUI parity. F06 thumbnail
ảnh nguồn decoded >64 MiB vẫn pending F16. Không publish.

## Handoff F10 → F11 — VM reconnect và nhiều isolate

Chat tiếp theo bắt đầu ngay, chỉ F11. Đọc AGENTS.md/progress/matrix trong Rust;
chỉ ghi `/Users/quocan/Github/flutter_network_viewer (rust)`, không sửa Flutter,
không spawn subagent. Dùng CodeGraph Flutter projectPath: context rồi một explore,
đọc file cụ thể khi output thiếu. Tests thực trước implementation: latest connect
wins; bootstrap getVM/getIsolate/enable profiler; delayed extensions; socket đóng
reconnect cùng URL; timeout profile giữ transport/data; isolate Exit hủy hung RPC;
hot restart một app tab; endpoint switch xóa session; Stop/dispose hủy attempts;
Clear giữ cursor exclusive boundary qua reconnect. Không coi Timeout là disconnect.

Có lifecycle.rs (tick clock, CaptureTransport trait) và VmWorker (bounded queues,
continuous stream pump, interruptible Stop), 163 tests pass. Engine hiện single
attached isolate và own CaptureStore; coordinator phải nối metadata/shared app
history, nhiều isolate và async connection cancellation. VmWorker::connect hiện
blocking trước khi có shutdown handle; cần thiết kế cancellation cho pending
connect/subscription. stop là terminal và xóa cursor; lấy/giữ cleared stream
position ở coordinator trước khi stop. Inline Logging decode có nhưng async
invoke/getObject resolution/truncated values và cross-stream ordering khi resolve
vẫn partial, ghi rõ nếu không xử lý trong scope F11. GUI/live SDK pending.

Bắt buộc CARGO_HOME=/private/tmp/fnv-cargo, RUSTUP_HOME=/private/tmp/fnv-rustup,
PATH=/private/tmp/fnv-cargo/bin:$PATH; ưu tiên locked/offline. Tests loopback cần
exec_command require_escalated. Cập nhật catalog/progress/matrix/README, chạy
regression/fmt/clippy. Khi pass list_projects lấy ID thật rồi tạo đúng một chat
F12 — Auto discovery và chọn app, yêu cầu bắt đầu ngay; nếu môi trường chặn ghi
rõ và không tạo chuỗi chat thất bại. Không publish.

## F11 — VM reconnect và nhiều isolate (hoàn thành core)

Thêm `src/coordinator.rs`: ConnectionFactory bất đồng bộ, ThreadConnector,
Coordinator/Config với clock caller, router một RPC trên một socket và nhiều
CaptureEngine. Shared CaptureStore được swap vào từng engine trong owner tick;
chỉ có một retained app history/budget, scoped IDs theo isolate từ F09. Bootstrap
getVM → getIsolate → ext.dart.io.httpEnableTimelineLogging(enabled: "true") trước
capture. Stream Isolate cùng Stdout/Stderr/Logging được đăng ký trên worker.
Extension chưa có được khám phá lại định kỳ; IsolateStart/Exit yêu cầu refresh
ngay. Giới hạn discovery 64 isolate mỗi VM. Metadata app ID dùng authority/pid/
startTime; nhiều isolate và hot restart giữ một session và lịch sử cũ. App online
khi còn isolate attach. Endpoint switch và Stop xóa session/data; socket mất thì
retry đúng normalized endpoint sau retry_ms (mặc định 3000), không auto discovery.

Profile Timeout giữ transport/data, retire rồi bootstrap lại isolate. getVM
health failure reconnect cùng URL; không coi generic profile RPC Timeout là
transport disconnect. VmClient phát hiện IsolateExit đúng isolate trong lúc đọc
RPC, bỏ correlation và trả cancellation Remote -32800; worker tiếp tục socket.
Coordinator loại reply của route retired, kể cả kết quả trả trễ. Test thật giữ
reply isolate cũ đến sau khi isolate mới đã capture, kiểm tra chỉ một socket.

Console nhiều isolate đi qua một FIFO chung trước batch nên giữ thứ tự wire;
buffer partial vẫn riêng isolate/stream. Queue raw/decoded và retained capture
có count/byte bounds; overflow cộng statistics. Clear/Pause bỏ pending console
và partial buffers ở mọi isolate; Clear loại detail/summary stale bằng revision.
Coordinator giữ cursor trước retire/Stop engine; Clear dùng exclusive timestamp+1
qua reattach/reconnect, kể cả Clear lúc disconnected. Clear trước summary đầu
đánh dấu baseline pending, summary kế tiếp chỉ lấy timestamp+1 và bỏ history.
Stop/Drop mới là terminal cleanup; có thể gọi connect sau Stop để mở capture mới.

ConnectionCancel đăng ký TCP clone trước TLS/WS handshake và streamListen;
Stop/Drop/latest-connect-wins shutdown socket để ngắt read setup đang treo.
Attempt receiver bị bỏ nên stale result không thể được install. System DNS và
TCP connect trước khi có socket vẫn có thể hoàn tất trên background thread;
cancellation check sau đó loại kết quả. Không tuyên bố OS DNS cancellable hay
absolute DNS deadline; không block owner/UI thread. ThreadConnector timeout mặc
định 8s mỗi RPC; host cần gọi tick để tiến state/timers. Chưa có GUI event loop.

TDD: viết 7 deterministic tests trước implementation, red unresolved coordinator
và ConnectionCancel. Sau đó viết 3 real loopback tests; red phát hiện không đưa
recoverable error lên coordinator và fake VM cần shutdown cả clone khi đóng.
Thêm regression tests trước fix cho Clear disconnected, Clear baseline chưa có
summary, multi-isolate console order, Pause/Clear mọi partial buffers, shared
queue budget; tổng F11 14 deterministic + 3 real WebSocket tests. Thêm health
Timeout/initial profile Timeout assertions. Không ignored/placeholder.

Full regression phát hiện F10 real Clear test chờ mailbox rỗng không chứng minh
worker đã đọc stream. Đổi fixture thêm complete line và chờ pending_console_len
trước Clear; vẫn kiểm tra partial tail bị loại. Deadline test Timeout thật F11
đổi 60ms thành 250ms để tránh timeout bootstrap do scheduling khi chạy song song;
clock polling/reconnect vẫn deterministic. Không đổi production timings.

Toolchain tạm đã bị xóa giữa hai phiên; khôi phục Rust stable 1.100.0, rustfmt,
clippy tại /private/tmp/fnv-cargo và /private/tmp/fnv-rustup. Fetch lại dependencies
bằng Cargo.lock, không thêm dependency hoặc thay lockfile. Không sửa Flutter.

Validation macOS ARM64 ngày 2026-11-13, toolchain tạm:
- `cargo test --locked --offline --quiet`: 180 pass (163 cũ + F11 17), 0 fail/ignored.
- `cargo fmt --check`: pass.
- `cargo clippy --all-targets --locked --offline -- -D warnings`: pass.
- Loopback chạy qua require_escalated vì sandbox chặn bind.

Chỉ core F11: GUI app tabs phải test ở F13; live Dart SDK/Windows và WSS cert
integration vẫn pending. Full Logging object error/truncated-string invoke/getObject
resolution và ordering khi async resolve vẫn pending adapter integration; inline
Logging/cross-isolate FIFO chưa phải toàn bộ Logging parity. F06 thumbnail nguồn
vượt 64MiB vẫn F16 pending. Rust chưa Git repo; không publish.

## Handoff F11 → F12 — Auto discovery và chọn app

Bắt đầu ngay, chỉ F12; không spawn subagent. Chỉ ghi tại
`/Users/quocan/Github/flutter_network_viewer (rust)`; không sửa Flutter. Đọc
AGENTS.md/progress/feature-matrix. Khảo sát vm_service_discovery.dart,
vm_service_discovery_test.dart, vm_service_discovery_widget_test.dart qua
CodeGraph projectPath `/Users/quocan/Github/flutter_network_viewer`: context rồi
đúng một explore, đọc file cụ thể nếu output thiếu. Trust index. Tests executable
trước code. Discovery process DDS local, auth encoded path, redirect/dedupe,
loại VM không Flutter và Viewer; chờ app start, nhiều app chờ chọn; mỗi scan thêm
một app rồi tắt; giữ app cũ; Stop/manual hủy scan. UI picker thuộc F13/F17 nếu
chưa có GUI, không suy ra UI parity từ core. Tích hợp Coordinator/ThreadConnector
F11: tick(now_ms), connect(endpoint, now), endpoint/store/running/is_online,
clear/set_paused/stop; backend nhiều app cần shared app management phù hợp.

F01–F11 có 180 tests pass; F11 có 14 deterministic và 3 socket tests, thêm module
coordinator, ConnectionCancel ở vm.rs, VmWorker::connect_coordinated. Managed
CaptureEngine dùng store swap; global console FIFO ở coordinator giữ thứ tự
wire. Router một outstanding RPC, bounded queues; getVM health timeout reconnect,
profile Timeout giữ socket và bootstrap lại isolate. Clear cursor/baseline transfer
qua reconnect phải giữ. IsolateExit hủy call trên VmClient cùng socket; late reply
không được route lại. System DNS/TCP connect chưa cancel trước khi có handle;
handshake/subscription cancellation đã integration tested. Async Logging object
resolve/truncated strings vẫn partial, GUI/live SDK toàn bộ pending.

Bắt buộc CARGO_HOME=/private/tmp/fnv-cargo RUSTUP_HOME=/private/tmp/fnv-rustup
PATH=/private/tmp/fnv-cargo/bin:$PATH; ưu tiên locked/offline. Nếu /tmp bị dọn,
khôi phục toolchain/cache và không gọi binary tests cũ để báo source mới pass.
Tests loopback cần exec_command require_escalated do sandbox bind EPERM. Chạy
regression/fmt/clippy, cập nhật catalog/progress/matrix/README. Khi F12 pass,
list_projects lấy ID thật, tạo đúng một chat F13 — Desktop shell, tabs và Network,
yêu cầu bắt đầu ngay. Dùng project Flutter Network nếu Rust chưa đăng ký. Nếu
môi trường chặn, ghi rõ và không tạo chuỗi chat lỗi. Không publish.

## F12 — Auto discovery và chọn app (core)

Đã triển khai `src/discovery.rs`, chỉ ghi Rust, không đổi source Flutter,
Cargo.toml hoặc Cargo.lock. Đã khảo sát đúng một CodeGraph context + một explore
trên Flutter; đọc phần test discovery bị trim để hoàn tất đặc tả.

`process_uris` chỉ lấy --vm-service-uri từ dòng DDS (development-service,
dds.dart/dds_aot.dart.snapshot), local http/ws, có port, không credentials;
dedupe source và tối đa 24 candidates. Giữ percent-encoded auth path nguyên vẹn.
HTTP resolution tối đa 4 hops, bounded 16KiB headers, timeout mỗi hop; hỗ trợ
DevTools nested uri local và chặn remote targets. Probe getVM/getIsolate tối đa
8 isolates, yêu cầu ext.flutter.*, loại self pid/OS và package Viewer. Dừng ngay
ở Flutter isolate đầu tiên: isolate sau đã exit không được làm mất app hợp lệ.
Dedupe theo pid/startTime/OS và sort name. Probe socket được shutdown khi xong.

`ThreadDiscovery` chạy process listing/probes ở background. Unix dùng /bin/ps,
Windows dùng PowerShell CIM cho dart/dartaotruntime. Process listing có deadline
5s và cap 4MiB. Probe timeout mặc định 2s cho mỗi HTTP hop/RPC; không có một
absolute deadline cho cả scan. Candidates được probe tuần tự, tối đa 24, không
port scan. ConnectionCancel đăng ký socket HTTP/VM để Stop ngắt hung reads;
OS DNS/TCP trước handle vẫn có giới hạn/cancellation giống F11, không tuyên bố
system DNS cancellable. Process enumeration Windows và live Dart SDK chưa test.

`AppDiscovery<F,D>` quản lý coordinator độc lập cho từng VM, swap chung một
CaptureStore vào mỗi owner operation để retained budget/FIFO không nhân theo
app. Coordinator thêm internal swap_store/vm_key, ThreadConnector Clone;
ConnectionCancel thêm internal register visibility/close_registered cho probes.
Clock caller qua tick(now_ms), start_scan, select(vm_key, now), stop_discovery,
connect_manual, stop, clear, set_paused. store/targets/searching/busy/endpoint/
connections/is_online/last_error dùng được từ GUI host tương lai.

Một scan không overlap; không có app thì retry theo retry_ms (3000ms production),
nhiều app thì chờ chọn. Thành công attach một app dừng discovery, xóa choices;
kết nối lỗi trước attach quay lại scan. App đã attach reconnect cùng endpoint,
không rediscover. Explicit scan thêm một app mới, giữ socket/cursors/history cũ;
exclude cả app manual theo VM identity hiện tại. Stop discovery chỉ hủy candidate
chưa attach; Stop/manual hủy scan và stale receiver. Manual thay tất cả app;
kể cả URL manual lỗi cũng hủy scan. Drop hủy scan và coordinators. Global
Pause/Clear đi qua mọi app, giữ clear cursor/baseline của F11.

TDD: viết 8 deterministic + 3 socket tests trước module; red unresolved discovery.
Sau implementation thêm regression tests trước fixes: exclude manual app, failed
auto attachment retry, retained global record budget (assert scoped vmRequestId),
manual URL lỗi vẫn hủy scan. Fake VM summary tôn trọng updatedSince như protocol.
Socket fixture thêm isolate thứ hai trả error; red found=0 trước fix dừng sớm ở
Flutter isolate đầu tiên. Kết quả F12: 12 deterministic + 3 real HTTP/WebSocket
(redirect/token/dedupe, nonFlutter/Viewer, remote redirect/hung HTTP cancellation).

Validation source mới macOS ARM64 ngày 2026-11-13, Rust 1.100.0:
- CARGO_HOME=/private/tmp/fnv-cargo, RUSTUP_HOME=/private/tmp/fnv-rustup,
  PATH=/private/tmp/fnv-cargo/bin:$PATH.
- `cargo test --locked --offline --quiet`: 195 pass (180 cũ + F12 15), 0 fail/ignored.
- `cargo fmt --check`: pass.
- `cargo clippy --all-targets --locked --offline -- -D warnings`: pass.
- Tests loopback chạy require_escalated vì sandbox bind hạn chế.

F12 là core; GUI picker/loading/Stop-Start/preferences cần GUI tests F13/F17.
Multi-app capture/shared history được kiểm tra qua deterministic transports;
combined discovery→multi-app real socket/console acceptance vẫn cần F21. Không
coi fake probe là live Dart SDK smoke. F11 async Logging object/truncated resolve,
F06 >64MiB thumbnail, Windows/WSS cert integration vẫn pending. Không publish.

## Handoff F12 → F13 — Desktop shell, tabs và Network

BẮT ĐẦU NGAY, chỉ F13, mỗi chat một tính năng. Không spawn subagent. Chỉ ghi
`/Users/quocan/Github/flutter_network_viewer (rust)`; không sửa Flutter tại
`/Users/quocan/Github/flutter_network_viewer`. Đọc AGENTS.md/progress/matrix trước.
Khảo sát Flutter viewer_widget/application_tabs/network_columns và UI liên quan
bằng CodeGraph projectPath Flutter: context rồi một explore, đọc file cụ thể khi
output thiếu. Không dùng grep để thay structural CodeGraph/trust index.

Tests executable trước implementation. F13: desktop shell macOS/Windows, All/app
lọc network lẫn console, search URL/method/status kết hợp filters, request pending
cập nhật khi chọn, 6 inspector tabs, cURL/copy/export thật, sidebar hẹp/cuộn cột,
Escape/outside dismiss. Có thể dùng egui/eframe như dự kiến README, chưa có GUI
và chưa có GUI dependency. GUI phải có executable UI tests, chạy app thật khi
môi trường hỗ trợ; core unit tests không đủ parity. Picker discovery từ F12 phải
có UI tests khi có GUI; settings/toast/help sâu thuộc F17, console detail F14,
JSON/editor F15 và previews/performance F16. Giữ scope, ghi rõ Windows không chạy
nếu chỉ có macOS. Nếu môi trường chặn thì báo đúng, không đánh dấu pass/hay tạo
chuỗi chat lỗi.

Backend F12 `AppDiscovery<ThreadConnector,ThreadDiscovery>` có new(factory,
discovery,CoordinatorConfig), tick(now), start_scan(now), select(vm_key,now),
stop_discovery, connect_manual(uri,now), clear(now), set_paused(bool), stop;
store(), targets(), searching(), busy(), connections(), endpoint(), is_online(id),
last_error(). VM connectors/probes chạy background, owner tick nonblocking. Một
shared store cho mọi app, independent coordinators. Manual replaces all apps;
scan thêm một app; Stop scan giữ live apps. Store references dùng trong frame,
đừng copy/serialize body mọi frame. Exports/cURL/settings core đã có F01/F08.
F01–F12 hiện 195 tests pass/0 fail/ignored; fmt/clippy pass, Cargo.lock không đổi.

BẮT BUỘC env toolchain /private/tmp/fnv-cargo và /private/tmp/fnv-rustup, thêm
/private/tmp/fnv-cargo/bin vào PATH; ưu tiên --locked --offline. Nếu cache/toolchain
bị dọn phải khôi phục rồi test source mới, không dùng binary cũ báo pass.
GUI dependencies nếu thiếu cần fetch network với escalation khi bị sandbox chặn.
Tests loopback require_escalated. Rust chưa Git repo, không publish. Cập nhật
catalog/matrix/progress/README và ghi handoff cụ thể. Chỉ khi F13 pass, list_projects
lấy ID thật và tạo đúng một chat F14 — Desktop Console và inspector, yêu cầu bắt
đầu ngay; dùng project Flutter Network nếu Rust chưa đăng ký.

## F13 — Desktop shell, tabs và Network

Đã triển khai GUI egui/eframe 0.33.3 với renderer OpenGL; chỉ ghi Rust, không sửa
Flutter, không spawn subagent, không publish. Đã đọc AGENTS/progress/matrix,
khảo sát Flutter đúng một CodeGraph context + một explore, đọc các file test và
inspector cụ thể khi output chưa đủ. Rust chưa Git repo/CodeGraph; không init.

`src/desktop.rs::DesktopUi` render app tabs, navigation, toolbar, Network filters,
bảng cuộn, inspector sáu tab, modal connect/discovery/export. `ShellStatus` chỉ giữ
metadata online/searching/targets/error. `Action` gửi thao tác cho production host
`src/desktop_host.rs::DesktopApp`, tick discovery/coordinators bằng clock monotonic
và dispatch capture actions. `src/main.rs` mở native window 1360x860, min 480x360,
light mặc định, load settings path theo OS. macOS lưu tại Library/Application
Support/flutter-network-viewer-rust/settings.json; Windows dùng APPDATA. Fail I/O
settings được báo stderr (notification GUI sâu F17).

All/từng app lọc cả Network và Console theo session ID. App tabs cuộn ngang,
online state lấy backend; đổi app xóa request selection. Network search không phân
biệt hoa/thường, URL/method/status/source phối hợp method + Errors/Pending/Success.
Reset filters, Pause/Resume, Clear, Find apps/Stop scan và manual URL có wiring thật.
Manual connect persist endpoint đã normalize; scan/manual supersede tuân theo F12.
Picker có select target/Cancel/Escape/outside; cancel gửi StopScan, giữ app cũ.

Network hiển thị Method/Status/Duration/URL, ba divider kéo và persist widths;
header/rows dùng cùng boundaries. Cửa sổ/sidebar hẹp vẫn có horizontal scrolling.
Inspector split khi >=1100px, modal khi hẹp; resize split persist network width.
Selection giữ key, đọc record sống mỗi frame, không snapshot request pending;
completion event thay nội dung khi đang chọn. Sáu tab: Overview/Request header/
Request body/Response header/Response body/Raw. Overview có URL/method/request ID/
source/started/status/response size/query params/error. Raw dùng serde record cùng
schema capture/export. Detail tab persist và giữ khi đổi request/restart. Headers/
body hiện hiển thị text; JSON/tree/form/image/color UI thuộc F15/F16.

Copy cURL dùng core shell quoting; Copy detail dùng egui native clipboard command.
Export dialog nhận đường dẫn, ghi JSON schema v1 từ store.export() (tất cả apps và
records), chỉ báo thành công sau khi write xong, lỗi hiển thị trong cửa sổ. Không
có OS Save dialog, đường dẫn được nhập trong dialog của ứng dụng.

TDD: viết sáu input UI tests trước module desktop, chạy đỏ unresolved import;
download/compile eframe trước red. Sau đó thêm divider/layout, menu status/picker,
many-app scroll, tab restart và Overview/Raw tests. Divider test bắt lỗi delta lấy
cả pointer movement trước khi nhấn; guard any_pressed đã sửa. Many-app test bắt
scrollbar nổi che hit target tab: toolbar dùng nonfloating scrollbar. Overview/Raw
test chạy đỏ missing request ID rồi sửa metadata và raw serialization.

11 tests trong tests/desktop_ui.rs chạy Context::run production renderer với raw
pointer/key/text/wheel events, assert widget geometry + nội dung thực + file I/O +
clipboard output, không phải test backend được đổi tên UI. Bao gồm 15 app tabs
cuộn tới cuối ở width480 rồi lọc cả hai danh sách, widths 480/860/1100/1360,
Escape/outside dismiss dropdown/modal/picker, pending live update, sáu tab,
clipboard cURL/detail, actual export, divider persistence/header alignment,
inspector tab persistence qua selection/restart wide/narrow.

Native acceptance executable examples/f13_native_smoke.rs mở eframe window thật,
wrap chính production DesktopApp, inject input qua raw_input_hook; backend dùng
real local WebSocket VM fixture examples/f13_fake_vm.rs. Check pending→201 trong
inspector đang chọn (f13.complete RPC chỉ fixture), cURL/detail đọc lại bằng arboard
OS clipboard, Console log, Pause/Resume, file export schema/count, Clear/selection,
backend stop và screenshot PNG. Test restore clipboard text nếu có; xóa artifacts
cũ trước run để không pass bằng output stale. Run tools/f13_native_smoke.py tạo
fixture mới mỗi lần, teardown server, window tự đóng. Không cần Dart SDK.

Có thử app window qua CUA và thấy render native; công cụ mất window reference sau
relaunch nên dùng executable native smoke cho kết quả lặp lại được. Bundle tạm
trong target đã xóa, không có packaging/release ở F13. Test artifacts tại
`docs/artifacts/f13`: native-smoke.log, native-network.png, native-export.json,
regression.log. Screenshot/export chứa dữ liệu synthetic, không capture app thật.

Validation macOS ARM64 ngày 2026-11-13, Rust 1.100.0:
- CARGO_HOME=/private/tmp/fnv-cargo, RUSTUP_HOME=/private/tmp/fnv-rustup,
  PATH=/private/tmp/fnv-cargo/bin:$PATH.
- `cargo test --locked --offline --quiet`: 206 pass (195 cũ + F13 11), 0 fail/ignored.
- `cargo fmt --check`: pass.
- `cargo clippy --all-targets --locked --offline -- -D warnings`: pass.
- `python3 tools/f13_native_smoke.py`: native macOS acceptance pass, exit0.
- GUI deps cần network escalation lần đầu; sau đó locked/offline. Loopback tests
  và native smoke chạy require_escalated vì sandbox hạn chế bind/GUI/clipboard.

Giới hạn cần giữ rõ: Windows chưa build/chạy, không tuyên bố Windows pass; F21
phải chạy Windows/macOS release acceptance. Không live Dart SDK/WSS cert tests.
F13 Console chỉ list + app scope, console detail/grouping/source/level/split F14;
JSON editor F15; virtualization/cache/previews và ảnh nguồn >64MiB F16;
settings/help/toasts/theme/locale F17. GUI accessibility/OS installer acceptance F21.
Native fixture Stdout tại streamListen (trước bootstrap app) chưa xuất hiện trong
GUI; fixture được đổi gửi sau HTTP profile đầu tiên. Đây chưa chứng minh console
trước bootstrap được bảo toàn: F14 cần regression/audit cho early Stdout và ghi
rõ kết quả, không xem fixture thay đổi là fix backend. Async Logging object errors/
truncated strings vẫn partial từ F11; combined discovery→multi-app real socket/
console acceptance F21 vẫn pending. OS DNS/TCP trước handle chưa absolute cancel.

## Handoff F13 → F14 — Desktop Console và inspector

BẮT ĐẦU NGAY, chỉ F14, mỗi chat một tính năng. Không spawn subagent. Chỉ ghi
`/Users/quocan/Github/flutter_network_viewer (rust)`; không sửa Flutter tại
`/Users/quocan/Github/flutter_network_viewer`. Đọc AGENTS/progress/matrix trước.
Khảo sát Flutter flutter_console/inspector_performance/layout_preferences bằng
CodeGraph projectPath Flutter: context rồi một explore; đọc file cụ thể nếu output
thiếu. Trust index, không grep structural thay CodeGraph.

Tests executable TRƯỚC implementation: Console source/level/search phối hợp app,
group JSON/error, chọn detail/copy/raw, split riêng Network/Console, log update
không serialize raw chưa mở hoặc reset toolbar/focus/scroll, selection khác không
serialize raw chưa mở. Dùng console core F04, JSON core F02/F03 nếu cần; UI JSON
đầy đủ thuộc F15. GUI phải có input tests và native smoke cho phần thêm, không chỉ
backend unit tests. Audit early Stdout trước getVM/getIsolate/profile bootstrap:
fixture F13 đã gửi quá sớm và log không thấy; cần regression trước mọi fix, không
bỏ qua bằng đổi fixture để coi console parity pass.

GUI có sẵn `src/desktop.rs::DesktopUi`, `ShellStatus`, `Action`, `UiProbe` hình học
cho tests; renderer `show(ctx,&CaptureStore,&ShellStatus)->Vec<Action>`. Hiện Console
chỉ list + app filter/search message. AppSettings có console_inspector_width riêng.
`src/desktop_host.rs::DesktopApp` public ui/capture, native host production dùng
AppDiscovery<ThreadConnector,ThreadDiscovery>. `src/main.rs` chọn settings path;
không serialize/copy capture body toàn bộ mỗi frame. Source coordinator/store APIs
không đổi ở F13. Backend mọi app shared budget/cursors/Pause/Clear từ F12.

Tests F13 `tests/desktop_ui.rs` 11 input-driven tests, tổng206 pass; native fixture
và acceptance ở examples/f13_fake_vm.rs + examples/f13_native_smoke.rs, runner
`python3 tools/f13_native_smoke.py [output_dir]`. Test có pending completion RPC,
OS clipboard đọc lại, export/Clear và screenshot. Mỗi run cần fresh fixture; runner
lo teardown. Artifacts F13 trong docs/artifacts/f13; không ghi đè chúng bằng F14,
tạo artifacts/f14 và smoke riêng hoặc extend có version rõ. `arboard` dev dependency
đã có, egui/eframe0.33.3 glow(default_fonts,x11,wayland), không thêm dependency nếu
không cần. GUI chưa native accessibility tree/accesskit, F21 cần audit.

BẮT BUỘC env CARGO_HOME=/private/tmp/fnv-cargo RUSTUP_HOME=/private/tmp/fnv-rustup,
PATH=/private/tmp/fnv-cargo/bin:$PATH; ưu tiên --locked --offline. Nếu toolchain/cache
bị dọn, restore và test source mới, không dùng binary cũ báo pass. Loopback/native
smoke cần exec_command require_escalated. Rust chưa Git repo; không publish. Cập
nhật catalog/matrix/progress/README, ghi handoff chi tiết. Khi F14 pass regression/
fmt/clippy/UI/native, list_projects lấy ID thật và tạo đúng một chat F15 — Desktop
JSON và beautify, yêu cầu bắt đầu ngay; dùng Flutter Network project nếu Rust chưa
đăng ký. Nếu môi trường chặn thì ghi rõ, không đánh dấu pass/tạo chuỗi chat lỗi.

Đã tạo đúng một chat kế tiếp `F14 — Desktop Console và inspector`:
`01a1d2cb-2bc4-7aa2-b027-096111cef70d`, project Flutter Network, local, yêu cầu bắt
đầu ngay với handoff trên. Không tạo chat khác hoặc publish.

## F14 — Desktop Console và inspector

Đã hoàn thành phạm vi F14 trên macOS ARM64 ngày 2026-11-13. Chỉ ghi Rust,
không sửa Flutter, không spawn subagent/publish. Đọc AGENTS/matrix/handoff và khảo
sát Flutter bằng đúng một CodeGraph context + một explore; dùng core Console F04.

Console render grouped entries trước source/level/search/app filters. Search dùng
core search_text (display message, logger, error, stack, args, source). Controls có
All log sources/Stdout/Stderr/Logging và log/debug/info/warn/warning/error. Giữ
raw event/ANSI/Flutter prefixes trong export và raw; chỉ clean khi display. JSON
và Flutter exception dùng grouping bounded F04, exception có level error. List
selectable mở Console inspector: level/source/logger/isolate/timestamp/severity,
message/error/stack/args, Detail/Raw và Copy detail dùng OS clipboard egui.

Console inspector split khi width>=1100, modal khi hẹp, Escape/outside đóng.
Split dùng ID khác Network, kéo/persist console_inspector_width riêng; qua đổi
page/selection/restart không đè network_inspector_width. Raw chỉ serialize khi
mở, cache theo selected record + event, không encode lại mỗi frame hay do log
khác; grouped completion của record đang chọn invalidate đúng. Clear/app switch/
record eviction bỏ selection và raw cache. Stable egui IDs giữ search focus và
list scroll khi append; immediate-mode toolbar vẫn vẽ mỗi frame, state không reset.
Grouped list cache theo first/last ID; grouping còn clone events khi log history
thay đổi, list chưa virtualize, full rendering/cache stress acceptance thuộc F16.

Early Stdout audit: viết executable regression trước mọi sửa, chạy đỏ với [] thay
vì ["early"] khi notification đến trước getVM/getIsolate/profile. Xác nhận lỗi
coordinator bỏ event vì chưa có engine, không phải fixture workaround. Đã thêm
bounded pre-bootstrap notification queue theo count/bytes, drain một lần khi engine
sẵn sàng trước console batch. Clear/Pause/Stop/detach xóa queue; exited isolate
không nhận replay. Regression kiểm tra normal/Clear/Pause và bounded count/bytes.
Public store/coordinator APIs không đổi. Async Logging object/truncated-string
resolve vẫn partial như F11; queue này không bổ sung object-resolution RPC.

TDD: thêm bốn Console input tests trước implementation, chạy đỏ missing UI fields;
regression early Stdout chạy đỏ mất log. Thêm warn-level test chạy đỏ vì VM dùng
"warn", sau đó hỗ trợ cả warn/warning; test cũng kiểm tra live group completion
invalidate raw đúng một lần. Tổng 5 tests mới trong tests/desktop_ui.rs và 2
regressions trong tests/vm_coordinator.rs. UI tests gửi pointer/key/text/wheel vào
renderer thật: app/source/level/search phối hợp, logger/error/stack/args search,
JSON/exception grouping, selected detail/raw clipboard outputs, modal Escape/
outside, live append giữ focus/scroll/selection và raw serialization=0 khi chưa mở,
raw cache reuse/invalidation, Console/Network split kéo/lưu/restart độc lập.

Native executable examples/f14_native_smoke.rs wrap production DesktopApp, input
qua raw_input_hook, dùng fresh real WebSocket examples/f14_fake_vm.rs. Fixture gửi
Stdout/Stderr/Logging trong streamListen trước getVM (không trì hoãn đến profile).
8 records thành 4 grouped rows, bộ lọc source+level+search, JSON và exception
inspector, Logging metadata, detail/raw đọc lại OS clipboard bằng arboard, raw
cache count, resize860 modal/Escape, Clear, screenshot đều pass. Runner
python3 tools/f14_native_smoke.py [output_dir] teardown fixture và chống stale PNG.
Artifacts mới docs/artifacts/f14, không ghi đè F13. Screenshot đã xem trực tiếp;
font mặc định còn thiếu glyph box-drawing của Flutter exception, font coverage/
accessibility/packaging acceptance cần F21, text/raw vẫn giữ nguyên Unicode.

Validation: bắt buộc CARGO_HOME=/private/tmp/fnv-cargo,
RUSTUP_HOME=/private/tmp/fnv-rustup, PATH=/private/tmp/fnv-cargo/bin:$PATH.
- cargo test --locked --offline --quiet: **213 pass** (197 core +16 UI),0 fail/ignored.
- cargo fmt --check: pass.
- cargo clippy --all-targets --locked --offline -- -D warnings: pass.
- python3 tools/f14_native_smoke.py: native macOS pass, exit0.
- python3 tools/f13_native_smoke.py docs/artifacts/f14/f13-regression: native F13
  regression pass; log f14/f13-native-regression.log, không ghi đè artifacts F13.
- Loopback/native dùng require_escalated; toolchain/cache còn đầy đủ, test source
  mới, Cargo.lock không thay đổi.
- Hai lần full-suite đầu timeout tại bootstrap của hai coordinator_socket tests
  cũ. Targeted và hai full-suite reruns sau pass không đổi fixture/nới timeout;
  chưa xác định nguyên nhân scheduling. Không giấu lần fail hoặc suy ra stress pass.

Catalog/matrix/README đã cập nhật. Windows chưa build/chạy, live Dart SDK/WSS,
combined discovery→multi-app real socket, native accessibility và toàn-app stress
vẫn pending F21/F16. F15 JSON/editor, F16 previews/performance, F17 settings/help/
toasts còn pending. Không tuyên bố toàn app parity.

## Handoff F14 → F15 — Desktop JSON và beautify

BẮT ĐẦU NGAY, chỉ F15, mỗi chat một tính năng, không spawn subagent. CHỈ GHI
`/Users/quocan/Github/flutter_network_viewer (rust)`; không sửa Flutter ở
`/Users/quocan/Github/flutter_network_viewer`. Đọc AGENTS/progress/matrix trước.
Khảo sát Flutter json_beautify/json_body_height/json_find/virtual_json bằng
CodeGraph projectPath Flutter: context rồi một explore, đọc file cụ thể khi thiếu;
trust index. Rust chưa Git repo/CodeGraph, không tự init/publish.

Viết executable tests TRƯỚC implementation: Format/Ctrl+Enter; JSON/Tree/copy/find/
color; draft/folds/query giữ khi đổi tab/resize; lỗi dòng/cột; scroll ngang/dọc;
Ctrl+A copy toàn bộ không gutter. Dùng core F02/F03/F07; không mở rộng sang body
preview/performance toàn bộ F16/settings-help F17. GUI phải có input tests và
native smoke, backend tests không đủ parity. JSON navigation hiện là placeholder
trong src/desktop.rs, AppSettings có json_input_fraction. DesktopUi::show render
production, UiProbe geometry/content cho tests, DesktopApp trong desktop_host.rs
là host thật. Giữ public backend APIs nếu không cần đổi, không snapshot/serialize
mọi body mỗi frame. Request inspector body tabs vẫn text, JSON body interaction
nên xem spec F15 và ghi phần performance F16 còn lại rõ ràng.

F14 thêm selected_log, console_raw_serializations counter, cache grouped logs theo
first/last IDs, Detail/Raw raw cache theo event, Console source/level filters và
modal/split width>=1100. Network/Console vẫn chung query như Flutter. Toolbar
stable ID capture-search; UI append test giữ focus/scroll. Console grouping còn
clone history khi first/last thay đổi, list chưa virtualize: F16 cần tối ưu/stress.
Early Stdout đã regression-fix bằng bounded queue coordinator, Clear/Pause/Stop
purge; native F14 gửi 3 streams trước bootstrap đã pass. Async Logging object/
truncated resolve vẫn partial. Font box-drawing Flutter exception còn thiếu glyph
trong egui mặc định; F21 font/accessibility acceptance phải xem lại.

213 tests pass (197 core+16 input UI),0 fail/ignored; fmt/clippy all-targets pass.
Hai initial full runs có socket bootstrap timeout, targeted và subsequent full
runs pass không đổi fixture/deadline, chưa kết luận nguyên nhân. F14 native smoke
và artifacts docs/artifacts/f14; không ghi đè F13/F14 artifacts bằng F15. Có runner
F13/F14: python3 tools/f13_native_smoke.py, tools/f14_native_smoke.py [output_dir],
real WebSocket fixture, production host, native clipboard readback/screenshot.
F15 tạo artifacts/f15 và smoke tương ứng. Egui/eframe0.33.3 glow và arboard đã có.

BẮT BUỘC env CARGO_HOME=/private/tmp/fnv-cargo RUSTUP_HOME=/private/tmp/fnv-rustup
PATH=/private/tmp/fnv-cargo/bin:$PATH; ưu tiên --locked --offline. Nếu /tmp bị dọn
restore rồi compile/test source mới. Loopback/native cần require_escalated vì
sandbox EPERM/GUI/clipboard. Windows/live SDK/WSS/multi-app acceptance vẫn F21.
Cập nhật catalog/matrix/progress/README và handoff. Chỉ khi F15 regression/fmt/
clippy/UI/native pass: list_projects lấy ID thật, tạo đúng một chat F16 — Desktop
body previews và performance, yêu cầu bắt đầu ngay. Dùng project Flutter Network
nếu Rust chưa đăng ký; không tạo chuỗi chat khi môi trường chặn hoặc test chưa pass.

## F15 — Desktop JSON và beautify

Hoàn thành phạm vi F15 trên macOS ARM64 ngày 2026-11-13. Chỉ ghi thư mục Rust,
không sửa Flutter, không spawn subagent/publish. Đã đọc AGENTS/progress/matrix,
khảo sát Flutter bằng đúng một CodeGraph context + một explore. Explore chỉ trả
một phần source beautify nên đọc file cụ thể json_beautify_screen.dart và hai
test json_beautify/json_body_height để bổ sung đặc tả; trust index.

Thêm src/desktop_json.rs dùng core JsonDocument F02, JsonFindController F03 và
color_previews F07. DesktopUi giữ JsonWorkspace qua navigation; input hai pane,
Format hoặc Ctrl/Cmd+Enter, JSON null/scalar/array/object, invalid hiển thị message
và line/column UTF-16, giữ raw draft và kết quả hợp lệ trước. Divider kéo/persist
json_input_fraction với min pane 240 hoặc nửa chiều rộng khi cửa sổ quá nhỏ;
double-click reset 0.5. Input/output cuộn riêng, output ngang/dọc, text không wrap.
JSON/Tree có fold riêng; query/options/active match giữ qua mode, page, resize.
Find cục bộ Ctrl/Cmd+F ngay cả khi draft có focus, case/whole-word/regex/error,
Previous/Next wrap và F3/Shift+F3, highlight active/other, unfold ancestors và
scroll tới đúng vị trí ngang/dọc bằng UTF-16→character→galley coordinates.

Gutter/fold buttons và swatches nằm ngoài buffer selectable. OS clipboard Copy
JSON trả toàn bộ immutable formatted document ở cả JSON/Tree, kể cả folded rows.
Click swatch copy giá trị màu gốc. Ctrl/Cmd+A trong output chọn visible text và
Copy trả toàn document không gutter; selection thường dùng TextEdit read-only
multiline, không chèn màu/gutter vào dữ liệu. Fold glyph dùng +/- ASCII để tránh
missing glyph đã thấy trong screenshot lần đầu; F14 exception font coverage vẫn F21.

Inspector Request body/Response body valid JSON dùng chung JsonViewer trong scope
egui ID riêng, mỗi bên giữ mode/query/folds khi đổi tab; dữ liệu source đổi mới
parse document, refresh search và reset folds. Plain body và headers không có find.
Copy detail hiện vẫn trả body raw như F13, Copy JSON trả formatted document.
API backend/capture/coordinator không đổi; không serialize tất cả capture mỗi frame.
Hiện inspector vẫn tạo content string cho record đang chọn, JsonViewer build toàn
visible-by-fold text/LayoutJob mỗi frame, match styling scan từng character; CHƯA
virtualize viewport hay benchmark/stress. F16 phải tối ưu, không coi F15 là pass
performance toàn app. Body-type MIME/base64/image/form/raw switching thuộc F16.
Probe json.text cũng clone formatted text để hỗ trợ UI automation; F16 cần tránh
probe/measurement làm sai budget trên document lớn.

TDD: thêm sáu JSON input tests và chạy đỏ trước implementation (missing UiProbe.json;
đã sửa lỗi raw-string delimiter trong test trước lượt đỏ này), log tdd-red.log.
Sau đó thêm test active match retention/F3 (đỏ Shift+F3 đọc modifier sai), horizontal
find Unicode long line (đỏ chưa cuộn ngang) và local request/plain/header visibility.
Tổng 9 tests mới, tests/desktop_ui.rs có 25 tests. Test điều khiển pointer/key/text/
wheel vào renderer sản phẩm, không chỉ backend state. Những lỗi đã sửa: geometry
json-output không clip khiến automation wheel ở ngoài viewport, Shift+F3 dùng
event.modifiers, reveal scroll ngang bằng galley cursor UTF-16 mapping.

Native examples/f15_native_smoke.rs wrap production DesktopApp; runner
python3 tools/f15_native_smoke.py [output_dir] khởi động fresh f13_fake_vm thực
WebSocket và teardown fixture. Native kiểm tra Format/Ctrl+Enter/error giữ draft,
JSON/Tree/fold/find/highlight, active 2/2 giữ qua tab/resize860, swatch OS clipboard,
hai chiều scroll, Ctrl+A/Copy đọc lại OS clipboard nguyên document không gutter,
response body JSON/Tree/copy và PNG. Không dùng binary cũ. Có clipboard restore.
Smoke hook giữ Screenshot events khi inject input, tránh làm rơi completion event.
Ảnh đã xem trực tiếp: docs/artifacts/f15/native-beautify.png và native-json.png.
F13/F14 artifacts giữ nguyên; regression mới ở f15/f13-regression, f15/f14-regression.

Validation env bắt buộc CARGO_HOME=/private/tmp/fnv-cargo,
RUSTUP_HOME=/private/tmp/fnv-rustup, PATH=/private/tmp/fnv-cargo/bin:$PATH.
- cargo test --locked --offline --quiet: **222 pass** (197 core +25 UI), 0 fail/ignored
  trong full rerun cuối, docs/artifacts/f15/regression-rerun.log.
- cargo fmt --check: pass.
- cargo clippy --all-targets --locked --offline -- -D warnings: pass (clippy.log).
- python3 tools/f15_native_smoke.py: pass (native-smoke.log).
- python3 tools/f14_native_smoke.py docs/artifacts/f15/f14-regression: pass.
- python3 tools/f13_native_smoke.py docs/artifacts/f15/f13-regression: pass.
- Loopback/native dùng exec_command require_escalated. Cargo.lock không đổi,
  source mới compile bằng toolchain/cache còn sẵn. Rust chưa Git repo.

Các lần fail được giữ rõ: initial native lookup không tính isolate prefix request
ID; một rerun không thay draft đúng lúc (chưa reproduce ở run cuối); screenshot
completion bị smoke hook bỏ đã sửa. Full regression trước lượt cuối timeout tại
real_exit_interrupts_hung_rpc_same_socket_hot_restart_and_late_reply_is_ignored
(tests/coordinator_socket.rs:148 VM did not progress). Đây cùng nhóm bootstrap
flake đã ghi F14; targeted coordinator_socket và full rerun pass, không sửa fixture
hay nới deadline. Full run sớm hơn trước horizontal fix cũng pass. Chưa kết luận
nguyên nhân scheduling/GUI concurrency; không giấu failed runs hoặc suy ra stress.

Catalog/matrix/README cập nhật, giữ partial cho tests kết hợp visible-row rendering,
body-type/theme/performance. Windows, live Dart SDK/WSS, combined discovery multi-app,
native accessibility và font audit còn F21; async Logging object-resolution vẫn
partial F11. Không tuyên bố toàn app parity.

## Handoff F15 → F16 — Desktop body previews và performance

BẮT ĐẦU NGAY, chỉ F16, mỗi chat đúng một tính năng, executable tests TRƯỚC
implementation. Không spawn subagent. CHỈ GHI
/Users/quocan/Github/flutter_network_viewer (rust); không sửa Flutter. Đọc
AGENTS.md, docs/feature-matrix.md, docs/progress.md trước. Khảo sát Flutter body
previews/native_capture_list/native_json_view/native_image_view/inspector_performance
bằng CodeGraph projectPath /Users/quocan/Github/flutter_network_viewer: context rồi
một explore, chỉ đọc file cụ thể khi source chưa đủ. Rust chưa Git/CodeGraph,
không tự init/publish.

F16 spec: form/image/raw switch; live body invalidate đúng; list/JSON chỉ render
viewport visible rows; selected detail không đổi do event khác; Clear thả cache/
selection; overlay/clipping/disposal; host tạo lỗi fallback nếu dùng native host;
thumbnail ảnh nguồn vượt giải mã budget 64MiB F06. Dùng core F02/F03/F05/F06/F07.
Cần executable input UI + native smoke + stress/measurement, backend pass không
đủ parity. F15 JSON interactions hiện implemented, đừng regress Ctrl+A copy toàn
document không gutter, cross-line selection, Unicode offsets, find reveal ngang/
dọc và active match retention qua mode/page/resize. Theme/settings/help/toasts F17.

src/desktop_json.rs có JsonWorkspace và JsonViewer, JsonUiProbe trong UiProbe.json.
DesktopUi giữ workspace + body_json[request,response] theo selected request; mỗi
tab body có egui ID scope riêng. JsonViewer source comparison tránh parse lại
document không đổi, nhưng LayoutJob/display/gutter/swatches vẫn build toàn bộ
fold-visible rows mỗi frame, highlight scan character×matches, probe clone toàn
document. Cần tối ưu thật; đây chưa pass viewport virtualization/stress. Inspector
src/desktop.rs hiện tạo content string mỗi frame cho selected record, Raw serialize
selected mỗi frame từ F13. Console grouping cache first/last ID vẫn clone history
khi append, list chưa virtualize. Console raw lazy cache/counter từ F14 phải giữ.

src/desktop.rs::DesktopUi::show(ctx,&CaptureStore,&ShellStatus)->Vec<Action>
là renderer thật. src/desktop_host.rs::DesktopApp public ui/capture là host production.
Network sáu tab, split width>=1100/modal hẹp; Console có split riêng persisted,
filters/source/level/search, group JSON/exception, detail/raw. Probe widget geometry
dùng input-driven tests tests/desktop_ui.rs. Capture source APIs không đổi ở F15.
Core settings có json_input_fraction; body preview helpers xem src/form_body.rs,
src/image_body.rs, src/body_color.rs.

222 tests pass (197 core+25 UI), fmt/clippy all-targets pass, native macOS F15 +
F13/F14 regressions pass. F15 artifacts docs/artifacts/f15; không ghi đè chúng bằng
F16, tạo artifacts/f16. Native runners tools/f13_native_smoke.py,
f14_native_smoke.py, f15_native_smoke.py [output_dir]; F15 reuse fresh
examples/f13_fake_vm.rs, native wrapper examples/f15_native_smoke.rs. arboard đã có,
egui/eframe0.33.3 glow. F15 smoke hook giữ screenshot events khi inject input.
F15 native smoke đã đọc OS clipboard và xem screenshot. Có một draft replacement
smoke failure chưa reproduce ở run cuối; giữ diagnostics logs. Coordinator socket
bootstrap flake vẫn hiện 1 full run fail, targeted/full rerun pass không thay timeout.

Early Stdout bounded pre-bootstrap queue đã fixed F14, native fixture gửi 3 streams
trước getVM; Clear/Pause/Stop purge. Async Logging object/truncated resolution
partial, DNS/TCP trước handle chưa tuyệt đối cancel/deadline; Windows/live SDK/WSS/
combined discovery multi-app/accessibility/font coverage acceptance F21.
F06 ảnh nguồn>64MiB thumbnail thuộc F16 bắt buộc.

BẮT BUỘC env CARGO_HOME=/private/tmp/fnv-cargo RUSTUP_HOME=/private/tmp/fnv-rustup
PATH=/private/tmp/fnv-cargo/bin:$PATH; ưu tiên --locked --offline. Nếu /tmp bị dọn
restore toolchain/cache và compile/test source mới, không chạy binary cũ báo pass.
Loopback/native cần require_escalated vì sandbox EPERM/GUI/clipboard. Không publish.
Cập nhật catalog/matrix/progress/README và handoff. Chỉ khi F16 regression/fmt/
clippy/UI/native/performance pass: list_projects lấy ID thật, mở đúng một chat
F17 — Help, notifications và desktop settings, yêu cầu bắt đầu ngay. Dùng project
Flutter Network nếu Rust chưa đăng ký. Môi trường chặn thì báo rõ, không đánh dấu
pass/tạo chuỗi chat thất bại.

Đã tạo đúng một chat kế tiếp `F16 — Desktop body previews và performance`:
`01a1d2ea-19b8-7fe2-a934-c6191a9d5702`, project Flutter Network, local, yêu cầu
bắt đầu ngay, chỉ ghi Rust và đọc handoff trên. Không tạo chat khác hoặc publish.

## F16 — Desktop body previews và performance

Hoàn thành F16 trên macOS ARM64 ngày 2026-11-13. Chỉ ghi thư mục Rust; không sửa
Flutter, không spawn subagent, không init Git/CodeGraph hoặc publish. Đã đọc
AGENTS/matrix/handoff, dùng đúng một CodeGraph context rồi một explore ở Flutter;
đọc thêm network_body_section.dart và inspector_performance_test.dart vì explore
chỉ trả đoạn source. Không coi catalog/acceptance specification là test pass.

src/desktop_body.rs dùng F05 FormBody và F06 ImageAsset: Form/Raw và Image/Raw,
URL fields lặp/Unicode, multipart file metadata, incomplete notice, bodyPreview
fallback và omitted reason. Request/response giữ resource riêng; body identity
(body/bodyPreview, MIME, encoding, omission/limit) chỉ invalidate khi nội dung
liên quan đổi. Status-only update giữ preview; MIME đổi bỏ preview cũ. JSON và
scalar/form colors dùng core F02/F03/F07; swatch copy giá trị gốc. Copy detail
luôn giữ raw capture, không lấy text rút gọn của automation probe.

Ảnh decode qua một worker toàn app, queue tối đa một task. Mỗi BodyViewer có weak
lifetime; task chưa chạy bị bỏ nếu owner chết, kết quả cũ không thể bind vào body
mới. Task đang decode hoàn tất rồi drop nếu đã bị dispose; không hứa cancel
codec giữa chừng. Chuyển RGBA từ worker không clone buffer; một texture cho
inline/enlarged, zoom/pan/Fit không decode/upload lại. Modal dùng cùng clipped
egui surface, nested inspector hẹp/Escape đã test; không có child platform view
hay native host create/IPC cần fallback. Corrupt pixels/decoder failure hiển thị
unavailable và vẫn cho Raw. Clear/replacement/drop thả texture và receiver.

src/image_thumbnail_macos.rs gọi ImageIO/CoreGraphics/CoreFoundation với RAII,
thumbnail tối đa 4096px, EXIF transform và straight-alpha RGBA. Source 9000x9000
vượt 64MiB có thumbnail thật trong executable F06 test đã đổi từ reject sang
accept trên macOS. Rust full-frame decode vẫn guard 64MiB; nền tảng khác giữ
reject over-budget, backend thumbnail Windows và chạy Windows còn F21. Không
khẳng định peak allocation bên trong ImageIO bị giới hạn đúng 64MiB: budget này
là Rust decoded raster/output; native codec có working memory riêng.

Network/Console chỉ tạo widgets hàng trong viewport. Console summary cố định
một dòng, multiline đầy đủ ở detail. Grouping mượn input thay vì clone cả history,
append chỉ regroup group chạm tail 200 raw records; tests đếm bounded work. Raw
console và detail cache giữ qua unrelated append, kể cả selection trong tail.
NetworkRecord thêm request/completion revision nội bộ, serde(skip), không đổi
wire/export. Inspector cache theo selected key/revisions/tab; Raw chỉ serialize
khi mở và chỉ lại khi selected record đổi. Store clear/removal epoch invalidate
selection/cache ngay cả khi cùng IDs được ingest lại trước frame kế tiếp.

JSON viewport tự vẽ từng row và dùng selection endpoints (row, character), giữ
UTF-16 core→Unicode character→galley mapping, kéo selection nhiều dòng và
Ctrl+A/Copy toàn document kể cả folded/offscreen rows. Các tương tác F15 giữ:
JSON/Tree/folds/find/options/F3, swatches riêng, horizontal/vertical reveal và
active match qua mode/tab/resize. Row/fold/color indexes cache theo document và
fold mode; match lookup binary search thay vì character×all-matches. Source JSON
lỗi cũng cache kết quả, không reparse mỗi frame. Probe JSON tối đa 65.536 chars,
inspector tối đa 8.192 chars; Copy lấy immutable full document. Parse/index lần
đầu vẫn đồng bộ: xem số đo cold riêng, không suy ra tất cả frame đều dưới 16ms.

TDD: 3 tests đầu chạy đỏ do missing probe/counters trước implementation
(tdd-red.log). Các bài bổ sung chạy đỏ thật: Unicode drag selection trả rỗng
(ui-selection.log), status-only update rebuild body (tdd-body-identity-red.log),
Clear→same IDs giữ stale selection (tdd-clear-generation-red.log), Raw/console
work counters (tdd-raw-color-red.log, tdd-console-tail-red.log). Đã sửa rồi chạy
pass. Nested-modal test mở rộng sang 860/1440 đã pass ngay (nested-modal-ui.log),
không gọi đó là red. Image pan/Fit có input-driven assertions riêng trong test
ảnh hiện có. Tổng **12 UI tests F16 mới**, tests/desktop_ui.rs có **37 tests**.

Native tools/f16_native_smoke.py + examples/f16_fake_vm.rs/f16_native_smoke.rs
chạy fresh WebSocket, production DesktopApp, OS clipboard readback/restore,
Form/Raw, live Form→updated/image→corrupt qua RPC rồi poll thật, image enlarge/
zoom, 20k-row JSON viewport/offscreen find/Unicode full copy, Clear resources,
screenshots. Hook giữ Screenshot completion khi inject events. Đã xem trực tiếp
native-body.png và native-json.png; glyph coverage/accessibility audit vẫn F21.
Native creation-failure assertions Flutter không có đường code tương đương vì
egui là renderer chính; catalog ghi rõ kiến trúc thay thế thay vì fake native pass.

Đo thật, debug build macOS:
- examples/f16_stress.rs: 4 vòng × 4.500 request + JSON 20.000 phần tử, 200 warm
  frames/vòng. 20.002 JSON rows chỉ dựng 39; cold 108–112ms, warm p95 3,70–4,31ms.
  RSS baseline 9.6MiB, cuối 60.4MiB; warm growth cao nhất 64KiB trong một vòng.
  Assert p95<50ms, warm RSS tăng<64MiB, 4 vòng tổng tăng<256MiB. Không coi RSS
  allocator/font cache sau Clear là body còn được sở hữu; có assertions riêng
  về selection/texture/detail resource. Đây là 4 vòng đo, không chứng minh vô hạn.
- Native host: 180 frames, chỉ dựng 36/20.002 rows; native-measurement.json ghi
  p50/p95/RSS của run cuối (p95 khoảng 3ms). Native RSS gồm GL/OS window/clipboard.
- Không suy performance GUI từ unit tests; có cả renderer input và native host.

Validation với CARGO_HOME=/private/tmp/fnv-cargo,
RUSTUP_HOME=/private/tmp/fnv-rustup, PATH=/private/tmp/fnv-cargo/bin:$PATH:
- cargo test --locked --offline --quiet: **234 pass (197 core +37 UI)**,
  0 fail/ignored, docs/artifacts/f16/regression-final.log.
- cargo fmt --check, cargo clippy --all-targets --locked --offline -- -D warnings:
  pass, fmt.log/clippy.log.
- cargo run --locked --offline --example f16_stress -- <artifact JSON>: pass,
  stress.log/stress-measurement.json.
- python3 tools/f16_native_smoke.py: pass, native-smoke-final.log,
  native-measurement.json/native-body.png/native-json.png.
- F13/F14/F15 native regressions pass trên F16 renderer, artifacts ở
  docs/artifacts/f16/f13-regression, f14-regression, f15-regression; không ghi đè
  artifacts các chat F13/F14/F15 trước. Probe pan field được thêm sau lượt native
  regression đó; full regression + F16 native được compile/run lại trên source
  cuối. Cargo.lock không đổi. Loopback/GUI/clipboard/process RSS dùng escalation.

Failures/limits ghi rõ: native initial lookup dùng sai session prefix đã sửa
bằng lấy key thật từ capture (native-initial-failure.log); stress lần đầu có lỗi
closing delimiter rồi fixture thiếu sdk.schemaVersion (đã sửa), process RSS ps
bị sandbox EPERM (stress-sandbox-denied.log), rerun có escalation pass. Đây là
fixture/tool issues, không gán thành pass. Red input tests nêu trên phát hiện
lỗi implementation và được giữ logs. Full final F16 regression không thấy
coordinator bootstrap flake cũ F14/F15, không khẳng định đã sửa nguyên nhân.

README/catalog/matrix cập nhật. Settings/language/theme/help/toasts F17; Windows
build/run và thumbnail backend, live Dart SDK/WSS, combined discovery→multiapp,
accessibility/font acceptance F21. Async Logging object-resolution partial F11,
OS DNS/TCP trước socket handle chưa tuyệt đối cancel/deadline. Không tuyên bố
parity toàn app, không publish.

## Handoff F16 → F17 — Help, notifications và desktop settings

BẮT ĐẦU NGAY, chỉ F17, mỗi chat đúng một tính năng, executable tests TRƯỚC
implementation. Không spawn subagent. CHỈ GHI
/Users/quocan/Github/flutter_network_viewer (rust); không sửa Flutter. Đầu tiên
đọc AGENTS.md, docs/feature-matrix.md, docs/progress.md (handoff cuối file).
Khảo sát Flutter help_dialog/viewer_notification/viewer_dropdown/settings bằng
CodeGraph projectPath /Users/quocan/Github/flutter_network_viewer: context rồi
một explore, chỉ đọc file cụ thể khi source thiếu. Rust chưa Git/CodeGraph,
không tự init/publish. Project Flutter dùng nếu Rust chưa đăng ký.

F17 spec: VI/EN, light/dark; help tách manual connect, discussion mở đúng URL;
toast 4 loại, tối đa 3 dòng, góc trái dưới; success/info 3s, warning/error/confirm
chờ; cancel=false; replace/dispose hủy timer; clipboard thật; cửa sổ hẹp scroll.
Dùng AppSettings F08 và giữ persistence layout/tab/filter core behavior. Phải có
input UI tests và native smoke, không suy parity từ backend tests. Existing UI
còn English literals, default light native context; F17 cần nối theme/locale và
localize cả omitted/body notices. Không sửa backend API để né UI spec.

F16 src/desktop_body.rs BodyViewer: Form/Image/Raw, single decode worker queue1,
weak owner discard, shared texture inline/modal, zoom/pan/Fit, corrupt fallback,
live identity invalidation body/MIME/encoding/omission; status-only không reload.
ImageIO thumbnail source>64MiB implemented macOS; Windows codec fallback guard
còn giữ reject, backend thumbnail + build/run acceptance F21. ImageIO working
memory không có total64MiB guarantee; output raster tối đa4096²×4. Egui dùng
cùng surface nên child native host failure/IPC assertions inapplicable.

DesktopUi cache selected inspector theo NetworkRecord internal serde(skip)
request/completion revisions; Raw lazy, console grouping tail200, all lists và
JSON viewport-only widgets/galleys. Store revision clear/removal epoch dọn
selection/cache ngay cả reingest same keys. BodyViewer và body_json hai bên có
scope riêng. Probe text bounded: json.text65.536chars, inspector8.192chars;
Copy luôn full data. JSON selection custom endpoints Unicode character+row,
cross-line drag, auto-scroll, Ctrl+A copy full formatted document không gutter;
đừng regress folds/find/F3/active retention/horizontal reveal/Unicode swatches.
Parse/index JSON cold vẫn synchronous108–112ms ở20k rows debug; warm p953,7–4,3ms,
39 rows renderer/36 native; artifacts có RSS thật, không suy infinite stability.

234 tests pass (197 core+37 UI),0 fail/ignored; fmt/clippy all-targets pass;
F16 native fresh WebSocket + clipboard/live Form/image updates/screenshots/
measurement pass, F13/F14/F15 native regressions pass. Artifacts docs/artifacts/f16,
không ghi đè; tạo artifacts/f17. Runners tools/f13_native_smoke.py,
f14_native_smoke.py, f15_native_smoke.py, f16_native_smoke.py [output_dir].
Production host src/desktop_host.rs::DesktopApp public ui/capture; renderer
DesktopUi::show(ctx,&CaptureStore,&ShellStatus)->Vec<Action>. Native hook phải giữ
Screenshot events khi inject input. arboard dev dep sẵn. F16 fixture
examples/f16_fake_vm.rs có form/image/large-json20k và f13.complete RPC làm
live Form→updated/image→corrupt; fixture F13/F14/F15 giữ nguyên.

BẮT BUỘC env CARGO_HOME=/private/tmp/fnv-cargo RUSTUP_HOME=/private/tmp/fnv-rustup
PATH=/private/tmp/fnv-cargo/bin:$PATH; ưu tiên --locked --offline. Nếu /tmp bị dọn
restore toolchain/cache và compile source mới, không dùng binary cũ báo pass.
Loopback/native/clipboard/ps RSS cần require_escalated. F14/F15 socket bootstrap
flake có lịch sử nhưng full F16 final pass, không khẳng định fix. Native failure
ban đầu F16 do hardcoded key đã sửa; logs giữ. Windows/live SDK/WSS/multiapp/
font/accessibility còn F21; async Logging object-resolution vẫn partial.

Cập nhật catalog/matrix/progress/README và handoff cuối. Chỉ khi F17 regression/
fmt/clippy/UI/native pass: list_projects lấy ID thật, mở đúng một chat F18 — MCP
read-only HTTP, yêu cầu bắt đầu ngay. Không tạo chuỗi chat khi môi trường chặn
hoặc test chưa pass; không publish/gửi tin nhắn/automation.

Đã tạo đúng một chat kế tiếp `F17 — Help, notifications và desktop settings`:
`01a1d303-c96a-7c81-b949-4b2059f014a2`, project Flutter Network, local, yêu cầu
bắt đầu ngay, chỉ ghi Rust và đọc handoff trên. Không tạo chat khác hoặc publish.

## F17 — Help, notifications và desktop settings (2026-11-13)

Đã triển khai trong Rust, không sửa Flutter, không spawn subagent. Đã đọc Flutter
qua CodeGraph context → một explore; đọc riêng viewer_notification.dart và ARB
vì output explore bị trim. Không init Git/CodeGraph Rust, không publish.

- `src/desktop_support.rs`: help dialog tách khỏi manual connect, hướng dẫn 5 bước,
  OpenUrl đúng GitHub Discussions; luôn có địa chỉ selectable và Copy address để
  dùng khi máy không có browser. OpenUrl được kiểm bằng platform command, không
  tự mở browser trong native test.
- Settings nối AppSettings F08: VI/EN, light/dark, auto discovery và URL
  DevTools/VM Service; normalize trước khi lưu và dispatch Connect. Persist lỗi
  giữ giá trị đã commit và hiện Error; capture/filter/draft giữ khi đổi ngôn ngữ
  hoặc background scan cập nhật. MCP UI/server chưa làm (F18).
- Locale/theme áp dụng trên production DesktopUi, bao gồm toolbar, filters,
  inspector tabs/labels, JSON controls, Form/Image/Raw và notices body bị bỏ qua,
  ảnh lỗi/form chưa đầy đủ. Payload/copy nguyên vẹn. Chuyển locale chỉ invalidates
  cache nhãn detail, không bỏ preview/JSON state. Bundle JetBrains Mono Regular
  từ Flutter kèm OFL/README; glyph test kiểm dấu tiếng Việt cho cả hai font family.
- Notifications bốn loại/color, ba hàng title/description/actions, góc trái dưới
  cách 20px, width min(380,window−40); dài thì wrap/scroll. Đây là ba hàng bố cục
  như Flutter, không cắt description thành ba dòng chữ. Success/info tự đóng sau
  đúng 3s tính từ frame mount; warning/error/confirmation mặc định chờ. OK/Confirm
  true; Cancel/timeout/replace/drop false. Override auto-hide/delay có test,
  delay âm reject, zero đóng ngay. Deadline sở hữu bởi session, UI clock + repaint,
  không tạo thread/timer detached nên replace/drop không thể đóng session kế tiếp.
  Toast không modal; copy entry points ghi native platform clipboard và toast Success.
- Help/settings giữ header/actions ngoài scroll; narrow 360×300 có input/native
  wheel + button checks. Thông báo scroll đến Confirm và chọn được.

Test được viết executable trước implementation: `docs/artifacts/f17/red.log`
(báo thiếu module/API), `red-options.log`, `red-glyphs.log`. Native screenshot
phát hiện font egui mặc định thiếu dấu VI nên thêm font và glyph assertion.
`tests/f17_ui.rs` có **15 renderer/input tests**. Không dùng ignored/placeholder.

Verification (env CARGO_HOME=/private/tmp/fnv-cargo,
RUSTUP_HOME=/private/tmp/fnv-rustup, PATH=/private/tmp/fnv-cargo/bin:$PATH):
- `cargo test --locked --offline --quiet`: **249 pass (197 core +52 renderer/UI)**,
  0 fail/ignored; `docs/artifacts/f17/regression-final.log`.
- `cargo fmt --check`, `cargo clippy --all-targets --locked --offline -- -D warnings`:
  pass, `fmt.log`, `clippy.log`. Clippy đầu phát hiện collapsible_if, đã sửa,
  giữ `clippy-first.log`.
- `python3 tools/f17_native_smoke.py`: production DesktopApp, clipboard OS đọc lại
  đúng URL Discussion, lưu/tải lại VI+dark, help/settings/notification hẹp, timeout
  3s wall clock, confirm/cancel và error chờ, nhập URL từ Settings và nhận 3 records
  từ fresh WebSocket fixture F16. `f17-native-final.log`, `native-result.json`,
  help-en-light/help-vi-dark/settings-vi-dark/help-narrow/settings-narrow/
  notification-narrow.png. Smoke compile source qua cargo mỗi lần, tự đóng;
  restore clipboard text cũ nếu có.
- Native F13/F14/F15/F16 regression pass trên renderer F17, artifacts mới ở
  `docs/artifacts/f17/f13-regression` ... `f16-regression`; logs f13-native-final,
  f14-native-final, f15-native-fixed, f16-native-final. Không ghi đè F13–F16 cũ.

Failures giữ rõ: native-first lấy geometry ngay sau resize nên scroll assertion
fail; runner thêm settling frames. Native-second hoàn thành nhưng còn một frame
sau Close đi vào nhánh panic; runner cho phép frame closing. Đây là runner fixes,
không bỏ assertion. `ui-third` dùng bytes ảnh có thể decode async chưa xong;
fixture lỗi được đổi thành invalid base64 để test notice deterministic.
Khi thêm font, native F15 phát hiện probe TextEdit trả rect toàn nội dung dài,
center nằm ngoài viewport sau cuộn (diagnostic y −4286..4252). Red regression mới
`red-json-input-bounds.log` tái hiện; production probe nay intersect clip_rect,
không thay text editing/backend và không nới assertion. Native F15 rerun pass.
Giữ f15-native-first-failure/f15-native-diagnostic/f15-native-fixed logs.
Small-window test phải dismiss toast đang phủ Close rồi mới đóng Settings, đúng
cách người dùng thao tác. Chưa suy ra parity toàn app/Windows từ macOS tests.

README/matrix/catalog cập nhật mapping F17. Còn F18 MCP read-only HTTP, F19
structured diagnostics/resource samples, F20 updater và F21 build/release/acceptance.
Các giới hạn cũ còn nguyên: Windows chưa build/run và thiếu thumbnail backend;
ImageIO source>64MiB có bounded output nhưng không guarantee tổng codec working
memory≤64MiB; live Dart SDK/WSS/combined discovery→multiapp/accessibility còn F21.
Font VI đã kiểm glyph/native macOS, chưa font/accessibility acceptance mọi OS.
Initial large JSON parse/index vẫn synchronous; async Logging object resolution
partial; OS DNS/TCP trước socket handle chưa tuyệt đối cancel/deadline. Không
thấy bootstrap flake ở full F17 regression, không khẳng định đã fix nguyên nhân.

## Handoff F17 → F18 — MCP read-only HTTP

BẮT ĐẦU NGAY; chat này chỉ F18. Mỗi chat đúng một tính năng, executable tests
TRƯỚC implementation, pass regression/fmt/clippy/UI/native rồi mới tạo đúng một
chat F19. Không spawn subagent. CHỈ GHI
`/Users/quocan/Github/flutter_network_viewer (rust)`; không sửa Flutter.
Đọc AGENTS.md, docs/feature-matrix.md, docs/progress.md (handoff cuối).

Khảo sát Flutter viewer_mcp_server/settings bằng CodeGraph projectPath
`/Users/quocan/Github/flutter_network_viewer`: context rồi một explore; đọc file
cụ thể nếu output thiếu. Rust chưa Git/CodeGraph, không init/publish.
F18 spec: MCP off mặc định, AppSettings F08 opt-in/validated port38943;
5 read-only tools app/request list/detail/log/stats, protocol versions,
loopback-only, origin/host validation, routes/version errors, port conflict/retry/
races, stop thả port. UI settings toggle/port/retry/copy Codex+Claude configs đúng
port và VI/EN/light/dark/narrow scroll; executable HTTP integration + renderer
input + native OS clipboard, backend tests không đủ UI parity. Không làm F19/F20.

F17 production support: `src/desktop_support.rs::SupportUi::show` nhận settings,
Notifications, UiProbe, actions. Settings hiện language/theme/auto discovery/URI,
MCP UI chưa có. `Notifications::show(kind,description,confirmation)` trả receipt;
`show_with_options` cho override hide/delay; replace/drop false, mounted deadline
qua ctx time, không background timer. `desktop_support::copy` gửi OS platform copy
và marker success toast; cuối DesktopUi::show consume marker. `apply` nối locale/
theme + bundled font một lần; `tr(ctx,text)` từ desktop_vi.json, help riêng JSON.
Phải localize nhãn MCP thêm vào table. AppSettings F08 đã persist MCP opt-in/port;
`src/desktop_host.rs::DesktopApp` có ui/capture public và dispatch Action;
DesktopUi::show(ctx,&CaptureStore,&ShellStatus)->Vec<Action>. Thêm MCP state/status/
actions hợp lý, đừng fake server/UI pending thành pass.

F16 preview/cache/JSON behavior giữ nguyên; F17 locale change invalidates detail
label caches nhưng giữ preview/find/folds. JSON input probe đã clip viewport,
đừng revert vì native F15 click large draft cần điểm visible. Body payload/copy
không dịch. Egui chung surface không child native hosts/IPC. Native input hook phải
giữ Screenshot events khi inject. arboard dev dependency sẵn, eframe/egui0.33.3.

249 tests pass (197 core +52 UI/renderer),0 fail/ignored; fmt/clippy all-targets
pass; F17 native freshVM/clipboard/settings reload/VI glyph/narrow/timers pass;
F13/F14/F15/F16 native regression pass. Artifacts docs/artifacts/f17, không ghi đè;
F18 tạo docs/artifacts/f18. Runners tools/f13_native_smoke.py ... f17_native_smoke.py
[output_dir]. F17 runner tự mở fresh f16_fake_vm và native host. Tất cả cargo dùng
CARGO_HOME=/private/tmp/fnv-cargo RUSTUP_HOME=/private/tmp/fnv-rustup
PATH=/private/tmp/fnv-cargo/bin:$PATH, ưu tiên --locked --offline. Nếu /tmp bị dọn
restore toolchain/cache và compile source mới. Loopback/native/clipboard/ps RSS
cần exec_command require_escalated. Không chạy binary cũ rồi báo pass.

Cập nhật catalog/matrix/README/progress và handoff cuối. Chỉ khi F18 đầy đủ pass:
list_projects lấy ID thật và tạo đúng một chat F19 — Diagnostics và resource usage,
yêu cầu bắt đầu ngay. Dùng project Flutter Network nếu Rust chưa đăng ký. Nếu
môi trường chặn ghi rõ, không đánh dấu pass hoặc tạo chuỗi chat thất bại.
Không publish/gửi tin nhắn/automation.

Đã tạo đúng một chat kế tiếp `F18 — MCP read-only HTTP`:
`01a1d31c-24fe-7c52-9886-372c467c34d1`, project Flutter Network, local, yêu cầu
bắt đầu ngay và chỉ ghi Rust. Không tạo chat khác hoặc publish.

## F18 — MCP read-only HTTP (2026-11-13)

Implemented trên macOS; chỉ ghi Rust, không sửa Flutter, không spawn subagent,
không init Git/CodeGraph hoặc publish. Đã dùng Flutter CodeGraph context → đúng
một explore, đọc các file cụ thể bị trim (MCP tools/test, settings và model).

- `src/mcp.rs`: loopback `127.0.0.1` HTTP `/mcp`, JSON responses; initialize,
  ping, notifications/initialized, tools/list/call và DELETE session. Protocols
  2024-11-05, 2025-03-26, 2025-06-18, 2025-11-25; initialize negotiate version,
  unknown version header 400, route 404, GET 405 (không có SSE notifications).
  Host chỉ localhost/127.0.0.1, validate port nếu có; từ chối mọi Origin, duplicate
  headers, transfer-encoding và malformed framing. Content-Length HTTP/1.1;
  16 connections, request≤1MiB, header≤16KiB, deadline5s, read/write nonblocking.
  Server phục vụ local MCP clients; không mở external interface/CORS/cloud relay.
- Năm tools đúng tên Flutter, annotations readOnly=true/destructive=false:
  list_sessions, list_requests, get_request, list_logs, get_capture_summary.
  Scope session/time inclusive, kết hợp filters, newest-first requests/logs,
  pagination, bounded body/log chunks, arguments opt-in, error/stack/logger search,
  status/method/log groups và10 slowest completed. Body JSON format trước chunk,
  UTC timestamps, default session labels và raw response byte counts đối chiếu
  Flutter. UTF-16 offsets; chunk cắt giữa surrogate pair được encode U+FFFD vì
  Rust/UTF-8 JSON không biểu diễn unpaired surrogate; full pair giữ đúng emoji.
  Captured app data được ghi rõ là untrusted, không là instructions.
- Production `DesktopApp` sở hữu public `mcp`, configure/poll mỗi frame, query
  trực tiếp CaptureStore + online set hiện tại; không snapshot/clone toàn capture,
  không fake UI. Bind/start/stop là thao tác synchronous serialized bởi owner;
  không có pending async bind có thể bật lại sau stop. Conflict giữ error cho
  Retry, không retry nền mỗi frame. Reconfigure/stop/drop đóng listener, active
  sockets và sessions; retry action cấu hình đúng settings đã commit.
- Settings opt-in mặc định false/port38943 dùng AppSettings F08, validate1..65535,
  show status/error/retry, save/restart, selectable endpoint, copy Codex + Claude
  HTTP/stdio bridge đúng port qua platform clipboard + success toast. MCP labels
  VI/EN, light/dark, wrapped controls và narrow scroll. Modal có max_height theo
  viewport để geometry không phụ thuộc height frame cũ. F17 narrow manual test
  nay cuộn từ đáy MCP về phần Connection trước khi click, giữ nguyên assertions.

Test executable viết trước implementation: `red.log` thiếu module/API,
`red-ui.log` thiếu RetryMcp/status; bổ sung red-summary-parity/red-format-config
cho các chi tiết Flutter, red-opening + opening-diagnostic tái hiện modal dịch
geometry. Không ignored, placeholder hoặc acceptance catalog giả làm pass.
`tests/f18_mcp.rs`:7 core/HTTP tests; `tests/f18_ui.rs`:3 renderer/input tests.

Verification final (env CARGO_HOME=/private/tmp/fnv-cargo,
RUSTUP_HOME=/private/tmp/fnv-rustup, PATH=/private/tmp/fnv-cargo/bin:$PATH):
- `cargo test --locked --offline --quiet`: **259 pass (204 core/HTTP +55 UI)**,
  0 fail/ignored; `docs/artifacts/f18/regression-final.log`.
- `cargo fmt --check`, `cargo clippy --all-targets --locked --offline -- -D warnings`:
  pass, `fmt.log`, `clippy-final.log`. Không thêm dependency.
- F13/F14/F15/F16/F17/F18 native sequential regressions pass, current source
  được cargo compile, artifacts `docs/artifacts/f18/f13-regression` ...
  `f17-regression`, `native-final`; logs f13-native-final ... f18-native-final.
- Sau metadata/config refinements, final source F18 native rerun pass:
  `native-final-source.log`, `native-final-source/native-result.json` và screenshots.
  Production DesktopApp + fresh WebSocket fixture, input toggle→conflict→retry,
  port edit/restart, settings reload, native clipboard Codex/Claude đọc lại đúng,
  HTTP summary đọc3 requests live, VI/dark/narrow scroll và off thả port.
  Không chạy external Codex/Claude client thật; config/HTTP contract đã kiểm,
  live SDK và Windows acceptance vẫn F21. Không ghi đè artifacts F13–F17 cũ.

Failures giữ trong artifacts: first full run gặp2 coordinator_socket bootstrap
"VM did not progress" (flake cũ), full reruns sau pass không thay coordinator.
Regression second phát hiện Settings modal size oscillation khiến click Language/
Theme lệch; fixed production max_height và thêm red test. Native-first đến bước
narrow clipboard còn toast Save che button; runner dismiss toast trước click,
reruns pass. Pending socket teardown trên macOS có thể EOF hoặc ConnectionReset;
assertion chấp nhận đúng hai biểu hiện đóng socket. Khi thêm formatted body,
raw byte count assertion bắt regression37 thay28 và đã sửa riêng raw byte count.
Không bỏ assertion và không suy pass mọi OS từ native macOS.

README/matrix/catalog mapping cập nhật F18. F19 diagnostics/resource usage và
F20 updater chưa triển khai. Các giới hạn cũ vẫn còn: Windows/thumbnail backend,
live SDK/WSS/combined discovery→multiapp/accessibility/font mọi OS F21; ImageIO
bounded output không guarantee tổng codec working memory≤64MiB; cold JSON parse/
index synchronous; async Logging object resolution partial; DNS/TCP trước socket
handle chưa có absolute cancel/deadline. MCP queries chạy trên owner/UI thread;
không phải benchmark latency cho nhiều concurrent clients. Không có background
server thread hoặc detached timer cần handoff/dispose riêng.

## Handoff F18 → F19 — Diagnostics và resource usage

BẮT ĐẦU NGAY. Mỗi chat đúng một tính năng, executable tests TRƯỚC implementation;
chat kế tiếp CHỈ F19. Không spawn subagent. CHỈ GHI
`/Users/quocan/Github/flutter_network_viewer (rust)`; không sửa Flutter
`/Users/quocan/Github/flutter_network_viewer` dù project chat dùng Flutter Network.
Đọc AGENTS.md, docs/feature-matrix.md và handoff cuối docs/progress.md ở Rust.
Khảo sát Flutter viewer_diagnostics/process_resource_usage qua CodeGraph với
projectPath Flutter: context rồi đúng một explore, đọc file cụ thể nếu trim.
Rust chưa Git/CodeGraph, không tự init/publish.

F19 spec: structured logs rotate có budget; không chứa body/header/console hoặc
VM token/URL có auth; path lỗi không phá capture. Memory sample định kỳ, lỗi xóa
sample và phục hồi, disposal dừng timers/workers. Viết deterministic/executable
contracts và integration trước implementation; nếu có UI diagnostics/resource
usage phải có renderer/input + native test tương ứng. Nối production lifecycle,
không làm updater F20/release F21 trong chat F19. Đọc Flutter làm chuẩn cụ thể,
không đoán chỉ từ catalog.

F18 production: src/mcp.rs McpServer do DesktopApp public mcp sở hữu;
configure(enabled,port,restart) synchronous owner-serialized, poll(&CaptureStore,
&HashSet<String>) mỗi frame. Truy vấn live store không clone snapshot, socket
nonblocking16/deadline5s; stop/drop thả listener+active sockets. DesktopApp::update
cập nhật SupportUi.mcp_status rồi render; Action::RetryMcp dispatch configure true.
Settings port changes/toggle lưu AppSettings, apply lifecycle frame kế tiếp,
retry explicit. SupportUi public trên DesktopUi; MCP controls chung settings
scroll, max_height độc lập bounds frame cũ (đừng revert). MCP default off38943,
5 read-only tools, configs clipboard có Codex/Claude HTTP + Claude stdio bridge.

F17 SupportUi/Notifications giữ owned mounted deadlines3s, warning/error chờ,
Cancel/replace/drop=false, no detached timers. desktop_support::copy dùng native
platform output + marker success toast; apply/tr dùng desktop_vi.json + font
JetBrains Mono/OFL. Payload/copy không dịch. Native input hook giữ Screenshot
events. F16 body/JSON caches và clip rect input F15/F17 giữ nguyên; egui surface
không child native hosts/IPC. AppDiscovery/desktop capture APIs không đổi F18.

259 tests pass (204 core/HTTP+55 UI),0 fail/ignored; fmt/clippy all-targets pass;
native F13–F18 regressions + final F18 source HTTP/clipboard/settings/port release
pass. Artifacts docs/artifacts/f18, tạo mới docs/artifacts/f19. Runners
python3 tools/f13_native_smoke.py ... f18_native_smoke.py [output_dir]; F17/F18 tự
mở fresh f16_fake_vm. Nếu native narrow click bị toast che, dismiss trước;
đừng bỏ assertion hoặc suy clipboard pass từ platform output backend.

BẮT BUỘC CARGO_HOME=/private/tmp/fnv-cargo RUSTUP_HOME=/private/tmp/fnv-rustup
PATH=/private/tmp/fnv-cargo/bin:$PATH; ưu tiên --locked --offline. Nếu /tmp bị dọn,
restore toolchain/cache và compile SOURCE mới. Loopback/native/clipboard/ps RSS
cần exec_command require_escalated. Coordinator_socket bootstrap flake cũ có thể
xuất hiện; ghi failure, rerun có bằng chứng, không khẳng định nguyên nhân đã fix.

Cập nhật catalog/matrix/README/progress/handoff. Chỉ khi F19 regression/fmt/clippy/
integration/UI/native phù hợp đầy đủ pass: list_projects lấy ID thật, mở đúng một
chat F20 — Updater, yêu cầu bắt đầu ngay. Rust chưa đăng ký thì dùng Flutter
Network. Nếu môi trường chặn ghi rõ, không mark pass/tạo chuỗi chat thất bại.
Không publish/gửi tin nhắn/automation.

Đã tạo đúng một chat kế tiếp `F19 — Diagnostics và resource usage`:
`01a1d330-d7b9-7a31-b02b-884ab9df52a0`, project Flutter Network, local, yêu cầu
bắt đầu ngay; chỉ ghi Rust và đọc handoff trên. Không tạo chat khác/publish.

## F19 — Diagnostics và resource usage (2026-11-13)

Implemented trên macOS; chỉ sửa Rust, không spawn subagent, không init Git/CodeGraph
hay publish. Flutter được khảo sát bằng CodeGraph context → đúng một explore,
đọc test diagnostics/resource và các điểm gọi viewer_server/main cụ thể.

- `src/diagnostics.rs`: JSONL có time UTC/PID/event và metadata bool/count;
  enum event không nhận chuỗi tự do, URL, lỗi transport, token, header/body/console.
  Startup/shutdown, capture/discovery start/stop, connection attempt/failure/state,
  pause/clear nối `DesktopApp` production. Connection state chỉ ghi khi thay đổi.
  Queue128KiB, flush sau250ms theo owner tick; `viewer.log/.1/.2`, mặc định
  mỗi file≤512KiB. Path/disk lỗi bỏ batch, không propagate vào capture, batch
  mới phục hồi khi path dùng được. Shutdown flush và ngăn enqueue tiếp.
  Không detached timer/worker; bounded synchronous I/O trên owner thread,
  nên không tuyên bố disk latency độc lập với rendering/capture cadence.
- `src/resources.rs`: native macOS TASK_VM_INFO physical footprint (như Flutter),
  Windows GetProcessMemoryInfo working set và Linux /proc/self/status VmRSS.
  Không gọi ps trong production. Sample ngay khi khởi tạo rồi2s theo clock owner;
  lỗi xóa sample, lần sau phục hồi, stop ngăn sample. Owner drop không để lại
  callback/thread/timer. MB thập phân, RAM — khi unavailable. Windows/Linux code
  chưa được build/chạy native ở F19, còn F21.
- `DesktopApp::tick/dispatch` là đường production được update sử dụng; public
  diagnostics/resources, with_diagnostics để chọn path cụ thể. Mặc định log vào
  desktop log directory; FNV_DIAGNOSTICS_DIR override. Native regressions dùng
  override trong Rust artifacts để không ghi ra HOME. Footer RAM ở egui bottom
  panel, vẫn visible cửa sổ hẹp; cập nhật không invalidate capture detail caches.
  F18 settings modal max_height, F15/F17 clipping và Screenshot events giữ nguyên.

Executable tests trước implementation: `red.log` thiếu diagnostics/resources và
host API; `red-ui.log` thiếu RAM UI/probe. `tests/f19_diagnostics.rs`:4 tests
rotation/queue/shutdown, blocked path/recovery/production dispatch với secret URI,
clock sampling/failure/recovery/stop/drop, native process sample>0.
`tests/f19_ui.rs`:1 renderer/input test ở EN rộng và VI hẹp, RAM unavailable→sample
→failure→recovery, text shapes/geometry, selected detail/body/raw cache không
rebuild, Escape dismiss inspector modal rồi click Settings. Backend tests không
được dùng để suy native UI pass.

Verification source cuối (CARGO_HOME=/private/tmp/fnv-cargo,
RUSTUP_HOME=/private/tmp/fnv-rustup, PATH=/private/tmp/fnv-cargo/bin:$PATH):
- `cargo test --locked --offline --quiet`: **264 pass (208 core/HTTP +56 UI)**,
  0 fail/ignored, `docs/artifacts/f19/regression-final.log`.
- `cargo fmt --check`, `cargo clippy --all-targets --locked --offline -- -D warnings`:
  pass, `fmt-final.log`, `clippy-final.log`. Không thêm dependency.
- `tools/f19_native_smoke.py` build source hiện tại và mở fresh WebSocket VM;
  production host capture3 requests trong lúc path log bị chặn, ≥2 RAM samples,
  unblock path→log phục hồi, native Pause/Resume, Settings input, VI/dark480×360,
  ảnh footer RAM và shutdown flush. `native-final.log`, `native-final/native-result.json`.
  Native memory203359312 bytes, independent ps RSS199872KiB (hai metric khác nhau),
  `native-final/memory.json`; không dùng fake samples/servers thay production host.
  Fixture VM chỉ là test peer như các feature trước.
- Native F13/F14/F15/F16/F17/F18 sequential regression đều exit0 trên cùng
  production source: `f13-regression.log` … `f18-regression.log` và thư mục tương ứng.
  Bao gồm OS clipboard, JSON/body, settings reload, MCP HTTP/port retry/release.
  Không ghi đè artifacts feature cũ.

Failures giữ để review: `clippy-first.log` unused-mut trong UI harness đã sửa;
`native-first.log` đạt assertions rồi panic do harness gọi update thêm frame sau
khi đã take/drop app. Đã guard None và native-final exit0; result.json ở native-first
không đại diện process pass. `feature-final.log` tái hiện UI test click Settings
bị inspector modal hẹp chặn; sửa test dismiss bằng Escape, không bỏ assertion;
`feature-final-2.log` pass và full regression-final pass sau bổ sung secret URI
với log path writable. Production behavior không bị thay để né test.

README/matrix/catalog mapping cập nhật F19. Updater F20 và build/release F21 chưa
làm. Giới hạn cũ còn nguyên: Windows/thumbnail backend/native resource validation,
live SDK/WSS/combined discovery→multiapp/accessibility/font mọi OS F21; ImageIO
bounded output không guarantee total codec working memory≤64MiB; cold JSON
parse/index synchronous; async Logging resolution partial; DNS/TCP trước socket
handle chưa absolute cancellable/deadline; MCP queries chạy owner/UI thread.

## Handoff F19 → F20 — Updater

BẮT ĐẦU NGAY. Mỗi chat đúng một tính năng, executable tests TRƯỚC implementation.
Chat kế tiếp CHỈ F20 — Updater; không spawn subagent. CHỈ GHI
`/Users/quocan/Github/flutter_network_viewer (rust)`; không sửa Flutter
`/Users/quocan/Github/flutter_network_viewer` dù project dùng Flutter Network.
Đọc AGENTS.md, docs/feature-matrix.md và handoff cuối docs/progress.md ở Rust.
Khảo sát Flutter app_updater/settings_updates và update-feed/release-encoding tests
qua CodeGraph projectPath Flutter: context rồi đúng một explore, đọc file cụ thể
nếu thiếu. Rust chưa Git/CodeGraph, không tự init/publish.

F20: feed đúng OS/CPU, version/progress, serialized checks, retry lỗi/no-update,
native callbacks, confirmation một lần, unsupported không gọi native, dispose
listener, signature/checksum; không cài thật trong test. UI production phải nối
renderer/input + native test, không chỉ backend/canned sample. Không làm release
F21 trong chat F20. Không publish/gửi tin nhắn/automation.

F19 mới: src/diagnostics.rs typed metadata-only Diagnostics; file budget512KiB×3,
queue128KiB, owner flush250ms+drop; paths lỗi swallowed và discard batch, recover
batch mới. `DesktopApp` public diagnostics/resources; `with_diagnostics(settings,
PathBuf)`, tick(now), dispatch(action,now). tick drives capture, real RAM và state
logs; update calls tick rồi UI. Drop stop RAM + app_stopped + log shutdown flush.
FNV_DIAGNOSTICS_DIR override dùng trong regression để chỉ ghi Rust. Đừng log URL,
VM token, transport error text, headers/bodies/console hoặc arbitrary strings.
ResourceUsage sampling owner-clock ngay+2s, failure None→RAM —, next sample retry;
macOS native task_info physical footprint, Windows/Linux chưa verified (F21).
Footer RAM bottom panel; không invalidate selected capture caches. Disk I/O owner
synchronous bounded batch; cadence phụ thuộc event loop, không background workers.

Giữ F18 public DesktopApp.mcp configure synchronous owner serialized, poll live
CaptureStore+online mỗi frame; off38943, loopback16 connections/deadline5s,
5 tools chunks UTF16, stop/drop closes active sockets/port. SupportUi.mcp_status
trước render, Action::RetryMcp; Settings modal max_height độc lập frame cũ để không
oscillate geometry. F17 Notifications owned deadlines3s/no detached timers;
copy OS clipboard, VI/font; Screenshot events native hooks không được bỏ.
F16 caches và F15/F17 clipping giữ nguyên. Không native child hosts/IPC.

264 tests pass (208 core/HTTP+56 UI),0 fail/ignored; fmt/clippy all-targets pass.
Native F19 + F13–F18 regressions pass. Artifacts docs/artifacts/f19, không ghi đè;
F20 tạo docs/artifacts/f20. Runners python3 tools/f13_native_smoke.py …
f19_native_smoke.py [output_dir]; F17–F19 tự mở fresh f16_fake_vm. F19 native runner
cần thư mục output mới (blocker path test). Narrow inspector modal phải dismiss
bằng Escape trước click nav; toast có thể che buttons, dismiss trước, giữ assertions.

BẮT BUỘC CARGO_HOME=/private/tmp/fnv-cargo RUSTUP_HOME=/private/tmp/fnv-rustup
PATH=/private/tmp/fnv-cargo/bin:$PATH; ưu tiên --locked --offline. Nếu cache/toolchain
bị dọn phải restore và compile SOURCE mới, không báo pass từ binary cũ.
Loopback/native/clipboard/ps RSS cần exec_command require_escalated.
Coordinator_socket bootstrap flake cũ có thể xuất hiện; giữ failure và rerun có
bằng chứng, không tuyên bố fix nếu chưa sửa nguyên nhân.

Cập nhật catalog/matrix/README/progress/handoff. Chỉ khi F20 regression/fmt/clippy/
integration/UI/native phù hợp đầy đủ pass: list_projects lấy ID thật, mở đúng một
chat F21 — Build/release và acceptance toàn app, yêu cầu bắt đầu ngay. Dùng Flutter
Network nếu Rust chưa đăng ký. Nếu môi trường chặn ghi rõ, không mark pass/tạo
chuỗi chat thất bại. Không publish, cài update thật, gửi tin nhắn hoặc automation.

Đã tạo đúng một chat kế tiếp `F20 — Updater`:
`01a1d33b-51aa-71d3-8c2b-25e6e40fceb8`, project Flutter Network, local, yêu cầu
bắt đầu ngay và chỉ ghi Rust; đọc handoff F19→F20 phía trên. Không tạo chat khác.


## F20 — Updater (2026-11-13)

Implemented macOS; chỉ ghi Rust, không spawn subagent/init Git/CodeGraph/publish.
Flutter khảo sát bằng context → đúng một explore, sau đó đọc app_updater/settings
updates và tool feed/encoding tests cụ thể còn thiếu. Không triển khai release F21.

- `src/updater.rs`: feeds đúng macOS ARM64/x86_64, Windows x64; platform khác
  không configure/poll/close native. Owner-serialized check, busy spinner,
  fetch timeout60s với listener teardown/reconfigure khi retry. Retry sau lỗi,
  no-update, finish/cancel download; macOS giữ Current sau Sparkle abort, Windows
  vẫn báo error. Windows background Available phát foreground confirmation đúng
  một lần. Native callback Downloading/Downloaded/Installing và cleanup owned;
  không detached Rust workers/timers hoặc callback giữ Rust pointer.
- `native/updater_macos.m`: compile Objective-C ARC bằng build.rs; dynamic-load
  Sparkle2.9.6 từ bundle Frameworks, main-thread only, weak delegate. Configure
  yêu cầu Ed25519 public key32bytes + ba plist flags tắt auto-check/download/install
  rõ ràng; thiếu bundle/framework/key thì fail + retry. Sparkle chuẩn quản lý
  native confirmation/progress/verification/install. Native callbacks chỉ enqueue
  tối đa32 typed events; drop detach owner, bỏ queue, không giữ Rust host.
  Sparkle không cung cấp public cancel-all API: drop listener ngăn callback vào
  host nhưng không tuyên bố hủy tuyệt đối native network đang chạy.
- `src/updater_windows.rs`: dynamic-load DLL bên cạnh exe với restricted search,
  WinSparkle0.8.1 x64 API, public DSA key, disable automatic checks, bounded enum
  callback queue, singleton owner, cleanup joins native work trước unload. Verify
  set_dsa_pub_pem return value; shutdown callback chỉ yêu cầu host close sau native
  installer request. Windows chưa build/chạy, để F21, không suy native parity từ
  controller tests trên macOS. WinSparkle native UI quản lý download progress.
- `DesktopApp` owns updater, production startup background-check một lần trong
  first update, tick callbacks rồi truyền View sang SupportUi; Action::CheckUpdates
  chạy foreground. Windows Installing gửi Close viewport. Settings EN/VI version
  từ Cargo, trạng thái/Current green tooltip, busy spinner/disable check, lỗi/retry,
  available action, unsupported disabled, narrow scroll. F13–F19 APIs/cache/clipping,
  notification/native Screenshot hooks giữ nguyên.
- Vendored Sparkle/WinSparkle runtime/license/provenance trong vendor. SHA256SUMS
  được build.rs kiểm tra với Python trước build. Sparkle sign_update verifier
  thực chạy valid/unsigned/wrong signature/wrong key/tampered inert archive.
  SHA256 ở đây pin updater runtime, không phải thay chữ ký archive update.
  Native Sparkle/WinSparkle xác minh archive trước cài; không tự viết crypto.

Executable tests viết và chạy trước implementation: red.log thiếu updater API;
red-ui.log thiếu View/action/Settings UI; red-integrity.log thiếu checksum module;
red-cancel.log tái hiện busy không clear sau finish download, rồi sửa controller.
- tests/f20_updater.rs:5 tests feed/unsupported, serialized/retry/timeout/drop,
  Current→abort platform policy, Windows confirmation once/native progress,
  native exceptions/macOS confirmation, finish/cancel clears busy.
- tests/f20_ui.rs:1 production renderer/input test EN/VI, version text, shapes lỗi
  localized, progress/busy, retry/current/available, unsupported, narrow geometry.
- tools/test_f20_integrity.py:2 tests vendor SHA256 tamper + real Sparkle verifier
  valid/unsigned/wrong signature/wrong key/changed archive. Key test cố định, không
  phải release key. Release feed generation và output encoding tool tests chưa
  port trong F20; được ghi partial mapping, còn F21.

Verification source cuối với CARGO_HOME=/private/tmp/fnv-cargo,
RUSTUP_HOME=/private/tmp/fnv-rustup, PATH=/private/tmp/fnv-cargo/bin:$PATH:
- cargo test --locked --offline --quiet: **270 pass (213 core/HTTP+57 UI)**,
  0 fail/ignored, docs/artifacts/f20/regression-verified.log.
- cargo fmt --check, cargo clippy --all-targets --locked --offline -- -D warnings:
  pass, fmt-verified.log/clippy-verified.log. Không thêm Cargo dependencies.
- 2 integrity tests pass, integrity-verified.log. Native runner compile current
  source rồi dựng isolated test app bundle, Sparkle thật với loopback appcast,
  malformed→retry→Current callbacks; foreground Settings click/version, VI-dark
  480×360, screenshot spinner/Current; process mới cho Available/native confirmation
  và listener drop. native-verified.log, native-verified/native-result.json,
  available-result.json và http-requests.json (chỉ4 feed requests). Không download
  hay cài installer. Feed UTF8 title/description được Sparkle parse; archive trỏ
  example.invalid và không có bước chấp nhận cài đặt trong test.
- Native sequential regressions F13–F19 đều exit0 trên source cuối:
  f13-verified.log … f19-verified.log và directories cùng tên. Fresh VM, native
  clipboard/export, Settings persistence, MCP/port lifecycle, RAM/log path recovery.
- Ảnh native-verified/settings-current-en.png, settings-checking.png,
  settings-current-vi-narrow.png được giữ; ảnh VI narrow đã inspect trực tiếp.

Failures giữ đầy đủ: first.log clang module cache cố ghi ngoài Rust; bỏ -fmodules,
không cần cache đó. second.log SPUUpdater không có setter delegate; cleanup dùng
weak owner đúng public API. fmt-final.log chỉ test formatting, fmt-final-2 và
fmt-verified pass sau cargo fmt. native-final.log timeout Available do check ngay
sau foreground Current dialog còn giữ session; tách Current và Available thành
hai fresh processes, native-final-2 rồi native-verified pass. Không đổi production
để né native session hoặc bỏ assertions. red-cancel giữ test failure trước fix.

Catalog/matrix/README cập nhật. F20 implemented macOS, Windows native còn F21 như
các feature trước. Cargo package vẫn0.1.0; feed URLs đang cùng repository Flutter,
artifact hiện tại là Flutter. **F21 phải quyết định/cấu hình Rust version/build và
feed/archive tương thích trước release**, không ship Rust trỏ nhầm Flutter binary.
Không có release/private signing key, không publish/cài update thật.

## Handoff F20 → F21 — Build/release và acceptance toàn app

BẮT ĐẦU NGAY. Mỗi chat đúng một tính năng, executable tests TRƯỚC implementation.
Chat kế tiếp CHỈ F21 — Build/release và acceptance toàn app. Không spawn subagent.
CHỈ GHI `/Users/quocan/Github/flutter_network_viewer (rust)`; không sửa Flutter
`/Users/quocan/Github/flutter_network_viewer` dù dùng project Flutter Network.
Đọc AGENTS.md, docs/feature-matrix.md và handoff cuối docs/progress.md ở Rust.
Rust chưa Git/CodeGraph; không tự init/publish. Không tạo chuỗi chat tiếp theo
sau F21 khi không còn feature pending. Không cài update thật/gửi tin nhắn/automation.

F20 mới: src/updater.rs controller Backend/Event/View, NativeBackend;
DesktopApp public updater, startup background check một lần trong update, tick
poll và copy View, Action::CheckUpdates foreground. macOS native bridge ObjC ARC
build.rs, dynamic Sparkle2.9.6 từ .app/Contents/Frameworks; key SUPublicEDKey
`TNPdGQZDTuvlcdgdReTFjir/+sfSLq5+o92dT4IolFY=` + plist flags
SUEnableAutomaticChecks/SUAutomaticallyUpdate/SUAllowsAutomaticUpdates=false
rõ ràng, SUEnableInstallerLauncherService=true. Standalone cargo executable
không bundle→Failed/retry, không giả successful updater. Native controller chỉ
listener cleanup, không guarantee cancel network do Sparkle public API giới hạn.
Windows updater_windows.rs loads WinSparkle0.8.1 x64 DLL cùng exe, public DSA key
native/windows_dsa_pub.pem, restricted DLL search, callback queue and cleanup;
chưa compile/native Windows. Runtime/license/checksum/provenance vendor/,
build.rs Python verify_updater_vendor.py trước compile. F21 packaging phải copy
framework nguyên layout symlinks/helpers, WinSparkle x64 DLL, và sign đúng nested
code nếu có identity; thiếu identity/key phải báo pending, không tự publish.

**Feed URLs hiện tại vẫn shared Flutter releases/latest/download/appcast-*.xml**,
Cargo0.1.0. F21 cần làm Rust metadata/version/build, archives/setup/feed/checksum và
đảm bảo Rust app không cài nhầm Flutter artifacts. Đừng publish/ghi production feed.
Khảo sát Flutter CI/release tools/encoding tests bằng CodeGraph theo instructions;
UTF8 feed/release notes, pinned signatures/build numbers/installer contract phải
có executable tests. Không coi SHA256 vendor pin là chữ ký update archive.
Native tests chỉ inert bytes/appcast, không chấp nhận installer.

270 Rust tests pass (213 core/HTTP+57 UI),2 integrity Python tests pass,0 ignored;
fmt/clippy all-targets pass. Artifacts docs/artifacts/f20/*-verified.log + dirs;
không ghi đè artifacts cũ. Runners f13..f20_native_smoke.py [output_dir], F19 cần
fresh output. F20 creates bundle and loopback feed with real Sparkle; Current
foreground dialog giữ native session, Available cần fresh process (runner đã làm).
Giữ Screenshot events hook; narrow inspector Escape trước nav, dismiss toast
trước buttons, không bỏ assertions. F17 notifications owner deadlines, F18 MCP
owner-loop serialized, F19 bounded metadata diagnostics/RAM owner clock vẫn giữ.

BẮT BUỘC CARGO_HOME=/private/tmp/fnv-cargo RUSTUP_HOME=/private/tmp/fnv-rustup
PATH=/private/tmp/fnv-cargo/bin:$PATH; ưu tiên --locked --offline. Cache bị dọn thì
restore/compile source mới, không chạy binary cũ báo pass. Loopback/native/clipboard/
ps RSS cần exec_command require_escalated. Coordinator_socket bootstrap flake cũ
có thể xuất hiện; giữ failure/rerun evidence, không khẳng định fix nguyên nhân.

Acceptance còn: CI fmt/clippy/unit/integration/UI; macOS ARM/Intel + Windows x64
build/native, installer metadata/icons/font/license/signatures; fake VM full flow
capture/filter/inspect/pause/clear/MCP/export/restart settings/stress; live SDK/WSS,
combined discovery→multiapp, accessibility/font mọi OS. Windows native RAM working
set/Linux VmRSS/thumbnail chưa verified; ImageIO bounded output4096²×4 không
absolute total codec memory64MiB; JSON cold parse/index synchronous; async Logging
object-resolution partial F11; DNS/TCP trước socket handle chưa absolute cancel/
deadline; MCP owner/UI thread không benchmark concurrent latency. Báo trung thực
blocker nền tảng/SDK/signing; không đánh dấu acceptance toàn app nếu chưa chạy.

Đã tạo đúng một chat kế tiếp `F21 — Build/release và acceptance toàn app`:
`01a1d34e-bb79-7d11-9bca-77e8b164c9f3`, project Flutter Network, local, yêu cầu
bắt đầu ngay, chỉ ghi Rust và đọc handoff F20→F21 phía trên. Không tạo chat khác.

## F21 — Build/release và acceptance toàn app (2026-11-13)

**Partial; chưa đạt acceptance toàn app/chưa release-ready.** Chỉ ghi Rust,
không subagent, không init Git/CodeGraph, không publish/feed production/install,
không chat kế tiếp. Flutter source snapshot: 0 differences. Khảo sát Flutter
CodeGraph context → đúng một explore; đọc các release tools/CI cụ thể bị thiếu.

Executable tests trước implementation:
- `red-release.log`: thiếu module release. `red-feed.log`: production còn dùng
  Flutter latest feed, test yêu cầu Rust feed. Sau đó chọn kênh cố định riêng
  `rust-updates/rust-appcast-*` để không phụ thuộc latest release Flutter.
- `red-resource.log`: thiếu resource Windows. `red-manifest.log`: manifest rỗng
  bị chấp nhận trước fix. Các lỗi/rerun đều giữ trong `docs/artifacts/f21`.
- 8 Python release tests cuối pass: metadata/version/build monotonic contract,
  UTF-8 notes/workflow outputs/XML dưới simulated cp1252 locale, archive/platform/
  signature rejection, staging assets/full Sparkle symlinks/helpers/WinSparkle DLL,
  checksum tamper/empty/duplicate/path rejection, font glyph tiếng Việt, native
  icon containers, installer/resource/CI contracts. `release-final.log`.

Implementation:
- `tools/release.py`, `packaging/release.json`: Rust0.1.0/build1, tag rust-v0.1.0,
  ID dev.quocan.flutterNetworkViewer.rust; archives flutter-network-viewer-rust-*,
  feed enclosures chỉ đúng Rust filename/tag. Runtime feed fixed Rust channel,
  không fallback Flutter. Channel chưa publish; missing feed báo lỗi/retry thật.
  Main window và WinSparkle app name thêm Rust; settings directory đã riêng.
- Local package commands build --release --locked --offline, lipo verify,
  full framework copy symlinks/helpers, icons/fonts/OFL/GPL/updater licenses,
  archive/checksum/UTF8 output/status release_ready=false. `native/windows.rc`
  version/icon và build.rs native Windows rc.exe hook; metadata validator giữ
  resource version khớp Cargo/build. `packaging/windows.iss` per-user x64, Rust
  app ID/directory, close running app, updater DLL; chưa run Inno trên Windows.
- Optional inside-out codesign preserves existing helper entitlements; chưa có
  identity để exercise. macOS signer CryptoKit kiểm tra derived public key,
  stdin private seed; Windows DSA signer kiểm tra key + verify output theo vendor
  WinSparkle0.8.x digest contract. Thiếu key fail-closed, không feed unsigned.
  Không có matching private release keys nên **không claim release signing pass**.
- CI checks.yml matrix macOS ARM/Intel + Windows MSVC: fmt/clippy/core/integration/
  renderer/input tests, release contracts, native macOS regression subset, release
  builds/packages, Windows Inno, manifests và upload artifact. Contents:read,
  không publish job. **Workflow mới cấu hình, chưa chạy GitHub CI**.
- `test_f20_integrity.py` nhận FNV_INTEGRITY_ARTIFACTS để run F21 không ghi artifact
  F20; test behavior và production keys không đổi. Windows cfg-only needless return
  phát hiện bởi cross clippy đã sửa. Không đổi F17 deadlines/F18 owner serialized
  HTTP/F19 metadata-only diagnostics/resource cadence hoặc API production khác.

Source cuối verification với toolchain tạm bắt buộc:
- `regression-final.log`: **270 Rust pass =213 core/HTTP +57 UI**,0 fail/ignored.
  `fmt-final.log`, `clippy-final.log` all-targets pass.
- `windows-check-second.log` all-targets cross check pass;
  `windows-clippy-final.log` all-targets cross clippy pass. First offline check
  thiếu clipboard-win; cargo fetch --locked --target tải cache, rồi pass. Actual
  Windows linked release build fail vì **không có MSVC link.exe**:
  `windows-build-first.log`. Cross compile-check không phải native acceptance.
- `release-final.log`:8 pass; `integrity-verified.log`:2 real Sparkle verifier/
  vendor pin tests pass, artifacts cô lập F21. SHA256 không thay chữ ký update.
- `package-final.log`, `intel-final.log`: actual release builds ARM64 và Intel pass.
  `package-layout-final.json`: actual zip SHA256, lipo architecture, plist/key/flags,
  full Sparkle symlink và helpers, icons/fonts/license verified.
  Zips trong package-final/ và intel-final/; preview chưa signed/notarized.
  `bundle-startup-final/result.json`: actual packaged main cả hai chạy và ghi
  settings/diagnostics trong isolated HOME rồi đóng. Intel chạy Rosetta trên ARM64,
  không phải native Intel hardware acceptance.
- `native-final.log` + `native-final/`: production DesktopApp/fresh fake VM, pointer/
  keyboard URL+method filter, pending→201 inspector, OS clipboard, Console,
  Pause/Resume, export2 records, MCP real HTTP summary2, Clear/cache/selection,
  port release/screenshot. Không seed store trực tiếp để giả capture.
- `multiapp-verified.log` + result: injected DDS process listing, real HTTP DDS
  resolution/getVM/getIsolate probes, choice giữ chưa connect, first app live,
  scan second app giữ history,2 sessions/4 requests/shared Console, global Pause/
  Clear qua nhiều polls. Không claim live process enumeration hoặc multiapp GUI.
- `live-first.log`, `live-first/live-export.json`: **real Dart SDK** từ Flutter SDK
  đã có trên máy, HTTP201 request quantity và response Cà phê/ảnh Unicode, Stdout
  và developer.log. Fixture chỉ ở tests/fixtures/f21, không sửa Flutter SDK/source.
  Đây là Dart VM capture acceptance; live Flutter discovery/GUI còn pending.
- `wss-third.log` + TLS server evidence: native TLS rejects self-signed WSS,
  thực nhận TLS handshake/failed-trust closure. Không thêm cert/keychain exceptions.
  Trusted WSS full capture chưa chạy. Keys trong WSS artifacts là inert test keys.
- `native-regression.log` và `native-regression/f13..f20`: **tất cả native F13–F20
  pass** trên source đã compile hiện tại (trước final Windows-only lint/tên main
  window; sau đó compile lại full regression/native F21/package source cuối).
  F17 persistence/reload/clipboard, F18 HTTP/MCP/ports, F19 RAM/path recovery,
  F20 real Sparkle error→retry→Current + Available fresh process, không installer.
- `stress-first.json`:4 renderer select/Clear cycles,20.002 JSON rows/render37,
  warm p95 3.86–3.90ms; cold112–119ms; RSS9.7→60.9MiB. Cold parse đồng bộ vẫn còn.
  F16 native stress trong regression pass; không suy total codec memory64MiB.
- Vietnamese glyph cmap và native icons test pass. Đã inspect trực tiếp ảnh
  `native-regression/f20/settings-current-vi-narrow.png`, VI/dark/480×360 rõ glyph.
  Không suy ra screen-reader/a11y pass từ glyph/screenshot/input tests.

Failure evidence: release-first thiếu Python tomllib (system3.9; dùng Python3.14
Homebrew, CI3.12); iconutil rejected input iconset, dựng standard icns PNG elements
và giữ assets nguồn; native-first shared search còn lọc Console, runner reset trước
assert1 log (không bỏ assertion); multiapp-first/second incomplete HTTP fixture
headers/close, bounded full-header peek + consume HTTP before close, final pass
không RPC trace timing; WSS-first system LibreSSL bind không hỗ trợ host:port,
WSS-second native trust closes TLS EOF thay alert; runner dùng OpenSSL3 và kiểm
chứng TLS failure tại server, third pass. Không claim các lỗi fixture là fix
production. Không thấy coordinator bootstrap flake trong final run, không claim
đã sửa nguyên nhân lịch sử. Các failed logs không bị ghi đè.

Remaining gates / blockers, chi tiết `docs/release.md`:
1. Windows linked build/PE resource/Inno/native GUI/updater/clipboard/persistence/
   working set/thumbnail backend: cần Windows + MSVC, không có trên host này.
2. Có 0 valid signing identities; không private release key. Developer ID codesign
   branch, notarize/staple, Authenticode, matching key archive signature và installer
   compatibility chưa verified; dependency license/source review còn pending.
3. Trusted WSS, live Flutter discovery/multiapp GUI, screen-reader/accessibility/
   OS font acceptance mọi OS; eframe hiện chưa enable optional AccessKit bridge.
4. Intel hardware native acceptance; Rosetta startup không thay được điều này.

README/matrix/catalog/release/progress cập nhật, không mark complete. F21 là feature
cuối, **không tạo F22/chat khác**. Những giới hạn F11 Logging resolution, DNS/TCP
pre-handle cancel/deadline, synchronous JSON cold parse, owner MCP latency chưa
benchmark, ImageIO total codec memory và Linux VmRSS chưa verified vẫn giữ nguyên.


F21 supplemental verification: `signer-verified.log` **1 test pass** dùng chính
`tools/sign_macos_update.swift` với ephemeral test seed/inert UTF8 archive: real
Sparkle verifier nhận chữ ký, wrong trust anchor bị helper từ chối, bytes sửa bị
verifier từ chối. Không có matching production release key và không claim code
sign/notarization. CI macOS đã thêm test này; tổng Python F21 là8 release +2
integrity +1 actual helper. `icon-decode.log`: macOS sips giải mã icon icns thật
thành PNG thành công. Docs tests rerun8 pass, `release-docs-final.log`.


## UI parity — shell và bảng Network (2026-11-13)

Đối chiếu trực tiếp code Flutter, không dùng GUI automation để nghiệm thu đợt này.
Nguồn chuẩn commit `2882427119f28e7fb96cb7700aab7762a65c70a9`;
`lib/ui/viewer_shell.dart`, `viewer_theme.dart`, `network_request_table.dart`,
`components.dart`. Không sửa nguồn Flutter. CodeGraph Rust đã init theo người dùng.

Đã sửa `src/desktop_network.rs`: Method → URL → Status → Time; header 30px
cố định khi cuộn dọc; hàng 52px; URL/path + host/app/clock ở dòng phụ; màu method,
status từ ViewerPalette; chọn cả hàng xanh và marker 4px. URL lấp phần còn lại,
minimum 140px, divider 6px; kéo ba divider theo quy tắc Flutter (divider cuối
phân phối giữa Status và Time), lưu widths qua Settings. Renderer vẫn chỉ dựng
viewport. Cập nhật test divider cũ vì giả định ba cột tăng độc lập không đúng Flutter.

Shell: header co theo nhãn dịch; banner lỗi/pause đặt trước toolbar; empty/no-results
có icon/title/action ở giữa, scroll khi thấp; Reset filters cho cả Network/Console.
Footer 25px, RAM giữ trong cửa sổ hẹp. Inspector chỉ lưu width khi nhả tại divider;
click toolbar/dialog không ghi đè width. Help Close chuyển sau Discussion/Copy để
không bị toast clipboard góc trái dưới che. Tên cửa sổ dùng Flutter Network Viewer.

TDD: empty-filter, banner, thứ tự cột/hàng 52px đều fail trước implementation.
Validation source cuối: **276 pass, 0 failed, 0 ignored**; fmt và all-targets clippy
`-D warnings` pass. Logs: `docs/artifacts/ui-parity/{regression,clippy,fmt}.log`.
Full tests cần quyền loopback; lượt sandbox thất bại PermissionDenied được chạy
lại ngoài sandbox. Không bỏ test/assert để qua lỗi; bổ sung assert Help thực sự đóng.

Chưa nghiệm thu toàn app: Console row layout, inspector, JSON/body, settings/help
và dialog còn cần đối chiếu từng thành phần với Flutter. Chưa build lại package
release; package cũ trong docs/artifacts/f21 không chứa thay đổi này. Ưu tiên macOS
trước theo người dùng; Windows/native visual acceptance vẫn pending. Không tuyên
bố pixel parity từ renderer tests. Không tạo chat tiếp theo trong đợt chỉnh UI này.


## Sửa nền đen do panel trong suốt (2026-11-13)

Ảnh người dùng cho thấy JSON theme sáng bị đen. Root cause: Frame::NONE của
CentralPanel và hai inspector SidePanel không có fill, lộ màu clear đen native.
Sửa ba frame tô visual.panel_fill. Không đổi geometry/margin.
Test `every_page_paints_an_opaque_background_in_both_themes` fail trước fix,
pass sau fix: kiểm tra Shape::Rect fill/clip thực tại vùng trống của Network,
Console và hai nửa JSON, cả 700/1360px và light/dark. Không chỉ kiểm style token.
277 tests pass, build/fmt/clippy pass; log `artifacts/ui-parity/background-regression.log`.
Đã build lại target/debug/flutter-network-viewer. Không kiểm GUI native trong
lượt này; regression kiểm paint commands của renderer sản phẩm.


## Sửa khe đen giữa panel (2026-11-13)

Bổ sung nền kín toàn viewport trong DesktopUi::show và override DesktopApp::clear_color
bằng visual.panel_fill. Fill từng panel trước đó chưa phủ spacing/gutter giữa panel.
Test viewport_background_covers_panel_gaps_and_outer_edges fail trước/pass sau fix,
kiểm nền toàn cửa sổ ở light/dark; test nền từng tab giữ pass. 278 tests pass khi
rerun; lần đầu coordinator_socket timeout VM did not progress, giữ log failed,
không sửa timeout/assert. Build/fmt/clippy pass.
Đã tạo target/debug/FNV Rust.app và kiểm SHA256 executable khớp target/debug binary.
Computer Use từ chối quyền FNV Rust; không có ảnh native xác minh lượt này.
Người dùng yêu cầu sửa rồi giao đường dẫn app, không tiếp tục GUI.


## Đồng bộ typography, spacing và controls (2026-11-13)

Đã áp dụng font thật Medium/SemiBold/Bold/Italic từ Flutter/OFL, font icon riêng;
header căn trái/SemiBold16; badges Bold11; inspector margins/pill tabs; Console
badge/clock/message/padding; JSON outer/editor margins, border/radius, toolbar,
divider và mono line-height1.65; Settings sections padding24/title16/labels12,
fields outline padding12; Help icons/typography/gaps; tooltip400ms/dialog radius28.
Danh sách mapping và giới hạn cụ thể ở ui-style-parity.md.

Font installation cần pass egui kế tiếp: request_discard trước khi render named
families lần đầu, tránh panic FontFamily is not bound. Test header và JSON fail
trước thay đổi; tests actual glyph/render/layout pass. Test Settings helper cuộn
trước click vì padding thật làm một số control ở ngoài viewport; giữ các assert.
Validation: 281 tests pass/0failed/0ignored, fmt/clippy all-targets -Dwarnings pass;
logs artifacts/ui-parity/styles-{regression,clippy}.log. Build lại debug .app,
kiểm SHA256 executable bằng binary mới; không kiểm GUI/native screenshot lượt này.


## Sửa Settings theo route Flutter (2026-11-13)

Bỏ modal Settings, tạo AppBar56/Back và nội dung riêng có cuộn; khung764/lề28,
đúng thứ tự Interface/Connection/MCP/App updates, Theme trước Language. Đồng bộ
card header icons, switch, fields, buttons; Connection bỏ nội dung thừa, nhãn wrap
ở360px; MCP responsive port/endpoint và menu chọn client cho nút copy setup.
Chi tiết mapping và giới hạn ở ui-style-parity.md. Giữ nền opaque và first-frame fonts.

TDD test full-page fail trước fix (shell vẫn hiện), pass sau fix. Regression **282
pass/0failed/0ignored**, fmt/clippy all-targets -Dwarnings/build pass. Tests thao tác
thật qua renderer input cho Settings/Connection/MCP/updater, narrow/locale/theme,
Back/Escape/menu. Logs artifacts/ui-parity/settings-*.log; debug .app cập nhật và
SHA256 executable khớp binary. Không chạy native screenshot trong lượt sửa này;
không tuyên bố toàn app pixel parity. Không tạo chat kế tiếp trong đợt chỉnh UI.


## Sửa icon Back Settings (2026-11-13)

Thay glyph font private-use của nút Back bằng ba stroke vector mũi tên trái,
icon24/stroke2 và hit target40; giữ click/keyboard, thêm tên accessibility và tooltip
Back/Quay lại. Tránh hiển thị symbol khác khi mapping font icon không tương ứng.
TDD kiểm paint segments thật: fail trước/pass sau; test click quay lại giữ pass.
Regression283 pass, clippy/build/fmt pass. Metadata accessibility bổ sung sau regression
đã được chạy lại test arrow và compile/clippy. Logs artifacts/ui-parity/back-arrow-*.log.
Đã đồng bộ .app debug, SHA256 executable khớp binary mới. Chưa kiểm screenshot native.

## Sửa Response body/Tree và khung chi tiết request

Đối chiếu Flutter DataSection, JsonFindBar và macOS NativeJsonViewer. Toolbar
chung một hàng cao 30, pill JSON/Tree rộng 128 và nút copy icon. View dùng nền
surface, chữ mono 12, hàng 22, lề dọc 6. JSON tách số dòng và nút gập; Tree không
số dòng, có connector, count, indent và màu theo kiểu. Copy/find/fold giữ dữ liệu.
Inspector dùng vùng bố cục riêng bị clip vào kích thước divider; URL dành chỗ
cho nút cURL. Chọn request hay đổi tab không tự đổi chiều rộng, kéo vẫn lưu.
TDD tái hiện chiều rộng đổi 684→368 trước sửa; test renderer xác nhận cả
chiều rộng lưu và chiều rộng sau kéo, số dòng, toolbar và style Tree.
285 tests pass; fmt/clippy/build pass. Logs artifacts/ui-parity/body-*.log.
Đã cập nhật debug .app và xác minh SHA256. Chưa đối chiếu screenshot native.


## Ẩn scrollbar và dùng icon Copy — Rust (2026-11-13)

Áp dụng AlwaysHidden cho mọi ScrollArea của giao diện Rust, vẫn giữ cuộn hai
chiều bằng wheel/trackpad. Các nút Copy ở request cURL, chi tiết request/console,
JSON, Help và MCP dùng chung content_copy từ font Google Material Icons đã
bundle; giữ tooltip dịch VI/EN, accessible name và hành vi clipboard/toast.
Bỏ hình Copy vẽ thủ công trong JSON để dùng cùng font icon với phần còn lại.

Hai UI tests trong tests/desktop_ui.rs tái hiện lỗi trước fix: scrollbar còn
được paint, cURL chưa paint icon. Sau fix kiểm paint thực tế, wheel hai chiều
và clipboard đều pass. Regression 287 pass/0 failed/0 ignored; cargo fmt --check,
cargo clippy --all-targets --locked --offline -- -D warnings và cargo build
--locked --offline pass. Bộ test tích hợp cần quyền socket localhost; lần chạy
trong sandbox bị PermissionDenied, lần chạy có quyền socket đã pass toàn bộ.
Logs: docs/artifacts/ui-parity/scroll-copy-{regression,clippy}.log; build/hash
ở scroll-copy-build.txt. Đã đồng bộ target/debug/FNV Rust.app, SHA256 executable
khớp binary vừa build. Không chạy screenshot native trong lượt sửa này.
Source Flutter đã được hoàn nguyên các thay đổi nhầm dự án; git status sạch.


## Sửa request inspector và nền bảng (2026-11-13)

Chỉ sửa Rust. URL của inspector dùng layout trái riêng, không còn chịu layout
phải của nút cURL. Overview hiển thị các trường key/value trong card có title,
viền, separator, font và spacing; request/response headers dùng bảng key/value
responsive, giá trị chọn/copy được. Raw dùng chung JSON/Tree viewer có syntax,
fold/find/copy; trạng thái viewer được giải phóng khi đổi request/clear.
Header/body vắng dữ liệu hiển thị “Không có dữ liệu” ở locale VI (“No data” ở
EN), không hiển thị null/{} hoặc nút Copy rỗng. Omitted body giữ cảnh báo giới hạn.
Nền list/inspector dùng surface (trắng ở light, surface tối ở dark). Request
được chọn có overlay RGB(37,99,235), alpha26/255 ≈10%, giữ chữ text/muted để
đủ tương phản thay chữ trắng của nền chọn cũ.

UI tests mới trong tests/desktop_ui.rs kiểm vị trí paint URL, card/background/font,
Raw JSON clipboard, empty states VI ở bốn tab, nền trắng và RGBA/text của row
chọn. Test căn URL tái hiện paint URL lệch trái trước khi đổi add_sized sang add.
Tổng regression 290 pass/0 failed/0 ignored; fmt/clippy all-targets -D warnings và
build pass. Logs docs/artifacts/ui-parity/inspector-*.log, hash trong
inspector-build.txt. Đã cập nhật target/debug/FNV Rust.app, hash executable khớp
binary mới. Chưa kiểm screenshot native trong lượt sửa này.


## Cuộn URL inspector (2026-11-13)

Bỏ truncate ở URL đầu inspector Rust; đặt label Extend trong ScrollArea ngang,
ẩn scrollbar, giữ layout trái và nút cURL ngoài vùng cuộn. Scroll ID theo session/
request để URL mới bắt đầu ở vị trí trái. Wheel ngang, wheel dọc (egui chuyển
sang trục duy nhất) và drag background cuộn nội dung mà không dịch nút Copy.

TDD: test inspector_url_scroll_reveals_end_keeps_copy_fixed_and_resets_per_request
fail trước fix vì text bị truncate. Sau fix kiểm paint full URL dài, wheel đến
ký tự cuối trong clip, nút Copy giữ vị trí và clipboard đủ URL; đổi request giữ
căn trái. Regression 291 pass/0failed/0ignored; fmt/clippy all-targets -Dwarnings/
build pass. Logs docs/artifacts/ui-parity/url-scroll-*.log; build/hash ở
url-scroll-build.txt. Đã đồng bộ target/debug/FNV Rust.app và xác minh hash.
Chưa kiểm screenshot native trong lượt sửa này. Không sửa source Flutter.


## Icon X thanh find và JSON collapse (2026-11-13)

Rust: nút X không còn dùng ký tự nhân cỡ chữ mặc định. X/Previous/Next dùng
Google Material Icons cùng font18, hit target24 và tooltip/accessibility name.
Giữ thao tác clear query và điều hướng matches. Code JSON collapse append
` ... ` và closing row tương ứng để có `{ ... }`, `[ ... ]`, giữ dấu phẩy nếu
container còn sibling. Chỉ đổi trình bày, source/copy/find/fold indexes giữ nguyên.

TDD hai UI tests mới: fail trước fix; sau fix kiểm glyph/font/size, X clear,
object/array/nested/root/trailing comma, expand và Copy vẫn đủ dữ liệu.
50 desktop UI tests pass. Regression chạy tuần tự 293 pass/0failed/0ignored;
fmt/clippy all-targets -D warnings/build pass. Hai lần regression song song
ban đầu có socket MCP/VM failures; test MCP riêng pass và full serial pass.
Lưu cả logs lỗi và thành công tại docs/artifacts/ui-parity/fold-icons-*.
Đã cập nhật target/debug/FNV Rust.app, hash executable khớp binary build mới,
chi tiết fold-icons-build.txt. Không sửa Flutter; chưa chụp screenshot native.


## Bỏ hàng tiêu đề lặp trong tab request (2026-11-13)

Bỏ cả hàng tên tab + nút Copy và separator dưới hàng đó ở card Overview,
Request header, Response header. Giữ card và các hàng key/value; Request body,
Response body và toolbar JSON/Tree của Raw không thay đổi vì không có hàng tên
tab lặp kiểu này. Kiểm thử UI hiện có đổi contract để kiểm không paint tên tab
trong card, không còn copy-detail và dữ liệu bắt đầu sát lề12. Red trước fix,
50 desktop UI pass sau fix. Regression 293 pass/0failed/0ignored; fmt/clippy/
build pass. Lần regression đầu timeout coordinator socket VM; lần chạy lại full
serial pass. Lưu cả hai logs ở docs/artifacts/ui-parity/card-header-*.
Đã cập nhật target/debug/FNV Rust.app và xác minh hash với binary mới build.
Không sửa Flutter; không kiểm screenshot native trong lượt này.


## Sửa Settings icons và alignment (2026-11-13)

Chỉ Rust. Section icons dùng đúng codepoints của font Material Icons bundled:
Interface=tune E683; Connection=lan_outlined F062C; MCP=hub_outlined F0616;
App updates=system_update_alt E637. Trước đây dùng mapping của phiên bản font
khác nên paint thành icon không liên quan. Toggle auto-detect bỏ add_sized
centered label, dùng child layout left để chữ căn trái. MCP port/endpoint có
layout top_down riêng để label ở trên input, cả khi hai field cạnh nhau ở wide;
ở narrow stack và port dùng toàn chiều rộng. Bỏ fixed-width240 copy/client
wrapper để nhóm action bám phải, menu client ngoài cùng phải, Copy/Save kế tiếp;
Connection và App updates giữ nút căn phải.

TDD settings_icons_labels_fields_and_actions_follow_requested_layout fail trước
fix vì icon sai. Sau fix kiểm paint glyph/family, x label auto-detect, y/x label
các input và right edge các action. Tests f17_ui/f18_ui: 22 pass; full serial
regression 294 pass/0failed/0ignored; fmt/clippy all-targets -Dwarnings/build
pass. Evidence docs/artifacts/ui-parity/settings-layout-*; đã cập nhật
 target/debug/FNV Rust.app, xác minh hash. Chưa kiểm screenshot native lượt này.


## Header filters, compact search và MCP vertical alignment (2026-11-13)

Rust: bỏ hai ComboBox Method/Status khỏi thanh search Network, chuyển popup
chọn cùng giá trị/keys cũ vào nút sát phải header tương ứng. Nút tròn16, nền
trắng, border1/line và tam giác xuống accent xanh; margin phải4. Header vẫn
hiện khi filter không có kết quả để người dùng đổi lại filter. Title Method/
Status dùng font8/lề4 để đủ chỗ ở cột mặc định hẹp. Search Network mở rộng hết
hàng; Console giữ Source/Level. Search content padding ngoài8x4 (Frame margin
7x3 cộng stroke1), font10, gap icon/text8, hint #9CA3AF, TextEdit margin0.

Settings: TextEdit dùng vertical_align Center; endpoint dùng child horizontal
Align::Center trong chiều cao28 + padding dọc8. Probe endpoint trỏ cả field để
kiểm vị trí chữ giữa field thực tế. Test input port/endpoint tâm chữ khớp tâm y.

UI tests mới kiểm filter header/circle/right edge, padding/hint font/color,
combined filter và recover từ empty results, MCP text center. Regression297
pass/0failed/0ignored (serial), fmt/clippy all-targets -Dwarnings/build pass.
Evidence docs/artifacts/ui-parity/header-filters-*. Đã cập nhật
 target/debug/FNV Rust.app và xác minh hash. Không sửa Flutter, chưa kiểm native
screenshot trong lượt này.


## Đồng bộ outer padding ba hàng header (2026-11-13)

Rust: main header/navigation/search panel dùng cùng outer margin ngang8/dọc4.
Main header48 giữ nguyên; navigation wide48 (thay50), narrow94 giữ đủ hai hàng;
search Network40 = content32 + padding8; Console48 giữ controls40. Probe bounds
và title x cập nhật theo padding mới; không đổi padding bên trong search đã sửa.

TDD three_header_rows_have_eight_by_four_outer_padding kiểm outer/inner rect
cả bốn cạnh ở width480/1360, search-frame cách row edges đúng8/4. Red trước fix,
8 ui_parity pass sau fix. Full serial regression298 pass/0failed/0ignored;
fmt/clippy all-targets -Dwarnings/build pass. Evidence
 docs/artifacts/ui-parity/header-padding-*. Đã đồng bộ target/debug/FNV Rust.app,
xác minh hash binary; không sửa Flutter, chưa kiểm native screenshot lượt này.


## Header kết nối thủ công và chọn dự án (2026-11-13)

Chỉ Rust. Chuyển app/session selector lên header dòng1; bỏ chip trạng thái
và nút scan icon cũ. Header có URL + input + Sử dụng, kế bên nút Material
icon/text “Quét dự án & kết nối”. Các action icon dòng1 dùng nền trắng,
border xám nhạt, bo góc7, icon xanh. Settings/Help giữ bên phải; app list và
nhóm URL/scan cuộn ngang độc lập khi cửa sổ hẹp, không hiện scrollbar.
Giữ header48 và outer padding8x4; thu gọn padding nút Sử dụng để không
đẩy các hàng header bên dưới.

Không tự quét hoặc kết nối URL đã lưu khi mở app, kể cả preference cũ
autoDetect=true. Bỏ auto-detect toggle khỏi Settings. Mỗi lần bấm quét chỉ
quét một lần: 1 dự án mới tự kết nối; từ2 dự án mở overlay list dọc để chọn;
không tìm thấy hoặc lỗi thì dừng, người dùng chủ động quét lại. Chọn dự án
đóng picker ngay khi bắt đầu kết nối. Kết nối đã thiết lập vẫn giữ cơ chế
reconnect hiện có khi mất transport.

TDD manual_connection_controls_and_app_list_live_in_first_header_row kiểm
header placement, bỏ chip cũ, manual URI normalize, scan và vertical picker.
Thêm startup test với saved URI/autoDetect=true và narrow-header scroll test;
giữ app filtering Network/Console và cập nhật discovery retry contracts.
Full serial regression301 pass/0failed/0ignored; fmt/clippy all-targets
-Dwarnings/build pass. Evidence docs/artifacts/ui-parity/manual-connect-*.
Đã đồng bộ target/debug/FNV Rust.app và xác minh SHA256 binary.
Chưa kiểm screenshot native trong lượt này.


## Căn phải nhóm URL/scan và bỏ wrapper (2026-11-13)

Chỉ Rust. Nhóm URL/Use/scan căn phải trong vùng header-connections, giữ
Settings/Help ngoài cùng phải. Bỏ label URL và toàn bộ Frame có border/margin
bao quanh input/nút; giữ gap8 giữa input, Use và scan. English label đổi từ
Use URL sang Use; Vietnamese vẫn Sử dụng.

TDD header_connection_controls_align_right_without_url_wrapper: red trước
fix, green sau fix; kiểm gap12 đến Settings ở widths1360/1600, gap8 giữa
controls, text Use, không có URL label và không có rectangle stroke bao
cả input/nút. Toàn bộ55 desktop UI tests pass gồm manual connect và narrow
scroll. Full serial regression302 pass/0failed/0ignored, fmt/clippy/build
pass; evidence docs/artifacts/ui-parity/header-right-*. Đã đồng bộ
target/debug/FNV Rust.app, xác minh SHA256. Chưa kiểm native screenshot.


## Căn chiều cao nút Use (2026-11-13)

Rust: allocation của Use đổi28 thành32 để khớp chiều cao thực32 của button
và nút scan. Trước đây button nở ra ngoài allocation và lệch xuống2px.
Bổ sung assertion height/top trong header_connection_controls_align_right_without_url_wrapper
ở widths1360/1600: red với Use y10..42 và scan y8..40; green sau fix.
Regression302 pass/0failed/0ignored; fmt/clippy/build pass. Evidence
docs/artifacts/ui-parity/use-height-*. Đã cập nhật FNV Rust.app và xác minh
SHA256; không sửa Flutter, chưa kiểm native screenshot.


## Project list căn phải và padding (2026-11-13)

Rust: project list co theo kích thước chữ (giới hạn360 wide/140 narrow),
căn cùng nhóm URL/Use/scan về phải, cách input12. Khoảng trống dư nằm
trước list. Đo scan button qua cùng LayoutJob dùng render để giữ alignment
khi locale hoặc trạng thái scan thay đổi. Hai vùng scroll apps/controls
giữ độc lập ở narrow. Frame list margin0; từng chip dùng text13 và padding
ngang8/dọc4, spacing giữa chips0. Giữ shared pill renderer của navigation.

TDD project_list_is_right_grouped_with_zero_outer_and_eight_by_four_chip_padding
red trước fix; green kiểm list/input gap, zero outer padding và kích thước/
vị trí text trong từng chip ở widths1360/1600 (tolerance pixel rounding0.51).
56 desktop UI tests pass, gồm filtering nhiều app và narrow scroll. Full
serial regression303 pass/0failed/0ignored; fmt/clippy/build pass. Evidence
docs/artifacts/ui-parity/project-padding-*. Đã đồng bộ FNV Rust.app và
xác minh SHA256. Không sửa Flutter; chưa kiểm native screenshot lượt này.


## Khôi phục width project list (2026-11-13)

Rust: bỏ co width theo nội dung đã thêm ở lượt trước; khôi phục360px wide,
140px narrow. Giữ right alignment, outer padding0 và từng chip8x4.
Bổ sung assertion list width360 vào UI test project list: red trước fix,
green sau fix. 56 desktop UI tests pass; full serial regression303 pass,
0failed/0ignored. fmt/clippy/build pass. Evidence project-width-* trong
docs/artifacts/ui-parity. Đã đồng bộ FNV Rust.app và xác minh SHA256.
Không sửa Flutter; chưa kiểm native screenshot.


## Chuyển search lên dòng2, width350 (2026-11-16)

Chỉ Rust. Tách capture_search dùng chung Network/Console; chuyển search
lên navigation row, ngay trái pause/clear/export với gap12, width350
(gồm border và padding). Giữ padding8x4/font10/hint xám/gap icon8 cùng
reset behavior. Network bỏ filter panel dòng3; Console giữ Source/Level
ở dòng3. JSON không có capture search/actions. Navigation chuyển layout
hai hàng dưới960px để đủ tabs và search/actions; nhóm search/actions
có scroll ngang ẩn scrollbar ở narrow, search vẫn350.

TDD search_is_350_wide_in_navigation_before_capture_actions red trước fix,
green sau fix: kiểm Network/Console width/placement/gap/vertical center
và không có Network dòng3. Geometry/padding regression cập nhật theo
layout2 dòng; giữ filter/search/reset tests. 65 UI tests pass
(desktop_ui57 + ui_parity8). Full serial regression304 pass/0failed/0ignored;
fmt/clippy/build pass. Toolchain /tmp bị dọn nên đã tải lại Rust stable
và locked dependencies. Evidence docs/artifacts/ui-parity/search-row-*.
Đã thay binary app bằng atomic replace sau khi copy trực tiếp báo EIO;
SHA256 app và binary build trùng nhau. Chưa kiểm native screenshot.


## Bỏ branding bên trái header (2026-11-16)

Rust: bỏ logo hub và text Flutter Network Viewer ở dòng1 cùng allocation
của chúng. Nhóm project/URL/scan/settings/help tiếp tục căn phải.
Thay UI contract của title bằng header_does_not_render_brand_logo_or_title
ở widths480/1360: red trước fix, green sau fix. 65 UI tests pass; full
regression304 pass/0failed/0ignored. fmt/clippy/build pass. Evidence
docs/artifacts/ui-parity/remove-brand-*. Đã atomic-replace app binary và
xác minh SHA256. Không sửa Flutter; chưa kiểm native screenshot.


## Header desktop gọn hai dòng và viền input (2026-11-16)

Triển khai thiết kế header v5 đã được duyệt, chỉ trong Rust. Hai dòng32px;
tabs giữ kiểu pill cũ, width bằng nhau114px và nội dung căn giữa; thu nhỏ
ở narrow và dùng icon ở dưới440px. Project list lấp phần còn lại dòng1,
Scan → Manual URL → Settings/Help. Bỏ input URL/Use inline; Manual URL
mở modal title/input/Find, URL sai giữ popup với lỗi, URL hợp lệ normalize,
phát Action::Connect và đóng popup. Dòng2 căn phải, Network/Console search
350px và pause/clear/download; JSON Format/Find/Copy/Download. Console
Source/Level chuyển xuống heading dạng dropdown tròn như Network, vẫn
truy cập được khi dữ liệu trống hoặc không có kết quả.

Sửa selection.stroke vốn màu trắng khiến TextEdit mất viền focus. Input
mặc định xám, focus xanh trong cả theme sáng/tối; composite capture search,
JSON Find và JSON multiline có outline theo focus thật. Settings, Manual
URL và body JSON Find dùng cùng quy tắc. Không sửa Flutter.

TDD thêm4 UI tests cho compact header/modal/filter, input paint sáng/tối,
JSON toolbar export thực và settings input; red logs được lưu. Regression
308 pass/0failed/0ignored; fmt, clippy -D warnings và build pass. Bằng chứng:
docs/artifacts/ui-parity/compact-header-{red,input-red,fmt,regression,clippy,build}.log.
Kiểm tra native app trực tiếp: header không dư bên phải, search idle xám /
focus xanh, popup Manual URL focus xanh, Console headings và JSON toolbar
đúng vị trí. Test f19_ui xác minh Settings vẫn bấm được ở360px. App bundle
đã atomic replace và SHA256 trùng binary build:
d97baffbde4d42b3e66e3a877c14922b749c3c27072d40c7b4da75187d739893.


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


## Cỡ chữ thống nhất và bảng chữ thuần (2026-11-16)

Theo yêu cầu UI mới: tất cả text của Rust dùng chung cỡ mặc định14,
bao gồm title/button, Network/Console, inspector, JSON/code/Tree/find,
Settings/Help/notifications. Material Icons giữ kích thước icon riêng.
Settings → Interface → Font size có numeric field (nhập/kéo) và nút−/+,
giới hạn10–24, áp dụng frame tiếp theo và lưu fontSize khi khởi động lại.
Giá trị lỗi/missing dùng14; lỗi ghi giữ giá trị đã commit và popup lỗi.
Không thay đổi source Flutter.

Network Method/Status chuyển sang chữ thuần màu text, không badge/nền màu;
Console level cũng bỏ badge. Thêm line dưới header và line dọc liên tục
qua từng row Network, giữ drag resize/filter/selection. Cột có min width
theo cỡ chữ; row height tăng theo cỡ chữ. JSON cập nhật line height và
invalidate width cache khi đổi font. Header tabs rộng hơn để text14 vừa;
search/header/footer tăng chiều cao cho font lớn.

TDD: unified_font_setting_updates_all_painted_text_and_plain_network_grid
red trước implementation, green sau. Test kiểm paint sections cùng cỡ14,
Settings−/+15→14, tất cả inspector tabs với10/24/14 ở light/dark, JSON,
Help, table grid và không có badge. Settings test
font_size_defaults_validates_and_survives_restart kiểm defaults/range/NaN/
Infinity/corrupt values và reload file thật. Cập nhật assertions cỡ chữ cũ.
Regression phát hiện context lock lồng trong glyph measurement; đã sửa bằng
resolve FontId trước fonts_mut. Cũng sửa save-result phát mỗi frame thành
chỉ khi giá trị thay đổi; các notification/scroll regression đều pass.

Validation:313 pass/0failed/0ignored, cargo test --locked -- --test-threads=1;
cargo fmt --check, cargo clippy --all-targets --locked -- -D warnings,
cargo build --locked đều pass. Sandbox ban đầu chặn localhost sockets;
full regression cuối chạy host với quyền được cấp. Logs lưu trong
 docs/artifacts/ui-parity/unified-font-* (giữ cả các lượt red/failure).
Native đã xem shell và Settings có field14/−/+; thao tác click+ bị CUA
noWindowsAvailable, không tuyên bố native click pass; pointer UI test pass.
App bundle target/debug/FNV Rust.app được atomic replace; SHA256 trùng:
c60bb314e51a718a465647f01139f880f4e850f551e0d0f211624b98703da6c2.
Đây là đợt chỉnh UI theo yêu cầu, không mở chuỗi port mới; các blocker
acceptance F21 trong release.md vẫn giữ nguyên.


## F13 — Chặn nội dung cuộn đè lên table header (2026-11-16)

Sửa src/desktop_network.rs: giới hạn clip của network-rows tại header.bottom()
trước khi render rows. Egui ScrollArea mặc định mở rộng clip_rect_margin3px
khiến row text/grid/selection được vẽ sau nền header và tràn lên tiêu đề.
Clip mới áp dụng cho toàn bộ painter và hit testing trong vùng content.

TDD: scrolling_network_rows_are_clipped_below_fixed_header thực sự cuộn wheel23px
với120 records, dừng giữa row; kiểm header đứng yên và text clip không vượt
mép dưới header ở light/dark, font14/24. Red tái hiện GET clip top91 trong khi
header bottom94; sau sửa green. Không sửa Flutter.

314 tests pass/0failed/0ignored qua cargo test --locked -- --test-threads=1
(host execution cho localhost VM sockets). cargo fmt --check,
cargo clippy --all-targets --locked -- -D warnings và cargo build --locked pass.
Evidence docs/artifacts/ui-parity/header-scroll-{red,green,regression,fmt,clippy,build}.log.
Đồng bộ atomic target/debug/FNV Rust.app, SHA256 binary/bundle khớp:
65023bb2a64f35dd4898b16efbc57444af607436bbe02e524cf6f92e8156daf0.
Kiểm chứng bằng production egui renderer/input; chưa kiểm native screenshot
lượt này. F21 environment blockers giữ nguyên, không tạo chuỗi chat thất bại.


## F13 — Kéo sát title và filter (2026-11-16)

Sửa minimum column widths trong src/desktop_network.rs. Bỏ ước lượng theo
font_size*4.8/3.6 đã tạo khoảng trống dư (~19.6px Method ở font14).
Đo galley của title đã dịch bằng font thực; minimum gồm left inset4,
title width làm tròn lên, gap2, filter16, right inset4. Dùng cùng minimum
khi clamp các divider0/1/2 và khi render để không bị bật rộng trở lại.
Giữ validation widths tương thích Settings và hành vi filter/resize.

TDD UI network_columns_shrink_to_small_title_filter_gap red→green:
pointer kéo divider Method/Status hết cỡ, paint gap1.5–3.5px, toàn title
không bị clip, filter GET/Success vẫn bấm được; EN/VI, font14/24.
Full315 tests pass/0failed/0ignored (host execution cho localhost sockets).
cargo fmt --check, cargo clippy --all-targets --locked -- -D warnings,
cargo build --locked pass. Logs docs/artifacts/ui-parity/filter-gap-*.
Không sửa Flutter. Native screenshot chưa kiểm lượt này; kiểm production
egui painter/input. Bundle target/debug/FNV Rust.app được atomic replace,
SHA256 executable khớp binary:
638efec022ffb473be4407303d1c12e892ca83ecef60fd25fa4f50fe0ffe428d.
F21 environment blockers giữ nguyên; không tạo chuỗi chat thất bại.


## F20 — Không popup lỗi cập nhật nền lúc mở app (2026-11-16)

Nguyên nhân: DesktopApp tự check(true) lần đầu; SupportUi dùng view.status
để notify nên configure/feed/network failure của background check hiện popup
Error ngay startup. Sửa Updater::view: background Failed/Current trình bày Idle,
giữ status nội bộ để retry/debug. Manual check(false) vẫn hiển thị lỗi/current;
Available/download/install giữ hành vi thông báo và native confirmation.

TDD startup_update_failure_is_silent_but_manual_retry_reports_errors red→green:
controller thật + fake backend + production renderer/input, cả macOS/Windows
branch. Configure failure/check failure/async failure/60s timeout/current đều
không popup ở startup hoặc khi mở Settings; click CheckUpdates rồi manual
failure vẫn popup đúng và dismiss không lặp. Existing updater tests pass.
Không sửa Flutter, chưa kiểm native screenshot lượt này.

316 regression tests pass/0failed/0ignored trên lượt retry; lượt đầu gặp test
coordinator_socket VM did not progress (giữ log). fmt/clippy -D warnings/build
pass. Evidence docs/artifacts/ui-parity/startup-update-{red,green,fmt,clippy,
build,regression,regression-retry}.log. Atomic replace FNV Rust.app, SHA256
khớp binary build:
f68a830b5036121aa37269ad3ff076b887bd7bb21ddc35aef97cbd10d1d7d92f.
F21 environment blockers giữ nguyên; không tạo chuỗi chat thất bại.


## Vintage widget lab — bước duyệt style (2026-11-16)

Yêu cầu mới là tạo kit/demo trước, không migrate app hiện tại cho đến khi người
dùng duyệt. Thêm src/vintage.rs với palette xanh ngà/mực xanh đen, radius0,
line1px, không shadow. Bộ primitives card/button/icon/tabs/input/toggle và
các composition phục vụ35 nhóm widget, chia7 tab Overview/Controls/Navigation/
Tables/JSON/Bodies/Feedback. Catalog đầy đủ đối chiếu source hiện tại:
 docs/vintage-widget-inventory.md; executable mapping vintage::INVENTORY.

Native DesktopApp khởi động với ui.vintage_open=true. DesktopUi mặc địnhfalse
để giữ renderer/test cũ. Demo có nút “Mở app hiện tại →”; native host có nút
quay lại demo. Các renderer cũ được giữ, style cũ được restore khi chuyển về.
Không thay đổi Flutter và không tự migrate/publish hoặc tạo chat port kế tiếp.

Kit dùng chung fontSize14 mặc định và Settings−/+/numeric field10–24 hiện có.
JSON code dùng tokenizer production qua vintage_syntax_job với7 màu thay thế;
Tree giữ cùng7 vai trò màu. Find dùng controller regex/case/word của app,
chỉ thay nền highlight; fold/copy giữ JSON đầy đủ. Gallery dùng dữ liệu mẫu,
chưa nối thao tác mẫu Connect/Download với các dịch vụ thật của app.

TDD tests/vintage_ui.rs trước code (red thiếu fields/module), sau đó4 tests
pass: zero radius và một font size ở mọi tab14/20; stepper/toggle,7 màu JSON,
fold/copy/modal/notice; kiểm tất cả35 example probe IDs có thật, ảnh enlarge,
nav/back ở480x360 và1360x860; native host default route và legacy defaults.
Full320 tests pass/0failed/0ignored. cargo fmt --check,
cargo clippy --all-targets --locked -- -D warnings và cargo build --locked pass.
Logs docs/artifacts/ui-parity/vintage-{red,green,fmt,clippy,build,regression,
regression-final}.log. Native CUA getApp thành công nhưng getScreenshot timeout
và reset kernel, chưa có ảnh native xác nhận; không đánh dấu native visual pass.
App bundle target/debug/FNV Rust.app được atomic replace; SHA256 khớp binary:
72ba6c26137714367b4fe338292a692f8ce589c46fe45e9c843e5001852519c9.
Cần người dùng duyệt palette/widget trong demo trước bất kỳ migration toàn app.


## Vintage demo — sửa6 điểm review (2026-11-16)

Chỉ chỉnh src/vintage.rs, giữ nguyên renderer app cũ/Flutter và bước chờ duyệt.
1. Bỏ override_text_color cố định để selected option dùng selection foreground
   BACKGROUND#EBF0E4 trên nền PRIMARY#282F26. Primary button và selected table
   row cũng dùng đúng BACKGROUND cho text, thay vì SURFACE sáng khác mã màu.
2. Helper notice_heading dùng icon trong tile vuông28px nềnPRIMARY, glyph
   BACKGROUND. Dùng chung cho Info/Success/Warning/Error và confirmation dialog.
3. Helper input_outline vẽ viềnPRIMARY1px bên trong response rect cho input,
   search, multiline, numeric field; không còn dùng viền focus màu giấy gần nền.
   Giữ selection stroke màuBACKGROUND để chữ được chọn đủ tương phản.
4. Toàn card JSON/code/Tree dùng BACKGROUND#EBF0E4. Giữ7 vai trò màu cú pháp.
5. Table scope item_spacing.y=0, không có gap10px giữa header/rows; thêm top/
   left edges và outline liên tục quanh toàn viewport. Giữ fixed header, wheel,
   filter và row selection; scrollbar ẩn tránh cột phải lệch với header.
6. Bỏ circular spinner trong vintage demo, thêm reusable loading(ui,count):
   clamp3–10, slot16px nềnBACKGROUND/viềnPRIMARY1px, gap4px. Ô active fillPRIMARY
   inset3px = border1+padding2. Mỗi200ms next, modulo count lặp vô hạn và request
   repaint; gallery có5 ô trong scan row cùng biến thể compact3/wide10.

TDD:4 tests mới trong tests/vintage_ui.rs trước code. Red thiếu loading API;
lượt green đầu phát hiện assertion phải xét effective TextShape fallback màu
và chờ popup fade hoàn tất. Sau sửa assertions test đúng paint production:
selected_popup_options_use_paper_text_and_focused_inputs_keep_ink_borders,
notice_icon_has_ink_tile_paper_glyph_and_json_has_paper_background,
vintage_table_grid_is_contiguous_and_has_outer_edges,
square_loader_cycles_one_inset_cell_for_three_to_ten_slots.
Kiểm đủ3..10 slots qua2 vòng và wrap, chỉ1 ô active mỗi frame, exact fill/border/
padding; input focus thật, dropdown mở thật, modal/notice thực, table geometry
và outer viewport border.8 vintage tests pass; full324 pass/0fail/0ignored.
cargo fmt --check, cargo clippy --all-targets --locked -- -D warnings,
cargo build --locked pass. Evidence docs/artifacts/ui-parity/vintage-refine-*.
Không kiểm native screenshot lượt này; kiểm production egui renderer/input.
Bundle target/debug/FNV Rust.app được atomic replace, SHA256 khớp binary:
adff3e705b8af5ee3bfe96821f4dfcfe2c808dae852b3ed2082d3ad7f818d20f.
Chưa migrate toàn app, chưa tạo chat mới; chờ người dùng review tiếp.


## Vintage demo — metadata và Console dạng table (2026-11-16)

Chỉ chỉnh demo vintage theo review mới. Thêm reusable key_value_table với
header Title / Value; khi có Copy thêm cột Action. Metadata dùng2 cột, Form
fields/file metadata dùng3 cột. Các hàng/cột liền nhau, có viền ngoài; value
selectable/truncate với tooltip đầy đủ, Copy giữ nguyên Unicode/raw value.
Render nội dung trong child UI riêng để không thay đổi cursor của hàng cha.

Console dùng4 cột Source / Level / Time / Message, header cố định, viền liên
tục, selected row dùng màu primary/paper. Vùng scroll clip nội dung dưới header,
cuộn ngang cho cửa sổ hẹp. Giới hạn chiều rộng gallery theo viewport thực tế
để cột phải và Action không tràn khỏi cửa sổ desktop. Giữ font chung và JSON.

TDD2 UI tests trước code: metadata_and_form_use_aligned_grids_with_optional_copy_action_column
và console_has_four_real_columns_grid_selection_and_fixed_header_when_scrolling.
Red thiếu table probes; test tiếp bắt row overlap do ui.put và Copy ngoài viewport.
Green kiểm2/3-column geometry, row continuity, pointer Copy ra clipboard đúng
Unicode,4 header Console, selected colors, outer border, wheel/fixed header và
clip.10 vintage tests pass; full326 pass/0failed/0ignored. cargo fmt --check,
cargo clippy --all-targets --locked -- -D warnings, cargo build --locked pass.
Evidence docs/artifacts/ui-parity/vintage-tables-{red,green,regression,fmt,clippy,build}.log.
Không kiểm screenshot native; kiểm production egui renderer/input.
Atomic replace bundle target/debug/FNV Rust.app; SHA256 khớp binary:
8661490de25234285fb34afc8764bbaf55e1af4dbc524a8107d4a4ee3bb04e2f.
Chưa migrate app cũ/Flutter, tiếp tục chờ duyệt demo; không tạo chat port mới.


## Vintage demo — square icon buttons (2026-11-16)

Mọi nút chỉ chứa glyph dùng chung vintage::icon:34x34px, padding4px,
corner0; toolbar/Controls và Copy cột Action dùng cùng helper. Không kéo dài
nút Copy theo chiều rộng cell; giữ selectable value, tooltip và clipboard.
Legacy app/Flutter không đổi, vẫn chờ duyệt kit.

TDD icon_buttons_are_square_in_controls_toolbar_and_table_actions trước code:
red bắt Controls icon42x34 ở font10. Green kiểm width==height ở10/14/24,
Controls/toolbar5 glyph và3 Action buttons nằm trong cell.11 vintage UI tests
pass, full327 pass/0failed/0ignored; cargo fmt --check, cargo clippy
--all-targets --locked -- -D warnings và cargo build --locked pass.
Evidence docs/artifacts/ui-parity/vintage-icons-{red,green,regression,fmt,clippy,build}.log.
Atomic replace bundle target/debug/FNV Rust.app; SHA256 khớp binary:
3bd043e5752e45448790e86a24ca363c863e0865313b5c5ec2959a6fe2070d50.
Kiểm production egui UI/input, chưa kiểm screenshot native. Không tạo chat port
mới trong bước review demo.


## Vintage demo — key/value/action content alignment (2026-11-16)

Metadata và Form dùng cùng key_value_table: Title content căn trái8px,
Value content căn phải8px bằng right_to_left layout (giữ thứ tự chữ bình thường,
selectable/truncate/tooltip), Action đặt child rect34x34 tại tâm cell và dùng
icon helper hiện có. Header giữ style hiện tại. Legacy app/Flutter giữ nguyên.

TDD key_value_content_aligns_title_left_value_right_and_actions_center trước
code: red bắt Value đang căn giữa. Green đo text galley rect/position thực và
clip của production egui, Title/Value và Action center ở font14/24; nút giữ vuông.
12 vintage UI tests pass; full328 pass/0failed/0ignored; cargo fmt --check,
cargo clippy --all-targets --locked -- -D warnings, cargo build --locked pass.
Evidence docs/artifacts/ui-parity/vintage-alignment-{red,green,regression,fmt,clippy,build}.log.
Atomic replace bundle target/debug/FNV Rust.app; SHA256 khớp binary:
c54153c10a82dd60d7323909e09c1a025f67df4889b6329aed6de9ffcddaa010.
Không kiểm screenshot native; chưa migrate app/chưa tạo chat port kế tiếp.


## Vintage demo — hover action scale1.2 (2026-11-16)

Thêm reusable hover_action(ui,show): ghi nhận paint range của control, khi
hover/enabled transform toàn bộ shape gồm background/border/glyph/text mesh
1.2 lần quanh tâm. Không thay allocation hay font setting; hover rời về1.0.
Re-register cùng widget ID với rect phóng lớn để viền phóng ra vẫn click được.
Tắt expansion mặc định của egui để tỷ lệ đúng1.2, không cộng thêm1px hover.

Dùng cho button/quiet/icon/tab/toggle/Copy/confirmation/image actions,
2 dropdown và từng option, checkbox/radio, numeric font field, JSON find
option toggles, color swatch và links. Disabled controls không scale. Base
font từ Settings giữ nguyên; icon base34x34 vẫn vuông khi scale40.8x40.8.
Chỉ demo vintage, không đổi legacy app/Flutter; chờ duyệt trước migration.

TDD action_hover_scales_button_icon_and_dropdown_paint_without_moving_layout
trước code: red thiếu background1.2x. Green đo painted rect và galley size,
restore khi pointer rời, allocation ổn định, click thật ở viền ngoài icon
phát clipboard command. Thêm hover_applies_to_action_variants_and_popup_options_but_not_disabled_buttons
kiểm font14/24, quiet/disabled/checkbox/radio/toggle, dropdown popup option
phóng và chọn được. Harness click chuyển pointer ra ngoài sau click để các
test geometry/focus trước đó tiếp tục đo trạng thái không hover.
14 vintage UI tests pass; full330 pass/0failed/0ignored; cargo fmt --check,
cargo clippy --all-targets --locked -- -D warnings và cargo build --locked pass.
Evidence docs/artifacts/ui-parity/vintage-hover-{red,green,regression,fmt,clippy,build}.log.
Atomic replace bundle target/debug/FNV Rust.app; SHA256 khớp binary:
b55af4b60ce929b6ad152959c7895ba05a790e0f0719699b65e6a85a6f86777f.
Kiểm production egui paint/input; chưa kiểm native screenshot. Không tạo chat
port mới trong bước review demo.


## Vintage demo — render hover bằng kích thước thật, sửa chữ mờ (2026-11-16)

Theo correction của người dùng, thay hoàn toàn cách transform mesh/zoom đã
render. hover_action hiện nhận callback(ui,factor); cache kích thước normal và
widget ID để biết hover, cấp rect width/height1.2, tạo child UI với font sizes,
button padding/interact size và icon spacing lớn hơn trước khi render. Icon
truyền size34x34*factor và glyph18*factor trực tiếp. Không còn Shape::transform,
TSTransform hay thay đổi font atlas/mesh sau render. Hit area dùng rect thực
của widget. Shared base font Settings vẫn giữ14 (hover16.8), glyph18→21.6.

Dropdown popup reset về base style rồi mỗi option áp dụng hover riêng, tránh
scale font hai lần. Layout wrapper cấp slot trước để giữ wrap ở cửa sổ hẹp;
Action table dùng slot tâm nên icon lớn vẫn giữ tâm và tỉ lệ vuông.
Legacy app/Flutter không đổi; chỉ demo vintage, chờ duyệt migration.

TDD đổi action_hover_rebuilds_widget_dimensions_and_font_at_larger_size trước
code: red bắt Response.rect chưa tăng width thật. Test kiểm width/height thực,
font_id.size*1.2 trong LayoutJob, galley rows bằng fresh font layout (thay vì
mesh đã kéo giãn), restore và click clipboard viền ngoài vẫn chạy. Thêm test
hovered_table_icon_keeps_center_and_rasterizes_at_current_dpi trước fix center:
red bắt icon lệch tâm. Green kiểm font14/24, native DPI1/2, icon vuông/tâm Action,
font21.6 và galley pixels_per_point đúng DPI; giữ regression các action variants,
popup selection, disabled, Copy và narrow navigation. Harness truyền native DPI
qua ViewportInfo, không dùng zoom factor để giả lập Retina.
15 vintage UI tests pass; full331 pass/0failed/0ignored. cargo fmt --check,
cargo clippy --all-targets --locked -- -D warnings, cargo build --locked pass.
Evidence docs/artifacts/ui-parity/vintage-hover-sharp-{red,center-red,green,regression,fmt,clippy,build}.log.
Atomic replace bundle target/debug/FNV Rust.app; SHA256 khớp binary:
f4c5df889647dcd7a374d89d8047b5198318e4d0781fad3805960e3556b5506f.
Không kiểm screenshot native; kiểm production egui layout/font/paint/input ở DPI1/2.
Không tạo chat port kế tiếp trong bước duyệt demo.


## Vintage demo — hover tại chỗ, giữ layout (2026-11-16)

hover_action giữ layout footprint của control ở normal size, không advance
parent cursor theo response rect phóng lớn. Cache cả normal offset để giữ đúng
vị trí control bên trong slot; child UI top_down/Min đảm bảo dropdown không tự
căn dọc lệch và không làm hàng cha tăng/giảm chiều cao. Hover child được cấp
rect1.2 quanh tâm normal rect; widget/font được render mới như correction trước,
hit area vẫn là rect lớn thật. Không quay lại bitmap/mesh zoom.

TDD hover_grows_in_place_without_moving_neighbors_rows_or_panels trước code:
red bắt hover tab Controls đẩy checkbox5px. Test sâu ở font24 bắt dropdown
thay đổi layout hàng7px và lệch tâm; fix bằng normal offset + child alignment.
Green so sánh toàn bộ probe rectangles còn lại bằng equality trước/sau hover,
kiểm center trong1px, width/height1.2 (rounding tolerance1.5px), restore toàn bộ
map, font14/24; cover nav tab, primary/icon/dropdown và Copy table. Harness
reveal scroll tới control trước đo rồi chờ scroll settle, không click dropdown
để mở popup ngoài ý định test. Regression font/DPI1/2, popup, disabled và Copy
vẫn pass.16 vintage UI tests/332 full regression pass,0failed/0ignored;
cargo fmt --check, cargo clippy --all-targets --locked -- -D warnings và
cargo build --locked pass. Evidence docs/artifacts/ui-parity/vintage-hover-fixed-slot-*.
Atomic replace bundle target/debug/FNV Rust.app; SHA256 khớp binary:
21630ae1a432884977645d69f0f2b922a14533e6883d1a111680b0ac02a825bd.
Demo vintage only; legacy app/Flutter giữ nguyên, chưa migrate/chưa tạo chat mới.
Không kiểm native screenshot; production egui geometry/font/paint/input verified.


## Vintage demo — nền control đặc và thứ tự vẽ hover (2026-11-16)

Theo review mới, mọi control qua hover_action có underlay SURFACE alpha255
bao phủ toàn response rect, kể cả quiet/link/checkbox/radio/disabled. Thay
weak_bg_fill và non-primary button fill từ transparent thành SURFACE. Primary,
selection và swatch vẫn vẽ màu riêng lên nền đặc, giữ palette và7 màu JSON.

Control hover chuyển paint shapes (không transform) sang sublayer cùng Order,
ngay trên layer cha; clip giữ nguyên và hit testing vẫn ở layer gốc. Nhờ đó
neighbor labels được vẽ sau cũng không đè lên dropdown phóng to. Popup/layer
cao hơn vẫn theo thứ tự egui. Kích thước font thực, DPI, normal layout slot,
center và square icons không đổi.

TDD2 tests trước code: action_controls_have_solid_backgrounds_at_rest_and_hover
red bắt quiet action không có nền; hovered_dropdown_paints_opaque_surface_after_neighbor_labels
red bắt dropdown không có nền đặc. Green kiểm full response background alpha255
cả normal/hover ở primary/quiet/disabled/icon/dropdown/checkbox/radio; kiểm shape
order opaque backing sau neighbor label và own text sau backing. Các kiểm thử
popup selection, enlarged-rim clipboard, no-reflow và DPI1/2 vẫn pass.
18 vintage UI tests/334 full regression pass,0failed/0ignored; cargo fmt --check,
cargo clippy --all-targets --locked -- -D warnings, cargo build --locked pass.
Evidence docs/artifacts/ui-parity/vintage-opaque-{red,order-red,green,regression,fmt,clippy,build}.log.
Atomic replace bundle target/debug/FNV Rust.app; SHA256 khớp binary:
203a2ab50aca2d04ec7ee6acc81af0291432d2224ff952ee60ddd90fdb30a320.
Chỉ demo vintage, legacy app/Flutter giữ nguyên; chưa migrate/chưa tạo chat mới.
Không kiểm native screenshot; production egui paint order/background/input verified.


## Vintage demo — giảm hover xuống1.1 (2026-11-16)

Đổi hệ số hover_action1.2→1.1 theo yêu cầu; width/height/font/icon cùng tăng10%.
Giữ normal layout slot, vị trí tâm, nền đặc và paint overlay. Font14→15.4,
glyph18→19.8, icon34→37.4; không mesh zoom, không đổi base font Settings.

TDD cập nhật expected1.1 và glyph19.8 trước code, red bắt width primary còn1.2.
Green18 UI tests pass: dimensions, LayoutJob font ratio, native DPI1/2, fixed
layout, opaque/order, popup/disabled/clipboard. Variant font test đổi từ ratio
bề rộng glyph raster sang font_id.size thực; native hinting ở cỡ15.4 không đảm
bảo bề rộng glyph đúng tỷ lệ toán học. Full334 pass/0failed/0ignored;
cargo fmt --check, cargo clippy --all-targets --locked -- -D warnings và
cargo build --locked pass. Evidence docs/artifacts/ui-parity/vintage-hover-110-*.
Atomic replace bundle target/debug/FNV Rust.app; SHA256 khớp binary:
1f899647033df0808dfab91d89fccdad7a5a48b4bfa399d5bc9bfd2ae7b706f6.
Chỉ demo vintage; legacy app/Flutter không đổi, chưa migrate/chưa tạo chat mới.
Không kiểm native screenshot; production egui UI tests verified.


## Vintage demo — dropdown không scale (2026-11-16)

Tách action_control với flag scale_on_hover; hover_action giữ1.1, fixed_action
cho dropdown/option giữ1.0 cả width/height/font. Áp dụng Theme và Network filter,
cả nút đóng và menu options. Nền đặc và hover paint overlay vẫn dùng chung;
chọn option/selected colors giữ nguyên. Control khác vẫn hover1.1 tại chỗ.

TDD dropdowns_and_options_keep_dimensions_and_font_size_on_hover trước code:
red bắt closed dropdown đổi rect. Green kiểm exact response rect equality và
font size không đổi khi hover ở font14/24 cho cả2 dropdown và option, click chọn
Vintage ink/GET thực sự cập nhật selected label. Các test hover trước cập nhật
exception dropdown1.0; nút/icon vẫn1.1.19 vintage UI tests/335 full regression
pass,0failed/0ignored; cargo fmt --check, cargo clippy --all-targets --locked
-- -D warnings và cargo build --locked pass. Evidence
 docs/artifacts/ui-parity/vintage-dropdown-no-scale-*.
Atomic replace bundle target/debug/FNV Rust.app; SHA256 khớp binary:
e7e345d99bf0f42a54af7984e919b55f5471e425e628d351f91d646c3c8e1896.
Chỉ demo vintage; legacy app/Flutter không đổi, chưa migrate/chưa tạo chat mới.
Không kiểm native screenshot; production egui geometry/font/input verified.


## Vintage demo — cursor cho widget có thể nhấn (2026-11-16)

Shared action_control trả on_hover_cursor(PointingHand) cho enabled click
controls không nhận drag. Cover button/icon/tab/toggle/checkbox/radio/dropdown/
options/link/swatch; Network và Console selectable rows cũng đặt PointingHand.
Không ép cursor trên disabled, TextEdit, numeric DragValue hay resize splitter;
giữ Default/Text/ResizeHorizontal và restore Default khi pointer ra ngoài.
Không đổi scale1.1, dropdown no-scale, opaque backing hay font/layout.

TDD2 tests trước code: clickable_widgets_and_dropdown_options_use_hand_cursor
red bắt tab Controls vẫn Default; disabled_text_and_resize_widgets_keep_appropriate_cursors
baseline pass. Green kiểm platform_output.cursor_icon từ pointer events thật ở
13 control/row variants và cả2 dropdown menu options, cùng disabled/TextEdit/
numeric/splitter/reset.21 vintage UI tests/337 full regression pass,
0failed/0ignored; cargo fmt --check, cargo clippy --all-targets --locked -- -D
warnings và cargo build --locked pass. Evidence docs/artifacts/ui-parity/vintage-cursor-*.
Atomic replace bundle target/debug/FNV Rust.app; SHA256 khớp binary:
b84479355dea631fcb598de1e4a5f19d931d026943b884d86b593fca1bd026b7.
Chỉ demo vintage; legacy app/Flutter giữ nguyên, chưa migrate/chưa tạo chat mới.
Không kiểm native screenshot; production egui platform cursor/input verified.


## Vintage demo — dropdown list padding (2026-11-16)

Theme/Network ComboBox popup_style menu_margin=0, không padding ngoài list.
Popup content item_spacing.y=0, options liền nhau. Shared dropdown_option dùng
selectable Button full available width với padding12px ngang/8px dọc, giữ
fixed_action (không scale), opaque background và PointingHand. Chọn option
cập nhật selected value/mark_changed, popup close behavior giữ nguyên.

TDD dropdown_list_has_no_outer_padding_and_options_keep_internal_padding trước
code: red bắt option không rộng hết list. Green kiểm cả2 popup ở font14/24:
rows cùng left/right, không gap, text padding>=12/8 (tolerance raster0.5), popup
border1px bao sát union rows (không menu margin). Các tests selected text,
no-scale, cursor, option selection giữ pass.22 vintage UI tests/338 full
regression pass,0failed/0ignored; cargo fmt --check, cargo clippy --all-targets
--locked -- -D warnings và cargo build --locked pass. Evidence
 docs/artifacts/ui-parity/vintage-dropdown-padding-*.
Atomic replace bundle target/debug/FNV Rust.app; SHA256 khớp binary:
74d3c06a8abd335f5b63e2522f7cd68c27d92abce41d7227ce46352b617775b2.
Chỉ demo vintage, legacy app/Flutter giữ nguyên; chưa migrate/chưa tạo chat mới.
Không kiểm native screenshot; production egui geometry/paint/input verified.


## Vintage demo — project selector chữ thuần (2026-11-16)

Bỏ prefix trạng thái ●/○ trước pgapp/staging trong project selector. Labels
hiện All/pgapp/staging; selected style và click behavior giữ nguyên.
TDD project_selector_uses_plain_names_without_status_badges trước code: red
thiếu plain-name project probe; green kiểm painted labels không glyph badge,
click cả3 lựa chọn đổi selected fill/text đúng palette.23 vintage UI tests/
339 full regression pass,0failed/0ignored; cargo fmt --check, cargo clippy
--all-targets --locked -- -D warnings và cargo build --locked pass.
Evidence docs/artifacts/ui-parity/vintage-project-plain-*.
Atomic replace bundle target/debug/FNV Rust.app; SHA256 khớp binary:
8ce5d20415cf4880a280b3a47e1426376afec5a8e7969adc2c798f040831812a.
Chỉ demo vintage; legacy app/Flutter giữ nguyên, chưa migrate/chưa tạo chat mới.
Không kiểm native screenshot; production egui paint/input verified.


## Vintage production — JSON folds, tree connectors và Cascadia Mono

Sau khi duyệt, app Rust mở Network thật; dùng bộ Vintage cho Network/Console,
JSON workspace/inspector, form/header/metadata, Settings, Help và feedback.
Gallery vẫn dùng để kiểm tra component. Code và Tree dùng chung renderer với
production, có mũi tên riêng cho root/object/array lồng nhau, đường nối và 7 màu.
Cascadia Mono bundle kèm OFL; cỡ chữ mặc định 14, chỉnh 10–24 trong Settings.

343 Rust tests và 8 test packaging pass, fmt/Clippy/build pass. Đã quan sát
màn hình Network khi khởi động bản kiểm tra riêng trên macOS; JSON fold/connector,
selection/search/copy và bảng được kiểm bằng production egui paint + input.
Bundle debug đã cập nhật, SHA256 `5ee98260a7dfde3851bab81904dc6f79a3ee9733f84c1e01c239b9c8c5d1422b`. Evidence `docs/artifacts/ui-parity/vintage-production-*`.


## Vintage — empty text và JSON placeholder

Network/Console and unselected inspectors show centered text only. Filter reset and connection remain in the toolbar. JSON workspace has no input heading and displays the new wrapping hints.

345 Rust tests pass (0 failed/ignored); fmt, Clippy and build pass. Verified production egui paint/input; no native screenshot for this change.
Bundle debug SHA256 `07c9d85a33b15e9140888e9d11912c07dc56036f0b887575e9d127afb4902840`. Evidence `docs/artifacts/ui-parity/vintage-empty-text-*`.


## Vintage — JSON pane padding

JSON workspace uses 4 px padding in both panes and top-left placeholder alignment. Verified typed text, wrapped hints and formatted output inset at widths 700 and 1360.

345 Rust tests pass; fmt, Clippy and build pass. Verified production egui paint/input. Bundle debug SHA256 `ce97ccd0ebb7c958a65005911c64f4842f8a35f1729bbd4c99849396cb265bf0`. Evidence `docs/artifacts/ui-parity/vintage-json-padding-*`.


## Vintage — center icon buttons

Square icon controls center the painted glyph on both axes at rest and on hover. UI test reproduced the Settings glyph 2 px left, then verified Settings, Help, Pause, Clear and Export. Copy icon test measures the actual glyph at 1× and 2× DPI.

346 Rust tests pass (0 failed/ignored); fmt, Clippy and build pass. Verified production egui paint and pointer input. Bundle debug SHA256 `234082ac24045755664303988f341ee7fb48e9e2ee960ccfa5714034e0be4c5f`. Evidence `docs/artifacts/ui-parity/vintage-icon-center-*`.


## Vintage — MCP Settings actions (2026-11-16)

MCP port and read-only endpoint share TextEdit font, 12 px padding, height and
border; only the opaque background differs. Replaced generic Copy/client menu
with icon + label buttons “Copy manual MCP for Codex” and “Copy manual MCP for
Claude”. Removed Save & restart: switch saves enablement and valid draft port
atomically, then dispatches service restart/stop. Enter applies port edits;
invalid enable is rejected, while disabling remains possible with an invalid
draft. Copy uses the committed port. Explicit Retry remains for service errors.

Test-first red reproduced obsolete mcp-save. Four F18 UI tests pass, including
painted text padding/font/background at 14 and 24 px, 360 px English/Vietnamese
light/dark interaction at default font, exact clipboard output and file reload.
347 Rust tests pass (0 failed/ignored); fmt, Clippy all targets and build pass.
Updated prior F17 button alignment regression and native smoke script for new
controls; native smoke was not rerun. Verified production egui paint/input, no
new native screenshot. Evidence: `docs/artifacts/ui-parity/vintage-mcp-*`.
Debug bundle SHA256 `f89f73c3618395630b8ae02d603eb416e1486a504656e28938034144db5853e4`.


## Vintage — compact Settings

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
Debug bundle SHA256 `41c9de45c5d323fa1f77d740f0f1a2f5f4e814d42bded895348178d7192b2a4c`.


## Vintage — Settings field widths

Settings field widths (2026-11-16): Theme and Language use equal 220 px
widths, bounded by available space. Read-only MCP endpoint measures its current
URL at the shared font size and adds 12 px padding on each side, capped to
available space; editable port style, selection and opaque background retained.
Test-first red reproduced full-width dropdowns. Production egui test verifies
widths, URL bounds and dropdown selection at 360/1360 px, fonts 14/24, EN/VI.
348 Rust regression tests pass (0 failed/ignored); focused F18 UI 5 pass;
fmt/Clippy/build pass. No new native screenshot. Evidence
`docs/artifacts/ui-parity/vintage-settings-width-*`.
Debug bundle SHA256 `45a6fd97e3e6a79cd3f4d8bf37f034110730191a722d21258d1215ee3d0a4200`.


## Vintage — Settings header padding

Settings header padding (2026-11-16): removed exact height; header follows
button/title content with 12px padding on all sides. Balanced title and
Back/Quay lại remain. Test-first failure confirmed the old zero margin, then
UI tests verify 12px insets, centered title/spacer, fixed header during scroll
and back navigation at 14/24px fonts. 348 regression tests pass, 0 failed/ignored;
fmt/Clippy/build pass. Production egui paint/input verified; no native screenshot.
Evidence `docs/artifacts/ui-parity/vintage-header-padding-*`.
Debug bundle SHA256 `70aa3fd255794e613d42be78b2326306c2711e8c9477d793f40ba98a7a1a9a3e`.


## Vintage — header padding 8px

Settings header padding follow-up: reduced 12px to 8px on all four sides.
Content-driven height and centered title retained. Updated UI assertions first
(red against 12px), then 348 regression tests pass, 0 failed/ignored.
fmt/Clippy/build pass. Production egui geometry/back/scroll tested; no native
screenshot. Evidence `docs/artifacts/ui-parity/vintage-header-padding8-*`.
Debug bundle SHA256 `bd5cbb8252c94ebe7eeeb346d9afa9a64901afd7b4010061075a4d9f5e671f36`.


## Vintage — stable Settings header on Back hover

Settings Back hover fix (2026-11-16): title and trailing spacer use the
reserved resting row height rather than the enlarged Back response. Header
remains content-sized with 8px padding; hover still grows Back 1.1x in place.
Test-first UI reproduced header growth from 48 to 52px. New UI test repeatedly
enters/exits hover, verifies stable header/content/title/spacer bounds, pointing
cursor, enlarged control centered in its slot and successful Back navigation
at widths 360/700/1360, fonts 14/24 and EN/VI. 349 regression tests pass,
0 failed/ignored; fmt/Clippy/build pass. Production egui paint/input tested;
no new native screenshot. Evidence `docs/artifacts/ui-parity/vintage-back-hover-*`.
Debug bundle SHA256 `ee00bbbd32ba125ea840c158915162cf9e8f27dce64a5c3477461ad1d5b0e854`.


## Vintage — display-only font size

Settings font-size display-only (2026-11-16): replaced DragValue with an
opaque bordered, centered display using hover-only sense and Label semantics.
It cannot focus, edit or drag. Only +/- change/persist font size; 10–24 bounds
and shared font remain. UI test first reproduced Text cursor on the editable
control, then verifies default cursor/no focus, typing/arrows/drag leave 14
unchanged, centered displayed values, button file persistence and limits.
350 Rust regression tests pass, 0 failed/ignored; fmt/Clippy/build pass.
Production egui input/paint verified; no new native screenshot. Evidence
`docs/artifacts/ui-parity/vintage-font-display-*`.
Debug bundle SHA256 `01c549bd1d9d30c4eb3f7027c65ff565accb184e085efba7f99cc8582b6bb57a`.


## Vintage — shared display text

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
Debug bundle SHA256 `01010bcdf3144eb042549df93044003190dcb535a12fa3147caaed8515d9e760`.


## Vintage — shared kit and square stepper audit

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
Debug bundle SHA256 `c71a53f5ea1be2a75f788051d152b2121c227806fff5edb5a7f77ed8afaebf74`.


## Vintage — MCP copy and empty table consistency

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
Debug bundle SHA256 `6fcde950c7cbb11674787c7bb27128d078b81bf7d2c34be4fe9506d14662942c`.


## Vintage — JSON workspace parent padding

JSON workspace parent padding (2026-11-16): replaced asymmetric margin
(left/right/bottom20px, top4px) with8px on all sides around both panes.
Inner pane padding4px and splitter16px retained. Added output pane geometry
probe; test-first red reproduced wrong top margin. Updated UI test verifies
left/top/right/bottom bounds and pane alignment at700/1360px; existing
placeholder/input/format padding test still passes. 356 Rust regression tests
pass,0 failed/ignored; fmt/Clippy/build pass. Production egui paint/input tested;
no native screenshot. Evidence `docs/artifacts/ui-parity/vintage-json-outer-*`.
Debug bundle SHA256 `a07ff268b4894888d63ee58abe873d35ba94cf5c91eaec25b39f040e7173ec00`.


## Vintage — JSON/Tree guides and toolbar

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
Debug bundle SHA256 `99a7a3d41fe3b7900b399e16e65969d2ef2ff7799c9020f7ea82c842f6ad9df1`.


## Vintage — restore original Tree connector geometry

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
Debug bundle SHA256 `c7e6395210e8420cefbb13a6b2fdf71224048c92108ef4ffa47b2e26a2044eed`.


## Vintage — center JSON and Tree fold arrows

The shared viewer aligns the painted fold triangle with the visible row glyphs,
excluding search and selection backgrounds. The expanded and collapsed arrows
keep the original hit areas and Tree connector geometry. The UI test first
reproduced a 3.25px offset in JSON at 14px, then verified both modes, both arrow
directions and root/nested rows at 14px and 24px. All 359 Rust tests passed with
zero failures or skips. Format, Clippy and build passed. Production egui
paint and input were tested; no native screenshot was taken.
Evidence: `docs/artifacts/ui-parity/vintage-json-arrow-*`.
Debug bundle SHA256 `72ad79ceb0c8b7434238e680eee5ca199dcc744923b1386e79a9e847384fd697`.


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
Debug bundle SHA256 `e6687d9480933ff7a11db5d66f4ac276bd9ba7249a7d8792a01f63cfafc1dd98`.


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
Debug bundle SHA256 `3b2cb8897d6b13c77c58bc9638d364482a383d5cd2f221cc9339a655bd1b5bec`.


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
Debug bundle SHA256 `8aa45994e35ec95d8b6077d0f1c81a05af91e6b022a467c3eb9a3de9e1b2beb2`.


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
Debug bundle SHA256 `23c90f50c4abc5082ebabb7ef368e50b676ba61a4aa99ee5213b5e0245f37c31`.


## Vintage — control background #D9DED3

Replaced the shared control background #E6EBDF with #D9DED3 and updated
the gallery palette label and widget inventory. The production UI fill test
first failed against the previous color and now passes for search, buttons,
inputs and color preview backings. All 362 Rust tests passed with zero failures
or skips; format, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/vintage-d9ded3-*`.
Debug bundle SHA256 `b6f42fc5a309949b60655d96abf3035db27453ff151cda82b13b8761263f06e7`.


## Vintage — control background #E0E5D9

Replaced #D9DED3 with #E0E5D9 in the shared light control palette. The UI fill test failed before the change and passed after it. All 362 tests, format, Clippy and build passed. Evidence: `docs/artifacts/ui-parity/vintage-e0e5d9-*`.
Debug bundle SHA256 `3fd7bebbaf10220458d9c7ec65d09672216e0d91eeba48fa21bc8af6b0ae0209`.


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
Debug bundle SHA256 `a29a0f196434b9bf3729dd961cc85476bffe8becae12492d8db76547aa529141`.


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
Debug bundle SHA256 `b013df6ce97b8f4f459a1591ea52fdd231b323c62ae8dca6bfec4a318b8fba1e`.


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
Debug bundle SHA256 `8875e5c3cf3d89ec9f421d0c896361152299e0daf361e335be0b6365ed5f6e0f`.


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
Debug bundle SHA256 `e590eec587237940c0cd08fbc4537f6d9206449484334e222b01539eeeb0e73f`.


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
Debug bundle SHA256 `dc435cbf704f1c9b20fb84100b8ffde4eda7fe16c97e2353f747426cb062fb06`.


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
Debug bundle SHA256 `b5bf513f6ee9fb0374876615407775bffbf4b8e67b94246a3291d759304d8168`.


## Vintage — Find & connect label (2026-11-17)

Renamed the connection scan action to “Find & connect” (VI: “Tìm & kết nối”)
and updated Help and the widget gallery. Action dispatch and probe IDs stay intact.
The production header UI assertion failed against the old label, then passed.
All 367 Rust tests passed with zero failed/ignored; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/vintage-find-connect-*`.
Debug bundle SHA256 `af7c0d44cbbb44afc7aa2110d700c909696a2c4096e6b3e5d28f46cf0c7f04dc`.


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
Debug bundle SHA256 `605ef16ebdf39082b710aa1ae9e4989421a13afd5a67c0aae908b50761060da3`.


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
Debug bundle SHA256 `d67d3277b610d05d71958eee8abf806d4577f8aede5ee1f0326eb3f04f39ae20`.


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
Debug bundle SHA256 `67055faa3ecf65c9c28a72a4b5650463e869fc6d51c4e6d52aa64b2840a24dfa`.
Geist TTF SHA256 `73894e0448cae90a92b6c2f8732b7bb9acb7b94c418bff559dad4a18e1de9659`.


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
Debug bundle SHA256 `0a70c155b967d060f73725f0bfcc9216c3c4f1cdea2f3de5c4e5b4e2a4f2450a`.


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
Debug bundle SHA256 `7c9fa25254d65ce252d9b1648553813451e7ae3f3b867fbd6e6657551db38f46`.


## UI — split the Overview URL into four fields (2026-11-17)

Overview starts with Method, Domain (host and optional explicit port), Endpoint
(raw path), and Query params. Query colors, raw encoding and copy behavior remain.
The full URL above the table is unchanged. Added Vietnamese labels for the new fields.

The renderer test failed first on the combined URL row, then passed with the
new order, encoded paths and queries, IPv6 with port, relative paths, and
per-row clipboard output. The query syntax and wrapping regressions still
pass. All 373 Rust tests passed; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/overview-url-split-*`.
Debug bundle SHA256 `808db4a24307a781536c8899715ffccebfbafca0346d3b00932f1367f6341d25`.


## UI — shared JSON/Tree segmented selector (2026-11-17)

The workspace and inspector now use one reusable segmented selector. It paints
a single square blue outline and one divider, with separate clickable JSON and
Tree areas, 8 px horizontal label padding, and a blue selected segment.

The renderer test failed first because both former buttons owned borders. It
now checks the single outline, divider, selection in both directions and cursor
in the workspace and inspector across light and dark themes. All 373 tests
passed; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/json-tree-segmented-control-*`.
Debug bundle SHA256 `51815616610a9a798b4df4a2a0e07546f0fbb7cb69567ecc14400974e227e5e9`.


## UI — full-width inspector metadata tables (2026-11-17)

Overview, Request header and Response header render their tables directly in
the vertical viewport. Removed the extra card border and its 12 px padding;
the table retains its grid, outer border, cell padding and copy actions.

The renderer test failed first on the inset, then verified that all three
tables reach the viewport edges and paint only one enclosing border. Wrapping
and copying still pass. All 373 tests passed; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/full-width-metadata-table-*`.
Debug bundle SHA256 `38a791de074a76a2c1cc87782103b22b725a11b00e9f321ee49ba2cad3d9d60d`.


## UI — keep HTTP scheme in the Overview Domain row (2026-11-17)

Domain displays and copies the captured scheme, host and any explicit port.
Credentials are omitted and no scheme is added to scheme-relative URLs.
The UI test reproduced the missing scheme, then passed for HTTP, HTTPS, IPv6,
explicit ports, and relative URLs. All 373 tests passed; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/overview-domain-scheme-*`.
Debug bundle SHA256 `3656de036b2e6537f2a263125f56c339e430c77bc9190ae6ac972175b1c80600`.


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
Debug bundle SHA256 `6a097771febcc428bbbcfec2accdd87b5aca602d2463909ff2605f8689128b72`.


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
Debug bundle SHA256 `69c610b7bf06c768d604a66d13eb0d20d774f6126234429b2443da58d3aa79bb`.


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
Debug bundle SHA256 `9ec6442b4e2b1bc6a2d9c4d3d425562730e4add37a43e5333e317c1c153c4e95`.


## UI — tighten inspector JSON spacing (2026-11-17)

Removed the extra space around the inspector JSON toolbar. The scrollbar lane
ends 6 pt above the toolbar, and the first JSON or Tree row starts 12 pt below
it (6 pt layout gap and 6 pt content inset). The reserved scrollbar lane and
other inspector content spacing are unchanged.

The UI test first measured the old 18 pt gap above the toolbar, then verified
both new distances in JSON and Tree views. All 375 tests passed; fmt,
Clippy and build passed. Evidence: `docs/artifacts/ui-parity/inspector-json-spacing-*`.
Debug bundle SHA256 `178beb1a877037660efe3836a51a0126c53437b235ba519a0e2f601f47118f20`.


## UI — enlarge table header filter icons (2026-11-17)

Network and Console table header filters retain 24 × 24 pt button frames
and use centered 20 pt arrow icons. Other icon button sizes are unchanged.

UI tests verify the frame and glyph size, column title clearance, and working
filter menus. All 375 tests passed; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/table-filter-size-*`.
Debug bundle SHA256 `217252cac26cc1f1ece62ffbe4cf52ddcafd1b604d74e69b07b9f40d40627a0a`.


## UI — join Console table header and rows (2026-11-17)

Removed the 6 pt gap below the Console table header. Rows and filtered empty
results meet the header edge. Scrolling row paint stays below the fixed header.

The UI test reproduced the gap before the fix and now checks contiguous rows,
the shared header/body edge, scroll clipping and the filtered empty state.
All 376 tests passed; fmt, Clippy and build passed. Evidence: `docs/artifacts/ui-parity/console-table-seam-*`.
Debug bundle SHA256 `3895a8e71bc8c62fe54b8ad71d1d1474a8417263fa44e9764b1015c1bfde2905`.


## UI — wrap complete Console messages (2026-11-17)

The Console Message column wraps complete log text within the cell and
preserves explicit line breaks. Each row grows as needed. Cached row heights
keep the scroll layout accurate while only visible rows are painted.

The input-driven UI test reproduced the truncated message and now verifies
long unbroken tokens, a final Unicode line, contiguous variable-height rows,
and reflow at a narrower width. All 377 tests passed; fmt, Clippy and build
passed. Evidence: `docs/artifacts/ui-parity/console-message-wrap-*`.
Debug bundle SHA256 `4f3f7db42b5441bf7289fbdac6f0b1058be9be37a975b1358d2d45de0df4e46b`.


## UI — share the Console and Network inspector components (2026-11-17)

The inspectors share their frame, heading row, primary tabs and tab spacing.
Console Detail presents structured log fields and expandable JSON messages;
Raw renders the original record and is still serialized only on demand.
Both use the same JSON/Tree viewer, search and clipboard controls as Network.
The shared viewer wraps its toolbar when panel width is limited.

UI tests cover geometry and control access in wide, narrow and modal layouts,
JSON/Tree and clipboard behavior, and independent search state. All 378 tests
passed; fmt, Clippy and build passed. Evidence: `docs/artifacts/ui-parity/shared-inspector-*`.
Debug bundle SHA256 `e5745cc8b100946d76786aa8f3a3b05367cd735cd762ee85ae65e3279af07e13`.


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
Debug bundle SHA256 `5dfca6b7e0596d3a64db0a0986b2818daee4b2ac3d87092bfb85cdff51546e40`.


## UI — Resize Console columns (2026-11-17)

Drag any of the three header dividers to resize its adjacent columns. The header
and rows share the resulting edges; Message wraps to its new width. Minimum
widths keep the filters and Message usable when shrinking the list.

The input-driven UI test first failed on the missing drag handle. It now checks
the resize cursor, every divider, adjacent widths, Message reflow and the Source
filter after resizing. All 379 regression tests and 13 focused Console UI tests
passed; fmt, Clippy and build passed. Evidence: `docs/artifacts/ui-parity/console-column-resize-*`.
Debug bundle SHA256 `9aaa4659acddd2cda7a614a2fa06b34d3716e4647b1d94c71d6ec9ecb680db5e`.


## UI — Network filter ALL labels (2026-11-17)

The Method and Status dropdowns display ALL for their unfiltered option in both
English and Vietnamese. Existing filter values and behavior are unchanged.

The UI test first failed on the old label, then passed for both languages and
both filters. Of 380 regression tests, one socket test timed out on the initial
run and passed when its suite was rerun. Fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/network-filter-all-*`.
Debug bundle SHA256 `458d87b2631bb3aa8d4e885cc12adbaf4b093562f4bb6bcef647cb23a8bcc237`.


## UI — Console Message tag filter (2026-11-17)

The Message header has a fixed-size popup with a gray input/Plus header
and a scrollable list of removable tag cards. Tags match case-insensitive
substrings against the displayed Message only. A match against any tag is
required when tags are present, alongside the other Console filters.

The input-driven UI test reproduced the missing control before the change and
now covers adding by click/Enter, duplicates, matching, deletion, filter reset
and scrolling a full tag list without changing the popup size. All 381 tests
passed; fmt, Clippy and build passed. Evidence: `docs/artifacts/ui-parity/console-message-tags-*`.
Debug bundle SHA256 `dbd39819eab5883dff528f29d84edd8b26e801abe19348503770f1b68e26bd47`.


## UI — Correct Console tag icons (2026-11-17)

Use the bundled font's Add and Close glyphs for tag actions. The font lookup
confirmed the old codes were Cast connected and Snowboarding. The UI test
first reproduced the wrong icon and now checks both centered 14 pt glyphs in
24 × 24 pt buttons. All 381 tests passed; fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/console-tag-icons-*`.
Debug bundle SHA256 `0575d03cbd820103bebfec5d15404285beeff3b6bf3a17f2a267645c9a52e28c`.


## UI — Wrap Console tag cards horizontally (2026-11-17)

Console tag cards have a gray border and size to the label and close button,
up to the width of the popup. Cards flow from the left and wrap with 8 pt
horizontal and vertical gaps. The popup stays fixed while its list scrolls.

The UI test covers shrink-to-fit widths, wrap alignment, gaps, gray borders,
and deleting an item after scrolling. It reproduced the missing card layout
before the change. Of 381 regression tests, one socket test timed out on the
initial run and passed when its suite was rerun. Fmt, Clippy and build passed.
Evidence: `docs/artifacts/ui-parity/console-tags-wrap-*`.
Debug bundle SHA256 `9d04ce281e6a48150317af35fc806d6e0cd185dea6578de51b8615b88787b649`.

## Repository cutover — Flutter to Rust

The Rust source replaces the Flutter application tree in a single commit on
main, preserving the Flutter project's Git history. The Dart VM integration
fixture remains under tests. Build outputs and local evidence stay on disk.

Verified from the staged tree: 381 Cargo tests, rustfmt, Clippy, vendored
updater checksums, and 11 packaging, signing, and integrity tests passed.

## Rust 0.1.9 (build 10) and GitHub Actions

Advance past Flutter 0.1.8 (build 9). Cargo, MCP, Windows resources, and
release packaging now agree on the version. The Rust workflow also runs for
pushes to main and names package artifacts using the validated release version.
Sparkle symlink checks run on a compatible host, and Git keeps vendored updater
files byte-for-byte for checksum verification.

Verified locally: 381 Rust tests, 9 release-tool tests, formatting, Clippy,
workflow YAML/triggers, and both macOS preview packages (version/build,
architecture, layout, and SHA-256). The Windows installer and hosted workflow
have not been run for this change. No release or feed was published.

## 2026-11-17 — Padding Manual URL và tên app

- `manual_url_modal_insets_the_header_input_and_find_button` phát hiện mép
  trong chỉ 1 pt trước khi sửa; kiểm tra đủ bốn mép sau khi đặt padding 12 pt
  ở hai kích thước cửa sổ (480 và 1360 pt).
- Tên hiển thị: Flutter Network Viewer. Kiểm tra layout bundle macOS và
  metadata/installer Windows qua `tools/test_f21_release.py`.
- macOS: 382 test Rust và 9 test đóng gói pass; fmt và clippy pass.
- Chụp lại slide 2 từ demo Rust, xuất lại GIF kết nối (3 frame), xác minh
  SHA-256 của 10 GIF còn lại không đổi.

## 2026-11-17 — Repair CI native smoke and socket fixtures (F21)

Screenshots from run 44038134966 showed the macOS native smoke using the
removed `connect` probe and Windows failing the initial socket bootstrap.
The macOS failure was reproduced before updating the test to `manual-url`
and `json-copy`, retaining native screenshot delivery during injected input.

A delayed HTTP-upgrade regression failed before the fixture fix. Accepted
sockets can inherit nonblocking mode from the listener (BSD/macOS and Windows);
the fixture now explicitly uses blocking mode for its blocking handshake.
The scheduling budget is 15 s; the timeout/reconnect test uses a 2 s RPC
budget rather than 250 ms. Its held RPC still times out, and assertions still
require the original transport, preserved Clear cursor, and one reconnect.
A separate 3.2 s startup test checks capture without advancing logical time.

Remaining workflow smoke tests were exercised rather than skipped:
- F17: assert the current Discussion action, replacing its removed copy action;
  do not require a connection toast to remain mounted until captures arrive.
- F19: explicitly connect its fresh VM; a saved URL no longer starts capture.
- F20: verify the localized result notification, then scroll the narrow Settings
  viewport to the updates action. Run injected input at 16 ms intervals.
- MCP fragmented-body test: declare 41 bytes for its 40-byte body, so the
  request actually remains pending until Stop instead of sometimes responding.

Validation on macOS ARM64: 384 Rust tests pass with default parallel execution;
fmt and all-target Clippy pass; 9 release, 2 updater integrity, 1 signing-helper
Python tests and vendor checks pass. Individual native F21, multi-app, F17,
F19, and Sparkle current/available checks pass. One earlier Sparkle run timed
out after its current-result assertions passed; a fresh full rerun succeeded.
Evidence: `docs/artifacts/ci-regression-2026-11-17`.
Windows hosted CI and Intel native execution must still validate this change;
no hosted run was triggered and no release was published.
Final verification also ran the complete native workflow sequence in order
(F21 → multi-app → F17 → F19 → F20 current/available), exit code 0.

## 2026-11-17 — Windows release linker shadowed by Git Bash (F21)

The next hosted run reached Package Rust preview and selected
`C:\Program Files\Git\usr\bin\link.exe`, which reports `/usr/bin/link:
extra operand`. Earlier Windows test/build steps succeeded under the runner's
native shell; the package step alone forced Git Bash and changed PATH order.

Removed that shell override. The runner now uses PowerShell on Windows and
Bash on macOS. `release.py package --github-output` appends validated UTF-8
metadata to GITHUB_OUTPUT only after packaging succeeds, preserving the
existing step ID/outputs consumed by the installer and artifact upload.
Windows packaging verifies that PATH resolves link.exe inside VCToolsInstallDir
before starting Cargo, so a shadowed or missing toolchain reports its cause.

Test-first: output/preflight tests failed before implementation. 13 release
Python tests now pass, covering output append, failure without output publish,
accepting MSVC/rejecting Git or missing linker, and preflight before Cargo.
The workflow contract rejects a Bash override on the package step. YAML parse,
384 Rust regression tests, fmt and all-target Clippy pass. A real macOS ARM64
release ZIP and SHA-256 check pass; actual GITHUB_OUTPUT matches UTF-8 metadata.
Evidence: `docs/artifacts/ci-msvc-linker-2026-11-17`.

Windows native linking/installer and the hosted workflow require another run;
this macOS host cannot establish a Windows build pass. No commit or push yet.

## CI Sparkle blocking modal regression — 2026-11-17

Run 44042285948 failed on macOS ARM64 after F20 printed PASS: a manual
no-update check opened Sparkle's blocking NSAlert and the child exceeded its
70 s timeout. A process sample confirmed NSAlert runModal. A native UI assertion
reproduced the modal before the fix. Availability checks now use Sparkle's
information API and the app notification; a known update still opens the native
install UI. Teardown dismisses that UI.

Native acceptance now rejects blocking modals, clicks the available-update
action, requires normal process exit, and writes fresh PASS JSON only after the
event loop returns. Current/available flows and two further consecutive runs
passed without requesting an installer archive. 384 Rust tests, fmt, all-target
Clippy, 13 release tests, 2 updater integrity tests and workflow YAML parse pass.
Evidence: `docs/artifacts/ci-sparkle-modal-2026-11-17`.

Workflow native stages are individually named, continue collecting diagnostics
after earlier failures, and upload failure evidence. Rust tests enable backtraces
and no-fail-fast. Windows in the same hosted run failed at cargo test, before
packaging; its detailed failure log is unavailable in this session. The supplied
log covers macOS only. Windows remains undiagnosed; no hosted rerun, commit,
push or release was performed for this fix.

## F21 — GitHub draft release after all builds (2026-11-17)

Added a release job depending on the complete desktop matrix. Only main pushes
and manual runs on main can create/update a draft; pull requests keep read-only
build permissions. Downloaded artifacts remain separated by platform so their
checksum manifests cannot overwrite each other. All three ZIPs and the Windows
installer must pass checksum verification before contacting GitHub. The draft
includes those packages, platform status files, combined SHA256SUMS and notes.

The tag/title/notes use Cargo and packaging metadata (currently rust-v0.1.9).
A rerun updates the existing draft and its assets; published versions are refused.
The job has scoped contents-write permission and serialized release operations.
Nothing publishes a release automatically.

Test-first import failure reproduced before implementation. Eight executable
orchestration tests pass using inert packages and mocked GitHub CLI, covering
new/existing drafts, pagination, API failure, published-version refusal and
missing/tampered/incompletely checksummed packages. All 13 packaging tests,
384 Rust tests, fmt, all-target Clippy and workflow YAML parsing pass locally.
Evidence: `docs/artifacts/ci-draft-release-2026-11-17`.
No GitHub mutation or hosted run was performed; hosted acceptance is pending.

## F21 — User-facing draft release notes (2026-11-17)

Compared the Flutter github_release_info.md template. Draft notes now contain
Có gì mới, Bản tải and Nếu macOS chặn ứng dụng, with all four Rust package
names generated from the current version. Installation steps use the Rust
Windows executable name; macOS guidance includes quarantine removal and opening
the app, scoped to trusted official downloads. Changelog/title remain maintained
in packaging/release.json; changes are not inferred automatically from commits.
Updated the current changelog for the Rust migration and Sparkle fix.

Test-first missing sections/dynamic notes failures reproduced. Nine draft tests
and all 13 packaging regressions pass, including a different version/changelog
and UTF-8 notes. Preview and logs: docs/artifacts/ci-release-notes-2026-11-17.
No hosted release was created or published.

## F19 — Pause input on slow CI frames (2026-11-17)

The hosted native smoke failed at the paused assertion. Its synthetic mouse
press/release were in separate frames; egui rejects a click held beyond 0.8 s.
A test against the production DesktopUi with 1.2 s frame intervals reproduced
the missing Pause action. A native control run with the old input and 1.2 s
injected stalls reproduced the exact app.capture.store().paused assertion.

F19 now batches press/release in one frame after a hover frame. It still uses
real production UI input and retains all pause, resume, capture, diagnostics,
memory and shutdown assertions. No click deadline was relaxed and no action
was dispatched directly to bypass the UI. A --slow-input stress mode is
available through tools/f19_native_smoke.py. CI runs the new portable UI
regression explicitly on every desktop platform.

Validation: new slow-frame UI test pass; native slow-input acceptance pass
before and after the old-input control; two additional normal native runs pass.
384 existing Rust tests, fmt, all-target Clippy, 9 draft-release and 13 packaging
tests pass. Workflow YAML and the new CI step validated.
Evidence: docs/artifacts/ci-f19-slow-click-2026-11-17.
Local native verification is macOS ARM64; hosted acceptance remains pending.
No commit, push or hosted workflow rerun was performed for this fix.

## F21 — GitHub release title

Release titles now follow `Flutter Network Viewer v<version>` from Cargo for
both new and refreshed drafts. Removed the preview label and outdated Windows
CI caveat from the release description. Version and tag are unchanged.
The old title failed both new assertions; 9 draft and 13 packaging tests pass.
The remote draft has not been changed.

## F21 — Flutter Network Viewer v1.0.1

Raised Cargo and Windows product version to 1.0.1 and the macOS/Windows build
number to 11. The Draft Release title is Flutter Network Viewer v1.0.1, its tag
is rust-v1.0.1, and the changelog leads with the migration from the Flutter
desktop implementation to Rust. No release was created or published locally.

Version/build regression failed on the old metadata, then passed. All 22
release and packaging tests, 385 Rust tests (including F19 slow-input UI),
fmt and Clippy passed. Verified all four package filenames use 1.0.1.

## F21 — Legacy Flutter appcast compatibility

Flutter 0.1.8 requests `appcast-<platform>.xml` from the latest public release.
The published `rust-v1.0.1` has no such assets. Both embedded public keys match
between Flutter and Rust. Ported the old CI signing wiring using the existing
platform secrets and the Rust signer. Release builds sign both macOS ZIPs and
the Windows installer; the draft job requires the three feeds, validates each
against its package, and includes them in the combined checksums and draft.
Pull requests do not access the signing secrets. Published releases are not
modified by the workflow.

The tests cover missing keys, checked-in workflow wiring, required appcasts,
wrong platform appcasts with valid checksums, signatures on inert test data,
and the published release guard. All 24 Python release/packaging tests, the
real macOS signer test, 385 Rust tests, fmt, Clippy and workflow YAML pass.
Evidence: `docs/artifacts/ci-legacy-update-feed-2026-11-17`.
Production signing and in-place Flutter-to-Rust installation remain pending.
Rust uses distinct bundle/installer IDs and a separate Rust update channel.
No commit, push, version bump or hosted release mutation was performed.

## F21 — Flutter Network Viewer v1.0.2

Bumped Cargo and Windows product version to 1.0.2 and native build to 12.
The workflow targets a new draft `rust-v1.0.2` with the legacy appcasts and
leaves published `rust-v1.0.1` unchanged. The version/build gate failed before
metadata was bumped. 24 release/packaging tests, 385 Rust tests, fmt and Clippy
passed. Evidence: `docs/artifacts/ci-v1.0.2`. Hosted signing and the installed
Flutter-to-Rust upgrade still require validation.

## F21 — macOS bundle validation for v1.0.3

Compared the published 0.1.8 and 1.0.2 Apple Silicon bundles. The 1.0.2 ZIP
matched the appcast length and Ed25519 signature using the 0.1.8 public key.
The old app passed strict codesign validation; the 1.0.2 app failed with
`code has no resources but signature indicates they must be present` because it
only had the Mach-O linker signature. Signing its outer bundle ad hoc while
preserving the vendor Sparkle signatures made strict validation pass.

macOS packaging now seals the bundle before zipping and requires strict deep
verification. Appcast descriptions use escaped HTML that can wrap. Bumped to
1.0.3/build 13; no existing GitHub release changed. The package regression
reproduced the same codesign failure before the fix. A real 1.0.3 ARM64 ZIP was
built, extracted and verified. All 25 release/packaging tests, macOS signer test,
385 Rust tests, fmt and Clippy passed. Evidence: `docs/artifacts/ci-v1.0.3-macos-signing`.
The full upgrade from the installed 0.1.8 app using a production-signed 1.0.3
feed, and hosted Intel validation, are still pending.

## F11 — Windows cancellation fixture startup deadline (2026-11-18)

The supplied Windows log failed before cancellation: ready_rx expired at 2 s
and the fixture's initial HTTP read timed out (10060). Separate the 15 s setup
budget from the unchanged 2 s peer-closure budget starting after Stop/Drop.
Keep the peer fallback timeout long enough for owner scheduling; a read timeout
is now rejected rather than counted as successful cancellation. Windows EOF
and connection-reset/closed errors are accepted.

A new production-Coordinator socket regression delays readiness by 3.2 s for
both pending handshake and pending subscription cases. It failed with the old
2 s readiness limit, then passed. Six socket tests pass on the final source;
386 Rust tests (including F19 example test), fmt and all-target Clippy pass.
Only the test fixture changed; runtime timeout and cancellation behavior are
unchanged. Restored the deleted temporary Rust 1.100.0 toolchain and locked
dependencies to execute verification. Local execution is macOS ARM64; Windows
hosted verification remains pending. No version bump, commit or push performed.
Evidence: `docs/artifacts/ci-cancellation-startup-2026-11-18`.

## UI — Shared macOS-style switch (2026-11-18)

Replaced the square toggle with one cross-platform vector widget: 44 x 22 pt
capsule track, 26 x 18 pt white thumb, blue on/gray off, a 120 ms slide, keyboard
focus outline and disabled appearance. Colors are in app_colors.rs. Gallery
and Settings use the same widget; it avoids the square action backing and
keeps a fixed size on hover. MCP header row height now fits the smaller switch
while keeping equal section padding. Other controls keep their square style.

The gallery geometry assertion reproduced the old switch before implementation.
UI tests cover production MCP off/on/off in both themes, gallery geometry and
hover layout, keyboard activation, disabled input and 1x/2x DPI. Existing MCP
save/restart, section padding and narrow UI tests pass. 388 Rust tests including
F19 example, fmt and Clippy pass. Native F17 smoke passes; the actual dark
Settings screenshot was inspected. Native execution was macOS ARM64; Windows
execution remains pending, while its rendering path is the same shared code.
Evidence: `docs/artifacts/ui-macos-switch-2026-11-18`.
No version bump, commit, push or release mutation performed.

## Settings dropdown kiểu macOS — 2026-11-18

Đổi Theme, Language và Font size sang `vintage::dropdown_row` dùng chung với
widget gallery: title bên trái, spacer, selected option sát phải, gap 8 pt và
hai chevron vẽ vector. Chỉ vùng selected option + arrow nhận click/hover;
hover tô nền, không scale hay thay đổi kích thước. Popup không padding ngoài,
option giữ padding và pointer. Font size là dropdown 10–22, mặc định 14;
chọn lưu ngay và dùng cùng cỡ chữ toàn app. Giữ Geist và Cascadia cho JSON/Tree.

TDD: test row/probe thất bại trước implementation (red.log). Bản cuối có
386 test Rust + 1 regression F19 pass, 0 failed/ignored; fmt và Clippy pass.
Native F17 pass trên macOS ARM64; đã xem ảnh Settings thực. Chưa chạy Windows.
Bằng chứng: `docs/artifacts/ui-macos-dropdown-2026-11-18`.
Build macOS ARM64 1.0.3 tại `dist/dropdown-macos-20261118`, ký ad-hoc thành công;
chưa commit/push/publish. Không mở chat port mới vì Windows acceptance còn pending.

## Dropdown hover contrast — 2026-11-18

Fixed invisible light hover: the old `control` token was white like the section
surface. Dedicated shared tokens now use #E7E7E7 (sampled from the supplied
macOS screenshot) and #454545 for dark mode. Only the trailing selected value
and arrows highlight; layout and pointer stay unchanged. The idle arrow circle
uses the same visible neutral color. UI regression first reproduced white hover,
then verifies exact colors, light/dark contrast, opaque fill, stable geometry,
hand cursor and no title hover highlight.

387 Rust regression tests + 1 F19 example test pass; fmt, Clippy and native F17
pass on macOS ARM64. Actual dark Settings screenshot inspected. Windows native
verification remains pending. Evidence: `docs/artifacts/ui-dropdown-hover-2026-11-18`.
Built signed ad-hoc app: `dist/dropdown-hover-macos-20261118` (1.0.3).
No commit/push/publish; no next port chat while hosted acceptance is pending.

## Settings sidebar — 2026-11-18

Settings now uses a full-height left navigation pane (350pt at desktop widths)
and a remaining-width content pane with 8pt outer padding and white light-mode
surface; dark mode keeps its matching dark surface. Root layout has no inset.
Menu rows use icon + 4pt gap + localized title, pointer cursor and blue selected
state: Interface, MCP, About us (renamed App updates). Only the selected section
is mounted. Existing controls, persistence, MCP lifecycle and update actions stay
in their respective sections. The sidebar compacts only for very narrow windows
to keep controls accessible. Narrow MCP copy actions stack rather than collapsing
into columns of single characters.

TDD: sidebar test failed before implementation. A second geometry test reproduced
the narrow copy-button problem; both regressions now pass. Updated existing UI
checks and native acceptance navigation for individually selected sections.
388 Rust tests + 1 F19 example test pass, no failures/ignored; fmt and Clippy pass.
Native F17 and F18 pass on macOS ARM64, including all three sidebar pages,
screenshots, narrow scrolling, OS clipboard, MCP restart/retry and persistence.
Inspected actual light Interface/MCP/About screenshots. Windows execution pending.
Evidence: `docs/artifacts/ui-settings-sidebar-2026-11-18`.
Final macOS ARM64 1.0.3 bundle: `dist/settings-sidebar-final-20261118`, ad-hoc signed.
No commit/push/publish. No next port chat while Windows acceptance remains pending.

## macOS dropdown popup — 2026-11-18

Replaced the Settings/gallery dropdown menu with a rounded neutral popup,
6pt outer inset, compact rows, blue rounded selection and a vector checkmark.
The selected row's center aligns vertically with the displayed value; popup
right edge is 20pt beyond the displayed text's right edge. Popup placement is
computed from selected index, font metrics and screen bounds. Long lists use
an initial scroll offset at the top screen boundary to retain selected-row
alignment; users can scroll to choose other sizes. Theme, language, font size
share this widget; table filter menus keep their existing compact list style.

TDD: geometry/tick test failed before implementation. Now covers light/dark,
10/14/22pt, selected-row alignment, exact 20pt offset, rounded blue fill, tick,
screen bounds and Escape. Persistence test scrolls from 22 back to 10 using
pointer/wheel events. Updated gallery inset expectations for the new reference.
389 Rust tests +1 F19 example regression pass, 0 failed/ignored; fmt and Clippy
pass. Native F17 measures the popup and captures both themes after opening
animation; screenshots inspected on macOS ARM64. Windows native pending.
Evidence: `docs/artifacts/ui-macos-popup-2026-11-18`.
Built signed ad-hoc macOS ARM64 app in `dist/settings-popup-macos-20261118`.
No version bump, commit, push or release publication. No next port chat while
hosted acceptance remains pending.

## Settings group surfaces and sidebar icon tiles — 2026-11-18

Removed redundant Interface/MCP/About section title rows from right content.
Kept the MCP status and switch. Shared group fill is #F8F8F8 sampled from the
supplied reference; dark groups use #303030. Existing border geometry remains;
no row dividers added. Each sidebar category has a distinct tile/ink pair
(blue, green, gray tiles), square with 4pt corner radius. Icon is centered at
90% tile size, with 4pt tile-to-label gap. No changes to global text size.

TDD: title-absence/tile test failed before implementation. UI checks inspect
actual fill, radius, centered glyph font size, 90% ratio and distinct colors.
Updated previous right-heading icon tests to check the sidebar tiles and kept
field/action, spacing and section-padding coverage; transparent MCP status-row
bounds are included when measuring layout padding after removing its title.
390 Rust tests +1 F19 example regression pass, 0 failed/ignored; fmt/Clippy pass.
Native F17 pass and actual Interface/MCP/About screenshots inspected on macOS
ARM64. Windows native verification pending. Evidence:
`docs/artifacts/ui-settings-tiles-2026-11-18`.
Final signed ad-hoc build: `dist/settings-tiles-macos-20261118` (1.0.3).
No commit, push or publication; no new port chat while acceptance is pending.

## Settings icon ratio, rounded groups and contextual buttons — 2026-11-18

Sidebar glyph ratio is now 80% of its centered square tile. Settings groups use
4pt corners. About content places localized “Current version {version} - Author
Nguyễn Quốc An” on the left, with the update action vertically centered at the
right in the same row; VI is “Phiên bản hiện tại 1.0.3 - Tác giả Nguyễn Quốc An”.
Text wraps inside its reserved width in narrow windows. Status/progress and
update behavior remain covered by existing tests.

Shared neutral-button styling inherits the containing UI's fill: Settings header
on white uses #F0F0F0; controls in gray groups use #E7E7E7. Dark equivalents are
#3A3A3A/#505050. Settings buttons, MCP copy actions and square copy icons use the
same inherited token. Input backgrounds and primary selections retain their
own colors. Existing outer strokes remain; no dividers added.

TDD: About-row probe test failed before implementation; final assertions check
VI text, shared vertical center, left/right placement, group radius, 80% icon
geometry and contrasting enabled button fills in both themes. 391 Rust tests
+1 F19 example regression pass, no failed/ignored; fmt and Clippy pass.
Native F17 passes on macOS ARM64; actual About and MCP screenshots inspected.
Windows native verification remains pending. Evidence:
`docs/artifacts/ui-settings-controls-2026-11-18`.
Signed ad-hoc macOS build: `dist/settings-controls-macos-20261118` (1.0.3).
No commit/push/publish; no next port chat while hosted acceptance is pending.

## Button corners — 2026-11-18

Icon, icon/text and text buttons share 4pt corners for fills and outlines.
Segmented actions use the radius on the outside edges and retain their divider.
Button and glyph dimensions stay unchanged. UI tests first reproduced square
Settings buttons, then verified 4pt painted corners in production and gallery,
including hover and disabled states. 174 renderer/UI tests plus fmt and Clippy
pass. Evidence: `docs/artifacts/ui-button-radius-2026-11-18`. Full socket
regression, native smoke and packaging remain pending: automatic approval review
could not run because its usage limit was reached.

## JSON Format button outline — 2026-11-18

Removed only the Format outline; the blue fill and 4pt corners remain. The
existing JSON toolbar test reproduced the outline first and now verifies the
painted button at rest and on hover, along with format, find, copy and export.
393 Rust tests, fmt and Clippy pass. Evidence: `docs/artifacts/ui-format-border-2026-11-18`. Verified
the macOS ARM64 package signature and checksum; refreshed `dist/latest`.

## Shared search bar — 2026-11-18

Network, Console, JSON workspace/inspectors and widget gallery use one
`desktop_style::search_bar` component. The outer field has a gray idle border,
white light-theme fill, 4pt padding, a search icon matching the input height,
4pt icon/text gap and no TextEdit padding. Clear is overlaid at right 4pt and
vertically centered; text reserves its space. Each caller owns the width.
Dark mode retains its contrasting control fill and focus keeps the blue border.
Capture reset retains its existing filter-reset behavior.

TDD reproduced old 8/3pt padding before implementation. UI regressions exercise
10/14/22pt geometry, typing, clear, focus borders, Network/Console/JSON workspace
and inspector placement; the gallery uses the same component. Toolbar height
fits the field, avoiding clipped borders. 394 Rust tests pass, no failures
or ignored tests; fmt and Clippy pass. Verified macOS ARM64 checksum/signature
and replaced `dist/latest`. Windows native verification remains pending.
Evidence: `docs/artifacts/ui-shared-search-2026-11-18`.

### Search bar corners and clear visibility — 2026-11-18

The shared search bar uses a 4pt radius for both fill and border, including
focus. The clear icon and its hit target only exist when the input has text.
The capture X still clears active filters when used. Regression
`shared_search_geometry_and_clear_work_on_capture_and_json_at_all_font_sizes`
failed on the old always-visible X and now verifies both states and the
painted radius. 394 Rust tests, fmt and Clippy pass. The verified macOS ARM64
package replaces `dist/latest`. Evidence: `docs/artifacts/ui-search-corners-2026-11-18`.

### JSON workspace search before Format — 2026-11-18

The viewer keeps the search text before a formatted document exists and
applies it when formatting succeeds. The UI regression
`json_workspace_keeps_search_typed_before_formatting` reproduced the first
character disappearing on redraw, then verified successive characters,
Clear, and matching on Format. 395 Rust tests, fmt and Clippy pass.
The verified macOS ARM64 package replaces `dist/latest`.
Evidence: `docs/artifacts/ui-json-query-before-format-2026-11-18`.

### Header toolbar right alignment — 2026-11-18

Both rows retain 8pt horizontal padding (vertical padding is 2pt on row 1
and 4pt on row 2). Compact connection controls and capture actions reserve
their actual widths. JSON Beautify aligns using the measured control width,
keeping horizontal scrolling available if the controls do not fit.
`header_rows_align_last_actions_to_the_same_eight_point_inset` reproduced
the offset and now checks Network, Console and JSON in English and Vietnamese
at multiple widths and font sizes. 396 Rust tests, fmt and Clippy pass.
Verified macOS ARM64 bundle replaces `dist/latest`. Evidence: `docs/artifacts/ui-header-align-2026-11-18`.

### F20 — native update smoke follows the About pane

GitHub run 44234020080: Windows passed, both macOS jobs failed at the native F20
step and draft release was skipped. The Intel log confirmed the
`support_scroll > 0` assertion; the same failure reproduced locally on ARM64.
About is now a separate pane. The smoke checks that its update button is fully
visible in the resized Vietnamese dark window, and the loopback fixture requires
three requests for malformed feed, retry and manual check. The real native
current and available paths both exit cleanly without an installer request.
macOS ARM64 F20, 396 Rust regression tests, fmt, Clippy and 3 updater
integrity/signing tests passed locally. Hosted rerun pending.
Evidence: `docs/artifacts/ci-f20-about-2026-11-18`.
