import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/ui/viewer_dropdown.dart';
import 'package:flutter_network_viewer/ui/viewer_theme.dart';

void main() {
  for (final brightness in Brightness.values) {
    testWidgets('closed dropdown matches untouched dropdown in $brightness', (
      tester,
    ) async {
      final boundaryKey = GlobalKey();
      await tester.pumpWidget(
        RepaintBoundary(
          key: boundaryKey,
          child: MaterialApp(
            theme: viewerTheme(brightness),
            home: Scaffold(
              body: Row(
                children: [
                  for (var index = 0; index < 2; index++)
                    SizedBox(
                      width: 200,
                      child: ViewerDropdown<String>(
                        key: ValueKey(index),
                        value: 'All methods',
                        options: const [
                          DropdownOption('All methods', 'All methods'),
                          DropdownOption('GET', 'GET'),
                        ],
                        onChanged: (_) {},
                      ),
                    ),
                ],
              ),
            ),
          ),
        ),
      );
      final first = find.byKey(const ValueKey(0));
      final second = find.byKey(const ValueKey(1));
      final mouse = await tester.createGesture(
        kind: ui.PointerDeviceKind.mouse,
      );
      await mouse.addPointer(location: tester.getCenter(first));
      addTearDown(mouse.removePointer);
      await mouse.down(tester.getCenter(first));
      await mouse.up();
      await tester.pumpAndSettle();
      expect(find.byType(MenuItemButton), findsNWidgets(2));
      await mouse.down(tester.getCenter(first));
      await mouse.up();
      await tester.pumpAndSettle();
      expect(find.byType(MenuItemButton), findsNothing);

      final firstOrigin = tester.getTopLeft(first);
      final secondOrigin = tester.getTopLeft(second);
      await tester.runAsync(() async {
        final boundary =
            boundaryKey.currentContext!.findRenderObject()!
                as RenderRepaintBoundary;
        final image = await boundary.toImage();
        final bytes = (await image.toByteData(
          format: ui.ImageByteFormat.rawRgba,
        ))!;
        int pixel(Offset point) => bytes.getUint32(
          (point.dy.floor() * image.width + point.dx.floor()) * 4,
        );
        for (final sample in [const Offset(1, 20), const Offset(20, 5)]) {
          expect(
            pixel(firstOrigin + sample),
            pixel(secondOrigin + sample),
            reason:
                'Closed and untouched dropdowns must have identical border and fill',
          );
        }
        image.dispose();
      });
    });
  }
}
