import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/main.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';
import 'package:flutter_network_viewer/ui/viewer_dropdown.dart';
import 'package:flutter_network_viewer/ui/json_code_view.dart';

void main() {
  testWidgets(
    'filter menu selects a method and dismisses with Escape or outside tap',
    (tester) async {
      tester.view.physicalSize = const Size(860, 740);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final store = CaptureStore()
        ..ingest(
          jsonDecode(
            File('test/fixtures/sdk_envelope.json').readAsStringSync(),
          ),
        );
      final server = ViewerServer(store);
      await tester.pumpWidget(ViewerApp(store: store, server: server));
      await tester.tap(find.text('All methods'));
      await tester.pumpAndSettle();
      expect(find.text('OPTIONS').hitTestable(), findsOneWidget);
      expect(find.byIcon(Icons.check_rounded), findsOneWidget);
      await tester.tap(
        find.descendant(
          of: find.byType(MenuItemButton),
          matching: find.text('POST'),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byType(MenuItemButton), findsNothing);
      expect(find.text('/orders?draft=true'), findsOneWidget);
      expect(find.text('/feed'), findsNothing);
      expect(find.text('/profile'), findsNothing);

      final dropdown = find.widgetWithText(ViewerDropdown<String>, 'POST');
      await tester.tap(dropdown);
      await tester.pumpAndSettle();
      await tester.sendKeyEvent(LogicalKeyboardKey.escape);
      await tester.pumpAndSettle();
      expect(find.byType(MenuItemButton), findsNothing);
      await tester.tap(dropdown);
      await tester.pumpAndSettle();
      await tester.tapAt(const Offset(10, 10));
      await tester.pumpAndSettle();
      expect(find.byType(MenuItemButton), findsNothing);
      expect(find.text('/orders?draft=true'), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      server.dispose();
      store.dispose();
    },
  );

  testWidgets('inspector dialog updates when a pending request completes', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(860, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final fixture =
        jsonDecode(File('test/fixtures/sdk_envelope.json').readAsStringSync())
            as Map<String, dynamic>;
    final store = CaptureStore()..ingest(fixture);
    final server = ViewerServer(store);
    await tester.pumpWidget(ViewerApp(store: store, server: server));
    await tester.tap(find.text('/feed'));
    await tester.pumpAndSettle();
    expect(find.byType(Dialog), findsOneWidget);
    store.ingest({
      ...fixture,
      'events': [
        {
          ...fixture['events'][1] as Map<String, dynamic>,
          'requestId': 'fetch-3',
        },
      ],
    });
    await tester.pumpAndSettle();
    expect(
      find.descendant(
        of: find.byType(Dialog),
        matching: find.textContaining('201'),
      ),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox());
    server.dispose();
    store.dispose();
  });

  testWidgets(
    'desktop shows network details, console objects, filters and pause',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 900);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final store = CaptureStore()
        ..ingest(
          jsonDecode(
            File('test/fixtures/sdk_envelope.json').readAsStringSync(),
          ),
        );
      final server = ViewerServer(store);
      await tester.pumpWidget(ViewerApp(store: store, server: server));
      expect(find.text('Example Shop'), findsOneWidget);
      await tester.tap(find.text('/orders?draft=true'));
      await tester.pumpAndSettle();
      expect(
        find.text('https://api.example.com/orders?draft=true'),
        findsOneWidget,
      );
      for (final tab in [
        'Request header',
        'Request body',
        'Response header',
        'Response body',
      ]) {
        await tester.ensureVisible(find.text(tab));
        await tester.tap(find.text(tab));
        await tester.pumpAndSettle();
        expect(find.text('HEADERS'), findsNothing);
        expect(find.text('BODY'), findsNothing);
      }
      await tester.ensureVisible(find.text('Response body'));
      await tester.tap(find.text('Response body'));
      await tester.pumpAndSettle();
      expect(find.textContaining('"id": 42'), findsOneWidget);
      final pane = find.byKey(const ValueKey('inspector-pane'));
      final divider = find.byKey(const ValueKey('inspector-divider'));
      void expectJsonFillsInspector() {
        final json = find.byType(JsonCodeView);
        final expectedWidth = tester.getSize(pane).width - 36;
        expect(tester.getSize(json).width, closeTo(expectedWidth, .01));
        final viewport = find.descendant(
          of: json,
          matching: find.byType(SingleChildScrollView),
        );
        expect(
          tester.getSize(viewport).width,
          closeTo(expectedWidth - 2, .01),
          reason: 'The viewport fills the frame inside its 1px border',
        );
      }

      expectJsonFillsInspector();
      final initialWidth = tester.getSize(pane).width;
      await tester.drag(divider, const Offset(-240, 0));
      await tester.pumpAndSettle();
      expect(tester.getSize(pane).width, greaterThan(initialWidth + 200));
      expectJsonFillsInspector();
      expect(find.textContaining('"id": 42'), findsOneWidget);
      await tester.drag(divider, const Offset(-2000, 0));
      await tester.pumpAndSettle();
      expect(
        tester.getSize(find.byKey(const ValueKey('capture-list-pane'))).width,
        greaterThanOrEqualTo(360),
      );
      tester.view.physicalSize = const Size(1200, 900);
      await tester.pumpAndSettle();
      expect(tester.getSize(pane).width, lessThanOrEqualTo(832));
      expectJsonFillsInspector();
      expect(tester.takeException(), isNull);
      await tester.drag(divider, const Offset(2000, 0));
      await tester.pumpAndSettle();
      expect(tester.getSize(pane).width, 320);
      expectJsonFillsInspector();
      await tester.tap(find.byKey(const ValueKey('json-fold-0')));
      await tester.pumpAndSettle();
      expectJsonFillsInspector();
      await tester.tap(divider);
      await tester.pump(const Duration(milliseconds: 50));
      await tester.tap(divider);
      await tester.pumpAndSettle();
      expect(tester.getSize(pane).width, (1200 - 8) / 2);

      await tester.tap(find.text('Console'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Order created {"id":42}'));
      await tester.pumpAndSettle();
      expect(find.text('Console inspector'), findsOneWidget);
      expect(find.text('ARGUMENTS'), findsOneWidget);
      await tester.tap(
        find.byTooltip('Pause capture (discard incoming events)'),
      );
      await tester.pumpAndSettle();
      expect(find.textContaining('Capture paused'), findsOneWidget);
      await tester.tap(find.byTooltip('Resume capture'));
      await tester.enterText(find.byType(TextField).first, 'does-not-exist');
      await tester.pumpAndSettle();
      expect(find.text('No matching results'), findsOneWidget);
      await tester.tap(find.text('Reset filters'));
      await tester.pumpAndSettle();
      expect(find.text('Order created {"id":42}'), findsWidgets);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      server.dispose();
      store.dispose();
    },
  );

  for (final width in [860.0, 1100.0, 1360.0]) {
    testWidgets('empty desktop layout fits width $width and guide opens', (
      tester,
    ) async {
      tester.view.physicalSize = Size(width, 700);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final store = CaptureStore();
      final server = ViewerServer(store);
      await tester.pumpWidget(ViewerApp(store: store, server: server));
      expect(find.text('No requests'), findsOneWidget);
      await tester.tap(find.text('Connect an app'));
      await tester.pumpAndSettle();
      expect(find.text('Connect your Flutter app'), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      server.dispose();
      store.dispose();
    });
  }
}
