import 'dart:convert';

import '../models/capture.dart';

/// Converts the public dart:io HTTP profiling schema into retained viewer events.
class FlutterCaptureAdapter {
  static String timestamp(dynamic micros) => micros is num
      ? DateTime.fromMicrosecondsSinceEpoch(
          micros.toInt(),
          isUtc: true,
        ).toIso8601String()
      : DateTime.now().toUtc().toIso8601String();

  static Json headers(dynamic raw) => objectOf(raw).map(
    (name, value) =>
        MapEntry(name, value is List ? value.join(', ') : '$value'),
  );

  static Json body(dynamic raw) {
    if (raw == null) return {};
    if (raw is String) return {'body': raw};
    if (raw is! List) return {};
    final bytes = raw.cast<int>();
    try {
      return {
        'body': utf8.decode(bytes),
        'bodyEncoding': 'utf-8',
        'bodyByteLength': bytes.length,
      };
    } on FormatException {
      return {
        'body': base64Encode(bytes),
        'bodyEncoding': 'base64',
        'bodyByteLength': bytes.length,
      };
    }
  }

  static List<Json> network(Json profile) {
    final request = objectOf(profile['request']);
    final response = objectOf(profile['response']);
    final error = request['error'] ?? response['error'];
    final end =
        response['endTime'] ?? (error != null ? profile['endTime'] : null);
    final start = profile['startTime'];
    final common = <String, dynamic>{
      'requestId': '${profile['id']}',
      'source': 'dart.io.http',
    };
    return [
      {
        ...common,
        'phase': 'request',
        'timestamp': timestamp(start),
        'request': {
          ...request,
          'method': profile['method'],
          'url': profile['uri'],
          'headers': headers(request['headers']),
          ...body(profile['requestBody']),
        },
        'profileEvents': profile['events'],
      },
      if (end != null || error != null)
        {
          ...common,
          'phase': error == null ? 'response' : 'error',
          'timestamp': timestamp(end),
          if (start is num && end is num) 'durationMs': (end - start) / 1000,
          'response': {
            ...response,
            'headers': headers(response['headers']),
            'statusText': response['reasonPhrase'],
            ...body(profile['responseBody']),
          },
          if (error != null) 'error': {'message': '$error'},
        },
    ];
  }
}
