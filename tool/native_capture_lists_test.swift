// Compiled as main.swift by test_native_capture_lists_macos.sh.
import Cocoa

_ = NSApplication.shared
let window = NSWindow(
  contentRect: NSRect(x: 0, y: 0, width: 680, height: 600),
  styleMask: [.titled], backing: .buffered, defer: false)
let palette: [String: Any] = [
  "background": 0xfff6_f8fa, "surface": 0xffff_ffff, "text": 0xff17_2033,
  "muted": 0xff59_6778, "line": 0xffd6_dde5, "selected": 0xff1d_4ed8,
  "selectedText": 0xffff_ffff, "selectedMuted": 0xffdb_eafe, "marker": 0xff60_a5fa,
]
let host = NativeCaptureListHost(kind: "network")
window.contentView = host
host.frame = NSRect(x: 0, y: 0, width: 680, height: 600)
host.configure([
  "titles": ["Method", "URL", "Status", "Time"], "widths": [64, 56, 108], "palette": palette,
])
let keys = (0..<1000).map { "row-\($0)" }
let summaries: [[String: Any]] = keys.enumerated().map { i, key in
  [
    "key": key, "method": "OPTIONS", "path": "/items/\(i)", "status": "200",
    "meta": "example.test", "duration": "12 ms", "methodColor": 0xff25_63eb,
    "statusColor": 0xff04_7857,
  ]
}
host.update(["rows": summaries, "order": keys])
host.layoutSubtreeIfNeeded()
let canvas = host.subviews.first!
let visible = canvas.accessibilityRows() as? [NSAccessibilityElement] ?? []
precondition(visible.count > 0 && visible.count < 20, "Only visible rows should be exposed")
precondition(visible[0].accessibilityLabel()?.contains("/items/0") == true)
precondition(
  host.subviews.allSatisfy { $0.frame.height <= host.bounds.height },
  "Backing views must stay viewport-sized")
let vertical = host.subviews.compactMap { $0 as? NSScroller }.first!
precondition(vertical.isEnabled && vertical.knobProportion < 0.02)

func snapshot(_ view: NSView) -> Data {
  let bitmap = view.bitmapImageRepForCachingDisplay(in: view.bounds)!
  view.cacheDisplay(in: view.bounds, to: bitmap)
  return bitmap.representation(using: .png, properties: [:])!
}
let initial = snapshot(canvas)
var selected = ""
host.emit = { method, args in if method == "selected" { selected = args["key"] as? String ?? "" } }
precondition(visible[0].accessibilityPerformPress())
precondition(selected == "row-0")
precondition(snapshot(canvas) != initial, "Selecting a row must visibly redraw it")
// Exercise the same AppKit mouse handlers as a real divider drag. The view
// size stays constant, so this also catches missing column-only repaints.
func mouse(_ type: NSEvent.EventType, _ x: CGFloat) -> NSEvent {
  NSEvent.mouseEvent(
    with: type, location: canvas.convert(NSPoint(x: x, y: 15), to: nil),
    modifierFlags: [], timestamp: 0, windowNumber: window.windowNumber,
    context: nil, eventNumber: 0, clickCount: 1, pressure: 1)!
}
let beforeDrag = snapshot(canvas)
canvas.mouseDown(with: mouse(.leftMouseDown, 67))
canvas.mouseDragged(with: mouse(.leftMouseDragged, 107))
precondition(
  canvas.needsDisplay, "Divider drag must invalidate even when viewport size is unchanged")
precondition(snapshot(canvas) != beforeDrag)
var savedWidths: [Double] = []
host.emit = { method, args in
  if method == "widthsChanged" { savedWidths = args["widths"] as? [Double] ?? [] }
}
canvas.mouseUp(with: mouse(.leftMouseUp, 107))
precondition(savedWidths.first == 104, "Column width callback must preserve the drag result")
let dragged = snapshot(canvas)
host.configure([
  "titles": ["Method", "URL", "Status", "Time"], "widths": savedWidths, "palette": palette,
])
host.layoutSubtreeIfNeeded()
precondition(snapshot(canvas) == dragged, "Reapplying persisted widths must stay stable")
host.update(["rows": [["key": "row-0", "method": "POST", "path": "/patched", "status": "500"]]])
precondition(
  (canvas.accessibilityRows()?.first as? NSAccessibilityElement)?.accessibilityLabel()?.contains(
    "/patched") == true)

vertical.doubleValue = 0.5
NSApp.sendAction(vertical.action!, to: vertical.target, from: vertical)
precondition(
  (canvas.accessibilityRows()?.first as? NSAccessibilityElement)?.accessibilityLabel()?.contains(
    "/items/0 ") == false)
host.update(["rows": [], "removed": keys, "order": []])
host.layoutSubtreeIfNeeded()
precondition(
  canvas.accessibilityRows()?.isEmpty == true, "Clear must remove native summaries and rows")

let console = NativeCaptureListHost(kind: "console")
window.contentView = console
console.frame = NSRect(x: 0, y: 0, width: 350, height: 600)
console.configure(["palette": palette])
console.update([
  "rows": [
    [
      "key": "log", "level": "INFO", "time": "12:34:56.789",
      "message":
        "Tiếng Việt 😀: a long console message that wraps over several lines.\nSecond line.",
      "levelColor": 0xff25_63eb, "messageColor": 0xff17_2033,
    ]
  ], "order": ["log"],
])
console.layoutSubtreeIfNeeded()
let logCanvas = console.subviews.first!
let log = (logCanvas.accessibilityRows()?.first as? NSAccessibilityElement)!
precondition(log.accessibilityLabel()?.contains("Tiếng Việt 😀") == true)
precondition(log.accessibilityFrame().height > 36, "Multiline logs must grow instead of clipping")
precondition(!snapshot(logCanvas).isEmpty)
print(
  "Native AppKit lists: viewport rendering, selection, patching, scrolling, clear and multiline logs passed"
)
