import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/main.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';
import 'package:flutter_network_viewer/ui/json_code_view.dart';
import 'package:flutter_network_viewer/ui/json_tree_view.dart';

String document(WidgetTester tester) => tester
    .widget<SelectableText>(
      find.descendant(
        of: find.byType(JsonCodeView),
        matching: find.byType(SelectableText),
      ),
    )
    .textSpan!
    .toPlainText();

void main() {
  testWidgets('beautify divider resizes panes without losing edits or folds', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1360, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final store = CaptureStore();
    final server = ViewerServer(store);
    await tester.pumpWidget(ViewerApp(store: store, server: server));
    await tester.tap(find.text('Json beautify'));
    await tester.pumpAndSettle();
    final input = find.byKey(const ValueKey('json-beautify-input'));
    final left = find.byKey(const ValueKey('json-beautify-input-pane'));
    final right = find.byKey(const ValueKey('json-beautify-output-pane'));
    final divider = find.byKey(const ValueKey('json-beautify-divider'));
    double paneWidth(Finder pane) => tester.getSize(pane).width;
    expect(paneWidth(left), paneWidth(right));
    const source = '{"meta":{"name":"An"},"items":[1,true,null]}';
    await tester.enterText(input, source);
    await tester.tap(find.byKey(const ValueKey('format-json')));
    await tester.pumpAndSettle();
    await tester.tap(find.byKey(const ValueKey('json-fold-1')));
    await tester.pumpAndSettle();
    final folded = document(tester);
    expect(folded, contains('"meta": { … },'));

    final initialWidth = paneWidth(left);
    await tester.drag(divider, const Offset(-180, 0));
    await tester.pumpAndSettle();
    expect(paneWidth(left), lessThan(initialWidth - 140));
    expect(paneWidth(right), greaterThan(initialWidth + 140));
    expect(document(tester), folded);
    expect(tester.widget<TextField>(input).controller!.text, source);
    expect(tester.getSize(find.byType(JsonCodeView)).width, paneWidth(right));

    await tester.drag(divider, const Offset(-2000, 0));
    await tester.pumpAndSettle();
    expect(paneWidth(left), 240);
    await tester.drag(divider, const Offset(2000, 0));
    await tester.pumpAndSettle();
    expect(paneWidth(right), closeTo(240, .01));
    tester.view.physicalSize = const Size(860, 800);
    await tester.pumpAndSettle();
    expect(paneWidth(left), greaterThanOrEqualTo(240));
    expect(paneWidth(right), greaterThanOrEqualTo(240));
    expect(tester.takeException(), isNull);

    final retainedWidth = paneWidth(left);
    await tester.tap(find.text('Console'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('Json beautify'));
    await tester.pumpAndSettle();
    expect(paneWidth(left), retainedWidth);
    expect(document(tester), folded);
    await tester.tap(divider);
    await tester.pump(const Duration(milliseconds: 50));
    await tester.tap(divider);
    await tester.pumpAndSettle();
    expect(paneWidth(left), closeTo(paneWidth(right), .01));

    await tester.ensureVisible(find.text('Tree'));
    await tester.tap(find.text('Tree'));
    await tester.pumpAndSettle();
    await tester.tap(find.textContaining('meta: {}'));
    await tester.pumpAndSettle();
    await tester.drag(divider, const Offset(120, 0));
    await tester.pumpAndSettle();
    expect(find.textContaining('name: "An"'), findsOneWidget);
    expect(tester.widget<TextField>(input).controller!.text, source);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox());
    server.dispose();
    store.dispose();
  });

  for (final width in [860.0, 1360.0]) {
    testWidgets(
      'beautify Find retains 2 of 3 across views, tabs and resize at $width',
      (tester) async {
        tester.view.physicalSize = Size(width, 800);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final store = CaptureStore();
        final server = ViewerServer(store);
        await tester.pumpWidget(ViewerApp(store: store, server: server));
        await tester.tap(find.text('Json beautify'));
        await tester.pumpAndSettle();
        final source = find.byKey(const ValueKey('json-beautify-input'));
        final search = find.byKey(const ValueKey('json-find-input'));
        final counter = find.byKey(const ValueKey('json-find-count'));
        expect(search, findsNothing);
        await tester.enterText(
          source,
          '{"items":[{"name":"needle"},{"name":"needle"},{"name":"needle"}]}',
        );
        await tester.tap(find.byKey(const ValueKey('format-json')));
        await tester.pumpAndSettle();
        expect(search, findsOneWidget);
        expect(
          tester.getSize(find.byKey(const ValueKey('json-find-bar'))).width,
          400,
        );
        expect(tester.widget<Text>(counter).data, 'No results');
        await tester.enterText(search, 'needle');
        await tester.pumpAndSettle();
        await tester.ensureVisible(
          find.byKey(const ValueKey('json-find-next')),
        );
        await tester.tap(find.byKey(const ValueKey('json-find-next')));
        await tester.pumpAndSettle();
        expect(tester.widget<Text>(counter).data, '2 of 3');
        final code = tester.widget<JsonCodeView>(find.byType(JsonCodeView));
        expect(code.find!.active!.path, ['items', 1, 'name']);
        await tester.ensureVisible(find.text('Tree'));
        await tester.tap(find.text('Tree'));
        await tester.pumpAndSettle();
        expect(
          tester
              .widget<JsonTreeView>(find.byType(JsonTreeView))
              .find!
              .active!
              .path,
          ['items', 1, 'name'],
        );
        expect(find.textContaining('name: "needle"'), findsOneWidget);
        expect(tester.widget<Text>(counter).data, '2 of 3');
        await tester.drag(
          find.byKey(const ValueKey('json-beautify-divider')),
          const Offset(100, 0),
        );
        await tester.pumpAndSettle();
        expect(
          tester.getSize(find.byKey(const ValueKey('json-find-bar'))).width,
          400,
        );
        await tester.tap(find.text('Console'));
        await tester.pumpAndSettle();
        await tester.tap(find.text('Json beautify'));
        await tester.pumpAndSettle();
        expect(tester.widget<TextField>(search).controller!.text, 'needle');
        expect(tester.widget<Text>(counter).data, '2 of 3');
        await tester.enterText(source, '{"updated":"needle"}');
        await tester.tap(find.byKey(const ValueKey('format-json')));
        await tester.pumpAndSettle();
        expect(tester.widget<Text>(counter).data, '1 of 1');
        expect(find.textContaining('updated: "needle"'), findsOneWidget);
        await tester.enterText(source, '{invalid');
        await tester.tap(find.byKey(const ValueKey('format-json')));
        await tester.pumpAndSettle();
        expect(find.text('Invalid JSON'), findsOneWidget);
        await tester.tap(find.text('Close'));
        await tester.pumpAndSettle();
        expect(tester.widget<Text>(counter).data, '1 of 1');
        await tester.enterText(source, 'null');
        await tester.tap(find.byKey(const ValueKey('format-json')));
        await tester.pumpAndSettle();
        expect(tester.widget<Text>(counter).data, 'No results');
        // The same shortcut focuses Find even while the JSON input has focus.
        await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
        await tester.sendKeyEvent(LogicalKeyboardKey.keyF);
        await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
        await tester.pumpAndSettle();
        expect(tester.widget<TextField>(search).focusNode!.hasFocus, isTrue);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox());
        server.dispose();
        store.dispose();
      },
    );
  }

  for (final width in [860.0, 1360.0]) {
    testWidgets(
      'beautify formats JSON, switches views and retains drafts at $width',
      (tester) async {
        tester.view.physicalSize = Size(width, 800);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final store = CaptureStore();
        final server = ViewerServer(store);
        await tester.pumpWidget(ViewerApp(store: store, server: server));
        await tester.tap(find.text('Json beautify'));
        await tester.pumpAndSettle();
        final input = find.byKey(const ValueKey('json-beautify-input'));
        final format = find.byKey(const ValueKey('format-json'));
        const source = '{"meta":{"name":"An"},"items":[1,true,null]}';
        await tester.enterText(input, source);
        await tester.tap(format);
        await tester.pumpAndSettle();
        expect(
          document(tester),
          const JsonEncoder.withIndent('  ').convert(jsonDecode(source)),
        );
        await tester.ensureVisible(find.text('Tree'));
        await tester.tap(find.text('Tree'));
        await tester.pumpAndSettle();
        expect(find.byType(JsonTreeView), findsOneWidget);
        await tester.tap(find.textContaining('meta: {}'));
        await tester.pumpAndSettle();
        expect(find.textContaining('name: "An"'), findsOneWidget);

        const invalid = '{\n "value":\n}';
        await tester.enterText(input, invalid);
        await tester.tap(find.text('Console'));
        await tester.pumpAndSettle();
        await tester.tap(find.text('Json beautify'));
        await tester.pumpAndSettle();
        expect(tester.widget<TextField>(input).controller!.text, invalid);
        expect(find.byType(JsonTreeView), findsOneWidget);
        expect(find.textContaining('name: "An"'), findsOneWidget);
        await tester.tap(format);
        await tester.pumpAndSettle();
        expect(find.text('Invalid JSON'), findsOneWidget);
        expect(find.text('Line 3, column 1'), findsOneWidget);
        await tester.tap(find.text('Close'));
        await tester.pumpAndSettle();
        expect(tester.widget<TextField>(input).controller!.text, invalid);
        expect(find.textContaining('name: "An"'), findsOneWidget);

        // JSON null is a valid result, not an empty output state.
        await tester.enterText(input, 'null');
        await tester.tap(format);
        await tester.pumpAndSettle();
        expect(
          tester.widget<JsonTreeView>(find.byType(JsonTreeView)).value,
          isNull,
        );
        await tester.ensureVisible(find.text('JSON'));
        await tester.tap(find.text('JSON'));
        await tester.pumpAndSettle();
        expect(document(tester), 'null');

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
        await tester.ensureVisible(find.byTooltip('Copy formatted JSON'));
        await tester.tap(find.byTooltip('Copy formatted JSON'));
        await tester.pumpAndSettle();
        expect(copied, 'null');
        await tester.enterText(input, '');
        await tester.tap(format);
        await tester.pumpAndSettle();
        expect(find.text('Invalid JSON'), findsOneWidget);
        await tester.tap(find.text('Close'));
        await tester.pumpAndSettle();
        expect(document(tester), 'null');
        await tester.enterText(input, '42');
        await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
        await tester.sendKeyEvent(LogicalKeyboardKey.enter);
        await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
        await tester.pumpAndSettle();
        expect(document(tester), '42');
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox());
        server.dispose();
        store.dispose();
      },
    );
  }
}
