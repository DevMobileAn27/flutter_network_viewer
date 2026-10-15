import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/l10n/app_localizations.dart';
import 'package:flutter_network_viewer/models/capture.dart';
import 'package:flutter_network_viewer/ui/components.dart';
import 'package:flutter_network_viewer/ui/inspector.dart';
import 'package:flutter_network_viewer/ui/json_code_view.dart';
import 'package:flutter_network_viewer/ui/json_find_controller.dart';
import 'package:flutter_network_viewer/ui/json_tree_view.dart';
import 'package:flutter_network_viewer/ui/network_body_section.dart';

Widget app(Widget child, {Brightness brightness = Brightness.light}) =>
    MaterialApp(
      theme: viewerTheme(brightness),
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(body: child),
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

final input = find.byKey(const ValueKey('json-find-input'));
final count = find.byKey(const ValueKey('json-find-count'));
String countText(WidgetTester tester) => tester.widget<Text>(count).data!;

void main() {
  test(
    'find document matches JsonEncoder for nested, escaped and scalar values',
    () {
      final controller = JsonFindController();
      addTearDown(controller.dispose);
      for (final value in [
        {
          'quoted"key': [
            'Cà phê',
            true,
            null,
            {},
            [],
            {'emoji': '💙\nline'},
          ],
          'tail': 42,
        },
        {},
        [],
        'hello',
        null,
        false,
        12.5,
      ]) {
        controller.setValue(value);
        expect(
          controller.document,
          const JsonEncoder.withIndent('  ').convert(value),
        );
      }
    },
  );

  test(
    'case, whole word, regex, wraparound and invalid/zero width expressions',
    () {
      final controller = JsonFindController()
        ..setValue({
          'name': 'Chi chi child CHI',
          'nested': {'value': 'chi'},
        });
      addTearDown(controller.dispose);
      controller.update(query: 'chi');
      expect(controller.matches, hasLength(5));
      expect(controller.active!.path, ['name']);
      controller.move(-1);
      expect(controller.index, 4);
      expect(controller.active!.path, ['nested', 'value']);
      controller.move(1);
      expect(controller.index, 0);
      controller.update(matchCase: true);
      expect(controller.matches, hasLength(3));
      controller.update(wholeWord: true);
      expect(controller.matches, hasLength(2));
      controller.update(query: 'ch[iI]', regex: true, matchCase: false);
      expect(controller.matches, hasLength(4));
      controller.update(query: '[');
      expect(controller.error, isNotNull);
      expect(controller.active, isNull);
      controller.move(1);
      expect(controller.index, -1);
      controller.update(query: '^');
      expect(controller.matches, isEmpty);
      controller.update(query: '');
      expect(controller.error, isNull);
    },
  );

  test(
    'highlight spans preserve original text and find paths for keys and array values',
    () {
      final controller = JsonFindController()
        ..setValue({
          'chi': [
            {'name': 'chi'},
          ],
        });
      addTearDown(controller.dispose);
      controller.update(query: 'chi');
      expect(controller.matches.map((m) => m.path), [
        ['chi'],
        ['chi', 0, 'name'],
      ]);
      final spans = controller.spans(controller.document, 0);
      expect(TextSpan(children: spans).toPlainText(), controller.document);
      expect(
        spans.where((s) => s.style?.backgroundColor != null),
        hasLength(2),
      );
      expect(
        spans.where((s) => s.style?.backgroundColor == const Color(0xfffb923c)),
        hasLength(1),
      );
    },
  );

  for (final width in [320.0, 740.0]) {
    for (final brightness in Brightness.values) {
      testWidgets(
        'find highlights, unfolds, navigates and stays visible at $width ${brightness.name}',
        (tester) async {
          tester.view.physicalSize = Size(width, 700);
          tester.view.devicePixelRatio = 1;
          addTearDown(tester.view.resetPhysicalSize);
          addTearDown(tester.view.resetDevicePixelRatio);
          final value = {
            'nested': {
              'items': [
                for (var i = 0; i < 60; i++) {'name': 'item $i'},
              ],
            },
            'tail': 'target',
          };
          await tester.pumpWidget(
            app(
              NetworkBodySection(
                data: {'body': jsonEncode(value)},
                fillAvailable: true,
              ),
              brightness: brightness,
            ),
          );
          await tester.pumpAndSettle();
          expect(
            tester.getTopLeft(input).dx,
            lessThan(tester.getTopLeft(find.byTooltip('Formatted JSON')).dx),
          );
          expect(
            tester.getSize(find.byKey(const ValueKey('json-find-bar'))).width,
            400,
          );
          expect(
            tester
                .getSize(find.byKey(const ValueKey('json-find-input-box')))
                .width,
            240,
          );
          final inputTop = tester.getTopLeft(input).dy;
          await tester.tap(find.byKey(const ValueKey('json-fold-1')));
          await tester.pumpAndSettle();
          expect(document(tester), isNot(contains('item 59')));
          await tester.enterText(input, 'item 59');
          await tester.pumpAndSettle();
          expect(countText(tester), '1 of 1');
          expect(document(tester), contains('item 59'));
          final view = tester.widget<JsonCodeView>(find.byType(JsonCodeView));
          expect(view.find!.active!.path, ['nested', 'items', 59, 'name']);
          final vertical = tester.widget<SingleChildScrollView>(
            find.byKey(const ValueKey('json-body-scroll')),
          );
          final position = tester
              .state<ScrollableState>(
                find
                    .descendant(
                      of: find.byKey(const ValueKey('json-body-scroll')),
                      matching: find.byType(Scrollable),
                    )
                    .first,
              )
              .position;
          expect(vertical.scrollDirection, Axis.vertical);
          expect(position.pixels, greaterThan(0));
          expect(tester.getTopLeft(input).dy, inputTop);
          final code = tester.widget<SelectableText>(
            find.descendant(
              of: find.byType(JsonCodeView),
              matching: find.byType(SelectableText),
            ),
          );
          final spans = code.textSpan!.children!.cast<TextSpan>();
          expect(
            spans.any(
              (span) => span.style?.backgroundColor == const Color(0xfffb923c),
            ),
            isTrue,
          );
          await tester.ensureVisible(find.byTooltip('Tree view'));
          await tester.tap(find.byTooltip('Tree view'));
          await tester.pumpAndSettle();
          expect(find.byType(JsonTreeView), findsOneWidget);
          expect(find.textContaining('name: "item 59"'), findsOneWidget);
          expect(tester.widget<TextField>(input).controller!.text, 'item 59');
          await tester.enterText(input, 'item');
          await tester.pumpAndSettle();
          expect(countText(tester), '1 of 61'); // key "items" + 60 values
          await tester.sendKeyEvent(LogicalKeyboardKey.enter);
          await tester.pumpAndSettle();
          expect(countText(tester), '2 of 61');
          await tester.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
          await tester.sendKeyEvent(LogicalKeyboardKey.enter);
          await tester.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
          await tester.pumpAndSettle();
          expect(countText(tester), '1 of 61');
          await tester.ensureVisible(
            find.byKey(const ValueKey('json-find-word')),
          );
          await tester.tap(find.byKey(const ValueKey('json-find-word')));
          await tester.pumpAndSettle();
          expect(countText(tester), '1 of 60');
          await tester.ensureVisible(
            find.byKey(const ValueKey('json-find-regex')),
          );
          await tester.tap(find.byKey(const ValueKey('json-find-regex')));
          await tester.enterText(input, '[');
          await tester.pumpAndSettle();
          expect(countText(tester), 'Invalid regex');
          await tester.enterText(input, 'not-found');
          await tester.pumpAndSettle();
          expect(countText(tester), 'No results');
          expect(
            tester
                .widget<IconButton>(
                  find.byKey(const ValueKey('json-find-next')),
                )
                .onPressed,
            isNull,
          );
          await tester.sendKeyEvent(LogicalKeyboardKey.escape);
          await tester.pumpAndSettle();
          expect(tester.widget<TextField>(input).controller!.text, '');
          expect(countText(tester), 'No results');
          expect(tester.takeException(), isNull);
        },
      );
    }
  }

  for (final brightness in Brightness.values) {
    testWidgets(
      'No results and 2 of 3 retain layout and highlight in ${brightness.name}',
      (tester) async {
        tester.view.physicalSize = const Size(758, 500);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        await tester.pumpWidget(
          app(
            const NetworkBodySection(
              data: {'body': '{"items":["needle","needle","needle"]}'},
              fillAvailable: true,
            ),
            brightness: brightness,
          ),
        );
        await tester.pumpAndSettle();
        expect(countText(tester), 'No results');
        final tracked = [
          'json-find-input-box',
          'json-find-count',
          'json-find-case',
          'json-find-previous',
          'json-find-next',
          'json-find-clear',
        ];
        final rectangles = {
          for (final key in tracked)
            key: tester.getRect(find.byKey(ValueKey(key))),
        };
        await tester.enterText(input, 'needle');
        await tester.pumpAndSettle();
        expect(countText(tester), '1 of 3');
        await tester.tap(find.byKey(const ValueKey('json-find-next')));
        await tester.pumpAndSettle();
        expect(countText(tester), '2 of 3');
        final view = tester.widget<JsonCodeView>(find.byType(JsonCodeView));
        expect(view.find!.active!.path, ['items', 1]);
        final code = tester.widget<SelectableText>(
          find.descendant(
            of: find.byType(JsonCodeView),
            matching: find.byType(SelectableText),
          ),
        );
        final highlights = code.textSpan!.children!
            .cast<TextSpan>()
            .where((span) => span.style?.backgroundColor != null)
            .toList();
        expect(highlights, hasLength(3));
        expect(highlights[1].style!.backgroundColor, const Color(0xfffb923c));
        expect(highlights[0].style!.backgroundColor, const Color(0xfffde68a));
        expect(highlights[2].style!.backgroundColor, const Color(0xfffde68a));
        // Different counter text must not shift the options/navigation.
        for (final key in tracked) {
          expect(tester.getRect(find.byKey(ValueKey(key))), rectangles[key]);
        }
        await tester.tap(find.byTooltip('Tree view'));
        await tester.pumpAndSettle();
        expect(countText(tester), '2 of 3');
        expect(
          tester
              .widget<JsonTreeView>(find.byType(JsonTreeView))
              .find!
              .active!
              .path,
          ['items', 1],
        );
        await tester.tap(find.byKey(const ValueKey('json-find-next')));
        await tester.pumpAndSettle();
        expect(countText(tester), '3 of 3');
        await tester.tap(find.byKey(const ValueKey('json-find-next')));
        await tester.pumpAndSettle();
        expect(countText(tester), '1 of 3');
        await tester.tap(find.byKey(const ValueKey('json-find-previous')));
        await tester.pumpAndSettle();
        expect(countText(tester), '3 of 3');
        await tester.enterText(input, 'absent');
        await tester.pumpAndSettle();
        expect(countText(tester), 'No results');
        await tester.tap(find.byKey(const ValueKey('json-find-clear')));
        await tester.pumpAndSettle();
        expect(countText(tester), 'No results');
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets(
    'find scrolls horizontally and Cmd/Ctrl+F focuses the local search',
    (tester) async {
      tester.view.physicalSize = const Size(400, 500);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      await tester.pumpWidget(
        app(
          NetworkBodySection(
            data: {
              'body': jsonEncode({'long': '${'prefix ' * 60}far-needle'}),
            },
            fillAvailable: true,
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.enterText(input, 'far-needle');
      await tester.pumpAndSettle();
      final horizontal = tester.widget<SingleChildScrollView>(
        find.descendant(
          of: find.byType(JsonCodeView),
          matching: find.byType(SingleChildScrollView),
        ),
      );
      expect(horizontal.controller!.offset, greaterThan(0));
      for (final modifier in [
        LogicalKeyboardKey.metaLeft,
        LogicalKeyboardKey.controlLeft,
      ]) {
        await tester.ensureVisible(
          find.byKey(const ValueKey('json-find-clear')),
        );
        await tester.tap(find.byKey(const ValueKey('json-find-clear')));
        await tester.pumpAndSettle();
        await tester.tapAt(
          tester.getTopLeft(find.byType(JsonCodeView)) + const Offset(100, 40),
        );
        await tester.sendKeyDownEvent(modifier);
        await tester.sendKeyEvent(LogicalKeyboardKey.keyF);
        await tester.sendKeyUpEvent(modifier);
        await tester.pumpAndSettle();
        expect(tester.widget<TextField>(input).focusNode!.hasFocus, isTrue);
      }
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'find appears only for valid JSON, not text, images, base64 or headers',
    (tester) async {
      for (final body in ['{}', '[]', 'null', '42', 'false', '"text"']) {
        await tester.pumpWidget(
          app(NetworkBodySection(data: {'body': body}, fillAvailable: true)),
        );
        await tester.pumpAndSettle();
        expect(input, findsOneWidget);
      }
      for (final data in [
        {'body': 'plain text'},
        {'body': '<html>hello</html>'},
        {'body': '{"incomplete":'},
        {'body': '1234', 'bodyEncoding': 'base64'},
        {
          'body': base64Encode(
            File('test/fixtures/preview.png').readAsBytesSync(),
          ),
          'bodyEncoding': 'base64',
          'headers': {'content-type': 'image/png'},
        },
      ]) {
        await tester.pumpWidget(
          app(NetworkBodySection(data: data, fillAvailable: true)),
        );
        await tester.pumpAndSettle();
        expect(input, findsNothing);
      }
      await tester.pumpWidget(
        app(
          const SingleChildScrollView(
            child: DataSection('Headers', {'content-type': 'application/json'}),
          ),
        ),
      );
      expect(input, findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'both request and response bodies support find; replacing data refreshes results',
    (tester) async {
      tester.view.physicalSize = const Size(800, 700);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final record = NetworkRecord('app', 'request')
        ..apply({
          'phase': 'request',
          'request': {
            'url': 'https://example.com/',
            'body': '{"request":"needle"}',
          },
        })
        ..apply({
          'phase': 'response',
          'response': {'body': '{"response":"needle needle"}'},
        });
      await tester.pumpWidget(
        app(NetworkInspector(record: record, initialTab: 2)),
      );
      await tester.pumpAndSettle();
      await tester.enterText(input, 'needle');
      await tester.pumpAndSettle();
      expect(countText(tester), '1 of 1');
      await tester.ensureVisible(find.text('Response body'));
      await tester.tap(find.text('Response body'));
      await tester.pumpAndSettle();
      await tester.enterText(input, 'needle');
      await tester.pumpAndSettle();
      expect(countText(tester), '1 of 2');
      await tester.tap(find.byKey(const ValueKey('json-find-next')));
      await tester.pumpAndSettle();
      expect(countText(tester), '2 of 2');
      record.apply({
        'phase': 'response',
        'response': {'body': '{"updated":"needle"}'},
      });
      await tester.pumpWidget(
        app(NetworkInspector(record: record, initialTab: 2)),
      );
      await tester.pumpAndSettle();
      expect(countText(tester), '1 of 1');
      expect(document(tester), contains('updated'));
      expect(tester.takeException(), isNull);
    },
  );
}
