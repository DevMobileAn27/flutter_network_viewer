import 'dart:async';
import 'dart:convert';
import 'dart:io';

/// Small protocol fixture, independent of the viewer implementation.
class FakeVmService {
  late HttpServer server;
  final sockets = <WebSocket>[];
  final calls = <Map<String, dynamic>>[];
  final profiles = <String, Map<String, dynamic>>{};
  final isolateIds = ['isolates/1'];
  int clock = 2000000;
  bool profilerAvailable = true;
  bool flutterApp = true;
  int vmPid = 10;
  int vmStartTime = 1000;
  String rootLibrary = 'package:shop/main.dart';
  String authToken = 'token=';
  Completer<void>? detailGate;
  String get uri => 'ws://127.0.0.1:${server.port}/$authToken/ws';

  Future<void> start() async {
    server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
    server.listen((request) async {
      if (!WebSocketTransformer.isUpgradeRequest(request)) {
        request.response.write('Dart VM Service');
        await request.response.close();
        return;
      }
      final socket = await WebSocketTransformer.upgrade(request);
      sockets.add(socket);
      socket.listen((raw) async {
        final call = jsonDecode(raw as String) as Map<String, dynamic>;
        calls.add(call);
        final args = call['params'] as Map? ?? {};
        final method = call['method'];
        dynamic result;
        switch (method) {
          case 'invoke':
            result = {
              'type': '@Instance',
              'kind': 'String',
              'valueAsString': 'StateError: missing user',
            };
          case 'getVM':
            result = {
              'type': 'VM',
              'pid': vmPid,
              'startTime': vmStartTime,
              'operatingSystem': 'android',
              'targetCPU': 'arm64',
              'isolates': [
                for (final id in isolateIds)
                  {
                    'type': '@Isolate',
                    'id': id,
                    'name': 'main',
                    'number': id.split('/').last,
                  },
              ],
            };
          case 'getIsolate':
            result = {
              'type': 'Isolate',
              'id': args['isolateId'],
              'name': 'main',
              'number': '1',
              'rootLib': {
                'type': '@Library',
                'id': 'libraries/1',
                'uri': rootLibrary,
                'name': '',
              },
              'extensionRPCs': [
                if (profilerAvailable) 'ext.dart.io.getHttpProfile',
                if (flutterApp) 'ext.flutter.inspector.show',
              ],
            };
          case 'ext.dart.io.getHttpProfile':
            final since = int.tryParse('${args['updatedSince']}') ?? 0;
            result = {
              'type': 'HttpProfile',
              'timestamp': clock,
              'requests': [
                for (final p in profiles.values)
                  if ((p['_updated'] as int? ?? clock) >= since)
                    Map<String, dynamic>.from(p)
                      ..remove('requestBody')
                      ..remove('responseBody')
                      ..remove('_updated'),
              ],
            };
          case 'ext.dart.io.getHttpProfileRequest':
            await detailGate?.future;
            result = Map<String, dynamic>.from(profiles[args['id']]!)
              ..remove('_updated');
          default:
            result = {'type': 'Success'};
        }
        if (socket.readyState != WebSocket.open) return;
        try {
          socket.add(
            jsonEncode({'jsonrpc': '2.0', 'id': call['id'], 'result': result}),
          );
        } on StateError {
          // close() can close the sink before readyState leaves WebSocket.open.
        }
      });
    });
  }

  void event(String stream, Map<String, dynamic> event) {
    for (final socket in sockets) {
      if (socket.readyState != WebSocket.open) continue;
      socket.add(
        jsonEncode({
          'jsonrpc': '2.0',
          'method': 'streamNotify',
          'params': {
            'streamId': stream,
            'event': {
              'type': 'Event',
              'kind': stream == 'Logging' ? 'Logging' : 'WriteEvent',
              'isolate': {
                'type': '@Isolate',
                'id': isolateIds.first,
                'name': 'main',
                'number': '1',
              },
              'timestamp': 1792045056596,
              ...event,
            },
          },
        }),
      );
    }
  }

  Future<void> close() async {
    for (final socket in sockets) {
      await socket.close();
    }
    await server.close(force: true);
  }
}

Map<String, dynamic> profile({String id = 'request-1', bool complete = true}) =>
    {
      'type': 'HttpProfileRequest',
      'id': id,
      'method': 'POST',
      'uri': 'https://example.com/orders?draft=true',
      'startTime': 1000000,
      'endTime': 1010000,
      'request': {
        'headers': {
          'content-type': ['application/json'],
          'x-tags': ['one', 'two'],
        },
      },
      'requestBody': utf8.encode('{"quantity":2}'),
      'response': {
        'startTime': 1015000,
        if (complete) 'endTime': 1124000,
        'statusCode': 201,
        'reasonPhrase': 'Created',
        'headers': {
          'content-type': ['application/json'],
        },
      },
      'responseBody': utf8.encode('{"id":42,"name":"Cà phê"}'),
    };

Future<void> until(bool Function() predicate) async {
  for (var i = 0; i < 200; i++) {
    if (predicate()) return;
    await Future<void>.delayed(const Duration(milliseconds: 10));
  }
  throw StateError('Timed out waiting for capture');
}
