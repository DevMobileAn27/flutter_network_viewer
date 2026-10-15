#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."

# Flutter 3.41's universal-framework lipo verification fails with Xcode 27.
# Build an explicit architecture without changing the installed Flutter SDK.
# Usage: bash tool/build_macos.sh [arm64|x86_64]
viewer_arch="${1:-$(uname -m)}"
case "$viewer_arch" in
  arm64|x86_64) ;;
  *) echo 'Expected arm64 or x86_64' >&2; exit 1 ;;
esac
python3 tool/generate_app_icons.py
flutter build macos --release --config-only
(cd macos && pod install)
xcodebuild -workspace macos/Runner.xcworkspace -scheme Runner \
  -configuration Release -derivedDataPath build/macos \
  -destination "platform=macOS,arch=$viewer_arch" \
  "ARCHS=$viewer_arch" ONLY_ACTIVE_ARCH=YES build
