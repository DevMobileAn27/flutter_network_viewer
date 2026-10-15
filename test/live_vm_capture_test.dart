import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';

void main() {
  const uri = String.fromEnvironment('LIVE_VM_SERVICE_URI');
  test(
    'captures real Dart VM HTTP profile, stdout and developer.log',
    () async {
      final store = CaptureStore();
      final viewer = ViewerServer(
        store,
        pollInterval: const Duration(milliseconds: 200),
      );
      addTearDown(() async {
        await viewer.stop();
        viewer.dispose();
        store.dispose();
      });
      await viewer.start(uri: uri);
      expect(viewer.running, isTrue, reason: viewer.error);
      final complete = Completer<void>();
      void check() {
        if (!complete.isCompleted &&
            store.network.values.any(
              (r) => r.status == 201 && r.response['body'] != null,
            ) &&
            store.logs.any((r) => r.event['source'] == 'dart.stdout') &&
            store.logs.any((r) => r.event['source'] == 'dart.logging')) {
          complete.complete();
        }
      }

      store.addListener(check);
      check();
      await complete.future.timeout(const Duration(seconds: 15));
      store.removeListener(check);
      final record = store.network.values.firstWhere(
        (r) => r.status == 201 && r.response['body'] != null,
      );
      expect(record.request['body'], contains('quantity'));
      expect(record.response['body'], contains('Cà phê'));
      expect(record.duration, greaterThanOrEqualTo(0));
      expect(store.rejected, 0);
    },
    skip: uri.isEmpty
        ? 'Run with --dart-define=LIVE_VM_SERVICE_URI=<fixture URL>'
        : false,
  );
}
