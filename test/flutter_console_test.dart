import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/main.dart';
import 'package:flutter_network_viewer/models/capture.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';
import 'package:flutter_network_viewer/ui/inspector.dart';
import 'package:flutter_network_viewer/ui/components.dart';
import 'package:flutter_network_viewer/ui/json_code_view.dart';
import 'package:flutter_network_viewer/ui/viewer_dropdown.dart';

ConsoleRecord line(
  int id,
  String text, {
  String isolate = 'main',
  String source = 'dart.stdout',
  int millis = 0,
}) => ConsoleRecord(id, 'app', {
  'source': source,
  'message': text,
  'isolateId': isolate,
  'timestamp': DateTime.utc(2026, 10, 15, 8, 20, 58, millis).toIso8601String(),
});

void main() {
  test('groups Flutter JSON in order while preserving original records', () {
    final records = [
      line(0, 'before'),
      line(1, 'flutter: {'),
      line(2, 'flutter:   "name": "Cà phê",'),
      line(3, 'flutter:   "items": [1, 2]'),
      line(4, 'flutter: }'),
      line(5, 'after'),
    ];
    final entries = ConsoleRecord.grouped(records);
    expect(entries, hasLength(3));
    expect(entries[1].id, 1);
    expect(
      entries[1].displayMessage,
      '{\n  "name": "Cà phê",\n  "items": [1, 2]\n}',
    );
    expect(entries[1].event['rawEvents'], hasLength(4));
    expect(records[1].message, 'flutter: {');
    expect(records[1].event.containsKey('rawEvents'), isFalse);
    expect(entries[1].lineCount, 4);
  });

  test(
    'does not merge unrelated logs, incomplete JSON, isolates or sources',
    () {
      for (final records in [
        [line(0, '{'), line(1, 'unrelated'), line(2, '}')],
        [line(0, '{'), line(1, '"a": 1')],
        [line(0, '{'), line(1, '}', isolate: 'worker')],
        [line(0, '{'), line(1, '}', source: 'dart.stderr')],
        [line(0, '{'), line(1, '}', millis: 3000)],
        [line(0, 'first print'), line(1, 'second print')],
      ]) {
        expect(ConsoleRecord.grouped(records), hasLength(records.length));
      }
    },
  );

  test(
    'groups Flutter exception frames and keeps following output separate',
    () {
      final records = [
        line(0, 'flutter: ══╡ EXCEPTION CAUGHT BY WIDGETS LIBRARY ╞══'),
        line(1, 'flutter: A RenderFlex overflowed'),
        line(2, 'flutter: #0 Widget.build (package:shop/main.dart:42:7)'),
        line(3, 'flutter: ══════════════════════════'),
        line(4, 'Next request'),
      ];
      final entries = ConsoleRecord.grouped(records);
      expect(entries, hasLength(2));
      expect(entries.first.level, 'error');
      expect(entries.first.displayMessage, contains('#0 Widget.build'));
      expect(entries.last.message, 'Next request');
    },
  );

  test('terminal styles are removed only from the displayed message', () {
    final record = line(
      0,
      '\x1b[31mflutter: Error\x1b[0m\r\nflutter:   detail',
    );
    expect(record.displayMessage, 'Error\n  detail');
    expect(record.message, contains('\x1b[31m'));
  });

  for (final width in [860.0, 1360.0]) {
    testWidgets(
      'Flutter Console shows grouped JSON and searches logger/stack at $width',
      (tester) async {
        tester.view.physicalSize = Size(width, 900);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final store = CaptureStore()
          ..ingest({
            'session': {'id': 'app', 'appName': 'Shop'},
            'sdk': {'schemaVersion': 1},
            'events': [
              line(0, 'flutter: {').event,
              line(1, 'flutter:   "answer": 42').event,
              line(2, 'flutter: }').event,
              {
                'source': 'dart.logging',
                'level': 'error',
                'severity': 1000,
                'message': 'Cannot load profile',
                'loggerName': 'ProfileRepository',
                'error': 'StateError: missing user',
                'stackTrace': '#0 fetchUser (package:shop/profile.dart:24:3)',
                'isolateName': 'main',
                'timestamp': '2026-10-15T08:20:58.123Z',
              },
            ],
          });
        final server = ViewerServer(store);
        await tester.pumpWidget(ViewerApp(store: store, server: server));
        await tester.tap(find.text('Console'));
        await tester.pumpAndSettle();
        expect(find.byKey(const ValueKey('console-row-0')), findsOneWidget);
        expect(find.byKey(const ValueKey('console-row-1')), findsNothing);
        expect(find.textContaining('stdout · Shop · 3 lines'), findsNothing);
        final row = find.byKey(const ValueKey('console-row-0'));
        expect(tester.getSize(row).height, greaterThan(36));
        final rowText = tester
            .widgetList<Text>(
              find.descendant(of: row, matching: find.byType(Text)),
            )
            .toList();
        expect(rowText.map((text) => text.data), [
          'LOG',
          clockTime(DateTime.parse('2026-10-15T08:20:58.000Z')),
          '{\n  "answer": 42\n}',
        ]);
        expect(rowText.last.maxLines, isNull);
        expect(rowText.last.softWrap, isTrue);
        final time = tester.widget<Text>(
          find
              .text(clockTime(DateTime.parse('2026-10-15T08:20:58.123Z')))
              .first,
        );
        expect(time.maxLines, 1);
        expect(time.softWrap, isFalse);
        await tester.tap(row);
        await tester.pumpAndSettle();
        expect(find.byType(JsonCodeView), findsOneWidget);
        if (width < 900) {
          await tester.tap(find.byTooltip('Close inspector'));
          await tester.pumpAndSettle();
        }
        await tester.tap(
          find.widgetWithText(ViewerDropdown<String>, 'All sources'),
        );
        await tester.pumpAndSettle();
        await tester.tap(find.widgetWithText(MenuItemButton, 'stdout'));
        await tester.pumpAndSettle();
        expect(find.text('Cannot load profile'), findsNothing);
        expect(find.byKey(const ValueKey('console-row-0')), findsOneWidget);
        await tester.tap(find.widgetWithText(ViewerDropdown<String>, 'stdout'));
        await tester.pumpAndSettle();
        await tester.tap(find.widgetWithText(MenuItemButton, 'All sources'));
        await tester.pumpAndSettle();
        await tester.enterText(
          find.byType(TextField).first,
          'ProfileRepository',
        );
        await tester.pumpAndSettle();
        expect(find.text('Cannot load profile'), findsOneWidget);
        expect(find.byKey(const ValueKey('console-row-0')), findsNothing);
        await tester.enterText(find.byType(TextField).first, 'profile.dart:24');
        await tester.pumpAndSettle();
        await tester.tap(find.text('Cannot load profile'));
        await tester.pumpAndSettle();
        expect(find.byType(ConsoleInspector), findsOneWidget);
        expect(find.text('STACK TRACE'), findsOneWidget);
        expect(
          find.text('#0 fetchUser (package:shop/profile.dart:24:3)'),
          findsOneWidget,
        );
        expect(find.text('StateError: missing user'), findsOneWidget);
        expect(find.text('ARGUMENTS'), findsNothing);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox());
        server.dispose();
        store.dispose();
      },
    );
  }
}
