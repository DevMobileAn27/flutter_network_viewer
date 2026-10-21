import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/main.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';
import 'package:flutter_network_viewer/ui/json_code_view.dart';

void main() {
  testWidgets(
    'captured image previews in request and response with zoom and raw view',
    (tester) async {
      tester.view.physicalSize = const Size(1440, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final encoded = base64Encode(
        File('test/fixtures/preview.png').readAsBytesSync(),
      );
      final fixture =
          jsonDecode(File('test/fixtures/sdk_envelope.json').readAsStringSync())
              as Map<String, dynamic>;
      final events = fixture['events'] as List;
      for (final index in [0, 1]) {
        final section = index == 0 ? 'request' : 'response';
        events[index][section] = {
          ...events[index][section] as Map<String, dynamic>,
          'headers': {
            'Content-Type': ['image/png'],
          },
          'body': encoded,
          'bodyEncoding': 'base64',
        };
      }
      final store = CaptureStore()..ingest(fixture);
      final server = ViewerServer(store);
      await tester.pumpWidget(ViewerApp(store: store, server: server));
      await tester.tap(find.text('/orders?draft=true'));
      await tester.pumpAndSettle();
      for (final tab in ['Request body', 'Response body']) {
        await tester.ensureVisible(find.text(tab));
        await tester.tap(find.text(tab));
        await tester.pumpAndSettle();
        final preview = find.byKey(const ValueKey('captured-image-preview'));
        expect(preview, findsOneWidget);
        expect(tester.getSize(preview).height, greaterThan(320));
        expect(
          tester.getRect(preview).bottom,
          closeTo(
            tester
                .getRect(find.byKey(const ValueKey('json-body-scroll')))
                .bottom,
            .01,
          ),
        );
        expect(find.text('Raw (2.6 KB)'), findsOneWidget);
        final image = tester.widget<Image>(find.byType(Image).first);
        await tester.runAsync(
          () => precacheImage(image.image, tester.element(preview)),
        );
        await tester.pumpAndSettle();
        expect(
          tester.widget<RawImage>(find.byType(RawImage).first).image,
          isNotNull,
        );
        expect(
          find
              .byType(Image)
              .evaluate()
              .every((e) => (e.widget as Image).image is! NetworkImage),
          isTrue,
        );
        await tester.tap(find.byKey(const ValueKey('open-image')));
        await tester.pumpAndSettle();
        expect(find.byType(Dialog), findsOneWidget);
        final viewer = tester.widget<InteractiveViewer>(
          find.byType(InteractiveViewer),
        );
        expect(viewer.transformationController!.value.getMaxScaleOnAxis(), 1);
        await tester.tap(find.byTooltip('Zoom in'));
        await tester.pumpAndSettle();
        expect(
          viewer.transformationController!.value.getMaxScaleOnAxis(),
          greaterThan(1),
        );
        await tester.tap(find.byTooltip('Fit image'));
        await tester.pumpAndSettle();
        expect(viewer.transformationController!.value.getMaxScaleOnAxis(), 1);
        await tester.tap(find.byTooltip('Close'));
        await tester.pumpAndSettle();
        expect(find.byType(Dialog), findsNothing);
        await tester.tap(find.byKey(const ValueKey('image-preview-toggle')));
        await tester.pumpAndSettle();
        expect(find.text(encoded), findsOneWidget);
        expect(preview, findsNothing);
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
        await tester.tap(find.byTooltip('Copy Body (base64)'));
        await tester.pumpAndSettle();
        expect(copied, encoded);
        tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        );
        await tester.tap(find.byKey(const ValueKey('image-preview-toggle')));
        await tester.pumpAndSettle();
        expect(preview, findsOneWidget);
      }
      await tester.tap(find.byKey(const ValueKey('open-image')));
      await tester.pumpAndSettle();
      expect(find.byType(Dialog), findsOneWidget);
      store.clear();
      await tester.pumpAndSettle();
      expect(find.byType(Dialog), findsNothing);
      expect(
        find.byKey(const ValueKey('captured-image-preview')),
        findsNothing,
      );
      expect(PaintingBinding.instance.imageCache.currentSize, 0);
      expect(PaintingBinding.instance.imageCache.liveImageCount, 0);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      server.dispose();
      store.dispose();
    },
  );

  for (final kind in [
    'missing-mime',
    'unmarked',
    'wrapped',
    'json-string',
    'text-mime',
    'transport-base64',
    'url-safe',
    'ordinary-base64',
    'broken-unmarked',
    'jpeg',
    'data-uri',
    'invalid-base64',
    'corrupt-image',
    'binary',
    'json',
  ]) {
    testWidgets('body handles $kind and live response updates', (tester) async {
      tester.view.physicalSize = const Size(1200, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final fixture =
          jsonDecode(File('test/fixtures/sdk_envelope.json').readAsStringSync())
              as Map<String, dynamic>;
      final store = CaptureStore()..ingest(fixture);
      final server = ViewerServer(store);
      await tester.pumpWidget(ViewerApp(store: store, server: server));
      await tester.tap(find.text('/orders?draft=true'));
      await tester.pumpAndSettle();
      await tester.ensureVisible(find.text('Response body'));
      await tester.tap(find.text('Response body'));
      await tester.pumpAndSettle();
      final data = switch (kind) {
        'unmarked' || 'text-mime' => {
          'headers': {'content-type': 'text/plain'},
          'body': base64Encode(
            File('test/fixtures/preview.png').readAsBytesSync(),
          ),
        },
        'wrapped' => {
          'body': base64Encode(
            File('test/fixtures/preview.png').readAsBytesSync(),
          ).replaceAllMapped(RegExp(r'.{40}'), (match) => '${match[0]}\r\n'),
        },
        'json-string' => {
          'body': jsonEncode(
            base64Encode(File('test/fixtures/preview.png').readAsBytesSync()),
          ),
        },
        'transport-base64' => {
          'body': base64Encode(
            utf8.encode(
              base64Encode(File('test/fixtures/preview.png').readAsBytesSync()),
            ),
          ),
          'bodyEncoding': 'base64',
        },
        'url-safe' => {
          'body': base64UrlEncode(
            File('test/fixtures/preview.jpg').readAsBytesSync(),
          ).replaceAll('=', ''),
        },
        'ordinary-base64' => {
          'body': base64Encode(utf8.encode('ordinary text')),
        },
        'broken-unmarked' => {'body': 'iVBORw0KGgo%%%invalid'},
        'jpeg' => {
          'headers': {'CONTENT-TYPE': 'image/jpeg; charset=binary'},
          'body': base64Encode(
            File('test/fixtures/preview.jpg').readAsBytesSync(),
          ),
          'bodyEncoding': 'base64',
        },
        'data-uri' => {
          'body':
              'data:image/png;base64,${base64Encode(File('test/fixtures/preview.png').readAsBytesSync())}',
        },
        'missing-mime' => {
          'headers': <String, dynamic>{},
          'body': base64Encode(
            File('test/fixtures/preview.png').readAsBytesSync(),
          ),
          'bodyEncoding': 'base64',
        },
        'invalid-base64' => {
          'headers': {'content-type': 'image/jpeg'},
          'body': '%%%invalid',
          'bodyEncoding': 'base64',
        },
        'corrupt-image' => {
          'headers': {'content-type': 'image/jpeg'},
          'body': base64Encode([0xff, 0xd8, 0xff]),
          'bodyEncoding': 'base64',
        },
        'binary' => {
          'headers': {'content-type': 'application/octet-stream'},
          'body': base64Encode([1, 2, 3, 4]),
          'bodyEncoding': 'base64',
        },
        _ => {
          'headers': {'content-type': 'application/json'},
          'body': '{"ok":true}',
        },
      };
      store.ingest({
        ...fixture,
        'events': [
          {
            ...fixture['events'][1] as Map<String, dynamic>,
            'response': {'statusCode': 200, ...data},
          },
        ],
      });
      await tester.pumpAndSettle();
      final preview = find.byKey(const ValueKey('captured-image-preview'));
      if (kind == 'json') {
        expect(find.byType(JsonCodeView), findsOneWidget);
        expect(preview, findsNothing);
      } else if (kind == 'binary' ||
          kind == 'ordinary-base64' ||
          kind == 'broken-unmarked') {
        expect(find.text(data['body'] as String), findsOneWidget);
        expect(preview, findsNothing);
      } else {
        expect(preview, findsOneWidget);
        if (kind != 'invalid-base64') {
          final image = tester.widget<Image>(find.byType(Image).first);
          await tester.runAsync(
            () => precacheImage(
              image.image,
              tester.element(preview),
              onError: (_, _) {},
            ),
          );
          await tester.pumpAndSettle();
        }
        if (kind == 'invalid-base64' || kind == 'corrupt-image') {
          expect(find.textContaining('Unable to preview'), findsOneWidget);
          await tester.tap(find.byKey(const ValueKey('image-preview-toggle')));
          await tester.pumpAndSettle();
          expect(find.text(data['body'] as String), findsOneWidget);
        } else {
          expect(
            tester.widget<RawImage>(find.byType(RawImage).first).image,
            isNotNull,
          );
        }
      }
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox());
      server.dispose();
      store.dispose();
    });
  }
}
