---
tag: v0.1.7
title: Flutter Network Viewer v0.1.7
---

## Có gì mới

- Sửa lỗi kiểm tra mã trên GitHub Actions khi dùng Flutter SDK mới, giữ tương thích với SDK cũ.
- Chuyển danh sách request, console log, JSON, Tree và khung xem ảnh sang renderer native trên macOS và Windows.
- Khôi phục đường nối cha–con trong Tree và mở rộng toàn bộ nhánh theo mặc định; vẫn có thể đóng/mở từng nhánh.
- Khung ảnh phóng to có thanh công cụ nổi ở góc dưới bên phải. GIF/WebP động hiển thị khung hình đầu để giảm xử lý liên tục.
- Khung xem nhỏ và phóng to dùng chung ảnh đã giải mã; giải phóng tài nguyên khi xoá dữ liệu.
- Tối ưu cập nhật danh sách và xử lý nội dung chi tiết; thanh trạng thái chỉ hiển thị dung lượng RAM.
- Thu gọn bố cục, tận dụng chiều cao vùng xem JSON và ảnh.

## Downloads

- **macOS Apple Silicon (M-series):** `flutter-network-viewer-0.1.7-macos-arm64.zip`
- **macOS Intel:** `flutter-network-viewer-0.1.7-macos-x86_64.zip`
- **Windows x64 installer:** `flutter-network-viewer-0.1.7-windows-x64-setup.exe`
- **Windows x64 portable:** `flutter-network-viewer-0.1.7-windows-x64.zip`

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
