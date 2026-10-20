---
tag: v0.1.4
title: Flutter Network Viewer v0.1.4
---

## Có gì mới

- Đồng bộ màn hình Cài đặt: các nút thao tác nằm bên phải, mô tả màu xám đậm và trạng thái cập nhật hiển thị cạnh phiên bản. Khi có bản mới, nút đổi thành Update new version.
- Thiết lập MCP với nút Copy & Paste to và danh sách chọn Codex/Claude; mặc định là Codex, có thông báo sao chép thành công.
- Thay snackbar bằng popup ở góc trái bên dưới, hỗ trợ bốn trạng thái thông báo, nút xác nhận/hủy và tùy chọn tự ẩn.
- Làm rõ kết nối tự động và nhập URL DevTools thủ công; nút Start/Dừng có màu riêng, trạng thái tìm kiếm có vòng loading xanh dương và bỏ nút tìm kiếm trùng chức năng.
- Hoàn thiện nội dung hướng dẫn và bản dịch tiếng Việt, gồm nút tham gia thảo luận và thông báo khi chưa có dự án Flutter.
- Bổ sung tám GIF hướng dẫn ngắn theo từng tính năng trong README, sử dụng dữ liệu demo và mũi tên đỏ chỉ vị trí thao tác.
- Sửa lỗi kiểm tra Dart format khiến GitHub Actions dừng trước bước analyze và test.

## Downloads

- **macOS Apple Silicon (M-series):** `flutter-network-viewer-0.1.4-macos-arm64.zip`
- **macOS Intel:** `flutter-network-viewer-0.1.4-macos-x86_64.zip`
- **Windows x64 installer:** `flutter-network-viewer-0.1.4-windows-x64-setup.exe`
- **Windows x64 portable:** `flutter-network-viewer-0.1.4-windows-x64.zip`

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
