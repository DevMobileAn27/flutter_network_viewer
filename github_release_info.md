---
tag: v0.1.2
title: Flutter Network Viewer v0.1.2
---

## What’s new

- Check for updates at startup or from **Settings → App updates → Check for updates**.
- Confirm in the update window to download and install a new version.
- Signed update feeds for macOS Apple Silicon, macOS Intel and Windows x64.
- Windows installer alongside the portable ZIP download.
- Current app version and author displayed in Settings.

## Downloads

- **macOS Apple Silicon (M-series):** `flutter-network-viewer-0.1.2-macos-arm64.zip`
- **macOS Intel:** `flutter-network-viewer-0.1.2-macos-x86_64.zip`
- **Windows x64 installer:** `flutter-network-viewer-0.1.2-windows-x64-setup.exe`
- **Windows x64 portable:** `flutter-network-viewer-0.1.2-windows-x64.zip`

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

Run your Flutter app in debug/profile mode and connect using its Flutter DevTools or Dart VM Service URL. No SDK integration is required in the app being monitored.
