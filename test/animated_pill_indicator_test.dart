import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/ui/animated_pill_indicator.dart';

void main() {
  for (final destination in [1.0, 2.0]) {
    testWidgets('pill shrinks and fades between tabs 0 and $destination', (
      tester,
    ) async {
      final animation = AnimationController(
        vsync: tester,
        upperBound: destination,
        duration: const Duration(milliseconds: 400),
      );
      var repaints = 0;
      final painter = AnimatedPillIndicator(
        animation: animation,
        end: destination,
        color: const Color(0xff08785d),
      ).createBoxPainter(() => repaints++);

      void expectPill(double width, double height, double opacity) {
        final canvas = TestRecordingCanvas();
        painter.paint(
          canvas,
          const Offset(20, 10),
          const ImageConfiguration(size: Size(100, 32)),
        );
        final args = canvas.invocations.single.invocation.positionalArguments;
        final pill = args[0] as RRect;
        final paint = args[1] as Paint;
        expect(pill.width, closeTo(width, .001));
        expect(pill.height, closeTo(height, .001));
        expect(pill.outerRect.center, const Offset(70, 26));
        expect(paint.color.a, closeTo(opacity, .001));
      }

      expectPill(100, 32, 1);
      animation.forward();
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 200));
      expectPill(80, 25.6, .8);
      await tester.pump(const Duration(milliseconds: 200));
      expectPill(100, 32, 1);

      animation.reverse();
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 200));
      expectPill(80, 25.6, .8);
      await tester.pump(const Duration(milliseconds: 200));
      expectPill(100, 32, 1);
      expect(repaints, greaterThan(0));

      painter.dispose();
      final previousRepaints = repaints;
      animation.value = .5;
      expect(repaints, previousRepaints);
      animation.dispose();
    });
  }
}
