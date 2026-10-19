import 'dart:convert';
import 'dart:io';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/l10n/app_localizations.dart';
import 'package:flutter_network_viewer/ui/viewer_theme.dart';
import 'package:flutter_network_viewer/ui/viewer_dropdown.dart';
import 'package:flutter_network_viewer/main.dart';
import 'package:flutter_network_viewer/services/app_settings.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';
import 'package:flutter_network_viewer/services/viewer_mcp_server.dart';
import 'package:flutter_network_viewer/ui/settings_screen.dart';
import 'package:shared_preferences/shared_preferences.dart';

class TrackingServer extends ViewerServer {
  TrackingServer(super.store);
  int restarts = 0;

  @override
  Future<void> start({String? uri, bool automatic = false}) async {
    restarts++;
    autoDetect = automatic;
  }
}

class TrackingMcpServer extends ViewerMcpServer {
  TrackingMcpServer(super.store, super.viewer);
  bool enabled = false;
  bool failStart = false;
  int starts = 0;
  @override
  bool get running => enabled && error == null;

  @override
  Future<void> configure({
    required bool enabled,
    required int port,
    bool restart = false,
  }) async {
    if (!restart && this.enabled == enabled && this.port == port) return;
    this.enabled = enabled;
    this.port = port;
    if (enabled) starts++;
    error = enabled && failStart ? 'Port already in use' : null;
    notifyListeners();
  }
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  for (final mode in ['light', 'dark']) {
    testWidgets(
      'settings scrolls across its full width with a stable $mode header',
      (tester) async {
        tester.view.physicalSize = const Size(1360, 600);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        SharedPreferences.setMockInitialValues({'themeMode': mode});
        final preferences = await SharedPreferences.getInstance();
        final store = CaptureStore();
        final viewer = TrackingServer(store);
        final mcp = TrackingMcpServer(store, viewer);
        await tester.pumpWidget(
          ViewerApp(
            store: store,
            server: viewer,
            preferences: preferences,
            mcpServer: mcp,
          ),
        );
        await tester.pumpAndSettle();
        await tester.tap(find.byTooltip('Settings'));
        await tester.pumpAndSettle();
        final scroll = find
            .descendant(
              of: find.byType(SettingsScreen),
              matching: find.byType(Scrollable),
            )
            .first;
        final position = tester.state<ScrollableState>(scroll).position;
        final header = find
            .descendant(
              of: find.byType(AppBar),
              matching: find.byType(Material),
            )
            .first;
        final initialColor = tester.widget<Material>(header).color;

        // The card keeps its original centered width; its blank margins scroll too.
        final card = tester.getRect(find.byType(ViewerDropdown<ThemeMode>));
        expect(card.left, greaterThan(280));
        expect(card.right, lessThan(1080));
        for (final x in [30.0, 1330.0, 680.0]) {
          position.jumpTo(0);
          await tester.pumpAndSettle();
          await tester.sendEventToBinding(
            PointerScrollEvent(
              position: Offset(x, 300),
              scrollDelta: const Offset(0, 180),
            ),
          );
          await tester.pumpAndSettle();
          expect(position.pixels, greaterThan(0), reason: 'Pointer x=$x');
          final material = tester.widget<Material>(header);
          expect(material.color, initialColor);
          expect(material.elevation, 0);
          expect(material.surfaceTintColor, Colors.transparent);
        }
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox());
        mcp.dispose();
        viewer.dispose();
        store.dispose();
      },
    );
  }

  test('MCP defaults off and persists opt-in and a validated port', () async {
    SharedPreferences.setMockInitialValues({});
    final preferences = await SharedPreferences.getInstance();
    final settings = AppSettings(preferences);
    expect(settings.mcpEnabled, isFalse);
    expect(settings.mcpPort, 38943);
    await settings.setMcpPort(39941);
    await settings.setMcpEnabled(true);
    final restored = AppSettings(preferences);
    expect(restored.mcpEnabled, isTrue);
    expect(restored.mcpPort, 39941);
    await expectLater(settings.setMcpPort(0), throwsArgumentError);
    await expectLater(settings.setMcpPort(65536), throwsArgumentError);
    await settings.setMcpEnabled(false);
    final disabled = AppSettings(preferences);
    expect(disabled.mcpEnabled, isFalse);
    expect(disabled.mcpPort, 39941);
    settings.dispose();
    restored.dispose();
    disabled.dispose();
  });

  testWidgets(
    'MCP settings toggle, retry, validate ports and copy client configs',
    (tester) async {
      tester.view.physicalSize = const Size(860, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      SharedPreferences.setMockInitialValues({});
      final preferences = await SharedPreferences.getInstance();
      final store = CaptureStore();
      final viewer = TrackingServer(store);
      final mcp = TrackingMcpServer(store, viewer)..failStart = true;
      await tester.pumpWidget(
        ViewerApp(
          store: store,
          server: viewer,
          preferences: preferences,
          mcpServer: mcp,
        ),
      );
      await tester.pumpAndSettle();
      expect(mcp.starts, 0);
      await tester.tap(find.byTooltip('Settings'));
      await tester.pumpAndSettle();
      final toggle = find.byKey(const ValueKey('mcp-enabled'));
      final scroll = find
          .descendant(
            of: find.byType(SettingsScreen),
            matching: find.byType(Scrollable),
          )
          .first;
      await tester.scrollUntilVisible(toggle, 300, scrollable: scroll);
      expect(tester.widget<Switch>(toggle).value, isFalse);
      await tester.tap(toggle);
      await tester.pumpAndSettle();
      expect(preferences.getBool('mcpEnabled'), isTrue);
      expect(find.byKey(const ValueKey('mcp-error')), findsOneWidget);
      mcp.failStart = false;
      final port = find.byKey(const ValueKey('mcp-port'));
      final save = find.byKey(const ValueKey('save-mcp-port'));
      await tester.ensureVisible(save);
      await tester.enterText(port, '0');
      await tester.tap(save);
      await tester.pumpAndSettle();
      expect(find.text('Enter a port from 1 to 65535'), findsOneWidget);
      await tester.enterText(port, '39942');
      await tester.tap(save);
      await tester.pumpAndSettle();
      expect(mcp.running, isTrue);
      expect(mcp.port, 39942);
      expect(preferences.getInt('mcpPort'), 39942);
      expect(find.byKey(const ValueKey('mcp-error')), findsNothing);
      String? copied;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            copied = (call.arguments as Map)['text'] as String;
          }
          return null;
        },
      );
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        ),
      );
      final formats = find.byKey(const ValueKey('copy-mcp-formats'));
      await tester.ensureVisible(formats);
      expect(find.byKey(const ValueKey('copy-mcp-json')), findsNothing);
      await tester.tap(formats);
      await tester.pumpAndSettle();
      final copyJson = find.byKey(const ValueKey('copy-mcp-json'));
      await tester.tap(copyJson);
      await tester.pumpAndSettle();
      expect(
        jsonDecode(copied!)['mcpServers']['flutter-network-viewer']['url'],
        'http://127.0.0.1:39942/mcp',
      );
      expect(find.byKey(const ValueKey('copy-mcp-json')), findsNothing);
      await tester.tap(formats);
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('copy-mcp-codex')));
      await tester.pumpAndSettle();
      expect(
        copied,
        '[mcp_servers.flutter-network-viewer]\nurl = "http://127.0.0.1:39942/mcp"\n',
      );
      final copySetup = find.byKey(const ValueKey('copy-mcp-setup'));
      await tester.ensureVisible(copySetup);
      await tester.tap(copySetup);
      await tester.pumpAndSettle();
      expect(copied, mcp.setupCommand);
      expect(copied, contains('Transport: Streamable HTTP'));
      expect(copied, contains('Endpoint: http://127.0.0.1:39942/mcp'));
      expect(copied, isNot(contains('38943')));
      expect(
        copied,
        contains(
          'claude mcp add --transport http --scope user flutter-network-viewer',
        ),
      );
      expect(copied, contains('codex mcp add flutter-network-viewer --url'));
      expect(copied, contains(mcp.clientConfiguration));
      expect(copied, contains('"mcp-remote"'));
      expect(copied, contains('ChatGPT web/cloud connectors cannot reach'));
      expect(
        copied,
        contains('Do not claim access until a tool call succeeds'),
      );
      await tester.ensureVisible(toggle);
      await tester.tap(toggle);
      await tester.pumpAndSettle();
      expect(mcp.running, isFalse);
      expect(preferences.getBool('mcpEnabled'), isFalse);
      expect(viewer.restarts, 0);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      mcp.dispose();
      viewer.dispose();
      store.dispose();
    },
  );

  for (final width in [560.0, 1360.0]) {
    testWidgets('MCP settings adapt to width $width', (tester) async {
      tester.view.physicalSize = Size(width, 900);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      SharedPreferences.setMockInitialValues({});
      final preferences = await SharedPreferences.getInstance();
      final store = CaptureStore();
      final viewer = TrackingServer(store);
      final mcp = TrackingMcpServer(store, viewer);
      final settings = AppSettings(preferences);
      await tester.pumpWidget(
        MaterialApp(
          locale: const Locale('en'),
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          theme: viewerTheme(Brightness.light),
          home: SettingsScreen(
            settings: settings,
            server: viewer,
            mcpServer: mcp,
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.ensureVisible(find.byKey(const ValueKey('copy-mcp-setup')));
      await tester.pumpAndSettle();
      final port = tester.getRect(find.text('MCP port'));
      final endpoint = tester.getRect(find.text('Endpoint'));
      if (width >= 820) {
        expect(endpoint.top, closeTo(port.top, 0.1));
        expect(endpoint.left, greaterThan(port.right));
      } else {
        expect(endpoint.top, greaterThan(port.bottom));
        expect(endpoint.left, port.left);
      }
      final input = tester.getRect(find.byKey(const ValueKey('mcp-port')));
      final save = tester.getRect(find.byKey(const ValueKey('save-mcp-port')));
      expect(save.top, greaterThan(input.bottom));
      expect(save.height, greaterThanOrEqualTo(44));
      expect(
        tester.getSize(find.byKey(const ValueKey('copy-mcp-setup'))).height,
        greaterThanOrEqualTo(44),
      );
      expect(
        tester.getSize(find.byKey(const ValueKey('copy-mcp-formats'))).width,
        greaterThanOrEqualTo(44),
      );
      expect(find.text('Enable MCP'), findsNothing);
      expect(find.byTooltip('Enable MCP'), findsOneWidget);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      settings.dispose();
      mcp.dispose();
      viewer.dispose();
      store.dispose();
    });
  }

  testWidgets('saved MCP opt-in and port are restored on app launch', (
    tester,
  ) async {
    SharedPreferences.setMockInitialValues({
      'mcpEnabled': true,
      'mcpPort': 39943,
    });
    final preferences = await SharedPreferences.getInstance();
    final store = CaptureStore();
    final viewer = TrackingServer(store);
    final mcp = TrackingMcpServer(store, viewer);
    await tester.pumpWidget(
      ViewerApp(
        store: store,
        server: viewer,
        preferences: preferences,
        mcpServer: mcp,
      ),
    );
    await tester.pumpAndSettle();
    expect(mcp.running, isTrue);
    expect(mcp.port, 39943);
    expect(mcp.starts, 1);
    expect(viewer.restarts, 0);
    await tester.pumpWidget(const SizedBox());
    mcp.dispose();
    viewer.dispose();
    store.dispose();
  });

  test(
    'language preference persists across settings instances; unknown locale falls back',
    () async {
      SharedPreferences.setMockInitialValues({'languageCode': 'fr'});
      final preferences = await SharedPreferences.getInstance();
      final settings = AppSettings(preferences);
      expect(settings.languageCode, 'en');
      await settings.setLanguage('vi');
      expect(preferences.getString('languageCode'), 'vi');
      final restored = AppSettings(preferences);
      expect(restored.languageCode, 'vi');
      await expectLater(settings.setLanguage('fr'), throwsArgumentError);
      expect(settings.languageCode, 'vi');
      settings.dispose();
      restored.dispose();
    },
  );

  test(
    'theme defaults to light and persists across settings instances',
    () async {
      SharedPreferences.setMockInitialValues({});
      final preferences = await SharedPreferences.getInstance();
      final settings = AppSettings(preferences);
      expect(settings.languageCode, 'en');
      expect(settings.themeMode, ThemeMode.light);
      await settings.setThemeMode(ThemeMode.dark);
      expect(AppSettings(preferences).themeMode, ThemeMode.dark);
      await settings.setThemeMode(ThemeMode.light);
      expect(AppSettings(preferences).themeMode, ThemeMode.light);
      await expectLater(
        settings.setThemeMode(ThemeMode.system),
        throwsArgumentError,
      );
      settings.dispose();
    },
  );

  for (final width in [860.0, 1360.0]) {
    testWidgets(
      'settings switches languages without resetting capture or filters at width $width',
      (tester) async {
        tester.view.physicalSize = Size(width, 800);
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
        final server = TrackingServer(store);
        await tester.pumpWidget(
          ViewerApp(store: store, server: server, preferences: preferences),
        );
        await tester.pumpAndSettle();
        expect(
          Theme.of(tester.element(find.byType(Scaffold))).brightness,
          Brightness.light,
        );
        await tester.enterText(find.byType(TextField).first, 'orders');
        await tester.pumpAndSettle();
        await tester.tap(find.byTooltip('Settings'));
        await tester.pumpAndSettle();
        expect(find.text('Language'), findsWidgets);
        await tester.tap(find.byType(ViewerDropdown<String>).last);
        await tester.pumpAndSettle();
        await tester.tap(find.text('Tiếng Việt').last);
        await tester.pumpAndSettle();
        expect(find.text('Cài đặt'), findsOneWidget);
        expect(find.text('Ngôn ngữ'), findsWidgets);
        expect(preferences.getString('languageCode'), 'vi');
        await tester.tap(find.byType(ViewerDropdown<ThemeMode>));
        await tester.pumpAndSettle();
        await tester.tap(find.text('Đêm').last);
        await tester.pumpAndSettle();
        expect(
          Theme.of(tester.element(find.byType(Scaffold).last)).brightness,
          Brightness.dark,
        );
        expect(preferences.getString('themeMode'), 'dark');
        expect(server.restarts, 0);
        expect(store.network.length, 3);
        await tester.tap(find.byType(BackButton));
        await tester.pumpAndSettle();
        expect(find.text('Mạng'), findsOneWidget);
        expect(find.text('/orders?draft=true'), findsOneWidget);
        expect(find.text('/feed'), findsNothing);
        expect(
          Theme.of(tester.element(find.byType(Scaffold))).brightness,
          Brightness.dark,
        );
        await tester.tap(find.byTooltip('Cài đặt'));
        await tester.pumpAndSettle();
        await tester.tap(find.byType(ViewerDropdown<String>).last);
        await tester.pumpAndSettle();
        await tester.tap(find.text('English').last);
        await tester.pumpAndSettle();
        expect(find.text('Settings'), findsOneWidget);
        expect(preferences.getString('languageCode'), 'en');
        await tester.tap(find.byType(ViewerDropdown<ThemeMode>));
        await tester.pumpAndSettle();
        await tester.tap(find.text('Light').last);
        await tester.pumpAndSettle();
        expect(
          Theme.of(tester.element(find.byType(Scaffold).last)).brightness,
          Brightness.light,
        );
        await tester.enterText(
          find.byKey(const ValueKey('vm-service-uri')),
          '70000',
        );
        await tester.tap(find.text('Save & restart').first);
        await tester.pumpAndSettle();
        expect(find.textContaining('Enter a DevTools link'), findsOneWidget);
        expect(server.restarts, 0);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox());
        server.dispose();
        store.dispose();
      },
    );
  }

  testWidgets('Vietnamese and dark theme are restored on application launch', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1360, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    SharedPreferences.setMockInitialValues({
      'languageCode': 'vi',
      'themeMode': 'dark',
    });
    final preferences = await SharedPreferences.getInstance();
    final store = CaptureStore();
    final server = ViewerServer(store);
    await tester.pumpWidget(
      ViewerApp(store: store, server: server, preferences: preferences),
    );
    await tester.pumpAndSettle();
    expect(
      Theme.of(tester.element(find.byType(Scaffold))).brightness,
      Brightness.dark,
    );
    expect(find.text('Chưa có request'), findsOneWidget);
    await tester.tap(find.text('Kết nối ứng dụng'));
    await tester.pumpAndSettle();
    expect(find.text('Kết nối ứng dụng Flutter'), findsOneWidget);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox());
    server.dispose();
    store.dispose();
  });
}
