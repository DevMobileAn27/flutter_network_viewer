import Cocoa
import FlutterMacOS
import ImageIO

func check(_ condition: @autoclosure () -> Bool, _ message: String) {
  if !condition() { fatalError(message) }
}
_ = NSApplication.shared
let png = decodeNativeImage(try! Data(contentsOf: URL(fileURLWithPath: "test/fixtures/preview.png")))
let jpg = decodeNativeImage(try! Data(contentsOf: URL(fileURLWithPath: "test/fixtures/preview.jpg")))
check(png != nil && jpg != nil, "Decode PNG and JPEG without Flutter")
check(decodeNativeImage(Data([0xff, 0xd8, 0xff])) == nil, "Corrupt JPEG fails safely")
check(decodeNativeImage(Data("not an image".utf8)) == nil, "Invalid bytes fail safely")

// Generate a wide image to verify downsampling before it becomes a CGImage.
let context = CGContext(data: nil, width: 6000, height: 120, bitsPerComponent: 8, bytesPerRow: 0,
  space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue)!
context.setFillColor(NSColor.red.cgColor)
context.fill(CGRect(x: 0, y: 0, width: 6000, height: 120))
let encoded = NSMutableData()
let destination = CGImageDestinationCreateWithData(encoded, "public.png" as CFString, 1, nil)!
CGImageDestinationAddImage(destination, context.makeImage()!, nil)
check(CGImageDestinationFinalize(destination), "Encode fixture")
let limited = decodeNativeImage(encoded as Data)!
check(limited.width == 4096 && limited.height < 120, "Large images are downsampled preserving aspect")

let asset = NativeImageAsset(); asset.image = limited; asset.complete = true
let host = NativeImageHost(frame: NSRect(x: 0, y: 0, width: 400, height: 180))
let dialog = NativeImageHost(frame: host.frame)
host.bind(asset); dialog.bind(asset)
check(asset.hosts.allObjects.count == 2, "Preview and dialog share decoded resource")
host.configure(["background": 0xffee_eeee, "interactive": false, "label": "Ảnh 😀"])
let window = NSWindow(contentRect: host.frame, styleMask: [.titled], backing: .buffered, defer: false)
window.contentView = host
func snapshot() -> NSBitmapImageRep {
  let rep = host.bitmapImageRepForCachingDisplay(in: host.bounds)!
  host.cacheDisplay(in: host.bounds, to: rep)
  return rep
}
let rep = snapshot()
let center = rep.colorAt(x: rep.pixelsWide / 2, y: rep.pixelsHigh / 2)!.usingColorSpace(.deviceRGB)!
check(center.redComponent > 0.9 && center.greenComponent < 0.2, "Native viewport paints image pixels")
check(host.accessibilityLabel() == "Ảnh 😀", "Native image is accessible")
host.configure(["interactive": true, "background": 0xffeeeeee,
  "tools": ["Thu nhỏ", "Phóng to", "Vừa khung", "Đóng"]])
host.layoutSubtreeIfNeeded()
let bar = host.subviews.first!
check(!bar.isHidden && abs(bar.frame.maxX - host.bounds.width + 16) < 1 &&
  abs(bar.frame.maxY - host.bounds.height + 16) < 1, "Toolbar floats at bottom right")
let buttons = (bar as! NSBox).contentView!.subviews.compactMap { $0 as? NSButton }
check(buttons.count == 4 && buttons[1].toolTip == "Phóng to", "Toolbar has localized native controls")
buttons[1].performClick(nil)
check(host.zoom == 1.25, "Native toolbar zooms image")
buttons[2].performClick(nil)
check(host.zoom == 1, "Native toolbar fits image")
var closed = false
host.emit = { if $0 == "close" { closed = true } }
buttons[3].performClick(nil)
check(closed, "Native toolbar closes dialog")
host.control(1.25); check(host.zoom == 1.25, "Zoom uses existing bitmap")
host.control(100); check(host.zoom == 10, "Zoom capped at 10x")
host.control(nil); check(host.zoom == 1, "Fit resets transform")
host.bind(nil); dialog.bind(nil)
check(asset.hosts.allObjects.isEmpty, "Dispose detaches both hosts")
var failed = 0
host.emit = { if $0 == "error" { failed += 1 } }
let invalid = NativeImageAsset(); invalid.complete = true
host.bind(invalid); host.imageChanged()
check(failed == 1, "Decode failure signals fallback once")
print("Native image AppKit tests passed")
