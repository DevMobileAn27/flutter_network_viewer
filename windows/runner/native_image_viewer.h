#ifndef RUNNER_NATIVE_IMAGE_VIEWER_H_
#define RUNNER_NATIVE_IMAGE_VIEWER_H_
#include <flutter/binary_messenger.h>
#include <windows.h>
#include <memory>
class NativeImageViewer {
 public:
  NativeImageViewer(flutter::BinaryMessenger* messenger, HWND parent);
  ~NativeImageViewer();
 private:
  class Impl;
  std::unique_ptr<Impl> impl_;
};
#endif
