#ifndef RUNNER_NATIVE_CAPTURE_LISTS_H_
#define RUNNER_NATIVE_CAPTURE_LISTS_H_
#include <flutter/binary_messenger.h>
#include <windows.h>
#include <memory>

class NativeCaptureLists {
 public:
  NativeCaptureLists(flutter::BinaryMessenger* messenger, HWND parent);
  ~NativeCaptureLists();
 private:
  class Impl;
  std::unique_ptr<Impl> impl_;
};
#endif
