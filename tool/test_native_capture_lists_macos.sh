#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
viewer_sdk="${FLUTTER_ROOT:-$(cd "$(dirname "$(command -v flutter)")/.." && pwd)}"
viewer_framework="$viewer_sdk/bin/cache/artifacts/engine/darwin-x64/FlutterMacOS.xcframework/macos-arm64_x86_64"
viewer_test_dir="$(mktemp -d /tmp/viewer-appkit-test.XXXXXX)"
trap 'rm -rf "$viewer_test_dir"' EXIT
cp tool/native_capture_lists_test.swift "$viewer_test_dir/main.swift"
swiftc -F "$viewer_framework" -framework FlutterMacOS \
  -Xlinker -rpath -Xlinker "$viewer_framework" \
  -module-cache-path "$viewer_test_dir/cache" \
  macos/Runner/NativeCaptureLists.swift "$viewer_test_dir/main.swift" \
  -o "$viewer_test_dir/test"
"$viewer_test_dir/test"
