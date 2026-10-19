import 'package:auto_updater/auto_updater.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/services/app_updater.dart';

class FakeUpdates implements UpdateBackend {
  UpdaterListener? listener;
  String? feed;
  final checks = <bool>[];
  bool fail = false;
  @override
  void listen(UpdaterListener value) => listener = value;
  @override
  void unlisten(UpdaterListener value) => listener = null;
  @override
  Future<void> configure(String value) async => feed = value;
  @override
  Future<void> check({required bool background}) async {
    if (fail) throw StateError('offline');
    checks.add(background);
  }
}

void main() {
  test('each supported CPU gets its own published release feed', () {
    expect(
      updateFeedFor('macos', 'macos_arm64'),
      endsWith('appcast-macos-arm64.xml'),
    );
    expect(
      updateFeedFor('macos', 'macos_x64'),
      endsWith('appcast-macos-x86_64.xml'),
    );
    expect(
      updateFeedFor('windows', 'windows_x64'),
      endsWith('appcast-windows-x64.xml'),
    );
    expect(updateFeedFor('windows', 'windows_arm64'), isNull);
    expect(updateFeedFor('linux', 'linux_x64'), isNull);
    expect(
      updateFeedFor('macos', 'macos_arm64'),
      contains('/releases/latest/download/'),
    );
    expect(
      updateFeedFor('macos', 'macos_arm64'),
      startsWith('https://github.com/DevMobileAn27/flutter_network_viewer/'),
    );
  });

  test('checks are serialized and errors allow retry', () async {
    final native = FakeUpdates();
    final updater = AppUpdater(
      backend: native,
      feed: 'https://example.test/feed',
      isWindows: false,
    );
    addTearDown(updater.dispose);
    await updater.check(background: true);
    expect(updater.status, UpdateStatus.checking);
    await updater.check();
    expect(native.checks, [true]);
    native.listener!.onUpdaterError(null);
    expect(updater.status, UpdateStatus.failed);
    await updater.check();
    expect(native.checks, [true, false]);
    native.listener!.onUpdaterUpdateNotAvailable(null);
    expect(updater.status, UpdateStatus.current);
  });

  test('macOS keeps no-update result after Sparkle abort callback', () async {
    final native = FakeUpdates();
    final updater = AppUpdater(
      backend: native,
      feed: 'https://example.test/feed',
      isWindows: false,
    );
    addTearDown(updater.dispose);

    await updater.check(background: true);
    native.listener!.onUpdaterUpdateNotAvailable(null);
    native.listener!.onUpdaterError(null);
    expect(updater.status, UpdateStatus.current);

    // A real failure on the next attempt must still be visible.
    await updater.check();
    native.listener!.onUpdaterError(null);
    expect(updater.status, UpdateStatus.failed);
  });

  test('Windows errors after no-update are still reported', () async {
    final native = FakeUpdates();
    final updater = AppUpdater(
      backend: native,
      feed: 'https://example.test/feed',
      isWindows: true,
    );
    addTearDown(updater.dispose);
    await updater.check();
    native.listener!.onUpdaterUpdateNotAvailable(null);
    native.listener!.onUpdaterError(null);
    expect(updater.status, UpdateStatus.failed);
  });

  test('Windows background discovery opens confirmation only once', () async {
    final native = FakeUpdates();
    var quits = 0;
    final updater = AppUpdater(
      backend: native,
      feed: 'https://example.test/feed',
      isWindows: true,
      quitForUpdate: () async {
        quits++;
      },
    );
    addTearDown(updater.dispose);
    await updater.check(background: true);
    native.listener!.onUpdaterUpdateAvailable(null);
    await Future<void>.delayed(Duration.zero);
    expect(native.checks, [true, false]);
    expect(quits, 0);
    native.listener!.onUpdaterUpdateAvailable(null);
    native.listener!.onUpdaterUpdateDownloaded(null);
    expect(native.checks, [true, false]);
    expect(quits, 0);
    native.listener!.onUpdaterBeforeQuitForUpdate(null);
    expect(quits, 1);
    expect(updater.status, UpdateStatus.installing);
  });

  test('macOS leaves native confirmation and restart to Sparkle', () async {
    final native = FakeUpdates();
    var quits = 0;
    final updater = AppUpdater(
      backend: native,
      feed: 'https://example.test/feed',
      isWindows: false,
      quitForUpdate: () async {
        quits++;
      },
    );
    addTearDown(updater.dispose);
    await updater.check(background: true);
    native.listener!.onUpdaterUpdateAvailable(null);
    expect(native.checks, [true]);
    native.listener!.onUpdaterBeforeQuitForUpdate(null);
    expect(quits, 0);
  });

  test('unsupported platform never invokes the native updater', () async {
    final native = FakeUpdates();
    final updater = AppUpdater(backend: native, feed: null, isWindows: false);
    await updater.check();
    expect(native.checks, isEmpty);
    updater.dispose();
    expect(native.listener, isNull);
  });

  test(
    'native exception is contained and listener is removed on disposal',
    () async {
      final native = FakeUpdates()..fail = true;
      final updater = AppUpdater(
        backend: native,
        feed: 'https://example.test/feed',
        isWindows: false,
      );
      await updater.check();
      expect(updater.status, UpdateStatus.failed);
      final listener = native.listener!;
      updater.dispose();
      listener.onUpdaterUpdateAvailable(null);
      expect(native.listener, isNull);
    },
  );
}
