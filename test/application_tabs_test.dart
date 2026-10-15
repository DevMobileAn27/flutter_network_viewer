import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/main.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';
import 'package:flutter_network_viewer/ui/application_tabs.dart';

void main() {
  for (final width in [860.0, 1360.0]) {
    testWidgets('application tabs scroll and filter both captures at $width', (
      tester,
    ) async {
      tester.view.physicalSize = Size(width, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final store = CaptureStore();
      final server = ViewerServer(store);
      void addSession(int index) => store.ingest({
        'sdk': {'schemaVersion': 1},
        'session': {
          'id': 'repo-$index',
          'appName': 'Repo $index',
          'deviceName': 'Device $index',
        },
        'events': [
          {
            'requestId': 'request',
            'phase': 'request',
            'source': 'js.fetch',
            'request': {
              'method': 'GET',
              'url': 'https://example.com/repo-$index',
            },
          },
          {
            'source': 'js.console',
            'level': 'info',
            'message': 'Message from repo $index',
          },
        ],
      });
      for (var index = 1; index <= 16; index++) {
        addSession(index);
      }
      await tester.pumpWidget(ViewerApp(store: store, server: server));
      await tester.pumpAndSettle();
      expect(find.text('WORKSPACE'), findsNothing);
      expect(find.text('APPLICATIONS'), findsNothing);
      final tabs = find.byType(ApplicationTabs);
      expect(tester.getSize(tabs).width, lessThanOrEqualTo(480));
      expect(
        tester.getTopLeft(tabs).dx,
        greaterThan(tester.getTopRight(find.text('Console')).dx),
      );
      final scrollable = find.descendant(
        of: tabs,
        matching: find.byType(Scrollable),
      );
      final target = find.byKey(const ValueKey('application-repo-12'));
      await tester.scrollUntilVisible(target, 200, scrollable: scrollable);
      await tester.tap(target);
      await tester.pumpAndSettle();
      expect(find.text('/repo-12'), findsOneWidget);
      expect(find.text('/repo-1'), findsNothing);
      if (width >= 900) {
        expect(
          tester.getTopLeft(find.byKey(const ValueKey('capture-list-pane'))).dx,
          0,
        );
      }
      await tester.tap(find.text('Console'));
      await tester.pumpAndSettle();
      expect(find.text('Message from repo 12'), findsOneWidget);
      expect(find.text('Message from repo 1'), findsNothing);
      // A new connection must preserve the active application and mode.
      addSession(17);
      await tester.pumpAndSettle();
      expect(find.text('Message from repo 12'), findsOneWidget);
      expect(find.text('Message from repo 17'), findsNothing);
      final all = find.byKey(const ValueKey('application-all'));
      await tester.ensureVisible(all);
      await tester.pumpAndSettle();
      await tester.tap(all);
      await tester.pumpAndSettle();
      expect(find.text('Message from repo 17'), findsOneWidget);
      expect(find.text('Message from repo 16'), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      server.dispose();
      store.dispose();
    });
  }
}
