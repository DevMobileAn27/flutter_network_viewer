import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';

import 'support/fake_vm_service.dart';

void main() {
  late FakeVmService vm;
  late CaptureStore store;
  late ViewerServer viewer;
  setUp(() async {
    vm = FakeVmService();
    await vm.start();
    vm.profiles['request-1'] = profile()..['_updated'] = vm.clock;
    store = CaptureStore();
    viewer = ViewerServer(
      store,
      pollInterval: const Duration(milliseconds: 30),
      idlePollIncrement: const Duration(milliseconds: 10),
      maxPollInterval: const Duration(milliseconds: 50),
      retryInterval: const Duration(hours: 1),
    );
    await viewer.start(uri: vm.uri);
  });
  tearDown(() async {
    await viewer.stop();
    viewer.dispose();
    store.dispose();
    await vm.close();
  });

  void log(String text) =>
      vm.event('Stdout', {'bytes': base64Encode(utf8.encode(text))});

  test(
    'socket disconnect deletes project, requests, logs and pending logs',
    () async {
      log('retained\n');
      await until(() => store.logs.isNotEmpty);
      log('pending\n');
      await vm.sockets.last.close();
      await until(() => !viewer.running);
      expect(store.sessions, isEmpty);
      expect(store.network, isEmpty);
      expect(store.logs, isEmpty);
      await Future<void>.delayed(const Duration(milliseconds: 150));
      expect(store.export()['sessions'], isEmpty);
      expect(store.export()['console'], isEmpty);
    },
  );

  test(
    'Clear drops all captured history without replay at inclusive cursor',
    () async {
      log('partial before clear');
      await Future<void>.delayed(const Duration(milliseconds: 10));
      final detailCalls = vm.calls
          .where(
            (call) => call['method'] == 'ext.dart.io.getHttpProfileRequest',
          )
          .length;
      store.clear();
      await Future<void>.delayed(const Duration(milliseconds: 200));
      expect(store.sessions, isEmpty);
      expect(store.network, isEmpty);
      expect(store.logs, isEmpty);
      expect(
        vm.calls.where(
          (call) => call['method'] == 'ext.dart.io.getHttpProfileRequest',
        ),
        hasLength(detailCalls),
      );
      // Reconnecting to the same VM must not bring cleared remote history back.
      await viewer.start(uri: vm.uri);
      expect(store.network, isEmpty);
      expect(store.logs, isEmpty);
      log('after clear\n');
      await until(() => store.logs.isNotEmpty);
      expect(store.logs.single.message, 'after clear');
      vm.clock++;
      vm.profiles['new'] = profile(id: 'new');
      await until(() => store.network.isNotEmpty);
      expect(store.network.values.single.requestEvent['vmRequestId'], 'new');
    },
  );
}
