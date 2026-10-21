import 'dart:async';
import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';

import 'support/fake_vm_service.dart';

void main() {
  late FakeVmService vm;
  late CaptureStore store;
  late ViewerServer viewer;
  const base = Duration(milliseconds: 30);
  const step = Duration(milliseconds: 20);
  const cap = Duration(milliseconds: 90);

  setUp(() async {
    vm = FakeVmService();
    await vm.start();
    store = CaptureStore(maxRecords: 10);
    viewer = ViewerServer(
      store,
      pollInterval: base,
      idlePollIncrement: step,
      maxPollInterval: cap,
      retryInterval: const Duration(milliseconds: 20),
    );
  });
  tearDown(() async {
    await viewer.stop();
    viewer.dispose();
    store.dispose();
    await vm.close();
  });

  void log(String message) =>
      vm.event('Stdout', {'bytes': base64Encode(utf8.encode('$message\n'))});

  test('production timings are 1.4s, +0.2s, capped at 5s', () {
    final defaults = ViewerServer(store);
    expect(defaults.pollInterval, const Duration(milliseconds: 1400));
    expect(defaults.idlePollIncrement, const Duration(milliseconds: 200));
    expect(defaults.maxPollInterval, const Duration(seconds: 5));
    defaults.dispose();
  });

  test(
    'unchanged requests back off; new requests and responses reset',
    () async {
      vm.profiles['request-1'] = profile(complete: false);
      await viewer.start(uri: vm.uri);
      expect(viewer.networkPollInterval, base);
      await until(() => viewer.networkPollInterval == base + step);
      await until(() => viewer.networkPollInterval == base + step * 2);
      await until(() => viewer.networkPollInterval == cap);
      final details = vm.calls
          .where((c) => c['method'] == 'ext.dart.io.getHttpProfileRequest')
          .length;
      await Future<void>.delayed(cap * 2);
      expect(viewer.networkPollInterval, cap);
      expect(
        vm.calls.where(
          (c) => c['method'] == 'ext.dart.io.getHttpProfileRequest',
        ),
        hasLength(details),
      );

      // The response changes an existing row, rather than adding a new row.
      vm.clock++;
      vm.profiles['request-1'] = profile();
      await until(() => !store.network.values.single.pending);
      expect(viewer.networkPollInterval, base);
      await until(() => viewer.networkPollInterval == cap);
      vm.clock++;
      vm.profiles['request-2'] = profile(id: 'request-2');
      await until(() => store.network.length == 2);
      expect(viewer.networkPollInterval, base);
      // Network traffic must not reset the independent Console timer.
      expect(viewer.consolePollInterval, cap);
    },
  );

  test('console batches logs in order and backs off independently', () async {
    await viewer.start(uri: vm.uri);
    await until(() => viewer.consolePollInterval == cap);
    var updates = 0;
    var previousCount = 0;
    store.addListener(() {
      if (store.logs.length != previousCount) {
        previousCount = store.logs.length;
        updates++;
      }
    });
    log('first');
    log('second');
    log('third');
    await until(() => store.logs.length == 3);
    expect(store.logs.map((r) => r.message), ['first', 'second', 'third']);
    expect(updates, 1);
    expect(viewer.consolePollInterval, base);
    expect(viewer.networkPollInterval, cap);
    await until(() => viewer.consolePollInterval == base + step);
    await until(() => viewer.consolePollInterval == cap);
    await Future<void>.delayed(cap * 2);
    expect(viewer.consolePollInterval, cap);
  });

  test('pending console is bounded and Clear/Pause discard old logs', () async {
    await viewer.start(uri: vm.uri);
    await until(() => viewer.consolePollInterval == cap);
    for (var i = 0; i < 25; i++) {
      log('line $i');
    }
    await until(() => store.logs.length == 10);
    expect(store.logs.first.message, 'line 15');
    expect(store.logs.last.message, 'line 24');
    expect(store.evicted, 15);
    log('discard before clear');
    await Future<void>.delayed(const Duration(milliseconds: 5));
    store.clear();
    store.togglePause();
    log('discard during pause');
    await Future<void>.delayed(const Duration(milliseconds: 5));
    store.togglePause();
    log('after resume');
    await until(() => store.logs.isNotEmpty);
    expect(store.logs.map((r) => r.message), ['after resume']);
    expect(store.ignoredWhilePaused, 1);
  });

  test(
    'slow network polls never overlap and Stop cancels both timers',
    () async {
      await viewer.start(uri: vm.uri);
      vm.detailGate = Completer<void>();
      vm.profiles['request-1'] = profile();
      int profileCalls() => vm.calls
          .where((c) => c['method'] == 'ext.dart.io.getHttpProfile')
          .length;
      await until(
        () => vm.calls.any(
          (c) => c['method'] == 'ext.dart.io.getHttpProfileRequest',
        ),
      );
      final count = profileCalls();
      await Future<void>.delayed(cap * 2);
      expect(profileCalls(), count);
      await viewer.stop();
      vm.detailGate!.complete();
      await Future<void>.delayed(cap * 2);
      expect(profileCalls(), count);
      expect(store.network, isEmpty);
      await viewer.start(uri: vm.uri);
      expect(viewer.networkPollInterval, base);
      expect(viewer.consolePollInterval, base);
    },
  );
}
