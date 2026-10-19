import 'dart:async';

import 'package:flutter/material.dart';
import 'package:shared_preferences/shared_preferences.dart';

import 'services/capture_store.dart';
import 'services/app_settings.dart';
import 'services/app_updater.dart';
import 'l10n/app_localizations.dart';
import 'services/viewer_server.dart';
import 'services/viewer_mcp_server.dart';
import 'ui/viewer_shell.dart';
import 'ui/viewer_theme.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final preferences = await SharedPreferences.getInstance();
  final store = CaptureStore();
  final server = ViewerServer(store);
  final updater = AppUpdater.desktop();
  runApp(
    ViewerApp(
      store: store,
      server: server,
      preferences: preferences,
      updater: updater,
    ),
  );
  WidgetsBinding.instance.addPostFrameCallback((_) {
    unawaited(updater.check(background: true));
  });
  unawaited(
    server.start(
      automatic:
          preferences.getBool('autoDetectVmService') ??
          ViewerServer.defaultServiceUri.isEmpty,
      uri:
          preferences.getString('vmServiceUri') ??
          ViewerServer.defaultServiceUri,
    ),
  );
}

class ViewerApp extends StatefulWidget {
  const ViewerApp({
    super.key,
    required this.store,
    required this.server,
    this.preferences,
    this.mcpServer,
    this.updater,
  });
  final CaptureStore store;
  final ViewerServer server;
  final SharedPreferences? preferences;
  final ViewerMcpServer? mcpServer;
  final AppUpdater? updater;

  @override
  State<ViewerApp> createState() => _ViewerAppState();
}

class _ViewerAppState extends State<ViewerApp> {
  late final AppSettings _settings = AppSettings(widget.preferences);
  late final ViewerMcpServer _mcp =
      widget.mcpServer ?? ViewerMcpServer(widget.store, widget.server);

  @override
  void initState() {
    super.initState();
    _settings.addListener(_syncMcp);
    _syncMcp();
  }

  void _syncMcp() => unawaited(
    _mcp.configure(enabled: _settings.mcpEnabled, port: _settings.mcpPort),
  );

  @override
  void dispose() {
    _settings.removeListener(_syncMcp);
    _settings.dispose();
    if (widget.mcpServer == null) _mcp.dispose();
    widget.updater?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: _settings,
    builder: (context, child) => MaterialApp(
      title: 'Flutter Network Viewer',
      locale: _settings.locale,
      supportedLocales: AppLocalizations.supportedLocales,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      debugShowCheckedModeBanner: false,
      theme: viewerTheme(Brightness.light),
      darkTheme: viewerTheme(Brightness.dark),
      themeMode: _settings.themeMode,
      home: ViewerShell(
        store: widget.store,
        server: widget.server,
        settings: _settings,
        mcpServer: _mcp,
        updater: widget.updater,
      ),
    ),
  );
}
