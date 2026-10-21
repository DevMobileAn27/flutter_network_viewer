import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/l10n/app_localizations.dart';
import 'package:flutter_network_viewer/ui/json_code_view.dart';
import 'package:flutter_network_viewer/ui/json_tree_view.dart';
import 'package:flutter_network_viewer/ui/network_body_section.dart';

void main() {
  testWidgets('JSON and Tree fill the body viewport and long JSON scrolls', (
    tester,
  ) async {
    Future<void> showBody(Object body) async {
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: NetworkBodySection(
              data: {'body': jsonEncode(body)},
              fillAvailable: true,
              showTitle: false,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
    }

    final viewport = find.byKey(const ValueKey('json-body-scroll'));
    void expectFilled(Finder content) {
      expect(
        tester.getRect(content).bottom,
        closeTo(tester.getRect(viewport).bottom, .01),
      );
      expect(
        tester.getSize(content).height,
        closeTo(tester.getSize(viewport).height, .01),
      );
    }

    await showBody({'id': 42});
    expectFilled(find.byType(JsonCodeView));
    await tester.tap(find.byKey(const ValueKey('json-fold-0')));
    await tester.pumpAndSettle();
    expectFilled(find.byType(JsonCodeView));
    await tester.tap(find.text('Tree'));
    await tester.pumpAndSettle();
    expectFilled(find.byType(JsonTreeView));
    await tester.tap(find.text('JSON'));
    await tester.pumpAndSettle();
    await showBody(List.generate(100, (index) => 'item-$index'));
    expect(
      tester.getSize(find.byType(JsonCodeView)).height,
      greaterThan(tester.getSize(viewport).height),
    );
    await tester.drag(viewport, const Offset(0, -300));
    await tester.pumpAndSettle();
    expect(
      tester.getTopLeft(find.byType(JsonCodeView)).dy,
      lessThan(tester.getTopLeft(viewport).dy),
    );
    expect(tester.takeException(), isNull);
  });
}
