import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/main.dart';
import 'package:flutter_network_viewer/services/app_settings.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';
import 'package:flutter_network_viewer/ui/inspector.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test('invalid saved layout values fall back to usable defaults', () async {
    SharedPreferences.setMockInitialValues({
      'networkInspectorWidth': double.nan,
      'consoleInspectorWidth': -100.0,
      'jsonInputFraction': 1.5,
      'requestDetailTab': 4,
    });
    final settings = AppSettings(await SharedPreferences.getInstance());
    expect(settings.networkInspectorWidth, 400);
    expect(settings.consoleInspectorWidth, 400);
    expect(settings.jsonInputFraction, .5);
    expect(settings.requestDetailTab, 0);
    settings.dispose();
  });

  test(
    'old request detail selections migrate and all six new tabs persist',
    () async {
      for (final entry in {0: 0, 1: 2, 2: 4, 3: 5}.entries) {
        SharedPreferences.setMockInitialValues({'requestDetailTab': entry.key});
        final preferences = await SharedPreferences.getInstance();
        final settings = AppSettings(preferences);
        expect(settings.requestDetailTab, entry.value);
        settings.dispose();
      }
      SharedPreferences.setMockInitialValues({});
      final preferences = await SharedPreferences.getInstance();
      final settings = AppSettings(preferences);
      for (final index in [1, 2, 3, 4, 5, 0]) {
        await settings.setRequestDetailTab(index);
        final restored = AppSettings(preferences);
        expect(restored.requestDetailTab, index);
        restored.dispose();
      }
      settings.dispose();
    },
  );

  for (final width in [860.0, 1440.0]) {
    testWidgets('request tab follows selection and restart at width $width', (
      tester,
    ) async {
      tester.view.physicalSize = Size(width, 900);
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
      Future<void> launch() async {
        await tester.pumpWidget(
          ViewerApp(store: store, server: server, preferences: preferences),
        );
        await tester.pumpAndSettle();
      }

      TabController controller() => tester
          .widget<TabBar>(
            find
                .descendant(
                  of: find.byType(NetworkInspector),
                  matching: find.byType(TabBar),
                )
                .first,
          )
          .controller!;
      Future<void> closeDialog() async {
        if (width < 900) {
          Navigator.of(tester.element(find.byType(NetworkInspector))).pop();
          await tester.pumpAndSettle();
        }
      }

      await launch();
      await tester.tap(find.text('/orders?draft=true'));
      await tester.pumpAndSettle();
      await tester.ensureVisible(find.text('Response body'));
      await tester.tap(find.text('Response body'));
      await tester.pumpAndSettle();
      expect(controller().index, 4);
      await closeDialog();
      await tester.tap(find.text('/feed'));
      await tester.pumpAndSettle();
      expect(controller().index, 4);

      // A swipe must persist the selection as well as a TabBar tap.
      await tester.drag(find.byType(TabBarView).first, const Offset(-600, 0));
      await tester.pumpAndSettle();
      expect(controller().index, 5);
      expect(preferences.getString('requestDetailTabName'), 'raw');
      await closeDialog();
      await tester.pumpWidget(const SizedBox());
      await launch();
      await tester.tap(find.text('/profile'));
      await tester.pumpAndSettle();
      expect(controller().index, 5);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      server.dispose();
      store.dispose();
    });
  }

  testWidgets('each divider restores independently across tabs and restart', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1440, 900);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    SharedPreferences.setMockInitialValues({});
    final preferences = await SharedPreferences.getInstance();
    final store = CaptureStore();
    final server = ViewerServer(store);
    final pane = find.byKey(const ValueKey('inspector-pane'));
    final divider = find.byKey(const ValueKey('inspector-divider'));
    final jsonPane = find.byKey(const ValueKey('json-beautify-input-pane'));
    final jsonDivider = find.byKey(const ValueKey('json-beautify-divider'));
    double inspectorWidth() => tester.getSize(pane).width;
    Future<void> select(String name) async {
      await tester.tap(find.text(name));
      await tester.pumpAndSettle();
    }

    Future<void> launch() async {
      await tester.pumpWidget(
        ViewerApp(store: store, server: server, preferences: preferences),
      );
      await tester.pumpAndSettle();
    }

    await launch();
    await tester.drag(divider, const Offset(-240, 0));
    await tester.pumpAndSettle();
    final networkWidth = inspectorWidth();
    expect(networkWidth, greaterThan(600));
    expect(preferences.getDouble('networkInspectorWidth'), networkWidth);
    await select('Console');
    expect(inspectorWidth(), 400);
    await tester.drag(divider, const Offset(-100, 0));
    await tester.pumpAndSettle();
    final consoleWidth = inspectorWidth();
    expect(consoleWidth, lessThan(networkWidth));
    expect(preferences.getDouble('consoleInspectorWidth'), consoleWidth);
    await select('Json beautify');
    await tester.drag(jsonDivider, const Offset(160, 0));
    await tester.pumpAndSettle();
    final jsonWidth = tester.getSize(jsonPane).width;
    expect(preferences.getDouble('jsonInputFraction'), greaterThan(.5));
    await select('Network');
    expect(inspectorWidth(), networkWidth);
    await select('Console');
    expect(inspectorWidth(), consoleWidth);
    await select('Json beautify');
    expect(tester.getSize(jsonPane).width, jsonWidth);

    await tester.pumpWidget(const SizedBox());
    await launch();
    expect(inspectorWidth(), networkWidth);
    await select('Console');
    expect(inspectorWidth(), consoleWidth);
    await select('Json beautify');
    expect(tester.getSize(jsonPane).width, jsonWidth);

    // Temporary window constraints must not overwrite the saved desired layout.
    await select('Network');
    tester.view.physicalSize = const Size(900, 900);
    await tester.pumpAndSettle();
    expect(inspectorWidth(), lessThan(networkWidth));
    expect(preferences.getDouble('networkInspectorWidth'), networkWidth);
    tester.view.physicalSize = const Size(860, 900);
    await tester.pumpAndSettle();
    tester.view.physicalSize = const Size(1440, 900);
    await tester.pumpAndSettle();
    expect(inspectorWidth(), networkWidth);

    await tester.tap(divider);
    await tester.pump(const Duration(milliseconds: 50));
    await tester.tap(divider);
    await tester.pumpAndSettle();
    expect(preferences.getDouble('networkInspectorWidth'), 400);
    expect(preferences.getDouble('consoleInspectorWidth'), consoleWidth);
    await select('Json beautify');
    await tester.tap(jsonDivider);
    await tester.pump(const Duration(milliseconds: 50));
    await tester.tap(jsonDivider);
    await tester.pumpAndSettle();
    expect(preferences.getDouble('jsonInputFraction'), .5);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox());
    server.dispose();
    store.dispose();
  });
}
