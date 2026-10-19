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

### If macOS blocks the app

After moving the app into **Applications**, if macOS reports an unverified developer or a damaged app, run in **Terminal**:

```sh
xattr -dr com.apple.quarantine "/Applications/Flutter Network Viewer.app"
open "/Applications/Flutter Network Viewer.app"
```

Only use this for a trusted download from this repository’s official GitHub Releases.

## Upgrading

Versions without the updater need to install **0.1.2** manually once. This version can receive future published updates from within the app. Export captures you want to keep before updating.

Run your Flutter app in debug/profile mode and connect using its Flutter DevTools or Dart VM Service URL. No SDK integration is required in the app being monitored.
