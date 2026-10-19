import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/main.dart';
import 'package:flutter_network_viewer/services/app_settings.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';
import 'package:flutter_network_viewer/ui/network_request_table.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets(
    'all three column dividers resize and survive tab changes and restart',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 900);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      SharedPreferences.setMockInitialValues({});
      final preferences = await SharedPreferences.getInstance();
      final store = CaptureStore()
        ..ingest(
          jsonDecode(
            File('test/fixtures/sdk_envelope.json').readAsStringSync(),
          ),
        );
      final server = ViewerServer(store);
      addTearDown(() {
        server.dispose();
        store.dispose();
      });
      Future<void> launch() async {
        await tester.pumpWidget(
          ViewerApp(store: store, server: server, preferences: preferences),
        );
        await tester.pumpAndSettle();
      }

      Finder divider(int index) =>
          find.byKey(ValueKey('network-column-divider-$index'));
      List<double> positions() => [
        for (var i = 0; i < 3; i++) tester.getTopLeft(divider(i)).dx,
      ];
      await launch();
      expect(
        tester
            .widget<NetworkRequestTable>(find.byType(NetworkRequestTable))
            .initialWidths
            .take(2),
        AppSettings.minNetworkColumnWidths.take(2),
      );
      final initial = positions();
      await tester.drag(divider(0), const Offset(48, 0));
      await tester.pumpAndSettle();
      expect(positions()[0], greaterThan(initial[0]));
      await tester.drag(divider(1), const Offset(-60, 0));
      await tester.pumpAndSettle();
      expect(positions()[1], lessThan(initial[1]));
      await tester.drag(divider(2), const Offset(-24, 0));
      await tester.pumpAndSettle();
      expect(positions()[2], lessThan(initial[2]));
      final resized = positions();
      final saved = preferences.getString('networkColumnWidths');
      expect(saved, isNotNull);
      final row = find
          .ancestor(
            of: find.text('/orders?draft=true'),
            matching: find.byType(InkWell),
          )
          .first;
      expect(tester.getSize(row).height, 52);
      await tester.tap(find.text('Console'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Network'));
      await tester.pumpAndSettle();
      expect(positions(), resized);
      await tester.pumpWidget(const SizedBox());
      await launch();
      expect(positions(), resized);
      expect(preferences.getString('networkColumnWidths'), saved);
      // Header and rows use precisely the same column boundaries.
      expect(
        tester.getTopLeft(find.text('URL / APPLICATION')).dx,
        tester.getTopLeft(find.text('/orders?draft=true')).dx,
      );
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
    },
  );

  testWidgets(
    'wide saved columns stay usable in a narrow pane without overwriting widths',
    (tester) async {
      tester.view.physicalSize = const Size(900, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      const saved = '[320.0,180.0,200.0]';
      SharedPreferences.setMockInitialValues({'networkColumnWidths': saved});
      final preferences = await SharedPreferences.getInstance();
      final store = CaptureStore()
        ..ingest(
          jsonDecode(
            File('test/fixtures/sdk_envelope.json').readAsStringSync(),
          ),
        );
      final server = ViewerServer(store);
      addTearDown(() {
        server.dispose();
        store.dispose();
      });
      await tester.pumpWidget(
        ViewerApp(store: store, server: server, preferences: preferences),
      );
      await tester.pumpAndSettle();
      final horizontal = find.descendant(
        of: find.byType(NetworkRequestTable),
        matching: find.byType(SingleChildScrollView),
      );
      await tester.drag(horizontal, const Offset(-280, 0));
      await tester.pumpAndSettle();
      expect(
        tester
            .getTopLeft(find.byKey(const ValueKey('network-column-divider-0')))
            .dx,
        lessThan(320),
      );
      expect(preferences.getString('networkColumnWidths'), saved);
      expect(tester.takeException(), isNull);
      tester.view.physicalSize = const Size(1440, 800);
      await tester.pumpAndSettle();
      expect(preferences.getString('networkColumnWidths'), saved);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
    },
  );

  test('invalid persisted column layouts fall back safely', () async {
    for (final saved in [
      'bad json',
      'null',
      '[80,90]',
      '[-1,90,100]',
      '[88,"bad",108]',
      '[9000,90,108]',
    ]) {
      SharedPreferences.setMockInitialValues({'networkColumnWidths': saved});
      final settings = AppSettings(await SharedPreferences.getInstance());
      expect(
        settings.networkColumnWidths,
        AppSettings.defaultNetworkColumnWidths,
      );
      settings.dispose();
    }
  });
}
