# Flutter Network Viewer

Ứng dụng desktop dành cho macOS và Windows, giúp xem API request/response, đọc log Flutter và định dạng JSON trong cùng một cửa sổ. Không cần thêm thư viện vào ứng dụng đang theo dõi.

![Network: danh sách request và chi tiết response](docs/screenshots/network.jpg)

## Kết nối ứng dụng Flutter

1. Chạy ứng dụng cần theo dõi ở chế độ **debug** hoặc **profile**.
2. Sao chép **Flutter DevTools URL** hoặc **Dart VM Service URL** được in trong terminal sau khi chạy app.
3. Mở Flutter Network Viewer, bấm **Connect an app** khi chưa có dữ liệu, hoặc bấm biểu tượng **?** ở góc trên bên phải.
4. Dán URL, bấm **Connect an app** và đợi trạng thái **Connected**.
5. Thao tác trên ứng dụng Flutter để phát sinh request và log.

Có thể đổi URL tại **Settings → Connection → Save & restart**. Khi chạy lại ứng dụng và URL thay đổi, hãy cập nhật URL mới. Giữ ứng dụng Flutter đang chạy trong suốt quá trình theo dõi.

## Network

- Tìm request theo URL, method hoặc status; kết hợp bộ lọc method và trạng thái.
- Chọn một request để xem **Overview**, **Request header**, **Request body**, **Response header**, **Response body** và **Raw**.
- Với body JSON, chuyển giữa **JSON** và **Tree**, thu gọn các nhánh hoặc sao chép nội dung.
- Với response ảnh, xem trước và phóng to ảnh ngay trong phần body.
- Kéo divider giữa danh sách và chi tiết để đổi độ rộng. App ghi nhớ vị trí divider và tab chi tiết đang chọn khi chuyển request.

Nếu có nhiều ứng dụng, chọn tab tên ứng dụng để xem riêng hoặc **All** để xem tất cả.

## Console

Xem log Flutter, tìm theo nội dung và lọc theo nguồn hoặc mức độ log. Chọn một dòng để xem chi tiết, lỗi và stack trace khi có. Có thể kéo divider để điều chỉnh phần danh sách và chi tiết.

![Console: lọc log và xem chi tiết](docs/screenshots/console.jpg)

## Json beautify

1. Mở tab **Json beautify** và dán JSON vào khung bên trái.
2. Bấm **Format** hoặc **⌘/Ctrl + Enter**.
3. Xem kết quả ở dạng **JSON** hoặc **Tree**, tìm kiếm và sao chép kết quả.

Nếu JSON không hợp lệ, app hiển thị dòng và cột có lỗi. Kéo divider để thay đổi độ rộng hai khung; vị trí này được ghi nhớ. Công cụ dùng được ngay cả khi chưa kết nối ứng dụng.

![Json beautify: định dạng và tìm kiếm trong JSON](docs/screenshots/json-beautify.jpg)

### Tìm trong JSON

Thanh tìm kiếm có trong **Request body**, **Response body** khi nội dung là JSON, và trong kết quả **Json beautify**.

| Thao tác | Cách dùng |
| --- | --- |
| Mở ô tìm kiếm | **⌘/Ctrl + F** trong khu vực JSON |
| Phân biệt chữ hoa/thường | Bật **Aa** |
| Tìm nguyên từ | Bật **ab** |
| Tìm bằng biểu thức chính quy | Bật **.\*** |
| Kết quả tiếp theo / trước đó | Mũi tên xuống / lên, hoặc **Enter / Shift + Enter** khi đang nhập tìm kiếm |
| Xóa tìm kiếm | Nút **×** |

Kết quả được tô sáng trong nội dung. **2 of 3** nghĩa là đang ở kết quả thứ 2 trong 3 kết quả; **No results** nghĩa là chưa có kết quả để hiển thị. Nội dung tìm kiếm được giữ khi đổi giữa JSON và Tree.

## Tạm dừng, xóa và xuất dữ liệu

- **Pause / Resume**: tạm dừng hoặc tiếp tục nhận dữ liệu. Request và log phát sinh trong lúc tạm dừng sẽ không được bổ sung lại.
- **Clear**: xóa request và log đang lưu trong Viewer.
- **Export**: lưu toàn bộ dữ liệu đã thu thập thành file JSON, bao gồm cả mục đang bị ẩn bởi bộ lọc.

Dữ liệu chỉ được giữ trong phiên mở app; hãy xuất trước khi đóng nếu cần lưu lại. Khi đạt giới hạn lưu trữ, các mục cũ nhất được loại bỏ.

## Cài đặt và MCP

Bấm biểu tượng **bánh răng** để đổi giao diện sáng/tối, ngôn ngữ Việt/Anh hoặc URL kết nối. Các lựa chọn được ghi nhớ cho lần mở sau.

Để dùng dữ liệu trong công cụ AI có hỗ trợ MCP:

1. Vào **Settings → MCP** và bật công tắc.
2. Bấm **Copy setup command**, sau đó dán vào công cụ AI để thiết lập kết nối.
3. Nếu cần cấu hình thủ công, bấm mũi tên bên cạnh nút copy để chọn cấu hình JSON hoặc Codex.
4. Giữ Viewer mở và chạy công cụ AI trên cùng máy tính.

Cổng mặc định là **38943**. Nếu đổi cổng, bấm **Save & restart** rồi sao chép lại cấu hình. MCP cho phép AI đọc danh sách ứng dụng, request, chi tiết request, log và thống kê dữ liệu đang có.

![Cài đặt: giao diện, kết nối và MCP](docs/screenshots/settings.jpg)

## Cập nhật ứng dụng

App kiểm tra phiên bản mới khi mở. Bạn cũng có thể vào **Settings → App updates → Check for updates** để kiểm tra thủ công. Khi có bản mới, xác nhận trong cửa sổ cập nhật để tải và cài đặt; app sẽ đóng rồi mở lại. Hãy xuất capture cần giữ trước khi cập nhật.

Bản **0.1.0** cần tải và cài thủ công bản có updater một lần. Trên macOS, kéo app vào **Applications**. Trên Windows, nên dùng file `windows-x64-setup.exe`.

## Khi chưa thấy request hoặc log

- Kiểm tra trạng thái **Connected**, URL mới nhất và nút **Pause**.
- Thử thao tác gọi API hoặc ghi log mới sau khi kết nối; console không có log của thời điểm trước đó.
- Chạy ứng dụng Flutter ở **debug/profile**. Không hỗ trợ theo dõi app release hoặc Flutter Web.
- Hỗ trợ HTTP thông thường của Flutter, bao gồm cấu hình mặc định của Dio và package:http. Request từ WebView, thư viện HTTP native và dữ liệu WebSocket không xuất hiện trong danh sách này.
- Xóa nội dung tìm kiếm, đưa bộ lọc về **All** và kiểm tra tab ứng dụng đang chọn.

## Chạy từ mã nguồn

Cần Flutter **3.41 / Dart 3.11** trở lên. Trên macOS cần Xcode và CocoaPods; trên Windows cần Visual Studio với **Desktop development with C++**.

```sh
flutter pub get
flutter run -d macos
```

Trên Windows, thay lệnh cuối bằng `flutter run -d windows`.

Để tạo bản ứng dụng:

```sh
# macOS
bash tool/build_macos.sh

# Windows (chạy trên Windows)
flutter build windows --release
```

Bản macOS nằm tại `build/macos/Build/Products/Release/Flutter Network Viewer.app`. Bản Windows nằm trong `build/windows/x64/runner/Release/`; giữ nguyên các file đi kèm khi sao chép ứng dụng.


## Phát hành bản mới

GitHub Actions kiểm tra format, analyze và test cho pull request. Khi tăng version và build number trong `pubspec.yaml`, điền `github_release_info.md` rồi push lên `main`, workflow build macOS chip M, macOS Intel và Windows x64, tạo draft release kèm ZIP, installer Windows, feed cập nhật đã ký và checksum. Bạn kiểm tra draft trước khi publish.

Xem [hướng dẫn phát hành](docs/releasing.md) và [cấu hình khóa ký cập nhật](docs/app-updates.md). Cần cấu hình hai Actions secrets trước lần release đầu tiên.
