import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

/// Presentation metadata only. File bytes stay in the captured Raw body; the
/// parsed model never duplicates attachments or reads files from disk/network.
class FormBodyPart {
  const FormBodyPart({
    required this.name,
    required this.byteLength,
    this.value,
    this.filename,
    this.contentType,
  });
  final String name;
  final String? value, filename, contentType;
  final int byteLength;
  bool get isFile => filename != null;
}

class FormBody {
  const FormBody(this.parts, {this.incomplete = false});
  final List<FormBodyPart> parts;
  final bool incomplete;
}

String bodyContentType(dynamic headers) {
  if (headers is! Map) return '';
  for (final entry in headers.entries) {
    if ('${entry.key}'.toLowerCase() == 'content-type') {
      final value = entry.value;
      return value is List ? value.join('; ') : '$value';
    }
  }
  return '';
}

bool isFormBody(String contentType) {
  final mime = contentType.split(';').first.trim().toLowerCase();
  return mime == 'multipart/form-data' ||
      mime == 'application/x-www-form-urlencoded';
}

/// This entry point can be sent to compute() to keep binary parsing off the UI
/// isolate. Multipart delimiters are matched on bytes, never decoded as text.
FormBody parseFormBody(Map<String, Object?> args) {
  final raw = args['body'];
  if (raw is! String) return const FormBody([], incomplete: true);
  try {
    final type = ContentType.parse(args['contentType'] as String);
    final bytes = args['encoding'] == 'base64'
        ? base64Decode(raw.replaceAll(RegExp(r'\s'), ''))
        : Uint8List.fromList(utf8.encode(raw));
    if (type.mimeType.toLowerCase() == 'application/x-www-form-urlencoded') {
      final parts = <FormBodyPart>[];
      for (final item in utf8.decode(bytes).split('&')) {
        if (item.isEmpty) continue;
        if (parts.length >= 1000) return FormBody(parts, incomplete: true);
        final index = item.indexOf('=');
        final name = Uri.decodeQueryComponent(
          index < 0 ? item : item.substring(0, index),
        );
        final value = Uri.decodeQueryComponent(
          index < 0 ? '' : item.substring(index + 1),
        );
        parts.add(
          FormBodyPart(
            name: name,
            value: value,
            byteLength: utf8.encode(value).length,
          ),
        );
      }
      return FormBody(parts);
    }
    final boundary = type.parameters['boundary'];
    if (boundary == null ||
        boundary.isEmpty ||
        boundary.length > 200 ||
        boundary.contains('\r') ||
        boundary.contains('\n')) {
      return const FormBody([], incomplete: true);
    }
    final delimiter = utf8.encode('--$boundary');
    bool starts(int at, List<int> pattern) {
      if (at < 0 || at + pattern.length > bytes.length) return false;
      for (var i = 0; i < pattern.length; i++) {
        if (bytes[at + i] != pattern[i]) return false;
      }
      return true;
    }

    int nextBoundary(int from) {
      for (var i = from; i <= bytes.length - delimiter.length; i++) {
        if (i > 0 && !(i > 1 && bytes[i - 2] == 13 && bytes[i - 1] == 10)) {
          continue;
        }
        if (!starts(i, delimiter)) continue;
        final end = i + delimiter.length;
        final closing =
            starts(end, [45, 45]) &&
            (end + 2 == bytes.length || starts(end + 2, [13, 10]));
        if (starts(end, [13, 10]) || closing) return i;
      }
      return -1;
    }

    final parts = <FormBodyPart>[];
    var position = nextBoundary(0);
    var incomplete = false;
    while (position >= 0) {
      final after = position + delimiter.length;
      if (starts(after, [45, 45])) {
        return FormBody(parts, incomplete: incomplete);
      }
      final headerStart = after + 2;
      var headerEnd = -1;
      // Do not scan arbitrary attachment bytes looking for a corrupt header.
      for (
        var i = headerStart;
        i < bytes.length - 3 && i - headerStart < 65536;
        i++
      ) {
        if (starts(i, [13, 10, 13, 10])) {
          headerEnd = i;
          break;
        }
      }
      if (headerEnd < 0 || parts.length >= 1000) {
        return FormBody(parts, incomplete: true);
      }
      final headers = <String, String>{};
      for (final line
          in utf8
              .decode(
                Uint8List.sublistView(bytes, headerStart, headerEnd),
                allowMalformed: true,
              )
              .split('\r\n')) {
        final colon = line.indexOf(':');
        if (colon > 0) {
          headers[line.substring(0, colon).trim().toLowerCase()] = line
              .substring(colon + 1)
              .trim();
        }
      }
      final start = headerEnd + 4;
      final next = nextBoundary(start);
      final end = next < 0 ? bytes.length : next - 2;
      if (end < start) return FormBody(parts, incomplete: true);
      final disposition = _parameters(headers['content-disposition'] ?? '');
      final name = disposition['name'];
      if (name == null) {
        incomplete = true;
      } else {
        final data = Uint8List.sublistView(bytes, start, end);
        final filename =
            _extended(disposition['filename*']) ?? disposition['filename'];
        final charset = _parameters(
          headers['content-type'] ?? '',
        )['charset']?.toLowerCase();
        final value = filename != null
            ? null
            : charset == 'iso-8859-1' || charset == 'latin1'
            ? latin1.decode(data)
            : utf8.decode(data, allowMalformed: true);
        parts.add(
          FormBodyPart(
            name: name,
            value: value,
            filename: filename,
            contentType: headers['content-type'],
            byteLength: data.length,
          ),
        );
      }
      if (next < 0) return FormBody(parts, incomplete: true);
      position = next;
    }
    return FormBody(parts, incomplete: true);
  } on FormatException {
    return const FormBody([], incomplete: true);
  } on ArgumentError {
    // Uri.decodeQueryComponent reports malformed percent escapes this way.
    return const FormBody([], incomplete: true);
  }
}

Map<String, String> _parameters(String value) {
  final result = <String, String>{};
  final expression = RegExp(
    r'''(?:^|;)\s*([^;=\s]+)\s*=\s*(?:"((?:\\.|[^"\\])*)"|([^;]*))''',
  );
  for (final match in expression.allMatches(value)) {
    result[match[1]!.toLowerCase()] =
        match[2]?.replaceAllMapped(RegExp(r'\\(.)'), (m) => m[1]!) ??
        match[3]!.trim();
  }
  return result;
}

String? _extended(String? value) {
  if (value == null) return null;
  final first = value.indexOf("'");
  final second = value.indexOf("'", first + 1);
  if (first < 0 ||
      second < 0 ||
      value.substring(0, first).toLowerCase() != 'utf-8') {
    return null;
  }
  try {
    return Uri.decodeComponent(value.substring(second + 1));
  } on FormatException {
    return null;
  } on ArgumentError {
    return null;
  }
}
