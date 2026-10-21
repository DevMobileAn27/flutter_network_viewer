import Cocoa
import Darwin
import FlutterMacOS

class MainFlutterWindow: NSWindow {
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
  private var previousCPUTime: Double?
  private var previousSampleTime: Double?

  func sample() -> [String: Any] {
    var values: [String: Any] = [:]
    var usage = rusage()
    if getrusage(RUSAGE_SELF, &usage) == 0 {
      let cpuTime = Double(usage.ru_utime.tv_sec + usage.ru_stime.tv_sec)
        + Double(usage.ru_utime.tv_usec + usage.ru_stime.tv_usec) / 1_000_000
      let now = ProcessInfo.processInfo.systemUptime
      if let previousCPUTime = previousCPUTime,
        let previousSampleTime = previousSampleTime, now > previousSampleTime
      {
        // Like Activity Monitor, 100% means one fully occupied CPU core.
        values["cpuPercent"] = max(0, (cpuTime - previousCPUTime) / (now - previousSampleTime) * 100)
      }
      previousCPUTime = cpuTime
      previousSampleTime = now
    }

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
