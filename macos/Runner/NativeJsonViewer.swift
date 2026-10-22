import Cocoa
import FlutterMacOS

final class NativeJsonViewer {
  private let channel: FlutterMethodChannel
  private var hosts: [Int64: NativeJsonHost] = [:]
  init(messenger: FlutterBinaryMessenger) {
    channel = FlutterMethodChannel(
      name: "dev.quocan.flutterNetworkViewer/native_json", binaryMessenger: messenger)
    channel.setMethodCallHandler { [weak self] call, result in
      guard let self = self else {
        result(nil)
        return
      }
      if call.method == "capabilities" {
        result(1)
        return
      }
      guard let args = call.arguments as? [String: Any],
        let id = (args["id"] as? NSNumber)?.int64Value
      else {
        result(FlutterError(code: "arguments", message: "Missing JSON host ID", details: nil))
        return
      }
      if call.method == "create" {
        _ = self.view(id)
      } else if call.method == "dispose" {
        self.hosts.removeValue(forKey: id)?.removeFromSuperview()
      } else if let host = self.hosts[id] {
        switch call.method {
        case "clear": host.clear()
        case "document": host.load(args)
        case "configure": host.configure(args)
        case "matches": host.setMatches(args["ranges"] as? [[Int]] ?? [])
        case "active": host.setActive(args["index"] as? Int ?? -1)
        case "frame": host.isHidden = !(args["visible"] as? Bool ?? false)
        default:
          result(FlutterMethodNotImplemented)
          return
        }
      }
      result(nil)
    }
  }
  fileprivate func view(_ id: Int64) -> NativeJsonHost {
    if let host = hosts[id] { return host }
    let host = NativeJsonHost()
    host.emit = { [weak self] method, fields in
      self?.channel.invokeMethod(method, arguments: fields.merging(["id": id]) { _, new in new })
    }
    hosts[id] = host
    return host
  }
}
final class NativeJsonFactory: NSObject, FlutterPlatformViewFactory {
  let service: NativeJsonViewer
  init(service: NativeJsonViewer) { self.service = service }
  func createArgsCodec() -> (FlutterMessageCodec & NSObjectProtocol)? {
    FlutterStandardMessageCodec.sharedInstance()
  }
  func create(withViewIdentifier viewId: Int64, arguments args: Any?) -> NSView {
    service.view(((args as? [String: Any])?["id"] as? NSNumber)?.int64Value ?? viewId)
  }
}

private func jsonColor(_ value: Any?) -> NSColor {
  let n = (value as? NSNumber)?.uint32Value ?? 0xff00_0000
  return NSColor(
    srgbRed: CGFloat((n >> 16) & 255) / 255, green: CGFloat((n >> 8) & 255) / 255,
    blue: CGFloat(n & 255) / 255, alpha: CGFloat((n >> 24) & 255) / 255)
}
private struct JsonRow {
  let text: NSString
  let start: Int, end: Int, fold: Int, depth: Int, columns: Int
  let spans: [[Any]], maps: [[Int]]
  let swatch: NSColor?
  let branches: [Bool], last: Bool, count: Int
  init(_ value: [String: Any]) {
    text = (value["text"] as? String ?? "") as NSString
    columns = value["columns"] as? Int ?? text.length
    start = value["start"] as? Int ?? 0
    end = value["end"] as? Int ?? 0
    fold = value["fold"] as? Int ?? 0
    depth = value["depth"] as? Int ?? 0
    spans = value["spans"] as? [[Any]] ?? []
    maps = value["maps"] as? [[Int]] ?? []
    branches = value["branches"] as? [Bool] ?? []
    last = value["last"] as? Bool ?? true
    count = value["count"] as? Int ?? 0
    swatch = value["swatch"] == nil ? nil : jsonColor(value["swatch"])
  }
}

/// The backing layer is viewport-sized, independent of document size.
/// Only visible rows are shaped; the bounded cache is released on replacement.
final class NativeJsonHost: NSView {
  var emit: ((String, [String: Any]) -> Void)?
  private var code: [JsonRow] = [], treeRows: [JsonRow] = []
  private var document: NSString = ""
  private var tree = false
  private var countPattern = "{count} items"
  private var codeClosed = Set<Int>(), treeClosed = Set<Int>()
  private var visible: [Int] = []
  private var matches: [[Int]] = []
  private var active = -1
  private var palette: [String: Any] = [:]
  private var font = NSFont.monospacedSystemFont(ofSize: 12, weight: .regular)
  private var lineHeight: CGFloat = 22
  private var x: CGFloat = 0, y: CGFloat = 0, contentWidth: CGFloat = 0
  private var viewport = NSRect.zero
  private let canvas = JsonCanvas()
  private let vertical = NSScroller(), horizontal = NSScroller()
  private let cache = NSCache<NSNumber, NSAttributedString>()
  private var selectionAnchor: Int?, selectionEnd: Int?
  private var rows: [JsonRow] { tree ? treeRows : code }
  private var closed: Set<Int> { tree ? treeClosed : codeClosed }
  private var gutter: CGFloat { tree ? 12 : max(52, CGFloat(String(code.count).count) * 8 + 28) }
  override var isFlipped: Bool { true }
  override init(frame: NSRect) {
    super.init(frame: frame)
    wantsLayer = true
    canvas.wantsLayer = true
    canvas.owner = self
    addSubview(canvas)
    for scroller in [vertical, horizontal] {
      scroller.target = self
      scroller.action = #selector(scrollbar(_:))
      scroller.controlSize = .small
      addSubview(scroller)
    }
    cache.countLimit = 160
    cache.totalCostLimit = 2 * 1024 * 1024
    canvas.setAccessibilityElement(true)
    canvas.setAccessibilityRole(.textArea)
    canvas.setAccessibilityLabel("JSON viewer")
  }
  convenience init() { self.init(frame: .zero) }
  required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
  func clear() {
    code = []
    treeRows = []
    document = ""
    visible = []
    matches = []
    active = -1
    codeClosed = []
    treeClosed = []
    selectionAnchor = nil
    selectionEnd = nil
    x = 0
    y = 0
    contentWidth = 0
    cache.removeAllObjects()
    needsLayout = true
    canvas.needsDisplay = true
    canvas.setAccessibilityValue("")
  }
  func load(_ args: [String: Any]) {
    clear()
    document = (args["text"] as? String ?? "") as NSString
    code = (args["code"] as? [[String: Any]] ?? []).map(JsonRow.init)
    treeRows = (args["tree"] as? [[String: Any]] ?? []).map(JsonRow.init)
    for (i, row) in treeRows.enumerated()
    where row.fold > i && !(args["expanded"] as? Bool ?? true) {
      treeClosed.insert(i)
    }
    rebuild()
  }
  func configure(_ args: [String: Any]) {
    let nextTree = args["tree"] as? Bool ?? false
    if nextTree != tree {
      tree = nextTree
      x = 0
      y = 0
      selectionAnchor = nil
      selectionEnd = nil
    }
    countPattern = args["countPattern"] as? String ?? "{count} items"
    palette = args["palette"] as? [String: Any] ?? [:]
    let scale = max(0.5, min(3, args["fontScale"] as? CGFloat ?? 1))
    font = NSFont.monospacedSystemFont(ofSize: 12 * scale, weight: .regular)
    lineHeight = 22 * scale
    cache.removeAllObjects()
    rebuild()
    reveal()
    canvas.setAccessibilityLabel(tree ? "JSON tree viewer" : "JSON viewer")
  }
  func setMatches(_ ranges: [[Int]]) {
    matches = ranges.filter {
      $0.count == 2 && $0[0] >= 0 && $0[1] <= document.length && $0[1] > $0[0]
    }
    active = -1
    canvas.needsDisplay = true
  }
  func setActive(_ index: Int) {
    active = index >= 0 && index < matches.count ? index : -1
    reveal()
    canvas.needsDisplay = true
  }
  private func rebuild() {
    visible = []
    var index = 0
    var columns = 0
    let values = rows
    let folded = closed
    while index < values.count {
      visible.append(index)
      let row = values[index]
      columns = max(columns, row.columns + (tree && row.count > 0 ? 18 : 0) + (tree ? row.depth * 4 : 0) + 8)
      index = folded.contains(index) && row.fold > index ? row.fold + 1 : index + 1
    }
    // Conservative display columns include wide Unicode characters and swatches.
    contentWidth =
      gutter + CGFloat(columns) * ("M" as NSString).size(withAttributes: [.font: font]).width
    clamp()
    needsLayout = true
    canvas.needsDisplay = true
  }
  override func layout() {
    super.layout()
    viewport = NSRect(
      x: 0, y: 0, width: max(0, bounds.width - 12), height: max(0, bounds.height - 12))
    canvas.frame = viewport
    vertical.frame = NSRect(x: viewport.width, y: 0, width: 12, height: viewport.height)
    horizontal.frame = NSRect(x: 0, y: viewport.height, width: viewport.width, height: 12)
    clamp()
    canvas.needsDisplay = true
  }
  private func clamp() {
    let height = CGFloat(visible.count) * lineHeight + 12
    x = min(max(0, x), max(0, contentWidth - viewport.width))
    y = min(max(0, y), max(0, height - viewport.height))
    vertical.isEnabled = height > viewport.height
    horizontal.isEnabled = contentWidth > viewport.width
    vertical.knobProportion = min(1, viewport.height / max(1, height))
    horizontal.knobProportion = min(1, viewport.width / max(1, contentWidth))
    vertical.doubleValue = Double(y / max(1, height - viewport.height))
    horizontal.doubleValue = Double(x / max(1, contentWidth - viewport.width))
  }
  @objc private func scrollbar(_ sender: NSScroller) {
    let v = sender === vertical
    let page = v ? viewport.height : viewport.width
    let extent = v ? CGFloat(visible.count) * lineHeight + 12 : contentWidth
    var pos = v ? y : x
    switch sender.hitPart {
    case .decrementLine: pos -= lineHeight
    case .incrementLine: pos += lineHeight
    case .decrementPage: pos -= page
    case .incrementPage: pos += page
    default: pos = CGFloat(sender.doubleValue) * max(0, extent - page)
    }
    if v { y = pos } else { x = pos }
    clamp()
    canvas.needsDisplay = true
  }
  fileprivate func wheel(_ event: NSEvent) {
    let factor: CGFloat = event.hasPreciseScrollingDeltas ? 1 : lineHeight
    if event.modifierFlags.contains(.shift) && event.scrollingDeltaX == 0 {
      x -= event.scrollingDeltaY * factor
    } else {
      x -= event.scrollingDeltaX * factor
      y -= event.scrollingDeltaY * factor
    }
    clamp()
    canvas.needsDisplay = true
  }
  private func color(_ name: String) -> NSColor { jsonColor(palette[name]) }
  private func attributed(_ index: Int) -> NSAttributedString {
    if let value = cache.object(forKey: NSNumber(value: index)) { return value }
    let row = rows[index]
    let value = NSMutableAttributedString(
      string: row.text as String, attributes: [.font: font, .foregroundColor: color("text")])
    for span in row.spans {
      guard span.count == 3, let start = span[0] as? Int, let length = span[1] as? Int,
        let name = span[2] as? String,
        start >= 0, length >= 0, start + length <= value.length
      else { continue }
      let range = NSRange(location: start, length: length)
      value.addAttribute(.foregroundColor, value: color(name == "keyStrong" ? "key" : name), range: range)
      if tree && name == "keyStrong" {
        value.addAttribute(.font, value: NSFont.monospacedSystemFont(ofSize: font.pointSize, weight: .semibold), range: range)
      }
    }
    if tree && row.count > 0 {
      value.append(NSAttributedString(string: "  " + countPattern.replacingOccurrences(of: "{count}", with: String(row.count)),
        attributes: [.font: font, .foregroundColor: color("muted")]))
    }
    cache.setObject(value, forKey: NSNumber(value: index), cost: value.length * 8)
    return value
  }
  private func prefixWidth(_ row: JsonRow, _ length: Int) -> CGFloat {
    row.text.substring(to: max(0, min(length, row.text.length))).size(withAttributes: [.font: font])
      .width
  }
  private var characterWidth: CGFloat { (" " as NSString).size(withAttributes: [.font: font]).width }
  private func textX(_ row: JsonRow) -> CGFloat {
    gutter + (tree ? CGFloat(row.depth * 4 + 2) * characterWidth : 0) - x
  }
  private func branches(_ row: JsonRow, top: CGFloat) {
    guard tree && row.depth > 0 else { return }
    let path = NSBezierPath()
    path.lineWidth = 1
    func columnX(_ column: Int) -> CGFloat { gutter + (CGFloat(column * 4) + 0.5) * characterWidth - x }
    for (column, continues) in row.branches.enumerated() where continues {
      let left = columnX(column)
      path.move(to: NSPoint(x: left, y: top)); path.line(to: NSPoint(x: left, y: top + lineHeight))
    }
    let left = columnX(row.depth - 1), middle = top + lineHeight / 2
    path.move(to: NSPoint(x: left, y: top))
    path.line(to: NSPoint(x: left, y: row.last ? middle : top + lineHeight))
    path.move(to: NSPoint(x: left, y: middle))
    path.line(to: NSPoint(x: left + 2.5 * characterWidth, y: middle))
    color("muted").setStroke(); path.stroke()
  }
  private func highlights(_ row: JsonRow, start: Int, end: Int) -> [NSRange] {
    row.maps.compactMap { map in
      guard map.count == 3 else { return nil }
      let a = max(start, map[2])
      let b = min(end, map[2] + map[1])
      return b > a ? NSRange(location: map[0] + a - map[2], length: b - a) : nil
    }
  }
  fileprivate func paint(_ dirty: NSRect) {
    color("surface").setFill()
    dirty.fill()
    let values = rows
    let first = max(0, Int((y + dirty.minY - 6) / lineHeight))
    let last = min(visible.count, Int(ceil((y + dirty.maxY) / lineHeight)) + 1)
    guard first < last else { return }
    let a = selectionAnchor
    let b = selectionEnd
    for position in first..<last {
      let index = visible[position]
      let row = values[index]
      let top = 6 + CGFloat(position) * lineHeight - y
      let left = textX(row)
      branches(row, top: top)
      let rendered = NSMutableAttributedString(attributedString: attributed(index))
      func mark(_ range: NSRange, _ background: NSColor, _ foreground: NSColor) {
        guard NSMaxRange(range) <= rendered.length else { return }
        rendered.addAttributes(
          [.backgroundColor: background, .foregroundColor: foreground], range: range)
      }
      // Find the first overlapping range without scanning earlier matches.
      for mapping in row.maps where mapping.count == 3 {
        let searchStart = mapping[2]
        let searchEnd = mapping[2] + mapping[1]
        var low = 0
        var high = matches.count
        while low < high {
          let mid = (low + high) / 2
          if matches[mid][1] <= searchStart { low = mid + 1 } else { high = mid }
        }
        while low < matches.count && matches[low][0] < searchEnd {
          for range in highlights(
            row, start: max(searchStart, matches[low][0]), end: min(searchEnd, matches[low][1]))
          {
            mark(range, low == active ? NSColor.systemOrange : NSColor.systemYellow, NSColor.black)
          }
          low += 1
        }
      }
      if let a = a, let b = b {
        for range in highlights(row, start: min(a, b), end: max(a, b)) {
          mark(range, color("selected"), color("selectedText"))
        }
      }
      rendered.draw(at: NSPoint(x: left, y: top + 2))
      if !tree {
        color("surface").setFill()
        NSRect(x: 0, y: top, width: max(0, gutter - x), height: lineHeight).fill()
        (String(index + 1) as NSString).draw(
          at: NSPoint(x: 8 - x, y: top + 2),
          withAttributes: [.font: font, .foregroundColor: color("muted")])
      }
      if row.fold > index {
        let symbol = closed.contains(index) ? "▸" : "▾"
        (symbol as NSString).draw(
          at: NSPoint(x: tree ? left - 2 * characterWidth : gutter - 18 - x, y: top + 2),
          withAttributes: [.font: font, .foregroundColor: color(tree ? "accent" : "muted")])
        if !tree && closed.contains(index) {
          (" …" as NSString).draw(
            at: NSPoint(x: left + rendered.size().width, y: top + 2),
            withAttributes: [.font: font, .foregroundColor: color("muted")])
        }
      }
      if let swatch = row.swatch {
        let r = NSRect(x: left + rendered.size().width + 8, y: top + 4, width: 12, height: 12)
        NSColor.white.setFill()
        r.fill()
        swatch.setFill()
        r.fill()
        color("line").setStroke()
        NSBezierPath(rect: r).stroke()
      }
    }
  }
  private func reveal() {
    guard active >= 0 && active < matches.count else { return }
    let match = matches[active]
    let values = rows
    guard !values.isEmpty else { return }
    var low = 0
    var high = values.count
    while low < high {
      let mid = (low + high) / 2
      if values[mid].start <= match[0] { low = mid + 1 } else { high = mid }
    }
    var target = max(0, low - 1)
    if tree { while target > 0 && values[target].end <= match[0] { target -= 1 } }
    var changed = false
    for index in closed {
      let row = values[index]
      let end = tree ? row.end : values[row.fold].end
      if row.start <= match[0] && end > match[0] {
        if tree { treeClosed.remove(index) } else { codeClosed.remove(index) }
        changed = true
      }
    }
    if changed { rebuild() }
    if let position = visible.firstIndex(of: target) {
      y = max(0, CGFloat(position) * lineHeight - viewport.height * 0.35)
      let row = rows[target]
      let range = highlights(row, start: match[0], end: match[1]).first
      x = max(
        0,
        textX(row) + x + prefixWidth(row, range?.location ?? 0)
          - viewport.width * 0.35)
      clamp()
    }
  }
  private func hit(_ point: NSPoint) -> (Int, JsonRow)? {
    let position = Int(floor((point.y + y - 6) / lineHeight))
    guard position >= 0 && position < visible.count else { return nil }
    let index = visible[position]
    return (index, rows[index])
  }
  private func offset(_ point: NSPoint, row: JsonRow) -> Int {
    let local = max(0, point.x - textX(row))
    var low = 0
    var high = row.text.length
    while low < high {
      let mid = (low + high) / 2
      if prefixWidth(row, mid) < local { low = mid + 1 } else { high = mid }
    }
    // Never split a UTF-16 surrogate pair or combining character when copying.
    if low < row.text.length { low = row.text.rangeOfComposedCharacterSequence(at: low).location }
    for map in row.maps where map.count == 3 && low >= map[0] && low <= map[0] + map[1] {
      return map[2] + low - map[0]
    }
    return low == 0 ? row.start : min(row.end, document.length)
  }
  fileprivate func down(_ event: NSEvent) {
    window?.makeFirstResponder(canvas)
    let point = canvas.convert(event.locationInWindow, from: nil)
    guard let (index, row) = hit(point) else { return }
    let foldX = tree ? textX(row) - 2 * characterWidth : gutter - 20 - x
    if row.fold > index && point.x >= foldX && point.x <= (tree ? textX(row) + attributed(index).size().width : foldX + 20) {
      if tree {
        if treeClosed.contains(index) { treeClosed.remove(index) } else { treeClosed.insert(index) }
      } else {
        if codeClosed.contains(index) { codeClosed.remove(index) } else { codeClosed.insert(index) }
      }
      rebuild()
      return
    }
    let pos = offset(point, row: row)
    if !event.modifierFlags.contains(.shift) { selectionAnchor = pos }
    selectionEnd = pos
    if event.clickCount >= 2 {
      selectionAnchor = row.start
      selectionEnd = tree ? row.end : row.start + row.text.length
    }
    canvas.needsDisplay = true
  }
  fileprivate func drag(_ event: NSEvent) {
    guard selectionAnchor != nil else { return }
    let point = canvas.convert(event.locationInWindow, from: nil)
    if point.y < 0 { y -= lineHeight } else if point.y > viewport.height { y += lineHeight }
    clamp()
    if let (_, row) = hit(NSPoint(x: point.x, y: min(max(6, point.y), max(6, viewport.height - 1))))
    {
      selectionEnd = offset(point, row: row)
    }
    canvas.needsDisplay = true
  }
  fileprivate func key(_ event: NSEvent) -> Bool {
    let command = event.modifierFlags.contains(.command)
    if command && event.charactersIgnoringModifiers == "a" {
      selectionAnchor = 0
      selectionEnd = document.length
      canvas.needsDisplay = true
      return true
    }
    if command && event.charactersIgnoringModifiers == "c" {
      copySelection()
      return true
    }
    if command && event.charactersIgnoringModifiers == "f" {
      emit?("find", [:])
      return true
    }
    if event.keyCode == 99 || (command && event.charactersIgnoringModifiers == "g") {
      emit?("next", ["delta": event.modifierFlags.contains(.shift) ? -1 : 1])
      return true
    }
    switch event.keyCode {
    case 125: y += lineHeight
    case 126: y -= lineHeight
    case 121: y += viewport.height
    case 116: y -= viewport.height
    case 115: y = 0
    case 119: y = CGFloat(visible.count) * lineHeight
    case 123: x -= 40
    case 124: x += 40
    case 48:
      window?.selectNextKeyView(canvas)
      return true
    default: return false
    }
    clamp()
    canvas.needsDisplay = true
    return true
  }
  func selectedText() -> String? {
    guard let a = selectionAnchor, let b = selectionEnd, a != b else { return nil }
    let start = max(0, min(a, b))
    let end = min(document.length, max(a, b))
    guard end > start else { return nil }
    return document.substring(with: NSRange(location: start, length: end - start))
  }
  func copySelection() {
    guard let text = selectedText() else { return }
    NSPasteboard.general.clearContents()
    NSPasteboard.general.setString(text, forType: .string)
  }
  fileprivate func accessibleText() -> String {
    let first = max(0, Int(y / lineHeight))
    let last = min(visible.count, Int(ceil((y + viewport.height) / lineHeight)))
    return first < last
      ? (first..<last).map { rows[visible[$0]].text as String }.joined(separator: "\n") : ""
  }
}
private final class JsonCanvas: NSView {
  weak var owner: NativeJsonHost?
  override var isFlipped: Bool { true }
  override var acceptsFirstResponder: Bool { true }
  override func draw(_ dirtyRect: NSRect) { owner?.paint(dirtyRect) }
  override func scrollWheel(with event: NSEvent) { owner?.wheel(event) }
  override func mouseDown(with event: NSEvent) { owner?.down(event) }
  override func mouseDragged(with event: NSEvent) { owner?.drag(event) }
  override func keyDown(with event: NSEvent) {
    if owner?.key(event) != true { super.keyDown(with: event) }
  }
  override func performKeyEquivalent(with event: NSEvent) -> Bool {
    if window?.firstResponder === self && event.modifierFlags.contains(.command) {
      return owner?.key(event) ?? false
    }
    return super.performKeyEquivalent(with: event)
  }
  override func accessibilityValue() -> Any? { owner?.accessibleText() }
  @objc func copy(_ sender: Any?) { owner?.copySelection() }
}
