import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';

import '../l10n/strings.dart';
import '../models/capture.dart';
import '../services/app_settings.dart';
import 'components.dart';
import 'selection_builder.dart';
import 'native_capture_list.dart';

class NetworkRequestTable extends StatefulWidget {
  const NetworkRequestTable({
    super.key,
    required this.records,
    required this.selectedKey,
    this.selection,
    required this.applicationName,
    required this.onSelected,
    required this.initialWidths,
    required this.onWidthsChanged,
    this.visible = true,
  });

  final List<NetworkRecord> records;
  final String? selectedKey;
  final ValueListenable<String?>? selection;
  final String Function(NetworkRecord) applicationName;
  final ValueChanged<NetworkRecord> onSelected;
  // Method, Status and Time have saved widths; URL fills the remaining space.
  final List<double> initialWidths;
  final ValueChanged<List<double>> onWidthsChanged;
  final bool visible;

  @override
  State<NetworkRequestTable> createState() => _NetworkRequestTableState();
}

class _NetworkRequestTableState extends State<NetworkRequestTable> {
  static const _dividerWidth = 6.0;
  static const _minUrlWidth = 140.0;
  late List<double> _widths = List.of(widget.initialWidths);
  var _horizontal = ScrollController();
  var _vertical = ScrollController();
  Offset _nativeScroll = Offset.zero;
  int? _hovered;
  int? _dragging;
  final _localSelection = ValueNotifier<String?>(null);

  void _rememberNativeScroll(Offset offset) {
    if (_nativeScroll == offset) return;
    _nativeScroll = offset;
    ScrollController restore(ScrollController controller, double value) {
      if (controller.hasClients) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted && controller.hasClients) {
            controller.jumpTo(
              value.clamp(0, controller.position.maxScrollExtent),
            );
          }
        });
        return controller;
      }
      controller.dispose();
      return ScrollController(initialScrollOffset: value);
    }

    _horizontal = restore(_horizontal, offset.dx);
    _vertical = restore(_vertical, offset.dy);
  }

  @override
  void dispose() {
    _horizontal.dispose();
    _vertical.dispose();
    _localSelection.dispose();
    super.dispose();
  }

  List<double> _columns(double total) => [
    _widths[0],
    total - _widths.reduce((a, b) => a + b) - 3 * _dividerWidth,
    _widths[1],
    _widths[2],
  ];

  void _resize(int divider, double delta, double total) {
    final columns = _columns(total);
    final next = List<double>.of(_widths);
    final minimum = AppSettings.minNetworkColumnWidths;
    final spare = math.max(0.0, columns[1] - _minUrlWidth);
    switch (divider) {
      case 0:
        next[0] = (next[0] + delta)
            .clamp(minimum[0], math.min(1600, next[0] + spare))
            .toDouble();
      case 1:
        next[1] = (next[1] - delta)
            .clamp(minimum[1], math.min(1600, next[1] + spare))
            .toDouble();
      case 2:
        final sum = next[1] + next[2];
        next[1] = (next[1] + delta)
            .clamp(
              math.max(minimum[1], sum - 1600),
              math.min(1600, sum - minimum[2]),
            )
            .toDouble();
        next[2] = sum - next[1];
    }
    setState(() => _widths = next);
  }

  void _commit() {
    setState(() => _dragging = null);
    widget.onWidthsChanged(List.of(_widths));
  }

  Widget _cells(List<double> widths, List<Widget> cells) => Row(
    children: [
      for (var i = 0; i < cells.length; i++) ...[
        SizedBox(
          width: widths[i],
          child: Padding(
            padding: EdgeInsets.symmetric(
              horizontal: i == 0 || i == 2 ? 8 : 12,
            ),
            child: cells[i],
          ),
        ),
        if (i < cells.length - 1) const SizedBox(width: _dividerWidth),
      ],
    ],
  );

  Widget _record(NetworkRecord record, List<double> widths, bool selected) {
    final colors = context.palette;
    final secondary = selected ? colors.onSelectedRowMuted : colors.muted;
    final uri = Uri.tryParse(record.url);
    final path = uri == null || uri.host.isEmpty
        ? (record.request.isEmpty ? context.l10n.missingRequest : record.url)
        : '${uri.path.isEmpty ? '/' : uri.path}${uri.hasQuery ? '?${uri.query}' : ''}';
    final duration = record.duration == null ? '—' : '${record.duration} ms';
    return Material(
      key: ValueKey('network-row-${record.key}'),
      color: selected ? colors.selectedRow : Colors.transparent,
      animationDuration: Duration.zero,
      child: InkWell(
        splashFactory: NoSplash.splashFactory,
        highlightColor: Colors.transparent,
        hoverDuration: Duration.zero,
        onTap: () => widget.onSelected(record),
        child: Stack(
          fit: StackFit.expand,
          children: [
            Container(
              decoration: BoxDecoration(
                border: Border(bottom: BorderSide(color: colors.line)),
              ),
              child: _cells(widths, [
                Align(
                  alignment: Alignment.centerLeft,
                  child: Tooltip(
                    message: record.method,
                    child: StatusChip(
                      record.method,
                      prominent: true,
                      selected: selected,
                      color: colors.methodColor(record.method),
                    ),
                  ),
                ),
                Column(
                  mainAxisAlignment: MainAxisAlignment.center,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      path,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(
                        fontSize: 12,
                        color: selected ? colors.onSelectedRow : null,
                        fontWeight: selected
                            ? FontWeight.w600
                            : FontWeight.normal,
                      ),
                    ),
                    const SizedBox(height: 3),
                    Text(
                      '${uri?.host ?? ''} · ${widget.applicationName(record)} · ${clockTime(record.time)}',
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: TextStyle(fontSize: 10, color: secondary),
                    ),
                  ],
                ),
                Align(
                  alignment: Alignment.centerLeft,
                  child: Tooltip(
                    message: filterLabel(context, record.state),
                    child: StatusChip(
                      filterLabel(context, record.state),
                      prominent: true,
                      selected: selected,
                      color: colors.statusColor(
                        record.status,
                        transportError: record.error.isNotEmpty,
                      ),
                    ),
                  ),
                ),
                Tooltip(
                  message: duration,
                  child: Text(
                    duration,
                    textAlign: TextAlign.right,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(fontSize: 11, color: secondary),
                  ),
                ),
              ]),
            ),
            if (selected)
              Positioned(
                left: 0,
                top: 0,
                bottom: 0,
                width: 4,
                child: IgnorePointer(
                  child: ColoredBox(color: colors.selectedRowMarker),
                ),
              ),
          ],
        ),
      ),
    );
  }

  Widget _divider(int index, double left, double total, String column) {
    final active = _hovered == index || _dragging == index;
    void step(double delta) {
      _resize(index, delta, total);
      _commit();
    }

    return Positioned(
      left: left,
      top: 0,
      bottom: 0,
      width: _dividerWidth,
      child: Semantics(
        label: context.l10n.resizeNetworkColumn(column),
        onIncrease: () => step(16),
        onDecrease: () => step(-16),
        child: MouseRegion(
          cursor: SystemMouseCursors.resizeColumn,
          onEnter: (_) => setState(() => _hovered = index),
          onExit: (_) => setState(() => _hovered = null),
          child: GestureDetector(
            key: ValueKey('network-column-divider-$index'),
            behavior: HitTestBehavior.opaque,
            onHorizontalDragStart: (_) => setState(() => _dragging = index),
            onHorizontalDragUpdate: (details) =>
                _resize(index, details.delta.dx, total),
            onHorizontalDragEnd: (_) => _commit(),
            onHorizontalDragCancel: _commit,
            child: Center(
              child: Container(
                width: active ? 3 : 1,
                color: active
                    ? context.palette.accent
                    : context.palette.line.withValues(alpha: .7),
              ),
            ),
          ),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    if (!NativeLists.available) return _flutterTable(context);
    final colors = context.palette;
    final locale = Localizations.localeOf(context);
    return NativeCaptureList<NetworkRecord>(
      kind: 'network',
      records: widget.records,
      visible: widget.visible,
      keyOf: (r) => r.key,
      versionOf: (r) => (
        r.requestEvent,
        r.completionEvent,
        widget.applicationName(r),
        colors.dark,
        locale,
      ),
      rowOf: (r) {
        final uri = Uri.tryParse(r.url);
        final path = uri == null || uri.host.isEmpty
            ? (r.request.isEmpty ? context.l10n.missingRequest : r.url)
            : '${uri.path.isEmpty ? '/' : uri.path}${uri.hasQuery ? '?${uri.query}' : ''}';
        return {
          'method': r.method,
          'path': path,
          'meta':
              '${uri?.host ?? ''} · ${widget.applicationName(r)} · ${clockTime(r.time)}',
          'status': filterLabel(context, r.state),
          'duration': r.duration == null ? '—' : '${r.duration} ms',
          'methodColor': colors.methodColor(r.method).toARGB32(),
          'statusColor': colors
              .statusColor(r.status, transportError: r.error.isNotEmpty)
              .toARGB32(),
        };
      },
      selection: widget.selection ?? _localSelection,
      selectedKey: () => widget.selection?.value ?? widget.selectedKey,
      onSelected: widget.onSelected,
      titles: [
        context.l10n.methodColumn,
        context.l10n.urlColumn,
        context.l10n.statusColumn,
        context.l10n.timeColumn,
      ],
      widths: widget.initialWidths,
      onWidthsChanged: (widths) {
        _widths = List.of(widths);
        widget.onWidthsChanged(widths);
      },
      onScroll: _rememberNativeScroll,
      fallback: _flutterTable,
    );
  }

  Widget _flutterTable(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final minimum =
          _widths.reduce((a, b) => a + b) + _minUrlWidth + 3 * _dividerWidth;
      final total = math.max(constraints.maxWidth, minimum);
      final widths = _columns(total);
      final titles = [
        context.l10n.methodColumn,
        context.l10n.urlColumn,
        context.l10n.statusColumn,
        context.l10n.timeColumn,
      ];
      return ScrollConfiguration(
        behavior: ScrollConfiguration.of(context).copyWith(scrollbars: false),
        child: Scrollbar(
          controller: _vertical,
          notificationPredicate: (notification) =>
              notification.metrics.axis == Axis.vertical,
          child: Scrollbar(
            controller: _horizontal,
            thumbVisibility: total > constraints.maxWidth,
            notificationPredicate: (notification) =>
                notification.metrics.axis == Axis.horizontal,
            child: SingleChildScrollView(
              controller: _horizontal,
              scrollDirection: Axis.horizontal,
              child: SizedBox(
                width: total,
                height: constraints.maxHeight,
                child: Stack(
                  children: [
                    Column(
                      children: [
                        Container(
                          height: 30,
                          color: context.palette.surface,
                          child: _cells(widths, [
                            for (var i = 0; i < titles.length; i++)
                              Text(
                                titles[i],
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                textAlign: i == 3
                                    ? TextAlign.right
                                    : TextAlign.left,
                                style: TextStyle(
                                  fontSize: 9,
                                  color: context.palette.muted,
                                  letterSpacing: i < 2 ? 1 : null,
                                ),
                              ),
                          ]),
                        ),
                        Expanded(
                          child: ListView.builder(
                            controller: _vertical,
                            itemCount: widget.records.length,
                            itemExtent: 52,
                            itemBuilder: (context, index) {
                              final record = widget.records[index];
                              final selection = widget.selection;
                              return selection == null
                                  ? _record(
                                      record,
                                      widths,
                                      widget.selectedKey == record.key,
                                    )
                                  : SelectionBuilder<String>(
                                      key: ValueKey(record.key),
                                      selection: selection,
                                      value: record.key,
                                      builder: (context, selected) =>
                                          _record(record, widths, selected),
                                    );
                            },
                          ),
                        ),
                      ],
                    ),
                    for (var i = 0; i < 3; i++)
                      _divider(
                        i,
                        widths.take(i + 1).reduce((a, b) => a + b) +
                            i * _dividerWidth,
                        total,
                        titles[i],
                      ),
                  ],
                ),
              ),
            ),
          ),
        ),
      );
    },
  );
}
