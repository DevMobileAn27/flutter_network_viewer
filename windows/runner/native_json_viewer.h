#ifndef RUNNER_NATIVE_JSON_VIEWER_H_
#define RUNNER_NATIVE_JSON_VIEWER_H_
#include <flutter/binary_messenger.h>
#include <windows.h>
#include <memory>
class NativeJsonViewer {
 public:
  NativeJsonViewer(flutter::BinaryMessenger* messenger, HWND parent);
  ~NativeJsonViewer();
 private:
  class Impl;
  std::unique_ptr<Impl> impl_;
};
#endif
