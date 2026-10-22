import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/ui/native_capture_list.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'diff sends summaries once, patches changes, and releases removed rows',
    () {
      final diff = NativeListDiff<(String, int)>();
      var serialized = 0;
      Map<String, Object?>? update(List<(String, int)> rows) =>
          diff.update(rows, (r) => r.$1, (r) => r.$2, (r) {
            serialized++;
            return {'status': r.$2};
          });
      expect(update([('a', 1), ('b', 1)])!['order'], ['a', 'b']);
      expect(serialized, 2);
      expect(update([('a', 1), ('b', 1)]), isNull);
      expect(serialized, 2);
      final patch = update([('a', 1), ('b', 2)])!;
      expect(patch.containsKey('order'), isFalse);
      expect(patch['rows'], [
        {'key': 'b', 'status': 2},
      ]);
      expect(update([('b', 2)])!['removed'], ['a']);
      expect(update([])!['removed'], ['b']);
      expect(update([('a', 1)])!['rows'], [
        {'key': 'a', 'status': 1},
      ]);
    },
  );

  testWidgets(
    'native list separates selection, hides for overlays and disposes after updates',
    (tester) async {
      final calls = <MethodCall>[];
      debugDefaultTargetPlatformOverride = TargetPlatform.windows;
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      messenger.setMockMethodCallHandler(NativeLists.channel, (call) async {
        calls.add(call);
        return call.method == 'capabilities' ? 1 : null;
      });
      await NativeLists.initialize();
      debugDefaultTargetPlatformOverride = null;
      final selection = ValueNotifier<String?>(null);
      addTearDown(() {
        NativeLists.available = false;
        debugDefaultTargetPlatformOverride = null;
        messenger.setMockMethodCallHandler(NativeLists.channel, null);
        selection.dispose();
      });
      var selected = '';
      Widget app(List<int> rows, {bool visible = true}) => MaterialApp(
        home: Scaffold(
          body: NativeCaptureList<int>(
            kind: 'console',
            records: rows,
            keyOf: (r) => '$r',
            versionOf: (r) => r,
            rowOf: (r) => {'message': 'Demo $r'},
            selection: selection,
            selectedKey: () => selection.value,
            onSelected: (row) => selected = '$row',
            visible: visible,
            fallback: (_) => const Text('Flutter fallback'),
          ),
        ),
      );
      await tester.pumpWidget(app([1, 2]));
      await tester.pumpAndSettle();
      expect(calls.where((c) => c.method == 'create').length, 1);
      expect(calls.where((c) => c.method == 'update').length, 1);
      final id =
          (calls.firstWhere((c) => c.method == 'create').arguments
              as Map)['id'];
      calls.clear();
      selection.value = '2';
      await tester.pumpAndSettle();
      expect(calls.map((c) => c.method), ['selection']);
      final reply = Completer<void>();
      messenger.handlePlatformMessage(
        NativeLists.channel.name,
        const StandardMethodCodec().encodeMethodCall(
          MethodCall('selected', {'id': id, 'key': '1'}),
        ),
        (_) => reply.complete(),
      );
      await tester.runAsync(() => reply.future);
      expect(selected, '1');
      final release = NativeListOverlays.acquire();
      await tester.pumpAndSettle();
      expect(find.text('Flutter fallback'), findsOneWidget);
      expect(
        (calls.lastWhere((c) => c.method == 'frame').arguments
            as Map)['visible'],
        false,
      );
      release();
      await tester.pumpAndSettle();
      expect(find.text('Flutter fallback'), findsNothing);
      await tester.pumpWidget(app([2, 3], visible: false));
      await tester.pumpAndSettle();
      expect(
        (calls.lastWhere((c) => c.method == 'update').arguments as Map)['rows'],
        [
          {'key': '3', 'message': 'Demo 3'},
        ],
      );
      expect(
        (calls.lastWhere((c) => c.method == 'frame').arguments
            as Map)['visible'],
        false,
      );
      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
      expect(calls.last.method, 'dispose');
    },
  );

  testWidgets(
    'AppKit compositing keeps the native list present under Flutter overlays',
    (tester) async {
      final calls = <MethodCall>[];
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      messenger.setMockMethodCallHandler(NativeLists.channel, (call) async {
        calls.add(call);
        return call.method == 'capabilities' ? 1 : null;
      });
      debugDefaultTargetPlatformOverride = TargetPlatform.macOS;
      await NativeLists.initialize();
      debugDefaultTargetPlatformOverride = null;
      final selection = ValueNotifier<String?>(null);
      await tester.pumpWidget(
        MaterialApp(
          home: NativeCaptureList<int>(
            kind: 'network',
            records: const [1],
            keyOf: (r) => '$r',
            versionOf: (r) => r,
            rowOf: (r) => {'method': 'GET'},
            selection: selection,
            selectedKey: () => null,
            onSelected: (_) {},
            fallback: (_) => const Text('Flutter fallback'),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final release = NativeListOverlays.acquire();
      await tester.pumpAndSettle();
      expect(find.text('Flutter fallback'), findsNothing);
      expect(
        (calls.lastWhere((c) => c.method == 'frame').arguments
            as Map)['visible'],
        true,
      );
      release();
      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
      NativeLists.available = false;
      NativeLists.overlayFallback = false;
      messenger.setMockMethodCallHandler(NativeLists.channel, null);
      selection.dispose();
    },
  );

  testWidgets(
    'native creation failure releases host and shows Flutter fallback',
    (tester) async {
      final calls = <String>[];
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      messenger.setMockMethodCallHandler(NativeLists.channel, (call) async {
        calls.add(call.method);
        if (call.method == 'create') {
          throw PlatformException(code: 'test-failure');
        }
        return null;
      });
      final selection = ValueNotifier<String?>(null);
      await tester.pumpWidget(
        MaterialApp(
          home: NativeCaptureList<int>(
            kind: 'console',
            records: const [1],
            keyOf: (r) => '$r',
            versionOf: (r) => r,
            rowOf: (r) => {'message': 'Demo'},
            selection: selection,
            selectedKey: () => null,
            onSelected: (_) {},
            fallback: (_) => const Text('fallback'),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('fallback'), findsOneWidget);
      expect(calls, contains('dispose'));
      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
      messenger.setMockMethodCallHandler(NativeLists.channel, null);
      selection.dispose();
    },
  );
}
