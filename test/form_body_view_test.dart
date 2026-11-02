import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/l10n/app_localizations.dart';
import 'package:flutter_network_viewer/ui/network_body_section.dart';
import 'package:flutter_network_viewer/ui/components.dart';

Widget app(Map<String, dynamic> data, {String language = 'en'}) => MaterialApp(
  theme: viewerTheme(Brightness.light),
  locale: Locale(language),
  localizationsDelegates: AppLocalizations.localizationsDelegates,
  supportedLocales: AppLocalizations.supportedLocales,
  home: Scaffold(
    body: NetworkBodySection(data: data, fillAvailable: true, showTitle: false),
  ),
);

void main() {
  testWidgets(
    'multipart displays field values and file metadata, with exact Raw toggle',
    (tester) async {
      final body = base64Encode([
        ...utf8.encode(
          '--sample\r\nContent-Disposition: form-data; name="title"\r\n\r\nCà phê\r\n--sample\r\nContent-Disposition: form-data; name="document"; filename="demo.pdf"\r\nContent-Type: application/pdf\r\n\r\n',
        ),
        0,
        255,
        254,
        1,
        ...utf8.encode('\r\n--sample--\r\n'),
      ]);
      await tester.pumpWidget(
        app({
          'headers': {'content-type': 'multipart/form-data; boundary=sample'},
          'body': body,
          'bodyEncoding': 'base64',
        }),
      );
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('form-body-fields')), findsOneWidget);
      expect(find.text('title'), findsOneWidget);
      expect(find.text('Cà phê'), findsOneWidget);
      expect(find.text('demo.pdf'), findsOneWidget);
      expect(find.text('application/pdf · 4 B'), findsOneWidget);
      expect(
        find.byKey(const ValueKey('captured-image-preview')),
        findsNothing,
      );
      await tester.tap(find.byKey(const ValueKey('form-body-toggle')));
      await tester.pumpAndSettle();
      expect(find.text(body), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('form-body-toggle')));
      await tester.pumpAndSettle();
      expect(find.text('demo.pdf'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );
  testWidgets(
    'body updates replace form fields and MIME changes discard form preview',
    (tester) async {
      await tester.pumpWidget(
        app({
          'headers': {'content-type': 'application/x-www-form-urlencoded'},
          'body': 'name=An&name=Binh',
        }),
      );
      await tester.pumpAndSettle();
      expect(find.text('An'), findsOneWidget);
      expect(find.text('Binh'), findsOneWidget);
      await tester.pumpWidget(
        app({
          'headers': {'content-type': 'text/plain'},
          'body': 'name=An&name=Binh',
        }),
      );
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('form-body-fields')), findsNothing);
      expect(find.text('name=An&name=Binh'), findsOneWidget);
    },
  );
  for (final language in ['en', 'vi']) {
    testWidgets(
      'omitted body explains size limit even without captured value in $language',
      (tester) async {
        await tester.pumpWidget(
          app({
            'headers': {'content-type': 'multipart/form-data; boundary=sample'},
            'bodyOmitted': 'profile-size-limit',
            'bodyCaptureLimit': 4 * 1024 * 1024,
            'bodyByteLength': 59774016,
          }, language: language),
        );
        await tester.pumpAndSettle();
        expect(
          find.text(
            language == 'en' ? 'Body not captured' : 'Chưa thu thập body',
          ),
          findsOneWidget,
        );
        expect(find.textContaining('4 MB'), findsOneWidget);
        expect(tester.takeException(), isNull);
      },
    );
  }
}
