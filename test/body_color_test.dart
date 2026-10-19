import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/l10n/app_localizations.dart';
import 'package:flutter_network_viewer/ui/body_color.dart';
import 'package:flutter_network_viewer/ui/components.dart';
import 'package:flutter_network_viewer/ui/json_code_view.dart';

void main() {
  test(
    'recognizes CSS and Flutter colors without guessing IDs or substrings',
    () {
      for (final entry in <String, int>{
        '#B45309': 0xffb45309,
        '#abc': 0xffaabbcc,
        '#abcd': 0xddaabbcc,
        '#11223380': 0x80112233,
        '0x80112233': 0x80112233,
        'rgb(180, 83, 9)': 0xffb45309,
        'rgba(255, 0, 0, 0.5)': 0x80ff0000,
        'rgb(100% 0% 0% / 50%)': 0x80ff0000,
        'hsl(120, 100%, 50%)': 0xff00ff00,
        'hsla(240deg, 100%, 50%, 0.5)': 0x800000ff,
        'white': 0xffffffff,
        ' transparent ': 0x00000000,
      }.entries) {
        expect(
          parseBodyColor(entry.key)?.toARGB32(),
          entry.value,
          reason: entry.key,
        );
      }
      for (final text in [
        'B45309',
        '#12',
        '#12345',
        '#1234567',
        '#xyz',
        'id #abcdef',
        'rgb(300, 0, 0)',
        'rgba(0,0,0,2)',
        'rgb(NaN,0,0)',
        'hsl(0, 5, 2)',
        'url(https://example.com/#abc)',
        '123456',
      ]) {
        expect(parseBodyColor(text), isNull, reason: text);
      }
    },
  );

  for (final brightness in Brightness.values) {
    testWidgets(
      'color previews preserve JSON, search, copy and folding in ${brightness.name}',
      (tester) async {
        const value = {
          '#ff0000': 'key only',
          'textColor': '#B45309',
          'background': 'rgba(255, 0, 0, 0.5)',
          'id': 'B45309',
          'message': 'use #fff here',
        };
        final original = const JsonEncoder.withIndent('  ').convert(value);
        await tester.pumpWidget(
          MaterialApp(
            theme: viewerTheme(brightness),
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: const Scaffold(
              body: SingleChildScrollView(
                child: DataSection('Body', value, body: true, searchable: true),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        expect(find.byType(BodyColorSwatch), findsNWidgets(2));
        String document() => tester
            .widget<SelectableText>(
              find.descendant(
                of: find.byType(JsonCodeView),
                matching: find.byType(SelectableText),
              ),
            )
            .textSpan!
            .toPlainText();
        expect(document(), original);
        await tester.enterText(
          find.byKey(const ValueKey('json-find-input')),
          '#B45309',
        );
        await tester.pumpAndSettle();
        expect(find.text('1 of 1'), findsOneWidget);
        expect(document(), original);
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
        await tester.enterText(
          find.byKey(const ValueKey('json-find-input')),
          '',
        );
        await tester.pumpAndSettle();
        await tester.tap(find.byKey(const ValueKey('json-fold-0')));
        await tester.pumpAndSettle();
        expect(find.byType(BodyColorSwatch), findsNothing);
        await tester.tap(find.byKey(const ValueKey('json-fold-0')));
        await tester.pumpAndSettle();
        expect(find.byType(BodyColorSwatch), findsNWidgets(2));
        await tester.tap(find.byTooltip('Tree view'));
        await tester.pumpAndSettle();
        expect(find.byType(BodyColorSwatch), findsNWidgets(2));
        expect(tester.takeException(), isNull);
      },
    );
  }
}
