import 'package:flutter/material.dart';
import 'package:package_info_plus/package_info_plus.dart';

import '../l10n/strings.dart';
import '../services/app_settings.dart';
import '../services/app_updater.dart';
import '../services/viewer_server.dart';
import '../services/viewer_mcp_server.dart';
import 'components.dart';
import 'viewer_dropdown.dart';

class SettingsScreen extends StatefulWidget {
  const SettingsScreen({
    super.key,
    required this.settings,
    required this.server,
    required this.mcpServer,
    this.updater,
  });
  final AppSettings settings;
  final ViewerServer server;
  final ViewerMcpServer mcpServer;
  final AppUpdater? updater;

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends State<SettingsScreen> {
  late final TextEditingController _port;
  late final TextEditingController _mcpPort;
  late final Future<String> _version;
  bool _savingLanguage = false;
  bool _savingTheme = false;
  bool _savingPort = false;
  bool _invalidPort = false;
  bool _savingMcp = false;
  bool _invalidMcpPort = false;

  @override
  void initState() {
    super.initState();
    _version = PackageInfo.fromPlatform()
        .then((info) => info.version)
        .onError((_, _) => '—');
    _port = TextEditingController(
      text:
          widget.settings.preferences?.getString('vmServiceUri') ??
          widget.server.endpoint,
    );
    _mcpPort = TextEditingController(text: '${widget.settings.mcpPort}');
  }

  @override
  void dispose() {
    _port.dispose();
    _mcpPort.dispose();
    super.dispose();
  }

  Future<void> _changeLanguage(String? code) async {
    if (code == null) return;
    setState(() => _savingLanguage = true);
    try {
      await widget.settings.setLanguage(code);
    } catch (_) {
      if (mounted) _notice(context.l10n.settingsSaveFailed);
    } finally {
      if (mounted) setState(() => _savingLanguage = false);
    }
  }

  Future<void> _changeTheme(ThemeMode? mode) async {
    if (mode == null) return;
    setState(() => _savingTheme = true);
    try {
      await widget.settings.setThemeMode(mode);
    } catch (_) {
      if (mounted) _notice(context.l10n.settingsSaveFailed);
    } finally {
      if (mounted) setState(() => _savingTheme = false);
    }
  }

  Future<void> _savePort() async {
    String uri;
    try {
      uri = ViewerServer.normalizeUri(_port.text).toString();
      setState(() => _invalidPort = false);
    } on FormatException {
      setState(() => _invalidPort = true);
      return;
    }
    setState(() => _savingPort = true);
    try {
      final preferences = widget.settings.preferences;
      if (preferences != null &&
          !await preferences.setString('vmServiceUri', uri)) {
        throw StateError('Could not persist port preference');
      }
      if (preferences != null &&
          !await preferences.setBool('autoDetectVmService', false)) {
        throw StateError('Could not persist discovery preference');
      }
      await widget.server.stop();
      await widget.server.start(uri: uri);
      if (mounted) {
        _notice(
          widget.server.running
              ? context.l10n.portSaved
              : context.l10n.serverStartFailed(widget.server.error ?? ''),
        );
      }
    } catch (_) {
      if (mounted) _notice(context.l10n.settingsSaveFailed);
    } finally {
      if (mounted) setState(() => _savingPort = false);
    }
  }

  Future<void> _setAutoDetect(bool enabled) async {
    setState(() => _savingPort = true);
    try {
      final preferences = widget.settings.preferences;
      if (preferences != null &&
          !await preferences.setBool('autoDetectVmService', enabled)) {
        throw StateError('Could not persist discovery preference');
      }
      await widget.server.start(
        automatic: enabled,
        uri: _port.text.trim().isEmpty ? null : _port.text,
      );
    } catch (_) {
      if (mounted) _notice(context.l10n.settingsSaveFailed);
    } finally {
      if (mounted) setState(() => _savingPort = false);
    }
  }

  void _notice(String text) =>
      ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(text)));

  Future<void> _changeMcp(bool enabled) async {
    setState(() => _savingMcp = true);
    try {
      await widget.settings.setMcpEnabled(enabled);
      await widget.mcpServer.configure(
        enabled: enabled,
        port: widget.settings.mcpPort,
      );
    } catch (_) {
      if (mounted) _notice(context.l10n.settingsSaveFailed);
    } finally {
      if (mounted) setState(() => _savingMcp = false);
    }
  }

  Future<void> _saveMcpPort() async {
    final port = int.tryParse(_mcpPort.text.trim());
    setState(() => _invalidMcpPort = port == null || port < 1 || port > 65535);
    if (_invalidMcpPort) return;
    setState(() => _savingMcp = true);
    try {
      final changed = widget.settings.mcpPort != port;
      await widget.settings.setMcpPort(port!);
      await widget.mcpServer.configure(
        enabled: widget.settings.mcpEnabled,
        port: port,
        restart: !changed,
      );
      if (mounted && widget.mcpServer.error == null) {
        _notice(context.l10n.portSaved);
      }
    } catch (_) {
      if (mounted) _notice(context.l10n.settingsSaveFailed);
    } finally {
      if (mounted) setState(() => _savingMcp = false);
    }
  }

  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: Listenable.merge([
      widget.settings,
      widget.server,
      widget.mcpServer,
      if (widget.updater != null) widget.updater!,
    ]),
    builder: (context, child) => Scaffold(
      appBar: AppBar(
        title: Text(context.l10n.settings),
        backgroundColor: context.palette.surface,
        surfaceTintColor: Colors.transparent,
        elevation: 0,
        scrolledUnderElevation: 0,
      ),
      body: LayoutBuilder(
        builder: (context, constraints) {
          final horizontalPadding = constraints.maxWidth > 820
              ? (constraints.maxWidth - 764) / 2
              : 28.0;
          return ListView(
            padding: EdgeInsets.symmetric(
              horizontal: horizontalPadding,
              vertical: 28,
            ),
            children: [
              _section(Icons.tune, context.l10n.appearance, [
                _field(
                  context.l10n.theme,
                  ViewerDropdown<ThemeMode>(
                    value: widget.settings.themeMode,
                    options: [
                      DropdownOption(ThemeMode.light, context.l10n.lightTheme),
                      DropdownOption(ThemeMode.dark, context.l10n.darkTheme),
                    ],
                    onChanged: _savingTheme ? null : _changeTheme,
                  ),
                ),
                const SizedBox(height: 20),
                _field(
                  context.l10n.language,
                  ViewerDropdown<String>(
                    value: widget.settings.languageCode,
                    options: const [
                      DropdownOption('vi', 'Tiếng Việt'),
                      DropdownOption('en', 'English'),
                    ],
                    onChanged: _savingLanguage ? null : _changeLanguage,
                  ),
                ),
              ]),
              const SizedBox(height: 20),
              _section(Icons.lan_outlined, context.l10n.connection, [
                SwitchListTile(
                  key: const ValueKey('auto-detect-vm-service'),
                  contentPadding: EdgeInsets.zero,
                  title: Text(context.l10n.autoDetectFlutterApps),
                  subtitle: Text(context.l10n.autoDetectFlutterAppsDescription),
                  value: widget.server.autoDetect,
                  onChanged: _savingPort ? null : _setAutoDetect,
                ),
                const SizedBox(height: 16),
                _field(
                  context.l10n.vmServiceUrl,
                  TextField(
                    key: const ValueKey('vm-service-uri'),
                    controller: _port,
                    keyboardType: TextInputType.url,
                    // Background discovery/retries must not disable editing.
                    enabled: !_savingPort,
                    decoration: InputDecoration(
                      hintText: 'ws://127.0.0.1:12345/token=/ws',
                      errorMaxLines: 3,
                      errorText: _invalidPort
                          ? context.l10n.urlValidation
                          : null,
                    ),
                    onChanged: (_) {
                      if (_invalidPort) setState(() => _invalidPort = false);
                    },
                    onSubmitted: (_) {
                      if (!_savingPort) _savePort();
                    },
                  ),
                ),
                const SizedBox(height: 20),
                Align(
                  alignment: Alignment.centerLeft,
                  child: FilledButton.icon(
                    onPressed: _savingPort ? null : _savePort,
                    icon: const Icon(Icons.save_outlined, size: 18),
                    label: Text(
                      _savingPort
                          ? context.l10n.saving
                          : context.l10n.saveRestart,
                    ),
                  ),
                ),
              ]),
              const SizedBox(height: 20),
              _mcpSection(),
              if (widget.updater case final updater?) ...[
                const SizedBox(height: 20),
                _section(Icons.system_update_alt, context.l10n.appUpdates, [
                  FutureBuilder<String>(
                    future: _version,
                    builder: (context, snapshot) => Text(
                      'Version ${snapshot.data ?? '…'} - Nguyễn Quốc An',
                      key: const ValueKey('app-version'),
                      style: TextStyle(color: context.palette.muted),
                    ),
                  ),
                  const SizedBox(height: 12),
                  Text(
                    switch (updater.status) {
                      UpdateStatus.idle => context.l10n.updateReady,
                      UpdateStatus.checking => context.l10n.updateChecking,
                      UpdateStatus.available => context.l10n.updateAvailable,
                      UpdateStatus.current => context.l10n.updateCurrent,
                      UpdateStatus.failed => context.l10n.updateFailed,
                      UpdateStatus.installing => context.l10n.updateInstalling,
                    },
                    style: TextStyle(
                      color: switch (updater.status) {
                        UpdateStatus.idle => context.palette.muted,
                        UpdateStatus.failed => context.palette.danger,
                        UpdateStatus.available || UpdateStatus.current =>
                          context.palette.methodColor('POST'),
                        UpdateStatus.checking ||
                        UpdateStatus.installing => context.palette.info,
                      },
                    ),
                  ),
                  const SizedBox(height: 16),
                  FilledButton.icon(
                    key: const ValueKey('check-app-updates'),
                    onPressed: updater.supported && !updater.busy
                        ? () => updater.check()
                        : null,
                    icon: const Icon(Icons.refresh, size: 18),
                    label: Text(context.l10n.checkForUpdates),
                  ),
                ]),
              ],
            ],
          );
        },
      ),
    ),
  );

  Widget _mcpSection() {
    final mcp = widget.mcpServer;
    final busy = _savingMcp || mcp.busy;
    final status = StatusChip(
      mcp.busy
          ? context.l10n.starting
          : mcp.running
          ? context.l10n.mcpListening
          : widget.settings.mcpEnabled
          ? context.l10n.serverStopped
          : context.l10n.mcpDisabled,
      color: mcp.running ? context.palette.accent : context.palette.muted,
    );
    final port = _field(
      context.l10n.mcpPort,
      TextField(
        key: const ValueKey('mcp-port'),
        controller: _mcpPort,
        keyboardType: TextInputType.number,
        enabled: !busy,
        decoration: InputDecoration(
          contentPadding: const EdgeInsets.symmetric(
            horizontal: 12,
            vertical: 14,
          ),
          errorText: _invalidMcpPort ? context.l10n.portValidation : null,
          errorMaxLines: 4,
        ),
        onChanged: (_) {
          if (_invalidMcpPort) setState(() => _invalidMcpPort = false);
        },
        onSubmitted: (_) {
          if (!busy) _saveMcpPort();
        },
      ),
    );
    final endpoint = _field(
      context.l10n.mcpEndpoint,
      Container(
        constraints: const BoxConstraints(minHeight: 44),
        padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
        decoration: BoxDecoration(
          color: context.palette.input,
          borderRadius: BorderRadius.circular(7),
        ),
        child: Center(
          widthFactor: 1,
          heightFactor: 1,
          child: SelectableText(mcp.endpoint, style: mono),
        ),
      ),
    );
    return _section(
      Icons.hub_outlined,
      'MCP',
      [
        if (mcp.error != null) ...[
          Text(
            context.l10n.serverStartFailed(mcp.error!),
            key: const ValueKey('mcp-error'),
            style: TextStyle(color: context.palette.danger),
          ),
          const SizedBox(height: 16),
        ],
        LayoutBuilder(
          builder: (context, constraints) => constraints.maxWidth >= 680
              ? Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SizedBox(width: 160, child: port),
                    const SizedBox(width: 20),
                    Expanded(child: endpoint),
                  ],
                )
              : Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [port, const SizedBox(height: 16), endpoint],
                ),
        ),
        const SizedBox(height: 20),
        SizedBox(
          width: double.infinity,
          child: Wrap(
            alignment: WrapAlignment.end,
            spacing: 12,
            runSpacing: 12,
            children: [
              OutlinedButton.icon(
                key: const ValueKey('save-mcp-port'),
                onPressed: busy ? null : _saveMcpPort,
                style: OutlinedButton.styleFrom(
                  minimumSize: const Size(0, 44),
                  visualDensity: VisualDensity.standard,
                  tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                  shape: RoundedRectangleBorder(
                    borderRadius: BorderRadius.circular(10),
                  ),
                ),
                icon: const Icon(Icons.save_outlined, size: 18),
                label: Text(
                  _savingMcp ? context.l10n.saving : context.l10n.saveRestart,
                ),
              ),
              _mcpCopyButton(),
            ],
          ),
        ),
      ],
      trailing: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          status,
          const SizedBox(width: 12),
          Tooltip(
            message: context.l10n.enableMcp,
            child: Semantics(
              label: context.l10n.enableMcp,
              child: Switch(
                key: const ValueKey('mcp-enabled'),
                value: widget.settings.mcpEnabled,
                onChanged: busy ? null : _changeMcp,
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _mcpCopyButton() {
    final mcp = widget.mcpServer;
    return MenuAnchor(
      consumeOutsideTap: true,
      alignmentOffset: const Offset(0, 6),
      style: MenuStyle(
        backgroundColor: WidgetStatePropertyAll(context.palette.surface),
        surfaceTintColor: const WidgetStatePropertyAll(Colors.transparent),
        padding: const WidgetStatePropertyAll(EdgeInsets.all(6)),
        shape: WidgetStatePropertyAll(
          RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
            side: BorderSide(color: context.palette.line),
          ),
        ),
      ),
      menuChildren: [
        MenuItemButton(
          key: const ValueKey('copy-mcp-json'),
          onPressed: () => copyText(context, mcp.clientConfiguration),
          style: const ButtonStyle(
            minimumSize: WidgetStatePropertyAll(Size(0, 44)),
          ),
          leadingIcon: const Icon(Icons.data_object, size: 18),
          child: Text(context.l10n.copyMcpJson),
        ),
        MenuItemButton(
          key: const ValueKey('copy-mcp-codex'),
          onPressed: () => copyText(context, mcp.codexConfiguration),
          style: const ButtonStyle(
            minimumSize: WidgetStatePropertyAll(Size(0, 44)),
          ),
          leadingIcon: const Icon(Icons.terminal, size: 18),
          child: Text(context.l10n.copyMcpCodex),
        ),
      ],
      builder: (context, controller, child) => ClipRRect(
        borderRadius: BorderRadius.circular(10),
        child: ColoredBox(
          color: context.palette.primaryFill,
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Flexible(
                child: FilledButton.icon(
                  key: const ValueKey('copy-mcp-setup'),
                  onPressed: () => copyText(context, mcp.setupCommand),
                  style: FilledButton.styleFrom(
                    minimumSize: const Size(0, 44),
                    visualDensity: VisualDensity.standard,
                    tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                    backgroundColor: Colors.transparent,
                    foregroundColor: Colors.white,
                    overlayColor: Colors.white,
                    shape: const RoundedRectangleBorder(),
                  ),
                  icon: const Icon(Icons.copy_outlined, size: 18),
                  label: Text(context.l10n.copyMcpSetup),
                ),
              ),
              Container(
                width: 1,
                height: 24,
                color: Colors.white.withValues(alpha: .3),
              ),
              Semantics(
                expanded: controller.isOpen,
                child: IconButton(
                  key: const ValueKey('copy-mcp-formats'),
                  tooltip: context.l10n.mcpOtherFormats,
                  onPressed: () => controller.isOpen
                      ? controller.close()
                      : controller.open(),
                  style: IconButton.styleFrom(
                    minimumSize: const Size(44, 44),
                    visualDensity: VisualDensity.standard,
                    tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                    foregroundColor: Colors.white,
                    overlayColor: Colors.white,
                    shape: const RoundedRectangleBorder(),
                  ),
                  icon: const Icon(Icons.keyboard_arrow_down_rounded, size: 20),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _field(String title, Widget input) => Column(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Text(title, style: TextStyle(color: context.palette.muted, fontSize: 12)),
      const SizedBox(height: 8),
      Semantics(label: title, child: input),
    ],
  );

  Widget _section(
    IconData icon,
    String title,
    List<Widget> children, {
    Widget? trailing,
  }) => Card(
    margin: EdgeInsets.zero,
    child: Padding(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(icon, size: 20, color: context.palette.accent),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  title,
                  style: const TextStyle(
                    fontSize: 16,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              ?trailing,
            ],
          ),
          const SizedBox(height: 24),
          ...children,
        ],
      ),
    ),
  );
}
