# Cập nhật ứng dụng

App kiểm tra cập nhật khi mở và khi bấm **Settings → App updates → Check for updates**. Popup của Sparkle (macOS) hoặc WinSparkle (Windows) hiển thị phiên bản và nội dung release. Người dùng chọn tải/cài; app không tự cài âm thầm. Khi cài đặt, app đóng rồi mở lại. Xuất capture cần giữ trước khi cập nhật.

Mỗi nền tảng/chip có feed riêng trong **GitHub Releases → latest**. Draft chưa được công khai nên không xuất hiện trong app. Trên Windows nên cài bản `windows-x64-setup.exe`; installer cài theo user, không yêu cầu quyền administrator. Bản chạy từ ZIP sẽ được chuyển sang bản cài tại `%LOCALAPPDATA%\Programs\Flutter Network Viewer` khi cập nhật.

Các phiên bản cũ chưa tích hợp updater cần tải/cài thủ công một lần. Từ bản có updater trở đi mới có thể nhận bản tiếp theo trong app.

## Cấu hình ký cho maintainer (một lần)

Public keys được lưu trong `updates/` và nhúng vào app. Private keys đã được tạo trong `.local/update-signing/` (Git ignore, quyền đọc giới hạn). Sao lưu hai file private này ở nơi riêng tư. Không đổi public key sau khi phát hành mà chưa có kế hoạch chuyển khóa.

Thêm hai **repository Actions secrets** tại **GitHub → Settings → Secrets and variables → Actions**:

| Secret | Nội dung file local |
| --- | --- |
| `MACOS_UPDATE_PRIVATE_KEY` | `.local/update-signing/macos_ed25519.key` |
| `WINDOWS_UPDATE_PRIVATE_KEY` | `.local/update-signing/windows_dsa.pem` |

Nếu dùng GitHub CLI đã đăng nhập:

```sh
gh secret set MACOS_UPDATE_PRIVATE_KEY --repo DevMobileAn27/flutter_network_viewer < .local/update-signing/macos_ed25519.key
gh secret set WINDOWS_UPDATE_PRIVATE_KEY --repo DevMobileAn27/flutter_network_viewer < .local/update-signing/windows_dsa.pem
```

Khi cần tạo khóa cho một dự án mới, dùng `bash tool/generate_update_keys.sh` trên macOS. Script giữ nguyên khóa private đã có; không chạy để thay khóa của ứng dụng đã phát hành. Script đồng bộ public key Sparkle vào `SUPublicEDKey` trong `macos/Runner/Info.plist`.

Workflow yêu cầu secrets trước khi build release. Nó ký ZIP macOS bằng Ed25519, ký installer Windows theo giao thức WinSparkle 0.8.x, kiểm tra khóa ký khớp public key và tạo:

- `appcast-macos-arm64.xml`
- `appcast-macos-x86_64.xml`
- `appcast-windows-x64.xml`
- `flutter-network-viewer-VERSION-windows-x64-setup.exe`

Ba ZIP, installer, appcast và SHA256 được đính kèm draft. Khi Publish, app đọc feed từ bản latest. Không cần GitHub Pages hoặc server riêng.

Chữ ký update xác minh file trước khi cài, khác với **Apple Developer ID/notarization** hoặc **Windows Authenticode**. Chứng chỉ nền tảng chưa được cấu hình; đây vẫn là giới hạn phân phối hiện tại và có thể gây cảnh báo hệ điều hành. Sandbox macOS vẫn bật; Sparkle dùng Installer XPC và hai mach lookup exception dành riêng cho bundle ID của app.

## Kiểm thử trước khi phát hành rộng rãi

1. Cấu hình secrets và phát hành bản đầu tiên có updater; cài thủ công trên cả Mac và Windows.
2. Tăng version **và build number** (ví dụ `0.1.1+2` → `0.1.2+3`), điền release info rồi push.
3. Giữ draft để kiểm tra: app chưa được thông báo cập nhật.
4. Publish và đặt làm latest; trong app cũ bấm **Check for updates**.
5. Thử Cancel/Later, tải lại, cài đặt và mở lại. Kiểm tra đúng kiến trúc và các cài đặt vẫn được giữ.

Khi offline, feed chưa có, hoặc lỗi chữ ký, thao tác cần báo lỗi thay vì cài file không xác minh. Trạng thái kiểm tra được hiển thị trong Settings; lỗi từ kiểm tra nền không tự mở popup lỗi.
