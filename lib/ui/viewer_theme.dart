import 'package:flutter/material.dart';

extension ViewerThemeContext on BuildContext {
  ViewerPalette get palette =>
      ViewerPalette(Theme.of(this).brightness == Brightness.dark);
}

class ViewerPalette {
  const ViewerPalette(this.dark);
  final bool dark;

  Color get background =>
      dark ? const Color(0xff101317) : const Color(0xfff6f8fa);
  Color get surface => dark ? const Color(0xff171b21) : Colors.white;
  Color get sidebar => dark ? const Color(0xff14181d) : const Color(0xfff0f3f6);
  Color get input => dark ? const Color(0xff20252c) : const Color(0xffedf1f5);
  Color get muted => dark ? const Color(0xff8d98a7) : const Color(0xff596778);
  Color get description =>
      dark ? const Color(0xffb0b7c3) : const Color(0xff4b5563);
  Color get accent => dark ? const Color(0xff93c5fd) : const Color(0xff2563eb);
  Color get primaryFill => const Color(0xff2563eb);
  Color get selectedRow =>
      dark ? const Color(0xff1e40af) : const Color(0xff1d4ed8);
  Color get onSelectedRow => Colors.white;
  Color get onSelectedRowMuted => const Color(0xffdbeafe);
  Color get selectedRowMarker => const Color(0xff93c5fd);
  Color get danger => dark ? const Color(0xffff848a) : const Color(0xffbd293a);
  Color get line => dark ? const Color(0xff2a3038) : const Color(0xffd6dde5);
  Color get warning => dark ? const Color(0xffe4bc87) : const Color(0xff8c560b);
  Color get info => dark ? const Color(0xff91b9f1) : const Color(0xff285da8);
  Color get method => dark ? const Color(0xffb6a3ed) : const Color(0xff7150ad);
  Color get string => dark ? const Color(0xffb2d9a0) : const Color(0xff39712a);

  Color logColor(String level) => switch (level) {
    'error' => methodColor('DELETE'),
    'warn' => methodColor('PUT'),
    'info' => methodColor('GET'),
    'debug' => methodColor('PATCH'),
    _ => methodColor('CONNECT'),
  };

  Color methodColor(String method) => switch (method.toUpperCase()) {
    'GET' => dark ? const Color(0xff60a5fa) : const Color(0xff1d4ed8),
    'POST' => dark ? const Color(0xff34d399) : const Color(0xff047857),
    'PUT' => dark ? const Color(0xfffbbf24) : const Color(0xffb45309),
    'PATCH' => dark ? const Color(0xffc084fc) : const Color(0xff7e22ce),
    'DELETE' => dark ? const Color(0xfff87171) : const Color(0xffb91c1c),
    'HEAD' => dark ? const Color(0xff22d3ee) : const Color(0xff0e7490),
    'OPTIONS' => dark ? const Color(0xffa5b4fc) : const Color(0xff4338ca),
    'TRACE' => dark ? const Color(0xfff472b6) : const Color(0xffbe185d),
    'CONNECT' => dark ? const Color(0xffcbd5e1) : const Color(0xff475569),
    _ => muted,
  };

  Color statusColor(int? status, {bool transportError = false}) {
    if (transportError) return methodColor('DELETE');
    if (status == null) return muted;
    return switch (status) {
      >= 100 && < 200 => methodColor('HEAD'),
      >= 200 && < 300 => methodColor('POST'),
      >= 300 && < 400 => methodColor('OPTIONS'),
      >= 400 && < 500 => methodColor('PUT'),
      >= 500 && < 600 => methodColor('DELETE'),
      _ => muted,
    };
  }
}

ThemeData viewerTheme(Brightness brightness) {
  final colors = ViewerPalette(brightness == Brightness.dark);
  final inputBorder = OutlineInputBorder(
    borderRadius: BorderRadius.circular(7),
    borderSide: BorderSide(color: colors.accent),
  );
  return ThemeData(
    fontFamily: 'JetBrains Mono',
    brightness: brightness,
    scaffoldBackgroundColor: colors.background,
    colorScheme: ColorScheme.fromSeed(
      seedColor: colors.accent,
      brightness: brightness,
      primary: colors.accent,
      surface: colors.surface,
      error: colors.danger,
    ),
    dividerColor: colors.line,
    cardTheme: CardThemeData(color: colors.surface),
    appBarTheme: AppBarTheme(backgroundColor: colors.surface),
    textTheme: TextTheme(
      bodyMedium: const TextStyle(fontSize: 13),
      bodySmall: TextStyle(fontSize: 11, color: colors.muted),
      titleLarge: const TextStyle(fontSize: 22, fontWeight: FontWeight.w600),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: colors.surface,
      hoverColor: Colors.transparent,
      isDense: true,
      border: inputBorder,
      enabledBorder: inputBorder,
      focusedBorder: inputBorder.copyWith(
        borderSide: BorderSide(color: colors.accent, width: 1.5),
      ),
      disabledBorder: inputBorder.copyWith(
        borderSide: BorderSide(color: colors.accent.withValues(alpha: .4)),
      ),
      errorBorder: inputBorder.copyWith(
        borderSide: BorderSide(color: colors.danger),
      ),
      focusedErrorBorder: inputBorder.copyWith(
        borderSide: BorderSide(color: colors.danger, width: 1.5),
      ),
      contentPadding: const EdgeInsets.symmetric(horizontal: 12, vertical: 12),
    ),
    tooltipTheme: const TooltipThemeData(
      waitDuration: Duration(milliseconds: 400),
    ),
    useMaterial3: true,
  );
}
