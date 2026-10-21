import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';
import '../services/app_settings.dart';
import '../services/app_updater.dart';
import '../l10n/strings.dart';
import 'settings_screen.dart';
import 'help_dialog.dart';

import '../models/capture.dart';
import '../services/capture_store.dart';
import '../services/viewer_server.dart';
import '../services/viewer_mcp_server.dart';
import 'components.dart';
import 'viewer_notification.dart';
import 'animated_pill_indicator.dart';
import 'application_tabs.dart';
import 'viewer_dropdown.dart';
import 'inspector.dart';
import 'network_request_table.dart';
import 'process_resource_usage.dart';
import 'selection_builder.dart';
import 'resizable_split_view.dart';
import 'json_beautify_screen.dart';
import 'json_document.dart';
import 'capture_scope.dart';

class ViewerShell extends StatefulWidget {
  const ViewerShell({
    super.key,
    required this.store,
    required this.server,
    required this.settings,
    required this.mcpServer,
    this.updater,
  });
  final CaptureStore store;
  final ViewerServer server;
  final AppSettings settings;
  final ViewerMcpServer mcpServer;
  final AppUpdater? updater;
  @override
  State<ViewerShell> createState() => _ViewerShellState();
}

class _ViewerShellState extends State<ViewerShell>
    with SingleTickerProviderStateMixin {
  CaptureStore get store => widget.store;
  ViewerServer get server => widget.server;
  String? _session;
  final _networkSelection = ValueNotifier<String?>(null);
  final _logSelection = ValueNotifier<int?>(null);
  set _networkKey(String? value) => _networkSelection.value = value;
  set _logId(int? value) => _logSelection.value = value;
  List<NetworkRecord>? _cachedRequests;
  List<ConsoleRecord>? _cachedLogs;
  (String?, String, String, String)? _requestFilter;
  (String?, String, String, String)? _logFilter;
  int _activeTab = 0;
  bool get _console => _activeTab == 1;
  bool get _beautify => _activeTab == 2;
  String _query = '';
  String _method = 'All methods';
  String _status = 'All statuses';
  String _level = 'All levels';
  String _logSource = 'All log sources';
  int? _firstConsoleId;
  int? _lastConsoleId;
  List<ConsoleRecord> _groupedLogs = [];
  final _search = TextEditingController();
  late final TabController _tabs;
  late final Widget _beautifyScreen = RepaintBoundary(
    child: JsonBeautifyScreen(
      initialInputFraction: widget.settings.jsonInputFraction,
      onInputFractionChanged: (fraction) =>
          _rememberLayout(widget.settings.setJsonInputFraction(fraction)),
    ),
  );
  NetworkRecord? _inspectedRequest;
  Json? _inspectedRequestEvent;
  Json? _inspectedCompletionEvent;
  Widget? _requestInspector;
  ConsoleRecord? _inspectedLog;
  Widget? _logInspector;
  late int _captureRevision;

  @override
  void initState() {
    super.initState();
    _captureRevision = store.revision;
    _tabs = TabController(
      length: 3,
      vsync: this,
      animationDuration: const Duration(milliseconds: 400),
    );
    store.addListener(_captureChanged);
    server.addListener(_refresh);
  }

  void _captureChanged() {
    _cachedRequests = null;
    _cachedLogs = null;
    if (_captureRevision != store.revision) {
      _captureRevision = store.revision;
      _networkKey = null;
      _logId = null;
      _inspectedRequest = null;
      _inspectedRequestEvent = null;
      _inspectedCompletionEvent = null;
      _requestInspector = null;
      _inspectedLog = null;
      _logInspector = null;
      _firstConsoleId = null;
      _lastConsoleId = null;
      _groupedLogs = [];
      JsonDocumentCache.shared.clear();
      PaintingBinding.instance.imageCache.clear();
      PaintingBinding.instance.imageCache.clearLiveImages();
    }
    _refresh();
  }

  void _refresh() {
    if (mounted) {
      setState(() {
        if (_session != null && !store.sessions.containsKey(_session)) {
          _session = null;
          _networkKey = null;
          _logId = null;
        }
      });
    }
  }

  @override
  void dispose() {
    store.removeListener(_captureChanged);
    server.removeListener(_refresh);
    _networkSelection.dispose();
    _logSelection.dispose();
    _search.dispose();
    _tabs.dispose();
    super.dispose();
  }

  List<NetworkRecord> get _requests {
    final filter = (_session, _method, _status, _query);
    if (_cachedRequests != null && _requestFilter == filter) {
      return _cachedRequests!;
    }
    _requestFilter = filter;
    return _cachedRequests = store.network.values
        .where((r) {
          if (_session != null && r.sessionId != _session) return false;
          if (_method != 'All methods' && r.method != _method) return false;
          if (_status == 'Errors' && !r.failed) return false;
          if (_status == 'Pending' && !r.pending) return false;
          if (_status == 'Success' &&
              (r.status == null || r.status! < 200 || r.status! >= 400)) {
            return false;
          }
          if (_query.isEmpty) return true;
          return '${r.method} ${r.url} ${r.state} ${r.source}'
              .toLowerCase()
              .contains(_query);
        })
        .toList()
        .reversed
        .toList();
  }

  List<ConsoleRecord> get _consoleEntries {
    final first = store.logs.firstOrNull?.id;
    final last = store.logs.lastOrNull?.id;
    if (first != _firstConsoleId || last != _lastConsoleId) {
      _firstConsoleId = first;
      _lastConsoleId = last;
      _groupedLogs = ConsoleRecord.grouped(store.logs);
    }
    return _groupedLogs;
  }

  List<ConsoleRecord> get _logs {
    final filter = (_session, _logSource, _level, _query);
    if (_cachedLogs != null && _logFilter == filter) return _cachedLogs!;
    _logFilter = filter;
    return _cachedLogs = _consoleEntries
        .where(
          (r) =>
              (_session == null || r.sessionId == _session) &&
              (_logSource == 'All log sources' || r.source == _logSource) &&
              (_level == 'All levels' || r.level == _level) &&
              (_query.isEmpty || r.searchText.toLowerCase().contains(_query)),
        )
        .toList()
        .reversed
        .toList();
  }

  Future<void> _export() async {
    // Snapshot before the native save panel opens, while live capture continues.
    final snapshot = pretty(store.export());
    try {
      final location = await getSaveLocation(
        suggestedName:
            'flutter-capture-${DateTime.now().millisecondsSinceEpoch}.json',
        acceptedTypeGroups: [
          XTypeGroup(label: 'JSON capture', extensions: ['json']),
        ],
      );
      if (location == null) return;
      await File(location.path).writeAsString(snapshot);
      if (mounted) {
        _notice(
          context.l10n.exportedTo(location.path),
          type: ViewerNotificationType.success,
        );
      }
    } catch (e) {
      if (mounted) _notice(context.l10n.exportFailed(e.toString()));
    }
  }

  void _notice(
    String message, {
    ViewerNotificationType type = ViewerNotificationType.danger,
  }) => showViewerNotification(context, type: type, description: message);

  Future<void> _rememberLayout(Future<void> save) async {
    try {
      await save;
    } catch (_) {
      if (mounted) _notice(context.l10n.settingsSaveFailed);
    }
  }

  Widget _networkInspector(NetworkRecord record) {
    if (identical(record, _inspectedRequest) &&
        identical(record.requestEvent, _inspectedRequestEvent) &&
        identical(record.completionEvent, _inspectedCompletionEvent)) {
      return _requestInspector!;
    }
    _inspectedRequest = record;
    _inspectedRequestEvent = record.requestEvent;
    _inspectedCompletionEvent = record.completionEvent;
    return _requestInspector = NetworkInspector(
      key: const ValueKey('network-inspector'),
      record: record,
      initialTab: widget.settings.requestDetailTab,
      onTabChanged: (index) =>
          _rememberLayout(widget.settings.setRequestDetailTab(index)),
    );
  }

  Widget _consoleInspector(ConsoleRecord record) {
    if (identical(record, _inspectedLog)) return _logInspector!;
    _inspectedLog = record;
    return _logInspector = ConsoleInspector(
      key: const ValueKey('console-inspector'),
      record: record,
    );
  }

  Future<void> _settings() => Navigator.of(context).push<void>(
    MaterialPageRoute(
      builder: (context) => SettingsScreen(
        settings: widget.settings,
        server: server,
        mcpServer: widget.mcpServer,
        updater: widget.updater,
      ),
    ),
  );

  Future<void> _connectionGuide() async {
    final input = TextEditingController(text: server.endpoint);
    String? validation;
    try {
      final value = await showDialog<String>(
        context: context,
        builder: (context) => StatefulBuilder(
          builder: (context, update) => AlertDialog(
            title: Text(context.l10n.guideTitle),
            content: SizedBox(
              width: 610,
              child: SingleChildScrollView(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(
                      context.l10n.flutterConnectionGuide,
                      style: TextStyle(color: context.palette.description),
                    ),
                    const SizedBox(height: 20),
                    TextField(
                      key: const ValueKey('connect-vm-uri'),
                      controller: input,
                      maxLines: 3,
                      minLines: 1,
                      decoration: InputDecoration(
                        labelText: context.l10n.vmServiceUrl,
                        hintText: 'ws://127.0.0.1:12345/token=/ws',
                        errorText: validation,
                        errorMaxLines: 3,
                      ),
                    ),
                    const SizedBox(height: 16),
                    Text(
                      context.l10n.flutterCaptureScope,
                      style: TextStyle(color: context.palette.description),
                    ),
                  ],
                ),
              ),
            ),
            actions: [
              TextButton.icon(
                onPressed: () => Navigator.pop(context, 'auto'),
                icon: const Icon(Icons.radar, size: 18),
                label: Text(context.l10n.findFlutterApps),
              ),
              TextButton(
                onPressed: () => Navigator.pop(context),
                child: Text(context.l10n.gotIt),
              ),
              FilledButton(
                onPressed: () {
                  try {
                    final uri = ViewerServer.normalizeUri(input.text);
                    Navigator.pop(context, uri.toString());
                  } on FormatException {
                    update(() => validation = context.l10n.urlValidation);
                  }
                },
                child: Text(context.l10n.connectApp),
              ),
            ],
          ),
        ),
      );
      if (value == null || !mounted) return;
      if (value == 'auto') {
        await _autoConnect();
        return;
      }
      final preferences = widget.settings.preferences;
      if (preferences != null &&
          !await preferences.setString('vmServiceUri', value)) {
        if (mounted) _notice(context.l10n.settingsSaveFailed);
        return;
      }
      if (preferences != null &&
          !await preferences.setBool('autoDetectVmService', false)) {
        if (mounted) _notice(context.l10n.settingsSaveFailed);
        return;
      }
      await server.start(uri: value);
    } finally {
      // The route's closing animation may still be using the input controller.
      await Future<void>.delayed(const Duration(milliseconds: 300));
      input.dispose();
    }
  }

  Future<void> _autoConnect({String? uri}) async {
    final preferences = widget.settings.preferences;
    try {
      if (preferences != null &&
          !await preferences.setBool('autoDetectVmService', true)) {
        throw StateError('Could not save discovery preference');
      }
      await server.start(automatic: true, uri: uri);
    } catch (_) {
      if (mounted) _notice(context.l10n.settingsSaveFailed);
    }
  }

  @override
  Widget build(BuildContext context) {
    final requests = _requests;
    final logs = _logs;
    return Scaffold(
      body: SafeArea(
        child: Column(
          children: [
            _header(),
            if (server.searching)
              Container(
                width: double.infinity,
                padding: const EdgeInsets.symmetric(
                  horizontal: 18,
                  vertical: 10,
                ),
                color: context.palette.accent.withValues(alpha: .08),
                child: Wrap(
                  crossAxisAlignment: WrapCrossAlignment.center,
                  spacing: 12,
                  runSpacing: 8,
                  children: [
                    Text(
                      server.discoveredApps.isEmpty
                          ? context.l10n.waitingForFlutterApps
                          : context.l10n.chooseFlutterApp,
                      style: TextStyle(color: context.palette.description),
                    ),
                    for (final app in server.discoveredApps)
                      OutlinedButton(
                        onPressed: () => _autoConnect(uri: app.uri),
                        child: Text('${app.name} · ${Uri.parse(app.uri).port}'),
                      ),
                  ],
                ),
              ),
            if (server.error != null)
              _banner(
                context.l10n.serverStartFailed(server.error!),
                context.palette.danger,
              ),
            if (store.paused)
              _banner(context.l10n.pausedBanner, context.palette.warning),
            Expanded(
              child: LayoutBuilder(
                builder: (context, constraints) {
                  final inspector = constraints.maxWidth >= 900;
                  return Column(
                    children: [
                      _toolbar(),
                      Expanded(
                        child: IndexedStack(
                          index: _beautify ? 1 : 0,
                          children: [
                            Column(
                              children: [
                                _filters(),
                                Expanded(
                                  child: inspector
                                      ? ResizableSplitView(
                                          key: ValueKey(_console),
                                          defaultWidth: _console ? 400 : null,
                                          initialWidth: _console
                                              ? widget
                                                    .settings
                                                    .consoleInspectorWidth
                                              : widget
                                                    .settings
                                                    .networkInspectorWidth,
                                          onWidthChanged: (width) =>
                                              _rememberLayout(
                                                widget.settings
                                                    .setInspectorWidth(
                                                      width,
                                                      console: _console,
                                                    ),
                                              ),
                                          list: _console
                                              ? _consoleList(logs, true)
                                              : _networkList(requests, true),
                                          inspector: _console
                                              ? ValueListenableBuilder<int?>(
                                                  valueListenable:
                                                      _logSelection,
                                                  builder: (context, id, _) {
                                                    final record = logs
                                                        .where(
                                                          (r) => r.id == id,
                                                        )
                                                        .firstOrNull;
                                                    return record == null
                                                        ? _noSelection()
                                                        : _consoleInspector(
                                                            record,
                                                          );
                                                  },
                                                )
                                              : ValueListenableBuilder<String?>(
                                                  valueListenable:
                                                      _networkSelection,
                                                  builder: (context, key, _) {
                                                    final record = requests
                                                        .where(
                                                          (r) => r.key == key,
                                                        )
                                                        .firstOrNull;
                                                    return record == null
                                                        ? _noSelection()
                                                        : _networkInspector(
                                                            record,
                                                          );
                                                  },
                                                ),
                                        )
                                      : _console
                                      ? _consoleList(logs, false)
                                      : _networkList(requests, false),
                                ),
                              ],
                            ),
                            _beautifyScreen,
                          ],
                        ),
                      ),
                    ],
                  );
                },
              ),
            ),
            _footer(),
          ],
        ),
      ),
    );
  }

  Widget _header() => Container(
    padding: EdgeInsets.symmetric(horizontal: 22, vertical: 4),
    decoration: BoxDecoration(
      color: context.palette.surface,
      border: Border(bottom: BorderSide(color: context.palette.line)),
    ),
    child: Row(
      children: [
        Container(
          padding: EdgeInsets.all(8),
          decoration: BoxDecoration(
            color: context.palette.accent.withValues(alpha: .12),
            borderRadius: BorderRadius.circular(8),
          ),
          child: Icon(
            Icons.hub_outlined,
            color: context.palette.accent,
            size: 22,
          ),
        ),
        SizedBox(width: 12),
        Expanded(
          child: Text(
            'Flutter Network Viewer',
            style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600),
          ),
        ),
        StatusChip(
          server.searching
              ? context.l10n.searchingFlutterApps
              : server.busy
              ? context.l10n.starting
              : server.running
              ? context.l10n.connected
              : context.l10n.serverStopped,
          leading: server.searching
              ? SizedBox.square(
                  dimension: 12,
                  child: CircularProgressIndicator(
                    key: const ValueKey('discovery-loading'),
                    strokeWidth: 2,
                    color: context.palette.accent,
                  ),
                )
              : null,
          color: server.running
              ? context.palette.accent
              : context.palette.muted,
        ),
        SizedBox(width: 8),
        IconButton(
          tooltip: server.running || server.searching
              ? context.l10n.stopServer
              : context.l10n.startServer,
          color: server.running || server.searching
              ? const Color(0xffeab308)
              : context.palette.accent,
          onPressed: server.busy && !server.searching
              ? null
              : () async {
                  if (server.running || server.searching) {
                    await server.stop();
                  } else {
                    await server.start(
                      automatic: server.autoDetect,
                      uri:
                          widget.settings.preferences?.getString(
                            'vmServiceUri',
                          ) ??
                          server.endpoint,
                    );
                  }
                },
          icon: Icon(
            server.running || server.searching
                ? Icons.stop_circle_outlined
                : Icons.play_circle_outline,
            size: 21,
          ),
        ),
        IconButton(
          tooltip: context.l10n.settings,
          onPressed: _settings,
          icon: Icon(Icons.settings_outlined, size: 20),
        ),
        IconButton(
          tooltip: context.l10n.helpTitle,
          onPressed: () => showDialog<void>(
            context: context,
            builder: (_) => const HelpDialog(),
          ),
          icon: Icon(Icons.help_outline, size: 20),
        ),
      ],
    ),
  );

  Widget _toolbar() => Padding(
    padding: EdgeInsets.fromLTRB(20, 8, 12, 4),
    child: Row(
      children: [
        SizedBox(
          height: 38,
          child: Material(
            color: context.palette.input,
            borderRadius: BorderRadius.circular(30),
            clipBehavior: Clip.antiAlias,
            child: Padding(
              padding: const EdgeInsets.all(3),
              child: TabBar(
                controller: _tabs,
                isScrollable: true,
                tabAlignment: TabAlignment.start,
                onTap: (index) => setState(() => _activeTab = index),
                dividerColor: Colors.transparent,
                indicatorSize: TabBarIndicatorSize.tab,
                indicatorAnimation: TabIndicatorAnimation.linear,
                indicatorPadding: const EdgeInsets.symmetric(horizontal: 4),
                indicator: AnimatedPillIndicator(
                  animation: _tabs.animation!,
                  color: context.palette.primaryFill,
                  start: _tabs.previousIndex.toDouble(),
                  end: _tabs.index.toDouble(),
                ),
                splashBorderRadius: BorderRadius.circular(30),
                overlayColor: WidgetStateProperty.resolveWith(
                  (states) => states.contains(WidgetState.hovered)
                      ? Colors.transparent
                      : null,
                ),
                labelColor: Colors.white,
                unselectedLabelColor: context.palette.muted,
                labelStyle: const TextStyle(
                  fontSize: 13,
                  fontWeight: FontWeight.w600,
                ),
                unselectedLabelStyle: const TextStyle(fontSize: 13),
                labelPadding: const EdgeInsets.symmetric(horizontal: 18),
                tabs: [
                  _tab(
                    context.l10n.network,
                    Icons.swap_vert,
                    store.network.length,
                  ),
                  _tab(
                    context.l10n.console,
                    Icons.terminal,
                    _consoleEntries.length,
                  ),
                  _tab('Json beautify', Icons.data_object_rounded),
                ],
              ),
            ),
          ),
        ),
        if (_beautify) const Spacer(),
        if (!_beautify) ...[
          const SizedBox(width: 20),
          Expanded(
            child: Align(
              alignment: Alignment.centerRight,
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 480),
                child: ApplicationTabs(
                  sessions: store.sessions.values.toList(),
                  selectedId: _session,
                  isOnline: server.isOnline,
                  onChanged: (id) => setState(() {
                    _session = id;
                    _networkKey = null;
                    _logId = null;
                  }),
                ),
              ),
            ),
          ),
          const SizedBox(width: 12),
          IconButton(
            tooltip: store.paused
                ? context.l10n.resumeCapture
                : context.l10n.pauseCapture,
            onPressed: store.togglePause,
            icon: Icon(
              store.paused ? Icons.play_arrow : Icons.pause,
              size: 20,
              color: store.paused ? context.palette.warning : null,
            ),
          ),
          IconButton(
            tooltip: context.l10n.clearCapture,
            onPressed: () {
              store.clear();
              setState(() {
                _networkKey = null;
                _logId = null;
              });
            },
            icon: Icon(Icons.delete_sweep_outlined, size: 20),
          ),
          IconButton(
            tooltip: context.l10n.exportCapture,
            onPressed: store.network.isEmpty && store.logs.isEmpty
                ? null
                : _export,
            icon: Icon(Icons.file_download_outlined, size: 20),
          ),
        ],
      ],
    ),
  );

  Widget _tab(String name, IconData icon, [int? count]) => Tab(
    child: Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 17),
        SizedBox(width: 8),
        Text(name),
        if (count != null) ...[
          SizedBox(width: 8),
          Text('$count', style: TextStyle(fontSize: 11)),
        ],
      ],
    ),
  );

  Widget _filters() => Container(
    padding: EdgeInsets.fromLTRB(20, 0, 20, 8),
    decoration: BoxDecoration(
      border: Border(bottom: BorderSide(color: context.palette.line)),
    ),
    child: Column(
      children: [
        Row(
          children: [
            Expanded(
              child: TextField(
                controller: _search,
                onChanged: (value) =>
                    setState(() => _query = value.toLowerCase()),
                decoration: InputDecoration(
                  hintText: _console
                      ? context.l10n.searchConsole
                      : context.l10n.searchNetwork,
                  prefixIcon: Icon(Icons.search, size: 18),
                  suffixIcon: _query.isEmpty
                      ? null
                      : IconButton(
                          tooltip: context.l10n.clearSearch,
                          onPressed: () => setState(() {
                            _search.clear();
                            _query = '';
                          }),
                          icon: Icon(Icons.close, size: 16),
                        ),
                ),
              ),
            ),
            SizedBox(width: 10),
            if (_console) ...[
              _dropdown(
                _logSource,
                [
                  'All log sources',
                  'dart.stdout',
                  'dart.stderr',
                  'dart.logging',
                ],
                (v) => setState(() => _logSource = v),
                160,
              ),
              const SizedBox(width: 8),
              _dropdown(
                _level,
                ['All levels', 'log', 'info', 'warn', 'error', 'debug'],
                (v) => setState(() => _level = v),
                140,
                colorFor: (v) =>
                    v == 'All levels' ? null : context.palette.logColor(v),
              ),
            ] else ...[
              _dropdown(
                _method,
                [
                  'All methods',
                  'GET',
                  'POST',
                  'PUT',
                  'PATCH',
                  'DELETE',
                  'HEAD',
                  'OPTIONS',
                ],
                (v) => setState(() => _method = v),
                150,
                colorFor: (v) =>
                    v == 'All methods' ? null : context.palette.methodColor(v),
              ),
              SizedBox(width: 8),
              _dropdown(
                _status,
                ['All statuses', 'Success', 'Errors', 'Pending'],
                (v) => setState(() => _status = v),
                150,
                colorFor: (v) => switch (v) {
                  'Success' => context.palette.accent,
                  'Errors' => context.palette.danger,
                  'Pending' => context.palette.muted,
                  _ => null,
                },
              ),
            ],
          ],
        ),
      ],
    ),
  );

  Widget _dropdown(
    String value,
    List<String> values,
    ValueChanged<String> onChanged,
    double width, {
    Color? Function(String)? colorFor,
  }) => SizedBox(
    width: width,
    child: ViewerDropdown<String>(
      value: value,
      options: values
          .map(
            (v) => DropdownOption(
              v,
              filterLabel(context, v),
              color: colorFor?.call(v),
            ),
          )
          .toList(),
      onChanged: onChanged,
    ),
  );

  Widget _networkList(List<NetworkRecord> requests, bool inspector) {
    if (requests.isEmpty) return _empty(store.network.isEmpty);
    return NetworkRequestTable(
      records: requests,
      selectedKey: null,
      selection: _networkSelection,
      applicationName: (record) => store.sessions[record.sessionId]?.name ?? '',
      initialWidths: widget.settings.networkColumnWidths,
      onWidthsChanged: (widths) =>
          _rememberLayout(widget.settings.setNetworkColumnWidths(widths)),
      onSelected: (record) {
        _networkKey = record.key;
        if (!inspector) _detail(() => _networkInspector(record));
      },
    );
  }

  Widget _consoleList(List<ConsoleRecord> logs, bool inspector) {
    if (logs.isEmpty) return _empty(store.logs.isEmpty);
    return ListView.builder(
      itemCount: logs.length,
      itemBuilder: (context, index) {
        final r = logs[index];
        final palette = context.palette;
        final color = palette.logColor(r.level);
        return SelectionBuilder<int>(
          key: ValueKey(r.id),
          selection: _logSelection,
          value: r.id,
          builder: (context, selected) => Material(
            key: ValueKey('console-row-${r.id}'),
            color: selected ? palette.selectedRow : Colors.transparent,
            animationDuration: Duration.zero,
            child: InkWell(
              splashFactory: NoSplash.splashFactory,
              highlightColor: Colors.transparent,
              hoverDuration: Duration.zero,
              onTap: () {
                _logId = r.id;
                if (!inspector) _detail(() => ConsoleInspector(record: r));
              },
              child: Container(
                constraints: const BoxConstraints(minHeight: 36),
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
                decoration: BoxDecoration(
                  border: Border(bottom: BorderSide(color: palette.line)),
                ),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    StatusChip(
                      r.level.toUpperCase(),
                      color: color,
                      prominent: true,
                      selected: selected,
                    ),
                    const SizedBox(width: 6),
                    Text(
                      clockTime(r.time),
                      maxLines: 1,
                      softWrap: false,
                      style: mono.copyWith(
                        fontSize: 10,
                        color: selected
                            ? palette.onSelectedRowMuted
                            : palette.muted,
                      ),
                    ),
                    const SizedBox(width: 6),
                    Expanded(
                      child: Text(
                        r.displayMessage,
                        style: mono.copyWith(
                          color: selected
                              ? palette.onSelectedRow
                              : r.level == 'error'
                              ? palette.danger
                              : null,
                        ),
                        softWrap: true,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        );
      },
    );
  }

  Widget _empty(bool noData) => EmptyState(
    icon: noData
        ? (_console ? Icons.terminal : Icons.sensors)
        : Icons.search_off,
    title: noData
        ? (_console
              ? context.l10n.consoleEmptyTitle
              : context.l10n.networkEmptyTitle)
        : context.l10n.noResults,
    action: noData
        ? OutlinedButton.icon(
            onPressed: _connectionGuide,
            icon: Icon(Icons.link, size: 16),
            label: Text(context.l10n.connectApp),
          )
        : TextButton(
            onPressed: () => setState(() {
              _search.clear();
              _query = '';
              _method = 'All methods';
              _status = 'All statuses';
              _level = 'All levels';
              _logSource = 'All log sources';
              _session = null;
            }),
            child: Text(context.l10n.resetFilters),
          ),
  );

  Widget _noSelection() =>
      EmptyState(icon: Icons.manage_search, title: context.l10n.selectionTitle);

  void _detail(Widget Function() buildInspector) => showCaptureDialog(
    context: context,
    store: store,
    builder: (context) => Dialog(
      child: SizedBox(
        width: 700,
        height: 700,
        child: Column(
          children: [
            Align(
              alignment: Alignment.centerRight,
              child: IconButton(
                tooltip: context.l10n.closeInspector,
                onPressed: () => Navigator.pop(context),
                icon: Icon(Icons.close),
              ),
            ),
            Expanded(
              child: ListenableBuilder(
                listenable: store,
                builder: (context, child) => buildInspector(),
              ),
            ),
          ],
        ),
      ),
    ),
  );

  Widget _banner(String message, Color color) => Container(
    width: double.infinity,
    padding: EdgeInsets.symmetric(horizontal: 22, vertical: 10),
    color: color.withValues(alpha: .1),
    child: Text(message, style: TextStyle(color: color, fontSize: 12)),
  );

  Widget _footer() => Container(
    height: 25,
    padding: EdgeInsets.symmetric(horizontal: 18),
    decoration: BoxDecoration(
      color: context.palette.surface,
      border: Border(top: BorderSide(color: context.palette.line)),
    ),
    child: Row(
      children: [
        Icon(
          Icons.circle,
          size: 6,
          color: server.running
              ? context.palette.accent
              : context.palette.muted,
        ),
        SizedBox(width: 7),
        Text(
          context.l10n.connectionCount(server.connections),
          style: TextStyle(fontSize: 10, color: context.palette.muted),
        ),
        SizedBox(width: 16),
        Expanded(
          child: Text(
            context.l10n.captureStats(
              _console ? _logs.length : _requests.length,
              store.evicted,
              store.ignoredWhilePaused,
            ),
            overflow: TextOverflow.ellipsis,
            style: TextStyle(fontSize: 10, color: context.palette.muted),
          ),
        ),
        if (server.invalidMessages + store.rejected > 0)
          Tooltip(
            message: server.lastProtocolError ?? context.l10n.unsupportedEvent,
            child: Text(
              context.l10n.invalidCount(
                server.invalidMessages + store.rejected,
              ),
              style: TextStyle(fontSize: 10, color: context.palette.danger),
            ),
          ),
        const SizedBox(width: 16),
        const ProcessResourceUsage(),
      ],
    ),
  );
}
