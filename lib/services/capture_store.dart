import 'dart:collection';
import 'dart:convert';

import 'package:flutter/foundation.dart';

import '../models/capture.dart';

/// Bounded live capture. IDs are scoped to the SDK session, never to a socket.
class CaptureStore extends ChangeNotifier {
  CaptureStore({this.maxRecords = 5000, this.maxBytes = 64 * 1024 * 1024});
  final int maxRecords;
  final int maxBytes;
  final sessions = <String, DeviceSession>{};
  final network = <String, NetworkRecord>{};
  final logs = Queue<ConsoleRecord>();
  final _retention = Queue<({String key, bool log, int bytes})>();
  final _recordBytes = <String, int>{};
  int _bytes = 0;
  int _nextLog = 0;
  int rejected = 0;
  int ignoredWhilePaused = 0;
  int evicted = 0;
  bool paused = false;
  int revision = 0;

  /// Returns the validated session ID, allowing the server to track presence.
  String ingest(dynamic payload) {
    if (payload is! Map) {
      throw const FormatException('Expected an SDK envelope object.');
    }
    final envelope = objectOf(payload);
    final metadata = objectOf(envelope['session']);
    final sdk = objectOf(envelope['sdk']);
    final events = envelope['events'];
    if (metadata['id'] is! String ||
        (metadata['id'] as String).isEmpty ||
        events is! List) {
      throw const FormatException(
        'Envelope must contain session.id and events[].',
      );
    }
    if (sdk['schemaVersion'] != 1) {
      throw const FormatException('Unsupported SDK schemaVersion; expected 1.');
    }
    final id = metadata['id'] as String;
    sessions[id] = DeviceSession(metadata, sdk);
    // Retain only a bounded session history; active data stays attributable.
    while (sessions.length > 100) {
      final unused = sessions.keys
          .where(
            (key) =>
                key != id &&
                !network.values.any((r) => r.sessionId == key) &&
                !logs.any((r) => r.sessionId == key),
          )
          .firstOrNull;
      if (unused == null) break;
      sessions.remove(unused);
    }
    for (final value in events) {
      if (paused) {
        ignoredWhilePaused++;
        continue;
      }
      if (value is! Map) {
        rejected++;
        continue;
      }
      final event = objectOf(value);
      if ([
        'js.console',
        'dart.stdout',
        'dart.stderr',
        'dart.logging',
      ].contains(event['source'])) {
        final record = ConsoleRecord(_nextLog++, id, event);
        final bytes = utf8.encode(jsonEncode(event)).length;
        logs.add(record);
        _retention.add((key: '${record.id}', log: true, bytes: bytes));
        _bytes += bytes;
      } else if (event['requestId'] is String &&
          ['request', 'response', 'error', 'abort'].contains(event['phase']) &&
          (event['durationMs'] == null || event['durationMs'] is num) &&
          (objectOf(event['response'])['statusCode'] == null ||
              objectOf(event['response'])['statusCode'] is num)) {
        final key = jsonEncode([id, event['requestId']]);
        final record = network.putIfAbsent(key, () {
          _retention.add((key: key, log: false, bytes: 0));
          return NetworkRecord(id, event['requestId'] as String);
        });
        _bytes -= _recordBytes[key] ?? 0;
        record.apply(event);
        final bytes = utf8.encode(jsonEncode(record.toJson())).length;
        _recordBytes[key] = bytes;
        _bytes += bytes;
      } else {
        rejected++;
      }
      _trim();
    }
    notifyListeners();
    return id;
  }

  void _trim() {
    while (_retention.isNotEmpty &&
        (_retention.length > maxRecords || _bytes > maxBytes)) {
      final oldest = _retention.removeFirst();
      if (oldest.log) {
        logs.removeFirst();
        _bytes -= oldest.bytes;
      } else {
        network.remove(oldest.key);
        _bytes -= _recordBytes.remove(oldest.key) ?? 0;
      }
      evicted++;
    }
  }

  void togglePause() {
    paused = !paused;
    notifyListeners();
  }

  void clear() {
    revision++;
    network.clear();
    logs.clear();
    _retention.clear();
    _recordBytes.clear();
    _bytes = 0;
    rejected = ignoredWhilePaused = evicted = 0;
    notifyListeners();
  }

  Json export() => {
    'format': 'flutter_network_viewer',
    'version': 1,
    'exportedAt': DateTime.now().toUtc().toIso8601String(),
    'sessions': sessions.values
        .map((s) => {'session': s.metadata, 'sdk': s.sdk})
        .toList(),
    'network': network.values.map((r) => r.toJson()).toList(),
    'console': logs.map((r) => r.toJson()).toList(),
    'statistics': {
      'rejected': rejected,
      'evicted': evicted,
      'ignoredWhilePaused': ignoredWhilePaused,
    },
  };
}
