---
tag: v0.1.3
title: Flutter Network Viewer v0.1.3
---

## Có gì mới

- Tự tìm ứng dụng Flutter đang chạy trên cùng máy; chọn project khi có nhiều ứng dụng và tự kết nối lại khi URL thay đổi.
- Kéo divider để thay đổi độ rộng các cột Network, ghi nhớ khi chuyển tab hoặc mở lại app. Method và Status mặc định ở chiều rộng tối thiểu; danh sách và chi tiết mặc định chia đôi cửa sổ.
- Thanh tab chi tiết có gradient xanh báo còn nội dung để cuộn ngang; bỏ tiêu đề Headers/Body lặp bên dưới.
- Xem preview chuỗi màu trong JSON và Tree, gồm HEX, RGB/HSL và màu Flutter.
- Tự nhận diện ảnh base64 trong body, kể cả thiếu thông tin encoding/content-type; hỗ trợ phóng to và xem Raw.
- Cập nhật README với hướng dẫn sử dụng và screenshot dữ liệu demo.

## Downloads

- **macOS Apple Silicon (M-series):** `flutter-network-viewer-0.1.3-macos-arm64.zip`
- **macOS Intel:** `flutter-network-viewer-0.1.3-macos-x86_64.zip`
- **Windows x64 installer:** `flutter-network-viewer-0.1.3-windows-x64-setup.exe`
- **Windows x64 portable:** `flutter-network-viewer-0.1.3-windows-x64.zip`

On macOS, unzip and drag **Flutter Network Viewer.app** into **Applications**.
On Windows, run the installer. For the portable ZIP, unzip and run **flutter_network_viewer.exe**. Keep the DLL files and `data` folder alongside the executable.

Use `SHA256SUMS.txt` to verify downloads. The `appcast-*.xml` files are used by the app updater.

### Nếu macOS chặn ứng dụng

Sau khi chuyển ứng dụng vào **Applications**, nếu macOS báo nhà phát triển chưa được xác minh hoặc ứng dụng bị hỏng, hãy chạy các lệnh sau trong **Terminal**:

```sh
xattr -dr com.apple.quarantine "/Applications/Flutter Network Viewer.app"
open "/Applications/Flutter Network Viewer.app"
```

Các lệnh trên gỡ thuộc tính cách ly (quarantine) của ứng dụng và mở lại app. Chỉ sử dụng với bản tải xuống đáng tin cậy từ GitHub Releases chính thức của kho mã nguồn này.

## Upgrading

Versions without the updater need to install **0.1.2** manually once. This version can receive future published updates from within the app. Export captures you want to keep before updating.

Chạy ứng dụng Flutter ở chế độ debug/profile và mở Viewer để tự tìm project. Có thể nhập Flutter DevTools hoặc Dart VM Service URL khi cần kết nối thủ công. Không cần thêm thư viện vào ứng dụng đang theo dõi.
