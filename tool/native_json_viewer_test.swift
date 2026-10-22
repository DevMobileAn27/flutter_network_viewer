import Cocoa
import FlutterMacOS

func check(_ condition: @autoclosure () -> Bool, _ message: String) {
  if !condition() { fatalError(message) }
}
let app = NSApplication.shared
let host = NativeJsonHost()
let window = NSWindow(
  contentRect: NSRect(x: 0, y: 0, width: 500, height: 300), styleMask: [.titled],
  backing: .buffered, defer: false)
window.contentView = host
host.frame = NSRect(x: 0, y: 0, width: 500, height: 300)
let text = "{\n  \"name\": \"Tiếng Việt 😀\",\n  \"nested\": {\n    \"color\": \"#ff0000\"\n  }\n}"
let ns = text as NSString
let lines = text.components(separatedBy: "\n")
var base = 0
let code: [[String: Any]] = lines.enumerated().map { index, line in
  let n = (line as NSString).length
  let row: [String: Any] = [
    "text": line, "start": base, "end": base + n, "fold": index == 0 ? 5 : index == 2 ? 4 : index,
    "depth": 0, "maps": [[0, n, base]], "spans": [],
  ]
  base += n + 1
  return row
}
let nested = ns.range(of: "\"nested\"").location
let color = ns.range(of: "\"color\"").location
let trees: [[String: Any]] = [
  [
    "text": "{} (2)", "start": 0, "end": ns.length, "fold": 3, "depth": 0,
    "maps": [[0, 1, 0], [1, 1, ns.length - 1]],
  ],
  [
    "text": "\"name\": \"Tiếng Việt 😀\"", "start": 4, "end": nested - 4, "fold": 1, "depth": 1,
    "maps": [[0, 23, 4]],
  ],
  [
    "text": "\"nested\": {} (1)", "start": nested, "end": ns.length - 2, "fold": 3, "depth": 1,
    "maps": [[0, 8, nested]],
  ],
  [
    "text": "\"color\": \"#ff0000\"", "start": color, "end": color + 18, "fold": 3, "depth": 2,
    "maps": [[0, 18, color]],
  ],
]
host.load(["text": text, "code": code, "tree": trees, "expanded": true])
let palette: [String: Any] = [
  "surface": 0xfffa_fafa, "text": 0xff22_2222, "muted": 0xff66_6666, "line": 0xffcc_cccc,
  "selected": 0xff00_55cc, "selectedText": 0xffff_ffff,
]
host.configure(["tree": false, "palette": palette])
host.layoutSubtreeIfNeeded()
let canvas = host.subviews[0]
func snapshot() -> NSBitmapImageRep {
  let rep = canvas.bitmapImageRepForCachingDisplay(in: canvas.bounds)!
  canvas.cacheDisplay(in: canvas.bounds, to: rep)
  return rep
}
let image = snapshot()
var ink = 0
for y in 0..<min(100, image.pixelsHigh) {
  for x in 0..<min(350, image.pixelsWide) {
    if let c = image.colorAt(x: x, y: y)?.usingColorSpace(.deviceRGB), c.redComponent < 0.7 {
      ink += 1
    }
  }
}
check(ink > 30, "JSON must paint native text")
func click(_ x: CGFloat, _ y: CGFloat) {
  let point = canvas.convert(NSPoint(x: x, y: y), to: nil)
  let event = NSEvent.mouseEvent(
    with: .leftMouseDown, location: point, modifierFlags: [], timestamp: 0,
    windowNumber: window.windowNumber, context: nil, eventNumber: 1, clickCount: 1, pressure: 1)!
  canvas.mouseDown(with: event)
}
click(38, 12)
check(
  (canvas.accessibilityValue() as? String)?.components(separatedBy: "\n").count == 1,
  "Root fold hides nested JSON")
host.setMatches([[color, color + 5]])
host.setActive(0)
check(
  (canvas.accessibilityValue() as? String)?.contains("#ff0000") == true,
  "Search opens enclosing code folds")
host.configure(["tree": true, "palette": palette])
// Configure preserves active result and reveals its closed ancestors.
check(
  (canvas.accessibilityValue() as? String)?.contains("#ff0000") == true,
  "Search reveals collapsed tree")
host.setMatches([])
host.load(["text": text, "code": code, "tree": trees, "expanded": true])
check(
  (canvas.accessibilityValue() as? String)?.contains("#ff0000") == true,
  "Nested tree starts fully expanded")
click(46, 56)
check(
  (canvas.accessibilityValue() as? String)?.contains("#ff0000") == false,
  "Expanded tree can still collapse a nested branch")
click(46, 56)
check(
  (canvas.accessibilityValue() as? String)?.contains("#ff0000") == true,
  "Tree disclosure opens child")
for key in ["a"] {
  let event = NSEvent.keyEvent(
    with: .keyDown, location: .zero, modifierFlags: .command, timestamp: 0,
    windowNumber: window.windowNumber, context: nil, characters: key,
    charactersIgnoringModifiers: key, isARepeat: false, keyCode: 0)!
  canvas.keyDown(with: event)
}
check(host.selectedText() == text, "Copy contains source JSON without gutter and preserves emoji")
host.clear()
check((canvas.accessibilityValue() as? String) == "", "Clear releases displayed content")
// Regression: terminal-style branches remain continuous through expanded
// descendants, and the final sibling stops its vertical connector mid-row.
let branchRows: [[String: Any]] = [
  ["text": "{}", "start": 0, "end": 40, "fold": 3, "depth": 0, "count": 2],
  ["text": "group: {}", "start": 2, "end": 25, "fold": 2, "depth": 1,
   "branches": [], "last": false, "count": 1],
  ["text": "child: 1", "start": 12, "end": 24, "fold": 2, "depth": 2,
   "branches": [true], "last": true],
  ["text": "tail: 2", "start": 27, "end": 39, "fold": 3, "depth": 1,
   "branches": [], "last": true],
]
host.load(["text": String(repeating: " ", count: 40), "code": [], "tree": branchRows, "expanded": true])
host.layoutSubtreeIfNeeded()
// All descendants start open; labels still toggle individual branches.
func branchInk(_ image: NSBitmapImageRep, _ px: Int, _ py: Int) -> Bool {
  // Bitmap reps can be Retina; coordinates in the host are points.
  let scale = CGFloat(image.pixelsWide) / canvas.bounds.width
  for yy in Int(CGFloat(py) * scale)..<Int(CGFloat(py + 2) * scale) {
    for xx in Int(CGFloat(px - 1) * scale)..<Int(CGFloat(px + 2) * scale) {
      if let c = image.colorAt(x: xx, y: yy)?.usingColorSpace(.deviceRGB), c.redComponent < 0.7 { return true }
    }
  }
  return false
}
let opened = snapshot()
let cw = (" " as NSString).size(withAttributes: [.font: NSFont.monospacedSystemFont(ofSize: 12, weight: .regular)]).width
let trunk = Int(12 + 0.5 * cw), childTrunk = Int(12 + 4.5 * cw)
check(branchInk(opened, trunk, 47) && branchInk(opened, trunk, 52) && branchInk(opened, trunk, 68),
  "Parent connector must continue through expanded child rows without gaps")
check(branchInk(opened, childTrunk, 54), "Nested child has its own connector")
check(!branchInk(opened, childTrunk, 66), "Last child connector stops at row midpoint")
check(branchInk(opened, trunk, 77) && !branchInk(opened, trunk, 88),
  "Final sibling connector stops at midpoint")
click(80, 38)
let collapsed = snapshot()
check(!branchInk(collapsed, trunk, 68), "Collapsed subtree removes descendant connectors")
host.load(["text": text, "code": code, "tree": trees, "expanded": false])
check((canvas.accessibilityValue() as? String)?.components(separatedBy: "\n").count == 1,
  "Explicit collapsed mode remains supported")
print("Native JSON AppKit tests passed")
