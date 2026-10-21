# Inspector selection performance

The JSON code view renders documents longer than 512 lines with a fixed-height,
lazy list. Syntax highlighting, text layout, color swatches and fold controls are
created for rows near the viewport. Short documents keep their existing
single selectable paragraph. Request bodies, console JSON and JSON Beautify
share this renderer.

Captured JSON strings use an LRU preparation cache (24 entries, 32 MiB estimated
budget). The estimate includes decoded data, formatted text and indexes; it is
not a measured heap limit. Oversized entries are rendered without retention.
Search indexes are created when searching or opening Tree, rather than on every
request selection. Mutable request objects are not cache keys: completed or
changed body strings get their own prepared data.

Search operates on the full document and scrolls to offscreen matches. Folding,
color swatches, manual text selection, full-document Select All/Copy and the
section Copy button remain available. Rows participating in an active selection
are retained while scrolling. Single very long lines and expanded Tree nodes
can still be expensive; this change does not impose a CPU ceiling.

## Measured comparison

Measured on macOS arm64 in Flutter profile mode using synthetic JSON. Each case
alternates two requests 12 times with a 150 ms pause after each rendered frame.
The baseline retains the same preparation cache and uses the eager JSON
renderer, isolating the rendering change. The two rows for each size are the
first and repeat workloads in the same process.

| Body | Workload | Eager CPU average | Virtual CPU average | Eager selection median | Virtual selection median |
| --- | --- | ---: | ---: | ---: | ---: |
| 13,641 bytes | First | 19.0% | 6.1% | 41.2 ms | 16.7 ms |
| 13,641 bytes | Repeat | 9.4% | 3.4% | 23.3 ms | 15.1 ms |
| 138,291 bytes | First | 87.4% | 6.7% | 1,056.2 ms | 14.5 ms |
| 138,291 bytes | Repeat | 86.6% | 5.9% | 1,062.1 ms | 14.4 ms |

The optimized short console-log workload averaged 2.8% CPU. CPU is cumulative
process CPU time divided by elapsed wall time: 100% represents one occupied
core. These workload averages do not measure instantaneous peaks or promise
that the complete app always remains under 8%. Live capture, discovery, large
images, text shape and other system activity can change the results.

See [measurement data](performance-selection-2026-10-21.json) for elapsed times,
frame build/raster totals, process memory and sample counts. Early measurements
interrupted by window occlusion were excluded from this comparison.

## Repeat the measurement

`tool/profile_viewer.dart` is a development-only entry point; it does not connect
to projects, start discovery, load preferences or check updates. It displays
synthetic request/log inspectors and exposes `ext.viewerProbe.run`. The normal
production entry point remains `lib/main.dart`.

```sh
flutter build macos --profile --config-only -t tool/profile_viewer.dart
xcodebuild -workspace macos/Runner.xcworkspace -scheme Runner \
  -configuration Profile -derivedDataPath build/macos \
  -destination 'platform=macOS,arch=arm64' ARCHS=arm64 ONLY_ACTIVE_ARCH=YES build
flutter run -d macos --profile -t tool/profile_viewer.dart \
  --use-application-binary='build/macos/Build/Products/Profile/Flutter Network Viewer.app'
```

Use the WebSocket URI printed by that process in another terminal:

```sh
dart run tool/measure_viewer.dart '<VM service WebSocket URI>' /tmp/viewer-profile.json \
  medium,medium,large,large,console,small,consoleJson
```

The harness forces a resumed scheduler and requests frames to avoid measuring
desktop occlusion as low CPU usage. It rejects frame waits over two seconds.
It collects Dart VM CPU samples, Flutter frame timings and native process
CPU/memory samples. After profiling, quit the process and use
`bash tool/build_macos.sh arm64` for the normal release app.
