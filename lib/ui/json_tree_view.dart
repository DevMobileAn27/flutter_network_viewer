import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import '../l10n/strings.dart';
import 'components.dart';
import 'json_find_controller.dart';

/// Terminal-style branches, with children built only for expanded nodes.
class JsonTreeView extends StatefulWidget {
  const JsonTreeView({
    super.key,
    required this.value,
    this.initiallyExpanded = true,
    this.find,
  });
  final dynamic value;
  final bool initiallyExpanded;
  final JsonFindController? find;

  @override
  State<JsonTreeView> createState() => _JsonTreeViewState();
}

class _JsonTreeViewState extends State<JsonTreeView> {
  final _scroll = ScrollController();
  final _expanded = <String>{};
  final _matchAnchor = GlobalKey();

  @override
  void initState() {
    super.initState();
    if (widget.initiallyExpanded) _expanded.add('[]');
    widget.find?.addListener(_searchChanged);
    _revealMatch();
  }

  @override
  void didUpdateWidget(JsonTreeView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.value != widget.value) {
      _expanded.clear();
      if (widget.initiallyExpanded) _expanded.add('[]');
      _revealMatch();
    }
    if (oldWidget.find != widget.find) {
      oldWidget.find?.removeListener(_searchChanged);
      widget.find?.addListener(_searchChanged);
      _revealMatch();
    }
  }

  void _searchChanged() => setState(_revealMatch);

  void _revealMatch() {
    final match = widget.find?.active;
    if (match == null) return;
    for (var i = 0; i < match.path.length; i++) {
      _expanded.add(jsonEncode(match.path.sublist(0, i)));
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      final target = _matchAnchor.currentContext;
      if (target != null) {
        Scrollable.ensureVisible(
          target,
          alignment: .35,
          duration: const Duration(milliseconds: 150),
        );
      }
    });
  }

  @override
  void dispose() {
    widget.find?.removeListener(_searchChanged);
    _scroll.dispose();
    super.dispose();
  }

  List<Widget> _nodes(
    dynamic value,
    List<Object> path,
    List<bool> ancestors,
    double characterWidth, {
    String? name,
    bool last = true,
  }) {
    final colors = context.palette;
    final collection = value is Map || value is List;
    final count = collection ? value.length as int : 0;
    final expandable = collection && count > 0;
    final id = jsonEncode(path);
    final node = widget.find?.nodes[id];
    final active = widget.find?.active;
    final selected = active != null && jsonEncode(active.path) == id;
    final open = _expanded.contains(id);
    final prefix = name == null
        ? ''
        : '${ancestors.map((continues) => continues ? '│   ' : '    ').join()}${last ? '└── ' : '├── '}';
    // Escape control characters in keys to keep each branch on one line.
    final encodedName = name == null ? null : jsonEncode(name);
    final label = encodedName == null
        ? ''
        : '${encodedName.substring(1, encodedName.length - 1)}: ';
    final bracket = value is Map ? '{}' : '[]';
    final text = Text.rich(
      TextSpan(
        children: [
          TextSpan(
            text: prefix,
            // Paint continuous connectors across the full row height below.
            style: const TextStyle(color: Colors.transparent),
          ),
          TextSpan(
            text: expandable ? (open ? '▾ ' : '▸ ') : '  ',
            style: TextStyle(color: colors.accent),
          ),
          if (label.isNotEmpty)
            TextSpan(
              children: node?.keyStart == null
                  ? [TextSpan(text: label)]
                  : [
                      ...widget.find!.spans(
                        label.substring(0, label.length - 2),
                        node!.keyStart! + 1,
                      ),
                      const TextSpan(text: ': '),
                    ],
              style: TextStyle(
                color: colors.info,
                fontWeight: collection ? FontWeight.w600 : FontWeight.normal,
              ),
            ),
          TextSpan(
            children: node == null
                ? [TextSpan(text: collection ? bracket : jsonEncode(value))]
                : collection
                ? [
                    ...widget.find!.spans(bracket[0], node.valueStart),
                    ...widget.find!.spans(bracket[1], node.end - 1),
                  ]
                : widget.find!.spans(jsonEncode(value), node.valueStart),
            style: TextStyle(
              color: collection
                  ? colors.accent
                  : value is String
                  ? colors.string
                  : value is bool
                  ? colors.method
                  : value == null
                  ? colors.muted
                  : colors.warning,
            ),
          ),
          if (collection && count > 0)
            TextSpan(
              text: '  ${context.l10n.itemCount(count)}',
              style: TextStyle(color: colors.muted),
            ),
        ],
      ),
      style: mono,
      softWrap: false,
    );
    final row = CustomPaint(
      painter: name == null
          ? null
          : _TreeBranches(
              ancestors: ancestors,
              last: last,
              characterWidth: characterWidth,
              color: colors.muted,
            ),
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 3, horizontal: 2),
        child: text,
      ),
    );
    final result = <Widget>[
      if (expandable)
        Semantics(
          expanded: open,
          child: InkWell(
            key: ValueKey('tree-node:$id'),
            borderRadius: BorderRadius.circular(4),
            onTap: () => setState(() {
              if (open) {
                _expanded.remove(id);
              } else {
                _expanded.add(id);
              }
            }),
            child: row,
          ),
        )
      else
        row,
    ];
    if (selected) {
      result[0] = Container(
        key: _matchAnchor,
        decoration: BoxDecoration(
          color: colors.accent.withValues(alpha: .08),
          borderRadius: BorderRadius.circular(4),
        ),
        child: result[0],
      );
    }
    if (expandable && open) {
      final entries = value is Map
          ? value.entries.toList()
          : (value as List).asMap().entries.toList();
      for (var index = 0; index < entries.length; index++) {
        final entry = entries[index];
        result.addAll(
          _nodes(
            entry.value,
            [...path, entry.key as Object],
            name == null ? [] : [...ancestors, !last],
            characterWidth,
            name: value is List ? '[${entry.key}]' : '${entry.key}',
            last: index == entries.length - 1,
          ),
        );
      }
    }
    return result;
  }

  @override
  Widget build(BuildContext context) {
    final metrics = TextPainter(
      text: const TextSpan(text: ' ', style: mono),
      textDirection: Directionality.of(context),
      textScaler: MediaQuery.textScalerOf(context),
    )..layout();
    final characterWidth = metrics.width;
    metrics.dispose();
    return SizedBox(
      width: double.infinity,
      child: Material(
        color: context.palette.surface,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(6),
          side: BorderSide(color: context.palette.line),
        ),
        clipBehavior: Clip.antiAlias,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(10, 8, 10, 6),
          child: Scrollbar(
            controller: _scroll,
            thumbVisibility: true,
            notificationPredicate: (notification) =>
                notification.metrics.axis == Axis.horizontal,
            child: SingleChildScrollView(
              controller: _scroll,
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsets.only(bottom: 8),
              child: SelectionArea(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: _nodes(widget.value, [], [], characterWidth),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _TreeBranches extends CustomPainter {
  const _TreeBranches({
    required this.ancestors,
    required this.last,
    required this.characterWidth,
    required this.color,
  });
  final List<bool> ancestors;
  final bool last;
  final double characterWidth;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    final pen = Paint()
      ..color = color
      ..strokeWidth = 1;
    double xAt(int column) => 2 + (column * 4 + .5) * characterWidth;
    for (var column = 0; column < ancestors.length; column++) {
      if (ancestors[column]) {
        final x = xAt(column);
        canvas.drawLine(Offset(x, 0), Offset(x, size.height), pen);
      }
    }
    final x = xAt(ancestors.length);
    final middle = size.height / 2;
    canvas.drawLine(Offset(x, 0), Offset(x, last ? middle : size.height), pen);
    canvas.drawLine(
      Offset(x, middle),
      Offset(x + 2.5 * characterWidth, middle),
      pen,
    );
  }

  @override
  bool shouldRepaint(_TreeBranches oldDelegate) =>
      last != oldDelegate.last ||
      characterWidth != oldDelegate.characterWidth ||
      color != oldDelegate.color ||
      !listEquals(ancestors, oldDelegate.ancestors);
}
