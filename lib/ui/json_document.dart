import 'dart:convert';

class JsonFindNode {
  JsonFindNode(this.path, this.keyStart, this.valueStart);
  final List<Object> path;
  final int? keyStart;
  final int valueStart;
  int end = 0;
}

/// Immutable captured JSON and lazily prepared presentation data shared by views.
class JsonDocument {
  JsonDocument.fromValue(this.value);
  final dynamic value;
  late final String text = const JsonEncoder.withIndent('  ').convert(value);
  late final List<String> lines = List.unmodifiable(text.split('\n'));
  late final List<int> offsets = _offsets();
  late final Map<int, int> ranges = _ranges();
  late final Map<String, JsonFindNode> nodes = _nodes();
  late final int columns = lines.fold<int>(0, (width, line) {
    final cells = line.runes.fold<int>(0, (n, c) => n + (c < 128 ? 1 : 2));
    return cells > width ? cells : width;
  });

  List<int> _offsets() {
    var offset = 0;
    final result = <int>[];
    for (final line in lines) {
      result.add(offset);
      offset += line.length + 1;
    }
    return List.unmodifiable(result);
  }

  Map<int, int> _ranges() {
    final ranges = <int, int>{};
    final openings = <int>[];
    var line = 0;
    var inString = false;
    var escaped = false;
    for (final character in text.codeUnits) {
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
        if (start < line) ranges[start] = line;
      }
    }
    return Map.unmodifiable(ranges);
  }

  Map<String, JsonFindNode> _nodes() {
    final buffer = StringBuffer();
    final nodes = <String, JsonFindNode>{};
    void write(dynamic value, List<Object> path, int depth, int? keyStart) {
      final node = JsonFindNode(path, keyStart, buffer.length);
      nodes[jsonEncode(path)] = node;
      if (value is Map || value is List) {
        final entries = value is Map
            ? value.entries.toList()
            : (value as List).asMap().entries.toList();
        buffer.write(value is Map ? '{' : '[');
        for (var i = 0; i < entries.length; i++) {
          final entry = entries[i];
          buffer.write('\n${'  ' * (depth + 1)}');
          final start = value is Map ? buffer.length : null;
          if (value is Map) buffer.write('${jsonEncode(entry.key)}: ');
          write(entry.value, [...path, entry.key as Object], depth + 1, start);
          if (i < entries.length - 1) buffer.write(',');
        }
        if (entries.isNotEmpty) buffer.write('\n${'  ' * depth}');
        buffer.write(value is Map ? '}' : ']');
      } else {
        buffer.write(jsonEncode(value));
      }
      node.end = buffer.length;
    }

    write(value, [], 0, null);
    return Map.unmodifiable(nodes);
  }
}

/// Keys are immutable captured strings, never mutable Maps or request IDs.
/// The conservative estimate includes decoded objects, indexes and text copies.
class JsonDocumentCache {
  JsonDocumentCache({this.maxBytes = 32 * 1024 * 1024, this.maxEntries = 24});
  static final shared = JsonDocumentCache();
  final int maxBytes;
  final int maxEntries;
  final _entries = <String, JsonDocument>{};
  int estimatedBytes = 0;
  int get length => _entries.length;
  void clear() {
    _entries.clear();
    estimatedBytes = 0;
  }

  int _cost(String source) => 4096 + source.length * 64;

  JsonDocument decode(String source) {
    final cached = _entries.remove(source);
    if (cached != null) {
      _entries[source] = cached;
      return cached;
    }
    final result = JsonDocument.fromValue(jsonDecode(source));
    final cost = _cost(source);
    if (cost <= maxBytes && maxEntries > 0) {
      while (_entries.isNotEmpty &&
          (estimatedBytes + cost > maxBytes || _entries.length >= maxEntries)) {
        final key = _entries.keys.first;
        _entries.remove(key);
        estimatedBytes -= _cost(key);
      }
      _entries[source] = result;
      estimatedBytes += cost;
    }
    return result;
  }
}
