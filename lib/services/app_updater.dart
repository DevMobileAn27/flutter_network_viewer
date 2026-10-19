import 'dart:async';
import 'dart:ffi';
import 'dart:io';

import 'package:auto_updater/auto_updater.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

enum UpdateStatus { idle, checking, available, current, failed, installing }

/// Separate feeds prevent installing a build for another OS or CPU.
String? updateFeedFor(String operatingSystem, String abi) {
  final platform = switch ((operatingSystem, abi)) {
    ('macos', 'macos_arm64') => 'macos-arm64',
    ('macos', 'macos_x64') => 'macos-x86_64',
    ('windows', 'windows_x64') => 'windows-x64',
    _ => null,
  };
  return platform == null
      ? null
      : 'https://github.com/DevMobileAn27/flutter_network_viewer/releases/latest/download/appcast-$platform.xml';
}

abstract interface class UpdateBackend {
  void listen(UpdaterListener listener);
  void unlisten(UpdaterListener listener);
  Future<void> configure(String feed);
  Future<void> check({required bool background});
}

class SparkleUpdateBackend implements UpdateBackend {
  @override
  void listen(UpdaterListener listener) => autoUpdater.addListener(listener);
  @override
  void unlisten(UpdaterListener listener) =>
      autoUpdater.removeListener(listener);
  @override
  Future<void> configure(String feed) => autoUpdater.setFeedURL(feed);
  @override
  Future<void> check({required bool background}) =>
      autoUpdater.checkForUpdates(inBackground: background);
}

/// Sparkle/WinSparkle own the confirmation, download progress and installation.
/// No silent download or install is initiated by this controller.
class AppUpdater extends ChangeNotifier with UpdaterListener {
  AppUpdater({
    required UpdateBackend backend,
    required this.feed,
    required this.isWindows,
    Future<void> Function()? quitForUpdate,
  }) : _backend = backend,
       _quitForUpdate = quitForUpdate ?? SystemNavigator.pop;

  factory AppUpdater.desktop() => AppUpdater(
    backend: SparkleUpdateBackend(),
    feed: updateFeedFor(Platform.operatingSystem, Abi.current().toString()),
    isWindows: Platform.isWindows,
  );

  final UpdateBackend _backend;
  final String? feed;
  final bool isWindows;
  final Future<void> Function() _quitForUpdate;
  UpdateStatus status = UpdateStatus.idle;
  bool get supported => feed != null;
  bool get busy =>
      status == UpdateStatus.checking || status == UpdateStatus.installing;
  bool _configured = false;
  bool _listening = false;
  bool _background = false;
  bool _disposed = false;
  Timer? _timeout;

  Future<void> check({bool background = false}) async {
    if (!supported || busy || _disposed) return;
    _background = background;
    _setStatus(UpdateStatus.checking);
    _timeout = Timer(const Duration(seconds: 60), () {
      _setStatus(UpdateStatus.failed);
    });
    try {
      if (!_listening) {
        _backend.listen(this);
        _listening = true;
      }
      if (!_configured) {
        await _backend.configure(feed!);
        _configured = true;
      }
      if (!_disposed) await _backend.check(background: background);
    } catch (_) {
      _setStatus(UpdateStatus.failed);
    }
  }

  void _setStatus(UpdateStatus value) {
    if (_disposed) return;
    if (value != UpdateStatus.checking) _timeout?.cancel();
    status = value;
    notifyListeners();
  }

  @override
  void onUpdaterCheckingForUpdate(Appcast? appcast) {}
  @override
  void onUpdaterError(UpdaterError? error) => _setStatus(UpdateStatus.failed);
  @override
  void onUpdaterUpdateNotAvailable(UpdaterError? error) =>
      _setStatus(UpdateStatus.current);
  @override
  void onUpdaterUpdateAvailable(AppcastItem? item) {
    _setStatus(UpdateStatus.available);
    // WinSparkle's background check emits an event without showing its dialog.
    // A foreground check shows its native confirmation; it does not install.
    if (isWindows && _background && !_disposed) {
      _background = false;
      unawaited(_showWindowsConfirmation());
    }
  }

  Future<void> _showWindowsConfirmation() async {
    try {
      await _backend.check(background: false);
    } catch (_) {
      _setStatus(UpdateStatus.failed);
    }
  }

  @override
  void onUpdaterUpdateDownloaded(AppcastItem? item) {}
  @override
  void onUpdaterBeforeQuitForUpdate(AppcastItem? item) {
    _setStatus(UpdateStatus.installing);
    if (isWindows && !_disposed) unawaited(_quitForUpdate());
  }

  @override
  void dispose() {
    _disposed = true;
    _timeout?.cancel();
    if (_listening) _backend.unlisten(this);
    super.dispose();
  }
}
