import 'dart:convert';

typedef Json = Map<String, dynamic>;

Json objectOf(dynamic value) => value is Map
    ? value.map((key, value) => MapEntry(key.toString(), value))
    : <String, dynamic>{};

String pretty(dynamic value) {
  if (value is String) {
    try {
      return const JsonEncoder.withIndent('  ').convert(jsonDecode(value));
    } on FormatException {
      return value;
    }
  }
  return const JsonEncoder.withIndent('  ').convert(value);
}

class DeviceSession {
  DeviceSession(this.metadata, this.sdk);
  final Json metadata;
  final Json sdk;
  String get id => metadata['id'] as String;
  String get name => '${metadata['appName'] ?? 'Flutter'}';
  String get device => '${metadata['deviceName'] ?? 'Unknown device'}';
  String get platform => '${metadata['platform'] ?? 'unknown'}';
}

class NetworkRecord {
  NetworkRecord(this.sessionId, this.requestId);
  final String sessionId;
  final String requestId;
  Json requestEvent = {};
  Json completionEvent = {};

  String get key => jsonEncode([sessionId, requestId]);
  Json get request => objectOf(requestEvent['request']);
  Json get response => objectOf(completionEvent['response']);
  Json get error => objectOf(completionEvent['error']);
  String get method => '${request['method'] ?? '—'}';
  String get url => '${request['url'] ?? '(request not received)'}';
  String get source =>
      '${requestEvent['source'] ?? completionEvent['source'] ?? ''}';
  int? get status => (response['statusCode'] as num?)?.toInt();
  num? get duration => completionEvent['durationMs'] as num?;
  bool get failed => error.isNotEmpty || (status ?? 0) >= 400;
  bool get pending => completionEvent.isEmpty;
  String get state => error.isNotEmpty
      ? (completionEvent['phase'] == 'abort' ? 'Aborted' : 'Error')
      : status?.toString() ??
            (pending ? 'Pending' : '${completionEvent['phase']}');
  DateTime? get time => DateTime.tryParse(
    '${requestEvent['timestamp'] ?? completionEvent['timestamp']}',
  );
  int get responseBytes =>
      (response['bodyByteLength'] as num?)?.toInt() ??
      utf8
          .encode('${response['body'] ?? response['bodyPreview'] ?? ''}')
          .length;

  void apply(Json event) {
    if (event['phase'] == 'request') {
      requestEvent = event;
    } else {
      completionEvent = event;
    }
  }

  Json toJson() => {
    'sessionId': sessionId,
    'requestId': requestId,
    'requestEvent': requestEvent,
    'completionEvent': completionEvent,
  };

  String get curl {
    String quote(String value) => "'${value.replaceAll("'", "'\\''")}'";
    final parts = ['curl', '-X', method, quote(url)];
    objectOf(request['headers']).forEach((name, value) {
      parts.addAll(['-H', quote('$name: $value')]);
    });
    if (request['bodyEncoding'] == 'base64') {
      parts.addAll(['--data-binary', '@-']);
      return "printf %s ${quote('${request['body']}')} | base64 --decode | ${parts.join(' ')}";
    }
    if (request['body'] != null) {
      parts.addAll(['--data-raw', quote('${request['body']}')]);
    }
    return parts.join(' ');
  }
}

class ConsoleRecord {
  ConsoleRecord(this.id, this.sessionId, this.event);
  final int id;
  final String sessionId;
  final Json event;
  String get level => '${event['level'] ?? 'log'}';
  String get message => '${event['message'] ?? ''}';
  String get displayMessage => cleanMessage(message);
  String get source => '${event['source'] ?? ''}';
  String get sourceLabel => switch (source) {
    'dart.stdout' => 'stdout',
    'dart.stderr' => 'stderr',
    'dart.logging' => 'dart:developer',
    _ => source,
  };
  String get loggerName => '${event['loggerName'] ?? ''}';
  String? get error => _detail('error');
  String? get stackTrace => _detail('stackTrace');
  String? _detail(String name) {
    final value = event[name];
    if (value != null && '$value' != 'null' && '$value' != '') return '$value';
    for (final arg in event['args'] is List ? event['args'] as List : []) {
      final data = objectOf(arg);
      if (data['type'] == name) return '${data['value']}';
    }
    return null;
  }

  int get lineCount => displayMessage.split('\n').length;
  String get searchText =>
      '$displayMessage $sourceLabel $loggerName ${error ?? ''} ${stackTrace ?? ''} ${jsonEncode(event['args'] ?? [])}';

  // Keep the original event for export; terminal formatting is presentation only.
  static String cleanMessage(String value) => value
      .replaceAll(RegExp(r'\x1B\[[0-?]*[ -/]*[@-~]'), '')
      .split('\n')
      .map(
        (line) =>
            line.replaceFirst(RegExp(r'^flutter: ?'), '').replaceAll('\r', ''),
      )
      .join('\n');

  /// Reassemble only recognizable JSON / Flutter exception blocks. Ordinary
  /// consecutive print calls remain separate, and raw records stay untouched.
  static List<ConsoleRecord> grouped(Iterable<ConsoleRecord> records) {
    final input = records.toList();
    final result = <ConsoleRecord>[];
    for (var i = 0; i < input.length; i++) {
      final first = input[i];
      final start = first.displayMessage.trim();
      final json = start == '{' || start == '[';
      final exception = start.startsWith('══╡ EXCEPTION');
      if (!json && !exception ||
          !['dart.stdout', 'dart.stderr'].contains(first.source)) {
        result.add(first);
        continue;
      }
      final block = <ConsoleRecord>[first];
      var text = first.displayMessage;
      var end = i;
      for (var j = i + 1; j < input.length && j < i + 200; j++) {
        final next = input[j];
        if (next.sessionId != first.sessionId ||
            next.source != first.source ||
            next.event['isolateId'] != first.event['isolateId'] ||
            (next.time != null &&
                first.time != null &&
                next.time!.difference(first.time!).inMilliseconds.abs() >
                    2000)) {
          break;
        }
        text += '\n${next.displayMessage}';
        if (text.length > 65536) break;
        block.add(next);
        var complete =
            exception &&
            RegExp(r'^═{8,}$').hasMatch(next.displayMessage.trim());
        if (json && RegExp(r'[}\]]$').hasMatch(next.displayMessage.trim())) {
          try {
            final decoded = jsonDecode(text);
            complete = decoded is Map || decoded is List;
          } on FormatException {
            /* Await the rest of the JSON block. */
          }
        }
        if (complete) {
          end = j;
          break;
        }
      }
      if (end == i) {
        result.add(first);
      } else {
        result.add(
          ConsoleRecord(first.id, first.sessionId, {
            ...first.event,
            'message': text,
            'rawEvents': block.map((r) => r.event).toList(),
            if (exception) 'level': 'error',
          }),
        );
        i = end;
      }
    }
    return result;
  }

  DateTime? get time => DateTime.tryParse('${event['timestamp']}');
  Json toJson() => {'sessionId': sessionId, 'event': event};
}
