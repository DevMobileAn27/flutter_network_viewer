import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/l10n/app_localizations.dart';
import 'package:flutter_network_viewer/ui/components.dart';
import 'package:flutter_network_viewer/ui/json_code_view.dart';
import 'package:flutter_network_viewer/ui/json_find_controller.dart';
import 'package:flutter_network_viewer/ui/virtual_json_code_view.dart';

Widget app(Object value, JsonFindController find) => MaterialApp(
  theme: viewerTheme(Brightness.light).copyWith(platform: TargetPlatform.macOS),
  localizationsDelegates: AppLocalizations.localizationsDelegates,
  supportedLocales: AppLocalizations.supportedLocales,
  home: Scaffold(
    body: JsonCodeView(value: value, find: find, viewportHeight: 400),
  ),
);

void main() {
  testWidgets(
    'large JSON builds visible rows, finds offscreen text and unfolds',
    (tester) async {
      final value = List.generate(
        1000,
        (i) => {'message': 'row-$i', 'color': '#3366ff'},
      );
      final findController = JsonFindController()..setValue(value);
      addTearDown(findController.dispose);
      await tester.pumpWidget(app(value, findController));
      await tester.pumpAndSettle();
      expect(find.byType(VirtualJsonCodeView), findsOneWidget);
      expect(
        find
            .byWidgetPredicate(
              (w) =>
                  w.key is ValueKey<String> &&
                  (w.key! as ValueKey<String>).value.startsWith(
                    'virtual-json-row-',
                  ),
            )
            .evaluate()
            .length,
        lessThan(40),
      );
      expect(find.textContaining('row-999', findRichText: true), findsNothing);
      await tester.tap(find.byKey(const ValueKey('json-fold-0')));
      await tester.pumpAndSettle();
      expect(find.text('[ … ]', findRichText: true), findsOneWidget);
      findController.update(query: 'row-999');
      await tester.pumpAndSettle();
      expect(
        find.textContaining('row-999', findRichText: true),
        findsOneWidget,
      );
      final list = tester.widget<ListView>(
        find.byKey(const ValueKey('virtual-json-lines')),
      );
      expect(list.controller!.offset, greaterThan(1000));
      expect(findController.active!.path, [999, 'message']);
      final row = tester.widget<RichText>(
        find.textContaining('row-999', findRichText: true),
      );
      var highlighted = false;
      row.text.visitChildren((span) {
        highlighted |= span.style?.backgroundColor == const Color(0xfffb923c);
        return true;
      });
      expect(highlighted, isTrue);
      final replacement = List.generate(800, (i) => 'new-$i');
      findController.setValue(replacement);
      await tester.pumpWidget(app(replacement, findController));
      await tester.pumpAndSettle();
      expect(find.textContaining('new-0', findRichText: true), findsOneWidget);
      expect(find.textContaining('row-999', findRichText: true), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'select all copies the complete large document without the gutter',
    (tester) async {
      final value = List.generate(1000, (i) => 'message-$i');
      final controller = JsonFindController()..setValue(value);
      addTearDown(controller.dispose);
      String? copied;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            copied = (call.arguments as Map)['text'] as String;
          }
          return null;
        },
      );
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        ),
      );
      await tester.pumpWidget(app(value, controller));
      await tester.pumpAndSettle();
      await tester.tap(
        find.textContaining('message-0', findRichText: true),
        kind: PointerDeviceKind.mouse,
      );
      await tester.pump();
      await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.keyA);
      await tester.pumpAndSettle();
      final list = tester.widget<ListView>(
        find.byKey(const ValueKey('virtual-json-lines')),
      );
      list.controller!.jumpTo(300);
      await tester.pumpAndSettle();
      await tester.sendKeyEvent(LogicalKeyboardKey.keyC);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
      await tester.pumpAndSettle();
      expect(copied, const JsonEncoder.withIndent('  ').convert(value));
      list.controller!.jumpTo(0);
      await tester.pumpAndSettle();
      // A later manual selection must copy only that range, never the gutter.
      final first = tester.getRect(
        find.textContaining('message-0', findRichText: true),
      );
      final second = tester.getRect(
        find.textContaining('message-1"', findRichText: true),
      );
      final pointer = await tester.startGesture(
        first.centerLeft + const Offset(1, 0),
        kind: PointerDeviceKind.mouse,
      );
      await pointer.moveTo(second.centerRight - const Offset(1, 0));
      await pointer.up();
      await tester.pumpAndSettle();
      await tester.sendKeyDownEvent(LogicalKeyboardKey.metaLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.keyC);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.metaLeft);
      await tester.pumpAndSettle();
      expect(copied, contains('message-0'));
      expect(copied, contains('message-1'));
      expect(copied, isNot(contains('message-999')));
      expect(tester.takeException(), isNull);
    },
    variant: TargetPlatformVariant.only(TargetPlatform.macOS),
  );
}
