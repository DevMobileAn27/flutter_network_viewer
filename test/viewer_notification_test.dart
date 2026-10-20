import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/l10n/app_localizations.dart';
import 'package:flutter_network_viewer/ui/components.dart';
import 'package:flutter_network_viewer/ui/viewer_notification.dart';

void main() {
  late BuildContext pageContext;
  var taps = 0;

  Future<void> mount(
    WidgetTester tester, {
    Locale locale = const Locale('en'),
    Brightness brightness = Brightness.light,
  }) async {
    await tester.pumpWidget(
      MaterialApp(
        locale: locale,
        supportedLocales: AppLocalizations.supportedLocales,
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        theme: viewerTheme(brightness),
        home: Scaffold(
          body: Builder(
            builder: (context) {
              pageContext = context;
              return Align(
                alignment: Alignment.topRight,
                child: TextButton(
                  onPressed: () => taps++,
                  child: const Text('Underlying action'),
                ),
              );
            },
          ),
        ),
      ),
    );
  }

  testWidgets('four variants, three rows and exact bottom-left position', (
    tester,
  ) async {
    await mount(tester);
    const variants = [
      (
        ViewerNotificationType.info,
        Icons.info_outline_rounded,
        'Information',
        0xff3b82f6,
      ),
      (
        ViewerNotificationType.success,
        Icons.check_circle_outline_rounded,
        'Success',
        0xff22c55e,
      ),
      (
        ViewerNotificationType.danger,
        Icons.error_outline_rounded,
        'Error',
        0xffef4444,
      ),
      (
        ViewerNotificationType.warning,
        Icons.warning_amber_rounded,
        'Warning',
        0xffeab308,
      ),
    ];
    for (final (type, icon, title, color) in variants) {
      final result = showViewerNotification(
        pageContext,
        type: type,
        description: 'Notification details',
      );
      await tester.pumpAndSettle();
      final rect = tester.getRect(
        find.byKey(const ValueKey('viewer-notification')),
      );
      expect(rect.left, 20);
      expect(rect.bottom, 580);
      expect(tester.widget<Icon>(find.byIcon(icon)).color, Color(color));
      expect(
        tester.getTopLeft(find.text(title)).dy,
        lessThan(tester.getTopLeft(find.text('Notification details')).dy),
      );
      expect(
        tester.getTopLeft(find.text('Notification details')).dy,
        lessThan(tester.getTopLeft(find.text('OK')).dy),
      );
      expect(find.byType(SnackBar), findsNothing);
      await tester.tap(find.text('OK'));
      await tester.pumpAndSettle();
      expect(await result, isTrue);
      expect(find.byKey(const ValueKey('viewer-notification')), findsNothing);
    }
  });

  testWidgets('confirmation waits for a choice and cancel returns false', (
    tester,
  ) async {
    await mount(tester);
    for (final confirm in [false, true]) {
      bool? answer;
      showViewerNotification(
        pageContext,
        title: 'Continue?',
        description: 'Confirm this action.',
        type: ViewerNotificationType.warning,
        confirmation: true,
      ).then((value) => answer = value);
      await tester.pumpAndSettle();
      expect(answer, isNull);
      expect(find.text('OK'), findsNothing);
      expect(find.text('Confirm'), findsOneWidget);
      expect(find.text('Cancel'), findsOneWidget);
      await tester.tap(find.text(confirm ? 'Confirm' : 'Cancel'));
      await tester.pumpAndSettle();
      expect(answer, confirm);
    }
  });

  testWidgets(
    'popup stays open, leaves app interactive and replaces previous',
    (tester) async {
      await mount(tester);
      final old = showViewerNotification(pageContext, description: 'First');
      await tester.pumpAndSettle();
      await tester.pump(const Duration(seconds: 10));
      expect(find.text('First'), findsOneWidget);
      taps = 0;
      await tester.tap(find.text('Underlying action'));
      expect(taps, 1);
      final current = showViewerNotification(
        pageContext,
        description: 'Second',
      );
      await tester.pumpAndSettle();
      expect(await old, isFalse);
      expect(find.text('First'), findsNothing);
      expect(find.byKey(const ValueKey('viewer-notification')), findsOneWidget);
      await tester.pumpWidget(const SizedBox());
      expect(await current, isFalse);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('autoHide honors the requested delay and never confirms', (
    tester,
  ) async {
    await mount(tester);
    bool? answer;
    showViewerNotification(
      pageContext,
      description: 'Timed confirmation',
      confirmation: true,
      autoHide: true,
      hideAfterMilisecond: 1500,
    ).then((value) => answer = value);
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 1499));
    expect(find.text('Timed confirmation'), findsOneWidget);
    expect(answer, isNull);
    await tester.pump(const Duration(milliseconds: 1));
    expect(find.text('Timed confirmation'), findsNothing);
    expect(answer, isFalse);
  });

  testWidgets('autoHide defaults to 3000ms and disabled ignores its delay', (
    tester,
  ) async {
    await mount(tester);
    final timed = showViewerNotification(
      pageContext,
      description: 'Default delay',
      autoHide: true,
    );
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 2999));
    expect(find.text('Default delay'), findsOneWidget);
    await tester.pump(const Duration(milliseconds: 1));
    expect(await timed, isFalse);
    expect(find.text('Default delay'), findsNothing);
    final manual = showViewerNotification(
      pageContext,
      description: 'Manual',
      autoHide: false,
      hideAfterMilisecond: 1,
    );
    await tester.pump();
    await tester.pump(const Duration(seconds: 10));
    expect(find.text('Manual'), findsOneWidget);
    await tester.tap(find.text('OK'));
    await tester.pump();
    expect(await manual, isTrue);
  });

  testWidgets('timers cancel on manual close, replacement and disposal', (
    tester,
  ) async {
    await mount(tester);
    final closed = showViewerNotification(
      pageContext,
      description: 'Close me',
      autoHide: true,
      hideAfterMilisecond: 1000,
    );
    await tester.pump();
    await tester.tap(find.text('OK'));
    await tester.pump();
    expect(await closed, isTrue);
    final replaced = showViewerNotification(
      pageContext,
      description: 'Replace me',
      autoHide: true,
      hideAfterMilisecond: 1000,
    );
    await tester.pump();
    final current = showViewerNotification(pageContext, description: 'Keep me');
    await tester.pump();
    expect(await replaced, isFalse);
    await tester.pump(const Duration(seconds: 2));
    expect(find.text('Keep me'), findsOneWidget);
    await tester.tap(find.text('OK'));
    await tester.pump();
    expect(await current, isTrue);
    final disposed = showViewerNotification(
      pageContext,
      description: 'Dispose me',
      autoHide: true,
      hideAfterMilisecond: 1000,
    );
    await tester.pump();
    await tester.pumpWidget(const SizedBox());
    expect(await disposed, isFalse);
    await tester.pump(const Duration(seconds: 2));
    expect(tester.takeException(), isNull);
  });

  testWidgets('zero delay hides immediately and negative delay is rejected', (
    tester,
  ) async {
    await mount(tester);
    final result = showViewerNotification(
      pageContext,
      description: 'Immediate',
      autoHide: true,
      hideAfterMilisecond: 0,
    );
    await tester.pump();
    await tester.pump(Duration.zero);
    expect(await result, isFalse);
    expect(find.text('Immediate'), findsNothing);
    expect(
      () => showViewerNotification(
        pageContext,
        description: 'Invalid',
        autoHide: true,
        hideAfterMilisecond: -1,
      ),
      throwsArgumentError,
    );
  });

  testWidgets('Vietnamese dark popup wraps and scrolls in a small window', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(320, 280);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    await mount(
      tester,
      locale: const Locale('vi'),
      brightness: Brightness.dark,
    );
    final result = showViewerNotification(
      pageContext,
      description: List.filled(20, 'Mô tả thông báo.').join(' '),
      confirmation: true,
    );
    await tester.pumpAndSettle();
    expect(find.text('Thông tin'), findsOneWidget);
    final popup = find.byKey(const ValueKey('viewer-notification'));
    expect(tester.getRect(popup).width, 280);
    expect(tester.widget<Material>(popup).color, const Color(0xff171b21));
    await tester.ensureVisible(find.text('Xác nhận'));
    await tester.tap(find.text('Xác nhận'));
    await tester.pumpAndSettle();
    expect(await result, isTrue);
    expect(tester.takeException(), isNull);
  });

  testWidgets('copy uses success popup and writes the actual clipboard', (
    tester,
  ) async {
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
    await mount(tester);
    await copyText(pageContext, 'copied payload', 'Copied successfully');
    await tester.pumpAndSettle();
    expect(copied, 'copied payload');
    expect(find.byIcon(Icons.check_circle_outline_rounded), findsOneWidget);
    expect(find.text('Success'), findsOneWidget);
    expect(find.text('Copied successfully'), findsOneWidget);
    expect(find.byType(SnackBar), findsNothing);
    await tester.tap(find.text('OK'));
    await tester.pumpAndSettle();
  });
}
