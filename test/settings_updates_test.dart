import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:flutter_network_viewer/main.dart';
import 'package:flutter_network_viewer/services/app_updater.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';
import 'package:flutter_network_viewer/ui/settings_screen.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'app_updater_test.dart' show FakeUpdates;

void main() {
  for (final language in ['en', 'vi']) {
    testWidgets(
      'Settings updates show version, progress and retry in $language',
      (tester) async {
        tester.view.physicalSize = const Size(1000, 1000);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        SharedPreferences.setMockInitialValues({'languageCode': language});
        final preferences = await SharedPreferences.getInstance();
        PackageInfo.setMockInitialValues(
          appName: 'Flutter Network Viewer',
          packageName: 'dev.quocan.flutterNetworkViewer',
          version: '0.1.1',
          buildNumber: '2',
          buildSignature: '',
        );
        final backend = FakeUpdates();
        final updater = AppUpdater(
          backend: backend,
          feed: updateFeedFor('macos', 'macos_arm64'),
          isWindows: false,
        );
        final store = CaptureStore();
        final server = ViewerServer(store);
        addTearDown(() async {
          await tester.pumpWidget(const SizedBox());
          server.dispose();
          store.dispose();
        });
        await tester.pumpWidget(
          ViewerApp(
            store: store,
            server: server,
            preferences: preferences,
            updater: updater,
          ),
        );
        await tester.pumpAndSettle();
        await tester.tap(find.byIcon(Icons.settings_outlined));
        await tester.pumpAndSettle();
        final button = find.byKey(const ValueKey('check-app-updates'));
        await tester.scrollUntilVisible(
          button,
          350,
          scrollable: find
              .descendant(
                of: find.byType(SettingsScreen),
                matching: find.byType(Scrollable),
              )
              .first,
        );
        await tester.pumpAndSettle();
        await tester.ensureVisible(button);
        await tester.pumpAndSettle();
        expect(find.text('Version 0.1.1 - Nguyễn Quốc An'), findsOneWidget);
        expect(
          find.text(language == 'en' ? 'App updates' : 'Cập nhật ứng dụng'),
          findsOneWidget,
        );
        await tester.tap(button);
        await tester.pump();
        expect(backend.feed, contains('/flutter_network_viewer/'));
        expect(backend.checks, [false]);
        expect(tester.widget<FilledButton>(button).onPressed, isNull);
        backend.listener!.onUpdaterError(null);
        await tester.pumpAndSettle();
        expect(tester.widget<FilledButton>(button).onPressed, isNotNull);
        expect(
          find.text(
            language == 'en'
                ? 'Could not check for updates. Check your connection and try again.'
                : 'Không thể kiểm tra cập nhật. Kiểm tra kết nối rồi thử lại.',
          ),
          findsOneWidget,
        );
        await tester.ensureVisible(button);
        await tester.tap(button);
        backend.listener!.onUpdaterUpdateNotAvailable(null);
        await tester.pumpAndSettle();
        expect(backend.checks, [false, false]);
        final statusIcon = find.byKey(const ValueKey('version-update-status'));
        expect(
          find.byTooltip(
            language == 'en'
                ? 'You’re up to date.'
                : 'Bạn đang dùng phiên bản mới nhất.',
          ),
          findsOneWidget,
        );
        expect(
          tester.widget<Icon>(statusIcon).icon,
          Icons.check_circle_outline_rounded,
        );
        expect(tester.widget<Icon>(statusIcon).color, const Color(0xff22c55e));
        expect(
          find.text(
            language == 'en'
                ? 'You’re up to date.'
                : 'Bạn đang dùng phiên bản mới nhất.',
          ),
          findsNothing,
        );

        await tester.tap(button);
        await tester.pump();
        backend.listener!.onUpdaterUpdateAvailable(null);
        await tester.pumpAndSettle();
        expect(tester.widget<Icon>(statusIcon).icon, Icons.update_rounded);
        expect(tester.widget<Icon>(statusIcon).color, const Color(0xff2563eb));
        expect(
          find.text(
            language == 'en' ? 'Update new version' : 'Cập nhật phiên bản mới',
          ),
          findsOneWidget,
        );
        await tester.ensureVisible(button);
        await tester.tap(button);
        await tester.pump();
        expect(backend.checks, [false, false, false, false]);
        expect(tester.widget<FilledButton>(button).onPressed, isNull);
        backend.listener!.onUpdaterUpdateNotAvailable(null);
        await tester.pumpAndSettle();
        expect(
          find.text(
            language == 'en' ? 'Check for updates' : 'Kiểm tra cập nhật',
          ),
          findsOneWidget,
        );
        expect(tester.takeException(), isNull);
      },
    );
  }
}
