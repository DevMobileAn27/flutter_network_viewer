import Cocoa
import Darwin
import FlutterMacOS

class MainFlutterWindow: NSWindow {
  private var nativeImage: NativeImageViewer?
  private var nativeJson: NativeJsonViewer?
  private var nativeLists: NativeCaptureLists?
  private var resourceChannel: FlutterMethodChannel?
  private let resourceSampler = ProcessResourceSampler()

  override func awakeFromNib() {
    let flutterViewController = FlutterViewController()
    let windowFrame = NSRect(x: self.frame.origin.x, y: self.frame.origin.y, width: 1360, height: 840)
    self.contentViewController = flutterViewController
    self.setFrame(windowFrame, display: true)
    self.minSize = NSSize(width: 860, height: 600)
    self.title = "Flutter Network Viewer"

    RegisterGeneratedPlugins(registry: flutterViewController)
    nativeLists = NativeCaptureLists(messenger: flutterViewController.engine.binaryMessenger)
    flutterViewController.registrar(forPlugin: "NativeCaptureLists").register(
      NativeCaptureListFactory(service: nativeLists!), withId: "dev.quocan.flutterNetworkViewer/capture_list")

    nativeJson = NativeJsonViewer(messenger: flutterViewController.engine.binaryMessenger)
    flutterViewController.registrar(forPlugin: "NativeJsonViewer").register(
      NativeJsonFactory(service: nativeJson!), withId: "dev.quocan.flutterNetworkViewer/json_view")

    nativeImage = NativeImageViewer(messenger: flutterViewController.engine.binaryMessenger)
    flutterViewController.registrar(forPlugin: "NativeImageViewer").register(
      NativeImageFactory(service: nativeImage!), withId: "dev.quocan.flutterNetworkViewer/image_view")

    resourceChannel = FlutterMethodChannel(
      name: "dev.quocan.flutterNetworkViewer/process_resources",
      binaryMessenger: flutterViewController.engine.binaryMessenger)
    resourceChannel?.setMethodCallHandler { [weak self] call, result in
      guard call.method == "sample" else {
        result(FlutterMethodNotImplemented)
        return
      }
      result(self?.resourceSampler.sample())
    }

    super.awakeFromNib()
  }
}

private class ProcessResourceSampler {
  func sample() -> [String: Any] {
    var values: [String: Any] = [:]
    var info = task_vm_info_data_t()
    var count = mach_msg_type_number_t(MemoryLayout<task_vm_info_data_t>.size / MemoryLayout<integer_t>.size)
    let status = withUnsafeMutablePointer(to: &info) { pointer in
      pointer.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
        task_info(mach_task_self_, task_flavor_t(TASK_VM_INFO), $0, &count)
      }
    }
    if status == KERN_SUCCESS {
      values["memoryBytes"] = info.phys_footprint
    }
    return values
  }
}
