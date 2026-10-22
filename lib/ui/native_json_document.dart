import 'dart:convert';

import 'body_color.dart';
import 'json_document.dart';

/// Serializable presentation data built off the UI isolate. Source positions
/// are UTF-16 on Dart, AppKit and Win32, including non-BMP characters.
Map<String, Object?> prepareNativeJson(String source) =>
    prepareNativeJsonValue(jsonDecode(source));

Map<String, Object?> prepareNativeJsonValue(dynamic value) {
  final document = JsonDocument.fromValue(value);
  final code = <Map<String, Object?>>[];
  int columns(String text) =>
      text.runes.fold(0, (n, c) => n + (c < 128 ? 1 : 2));
  final token = RegExp(
    r'"(?:\\.|[^"\\])*"|true|false|null|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?',
  );
  String kind(String text, bool key) => key
      ? 'key'
      : text.startsWith('"')
      ? 'string'
      : text == 'null'
      ? 'muted'
      : text == 'true' || text == 'false'
      ? 'bool'
      : 'number';
  for (var i = 0; i < document.lines.length; i++) {
    final line = document.lines[i];
    final spans = <List<Object>>[];
    int? swatch;
    for (final match in token.allMatches(line)) {
      final text = match.group(0)!;
      final key = match.end < line.length && line[match.end] == ':';
      spans.add([match.start, text.length, kind(text, key)]);
      if (!key && text.startsWith('"')) {
        swatch = parseBodyColor(jsonDecode(text) as String)?.toARGB32();
      }
    }
    code.add({
      'text': line,
      'columns': columns(line),
      'start': document.offsets[i],
      'end': document.offsets[i] + line.length,
      'fold': document.ranges[i] ?? i,
      'depth': 0,
      'spans': spans,
      'maps': [
        [0, line.length, document.offsets[i]],
      ],
      'swatch': ?swatch,
    });
  }
  final tree = <Map<String, Object?>>[];
  void visit(
    dynamic value,
    List<Object> path,
    String? name,
    List<bool> ancestors,
    bool last,
  ) {
    final node = document.nodes[jsonEncode(path)]!;
    final collection = value is Map || value is List;
    final count = value is Map
        ? value.length
        : value is List
        ? value.length
        : 0;
    final prefix = name == null ? '' : '$name: ';
    final body = collection ? (value is Map ? '{}' : '[]') : jsonEncode(value);
    final rowIndex = tree.length;
    final swatch = value is String ? parseBodyColor(value)?.toARGB32() : null;
    final text = '$prefix$body';
    final row = <String, Object?>{
      'text': text,
      'columns': columns(text),
      'start': node.keyStart ?? node.valueStart,
      'end': node.end,
      'fold': rowIndex,
      'depth': path.length,
      'branches': ancestors,
      'last': last,
      if (collection && count > 0) 'count': count,
      'spans': [
        if (prefix.isNotEmpty)
          [0, prefix.length, collection ? 'keyStrong' : 'key'],
        [prefix.length, body.length, collection ? 'accent' : kind(body, false)],
      ],
      'maps': [
        if (node.keyStart != null) [0, name!.length, node.keyStart! + 1],
        [prefix.length, collection ? 1 : body.length, node.valueStart],
        if (collection) [prefix.length + 1, 1, node.end - 1],
      ],
      'swatch': ?swatch,
    };
    tree.add(row);
    final childAncestors = name == null ? <bool>[] : [...ancestors, !last];
    if (value is Map) {
      var index = 0;
      for (final entry in value.entries) {
        final key = jsonEncode(entry.key);
        visit(
          entry.value,
          [...path, entry.key as Object],
          key.substring(1, key.length - 1),
          childAncestors,
          ++index == value.length,
        );
      }
    } else if (value is List) {
      for (var i = 0; i < value.length; i++) {
        visit(
          value[i],
          [...path, i],
          '[$i]',
          childAncestors,
          i == value.length - 1,
        );
      }
    }
    row['fold'] = tree.length - 1;
  }

  visit(document.value, [], null, [], true);
  return {'text': document.text, 'code': code, 'tree': tree};
}
