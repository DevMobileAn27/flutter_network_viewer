import 'dart:convert';

import 'package:flutter/material.dart';
import 'json_document.dart';

/// Search offsets refer to the formatted document, shared by JSON and Tree.
class JsonFindMatch {
  const JsonFindMatch(this.start, this.end, this.path);
  final int start;
  final int end;
  final List<Object> path;
}

class JsonFindController extends ChangeNotifier {
  JsonDocument? _prepared;
  String get document => _prepared?.text ?? '';
  Map<String, JsonFindNode> get nodes => _prepared?.nodes ?? const {};
  String query = '';
  bool matchCase = false;
  bool wholeWord = false;
  bool regex = false;
  String? error;
  List<JsonFindMatch> matches = [];
  int index = -1;
  JsonFindMatch? get active => index < 0 ? null : matches[index];

  void setValue(dynamic value, {JsonDocument? prepared}) {
    _prepared = prepared ?? JsonDocument.fromValue(value);
    _search();
  }

  void update({String? query, bool? matchCase, bool? wholeWord, bool? regex}) {
    this.query = query ?? this.query;
    this.matchCase = matchCase ?? this.matchCase;
    this.wholeWord = wholeWord ?? this.wholeWord;
    this.regex = regex ?? this.regex;
    _search();
  }

  void _search() {
    error = null;
    matches = [];
    index = -1;
    if (query.isNotEmpty) {
      try {
        final expression = RegExp(
          regex ? query : RegExp.escape(query),
          caseSensitive: matchCase,
          unicode: true,
          multiLine: true,
        );
        final word = RegExp(r'[\p{L}\p{N}_]', unicode: true);
        final ranges = expression.allMatches(document).where((match) {
          if (match.start == match.end) return false;
          if (!wholeWord) return true;
          return (match.start == 0 ||
                  !word.hasMatch(
                    document.substring(match.start - 1, match.start),
                  )) &&
              (match.end == document.length ||
                  !word.hasMatch(document.substring(match.end, match.end + 1)));
        });
        // Nodes are ordered by position. Advancing once avoids scanning the
        // entire tree separately for every match in a large response.
        final ordered = nodes.values.toList()
          ..sort(
            (a, b) => (a.keyStart ?? a.valueStart).compareTo(
              b.keyStart ?? b.valueStart,
            ),
          );
        var cursor = 0;
        for (final match in ranges) {
          while (cursor + 1 < ordered.length &&
              (ordered[cursor + 1].keyStart ??
                      ordered[cursor + 1].valueStart) <=
                  match.start) {
            cursor++;
          }
          var node = ordered[cursor];
          while (node.path.isNotEmpty && match.start >= node.end) {
            node =
                nodes[jsonEncode(node.path.sublist(0, node.path.length - 1))]!;
          }
          matches.add(JsonFindMatch(match.start, match.end, node.path));
        }
        if (matches.isNotEmpty) index = 0;
      } on FormatException catch (exception) {
        error = exception.message;
      }
    }
    notifyListeners();
  }

  void move(int delta) {
    if (matches.isEmpty) return;
    index = (index + delta) % matches.length;
    notifyListeners();
  }

  /// Split syntax tokens at match boundaries, preserving selectable text.
  List<TextSpan> spans(String text, int offset, {TextStyle? style}) {
    final end = offset + text.length;
    final spans = <TextSpan>[];
    var cursor = offset;
    // Binary search skips matches preceding this token.
    var low = 0;
    var high = matches.length;
    while (low < high) {
      final mid = (low + high) ~/ 2;
      if (matches[mid].end <= offset) {
        low = mid + 1;
      } else {
        high = mid;
      }
    }
    for (var i = low; i < matches.length && matches[i].start < end; i++) {
      final match = matches[i];
      final start = match.start.clamp(offset, end);
      final stop = match.end.clamp(offset, end);
      if (start > cursor) {
        spans.add(
          TextSpan(
            text: text.substring(cursor - offset, start - offset),
            style: style,
          ),
        );
      }
      spans.add(
        TextSpan(
          text: text.substring(start - offset, stop - offset),
          style: (style ?? const TextStyle()).copyWith(
            backgroundColor: i == index
                ? const Color(0xfffb923c)
                : const Color(0xfffde68a),
            color: const Color(0xff172033),
          ),
        ),
      );
      cursor = stop;
    }
    if (cursor < end) {
      spans.add(TextSpan(text: text.substring(cursor - offset), style: style));
    }
    return spans;
  }
}
