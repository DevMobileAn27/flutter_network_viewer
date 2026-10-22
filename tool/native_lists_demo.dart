// Local fixture: flutter run -d macos -t tool/native_lists_demo.dart
// No discovery, external connections or persisted settings.
import 'package:flutter/material.dart';
import 'package:flutter_network_viewer/main.dart' show ViewerApp;
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';
import 'package:flutter_network_viewer/ui/native_capture_list.dart';
import 'package:flutter_network_viewer/ui/native_json_view.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await NativeLists.initialize();
  await NativeJson.initialize();
  final store = CaptureStore();
  final events = <Map<String, Object?>>[];
  for (var i = 0; i < 1000; i++) {
    final time = DateTime(
      2026,
      10,
      21,
      12,
    ).add(Duration(seconds: i)).toIso8601String();
    events.addAll([
      {
        'requestId': 'demo-$i',
        'phase': 'request',
        'timestamp': time,
        'request': {
          'method': ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'OPTIONS'][i % 6],
          'url': 'https://example.test/api/items/$i?demo=true',
        },
      },
      {
        'requestId': 'demo-$i',
        'phase': 'response',
        'timestamp': time,
        'durationMs': 12 + i % 100,
        'response': {
          'statusCode': i % 7 == 0 ? 500 : 200,
          'headers': {'content-type': 'application/json'},
          'body': '{"id":$i,"message":"Local demo response"}',
        },
      },
      {
        'source': 'dart.stdout',
        'level': ['log', 'info', 'warn', 'error', 'debug'][i % 5],
        'timestamp': time,
        'message': i % 5 == 0
            ? 'Demo log $i: Tiếng Việt 😀 — a longer message that wraps onto several lines when the list gets narrow.\nSecond line, aligned with the message.'
            : 'Demo log $i: completed operation successfully.',
      },
    ]);
  }
  store.ingest({
    'session': {
      'id': 'native-demo',
      'appName': 'Native list demo',
      'deviceName': 'Local fixture',
      'platform': 'macos',
    },
    'sdk': {'schemaVersion': 1},
    'events': events,
  });
  runApp(ViewerApp(store: store, server: ViewerServer(store)));
}
