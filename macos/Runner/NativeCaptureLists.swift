import Cocoa
import FlutterMacOS

/// Native child views share the Flutter content coordinates. Dart explicitly
/// suspends them for routes, menus and notifications so overlays remain usable.
final class NativeCaptureLists {
  private let channel: FlutterMethodChannel
  private var lists: [Int64: NativeCaptureListHost] = [:]

  init(messenger: FlutterBinaryMessenger) {
    channel = FlutterMethodChannel(
      name: "dev.quocan.flutterNetworkViewer/native_lists", binaryMessenger: messenger)
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
        result(FlutterError(code: "arguments", message: "Missing native list ID", details: nil))
        return
      }
      if call.method == "create" {
        _ = self.view(id: id, kind: args["kind"] as? String ?? "network")
      } else if call.method == "dispose" {
        self.lists.removeValue(forKey: id)?.removeFromSuperview()
      } else if let list = self.lists[id] {
        switch call.method {
        case "configure": list.configure(args)
        case "update": list.update(args)
        case "selection": list.select(args["key"] as? String, notify: false)
        case "frame":
          if !(args["visible"] as? Bool ?? false) { list.emitScrollPosition() }
          list.isHidden = !(args["visible"] as? Bool ?? false)
        default:
          result(FlutterMethodNotImplemented)
          return
        }
      }
      result(nil)
    }
  }
  fileprivate func view(id: Int64, kind: String) -> NativeCaptureListHost {
    if let list = lists[id] { return list }
    let list = NativeCaptureListHost(kind: kind)
    list.emit = { [weak self] method, fields in
      self?.channel.invokeMethod(method, arguments: fields.merging(["id": id]) { _, new in new })
    }
    lists[id] = list
    return list
  }

}

final class NativeCaptureListFactory: NSObject, FlutterPlatformViewFactory {
  let service: NativeCaptureLists
  init(service: NativeCaptureLists) { self.service = service }
  func createArgsCodec() -> (FlutterMessageCodec & NSObjectProtocol)? {
    FlutterStandardMessageCodec.sharedInstance()
  }
  func create(withViewIdentifier viewId: Int64, arguments args: Any?) -> NSView {
    let values = args as? [String: Any] ?? [:]
    let id = (values["id"] as? NSNumber)?.int64Value ?? viewId
    return service.view(id: id, kind: values["kind"] as? String ?? "network")
  }
}

private final class CaptureSummary {
  let data: [String: Any]
  var measuredWidth: CGFloat = -1
  var measuredHeight: CGFloat = 36
  init(_ data: [String: Any]) { self.data = data }
  func text(_ name: String) -> String { data[name] as? String ?? "" }
  func color(_ name: String) -> NSColor { nativeColor(data[name]) }
}

private func nativeColor(_ value: Any?) -> NSColor {
  let n = (value as? NSNumber)?.uint32Value ?? 0xff00_0000
  return NSColor(
    srgbRed: CGFloat((n >> 16) & 255) / 255, green: CGFloat((n >> 8) & 255) / 255,
    blue: CGFloat(n & 255) / 255, alpha: CGFloat((n >> 24) & 255) / 255)
}

final class NativeCaptureListHost: NSView {
  let kind: String
  var emit: ((String, [String: Any]) -> Void)?
  private let vertical = NSScroller(frame: NSRect(x: 0, y: 0, width: 12, height: 100))
  private let horizontal = NSScroller(frame: NSRect(x: 0, y: 0, width: 100, height: 12))
  private var scrollX: CGFloat = 0
  private var scrollY: CGFloat = 0
  private var viewport = NSRect.zero
  private let canvas = CaptureCanvasView()
  private var rows: [String: CaptureSummary] = [:]
  private var order: [String] = []
  private var indices: [String: Int] = [:]
  private var offsets: [CGFloat] = [0]
  private var palette: [String: Any] = [:]
  private var titles: [String] = []
  private var widths: [CGFloat] = [64, 56, 108]
  private var configuredWidths: [CGFloat] = []
  private var selected: String?
  private var scrollTask: DispatchWorkItem?
  private var resizeTask: DispatchWorkItem?
  private var dragDivider: Int?
  private var lastDragX: CGFloat = 0
  private(set) var totalWidth: CGFloat = 0
  override var isFlipped: Bool { true }
  private let bodyFont = NSFont.monospacedSystemFont(ofSize: 12, weight: .regular)
  private let smallFont = NSFont.monospacedSystemFont(ofSize: 10, weight: .regular)
  private let badgeFont = NSFont.monospacedSystemFont(ofSize: 9, weight: .bold)

  init(kind: String) {
    self.kind = kind
    super.init(frame: .zero)
    wantsLayer = true
    canvas.wantsLayer = true
    canvas.owner = self
    addSubview(canvas)
    for scroller in [vertical, horizontal] {
      scroller.target = self
      scroller.action = #selector(scrollerChanged(_:))
      scroller.controlSize = .small
      addSubview(scroller)
    }
    canvas.setAccessibilityElement(true)
    canvas.setAccessibilityRole(.table)
    canvas.setAccessibilityLabel(kind == "network" ? "Network requests" : "Console logs")
  }
  required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
  deinit {
    resizeTask?.cancel()
    scrollTask?.cancel()
  }

  func emitScrollPosition() {
    emit?("scrolled", ["x": Double(scrollX), "y": Double(scrollY)])
  }
  private var contentHeight: CGFloat {
    kind == "network" ? CGFloat(order.count) * 52 : (offsets.last ?? 0)
  }
  private var visibleRowsRect: NSRect {
    NSRect(origin: NSPoint(x: scrollX, y: scrollY), size: viewport.size)
  }
  private func clampScroll() {
    scrollX = min(max(0, scrollX), max(0, totalWidth - viewport.width))
    scrollY = min(max(0, scrollY), max(0, contentHeight - viewport.height))
    vertical.isEnabled = contentHeight > viewport.height
    horizontal.isEnabled = totalWidth > viewport.width
    vertical.knobProportion = min(1, viewport.height / max(1, contentHeight))
    horizontal.knobProportion = min(1, viewport.width / max(1, totalWidth))
    vertical.doubleValue = Double(scrollY / max(1, contentHeight - viewport.height))
    horizontal.doubleValue = Double(scrollX / max(1, totalWidth - viewport.width))
  }
  private func setScroll(x: CGFloat, y: CGFloat) {
    scrollX = x
    scrollY = y
    clampScroll()
    canvas.needsDisplay = true
    window?.invalidateCursorRects(for: canvas)
    scrollTask?.cancel()
    let task = DispatchWorkItem { [weak self] in self?.emitScrollPosition() }
    scrollTask = task
    DispatchQueue.main.asyncAfter(deadline: .now() + 0.1, execute: task)
  }
  @objc private func scrollerChanged(_ sender: NSScroller) {
    let isVertical = sender === vertical
    let viewport = isVertical ? viewport.height : viewport.width
    let extent = isVertical ? contentHeight : totalWidth
    var value = isVertical ? scrollY : scrollX
    switch sender.hitPart {
    case .decrementLine: value -= 36
    case .incrementLine: value += 36
    case .decrementPage: value -= viewport
    case .incrementPage: value += viewport
    default: value = CGFloat(sender.doubleValue) * max(0, extent - viewport)
    }
    setScroll(x: isVertical ? scrollX : value, y: isVertical ? value : scrollY)
  }
  fileprivate func wheel(_ event: NSEvent) {
    let factor: CGFloat = event.hasPreciseScrollingDeltas ? 1 : 24
    var dx = event.scrollingDeltaX * factor
    var dy = event.scrollingDeltaY * factor
    if event.modifierFlags.contains(.shift) && dx == 0 {
      dx = dy
      dy = 0
    }
    setScroll(x: scrollX - dx, y: scrollY - dy)
  }

  func configure(_ args: [String: Any]) {
    palette = args["palette"] as? [String: Any] ?? [:]
    titles = args["titles"] as? [String] ?? []
    if let values = args["widths"] as? [NSNumber], values.count == 3 {
      let next = values.map { CGFloat(truncating: $0) }
      if next != configuredWidths {
        configuredWidths = next
        widths = zip(next, [CGFloat(64), 56, 96]).map { min(1600, max($0, $1)) }
      }
    }
    needsLayout = true
    canvas.needsDisplay = true
  }

  override func setFrameSize(_ newSize: NSSize) {
    guard newSize != frame.size else { return }
    super.setFrameSize(newSize)
    needsLayout = true
  }

  override func layout() {
    super.layout()
    let previousViewport = viewport
    let head: CGFloat = kind == "network" ? 30 : 0
    canvas.frame = bounds
    let previousWidth = totalWidth
    let bar: CGFloat = 12
    // The drawing layer is only as large as the viewport, independent of row
    // count. Native scrollers hold the position; no 50,000px backing texture.
    let bodyHeight = max(0, bounds.height - head)
    let needH = kind == "network" && widths.reduce(0, +) + 158 > bounds.width - bar
    let availableHeight = max(0, bodyHeight - (needH ? bar : 0))
    let needV = contentHeight > availableHeight
    let bodyWidth = max(0, bounds.width - (needV ? bar : 0))
    viewport = NSRect(x: 0, y: head, width: bodyWidth, height: availableHeight)
    vertical.frame = NSRect(x: bodyWidth, y: head, width: bar, height: availableHeight)
    horizontal.frame = NSRect(x: 0, y: head + availableHeight, width: bodyWidth, height: bar)
    vertical.isHidden = !needV
    horizontal.isHidden = !needH
    totalWidth = kind == "network" ? max(bodyWidth, widths.reduce(0, +) + 158) : bodyWidth
    if previousWidth != totalWidth && kind == "console" && !order.isEmpty {
      if previousWidth <= 1 {
        rebuildOffsets()
      } else {
        resizeTask?.cancel()
        let task = DispatchWorkItem { [weak self] in
          self?.rebuildOffsets()
          self?.needsLayout = true
          self?.canvas.needsDisplay = true
        }
        resizeTask = task
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.12, execute: task)
      }
    }
    clampScroll()
    if previousViewport != viewport || previousWidth != totalWidth {
      canvas.needsDisplay = true
    }
    window?.invalidateCursorRects(for: canvas)
  }

  func update(_ args: [String: Any]) {
    let oldY = scrollY
    let first = index(at: oldY)
    let anchor = first < order.count ? order[first] : nil
    let inset = first < order.count ? oldY - rowY(first) : 0
    for key in args["removed"] as? [String] ?? [] { rows.removeValue(forKey: key) }
    for row in args["rows"] as? [[String: Any]] ?? [] {
      if let key = row["key"] as? String { rows[key] = CaptureSummary(row) }
    }
    if let next = args["order"] as? [String] {
      order = next.filter { rows[$0] != nil }
      indices = Dictionary(uniqueKeysWithValues: order.enumerated().map { ($1, $0) })
    }
    rebuildOffsets()
    if oldY > 1, let anchor = anchor, let next = indices[anchor] {
      scrollY = max(0, rowY(next) + inset)
    } else if oldY <= 1 {
      scrollY = 0
    }
    clampScroll()
    needsLayout = true
    canvas.needsDisplay = true
  }

  private func rebuildOffsets() {
    offsets = [0]
    if kind == "console" {
      for key in order {
        guard let row = rows[key] else { continue }
        let width = max(16, totalWidth - messageX(row) - 8)
        if totalWidth > 1 && row.measuredWidth != width {
          row.measuredWidth = width
          let paragraph = NSMutableParagraphStyle()
          paragraph.lineBreakMode = .byWordWrapping
          let rect = (row.text("message") as NSString).boundingRect(
            with: NSSize(width: width, height: .greatestFiniteMagnitude),
            options: [.usesLineFragmentOrigin, .usesFontLeading],
            attributes: [.font: bodyFont, .paragraphStyle: paragraph])
          row.measuredHeight = max(36, ceil(rect.height) + 12)
        }
        offsets.append(offsets.last! + row.measuredHeight)
      }
    }
    clampScroll()
  }

  private func color(_ name: String) -> NSColor { nativeColor(palette[name]) }
  private func rowY(_ index: Int) -> CGFloat {
    kind == "network" ? CGFloat(index) * 52 : offsets[min(index, offsets.count - 1)]
  }
  private func rowHeight(_ index: Int) -> CGFloat {
    kind == "network" ? 52 : offsets[index + 1] - offsets[index]
  }
  private func index(at y: CGFloat) -> Int {
    if kind == "network" { return min(order.count, max(0, Int(y / 52))) }
    var lo = 0
    var hi = order.count
    while lo < hi {
      let mid = (lo + hi) / 2
      if offsets[mid + 1] <= y { lo = mid + 1 } else { hi = mid }
    }
    return lo
  }
  private func columns() -> [CGFloat] {
    [widths[0], max(140, totalWidth - widths.reduce(0, +) - 18), widths[1], widths[2]]
  }
  private func boundaries() -> [CGFloat] {
    let c = columns()
    return [c[0], c[0] + c[1] + 6, c[0] + c[1] + c[2] + 12]
  }
  private func messageX(_ row: CaptureSummary) -> CGFloat {
    8 + badgeWidth(row.text("level")) + 6
      + (row.text("time") as NSString).size(withAttributes: [.font: smallFont]).width + 6
  }
  private func badgeWidth(_ text: String) -> CGFloat {
    ceil((text as NSString).size(withAttributes: [.font: badgeFont]).width) + 12
  }

  private func text(
    _ text: String, _ rect: NSRect, _ font: NSFont, _ color: NSColor, wrap: Bool = false,
    right: Bool = false
  ) {
    guard rect.width > 0, rect.height > 0 else { return }
    let p = NSMutableParagraphStyle()
    p.lineBreakMode = wrap ? .byWordWrapping : .byTruncatingTail
    p.alignment = right ? .right : .left
    (text as NSString).draw(
      with: rect, options: [.usesLineFragmentOrigin, .usesFontLeading],
      attributes: [.font: font, .foregroundColor: color, .paragraphStyle: p])
  }
  private func badge(
    _ value: String, _ x: CGFloat, _ y: CGFloat, _ available: CGFloat, _ fill: NSColor,
    _ selected: Bool
  ) {
    let rect = NSRect(x: x, y: y, width: max(0, min(available, badgeWidth(value))), height: 20)
    fill.setFill()
    NSBezierPath(roundedRect: rect, xRadius: 5, yRadius: 5).fill()
    if selected {
      NSColor.white.withAlphaComponent(0.5).setStroke()
      NSBezierPath(roundedRect: rect.insetBy(dx: 0.5, dy: 0.5), xRadius: 5, yRadius: 5).stroke()
    }
    let textWidth = (value as NSString).size(withAttributes: [.font: badgeFont]).width
    let fitted = NSFont.monospacedSystemFont(
      ofSize: badgeFont.pointSize * min(1, max(1, rect.width - 10) / max(1, textWidth)),
      weight: .bold)
    text(value, rect.insetBy(dx: 5, dy: 3), fitted, .white)
  }

  fileprivate func isHeader(_ y: CGFloat) -> Bool { kind == "network" && y < viewport.minY }
  fileprivate func drawCanvas(_ dirty: NSRect) {
    color("background").setFill()
    dirty.fill()
    let body = dirty.intersection(viewport)
    if !body.isEmpty {
      NSGraphicsContext.saveGraphicsState()
      body.clip()
      let transform = NSAffineTransform()
      transform.translateX(by: 0, yBy: viewport.minY)
      transform.concat()
      drawRows(body.offsetBy(dx: 0, dy: -viewport.minY))
      NSGraphicsContext.restoreGraphicsState()
    }
    if kind == "network" && dirty.minY < 30 {
      NSGraphicsContext.saveGraphicsState()
      NSRect(x: 0, y: 0, width: bounds.width, height: 30).clip()
      drawHeader(dirty.intersection(NSRect(x: 0, y: 0, width: bounds.width, height: 30)))
      NSGraphicsContext.restoreGraphicsState()
    }
  }

  fileprivate func drawRows(_ dirty: NSRect) {
    color("background").setFill()
    dirty.fill()
    NSGraphicsContext.saveGraphicsState()
    let transform = NSAffineTransform()
    transform.translateX(by: -scrollX, yBy: -scrollY)
    transform.concat()
    defer { NSGraphicsContext.restoreGraphicsState() }
    let dirty = dirty.offsetBy(dx: scrollX, dy: scrollY)
    let start = index(at: dirty.minY)
    let end = min(order.count, index(at: dirty.maxY) + 1)
    guard start < end else { return }
    for i in start..<end {
      guard let row = rows[order[i]] else { continue }
      let y = rowY(i)
      let h = rowHeight(i)
      let chosen = selected == order[i]
      let fg = color(chosen ? "selectedText" : "text")
      let muted = color(chosen ? "selectedMuted" : "muted")
      if chosen {
        color("selected").setFill()
        NSRect(x: 0, y: y, width: totalWidth, height: h).fill()
        color("marker").setFill()
        NSRect(x: 0, y: y, width: 4, height: h).fill()
      }
      color("line").setFill()
      NSRect(x: 0, y: y + h - 1, width: totalWidth, height: 1).fill()
      if kind == "network" {
        let c = columns()
        let statusX = c[0] + c[1] + 12
        let timeX = statusX + c[2] + 6
        badge(row.text("method"), 8, y + 16, c[0] - 16, row.color("methodColor"), chosen)
        text(
          row.text("path"), NSRect(x: c[0] + 18, y: y + 8, width: c[1] - 24, height: 17), bodyFont,
          fg)
        text(
          row.text("meta"), NSRect(x: c[0] + 18, y: y + 29, width: c[1] - 24, height: 15),
          smallFont, muted)
        badge(row.text("status"), statusX + 8, y + 16, c[2] - 16, row.color("statusColor"), chosen)
        text(
          row.text("duration"), NSRect(x: timeX + 12, y: y + 18, width: c[3] - 24, height: 17),
          smallFont, muted, right: true)
      } else {
        let bx = badgeWidth(row.text("level"))
        let mx = messageX(row)
        badge(row.text("level"), 8, y + 6, bx, row.color("levelColor"), chosen)
        text(
          row.text("time"), NSRect(x: 8 + bx + 6, y: y + 8, width: mx - bx - 20, height: 16),
          smallFont, muted)
        text(
          row.text("message"),
          NSRect(x: mx, y: y + 6, width: max(16, totalWidth - mx - 8), height: h - 12), bodyFont,
          chosen ? fg : row.color("messageColor"), wrap: true)
      }
    }
    if kind == "network" {
      color("line").setFill()
      for x in boundaries() {
        NSRect(x: x + 2, y: dirty.minY, width: 1, height: dirty.height).fill()
      }
    }
  }

  fileprivate func drawHeader(_ dirty: NSRect) {
    color("surface").setFill()
    dirty.fill()
    let c = columns()
    let origin = scrollX
    var x: CGFloat = -origin
    for i in 0..<min(4, titles.count) {
      text(
        titles[i], NSRect(x: x + (i == 0 || i == 2 ? 8 : 12), y: 8, width: c[i] - 20, height: 16),
        smallFont, color("muted"), right: i == 3)
      x += c[i] + 6
    }
    color("line").setFill()
    for edge in boundaries() { NSRect(x: edge + 2 - origin, y: 0, width: 1, height: 30).fill() }
  }

  func select(_ key: String?, notify: Bool) {
    let old = selected
    selected = key
    for key in [old, key].compactMap({ $0 }) {
      if let i = indices[key] {
        canvas.setNeedsDisplay(
          NSRect(
            x: 0, y: rowY(i) - scrollY + viewport.minY, width: viewport.width, height: rowHeight(i)
          ).intersection(viewport))
      }
    }
    if notify, let key = key { emit?("selected", ["key": key]) }
  }
  fileprivate func mouseDown(_ event: NSEvent, in view: NSView, header: Bool) {
    window?.makeFirstResponder(canvas)
    let point = view.convert(event.locationInWindow, from: nil)
    let x = point.x + scrollX
    if kind == "network", let divider = boundaries().firstIndex(where: { abs($0 + 3 - x) <= 4 }) {
      dragDivider = divider
      lastDragX = event.locationInWindow.x
      return
    }
    if !header {
      let i = index(at: point.y - viewport.minY + scrollY)
      if i < order.count { select(order[i], notify: true) }
    }
  }
  fileprivate func handleDrag(_ event: NSEvent) {
    guard let divider = dragDivider else { return }
    let delta = event.locationInWindow.x - lastDragX
    lastDragX = event.locationInWindow.x
    let spare = max(0, columns()[1] - 140)
    if divider == 0 { widths[0] = min(min(1600, widths[0] + spare), max(64, widths[0] + delta)) }
    if divider == 1 { widths[1] = min(min(1600, widths[1] + spare), max(56, widths[1] - delta)) }
    if divider == 2 {
      let sum = widths[1] + widths[2]
      widths[1] = min(min(1600, sum - 96), max(max(56, sum - 1600), widths[1] + delta))
      widths[2] = sum - widths[1]
    }
    needsLayout = true
    layoutSubtreeIfNeeded()
    // Column geometry can change while the viewport size stays the same.
    canvas.needsDisplay = true
  }
  fileprivate func mouseUp() {
    if dragDivider != nil {
      dragDivider = nil
      emit?("widthsChanged", ["widths": widths.map(Double.init)])
    }
  }
  fileprivate func cursorRects(_ view: NSView, header: Bool) {
    guard kind == "network" else { return }
    let origin = scrollX
    for edge in boundaries() {
      view.addCursorRect(
        NSRect(
          x: edge - origin - 1, y: view.visibleRect.minY, width: 8, height: view.visibleRect.height),
        cursor: .resizeLeftRight)
    }
  }
  fileprivate func handleKey(_ event: NSEvent) -> Bool {
    guard !order.isEmpty, !event.modifierFlags.contains(.command) else { return false }
    let current = selected.flatMap { indices[$0] } ?? -1
    let next: Int
    switch event.keyCode {
    case 125: next = min(order.count - 1, current + 1)
    case 126: next = max(0, current - 1)
    case 115: next = 0
    case 119: next = order.count - 1
    default: return false
    }
    select(order[next], notify: true)
    var y = scrollY
    if rowY(next) < y { y = rowY(next) }
    if rowY(next) + rowHeight(next) > y + viewport.height {
      y = rowY(next) + rowHeight(next) - viewport.height
    }
    setScroll(x: scrollX, y: y)
    return true
  }
  fileprivate func visibleAccessibilityRows() -> [Any] {
    let rect = visibleRowsRect
    let start = index(at: rect.minY)
    let end = min(order.count, index(at: rect.maxY) + 1)
    guard start < end else { return [] }
    return (start..<end).compactMap { i -> CaptureAccessibilityRow? in
      guard let row = rows[order[i]], let window = window else { return nil }
      let element = CaptureAccessibilityRow()
      element.owner = self
      element.key = order[i]
      element.setAccessibilityRole(.row)
      element.setAccessibilityLabel(
        kind == "network"
          ? "\(row.text("method")) \(row.text("path")) \(row.text("status"))"
          : "\(row.text("level")) \(row.text("time")) \(row.text("message"))")
      element.setAccessibilitySelected(selected == order[i])
      element.setAccessibilityParent(canvas)
      element.setAccessibilityFrame(
        window.convertToScreen(
          canvas.convert(
            NSRect(
              x: 0, y: rowY(i) - scrollY + viewport.minY, width: viewport.width,
              height: rowHeight(i)
            ).intersection(viewport), to: nil)))
      return element
    }
  }
}

private final class CaptureCanvasView: NSView {
  weak var owner: NativeCaptureListHost?
  override var isFlipped: Bool { true }
  override var acceptsFirstResponder: Bool { true }
  override func draw(_ dirtyRect: NSRect) { owner?.drawCanvas(dirtyRect) }
  override func scrollWheel(with event: NSEvent) { owner?.wheel(event) }
  override func mouseDown(with event: NSEvent) {
    let point = convert(event.locationInWindow, from: nil)
    owner?.mouseDown(event, in: self, header: owner?.isHeader(point.y) ?? false)
  }
  override func mouseDragged(with event: NSEvent) { owner?.handleDrag(event) }
  override func mouseUp(with event: NSEvent) { owner?.mouseUp() }
  override func resetCursorRects() { owner?.cursorRects(self, header: true) }
  override func keyDown(with event: NSEvent) {
    if owner?.handleKey(event) != true { super.keyDown(with: event) }
  }
  override func accessibilityChildren() -> [Any]? { owner?.visibleAccessibilityRows() }
  override func accessibilityRows() -> [Any]? { owner?.visibleAccessibilityRows() }
}
private final class CaptureAccessibilityRow: NSAccessibilityElement {
  weak var owner: NativeCaptureListHost?
  var key = ""
  override func accessibilityPerformPress() -> Bool {
    owner?.select(key, notify: true)
    return true
  }
}
