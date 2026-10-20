import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/main.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';
import 'package:flutter_network_viewer/services/vm_service_discovery.dart';
import 'package:shared_preferences/shared_preferences.dart';

class MultipleApps extends VmServiceDiscovery {
  Completer<List<VmServiceTarget>>? pending;

  @override
  Future<List<VmServiceTarget>> discover() async => pending != null
      ? pending!.future
      : const [
          VmServiceTarget(
            uri: 'ws://127.0.0.1:1111/a=/ws',
            name: 'office',
            projectKey: 'package:office/main.dart',
            vmKey: '1',
          ),
          VmServiceTarget(
            uri: 'ws://127.0.0.1:2222/b=/ws',
            name: 'shop',
            projectKey: 'package:shop/main.dart',
            vmKey: '2',
          ),
        ];
}

void main() {
  testWidgets('offers project choices; Stop and Start control discovery', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(860, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    SharedPreferences.setMockInitialValues({
      'languageCode': 'en',
      'autoDetectVmService': true,
    });
    final preferences = await SharedPreferences.getInstance();
    final store = CaptureStore();
    final discovery = MultipleApps();
    final targets = await discovery.discover();
    final server = ViewerServer(store, discovery: discovery);
    await server.start(automatic: true);
    await tester.pumpWidget(
      ViewerApp(store: store, server: server, preferences: preferences),
    );
    // Discovery now has a continuously animated progress indicator.
    await tester.pump(const Duration(milliseconds: 300));
    final loading = find.byKey(const ValueKey('discovery-loading'));
    expect(loading, findsOneWidget);
    expect(
      tester.widget<CircularProgressIndicator>(loading).color,
      const Color(0xff2563eb),
    );
    expect(find.text('Searching…'), findsOneWidget);
    expect(
      tester
          .widget<IconButton>(
            find.widgetWithIcon(IconButton, Icons.stop_circle_outlined),
          )
          .color,
      const Color(0xffeab308),
    );
    expect(find.byIcon(Icons.radar), findsNothing);
    expect(find.text('office · 1111'), findsOneWidget);
    expect(find.text('shop · 2222'), findsOneWidget);
    expect(find.text('Select a running Flutter app:'), findsOneWidget);
    expect(tester.takeException(), isNull);
    await tester.tap(find.byIcon(Icons.stop_circle_outlined));
    await tester.pumpAndSettle();
    expect(server.searching, isFalse);
    expect(loading, findsNothing);
    expect(
      tester
          .widget<IconButton>(
            find.widgetWithIcon(IconButton, Icons.play_circle_outline),
          )
          .color,
      const Color(0xff2563eb),
    );
    expect(find.text('office · 1111'), findsNothing);
    discovery.pending = Completer<List<VmServiceTarget>>();
    await tester.tap(find.byIcon(Icons.play_circle_outline));
    await tester.pump(const Duration(milliseconds: 300));
    expect(server.busy, isTrue);
    expect(find.text('Searching…'), findsOneWidget);
    expect(find.text('Starting…'), findsNothing);
    expect(loading, findsOneWidget);
    discovery.pending!.complete(targets);
    await tester.pump(const Duration(milliseconds: 300));
    expect(server.searching, isTrue);
    expect(preferences.getBool('autoDetectVmService'), isTrue);
    expect(find.text('office · 1111'), findsOneWidget);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox());
    await server.stop();
    server.dispose();
    store.dispose();
  });
}
