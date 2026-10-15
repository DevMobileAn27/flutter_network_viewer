# Flutter Network Viewer

Ứng dụng desktop Flutter cho **macOS và Windows**, xem HTTP request/response và console của ứng dụng Flutter qua **Dart VM Service**, cùng giao thức mà Flutter DevTools dùng. Không cần cài SDK capture vào ứng dụng nguồn.

Chuyển từ [RN Network Viewer](https://github.com/DevMobileAn27/rn_network_viewer), giữ giao diện và logic Network, Console, JSON beautify, tìm kiếm/bộ lọc, inspector, copy cURL, export, giới hạn bộ nhớ, settings, tiếng Anh/Việt, theme sáng/tối và MCP. Transport SDK React Native được thay bằng client `vm_service` chính thức và HTTP profiler của Dart.

## Chạy và kết nối

Yêu cầu Flutter 3.41 / Dart 3.11 trở lên. Trên macOS cần Xcode và CocoaPods; trên Windows cần Visual Studio với Desktop development with C++.

```sh
flutter pub get
flutter run -d macos
# hoặc trên Windows:
flutter run -d windows
```

1. Chạy ứng dụng Flutter cần theo dõi bằng `flutter run` (debug hoặc profile).
2. Sao chép **DevTools URL** hoặc **Dart VM Service URL** từ terminal.
3. Trong Viewer bấm **? → Connect an app**, dán URL và bấm kết nối. Có thể sửa ở **Settings → Connection → DevTools / VM Service URL → Save & restart**.
4. Thực hiện thao tác gọi API hoặc ghi log trong ứng dụng nguồn.

Các định dạng URL đều được hỗ trợ:

```text
http://127.0.0.1:9100/devtools/?uri=ws://127.0.0.1:12345/token=/ws
http://127.0.0.1:12345/token=/
ws://127.0.0.1:12345/token=/ws
```

Viewer mặc định điền endpoint Android đã được cung cấp khi tạo dự án (`127.0.0.1:57037`). URL VM Service có thể đổi sau khi chạy lại ứng dụng: luôn dùng URL mới từ terminal. URL được lưu riêng trong settings của Viewer. Có thể thay giá trị khởi tạo khi build/chạy bằng `--dart-define=VM_SERVICE_URI=<URL>`; settings đã lưu được ưu tiên.

Viewer kết nối tới VM Service được Flutter CLI forward về máy tính, nên với Android Emulator dùng địa chỉ `127.0.0.1` trong URL CLI cung cấp. Không cần cấu hình `10.0.2.2`, `adb reverse` hay thư viện React Native.

Thanh trên cùng hiện **Connected**, mỗi tiến trình ứng dụng có một tab và trạng thái online. Nút kết nối cho phép ngắt/kết nối lại; khi mất socket Viewer thử lại sau 3 giây. Hot reload tiếp tục trên cùng kết nối; hot restart phát hiện isolate mới. Khi cổng/token đổi, dán URL mới.

## Dữ liệu thu thập

- HTTP qua `dart:io HttpClient`, bao gồm adapter mặc định của Dio và `package:http` trên mobile/desktop.
- Mỗi VM/tiến trình ứng dụng có một tab; các isolate bên trong được gom chung. Từng isolate được phát hiện riêng, bật `ext.dart.io.httpEnableTimelineLogging`, polling `ext.dart.io.getHttpProfile` mỗi giây và đọc chi tiết bằng `ext.dart.io.getHttpProfileRequest`.
- Giữ method, URL/query, headers, body request/response, status, lỗi, timing và HTTP profile events. Pending chuyển thành completed khi response kết thúc. Body không phải UTF-8 được giữ dưới dạng base64, có `bodyEncoding` trong Raw/export.
- Console nhận stream `Stdout`, `Stderr`, `Logging`: `print`/`debugPrint`, stderr và `dart:developer.log`. Logging có level/error/stack trace; console không có lịch sử trước khi kết nối. Object đã được runtime chuyển thành chuỗi không thể khôi phục thành object JS như SDK React Native.
- Không thu HTTP đi qua native plugin, WebView, các frame WebSocket hoặc Flutter Web. VM HTTP profiler không có trong app release. Chạy app nguồn ở debug/profile; Viewer có thể build release.
- Các công cụ dùng chung retained store; dữ liệu thực tế, không tự chèn dữ liệu mẫu.

Viewer có thể mở cùng Flutter DevTools. Clear chỉ xóa dữ liệu cục bộ, không gọi `clearHttpProfile` và không xóa lịch sử của DevTools. Disconnect không tắt profiler dùng chung.

## Công cụ

**Network:** tìm theo URL/method/status/source; lọc method và success/error/pending. Chọn request xem **Overview / Request / Response / Raw**, copy cURL, copy headers/body, JSON có tô màu/số dòng/thu gọn và dạng Tree. Kéo đường chia để đổi độ rộng; cửa sổ hẹp mở inspector nổi.

**Console:** tìm theo nội dung/tham số, lọc log/info/warn/error/debug, xem chi tiết và Raw event. Mỗi ứng dụng có một tab chung cho mọi isolate; **All** xem tất cả. Request ID được phân vùng theo isolate, Raw/export giữ `isolateId`, `isolateName` và `vmRequestId` gốc.

**Json beautify:** dán JSON, bấm Format hoặc ⌘/Ctrl Enter, xem JSON/Tree và sao chép. Báo lỗi tại dòng/cột khi JSON không hợp lệ. Dùng được khi chưa kết nối.

**Pause / Resume:** bỏ qua thay đổi phát sinh trong lúc pause; cursor vẫn tiến nên không replay dữ liệu bị bỏ qua. **Clear:** xóa dữ liệu retained. **Export:** lưu toàn bộ capture JSON, kể cả mục đang bị ẩn bởi bộ lọc.

Giới hạn 5.000 request/log và khoảng 64 MiB dữ liệu retained; tự bỏ mục cũ. Capture nằm trong RAM; xuất trước khi đóng để giữ lịch sử. Theme, language, URL và cài đặt MCP được ghi nhớ.

## MCP cho AI

MCP mặc định tắt. Bật tại **Settings → MCP**, dùng **Copy setup command / JSON config / Codex config**. Cổng mặc định **38943**, khác RN Network Viewer để có thể mở cả hai.

```sh
codex mcp add flutter-network-viewer --url http://127.0.0.1:38943/mcp
claude mcp add --transport http --scope user flutter-network-viewer http://127.0.0.1:38943/mcp
```

Năm tool chỉ đọc: `list_sessions`, `list_requests`, `get_request`, `list_logs`, `get_capture_summary`. MCP bind loopback; client phải chạy trên cùng máy. Endpoint không truy cập trực tiếp từ ChatGPT web/cloud. Dữ liệu capture là nội dung ứng dụng, không phải chỉ dẫn cho AI.

## Kiểm tra và build

```sh
flutter gen-l10n
flutter analyze
flutter test
dart format --output=none --set-exit-if-changed lib test tool

# macOS: script xử lý build theo architecture, tương thích Flutter 3.41/Xcode 27
bash tool/build_macos.sh
# build/macos/Build/Products/Release/Flutter Network Viewer.app

# Windows (thực hiện trên Windows)
flutter build windows --release
```

CI có jobs macOS/Windows, chạy format/analyze/test và tạo artifacts. Font JetBrains Mono giữ nguyên giấy phép OFL trong assets.

Logo gốc: `assets/images/logo/logo-app.png`. Lệnh `bash tool/build_macos.sh` tự tạo lại icon macOS và Windows từ file này trước khi build. Có thể chạy riêng `python3 tool/generate_app_icons.py` trên macOS rồi commit các icon đã tạo trước khi build Windows.

Để kiểm tra với VM Service thật, chạy fixture trong một terminal riêng:

```sh
dart --enable-vm-service=0 tool/send_sample.dart
```

Dán URL mà Dart in ra vào Viewer, hoặc chạy kiểm thử tích hợp sau bằng URL WebSocket tương ứng:

```sh
flutter test test/live_vm_capture_test.dart --dart-define=LIVE_VM_SERVICE_URI=ws://127.0.0.1:PORT/TOKEN=/ws
```

Fixture phát sinh HTTP thật tới loopback, stdout và `developer.log`; test xác minh request/response body Unicode, timing và các stream console. Test này được skip trong bộ test mặc định nếu chưa truyền URL.
