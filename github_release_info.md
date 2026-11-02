---
tag: v0.1.8
title: Flutter Network Viewer v0.1.8
---

## Có gì mới

- Hiển thị body form-data với các trường dữ liệu, tên file, MIME type, dung lượng và chế độ Raw; hỗ trợ cả form URL-encoded.
- Bỏ qua việc tự tải body khi tổng dung lượng request và response vượt 4 MB, giữ headers, thời gian và status để tránh tải upload lớn qua VM Service.
- Sửa phục hồi capture sau hot restart, hủy lần đọc từ isolate cũ và tự kết nối lại khi VM ngừng phản hồi.
- Mỗi lần bật Auto detect chỉ kết nối thêm một project rồi tự tắt; các project đã kết nối vẫn tiếp tục nhận request và console log.
- Bổ sung log chẩn đoán kết nối tự xoay vòng trên macOS và Windows để hỗ trợ kiểm tra lỗi.

## Downloads

- **macOS Apple Silicon (M-series):** `flutter-network-viewer-0.1.8-macos-arm64.zip`
- **macOS Intel:** `flutter-network-viewer-0.1.8-macos-x86_64.zip`
- **Windows x64 installer:** `flutter-network-viewer-0.1.8-windows-x64-setup.exe`
- **Windows x64 portable:** `flutter-network-viewer-0.1.8-windows-x64.zip`

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
