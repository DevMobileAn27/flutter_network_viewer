import 'dart:convert';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../l10n/strings.dart';
import 'body_color.dart';
import 'components.dart';
import 'json_find_controller.dart';

/// Large documents keep all data but only shape/layout rows near the viewport.
class VirtualJsonCodeView extends StatefulWidget {
  const VirtualJsonCodeView({
    super.key,
    required this.document,
    required this.lines,
    required this.offsets,
    required this.ranges,
    required this.columns,
    this.find,
  });
  final String document;
  final List<String> lines;
  final List<int> offsets;
  final Map<int, int> ranges;
  final int columns;
  final JsonFindController? find;

  @override
  State<VirtualJsonCodeView> createState() => _VirtualJsonCodeViewState();
}

class _VirtualJsonCodeViewState extends State<VirtualJsonCodeView> {
  final _vertical = ScrollController();
  final _horizontal = ScrollController();
  final _selection = GlobalKey<SelectionAreaState>();
  final _collapsed = <int>{};
  List<int> _visible = [];
  int _columns = 0;
  double _lineHeight = 20;
  double _gutter = 70;
  bool _allSelected = false;
  bool _keepSelection = false;
  String? _selectedText;
  static final _tokens = RegExp(
    r'"(?:\\.|[^"\\])*"|true|false|null|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?',
  );

  @override
  void initState() {
    super.initState();
    _prepare();
    widget.find?.addListener(_searchChanged);
    _reveal();
  }

  @override
  void didUpdateWidget(VirtualJsonCodeView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.find != widget.find) {
      oldWidget.find?.removeListener(_searchChanged);
      widget.find?.addListener(_searchChanged);
    }
    if (oldWidget.document != widget.document) {
      _selection.currentState?.selectableRegion.clearSelection();
      _collapsed.clear();
      _allSelected = false;
      _keepSelection = false;
      _selectedText = null;
      _prepare();
      if (_vertical.hasClients) _vertical.jumpTo(0);
    }
    if (oldWidget.document != widget.document ||
        oldWidget.find != widget.find) {
      _reveal();
    }
  }

  void _prepare() {
    _columns = widget.columns;
    _updateVisible();
  }

  void _updateVisible() {
    _visible = [];
    for (var line = 0; line < widget.lines.length; line++) {
      _visible.add(line);
      if (_collapsed.contains(line)) line = widget.ranges[line]!;
    }
  }

  void _searchChanged() => setState(_reveal);

  void _reveal() {
    final active = widget.find?.active;
    if (active == null) return;
    _collapsed.removeWhere(
      (line) =>
          active.start <
              widget.offsets[widget.ranges[line]!] +
                  widget.lines[widget.ranges[line]!].length &&
          active.end > widget.offsets[line],
    );
    _updateVisible();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || !_vertical.hasClients) return;
      var low = 0;
      var high = widget.offsets.length;
      while (low < high) {
        final mid = (low + high) ~/ 2;
        if (widget.offsets[mid] <= active.start) {
          low = mid + 1;
        } else {
          high = mid;
        }
      }
      final line = math.max(0, low - 1);
      final row = _visible.indexOf(line);
      final position = _vertical.position;
      _vertical.jumpTo(
        (row * _lineHeight - position.viewportDimension * .35).clamp(
          0.0,
          position.maxScrollExtent,
        ),
      );
      if (_horizontal.hasClients) {
        final text = widget.lines[line];
        final offset = (active.start - widget.offsets[line]).clamp(
          0,
          text.length,
        );
        final painter = TextPainter(
          text: TextSpan(
            text: text.substring(0, offset),
            style: DefaultTextStyle.of(context).style.merge(mono),
          ),
          textDirection: Directionality.of(context),
          textScaler: MediaQuery.textScalerOf(context),
        )..layout();
        final x = _gutter + painter.width;
        painter.dispose();
        _horizontal.jumpTo(
          (x - _horizontal.position.viewportDimension * .35).clamp(
            0.0,
            _horizontal.position.maxScrollExtent,
          ),
        );
      }
    });
  }

  void _selectAll() {
    _selection.currentState?.selectableRegion.selectAll(
      SelectionChangedCause.keyboard,
    );
    _allSelected = true;
  }

  void _copy() {
    if (_allSelected) {
      Clipboard.setData(ClipboardData(text: widget.document));
      _selection.currentState?.selectableRegion.hideToolbar();
    } else {
      if (_selectedText != null) {
        Clipboard.setData(ClipboardData(text: _selectedText!));
      }
      _selection.currentState?.selectableRegion.hideToolbar();
    }
  }

  @override
  void dispose() {
    widget.find?.removeListener(_searchChanged);
    _vertical.dispose();
    _horizontal.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colors = context.palette;
    final style = DefaultTextStyle.of(context).style.merge(mono);
    final strut = StrutStyle.fromTextStyle(style, forceStrutHeight: true);
    final metrics = TextPainter(
      text: TextSpan(text: '0', style: style),
      strutStyle: strut,
      textDirection: Directionality.of(context),
      textScaler: MediaQuery.textScalerOf(context),
    )..layout();
    final characterWidth = metrics.width;
    _lineHeight = metrics.height;
    _gutter = '${widget.lines.length}'.length * characterWidth + 56;
    metrics.dispose();
    return Container(
      key: const ValueKey('virtual-json'),
      decoration: BoxDecoration(
        color: colors.surface,
        border: Border.all(color: colors.line),
        borderRadius: BorderRadius.circular(6),
      ),
      clipBehavior: Clip.hardEdge,
      padding: const EdgeInsets.only(top: 10, bottom: 6),
      child: Scrollbar(
        controller: _vertical,
        notificationPredicate: (n) => n.metrics.axis == Axis.vertical,
        child: LayoutBuilder(
          builder: (context, constraints) => Scrollbar(
            controller: _horizontal,
            thumbVisibility: true,
            notificationPredicate: (n) => n.metrics.axis == Axis.horizontal,
            child: SingleChildScrollView(
              controller: _horizontal,
              scrollDirection: Axis.horizontal,
              child: SizedBox(
                width: math.max(
                  constraints.maxWidth,
                  _gutter + _columns * characterWidth + 48,
                ),
                height: constraints.maxHeight,
                child: Actions(
                  actions: {
                    SelectAllTextIntent: CallbackAction<SelectAllTextIntent>(
                      onInvoke: (_) {
                        _selectAll();
                        return null;
                      },
                    ),
                    CopySelectionTextIntent:
                        CallbackAction<CopySelectionTextIntent>(
                          onInvoke: (_) {
                            _copy();
                            return null;
                          },
                        ),
                  },
                  child: Listener(
                    onPointerDown: (event) {
                      if (event.buttons == 1) _allSelected = false;
                    },
                    child: Focus(
                      onKeyEvent: (_, event) {
                        if (event is KeyDownEvent &&
                            const [
                              LogicalKeyboardKey.escape,
                              LogicalKeyboardKey.arrowLeft,
                              LogicalKeyboardKey.arrowRight,
                              LogicalKeyboardKey.arrowUp,
                              LogicalKeyboardKey.arrowDown,
                            ].contains(event.logicalKey)) {
                          _allSelected = false;
                        }
                        return KeyEventResult.ignored;
                      },
                      child: SelectionArea(
                        key: _selection,
                        onSelectionChanged: (selection) {
                          _selectedText = selection?.plainText;
                          final keep =
                              selection != null &&
                              selection.plainText.isNotEmpty;
                          if (_keepSelection == keep) return;
                          _keepSelection = keep;
                          WidgetsBinding.instance.addPostFrameCallback((_) {
                            if (mounted) setState(() {});
                          });
                        },
                        contextMenuBuilder: (context, region) =>
                            AdaptiveTextSelectionToolbar.buttonItems(
                              anchors: region.contextMenuAnchors,
                              buttonItems: [
                                for (final item
                                    in region.contextMenuButtonItems)
                                  ContextMenuButtonItem(
                                    type: item.type,
                                    label: item.label,
                                    onPressed:
                                        item.type ==
                                            ContextMenuButtonType.selectAll
                                        ? _selectAll
                                        : item.type ==
                                              ContextMenuButtonType.copy
                                        ? _copy
                                        : item.onPressed,
                                  ),
                              ],
                            ),
                        child: ListView.builder(
                          key: const ValueKey('virtual-json-lines'),
                          controller: _vertical,
                          primary: false,
                          itemExtent: _lineHeight,
                          // Flutter 3.41 has no scrollCacheExtent yet. Retain the
                          // small cache while supporting both old and new SDKs.
                          // ignore: deprecated_member_use
                          cacheExtent: 80,
                          itemCount: _visible.length,
                          addAutomaticKeepAlives: false,
                          addRepaintBoundaries: false,
                          addSemanticIndexes: false,
                          itemBuilder: (context, index) => KeepAlive(
                            // Preserve selected rows when a drag scrolls them offscreen.
                            keepAlive: _keepSelection,
                            child: RepaintBoundary(
                              child: IndexedSemantics(
                                index: index,
                                child: _row(_visible[index], style, strut),
                              ),
                            ),
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _row(int line, TextStyle style, StrutStyle strut) {
    final colors = context.palette;
    final text = widget.lines[line];
    final spans = <TextSpan>[];
    final swatches = <Widget>[];
    void add(String value, int offset, [TextStyle? tokenStyle]) => spans.addAll(
      widget.find?.spans(
            value,
            widget.offsets[line] + offset,
            style: tokenStyle,
          ) ??
          [TextSpan(text: value, style: tokenStyle)],
    );
    var offset = 0;
    for (final match in _tokens.allMatches(text)) {
      if (match.start > offset) {
        add(text.substring(offset, match.start), offset);
      }
      final token = match.group(0)!;
      final isKey =
          token.startsWith('"') &&
          match.end < text.length &&
          text[match.end] == ':';
      final color = token.startsWith('"')
          ? (isKey ? colors.info : colors.string)
          : token == 'null'
          ? colors.muted
          : token == 'true' || token == 'false'
          ? colors.method
          : colors.warning;
      add(token, match.start, TextStyle(color: color));
      if (!isKey && token.startsWith('"')) {
        final value = jsonDecode(token) as String;
        final color = parseBodyColor(value);
        if (color != null) {
          swatches.add(
            SelectionContainer.disabled(
              child: Padding(
                padding: const EdgeInsets.only(left: 8),
                child: BodyColorSwatch(value: value, color: color),
              ),
            ),
          );
        }
      }
      offset = match.end;
    }
    add(text.substring(offset), offset);
    if (_collapsed.contains(line)) {
      spans.add(
        TextSpan(text: ' … ${widget.lines[widget.ranges[line]!].trimLeft()}'),
      );
    }
    return Row(
      key: ValueKey('virtual-json-row-$line'),
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        SelectionContainer.disabled(
          child: Container(
            width: _gutter - 12,
            margin: const EdgeInsets.only(right: 12),
            decoration: BoxDecoration(
              border: Border(right: BorderSide(color: colors.line)),
            ),
            child: Row(
              children: [
                Expanded(
                  child: ExcludeSemantics(
                    child: Text(
                      '${line + 1}',
                      textAlign: TextAlign.right,
                      style: style.copyWith(color: colors.muted),
                      strutStyle: strut,
                    ),
                  ),
                ),
                const SizedBox(width: 4),
                SizedBox(
                  width: 24,
                  height: _lineHeight,
                  child: widget.ranges.containsKey(line)
                      ? Tooltip(
                          message: _collapsed.contains(line)
                              ? context.l10n.expandJsonRange
                              : context.l10n.collapseJsonRange,
                          child: Semantics(
                            button: true,
                            expanded: !_collapsed.contains(line),
                            child: InkWell(
                              key: ValueKey('json-fold-$line'),
                              onTap: () => setState(() {
                                _allSelected = false;
                                if (!_collapsed.add(line)) {
                                  _collapsed.remove(line);
                                }
                                _updateVisible();
                              }),
                              child: Icon(
                                _collapsed.contains(line)
                                    ? Icons.chevron_right_rounded
                                    : Icons.keyboard_arrow_down_rounded,
                                size: 15,
                                color: colors.muted,
                              ),
                            ),
                          ),
                        )
                      : null,
                ),
              ],
            ),
          ),
        ),
        Text.rich(
          TextSpan(children: spans),
          style: style,
          strutStyle: strut,
          softWrap: false,
        ),
        ...swatches,
      ],
    );
  }
}
