import 'dart:convert';

import 'package:flutter/material.dart';

import '../l10n/strings.dart';
import 'components.dart';
import 'json_find_controller.dart';

/// A selectable JSON document; the gutter is kept out of copied text.
class JsonCodeView extends StatefulWidget {
  const JsonCodeView({super.key, required this.value, this.find});
  final dynamic value;
  final JsonFindController? find;

  @override
  State<JsonCodeView> createState() => _JsonCodeViewState();
}

class _JsonCodeViewState extends State<JsonCodeView> {
  final _scroll = ScrollController();
  late String _document;
  late List<String> _lines;
  final _ranges = <int, int>{};
  final _collapsed = <int>{};
  final _matchAnchor = GlobalKey();
  final _lineOffsets = <int>[];

  @override
  void initState() {
    super.initState();
    _format();
    widget.find?.addListener(_searchChanged);
    _revealMatch();
  }

  @override
  void didUpdateWidget(JsonCodeView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.value != oldWidget.value) {
      _format();
      _revealMatch();
    }
    if (widget.find != oldWidget.find) {
      oldWidget.find?.removeListener(_searchChanged);
      widget.find?.addListener(_searchChanged);
      _revealMatch();
    }
  }

  void _format() {
    _document = const JsonEncoder.withIndent('  ').convert(widget.value);
    _lines = _document.split('\n');
    _lineOffsets.clear();
    var offset = 0;
    for (final text in _lines) {
      _lineOffsets.add(offset);
      offset += text.length + 1;
    }
    _ranges.clear();
    _collapsed.clear();
    final openings = <int>[];
    var line = 0;
    var inString = false;
    var escaped = false;
    for (final character in _document.codeUnits) {
      if (character == 10) line++;
      if (inString) {
        if (escaped) {
          escaped = false;
        } else if (character == 92) {
          escaped = true;
        } else if (character == 34) {
          inString = false;
        }
      } else if (character == 34) {
        inString = true;
      } else if (character == 123 || character == 91) {
        openings.add(line);
      } else if (character == 125 || character == 93) {
        final start = openings.removeLast();
        if (start < line) _ranges[start] = line;
      }
    }
  }

  @override
  void dispose() {
    widget.find?.removeListener(_searchChanged);
    _scroll.dispose();
    super.dispose();
  }

  void _searchChanged() {
    setState(_revealMatch);
  }

  void _revealMatch() {
    final match = widget.find?.active;
    if (match == null) return;
    _collapsed.removeWhere(
      (line) =>
          match.start <=
              _lineOffsets[_ranges[line]!] + _lines[_ranges[line]!].length &&
          match.end > _lineOffsets[line],
    );
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
  Widget build(BuildContext context) {
    final colors = context.palette;
    final codeStyle = DefaultTextStyle.of(context).style.merge(mono);
    final visibleLines = <int>[];
    final visibleText = <String>[];
    for (var line = 0; line < _lines.length; line++) {
      visibleLines.add(line);
      final end = _ranges[line];
      if (end != null && _collapsed.contains(line)) {
        visibleText.add('${_lines[line]} … ${_lines[end].trimLeft()}');
        line = end;
      } else {
        visibleText.add(_lines[line]);
      }
    }
    final document = visibleText.join('\n');
    final tokens = RegExp(
      r'"(?:\\.|[^"\\])*"|true|false|null|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?',
    );
    final spans = <TextSpan>[];
    int? activeOffset;
    var renderedOffset = 0;
    for (var i = 0; i < visibleLines.length; i++) {
      final line = visibleLines[i];
      final text = _lines[line];
      final base = _lineOffsets[line];
      final active = widget.find?.active;
      if (active != null &&
          active.start >= base &&
          active.start <= base + text.length) {
        activeOffset = renderedOffset + active.start - base;
      }
      void add(String text, int offset, [TextStyle? style]) {
        spans.addAll(
          widget.find?.spans(text, base + offset, style: style) ??
              [TextSpan(text: text, style: style)],
        );
      }

      var offset = 0;
      for (final match in tokens.allMatches(text)) {
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
        offset = match.end;
      }
      add(text.substring(offset), offset);
      if (visibleText[i].length > text.length) {
        spans.add(TextSpan(text: visibleText[i].substring(text.length)));
      }
      if (i < visibleLines.length - 1) spans.add(const TextSpan(text: '\n'));
      renderedOffset += visibleText[i].length + 1;
    }
    final strut = StrutStyle.fromTextStyle(codeStyle, forceStrutHeight: true);
    final metrics = TextPainter(
      text: TextSpan(text: document, style: codeStyle),
      strutStyle: strut,
      textDirection: Directionality.of(context),
      textScaler: MediaQuery.textScalerOf(context),
    )..layout();
    final lineMetrics = metrics.computeLineMetrics();
    final documentHeight = metrics.height;
    Rect? matchRect;
    if (activeOffset != null && activeOffset < document.length) {
      final boxes = metrics.getBoxesForSelection(
        TextSelection(baseOffset: activeOffset, extentOffset: activeOffset + 1),
      );
      if (boxes.isNotEmpty) matchRect = boxes.first.toRect();
    }
    metrics.text = TextSpan(text: '${_lines.length}', style: codeStyle);
    metrics.layout();
    final numberWidth = metrics.width.ceilToDouble();
    metrics.dispose();
    return Container(
      width: double.infinity,
      decoration: BoxDecoration(
        color: colors.surface,
        border: Border.all(color: colors.line),
        borderRadius: BorderRadius.circular(6),
      ),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(0, 10, 0, 6),
        child: Scrollbar(
          controller: _scroll,
          thumbVisibility: true,
          notificationPredicate: (notification) =>
              notification.metrics.axis == Axis.horizontal,
          child: SingleChildScrollView(
            controller: _scroll,
            scrollDirection: Axis.horizontal,
            padding: const EdgeInsets.only(right: 12, bottom: 8),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Container(
                  margin: const EdgeInsets.only(right: 12),
                  padding: const EdgeInsets.symmetric(horizontal: 12),
                  decoration: BoxDecoration(
                    border: Border(right: BorderSide(color: colors.line)),
                  ),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      ExcludeSemantics(
                        child: SizedBox(
                          width: numberWidth,
                          child: Text(
                            visibleLines
                                .map((line) => '${line + 1}')
                                .join('\n'),
                            key: const ValueKey('json-line-numbers'),
                            softWrap: false,
                            textAlign: TextAlign.right,
                            strutStyle: strut,
                            style: codeStyle.copyWith(color: colors.muted),
                          ),
                        ),
                      ),
                      const SizedBox(width: 4),
                      SizedBox(
                        width: 20,
                        height: documentHeight,
                        child: Stack(
                          children: [
                            for (
                              var index = 0;
                              index < visibleLines.length;
                              index++
                            )
                              if (_ranges.containsKey(visibleLines[index]))
                                Positioned(
                                  top:
                                      lineMetrics[index].baseline -
                                      lineMetrics[index].ascent,
                                  height: lineMetrics[index].height,
                                  left: 0,
                                  right: 0,
                                  child: _foldButton(visibleLines[index]),
                                ),
                          ],
                        ),
                      ),
                    ],
                  ),
                ),
                Stack(
                  children: [
                    SelectableText.rich(
                      TextSpan(children: spans),
                      strutStyle: strut,
                      style: codeStyle.copyWith(
                        color: Theme.of(context).colorScheme.onSurface,
                      ),
                    ),
                    if (matchRect != null)
                      Positioned(
                        left: matchRect.left,
                        top: matchRect.top,
                        child: IgnorePointer(
                          child: SizedBox(
                            key: _matchAnchor,
                            width: matchRect.width,
                            height: matchRect.height,
                          ),
                        ),
                      ),
                  ],
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Widget _foldButton(int line) {
    final collapsed = _collapsed.contains(line);
    return Tooltip(
      message: collapsed
          ? context.l10n.expandJsonRange
          : context.l10n.collapseJsonRange,
      child: Semantics(
        button: true,
        expanded: !collapsed,
        child: Material(
          color: Colors.transparent,
          child: InkWell(
            key: ValueKey('json-fold-$line'),
            borderRadius: BorderRadius.circular(3),
            onTap: () => setState(() {
              if (collapsed) {
                _collapsed.remove(line);
              } else {
                _collapsed.add(line);
              }
            }),
            child: Icon(
              collapsed
                  ? Icons.chevron_right_rounded
                  : Icons.keyboard_arrow_down_rounded,
              size: 15,
              color: context.palette.muted,
            ),
          ),
        ),
      ),
    );
  }
}
