import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/ui/json_document.dart';
import 'package:flutter_network_viewer/ui/json_find_controller.dart';
import 'package:flutter_network_viewer/ui/native_capture_list.dart';
import 'package:flutter_network_viewer/ui/native_json_view.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  testWidgets(
    'native documents transfer once; navigation and mode use separate messages',
    (tester) async {
      final calls = <MethodCall>[];
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      messenger.setMockMethodCallHandler(NativeJson.channel, (call) async {
        calls.add(call);
        return null;
      });
      final doc = JsonDocument.fromValue({'a': 'red', 'b': 'red', 'c': 'red'});
      final findController = JsonFindController()
        ..setValue(doc.value, prepared: doc);
      Widget app(bool tree) => MaterialApp(
        home: SizedBox(
          width: 600,
          height: 400,
          child: NativeJsonView(
            document: doc,
            tree: tree,
            find: findController,
            fallback: (_) => const Text('fallback'),
          ),
        ),
      );
      await tester.pumpWidget(app(false));
      await tester.runAsync(() async {
        for (
          var i = 0;
          i < 100 && !calls.any((c) => c.method == 'document');
          i++
        ) {
          await Future<void>.delayed(const Duration(milliseconds: 10));
        }
      });
      await tester.pumpAndSettle();
      expect(calls.where((c) => c.method == 'document').length, 1);
      calls.clear();
      findController.update(query: 'red');
      await tester.pumpAndSettle();
      expect(findController.matches.length, 3);
      calls.clear();
      findController.move(1);
      await tester.pumpAndSettle();
      expect(calls.map((c) => c.method), ['active']);
      expect((calls.single.arguments as Map)['index'], 1);
      calls.clear();
      await tester.pumpWidget(app(true));
      await tester.pumpAndSettle();
      expect(calls.any((c) => c.method == 'document'), false);
      expect(
        (calls.firstWhere((c) => c.method == 'configure').arguments
            as Map)['tree'],
        true,
      );
      NativeLists.overlayFallback = true;
      final release = NativeListOverlays.acquire();
      await tester.pumpAndSettle();
      expect(find.text('fallback'), findsOneWidget);
      expect(
        (calls.lastWhere((c) => c.method == 'frame').arguments
            as Map)['visible'],
        false,
      );
      release();
      NativeLists.overlayFallback = false;
      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
      expect(calls.last.method, 'dispose');
      messenger.setMockMethodCallHandler(NativeJson.channel, null);
      findController.dispose();
    },
  );
  testWidgets(
    'native viewport follows ancestor scrolling and clips child windows',
    (tester) async {
      final calls = <MethodCall>[];
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      messenger.setMockMethodCallHandler(NativeJson.channel, (call) async {
        calls.add(call);
        return null;
      });
      final scroll = ScrollController();
      await tester.pumpWidget(
        MaterialApp(
          home: Align(
            alignment: Alignment.topLeft,
            child: SizedBox(
              width: 400,
              height: 200,
              child: SingleChildScrollView(
                controller: scroll,
                child: Column(
                  children: [
                    const SizedBox(height: 100),
                    SizedBox(
                      height: 400,
                      child: NativeJsonView(
                        document: JsonDocument.fromValue({'a': 1}),
                        tree: false,
                        fallback: (_) => const SizedBox(),
                      ),
                    ),
                    const SizedBox(height: 200),
                  ],
                ),
              ),
            ),
          ),
        ),
      );
      await tester.runAsync(() async {
        for (
          var i = 0;
          i < 100 && !calls.any((c) => c.method == 'document');
          i++
        ) {
          await Future<void>.delayed(const Duration(milliseconds: 10));
        }
      });
      await tester.pumpAndSettle();
      scroll.jumpTo(150);
      await tester.pumpAndSettle();
      final frame =
          calls.lastWhere((c) => c.method == 'frame').arguments as Map;
      expect(frame['y'], -50.0);
      expect(frame['clipY'], 50.0);
      expect(frame['clipHeight'], 200.0);
      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
      messenger.setMockMethodCallHandler(NativeJson.channel, null);
      scroll.dispose();
    },
  );

  testWidgets(
    'native creation error disposes the host and uses Flutter fallback',
    (tester) async {
      final calls = <String>[];
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      messenger.setMockMethodCallHandler(NativeJson.channel, (call) async {
        calls.add(call.method);
        if (call.method == 'create') {
          throw PlatformException(code: 'unavailable');
        }
        return null;
      });
      await tester.pumpWidget(
        MaterialApp(
          home: NativeJsonView(
            document: JsonDocument.fromValue({'a': 1}),
            tree: false,
            fallback: (_) => const Text('fallback'),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.text('fallback'), findsOneWidget);
      expect(calls, contains('dispose'));
      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
      messenger.setMockMethodCallHandler(NativeJson.channel, null);
    },
  );
}
