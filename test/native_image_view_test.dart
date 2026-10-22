import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/ui/native_image_view.dart';
import 'package:flutter_network_viewer/l10n/app_localizations.dart';
import 'package:flutter_network_viewer/ui/native_capture_list.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;

  testWidgets(
    'inline and enlarged views share bytes; zoom and fit do not reload',
    (tester) async {
      final calls = <MethodCall>[];
      messenger.setMockMethodCallHandler(NativeImages.channel, (call) async {
        calls.add(call);
        return null;
      });
      final image = NativeImageData(Uint8List.fromList([1, 2, 3]));
      final controller = NativeImageController();
      Widget app(bool inline) => MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Column(
          children: [
            if (inline)
              Expanded(
                child: NativeImageView(
                  key: const ValueKey('inline'),
                  image: image,
                  fallback: (_) => const Text('fallback'),
                ),
              ),
            Expanded(
              child: NativeImageView(
                key: const ValueKey('enlarged'),
                image: image,
                controller: controller,
                interactive: true,
                fallback: (_) => const Text('fallback'),
              ),
            ),
          ],
        ),
      );
      await tester.pumpWidget(app(true));
      await tester.pumpAndSettle();
      expect(calls.where((c) => c.method == 'load').length, 1);
      expect(calls.where((c) => c.method == 'create').length, 2);
      expect(find.byType(Image), findsNothing);
      final configure =
          calls.lastWhere((c) => c.method == 'configure').arguments as Map;
      expect(configure['tools'], ['Zoom out', 'Zoom in', 'Fit image', 'Close']);
      calls.clear();
      controller.zoom(1.25);
      controller.fit();
      await tester.pumpAndSettle();
      expect(calls.map((c) => c.method), ['control', 'control']);
      expect((calls.first.arguments as Map)['factor'], 1.25);
      expect((calls.last.arguments as Map)['factor'], null);
      await tester.pumpWidget(app(false));
      await tester.pumpAndSettle();
      expect(calls.any((c) => c.method == 'release'), false);
      await tester.pumpWidget(app(true));
      await tester.pumpAndSettle();
      expect(calls.any((c) => c.method == 'load'), false);
      image
          .release(); // Clear owner while the dialog/views still hold the image.
      await tester.pumpAndSettle();
      expect(calls.any((c) => c.method == 'release'), false);
      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
      expect(calls.where((c) => c.method == 'release').length, 1);
      controller.dispose();
      messenger.setMockMethodCallHandler(NativeImages.channel, null);
    },
  );

  testWidgets(
    'replacing a live native image binds the new resource without recreating the host',
    (tester) async {
      final calls = <MethodCall>[];
      messenger.setMockMethodCallHandler(NativeImages.channel, (call) async {
        calls.add(call);
        return null;
      });
      final first = NativeImageData(Uint8List.fromList([1]));
      final second = NativeImageData(Uint8List.fromList([2]));
      Widget app(NativeImageData image) => MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: NativeImageView(
          image: image,
          fallback: (_) => const Text('fallback'),
        ),
      );
      await tester.pumpWidget(app(first));
      await tester.pumpAndSettle();
      calls.clear();
      await tester.pumpWidget(app(second));
      await tester.pumpAndSettle();
      final bind = calls.singleWhere((c) => c.method == 'bind');
      expect((bind.arguments as Map)['image'], second.id);
      expect(
        calls.any((c) => c.method == 'create' || c.method == 'load'),
        false,
      );
      first.release();
      second.release();
      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
      expect(calls.where((c) => c.method == 'release').length, 2);
      messenger.setMockMethodCallHandler(NativeImages.channel, null);
    },
  );

  testWidgets('native failure releases host and displays Flutter fallback', (
    tester,
  ) async {
    final calls = <MethodCall>[];
    messenger.setMockMethodCallHandler(NativeImages.channel, (call) async {
      calls.add(call);
      if (call.method == 'create') throw PlatformException(code: 'no-host');
      return null;
    });
    final image = NativeImageData(Uint8List(3));
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: NativeImageView(
          image: image,
          fallback: (_) => const Text('fallback'),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.text('fallback'), findsOneWidget);
    expect(calls.any((c) => c.method == 'dispose'), true);
    await tester.pumpWidget(const SizedBox());
    image.release();
    await tester.pumpAndSettle();
    expect(calls.where((c) => c.method == 'release').length, 1);
    messenger.setMockMethodCallHandler(NativeImages.channel, null);
  });

  testWidgets(
    'Windows enlarged image stays native in its own dialog and hides under nested overlays',
    (tester) async {
      final calls = <MethodCall>[];
      messenger.setMockMethodCallHandler(NativeImages.channel, (call) async {
        calls.add(call);
        return null;
      });
      NativeLists.overlayFallback = true;
      final observer = NativeListRouteObserver();
      final image = NativeImageData(Uint8List(3));
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          navigatorObservers: [observer],
          home: Builder(
            builder: (context) => TextButton(
              onPressed: () => showDialog<void>(
                context: context,
                builder: (_) => Dialog(
                  child: SizedBox(
                    width: 400,
                    height: 300,
                    child: NativeImageView(
                      image: image,
                      modal: true,
                      interactive: true,
                      fallback: (_) => const Text('fallback'),
                    ),
                  ),
                ),
              ),
              child: const Text('open'),
            ),
          ),
        ),
      );
      await tester.tap(find.text('open'));
      await tester.pumpAndSettle();
      expect(
        (calls.lastWhere((c) => c.method == 'frame').arguments
            as Map)['visible'],
        true,
      );
      final release = NativeListOverlays.acquire();
      await tester.pumpAndSettle();
      expect(
        (calls.lastWhere((c) => c.method == 'frame').arguments
            as Map)['visible'],
        false,
      );
      release();
      await tester.pumpAndSettle();
      expect(
        (calls.lastWhere((c) => c.method == 'frame').arguments
            as Map)['visible'],
        true,
      );
      await tester.pumpWidget(const SizedBox());
      image.release();
      observer.dispose();
      await tester.pumpAndSettle();
      NativeLists.overlayFallback = false;
      messenger.setMockMethodCallHandler(NativeImages.channel, null);
    },
  );
}
