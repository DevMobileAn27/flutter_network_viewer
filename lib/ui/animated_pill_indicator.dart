import 'dart:math' as math;

import 'package:flutter/material.dart';

/// A sliding pill that shrinks and fades slightly between tab positions.
class AnimatedPillIndicator extends Decoration {
  const AnimatedPillIndicator({
    required this.animation,
    required this.color,
    this.start = 0,
    this.end = 1,
  });

  final Animation<double> animation;
  final Color color;
  final double start;
  final double end;

  @override
  BoxPainter createBoxPainter([VoidCallback? onChanged]) =>
      _PillPainter(animation, color, start, end, onChanged);
}

class _PillPainter extends BoxPainter {
  _PillPainter(
    this.animation,
    this.color,
    this.start,
    this.end,
    super.onChanged,
  ) {
    animation.addListener(_repaint);
  }

  final Animation<double> animation;
  final Color color;
  final double start;
  final double end;

  void _repaint() => onChanged?.call();

  @override
  void paint(Canvas canvas, Offset offset, ImageConfiguration configuration) {
    final bounds = offset & configuration.size!;
    final progress = start == end
        ? 0.0
        : ((animation.value - start) / (end - start)).clamp(0.0, 1.0);
    final pulse = math.sin(math.pi * progress);
    final scale = 1 - .2 * pulse;
    final rect = Rect.fromCenter(
      center: bounds.center,
      width: bounds.width * scale,
      height: bounds.height * scale,
    );
    canvas.drawRRect(
      RRect.fromRectAndRadius(rect, Radius.circular(rect.height / 2)),
      Paint()..color = color.withValues(alpha: color.a * scale),
    );
  }

  @override
  void dispose() {
    animation.removeListener(_repaint);
    super.dispose();
  }
}
