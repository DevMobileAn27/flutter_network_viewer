import 'dart:convert';
import 'json_document.dart';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../models/capture.dart';
import 'viewer_theme.dart';
import 'viewer_notification.dart';
import '../l10n/strings.dart';
import 'json_code_view.dart';
import 'json_tree_view.dart';
import 'json_find_bar.dart';
import 'json_find_controller.dart';
import 'body_color.dart';

export 'viewer_theme.dart';

const mono = TextStyle(
  fontFamily: 'JetBrains Mono',
  fontSize: 12,
  height: 1.65,
);

String clockTime(DateTime? date) => date == null
    ? '—'
    : '${date.toLocal().hour.toString().padLeft(2, '0')}:${date.toLocal().minute.toString().padLeft(2, '0')}:${date.toLocal().second.toString().padLeft(2, '0')}.${date.millisecond.toString().padLeft(3, '0')}';

Future<void> copyText(
  BuildContext context,
  String text, [
  String? message,
]) async {
  await Clipboard.setData(ClipboardData(text: text));
  if (context.mounted) {
    showViewerNotification(
      context,
      type: ViewerNotificationType.success,
      description: message ?? context.l10n.copied,
    );
  }
}

class StatusChip extends StatelessWidget {
  const StatusChip(
    this.text, {
    super.key,
    this.color,
    this.prominent = false,
    this.selected = false,
    this.leading,
  });
  final Widget? leading;
  final String text;
  final Color? color;
  final bool prominent;
  final bool selected;
  @override
  Widget build(BuildContext context) {
    final label = Text(
      text,
      maxLines: 1,
      overflow: TextOverflow.ellipsis,
      style: TextStyle(
        color: prominent
            ? (context.palette.dark ? const Color(0xff101828) : Colors.white)
            : color ?? context.palette.accent,
        fontSize: 11,
        fontWeight: prominent ? FontWeight.w700 : FontWeight.w600,
      ),
    );
    return Container(
      padding: EdgeInsets.symmetric(horizontal: 8, vertical: 4),
      decoration: BoxDecoration(
        color: (color ?? context.palette.accent).withValues(
          alpha: prominent ? 1 : .1,
        ),
        borderRadius: BorderRadius.circular(5),
        border: prominent && selected
            ? Border.all(color: Colors.white.withValues(alpha: .8))
            : null,
      ),
      child: leading == null
          ? label
          : Row(
              mainAxisSize: MainAxisSize.min,
              children: [leading!, const SizedBox(width: 6), label],
            ),
    );
  }
}

class EmptyState extends StatelessWidget {
  const EmptyState({
    super.key,
    required this.icon,
    required this.title,
    this.action,
  });
  final IconData icon;
  final String title;
  final Widget? action;
  @override
  Widget build(BuildContext context) => Center(
    child: SingleChildScrollView(
      child: Padding(
        padding: EdgeInsets.all(28),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              padding: EdgeInsets.all(20),
              decoration: BoxDecoration(
                color: context.palette.accent.withValues(alpha: .06),
                shape: BoxShape.circle,
              ),
              child: Icon(icon, size: 32, color: context.palette.accent),
            ),
            SizedBox(height: 20),
            Text(
              title,
              textAlign: TextAlign.center,
              style: TextStyle(fontSize: 18, fontWeight: FontWeight.w500),
            ),
            if (action != null) ...[SizedBox(height: 22), action!],
          ],
        ),
      ),
    ),
  );
}

class DataSection extends StatefulWidget {
  const DataSection(
    this.title,
    this.value, {
    super.key,
    this.body = false,
    this.initiallyExpanded = true,
    this.preview,
    this.searchable = false,
    this.fillAvailable = false,
    this.showTitle = true,
  });
  final String title;
  final dynamic value;
  final bool body;
  final bool initiallyExpanded;
  final Widget? preview;
  final bool searchable;
  final bool fillAvailable;
  final bool showTitle;

  @override
  State<DataSection> createState() => _DataSectionState();
}

class _DataSectionState extends State<DataSection> {
  bool _tree = false;
  late dynamic _decoded;
  late bool _isJson;
  JsonDocument? _prepared;
  final _find = JsonFindController();
  final _findFocus = FocusNode();
  bool get _canFind =>
      widget.searchable && widget.body && _isJson && widget.preview == null;

  @override
  void dispose() {
    _find.dispose();
    _findFocus.dispose();
    super.dispose();
  }

  @override
  void initState() {
    super.initState();
    _decode();
  }

  @override
  void didUpdateWidget(DataSection oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.value != oldWidget.value ||
        widget.body != oldWidget.body ||
        widget.searchable != oldWidget.searchable) {
      _decode();
    }
  }

  void _decode() {
    _prepared = null;
    _decoded = widget.value;
    _isJson = _decoded is Map || _decoded is List;
    if (widget.body && widget.value is String) {
      try {
        _prepared = JsonDocumentCache.shared.decode(widget.value as String);
        _decoded = _prepared!.value;
        _isJson = true;
      } on FormatException {
        /* plain text */
      }
    }
    if (_isJson) _prepared ??= JsonDocument.fromValue(_decoded);
    if (widget.searchable) _find.setValue(_decoded, prepared: _prepared);
  }

  Widget _content(BuildContext context, {double? viewportHeight}) {
    final value = widget.value;
    if (value == null || !widget.body && value is Map && value.isEmpty) {
      return Text(
        context.l10n.notCaptured,
        style: TextStyle(
          color: context.palette.muted,
          fontStyle: FontStyle.italic,
        ),
      );
    }
    if (widget.preview != null) return widget.preview!;
    if (widget.body && _isJson) {
      return _tree
          ? JsonTreeView(
              value: _decoded,
              initiallyExpanded: widget.initiallyExpanded,
              find: _canFind ? _find : null,
            )
          : JsonCodeView(
              value: _decoded,
              find: _canFind ? _find : null,
              viewportHeight: viewportHeight,
              prepared: _prepared,
            );
    }
    if (_decoded is Map || _decoded is List) {
      return JsonTree(
        value: _decoded,
        initiallyExpanded: widget.initiallyExpanded,
      );
    }
    final color = widget.body && value is String ? parseBodyColor(value) : null;
    if (color != null) {
      return Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Expanded(child: SelectableText('$value', style: mono)),
          Padding(
            padding: const EdgeInsets.only(top: 4, left: 8),
            child: BodyColorSwatch(value: value as String, color: color),
          ),
        ],
      );
    }
    return SelectableText('$value', style: mono);
  }

  @override
  Widget build(BuildContext context) {
    final title = widget.title;
    final value = widget.value;
    final section = Padding(
      padding: EdgeInsets.fromLTRB(18, widget.showTitle ? 8 : 4, 18, 4),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          if (_canFind && widget.showTitle)
            Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: Text(
                title.toUpperCase(),
                style: TextStyle(
                  fontSize: 10,
                  fontWeight: FontWeight.w700,
                  letterSpacing: 1.2,
                  color: context.palette.muted,
                ),
              ),
            ),
          LayoutBuilder(
            builder: (context, constraints) {
              final width = _canFind && constraints.maxWidth < 600
                  ? 600.0
                  : constraints.maxWidth;
              return SingleChildScrollView(
                key: const ValueKey('json-body-toolbar'),
                scrollDirection: Axis.horizontal,
                child: SizedBox(
                  width: width,
                  child: Row(
                    children: [
                      Expanded(
                        child: _canFind
                            ? Align(
                                alignment: Alignment.centerRight,
                                child: JsonFindBar(
                                  controller: _find,
                                  focusNode: _findFocus,
                                ),
                              )
                            : widget.showTitle
                            ? Text(
                                title.toUpperCase(),
                                style: TextStyle(
                                  fontSize: 10,
                                  fontWeight: FontWeight.w700,
                                  letterSpacing: 1.2,
                                  color: context.palette.muted,
                                ),
                              )
                            : const SizedBox.shrink(),
                      ),
                      if (_canFind) const SizedBox(width: 8),
                      if (widget.body && _isJson) ...[
                        SizedBox(
                          width: 128,
                          height: 30,
                          child: DefaultTabController(
                            length: 2,
                            initialIndex: _tree ? 1 : 0,
                            child: Material(
                              color: context.palette.input,
                              borderRadius: BorderRadius.circular(30),
                              clipBehavior: Clip.antiAlias,
                              child: Padding(
                                padding: const EdgeInsets.all(3),
                                child: TabBar(
                                  splashBorderRadius: BorderRadius.circular(30),
                                  onTap: (index) =>
                                      setState(() => _tree = index == 1),
                                  dividerColor: Colors.transparent,
                                  indicatorSize: TabBarIndicatorSize.tab,
                                  indicator: BoxDecoration(
                                    color: context.palette.surface,
                                    borderRadius: BorderRadius.circular(30),
                                    border: Border.all(
                                      color: context.palette.line,
                                    ),
                                  ),
                                  labelColor: context.palette.accent,
                                  unselectedLabelColor: context.palette.muted,
                                  labelStyle: const TextStyle(
                                    fontSize: 11,
                                    fontWeight: FontWeight.w600,
                                  ),
                                  unselectedLabelStyle: const TextStyle(
                                    fontSize: 11,
                                  ),
                                  labelPadding: EdgeInsets.zero,
                                  tabs: [
                                    Tooltip(
                                      message: context.l10n.formattedJson,
                                      child: const Tab(text: 'JSON'),
                                    ),
                                    Tooltip(
                                      message: context.l10n.treeView,
                                      child: Tab(text: context.l10n.treeTab),
                                    ),
                                  ],
                                ),
                              ),
                            ),
                          ),
                        ),
                        const SizedBox(width: 4),
                      ],
                      if (value != null)
                        IconButton(
                          tooltip: context.l10n.copySection(title),
                          iconSize: 15,
                          visualDensity: VisualDensity.compact,
                          onPressed: () => copyText(context, pretty(value)),
                          icon: Icon(Icons.copy_outlined),
                        ),
                    ],
                  ),
                ),
              );
            },
          ),
          SizedBox(height: 6),
          if (widget.fillAvailable)
            Expanded(
              child: LayoutBuilder(
                builder: (context, constraints) => SingleChildScrollView(
                  key: const ValueKey('json-body-scroll'),
                  child: ConstrainedBox(
                    constraints: BoxConstraints(
                      minHeight: constraints.maxHeight,
                    ),
                    child: _content(
                      context,
                      viewportHeight: constraints.maxHeight,
                    ),
                  ),
                ),
              ),
            )
          else
            _content(context),
        ],
      ),
    );
    return _canFind
        ? CallbackShortcuts(
            bindings: {
              const SingleActivator(LogicalKeyboardKey.keyF, meta: true):
                  _findFocus.requestFocus,
              const SingleActivator(LogicalKeyboardKey.keyF, control: true):
                  _findFocus.requestFocus,
              const SingleActivator(LogicalKeyboardKey.f3): () => _find.move(1),
              const SingleActivator(LogicalKeyboardKey.f3, shift: true): () =>
                  _find.move(-1),
            },
            child: Focus(child: section),
          )
        : section;
  }
}

/// Expand objects lazily: large structured console arguments stay navigable.
class JsonTree extends StatelessWidget {
  const JsonTree({
    super.key,
    required this.value,
    this.name,
    this.depth = 0,
    this.initiallyExpanded,
  });
  final dynamic value;
  final String? name;
  final int depth;
  final bool? initiallyExpanded;
  @override
  Widget build(BuildContext context) {
    final label = name == null ? '' : '$name: ';
    if (value is Map || value is List) {
      final entries = value is Map
          ? (value as Map).entries.toList()
          : (value as List).asMap().entries.toList();
      final bracket = value is Map ? '{…}' : '[…]';
      if (entries.isEmpty) {
        return SelectableText(
          '$label${value is Map ? '{}' : '[]'}',
          style: mono,
        );
      }
      return ExpansionTile(
        key: ValueKey('$depth:$name'),
        initiallyExpanded: initiallyExpanded ?? depth < 1,
        tilePadding: EdgeInsets.zero,
        childrenPadding: EdgeInsets.only(left: 14),
        dense: true,
        visualDensity: VisualDensity.compact,
        shape: Border(),
        collapsedShape: Border(),
        title: Text(
          '$label$bracket  ${context.l10n.itemCount(entries.length)}',
          style: mono.copyWith(color: context.palette.accent),
        ),
        children: entries
            .map(
              (e) => Align(
                alignment: Alignment.centerLeft,
                child: JsonTree(
                  value: e.value,
                  name: '${e.key}',
                  depth: depth + 1,
                ),
              ),
            )
            .toList(),
      );
    }
    return Padding(
      padding: EdgeInsets.symmetric(vertical: 2),
      child: SelectableText(
        '$label${jsonEncode(value)}',
        style: mono.copyWith(
          color: value is String
              ? context.palette.string
              : context.palette.warning,
        ),
      ),
    );
  }
}
