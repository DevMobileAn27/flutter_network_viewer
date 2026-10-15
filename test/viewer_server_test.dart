import 'dart:async';
import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/flutter_capture_adapter.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';

import 'support/fake_vm_service.dart';

void main() {
  test(
    'normalizes DevTools, encoded service, HTTP roots and WebSocket URLs',
    () {
      const ws = 'ws://127.0.0.1:57037/SfP6AjEheVE=/ws';
      for (final input in [
        ws,
        'http://127.0.0.1:57037/SfP6AjEheVE=/',
        'http://127.0.0.1:57037/SfP6AjEheVE=/devtools/?uri=$ws',
        'http://localhost:9100/?uri=${Uri.encodeComponent(ws)}',
      ]) {
        expect(ViewerServer.normalizeUri(input).toString(), ws);
      }
      expect(
        ViewerServer.normalizeUri('https://example.com/token=/').toString(),
        'wss://example.com/token=/ws',
      );
      for (final value in [
        '',
        '70000',
        'file:///tmp/test',
        'http:///missing',
        'http://localhost/devtools/',
      ]) {
        expect(() => ViewerServer.normalizeUri(value), throwsFormatException);
      }
    },
  );

  test(
    'adapter preserves timing, headers, Unicode, binary bodies and errors',
    () {
      final events = FlutterCaptureAdapter.network(profile());
      expect(events.first['request']['headers']['x-tags'], 'one, two');
      expect(events.last['response']['body'], contains('Cà phê'));
      expect(events.last['durationMs'], 124);
      final binary = profile()..['responseBody'] = [255, 254];
      expect(
        FlutterCaptureAdapter.network(binary).last['response']['bodyEncoding'],
        'base64',
      );
      expect(
        FlutterCaptureAdapter.network(binary).last['response']['body'],
        '//4=',
      );
      final failed = profile()
        ..['request'] = {'error': 'Connection refused'}
        ..remove('response');
      expect(FlutterCaptureAdapter.network(failed).last['phase'], 'error');
      expect(
        FlutterCaptureAdapter.network(profile(complete: false)),
        hasLength(1),
      );
    },
  );

  test(
    'one VM app groups its isolates without colliding request IDs',
    () async {
      final vm = FakeVmService();
      await vm.start();
      vm.profiles['request-1'] = profile(complete: false);
      final store = CaptureStore();
      final viewer = ViewerServer(
        store,
        pollInterval: const Duration(milliseconds: 30),
      );
      addTearDown(() async {
        await viewer.stop();
        viewer.dispose();
        store.dispose();
        await vm.close();
      });
      await viewer.start(uri: vm.uri);
      expect(viewer.error, isNull);
      expect(viewer.running, isTrue);
      expect(store.network.values.single.pending, isTrue);
      expect(viewer.isOnline(store.sessions.keys.single), isTrue);
      vm.clock++;
      vm.profiles['request-1'] = profile()..['_updated'] = vm.clock;
      await until(() => !store.network.values.single.pending);
      expect(store.network.length, 1);
      expect(store.network.values.single.status, 201);
      expect(store.network.values.single.curl, contains('--data-raw'));
      vm.event('Stdout', {'bytes': base64Encode(utf8.encode('hel'))});
      vm.event('Stdout', {'bytes': base64Encode(utf8.encode('lo\n'))});
      vm.event('Stderr', {'bytes': base64Encode(utf8.encode('bad\n'))});
      vm.event('Logging', {
        'logRecord': {
          'message': {'type': '@Instance', 'valueAsString': 'warning'},
          'level': 900,
        },
      });
      await until(() => store.logs.length == 3);
      expect(
        store.logs.map((e) => e.message),
        containsAll(['hello', 'bad', 'warning']),
      );
      expect(
        store.logs.map((e) => e.level),
        containsAll(['log', 'error', 'warn']),
      );
      vm.isolateIds.add('isolates/2');
      await until(() => store.network.length == 2);
      expect(store.sessions.length, 1);
      expect(
        store.network.values.map((r) => r.sessionId).toSet(),
        hasLength(1),
      );
      expect(
        store.network.values.map((r) => r.requestId).toSet(),
        hasLength(2),
      );
      expect(
        store.network.values.map((r) => r.requestEvent['isolateId']),
        containsAll(['isolates/1', 'isolates/2']),
      );
      expect(
        store.network.values.every(
          (r) => r.requestEvent['vmRequestId'] == 'request-1',
        ),
        isTrue,
      );
      vm.event('Stdout', {
        'isolate': {
          'type': '@Isolate',
          'id': 'isolates/2',
          'name': 'main',
          'number': '2',
        },
        'bytes': base64Encode(utf8.encode('second isolate\n')),
      });
      await until(() => store.logs.length == 4);
      expect(store.logs.last.sessionId, store.sessions.keys.single);
      expect(store.logs.last.event['isolateId'], 'isolates/2');
      vm.isolateIds.remove('isolates/2');
      await until(
        () =>
            (store.sessions.values.single.metadata['isolates'] as List)
                .length ==
            1,
      );
      expect(viewer.isOnline(store.sessions.keys.single), isTrue);
      await viewer.stop();
      expect(viewer.running, isFalse);
      expect(viewer.isOnline(store.sessions.keys.first), isFalse);
    },
  );

  test(
    'hot restart keeps one application tab and retains old requests',
    () async {
      final vm = FakeVmService();
      await vm.start();
      vm.profiles['request-1'] = profile();
      final store = CaptureStore();
      final viewer = ViewerServer(
        store,
        pollInterval: const Duration(milliseconds: 30),
      );
      addTearDown(() async {
        await viewer.stop();
        viewer.dispose();
        store.dispose();
        await vm.close();
      });
      await viewer.start(uri: vm.uri);
      final session = store.sessions.keys.single;
      vm.isolateIds.clear();
      await until(() => !viewer.isOnline(session));
      vm.isolateIds.add('isolates/restarted');
      await until(() => store.network.length == 2);
      expect(store.sessions.keys.single, session);
      expect(viewer.isOnline(session), isTrue);
      expect(
        store.network.values.map((r) => r.requestId).toSet(),
        hasLength(2),
      );
      expect(store.sessions.values.single.metadata['isolates'], [
        {'id': 'isolates/restarted', 'name': 'main'},
      ]);
    },
  );

  test(
    'separate VM endpoints with the same app name remain distinct',
    () async {
      final first = FakeVmService();
      final second = FakeVmService();
      await first.start();
      await second.start();
      first.profiles['request-1'] = profile();
      second.profiles['request-1'] = profile();
      final store = CaptureStore();
      final viewer = ViewerServer(store);
      addTearDown(() async {
        await viewer.stop();
        viewer.dispose();
        store.dispose();
        await first.close();
        await second.close();
      });
      await viewer.start(uri: first.uri);
      final oldSession = store.sessions.keys.single;
      await viewer.start(uri: second.uri);
      expect(store.sessions.length, 2);
      expect(store.network.length, 2);
      expect(viewer.isOnline(oldSession), isFalse);
      expect(viewer.isOnline(store.sessions.keys.last), isTrue);
    },
  );

  test('Clear and Pause do not replay unchanged remote history', () async {
    final vm = FakeVmService();
    await vm.start();
    vm.profiles['request-1'] = profile()..['_updated'] = vm.clock - 1;
    final store = CaptureStore();
    final viewer = ViewerServer(
      store,
      pollInterval: const Duration(milliseconds: 30),
    );
    addTearDown(() async {
      await viewer.stop();
      viewer.dispose();
      store.dispose();
      await vm.close();
    });
    await viewer.start(uri: vm.uri);
    expect(store.network.length, 1);
    store.clear();
    await Future<void>.delayed(const Duration(milliseconds: 100));
    expect(store.network, isEmpty);
    store.togglePause();
    vm.clock += 2;
    vm.profiles['paused'] = profile(id: 'paused')..['_updated'] = vm.clock - 1;
    await until(() => store.ignoredWhilePaused > 0);
    store.togglePause();
    await Future<void>.delayed(const Duration(milliseconds: 100));
    expect(store.network, isEmpty);
    vm.clock += 2;
    vm.profiles['new'] = profile(id: 'new')..['_updated'] = vm.clock - 1;
    await until(() => store.network.isNotEmpty);
    expect(store.network.values.single.requestEvent['vmRequestId'], 'new');
  });

  test(
    'new isolate extensions are discovered after a delayed app startup',
    () async {
      final vm = FakeVmService()..profilerAvailable = false;
      await vm.start();
      final store = CaptureStore();
      final viewer = ViewerServer(
        store,
        pollInterval: const Duration(milliseconds: 30),
      );
      addTearDown(() async {
        await viewer.stop();
        viewer.dispose();
        store.dispose();
        await vm.close();
      });
      await viewer.start(uri: vm.uri);
      expect(store.sessions, isEmpty);
      vm.profilerAvailable = true;
      vm.profiles['request-1'] = profile();
      await until(() => store.network.isNotEmpty);
      expect(store.sessions.values.single.name, 'shop');
      expect(
        vm.calls.any(
          (c) =>
              c['method'] == 'ext.dart.io.httpEnableTimelineLogging' &&
              c['params']['enabled'] == 'true',
        ),
        isTrue,
      );
    },
  );

  test('stdout preserves UTF-8 split across VM events', () async {
    final vm = FakeVmService();
    await vm.start();
    final store = CaptureStore();
    final viewer = ViewerServer(store);
    addTearDown(() async {
      await viewer.stop();
      viewer.dispose();
      store.dispose();
      await vm.close();
    });
    await viewer.start(uri: vm.uri);
    final bytes = utf8.encode('Cà phê\n');
    vm.event('Stdout', {'bytes': base64Encode(bytes.sublist(0, 2))});
    vm.event('Stdout', {'bytes': base64Encode(bytes.sublist(2))});
    await until(() => store.logs.isNotEmpty);
    expect(store.logs.single.message, 'Cà phê');
  });

  test(
    'Flutter developer logs retain logger, severity, error and stack in order',
    () async {
      final vm = FakeVmService();
      await vm.start();
      final store = CaptureStore();
      final viewer = ViewerServer(store);
      addTearDown(() async {
        await viewer.stop();
        viewer.dispose();
        store.dispose();
        await vm.close();
      });
      await viewer.start(uri: vm.uri);
      for (final severity in [0, 500, 800, 900, 1000, 1200]) {
        vm.event('Logging', {
          'logRecord': {
            'message': {
              'type': '@Instance',
              'valueAsString': 'severity $severity',
            },
            'loggerName': {'type': '@Instance', 'valueAsString': 'Repository'},
            'level': severity,
          },
        });
      }
      vm.event('Logging', {
        'logRecord': {
          'message': {'type': '@Instance', 'valueAsString': 'Failed'},
          'level': 0,
          'error': {
            'type': '@Instance',
            'id': 'objects/error',
            'kind': 'PlainInstance',
            'class': {'type': '@Class', 'name': 'StateError'},
          },
          'stackTrace': {
            'type': '@Instance',
            'valueAsString': '#0 fetchUser (main.dart:4:2)',
          },
        },
      });
      vm.event('Stdout', {'bytes': base64Encode(utf8.encode('after\n'))});
      await until(() => store.logs.length == 8);
      final logs = store.logs.where((r) => r.source == 'dart.logging').toList();
      expect(logs.map((r) => r.level), [
        'log',
        'debug',
        'info',
        'warn',
        'error',
        'error',
        'error',
      ]);
      expect(logs.take(6).map((r) => r.event['severity']), [
        0,
        500,
        800,
        900,
        1000,
        1200,
      ]);
      expect(logs.first.loggerName, 'Repository');
      expect(logs[6].error, 'StateError: missing user');
      expect(logs[6].stackTrace, contains('main.dart:4:2'));
      expect(
        store.logs.where((r) => r.source == 'dart.stdout').single.message,
        'after',
      );

      vm.event('Stdout', {
        'bytes': base64Encode(utf8.encode('discard partial')),
      });
      await Future<void>.delayed(const Duration(milliseconds: 30));
      store.clear();
      vm.event('Stdout', {'bytes': base64Encode(utf8.encode('new line\n'))});
      await until(() => store.logs.isNotEmpty);
      expect(store.logs.single.message, 'new line');
    },
  );

  test('Clear discards a detail response already in flight', () async {
    final vm = FakeVmService();
    await vm.start();
    final store = CaptureStore();
    final viewer = ViewerServer(
      store,
      pollInterval: const Duration(milliseconds: 30),
    );
    addTearDown(() async {
      await viewer.stop();
      viewer.dispose();
      store.dispose();
      await vm.close();
    });
    await viewer.start(uri: vm.uri);
    vm.detailGate = Completer<void>();
    vm.profiles['old'] = profile(id: 'old')..['_updated'] = vm.clock;
    await until(
      () => vm.calls.any(
        (c) => c['method'] == 'ext.dart.io.getHttpProfileRequest',
      ),
    );
    store.clear();
    vm.clock++;
    vm.detailGate!.complete();
    await Future<void>.delayed(const Duration(milliseconds: 150));
    expect(store.network, isEmpty);
  });

  test('latest connection wins and unexpected closure reconnects', () async {
    final oldVm = FakeVmService();
    final vm = FakeVmService();
    await oldVm.start();
    await vm.start();
    final store = CaptureStore();
    final viewer = ViewerServer(
      store,
      pollInterval: const Duration(milliseconds: 30),
    );
    addTearDown(() async {
      await viewer.stop();
      viewer.dispose();
      store.dispose();
      await oldVm.close();
      await vm.close();
    });
    await Future.wait([
      viewer.start(uri: oldVm.uri),
      viewer.start(uri: vm.uri),
    ]);
    expect(viewer.endpoint, vm.uri);
    expect(oldVm.sockets, isEmpty);
    expect(viewer.running, isTrue);
    await vm.sockets.last.close();
    await until(() => !viewer.running);
    await Future<void>.delayed(const Duration(milliseconds: 3100));
    await until(() => viewer.running && vm.sockets.length == 2);
    await until(() => store.sessions.keys.any(viewer.isOnline));
    await viewer.stop();
    final count = vm.sockets.length;
    await Future<void>.delayed(const Duration(milliseconds: 100));
    expect(vm.sockets.length, count);
    expect(viewer.running, isFalse);
  });

  test(
    'invalid input reports an error and disposal cancels connection attempts',
    () async {
      final store = CaptureStore();
      final viewer = ViewerServer(store);
      await viewer.start(uri: 'bad');
      expect(viewer.running, isFalse);
      expect(viewer.error, isNotNull);
      viewer.dispose();
      await viewer.start(uri: 'ws://localhost:1/ws');
      store.dispose();
    },
  );
}
