import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/l10n/app_localizations.dart';
import 'package:flutter_network_viewer/ui/components.dart';
import 'package:flutter_network_viewer/ui/json_code_view.dart';
import 'package:flutter_network_viewer/ui/json_tree_view.dart';

Widget bodyApp(dynamic value, {Brightness brightness = Brightness.light}) =>
    MaterialApp(
      theme: viewerTheme(brightness),
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(
        body: SingleChildScrollView(
          child: Column(
            children: [
              const DataSection('Headers', {
                'x-test': 'hidden',
              }, initiallyExpanded: false),
              DataSection('Body', value, body: true),
            ],
          ),
        ),
      ),
    );

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
  testWidgets(
    'JSON folds nested ranges, preserves line numbers and copies full data',
    (tester) async {
      final value = {
        'data': {
          'items': [
            {'name': 'A', 'syntax': 'braces } { ] [ " \\'},
          ],
          'empty': <String, dynamic>{},
        },
        'tail': true,
      };
      final original = const JsonEncoder.withIndent('  ').convert(value);
      await tester.pumpWidget(bodyApp(jsonEncode(value)));
      await tester.pumpAndSettle();
      expect(find.byTooltip('Click to collapse the range'), findsNWidgets(4));
      await tester.tap(find.byKey(const ValueKey('json-fold-2')));
      await tester.pumpAndSettle();
      expect(document(tester), contains('"items": [ … ],'));
      expect(document(tester), isNot(contains('"name"')));
      final lines = tester
          .widget<Text>(find.byKey(const ValueKey('json-line-numbers')))
          .data!
          .split('\n');
      expect(
        tester.getSize(find.byKey(const ValueKey('json-line-numbers'))).height,
        closeTo(tester.getSize(find.byType(SelectableText).last).height, .01),
        reason:
            'Multi-digit line numbers must stay on the same rows as the JSON',
      );
      expect(lines.take(3), ['1', '2', '3']);
      expect(lines.last, '${original.split('\n').length}');
      expect(lines, isNot(contains('4')));
      final codeLeft = tester.getTopLeft(find.byType(SelectableText).last).dx;
      await tester.tap(find.byKey(const ValueKey('json-fold-0')));
      await tester.pumpAndSettle();
      expect(document(tester), '{ … }');
      expect(find.byTooltip('Click to expand the range'), findsOneWidget);
      expect(tester.getTopLeft(find.byType(SelectableText).last).dx, codeLeft);
      await tester.tap(find.byKey(const ValueKey('json-fold-0')));
      await tester.pumpAndSettle();
      expect(document(tester), contains('"items": [ … ],'));

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
      await tester.tap(find.byTooltip('Copy Body'));
      await tester.pumpAndSettle();
      expect(jsonDecode(copied!), value);
      await tester.tap(find.byKey(const ValueKey('json-fold-2')));
      await tester.pumpAndSettle();
      expect(document(tester), original);
      await tester.tap(find.byKey(const ValueKey('json-fold-0')));
      await tester.pumpAndSettle();
      await tester.pumpWidget(bodyApp('{"changed": [1, 2]}'));
      await tester.pumpAndSettle();
      expect(jsonDecode(document(tester)), {
        'changed': [1, 2],
      });
      expect(find.byTooltip('Click to expand the range'), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('terminal tree preserves branches, array indexes and folding', (
    tester,
  ) async {
    await tester.pumpWidget(
      bodyApp(
        jsonEncode({
          'data': {
            'items': [
              {'name': 'A', 'active': true},
            ],
            'empty': <String, dynamic>{},
          },
          'last': {'leaf': null},
        }),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip('Tree view'));
    await tester.pumpAndSettle();
    expect(find.byType(JsonTreeView), findsOneWidget);
    expect(find.textContaining('name: "A"'), findsNothing);
    expect(find.textContaining('├── ▸ data: {}'), findsOneWidget);
    expect(find.textContaining('└── ▸ last: {}'), findsOneWidget);
    await tester.tap(find.textContaining('data: {}'));
    await tester.pumpAndSettle();
    expect(find.textContaining('│   ├── ▸ items: []'), findsOneWidget);
    expect(find.text('│   └──   empty: {}'), findsOneWidget);
    await tester.tap(find.textContaining('items: []'));
    await tester.pumpAndSettle();
    expect(find.textContaining('│   │   └── ▸ [0]: {}'), findsOneWidget);
    await tester.tap(find.textContaining('[0]: {}'));
    await tester.pumpAndSettle();
    expect(find.text('│   │       ├──   name: "A"'), findsOneWidget);
    expect(find.text('│   │       └──   active: true'), findsOneWidget);
    await tester.tap(find.textContaining('data: {}'));
    await tester.pumpAndSettle();
    expect(find.textContaining('name: "A"'), findsNothing);
    await tester.tap(find.textContaining('last: {}'));
    await tester.pumpAndSettle();
    expect(find.text('    └──   leaf: null'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  for (final brightness in Brightness.values) {
    testWidgets(
      'JSON body formats, selects and scrolls in ${brightness.name}',
      (tester) async {
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
        tester.view.physicalSize = const Size(320, 800);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final value = {
          'object': null,
          'message': 'Tài khoản hoặc mật khẩu không đúng',
          'status': 401,
          'nested': [
            {'escaped': 'a "quote" \\ newline\n', 'ok': true},
          ],
          'long': List.filled(40, 'long value').join(' '),
        };
        await tester.pumpWidget(
          bodyApp(jsonEncode(value), brightness: brightness),
        );
        await tester.pumpAndSettle();
        expect(find.text('x-test: "hidden"'), findsNothing);
        expect(document(tester), contains('\n  "message":'));
        expect(jsonDecode(document(tester)), value);
        final horizontal = find.descendant(
          of: find.byType(JsonCodeView),
          matching: find.byType(SingleChildScrollView),
        );
        final scroll = tester
            .widget<SingleChildScrollView>(horizontal)
            .controller!;
        expect(scroll.position.maxScrollExtent, greaterThan(100));
        await tester.sendEventToBinding(
          PointerScrollEvent(
            position: tester.getCenter(horizontal),
            scrollDelta: const Offset(180, 0),
          ),
        );
        await tester.pumpAndSettle();
        expect(scroll.offset, greaterThan(0));
        await tester.tap(find.byTooltip('Copy Body'));
        await tester.pumpAndSettle();
        expect(jsonDecode(copied!), value);
        await tester.tap(find.byTooltip('Tree view'));
        await tester.pumpAndSettle();
        expect(find.byType(JsonCodeView), findsNothing);
        expect(find.textContaining('status: 401'), findsOneWidget);
        await tester.tap(find.byTooltip('Formatted JSON'));
        await tester.pumpAndSettle();
        expect(jsonDecode(document(tester)), value);
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets(
    'empty objects and scalar JSON display; invalid JSON stays verbatim',
    (tester) async {
      for (final input in ['{}', '[]', 'null', 'false', '42', '"Tiếng Việt"']) {
        await tester.pumpWidget(bodyApp(input));
        await tester.pumpAndSettle();
        expect(jsonDecode(document(tester)), jsonDecode(input));
        expect(find.byTooltip('Click to collapse the range'), findsNothing);
      }
      const plain = '{"truncated": "unfinished';
      await tester.pumpWidget(bodyApp(plain));
      await tester.pumpAndSettle();
      expect(find.byType(JsonCodeView), findsNothing);
      expect(find.text(plain), findsOneWidget);
      expect(find.byTooltip('Tree view'), findsNothing);
    },
  );
}
