import 'dart:async';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';
import 'package:flutter_network_viewer/services/vm_service_discovery.dart';

import 'support/fake_vm_service.dart';

String command(String uri) =>
    '/sdk/dart development-service --vm-service-uri=$uri';

void main() {
  test(
    'discovers and connects to a real local Flutter app',
    () async {
      final targets = await VmServiceDiscovery().discover();
      expect(
        targets,
        isNotEmpty,
        reason: 'Run a Flutter project with DDS first.',
      );
      final store = CaptureStore();
      final viewer = ViewerServer(store);
      addTearDown(() async {
        await viewer.stop();
        viewer.dispose();
        store.dispose();
      });
      await viewer.start(automatic: true, uri: targets.first.uri);
      expect(viewer.running, isTrue, reason: viewer.error);
      expect(store.sessions, isNotEmpty);
      // Keep authentication tokens and captured request data out of test output.
      // ignore: avoid_print
      print(
        'Discovered Flutter apps: ${targets.map((app) => app.name).join(', ')}',
      );
    },
    skip: !const bool.fromEnvironment('LIVE_VM_DISCOVERY')
        ? 'Run with --dart-define=LIVE_VM_DISCOVERY=true and a local Flutter project'
        : false,
  );

  test('only uses local DDS process arguments, preserving auth tokens', () {
    final uris = VmServiceDiscovery.processUris('''
/sdk/dart development-service --vm-service-uri=http://127.0.0.1:1234/a=/
/sdk/dart development-service --vm-service-uri=http://127.0.0.1:1234/a=/
"C:\\sdk\\dart.exe" development-service --vm-service-uri "http://localhost:2345/b=/"
/sdk/dartaotruntime /cache/dds_aot.dart.snapshot --vm-service-uri=http://[::1]:3456/c=/
/sdk/dart devtools --vm-service-uri=http://127.0.0.1:4567/no=/
/sdk/dart development-service --vm-service-uri=http://example.com:5678/no=/
/sdk/dart development-service --vm-service-uri=file:///tmp/no
''');
    expect(uris.map((uri) => uri.port), [1234, 2345, 3456]);
    expect(uris.first.path, '/a=/');
  });

  test(
    'follows the VM redirect to the DDS token and deduplicates a VM',
    () async {
      final vm = FakeVmService();
      await vm.start();
      final redirect = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
      redirect.listen((request) async {
        await request.response.redirect(
          Uri.parse(
            'http://127.0.0.1:${vm.server.port}/token=/devtools/?uri=${Uri.encodeComponent(vm.uri)}',
          ),
        );
      });
      addTearDown(() async {
        await redirect.close(force: true);
        await vm.close();
      });
      final discovery = VmServiceDiscovery(
        readProcesses: () async => [
          command('http://127.0.0.1:${redirect.port}/original-token=/'),
          command(vm.uri),
        ].join('\n'),
      );
      final targets = await discovery.discover();
      expect(targets, hasLength(1));
      expect(targets.single.uri, vm.uri);
      expect(targets.single.name, 'shop');
      expect(targets.single.projectKey, 'package:shop/main.dart');
    },
  );

  test('ignores non-Flutter VMs and the Viewer itself', () async {
    final dartVm = FakeVmService()..flutterApp = false;
    final viewerVm = FakeVmService()
      ..vmPid = 11
      ..rootLibrary = 'package:flutter_network_viewer/main.dart';
    await dartVm.start();
    await viewerVm.start();
    addTearDown(() async {
      await dartVm.close();
      await viewerVm.close();
    });
    final discovery = VmServiceDiscovery(
      readProcesses: () async =>
          '${command(dartVm.uri)}\n${command(viewerVm.uri)}',
    );
    expect(await discovery.discover(), isEmpty);
  });

  test('reconnects the same project after both port and token change', () async {
    final first = FakeVmService();
    final second = FakeVmService()
      ..vmStartTime = 2000
      ..authToken = 'new-token=';
    final unrelated = FakeVmService()
      ..vmPid = 20
      ..rootLibrary = 'package:other/main.dart';
    await first.start();
    await second.start();
    await unrelated.start();
    final redirect = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    var currentUri = first.uri;
    redirect.listen((request) async {
      await request.response.redirect(
        Uri.parse(
          'http://127.0.0.1:${Uri.parse(currentUri).port}/devtools/?uri=${Uri.encodeComponent(currentUri)}',
        ),
      );
    });
    var processes = command('http://127.0.0.1:${redirect.port}/first-token=/');
    final store = CaptureStore();
    final viewer = ViewerServer(
      store,
      discovery: VmServiceDiscovery(readProcesses: () async => processes),
      retryInterval: const Duration(milliseconds: 25),
    );
    addTearDown(() async {
      await viewer.stop();
      viewer.dispose();
      store.dispose();
      await first.close();
      await second.close();
      await unrelated.close();
      await redirect.close(force: true);
    });
    await viewer.start(automatic: true);
    expect(viewer.endpoint, first.uri);
    expect(viewer.running, isTrue);
    // Simulate a run with a new VM URL while another project is also running.
    currentUri = second.uri;
    processes += '\n${command(unrelated.uri)}';
    for (final socket in first.sockets) {
      await socket.close();
    }
    await until(
      () => viewer.running && !viewer.busy && viewer.endpoint == second.uri,
    );
    expect(viewer.error, isNull);
    await viewer.stop();
    await Future<void>.delayed(const Duration(milliseconds: 80));
    expect(viewer.running, isFalse);
    expect(viewer.searching, isFalse);
  });

  test('waits for a choice when multiple projects are found', () async {
    final first = FakeVmService();
    final second = FakeVmService()
      ..vmPid = 20
      ..rootLibrary = 'package:other/main.dart';
    await first.start();
    await second.start();
    final store = CaptureStore();
    final viewer = ViewerServer(
      store,
      discovery: VmServiceDiscovery(
        readProcesses: () async =>
            '${command(first.uri)}\n${command(second.uri)}',
      ),
    );
    addTearDown(() async {
      await viewer.stop();
      viewer.dispose();
      store.dispose();
      await first.close();
      await second.close();
    });
    await viewer.start(automatic: true);
    expect(viewer.running, isFalse);
    expect(viewer.searching, isTrue);
    expect(viewer.discoveredApps, hasLength(2));
    await viewer.start(automatic: true, uri: second.uri);
    expect(viewer.running, isTrue);
    expect(viewer.endpoint, second.uri);
  });

  test(
    'Stop and manual connection supersede an unfinished discovery',
    () async {
      final pending = Completer<String>();
      final vm = FakeVmService();
      await vm.start();
      final store = CaptureStore();
      final viewer = ViewerServer(
        store,
        discovery: VmServiceDiscovery(readProcesses: () => pending.future),
      );
      addTearDown(() async {
        await viewer.stop();
        viewer.dispose();
        store.dispose();
        await vm.close();
      });
      final scan = viewer.start(automatic: true);
      await until(() => viewer.busy);
      await viewer.stop();
      await viewer.start(uri: vm.uri);
      pending.complete(command(vm.uri));
      await scan;
      expect(viewer.running, isTrue);
      expect(viewer.autoDetect, isFalse);
      expect(viewer.discoveredApps, isEmpty);
    },
  );

  test('waits and retries when a project starts after the Viewer', () async {
    final vm = FakeVmService();
    await vm.start();
    var processes = '';
    final store = CaptureStore();
    final viewer = ViewerServer(
      store,
      discovery: VmServiceDiscovery(readProcesses: () async => processes),
      retryInterval: const Duration(milliseconds: 25),
    );
    addTearDown(() async {
      await viewer.stop();
      viewer.dispose();
      store.dispose();
      await vm.close();
    });
    await viewer.start(automatic: true);
    expect(viewer.searching, isTrue);
    expect(viewer.error, isNull);
    processes = command(vm.uri);
    await until(() => viewer.running && !viewer.busy);
    expect(viewer.endpoint, vm.uri);
  });
}
