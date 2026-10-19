import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

import '../models/capture.dart';
import '../l10n/strings.dart';
import 'components.dart';
import 'animated_pill_indicator.dart';
import 'network_body_section.dart';

class NetworkInspector extends StatefulWidget {
  const NetworkInspector({
    super.key,
    required this.record,
    this.initialTab = 0,
    this.onTabChanged,
  });
  final NetworkRecord record;
  final int initialTab;
  final ValueChanged<int>? onTabChanged;

  @override
  State<NetworkInspector> createState() => _NetworkInspectorState();
}

class _NetworkInspectorState extends State<NetworkInspector>
    with SingleTickerProviderStateMixin {
  late final TabController _tabs;
  late int _lastTab;
  NetworkRecord get record => widget.record;

  @override
  void initState() {
    super.initState();
    _lastTab = widget.initialTab;
    _tabs = TabController(
      length: 6,
      vsync: this,
      initialIndex: _lastTab,
      animationDuration: const Duration(milliseconds: 400),
    );
    _tabs.addListener(_rememberTab);
  }

  void _rememberTab() {
    if (_tabs.index == _lastTab) return;
    setState(() => _lastTab = _tabs.index);
    widget.onTabChanged?.call(_lastTab);
  }

  @override
  void dispose() {
    _tabs.removeListener(_rememberTab);
    _tabs.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Column(
    children: [
      Padding(
        padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 16),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Padding(
              padding: const EdgeInsets.only(top: 2),
              child: Icon(
                Icons.data_object,
                size: 17,
                color: context.palette.accent,
              ),
            ),
            const SizedBox(width: 8),
            Expanded(
              child: SelectableText(
                record.request.isEmpty
                    ? context.l10n.missingRequest
                    : record.url,
                style: mono,
              ),
            ),
          ],
        ),
      ),
      Padding(
        padding: const EdgeInsets.fromLTRB(18, 0, 18, 12),
        child: Align(
          alignment: Alignment.centerLeft,
          child: SizedBox(
            height: 38,
            child: Material(
              color: context.palette.input,
              borderRadius: BorderRadius.circular(30),
              clipBehavior: Clip.antiAlias,
              child: _TabScrollHints(
                child: Padding(
                  padding: const EdgeInsets.symmetric(vertical: 3),
                  child: ScrollConfiguration(
                    behavior: ScrollConfiguration.of(
                      context,
                    ).copyWith(dragDevices: {...PointerDeviceKind.values}),
                    child: TabBar(
                      controller: _tabs,
                      isScrollable: true,
                      padding: const EdgeInsets.symmetric(horizontal: 3),
                      tabAlignment: TabAlignment.start,
                      dividerColor: Colors.transparent,
                      indicatorSize: TabBarIndicatorSize.tab,
                      indicatorAnimation: TabIndicatorAnimation.linear,
                      indicatorPadding: const EdgeInsets.symmetric(
                        horizontal: 4,
                      ),
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
                        fontSize: 12,
                        fontWeight: FontWeight.w600,
                      ),
                      unselectedLabelStyle: const TextStyle(fontSize: 12),
                      labelPadding: const EdgeInsets.symmetric(horizontal: 18),
                      tabs: [
                        Tab(text: context.l10n.overview),
                        Tab(text: context.l10n.requestHeader),
                        Tab(text: context.l10n.requestBody),
                        Tab(text: context.l10n.responseHeader),
                        Tab(text: context.l10n.responseBody),
                        Tab(text: context.l10n.raw),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
      Expanded(
        child: TabBarView(
          controller: _tabs,
          children: [
            ListView(
              children: [
                DataSection(context.l10n.requestId, record.requestId),
                DataSection(context.l10n.source, record.source),
                DataSection(
                  context.l10n.started,
                  record.time?.toLocal().toIso8601String(),
                ),
                DataSection(
                  context.l10n.status,
                  record.pending
                      ? context.l10n.waitingResponse
                      : '${filterLabel(context, record.state)} ${record.response['statusText'] ?? ''}',
                ),
                DataSection(
                  context.l10n.responseSize,
                  context.l10n.capturedBytes(record.responseBytes),
                ),
                if (record.error.isNotEmpty)
                  DataSection(context.l10n.error, record.error),
                DataSection(
                  context.l10n.queryParameters,
                  Uri.tryParse(record.url)?.queryParametersAll,
                ),
              ],
            ),
            ListView(
              children: [
                DataSection(
                  context.l10n.headers,
                  record.request['headers'],
                  showTitle: false,
                ),
              ],
            ),
            NetworkBodySection(
              data: record.request,
              fillAvailable: true,
              showTitle: false,
            ),
            _responsePane(body: false),
            _responsePane(body: true),
            SingleChildScrollView(
              padding: EdgeInsets.all(18),
              child: SelectableText(pretty(record.toJson()), style: mono),
            ),
          ],
        ),
      ),
    ],
  );

  Widget _responsePane({required bool body}) {
    if (body) {
      return Column(
        children: [
          if (record.pending)
            Padding(
              padding: const EdgeInsets.all(18),
              child: Text(
                context.l10n.waitingResponse,
                style: TextStyle(color: context.palette.muted),
              ),
            ),
          if (record.error.isNotEmpty)
            Flexible(
              child: SingleChildScrollView(
                child: DataSection(context.l10n.error, record.error),
              ),
            ),
          Expanded(
            flex: 3,
            child: NetworkBodySection(
              data: record.response,
              fillAvailable: true,
              showTitle: false,
            ),
          ),
        ],
      );
    }
    return ListView(
      children: [
        if (record.pending)
          Padding(
            padding: const EdgeInsets.all(18),
            child: Text(
              context.l10n.waitingResponse,
              style: TextStyle(color: context.palette.muted),
            ),
          ),
        if (record.error.isNotEmpty)
          DataSection(context.l10n.error, record.error),
        DataSection(
          context.l10n.headers,
          record.response['headers'],
          showTitle: false,
        ),
      ],
    );
  }
}

/// The TabBar owns its scroll controller; its notifications also report changes
/// caused by resizing, selecting a tab, or restoring the saved tab.
class _TabScrollHints extends StatefulWidget {
  const _TabScrollHints({required this.child});

  final Widget child;

  @override
  State<_TabScrollHints> createState() => _TabScrollHintsState();
}

class _TabScrollHintsState extends State<_TabScrollHints> {
  bool _left = false;
  bool _right = false;

  void _update(ScrollMetrics metrics) {
    if (metrics.axis != Axis.horizontal) return;
    final reversed = metrics.axisDirection == AxisDirection.left;
    final before = metrics.extentBefore > 1;
    final after = metrics.extentAfter > 1;
    final left = reversed ? after : before;
    final right = reversed ? before : after;
    if (left == _left && right == _right) return;
    setState(() {
      _left = left;
      _right = right;
    });
  }

  Widget _edge({required bool left, required bool visible}) {
    final blue = context.palette.primaryFill;
    return Positioned(
      left: left ? 0 : null,
      right: left ? null : 0,
      top: 0,
      bottom: 0,
      width: 32,
      child: IgnorePointer(
        child: AnimatedOpacity(
          opacity: visible ? 1 : 0,
          duration: const Duration(milliseconds: 160),
          child: DecoratedBox(
            decoration: BoxDecoration(
              gradient: LinearGradient(
                begin: left ? Alignment.centerLeft : Alignment.centerRight,
                end: left ? Alignment.centerRight : Alignment.centerLeft,
                colors: [
                  blue.withValues(alpha: .5),
                  blue.withValues(alpha: .18),
                  blue.withValues(alpha: 0),
                ],
                stops: const [0, .45, 1],
              ),
            ),
          ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) =>
      NotificationListener<ScrollMetricsNotification>(
        onNotification: (notification) {
          _update(notification.metrics);
          return false;
        },
        child: NotificationListener<ScrollNotification>(
          onNotification: (notification) {
            _update(notification.metrics);
            return false;
          },
          child: Stack(
            children: [
              widget.child,
              _edge(left: true, visible: _left),
              _edge(left: false, visible: _right),
            ],
          ),
        ),
      );
}

class ConsoleInspector extends StatelessWidget {
  const ConsoleInspector({super.key, required this.record});
  final ConsoleRecord record;
  @override
  Widget build(BuildContext context) => ListView(
    children: [
      Padding(
        padding: EdgeInsets.all(18),
        child: Text(
          context.l10n.consoleInspector,
          style: TextStyle(fontWeight: FontWeight.w600),
        ),
      ),
      Padding(
        padding: const EdgeInsets.symmetric(horizontal: 18),
        child: Wrap(
          spacing: 10,
          runSpacing: 8,
          crossAxisAlignment: WrapCrossAlignment.center,
          children: [
            StatusChip(
              record.level.toUpperCase(),
              color: context.palette.logColor(record.level),
              prominent: true,
            ),
            Text(record.sourceLabel, style: mono),
            if (record.event['severity'] != null)
              Text(
                '${context.l10n.level}: ${record.event['severity']}',
                style: mono,
              ),
          ],
        ),
      ),
      if (record.loggerName.isNotEmpty)
        DataSection(context.l10n.loggerName, record.loggerName),
      if (record.event['isolateName'] != null)
        DataSection('Isolate', record.event['isolateName']),
      DataSection(
        context.l10n.timestamp,
        record.time?.toLocal().toIso8601String(),
      ),
      DataSection(context.l10n.message, record.displayMessage, body: true),
      if (record.error != null) DataSection(context.l10n.error, record.error),
      if (record.stackTrace != null)
        DataSection(context.l10n.stackTrace, record.stackTrace),
      if (record.source == 'js.console' && record.event['args'] != null)
        DataSection(context.l10n.arguments, record.event['args']),
      ExpansionTile(
        title: Text(context.l10n.rawEvent, style: TextStyle(fontSize: 12)),
        children: [
          Padding(
            padding: EdgeInsets.all(18),
            child: SelectableText(pretty(record.event), style: mono),
          ),
        ],
      ),
    ],
  );
}
