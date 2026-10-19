import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';

class AppSettings extends ChangeNotifier {
  AppSettings(this.preferences)
    : _languageCode = preferences?.getString('languageCode') == 'vi'
          ? 'vi'
          : 'en',
      _themeMode = preferences?.getString('themeMode') == 'dark'
          ? ThemeMode.dark
          : ThemeMode.light,
      _mcpEnabled = preferences?.getBool('mcpEnabled') ?? false,
      _mcpPort = _restoreMcpPort(preferences?.getInt('mcpPort')),
      _networkInspectorWidth = _validWidth(
        preferences?.getDouble('networkInspectorWidth'),
      ),
      _consoleInspectorWidth = _restoreWidth(
        preferences?.getDouble('consoleInspectorWidth'),
      ),
      _jsonInputFraction = _restoreFraction(
        preferences?.getDouble('jsonInputFraction'),
      ),
      _requestDetailTab = _restoreTab(preferences),
      _networkColumnWidths = _restoreNetworkColumns(preferences);

  final SharedPreferences? preferences;
  String _languageCode;
  ThemeMode _themeMode;
  bool _mcpEnabled;
  int _mcpPort;
  double? _networkInspectorWidth;
  double _consoleInspectorWidth;
  double _jsonInputFraction;
  int _requestDetailTab;
  List<double> _networkColumnWidths;
  static const minNetworkColumnWidths = [64.0, 56.0, 96.0];
  static const defaultNetworkColumnWidths = [64.0, 56.0, 108.0];
  List<double> get networkColumnWidths =>
      List.unmodifiable(_networkColumnWidths);
  ThemeMode get themeMode => _themeMode;
  String get languageCode => _languageCode;
  Locale get locale => Locale(_languageCode);
  bool get mcpEnabled => _mcpEnabled;
  int get mcpPort => _mcpPort;
  double? get networkInspectorWidth => _networkInspectorWidth;
  double get consoleInspectorWidth => _consoleInspectorWidth;
  double get jsonInputFraction => _jsonInputFraction;
  int get requestDetailTab => _requestDetailTab;

  static bool _validNetworkColumns(List<dynamic> widths) =>
      widths.length == 3 &&
      List.generate(
        3,
        (i) =>
            widths[i] is num &&
            (widths[i] as num).isFinite &&
            widths[i] >= minNetworkColumnWidths[i] &&
            widths[i] <= 1600,
      ).every((valid) => valid);

  static List<double> _restoreNetworkColumns(SharedPreferences? preferences) {
    try {
      final value = jsonDecode(
        preferences?.getString('networkColumnWidths') ?? 'null',
      );
      if (value is List && _validNetworkColumns(value)) {
        return value.map((width) => (width as num).toDouble()).toList();
      }
    } on FormatException {
      // Ignore an incomplete or invalid layout from an older installation.
    }
    return [...defaultNetworkColumnWidths];
  }

  Future<void> setNetworkColumnWidths(List<double> widths) async {
    if (!_validNetworkColumns(widths)) {
      throw ArgumentError.value(widths, 'widths');
    }
    _networkColumnWidths = List.of(widths);
    if (preferences != null &&
        !await preferences!.setString(
          'networkColumnWidths',
          jsonEncode(widths),
        )) {
      throw StateError('Could not persist network columns');
    }
  }

  static double? _validWidth(double? value) =>
      value != null && value.isFinite && value >= 320 ? value : null;
  static double _restoreWidth(double? value) => _validWidth(value) ?? 400;
  static double _restoreFraction(double? value) =>
      value != null && value.isFinite && value > 0 && value < 1 ? value : .5;
  static const _detailTabs = [
    'overview',
    'requestHeader',
    'requestBody',
    'responseHeader',
    'responseBody',
    'raw',
  ];

  static int _restoreTab(SharedPreferences? preferences) {
    final name = preferences?.getString('requestDetailTabName');
    if (name != null) {
      final index = _detailTabs.indexOf(name);
      return index < 0 ? 0 : index;
    }
    // Retain the meaning of the former four tabs when upgrading.
    return switch (preferences?.getInt('requestDetailTab')) {
      1 => 2, // Request -> Request body
      2 => 4, // Response -> Response body
      3 => 5, // Raw
      _ => 0,
    };
  }

  // Layout widgets update locally. Keep these values current synchronously so
  // another request or pane can restore them even while the disk write awaits.
  // No global notification is needed for these local presentation preferences.
  Future<void> setInspectorWidth(double width, {required bool console}) async {
    if (!width.isFinite || width < 320) {
      throw ArgumentError.value(width, 'width');
    }
    if (console) {
      if (_consoleInspectorWidth == width) return;
      _consoleInspectorWidth = width;
    } else {
      if (_networkInspectorWidth == width) return;
      _networkInspectorWidth = width;
    }
    final key = console ? 'consoleInspectorWidth' : 'networkInspectorWidth';
    if (preferences != null && !await preferences!.setDouble(key, width)) {
      throw StateError('Could not persist inspector width');
    }
  }

  Future<void> setJsonInputFraction(double fraction) async {
    if (!fraction.isFinite || fraction <= 0 || fraction >= 1) {
      throw ArgumentError.value(fraction, 'fraction');
    }
    if (_jsonInputFraction == fraction) return;
    _jsonInputFraction = fraction;
    if (preferences != null &&
        !await preferences!.setDouble('jsonInputFraction', fraction)) {
      throw StateError('Could not persist JSON divider');
    }
  }

  Future<void> setRequestDetailTab(int index) async {
    if (index < 0 || index >= _detailTabs.length) {
      throw ArgumentError.value(index, 'index');
    }
    if (_requestDetailTab == index) return;
    _requestDetailTab = index;
    if (preferences != null &&
        !await preferences!.setString(
          'requestDetailTabName',
          _detailTabs[index],
        )) {
      throw StateError('Could not persist request detail tab');
    }
  }

  static int _restoreMcpPort(int? value) =>
      value != null && value >= 1 && value <= 65535 ? value : 38943;

  Future<void> setMcpEnabled(bool enabled) async {
    if (_mcpEnabled == enabled) return;
    if (preferences != null &&
        !await preferences!.setBool('mcpEnabled', enabled)) {
      throw StateError('Could not persist MCP preference');
    }
    _mcpEnabled = enabled;
    notifyListeners();
  }

  Future<void> setMcpPort(int port) async {
    if (port < 1 || port > 65535) throw ArgumentError.value(port, 'port');
    if (_mcpPort == port) return;
    if (preferences != null && !await preferences!.setInt('mcpPort', port)) {
      throw StateError('Could not persist MCP port');
    }
    _mcpPort = port;
    notifyListeners();
  }

  Future<void> setThemeMode(ThemeMode mode) async {
    if (mode == ThemeMode.system) {
      throw ArgumentError.value(mode, 'mode', 'Unsupported theme');
    }
    if (_themeMode == mode) return;
    if (preferences != null &&
        !await preferences!.setString('themeMode', mode.name)) {
      throw StateError('Could not persist theme preference');
    }
    _themeMode = mode;
    notifyListeners();
  }

  Future<void> setLanguage(String code) async {
    if (!['vi', 'en'].contains(code)) {
      throw ArgumentError.value(code, 'code', 'Unsupported language');
    }
    if (_languageCode == code) return;
    if (preferences != null &&
        !await preferences!.setString('languageCode', code)) {
      throw StateError('Could not persist language preference');
    }
    _languageCode = code;
    notifyListeners();
  }
}
