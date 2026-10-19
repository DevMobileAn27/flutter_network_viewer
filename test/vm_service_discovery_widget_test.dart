import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/main.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';
import 'package:flutter_network_viewer/services/vm_service_discovery.dart';
import 'package:shared_preferences/shared_preferences.dart';

class MultipleApps extends VmServiceDiscovery {
  @override
  Future<List<VmServiceTarget>> discover() async => const [
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
  testWidgets('offers project choices; Stop and radar control discovery', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(860, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    SharedPreferences.setMockInitialValues({'languageCode': 'en'});
    final preferences = await SharedPreferences.getInstance();
    final store = CaptureStore();
    final server = ViewerServer(store, discovery: MultipleApps());
    await server.start(automatic: true);
    await tester.pumpWidget(
      ViewerApp(store: store, server: server, preferences: preferences),
    );
    await tester.pumpAndSettle();
    expect(find.text('office · 1111'), findsOneWidget);
    expect(find.text('shop · 2222'), findsOneWidget);
    expect(find.text('Select a running Flutter app:'), findsOneWidget);
    expect(tester.takeException(), isNull);
    await tester.tap(find.byIcon(Icons.stop_circle_outlined));
    await tester.pumpAndSettle();
    expect(server.searching, isFalse);
    expect(find.text('office · 1111'), findsNothing);
    await tester.tap(find.byTooltip('Find running Flutter apps'));
    await tester.pumpAndSettle();
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
