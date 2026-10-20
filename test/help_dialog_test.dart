import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/main.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';
import 'package:flutter_network_viewer/ui/help_dialog.dart';
import 'package:shared_preferences/shared_preferences.dart';

void main() {
  for (final language in ['en', 'vi']) {
    testWidgets('Home help and manual connection stay separate in $language', (
      tester,
    ) async {
      tester.view.physicalSize = const Size(860, 740);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      SharedPreferences.setMockInitialValues({'languageCode': language});
      final preferences = await SharedPreferences.getInstance();
      final store = CaptureStore();
      final server = ViewerServer(store);
      await tester.pumpWidget(
        ViewerApp(store: store, server: server, preferences: preferences),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byIcon(Icons.help_outline));
      await tester.pumpAndSettle();
      expect(find.byType(HelpDialog), findsOneWidget);
      expect(
        find.text(
          language == 'en' ? 'Basic usage guide' : 'Hướng dẫn sử dụng cơ bản',
        ),
        findsOneWidget,
      );
      expect(find.byKey(const ValueKey('connect-vm-uri')), findsNothing);

      const channel = MethodChannel('plugins.flutter.io/url_launcher');
      final calls = <MethodCall>[];
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(channel, (
        call,
      ) async {
        calls.add(call);
        if (calls.length == 1) throw PlatformException(code: 'no-browser');
        return true;
      });
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          channel,
          null,
        ),
      );
      final discussion = find.byKey(const ValueKey('go-to-discussion'));
      expect(
        find.descendant(
          of: discussion,
          matching: find.text(
            language == 'en' ? 'Go to Discussion' : 'Tham gia thảo luận',
          ),
        ),
        findsOneWidget,
      );
      await tester.tap(discussion);
      await tester.pumpAndSettle();
      expect(find.text(HelpDialog.discussionUri.toString()), findsOneWidget);
      await tester.tap(discussion);
      await tester.pumpAndSettle();
      expect(calls, hasLength(2));
      expect(calls.last.method, 'launch');
      expect(
        calls.last.arguments['url'],
        'https://github.com/DevMobileAn27/flutter_network_viewer/discussions',
      );
      expect(calls.last.arguments['useWebView'], isFalse);
      expect(calls.last.arguments['useSafariVC'], isFalse);
      expect(server.autoDetect, isFalse);
      expect(find.byType(HelpDialog), findsOneWidget);
      await tester.tap(find.text(language == 'en' ? 'Close' : 'Đóng'));
      await tester.pumpAndSettle();
      expect(find.byType(HelpDialog), findsNothing);
      await tester.tap(
        find.widgetWithText(
          OutlinedButton,
          language == 'en' ? 'Connect an app' : 'Kết nối ứng dụng',
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('connect-vm-uri')), findsOneWidget);
      expect(find.byType(HelpDialog), findsNothing);
      await tester.tap(find.text(language == 'en' ? 'Close' : 'Đóng'));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      server.dispose();
      store.dispose();
    });
  }
}
