# Vintage widget lab và giao diện production

Bộ Vintage đã được duyệt và dùng mặc định trong app Rust. App khởi động vào
Network thật. Gallery vẫn có trong code và test để kiểm tra từng thành phần.
Code và Tree trong gallery dùng cùng renderer với JSON workspace và inspector.

## Phong cách từ hai ảnh tham chiếu

- Nền xanh ngà nhạt như giấy, mực xanh đen, đường kẻ mảnh; mật độ vừa,
  monospace và các nhãn nhỏ mang cảm giác thiết bị/biểu mẫu kỹ thuật.
- Theo yêu cầu mới, **radius = 0** cho card/button/input/tab/menu/dialog.
  Không shadow, gradient hoặc badge màu cho method/status.
- Nền `#EBF0E4`, primary/text `#282F26`, nền ô nhập và nút thường `#E0E5D9`, line `#899383`,
  secondary text `#687263`. Đây là bảng màu sáng đã dùng trong app.
- Cascadia Mono (Google Fonts, OFL), một cỡ chữ duy nhất: mặc định 14; field/nút −/+ vẫn dùng Settings fontSize
  10–24 và persist/reload của app. Icon giữ geometry riêng.
- JSON giữ **7 vai trò màu** của JsonSyntax: punctuation/text, key, string,
  number, boolean, collection accent, null/muted. Code dùng tokenizer hiện có;
  Tree cùng palette; tìm kiếm chỉ thêm nền highlight, không bớt role màu.
- Gallery dữ liệu mẫu, chia7 tab. Controls/selection/filter/fold/find/copy/
  stepper/modal/notice/zoom/split có thao tác demo. Không kết nối VM hay
  export file thật từ các nút mẫu; tác vụ thật chạy trong app production.

## Kiểm kê widget

Phạm vi rà soát: `desktop.rs`, `desktop_shell.rs`, `desktop_network.rs`,
`desktop_style.rs`, `desktop_support.rs`, `desktop_json.rs`, `desktop_body.rs`.
Các loại dữ liệu/biến thể dùng chung một primitive được ghi cùng một hàng.
Catalog executable `vintage::INVENTORY` ánh xạ35 nhóm tới widget probe thật;
UI test kiểm mỗi example có mặt trong gallery.

| Nhóm widget | Source hiện tại | Tab demo | Example ID |
|---|---|---|---|
| Primary / outline / quiet / disabled button | desktop_shell.rs; desktop_support.rs | Controls | `vintage-primary` |
| Icon action / tooltip / copy | desktop_style.rs | Controls | `vintage-copy` |
| Single-line input / focus / hint | desktop_support.rs | Controls | `vintage-input` |
| Search / clear / validation | desktop_shell.rs | Controls | `vintage-search` |
| Dropdown / menu selection | desktop_support.rs | Controls | `vintage-dropdown` |
| Numeric field / stepper / font size | desktop_support.rs | Controls | `vintage-font-value` |
| Toggle / checkbox / radio | desktop_support.rs | Controls | `vintage-switch` |
| Multiline / selectable text | desktop_json.rs | Controls | `vintage-multiline` |
| Page tabs | desktop_shell.rs | Navigation | `vintage-tab:Network` |
| Project selector / connection indicator | desktop_shell.rs | Navigation | `vintage-project:All` |
| Inspector tabs | desktop.rs | Navigation | `vintage-detail:Overview` |
| Toolbar / contextual actions | desktop_shell.rs; desktop_json.rs | Navigation | `vintage-pause` |
| Breadcrumb / back / external link | desktop_support.rs | Navigation | `vintage-link` |
| Network table / selection / grid | desktop_network.rs | Tables | `vintage-row:0` |
| Header filters / popup | desktop_network.rs | Tables | `vintage-table-filter` |
| Console source / level / time / message | desktop.rs | Tables | `vintage-console` |
| Metadata card / key-value rows | desktop.rs | Tables | `vintage-metadata` |
| Resizable split / inspector | desktop.rs | Tables | `vintage-splitter` |
| Settings section / field labels | desktop_support.rs | Controls | `vintage-settings` |
| JSON code / line numbers / syntax | desktop_json.rs | JSON | `vintage-json-code` |
| JSON Tree / fold / expand | desktop_json.rs | JSON | `vintage-json-fold` |
| JSON find / case / word / regex / navigation | desktop_json.rs | JSON | `vintage-json-find` |
| JSON editor / format / copy / download | desktop_json.rs | JSON | `vintage-json-copy` |
| Color swatch / value copy | desktop_body.rs | JSON | `vintage-swatch` |
| Raw / Form / Image / body modes | desktop_body.rs | Bodies | `vintage-body:Form` |
| Form fields / file metadata / omitted body | desktop_body.rs | Bodies | `vintage-form` |
| Image preview / fit / zoom / enlarged | desktop_body.rs | Bodies | `vintage-image-open` |
| Modal / manual connection / export | desktop.rs | Feedback | `vintage-dialog-open` |
| Confirmation / cancel / dismiss | desktop_support.rs | Feedback | `vintage-dialog-cancel` |
| Info / success / warning / error notice | desktop_support.rs | Feedback | `vintage-toast-open` |
| Help sections / link / copy | desktop_support.rs | Feedback | `vintage-help` |
| Loading / progress / retry | desktop_support.rs; desktop_body.rs | Feedback | `vintage-progress` |
| Empty / no results text | desktop_shell.rs | Feedback | `vintage-empty` |
| Footer / RAM / capture counters | desktop_shell.rs | Navigation | `vintage-footer` |
| Card / separator / scroll / responsive layout | desktop_support.rs; desktop.rs | Overview | `vintage-scroll` |

## Tổ chức code

`src/vintage.rs`: tokens + kit `card/button/icon/tabs/input/toggle`, các mẫu
composition, catalog và trạng thái gallery. Style chỉ áp dụng khi ở demo.
`desktop_support::apply` khôi phục style cũ khi quay về app.
`src/desktop_json.rs::vintage_syntax_job`: dùng tokenizer production và7 màu
thay thế, không thay đổi JsonSyntax của app cũ.
`DesktopUi::vintage_open` mặc địnhfalse giữ renderer/test cũ; native
`DesktopApp` bật true để tạm điều hướng startup theo yêu cầu.

## Điều kiện duyệt

Người dùng duyệt palette, nét kẻ, các trạng thái widget, khoảng cách, JSON
và bố cục ở demo. Chỉ sau phê duyệt mới chuyển từng màn hình thật sang kit.
Không tạo chat port kế tiếp trong bước duyệt bộ style này.


## Review refinement — 2026-11-16

- Selected option và chữ trên nền primary: `#EBF0E4`.
- Icon thông báo/dialog: tile vuông `#282F26`, glyph `#EBF0E4`.
- Input/search/multiline/numeric giữ outline `#282F26` cả khi focus.
- Card JSON/code/Tree: `#EBF0E4`; vẫn đủ7 vai trò màu cú pháp.
- Bảng không có gap giữa rows, có top/left/right/bottom perimeter.
- Thay circle spinner bằng `vintage::loading(ui,count)` có3–10 ô.
  Mỗi ô16px: border1px#282F26, nền#EBF0E4; ô active có inset gồm border1+
  padding2px và ô fill#282F26. Advance200ms, modulo count, request repaint.
  Feedback tab có mẫu3/5/10 ô; progress bar xác định phần trăm vẫn giữ riêng.

4 kiểm thử UI bổ sung kiểm màu thực sau fallback, focus border, popup icon,
background JSON, table geometry/perimeter và toàn bộ3–10 loader qua2 vòng.
324 regression pass, fmt/clippy/build pass. Chưa kiểm screenshot native lượt này.


## Metadata / Console table review — 2026-11-16

- `key_value_table`: Title / Value và Action tùy chọn; dùng cho Metadata
  (2 cột) và Form/file fields (3 cột với Copy). Value selectable, tooltip đầy đủ,
  copy raw value; hàng liền nhau, cột thẳng hàng, có viền ngoài và scroll ngang.
- Console: Source / Level / Time / Message; chọn hàng, header đứng yên và clip
  nội dung khi scroll; tooltip message đầy đủ. Dùng cùng paint_row với Network.
- Gallery giới hạn chiều rộng theo viewport thực, giữ cột Action trong cửa sổ.
- Mọi label/button/title giữ một font size từ Settings; JSON giữ7 vai trò màu.

2 UI tests mới: geometry2/3 cột, Copy bằng pointer/clipboard, Console4 header,
selected colors, perimeter, wheel/fixed header/clip.10 vintage tests và326 full
regression pass, fmt/clippy/build pass. Evidence `vintage-tables-*`.
Chỉ demo; chưa migrate app hiện tại, chưa kiểm screenshot native.


## Square icon buttons — 2026-11-16

`vintage::icon` dùng container34x34px, padding4px, radius0. Controls, toolbar
và Copy trong cột Action đều dùng helper này, không stretch theo cell. UI test
kiểm width=height tại cỡ chữ10/14/24 và nút Copy nằm trong cell.11 vintage UI
tests/327 regression pass; fmt/clippy/build pass. Evidence `vintage-icons-*`.


## Key/value/action alignment — 2026-11-16

Dòng content của `key_value_table`: Title căn trái8px, Value căn phải8px,
Action căn giữa cả hai trục; icon34x34. Áp dụng metadata2 cột và Form3 cột.
Giữ header, Copy/selection/tooltip, một cỡ chữ. Test paint/input kiểm font14/24;
12 vintage UI tests/328 regression pass; fmt/clippy/build pass. Evidence
`vintage-alignment-*`. Chưa migrate app cũ, chưa kiểm native screenshot.


## Hover action scale — 2026-11-16

`hover_action` phóng phần vẽ1.2 quanh tâm khi enabled+hover, gồm font mesh,
glyph, background và border; mở rộng vùng click theo phần vẽ, không reflow.
Áp dụng button/quiet/icon/tab/toggle, Copy, dropdown+option, checkbox/radio,
numeric font field, links, swatch và JSON find toggles. Disabled không phóng.
Một base font size trong Settings giữ nguyên; icon34x34 thành40.8x40.8 trên hover.
2 UI tests đo paint/font, click viền ngoài, dropdown selection, restore và
font14/24.14 vintage UI tests/330 regression pass; fmt/clippy/build pass.
Evidence `vintage-hover-*`; chưa migrate app/chưa kiểm native screenshot.


## Hover bằng thông số thật — correction 2026-11-16

Thay thế hoàn toàn cơ chế paint/mesh transform ở phần Hover action scale trên.
`hover_action(ui, callback(ui,factor))` cấp rect và font sizes lớn1.2 trước render;
font14→16.8, icon18→21.6, nút icon34→40.8. Không zoom phần đã render. Font base
Settings vẫn duy nhất; popup option reset base style trước áp dụng hover riêng.
Vùng bấm là rect mới thật. Icon cột Action vẫn giữ tâm, vuông ở cả normal/hover.
15 vintage UI tests/331 regression pass; font LayoutJob và fresh galley rows,
native DPI1/2, font14/24, click và narrow wrap được kiểm. fmt/clippy/build pass.
Evidence `vintage-hover-sharp-*`; chưa kiểm screenshot native/chưa migrate app.


## Hover tại chỗ — 2026-11-16

Layout luôn giữ slot normal, gồm vị trí offset; hover control render mới ở
width/height/font1.2 quanh tâm slot. Parent không lấy kích thước hover để bố trí
widget khác. Child alignment top_down/Min giữ dropdown tại chỗ; font vẫn raster
ở cỡ mới, icon vuông, hit area theo rect hover. Test font14/24 so sánh chính xác
mọi probe khác trước/sau hover tab/button/icon/dropdown/Action, kiểm tâm và restore.
16 vintage UI tests/332 regression pass, fmt/clippy/build pass. Evidence
`vintage-hover-fixed-slot-*`. Chưa migrate app/chưa kiểm native screenshot.


## Opaque control surfaces — 2026-11-16

Mọi action control có backing SURFACE alpha255 phủ full rect, cả quiet/link,
checkbox/radio và disabled. Button/weak fill dùng nền đặc; primary/selection/
swatch giữ màu riêng. Hover paint ở sublayer trên siblings để không lộ chữ vẽ
sau phía dưới; giữ clip, hit testing, font thực, slot cố định và7 màu JSON.
2 UI tests kiểm opaque normal/hover và shape order backing/neighbor/own text.
18 vintage UI tests/334 regression pass; fmt/clippy/build pass. Evidence
`vintage-opaque-*`. Chưa migrate app/chưa kiểm native screenshot.


## Hệ số hover hiện tại — 2026-11-16

Theo review mới, dùng1.1 thay1.2 trong các mục lịch sử phía trên. Font14→15.4,
glyph18→19.8, icon34→37.4. Giữ native font rendering, normal slot cố định,
center và nền đặc/overlay.18 UI tests/334 regression pass; fmt/clippy/build pass.
Evidence `vintage-hover-110-*`; chưa migrate app/chưa kiểm native screenshot.


## Dropdown không scale — 2026-11-16

Dropdown Theme/Network và toàn bộ options dùng fixed_action: hover giữ nguyên
width, height, font size. Giữ nền đặc/paint overlay và selected colors; control
khác vẫn hover1.1 tại chỗ. Test font14/24 kiểm rect equality, font và chọn option.
19 UI tests/335 regression pass; fmt/clippy/build pass. Evidence
`vintage-dropdown-no-scale-*`. Chưa migrate app/chưa kiểm native screenshot.


## Pointer tương tác — 2026-11-16

Enabled click controls (button/icon/tab/toggle/checkbox/radio/dropdown/option/
link/swatch) và selectable Network/Console rows dùng PointingHand khi hover.
Disabled giữ Default; TextEdit dùng Text, numeric/splitter dùng ResizeHorizontal.
Ra ngoài control trả Default.21 UI tests/337 regression pass; fmt/clippy/build
pass. Test cursor platform output bằng pointer events thực; evidence
`vintage-cursor-*`. Chưa migrate app/chưa kiểm native screenshot.


## Dropdown list/option padding — 2026-11-16

Khung list không padding (menu_margin0), giữa options không gap. Mỗi option
full width tự có padding12px ngang/8px dọc. Giữ viền ngoài1px, nền đặc,
no-scale, selected colors và PointingHand cho cả Theme/Network. Test font14/24
kiểm geometry, text padding và popup border sát rows.22 UI tests/338 regression
pass; fmt/clippy/build pass. Evidence `vintage-dropdown-padding-*`.
Chưa migrate app/chưa kiểm native screenshot.


## Project selector chữ thuần — 2026-11-16

All/pgapp/staging không còn status badge ●/○ trước text. Click/selected palette
vẫn giữ.23 UI tests/339 regression pass; fmt/clippy/build pass. Evidence
`vintage-project-plain-*`; chưa migrate app/chưa kiểm native screenshot.
