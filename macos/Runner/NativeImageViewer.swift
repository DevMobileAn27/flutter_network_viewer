import Cocoa
import FlutterMacOS
import ImageIO

/// ImageIO decodes one still frame off the main thread. The 4096px cap bounds
/// the decoded bitmap to 64 MiB; zooming never allocates another bitmap.
func decodeNativeImage(_ data: Data) -> CGImage? {
  guard let source = CGImageSourceCreateWithData(data as CFData,
    [kCGImageSourceShouldCache: false] as CFDictionary) else { return nil }
  return CGImageSourceCreateThumbnailAtIndex(source, 0, [
    kCGImageSourceCreateThumbnailFromImageAlways: true,
    kCGImageSourceCreateThumbnailWithTransform: true,
    kCGImageSourceThumbnailMaxPixelSize: 4096,
    kCGImageSourceShouldCacheImmediately: true,
  ] as CFDictionary)
}

final class NativeImageAsset {
  var image: CGImage?
  var complete = false
  var work: DispatchWorkItem?
  let hosts = NSHashTable<NativeImageHost>.weakObjects()
  deinit { work?.cancel() }
}

final class NativeImageViewer {
  private let channel: FlutterMethodChannel
  private let queue = DispatchQueue(label: "viewer.image.decode", qos: .userInitiated)
  private var assets: [Int64: NativeImageAsset] = [:]
  private var hosts: [Int64: NativeImageHost] = [:]
  init(messenger: FlutterBinaryMessenger) {
    channel = FlutterMethodChannel(name: "dev.quocan.flutterNetworkViewer/native_image", binaryMessenger: messenger)
    channel.setMethodCallHandler { [weak self] call, result in
      guard let self = self else { result(nil); return }
      if call.method == "capabilities" { result(1); return }
      guard let args = call.arguments as? [String: Any],
        let id = (args["id"] as? NSNumber)?.int64Value else {
        result(FlutterError(code: "arguments", message: "Missing image ID", details: nil)); return
      }
      switch call.method {
      case "load":
        guard let bytes = args["bytes"] as? FlutterStandardTypedData else {
          result(FlutterError(code: "arguments", message: "Missing image bytes", details: nil)); return
        }
        let asset = NativeImageAsset()
        self.assets[id] = asset
        let data = bytes.data
        let work = DispatchWorkItem { [weak asset] in
          // Do not retain assets while waiting in the serial decode queue.
          guard asset != nil else { return }
          let image = autoreleasepool { decodeNativeImage(data) }
          DispatchQueue.main.async { [weak asset] in
            guard let asset = asset else { return }
            asset.image = image; asset.complete = true; asset.work = nil
            for host in asset.hosts.allObjects { host.imageChanged() }
          }
        }
        asset.work = work
        self.queue.async(execute: work)
      case "release": self.assets.removeValue(forKey: id)
      case "create", "bind":
        let host = self.view(id)
        guard let imageId = (args["image"] as? NSNumber)?.int64Value,
          let asset = self.assets[imageId] else {
          result(FlutterError(code: "image", message: "Image resource has been released", details: nil)); return
        }
        host.bind(asset)
      case "dispose":
        if let host = self.hosts.removeValue(forKey: id) { host.bind(nil); host.removeFromSuperview() }
      default:
        if let host = self.hosts[id] {
          switch call.method {
          case "configure": host.configure(args)
          case "control": host.control((args["factor"] as? NSNumber)?.doubleValue)
          case "frame": host.isHidden = !(args["visible"] as? Bool ?? false)
          default: result(FlutterMethodNotImplemented); return
          }
        }
      }
      result(nil)
    }
  }
  fileprivate func view(_ id: Int64) -> NativeImageHost {
    if let host = hosts[id] { return host }
    let host = NativeImageHost()
    host.emit = { [weak self] name in self?.channel.invokeMethod(name, arguments: ["id": id]) }
    hosts[id] = host
    return host
  }
}
final class NativeImageFactory: NSObject, FlutterPlatformViewFactory {
  let service: NativeImageViewer
  init(service: NativeImageViewer) { self.service = service }
  func createArgsCodec() -> (FlutterMessageCodec & NSObjectProtocol)? { FlutterStandardMessageCodec.sharedInstance() }
  func create(withViewIdentifier viewId: Int64, arguments args: Any?) -> NSView {
    service.view(((args as? [String: Any])?["id"] as? NSNumber)?.int64Value ?? viewId)
  }
}

final class NativeImageHost: NSView {
  var emit: ((String) -> Void)?
  private var asset: NativeImageAsset?
  private var background = NSColor.windowBackgroundColor
  private var interactive = false
  private(set) var zoom: CGFloat = 1
  private var pan = NSPoint.zero
  private var drag: NSPoint?
  private var errorSent = false
  private let toolbar = NSBox()
  private var tools: [NSButton] = []
  override var isFlipped: Bool { true }
  override var acceptsFirstResponder: Bool { true }
  override var isOpaque: Bool { true }
  override init(frame: NSRect) {
    super.init(frame: frame)
    setAccessibilityElement(true)
    setAccessibilityRole(.image)
    toolbar.boxType = .custom; toolbar.titlePosition = .noTitle
    toolbar.cornerRadius = 10; toolbar.borderWidth = 1
    toolbar.contentViewMargins = .zero
    addSubview(toolbar)
    let symbols = ["minus.magnifyingglass", "plus.magnifyingglass", "arrow.up.left.and.arrow.down.right", "xmark"]
    for (i, symbol) in symbols.enumerated() {
      let button = NSButton(title: ["−", "+", "⛶", "×"][i], target: self, action: #selector(tool(_:)))
      button.tag = i; button.isBordered = false
      if #available(macOS 11.0, *) {
        button.image = NSImage(systemSymbolName: symbol, accessibilityDescription: nil)
        button.imagePosition = .imageOnly
      }
      tools.append(button); toolbar.contentView?.addSubview(button)
    }
    toolbar.isHidden = true
  }
  required init?(coder: NSCoder) { fatalError("init(coder:) has not been implemented") }
  @objc private func tool(_ sender: NSButton) {
    switch sender.tag {
    case 0: control(0.8)
    case 1: control(1.25)
    case 2: control(nil)
    default: emit?("close")
    }
  }
  override func layout() {
    super.layout()
    let width = min(184, max(0, bounds.width - 32))
    toolbar.frame = NSRect(x: max(0, bounds.width - width - 16),
      y: max(0, bounds.height - 56), width: width, height: 40)
    for (i, button) in tools.enumerated() {
      button.frame = NSRect(x: CGFloat(i) * width / 4, y: 0, width: width / 4, height: 40)
    }
  }
  func bind(_ value: NativeImageAsset?) {
    asset?.hosts.remove(self)
    asset = value
    value?.hosts.add(self)
    zoom = 1; pan = .zero; errorSent = false
    imageChanged()
  }
  func imageChanged() {
    if asset?.complete == true && asset?.image == nil && !errorSent {
      errorSent = true
      emit?("error")
    }
    needsDisplay = true
  }
  func configure(_ args: [String: Any]) {
    interactive = args["interactive"] as? Bool ?? false
    let n = (args["background"] as? NSNumber)?.uint32Value ?? 0xfff0f0f0
    background = NSColor(srgbRed: CGFloat((n >> 16) & 255) / 255,
      green: CGFloat((n >> 8) & 255) / 255, blue: CGFloat(n & 255) / 255, alpha: 1)
    func color(_ key: String, _ fallback: UInt32) -> NSColor {
      let n = (args[key] as? NSNumber)?.uint32Value ?? fallback
      return NSColor(srgbRed: CGFloat((n >> 16) & 255) / 255,
        green: CGFloat((n >> 8) & 255) / 255, blue: CGFloat(n & 255) / 255, alpha: 1)
    }
    toolbar.fillColor = color("surface", 0xfffafafa)
    toolbar.borderColor = color("line", 0xffcccccc)
    let labels = args["tools"] as? [String] ?? ["Zoom out", "Zoom in", "Fit image", "Close"]
    for (i, button) in tools.enumerated() {
      let label = i < labels.count ? labels[i] : ""
      button.toolTip = label; button.setAccessibilityLabel(label)
      if #available(macOS 10.14, *) { button.contentTintColor = color("text", 0xff222222) }
    }
    setAccessibilityElement(!interactive)
    toolbar.isHidden = !interactive
    needsLayout = true
    setAccessibilityLabel(args["label"] as? String ?? "Image preview")
    needsDisplay = true
  }
  func control(_ factor: Double?) {
    guard let factor = factor else { zoom = 1; pan = .zero; needsDisplay = true; return }
    guard factor.isFinite && factor > 0 else { return }
    let next = min(10, max(0.1, zoom * CGFloat(factor)))
    let ratio = next / zoom
    pan.x *= ratio; pan.y *= ratio; zoom = next
    needsDisplay = true
  }
  override func draw(_ dirtyRect: NSRect) {
    background.setFill(); bounds.fill()
    guard let image = asset?.image, bounds.width > 0, bounds.height > 0 else { return }
    let w = CGFloat(image.width), h = CGFloat(image.height)
    let scale = min(bounds.width / w, bounds.height / h) * zoom
    let rect = NSRect(x: (bounds.width - w * scale) / 2 + pan.x,
      y: (bounds.height - h * scale) / 2 + pan.y, width: w * scale, height: h * scale)
    NSGraphicsContext.current?.imageInterpolation = .high
    NSImage(cgImage: image, size: NSSize(width: w, height: h)).draw(
      in: rect, from: .zero, operation: .sourceOver, fraction: 1, respectFlipped: true, hints: nil)
  }
  override func mouseDown(with event: NSEvent) {
    window?.makeFirstResponder(self)
    if interactive { drag = convert(event.locationInWindow, from: nil) }
  }
  override func mouseDragged(with event: NSEvent) {
    guard interactive, let previous = drag else { return }
    let next = convert(event.locationInWindow, from: nil)
    pan.x += next.x - previous.x; pan.y += next.y - previous.y
    drag = next; needsDisplay = true
  }
  override func mouseUp(with event: NSEvent) {
    drag = nil
    if !interactive { emit?("open") }
  }
  override func magnify(with event: NSEvent) {
    if interactive { control(Double(1 + event.magnification)) }
  }
  override func scrollWheel(with event: NSEvent) {
    guard interactive else { super.scrollWheel(with: event); return }
    if event.modifierFlags.contains(.command) { control(exp(Double(event.scrollingDeltaY) * 0.01)) }
    else { pan.x += event.scrollingDeltaX; pan.y += event.scrollingDeltaY; needsDisplay = true }
  }
  override func keyDown(with event: NSEvent) {
    guard interactive else { super.keyDown(with: event); return }
    switch event.charactersIgnoringModifiers {
    case "+", "=": control(1.25)
    case "-": control(0.8)
    case "0": control(nil)
    default: super.keyDown(with: event)
    }
  }
}
