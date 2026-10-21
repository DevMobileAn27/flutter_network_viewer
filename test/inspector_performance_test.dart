import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/l10n/app_localizations.dart';
import 'package:flutter_network_viewer/main.dart';
import 'package:flutter_network_viewer/models/capture.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';
import 'package:flutter_network_viewer/ui/inspector.dart';
import 'package:flutter_network_viewer/ui/json_beautify_screen.dart';
import 'package:flutter_network_viewer/ui/json_document.dart';

class _RawProbe {
  int encodes = 0;
  Map<String, dynamic> toJson() {
    encodes++;
    return {'raw': 'only encode when opened'};
  }
}

Widget _app(Widget child) => MaterialApp(
  localizationsDelegates: AppLocalizations.localizationsDelegates,
  supportedLocales: AppLocalizations.supportedLocales,
  home: Scaffold(body: child),
);

void main() {
  for (final width in [860.0, 1360.0]) {
    testWidgets('Clear releases selected inspector and JSON cache at $width', (
      tester,
    ) async {
      tester.view.physicalSize = Size(width, 840);
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
      await tester.pumpAndSettle();
      await tester.tap(find.text('/orders?draft=true'));
      await tester.pumpAndSettle();
      expect(find.byType(NetworkInspector), findsOneWidget);
      JsonDocumentCache.shared.decode('{"private":"old captured body"}');
      expect(JsonDocumentCache.shared.length, greaterThan(0));
      if (width >= 900) {
        await tester.tap(find.byTooltip('Clear all captured data'));
      } else {
        // A Clear through MCP must also close an already opened inspector.
        store.clear();
      }
      await tester.pumpAndSettle();
      expect(store.sessions, isEmpty);
      expect(store.network, isEmpty);
      expect(store.logs, isEmpty);
      expect(find.byType(NetworkInspector), findsNothing);
      expect(find.byType(Dialog), findsNothing);
      expect(JsonDocumentCache.shared.length, 0);
      expect(JsonDocumentCache.shared.estimatedBytes, 0);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      server.dispose();
      store.dispose();
    });
  }
  testWidgets('request selection does not format unopened Raw tab', (
    tester,
  ) async {
    final probe = _RawProbe();
    final record = NetworkRecord('app', 'request')
      ..apply({
        'phase': 'request',
        'request': {'method': 'GET', 'url': 'https://example.com'},
        'rawOnly': probe,
      });
    await tester.pumpWidget(_app(NetworkInspector(record: record)));
    await tester.pumpAndSettle();
    expect(probe.encodes, 0);
    await tester.ensureVisible(find.text('Raw'));
    await tester.tap(find.text('Raw'));
    await tester.pumpAndSettle();
    expect(probe.encodes, greaterThan(0));
    expect(find.textContaining('only encode when opened'), findsOneWidget);
  });

  testWidgets('console selection does not serialize a collapsed Raw event', (
    tester,
  ) async {
    final probe = _RawProbe();
    final record = ConsoleRecord(0, 'app', {
      'source': 'dart.stdout',
      'message': 'Hello',
      'rawOnly': probe,
    });
    await tester.pumpWidget(_app(ConsoleInspector(record: record)));
    await tester.pumpAndSettle();
    expect(probe.encodes, 0);
    await tester.tap(find.text('Raw event'));
    await tester.pumpAndSettle();
    expect(probe.encodes, greaterThan(0));
  });

  testWidgets('selecting requests and logs leaves the shell toolbar intact', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1360, 840);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final store = CaptureStore()
      ..ingest(
        jsonDecode(File('test/fixtures/sdk_envelope.json').readAsStringSync()),
      );
    final server = ViewerServer(store);
    await tester.pumpWidget(ViewerApp(store: store, server: server));
    await tester.pumpAndSettle();
    final toolbar = tester.widget<TabBar>(find.byType(TabBar).first);
    final untouched = find.byKey(
      ValueKey(
        'network-row-${store.network.values.firstWhere((r) => r.url.endsWith('/feed')).key}',
      ),
    );
    final untouchedWidget = tester.widget(untouched);
    await tester.tap(find.text('/orders?draft=true'));
    await tester.pumpAndSettle();
    final detailState = tester.state(find.byType(NetworkInspector));
    expect(identical(tester.widget(untouched), untouchedWidget), isTrue);
    expect(
      identical(tester.widget(find.byType(TabBar).first), toolbar),
      isTrue,
    );
    await tester.tap(find.text('/profile'));
    await tester.pumpAndSettle();
    expect(
      identical(tester.state(find.byType(NetworkInspector)), detailState),
      isTrue,
    );
    expect(identical(tester.widget(untouched), untouchedWidget), isTrue);
    expect(
      identical(tester.widget(find.byType(TabBar).first), toolbar),
      isTrue,
    );
    await tester.tap(find.text('Console'));
    await tester.pumpAndSettle();
    final consoleToolbar = tester.widget<TabBar>(find.byType(TabBar).first);
    await tester.tap(find.text('Order created {"id":42}'));
    await tester.pumpAndSettle();
    expect(
      identical(tester.widget(find.byType(TabBar).first), consoleToolbar),
      isTrue,
    );
    expect(find.byType(ConsoleInspector), findsOneWidget);
    await tester.pumpWidget(const SizedBox());
    server.dispose();
    store.dispose();
  });

  testWidgets('unrelated events preserve detail but completion refreshes it', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1360, 840);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final fixture =
        jsonDecode(File('test/fixtures/sdk_envelope.json').readAsStringSync())
            as Map<String, dynamic>;
    final store = CaptureStore()..ingest(fixture);
    final server = ViewerServer(store);
    await tester.pumpWidget(ViewerApp(store: store, server: server));
    await tester.pumpAndSettle();
    await tester.tap(find.text('/feed'));
    await tester.pumpAndSettle();
    final detail = tester.widget<NetworkInspector>(
      find.byType(NetworkInspector),
    );
    final beautify = tester.widget<JsonBeautifyScreen>(
      find.byType(JsonBeautifyScreen, skipOffstage: false),
    );
    store.ingest({
      ...fixture,
      'events': [
        {'source': 'dart.stdout', 'message': 'Unrelated new log'},
      ],
    });
    await tester.pumpAndSettle();
    expect(
      identical(tester.widget(find.byType(NetworkInspector)), detail),
      isTrue,
    );
    expect(
      identical(
        tester.widget(find.byType(JsonBeautifyScreen, skipOffstage: false)),
        beautify,
      ),
      isTrue,
    );
    store.ingest({
      ...fixture,
      'events': [
        {
          'requestId': 'fetch-3',
          'phase': 'response',
          'response': {'statusCode': 200, 'body': '{"ok":true}'},
        },
      ],
    });
    await tester.pumpAndSettle();
    expect(
      identical(tester.widget(find.byType(NetworkInspector)), detail),
      isFalse,
    );
    expect(
      tester
          .widget<NetworkInspector>(find.byType(NetworkInspector))
          .record
          .status,
      200,
    );
    await tester.pumpWidget(const SizedBox());
    server.dispose();
    store.dispose();
  });
}
