import 'dart:math' as math;

import 'package:flutter/material.dart';

/// Whole color values only: do not interpret IDs or substrings as colors.
Color? parseBodyColor(String value) {
  final text = value.trim().toLowerCase();
  if (RegExp(r'^#[0-9a-f]{3,8}$').hasMatch(text)) {
    var hex = text.substring(1);
    if (hex.length == 3 || hex.length == 4) {
      hex = hex.split('').map((digit) => '$digit$digit').join();
    }
    if (hex.length == 6) return Color(0xff000000 | int.parse(hex, radix: 16));
    if (hex.length == 8) {
      // CSS hex uses RGBA; Flutter's explicit 0x notation uses ARGB.
      return Color(
        int.parse('${hex.substring(6)}${hex.substring(0, 6)}', radix: 16),
      );
    }
    return null;
  }
  if (RegExp(r'^0x[0-9a-f]{8}$').hasMatch(text)) {
    return Color(int.parse(text.substring(2), radix: 16));
  }
  final function = RegExp(r'^(rgba?|hsla?)\(([^()]*)\)$').firstMatch(text);
  if (function != null) {
    final parts = function
        .group(2)!
        .split(RegExp(r'\s*[,/]\s*|\s+'))
        .where((part) => part.isNotEmpty)
        .toList();
    if (parts.length != 3 && parts.length != 4) return null;
    double? channel(String part, double scale) {
      final percent = part.endsWith('%');
      final number = double.tryParse(
        percent ? part.substring(0, part.length - 1) : part,
      );
      if (number == null || !number.isFinite) return null;
      final result = percent ? number * scale / 100 : number;
      return result >= 0 && result <= scale ? result : null;
    }

    final alpha = parts.length == 4 ? channel(parts[3], 1) : 1.0;
    if (alpha == null) return null;
    if (function.group(1)!.startsWith('rgb')) {
      final rgb = parts.take(3).map((part) => channel(part, 255)).toList();
      if (rgb.any((value) => value == null)) return null;
      return Color.fromRGBO(
        rgb[0]!.round(),
        rgb[1]!.round(),
        rgb[2]!.round(),
        alpha,
      );
    }
    final hue = double.tryParse(parts[0].replaceFirst(RegExp(r'deg$'), ''));
    if (hue == null ||
        !hue.isFinite ||
        !parts[1].endsWith('%') ||
        !parts[2].endsWith('%')) {
      return null;
    }
    final saturation = channel(parts[1], 1);
    final lightness = channel(parts[2], 1);
    if (saturation == null || lightness == null) return null;
    return HSLColor.fromAHSL(alpha, hue % 360, saturation, lightness).toColor();
  }
  const names = {
    'black': 0xff000000,
    'white': 0xffffffff,
    'red': 0xffff0000,
    'green': 0xff008000,
    'blue': 0xff0000ff,
    'yellow': 0xffffff00,
    'gray': 0xff808080,
    'grey': 0xff808080,
    'silver': 0xffc0c0c0,
    'maroon': 0xff800000,
    'purple': 0xff800080,
    'fuchsia': 0xffff00ff,
    'lime': 0xff00ff00,
    'olive': 0xff808000,
    'navy': 0xff000080,
    'teal': 0xff008080,
    'aqua': 0xff00ffff,
    'orange': 0xffffa500,
    'transparent': 0x00000000,
  };
  return names.containsKey(text) ? Color(names[text]!) : null;
}

class BodyColorSwatch extends StatelessWidget {
  const BodyColorSwatch({super.key, required this.value, required this.color});
  final String value;
  final Color color;

  @override
  Widget build(BuildContext context) => Tooltip(
    message: value,
    child: Semantics(
      label: value,
      child: SizedBox(
        width: 12,
        height: 12,
        child: CustomPaint(painter: _ColorPainter(color)),
      ),
    ),
  );
}

class _ColorPainter extends CustomPainter {
  const _ColorPainter(this.color);
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final rect = Offset.zero & size;
    final shape = RRect.fromRectAndRadius(
      rect.deflate(.5),
      const Radius.circular(2),
    );
    canvas.save();
    canvas.clipRRect(shape);
    for (var y = 0.0; y < size.height; y += 4) {
      for (var x = 0.0; x < size.width; x += 4) {
        canvas.drawRect(
          Rect.fromLTWH(
            x,
            y,
            math.min(4, size.width - x),
            math.min(4, size.height - y),
          ),
          Paint()
            ..color = ((x + y) / 4).round().isEven
                ? Colors.white
                : const Color(0xffbdbdbd),
        );
      }
    }
    canvas.drawRect(rect, Paint()..color = color);
    canvas.restore();
    canvas.drawRRect(
      shape,
      Paint()
        ..color = const Color(0xff808080)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1,
    );
  }

  @override
  bool shouldRepaint(_ColorPainter oldDelegate) => color != oldDelegate.color;
}
