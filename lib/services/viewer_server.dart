import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:vm_service/vm_service.dart';

import '../models/capture.dart';
import 'capture_store.dart';
import 'flutter_capture_adapter.dart';
import 'vm_service_discovery.dart';

/// A DevTools-compatible VM Service client. The historical name is retained so
/// the copied UI and MCP tools share the same capture/presence interface.
class ViewerServer extends ChangeNotifier {
  ViewerServer(
    this.store, {
    this.pollInterval = const Duration(seconds: 1),
    this.retryInterval = const Duration(seconds: 3),
    VmServiceDiscovery? discovery,
  }) : _discovery = discovery ?? VmServiceDiscovery();
  static const defaultServiceUri = String.fromEnvironment('VM_SERVICE_URI');
  final CaptureStore store;
  final Duration pollInterval;
  final Duration retryInterval;
  final VmServiceDiscovery _discovery;
  bool autoDetect = false;
  bool get searching => autoDetect && _wanted && !running;
  List<VmServiceTarget> discoveredApps = const [];
  String? _preferredProject;
  VmService? _service;
  WebSocket? _socket;
  Timer? _timer;
  Timer? _retry;
  final _isolates = <String, Json>{};
  Json? _application;
  final _cursors = <String, int>{};
  final _fingerprints = <String, String>{};
  final _stdoutBuffers = <String, List<int>>{};
  int _stdoutRevision = 0;
  final _subscriptions = <StreamSubscription<dynamic>>[];
  bool _disposed = false;
  bool _wanted = false;
  int _generation = 0;
  bool busy = false;
  String endpoint = defaultServiceUri;
  String? error;
  String? lastProtocolError;
  int invalidMessages = 0;
  bool get running => _service != null;
  int get connections => running ? 1 : 0;
  int get port => Uri.tryParse(endpoint)?.port ?? 0;
  String get localUrl => endpoint;
  bool isOnline(String sessionId) =>
      running && _isolates.values.any((session) => session['id'] == sessionId);

  /// Accept DevTools links (including encoded uri=), HTTP service roots or ws URLs.
  static Uri normalizeUri(String input) {
    var uri = Uri.tryParse(input.trim());
    if (uri == null) throw const FormatException('Invalid VM Service URL.');
    final nested = uri.queryParameters['uri'];
    if (nested != null) uri = Uri.tryParse(nested);
    if (uri == null ||
        !['http', 'https', 'ws', 'wss'].contains(uri.scheme) ||
        uri.host.isEmpty ||
        uri.userInfo.isNotEmpty ||
        uri.hasFragment) {
      throw const FormatException(
        'Enter a DevTools URL or a ws:// VM Service URL.',
      );
    }
    if (uri.path.contains('/devtools')) {
      throw const FormatException(
        'The DevTools link must include its uri= parameter.',
      );
    }
    var path = uri.path;
    if (!path.endsWith('/ws')) {
      if (!path.endsWith('/')) path += '/';
      path += 'ws';
    }
    return uri.replace(
      scheme: ['https', 'wss'].contains(uri.scheme) ? 'wss' : 'ws',
      path: path,
      query: null,
    );
  }

  Future<void> start({String? uri, bool automatic = false}) async {
    if (_disposed) return;
    final requestedGeneration = _generation + 1;
    await stop();
    if (_disposed || _generation != requestedGeneration) return;
    _wanted = true;
    autoDetect = automatic;
    if (!automatic || uri != null) _preferredProject = null;
    if (automatic) {
      if (uri != null) endpoint = uri;
      await _discoverAndConnect();
      return;
    }
    try {
      endpoint = normalizeUri(uri ?? endpoint).toString();
    } on FormatException catch (e) {
      error = e.message;
      _wanted = false;
      _notify();
      return;
    }
    await _connect();
  }

  Future<void> _discoverAndConnect() async {
    if (_disposed || !_wanted || busy || running) return;
    final generation = _generation;
    busy = true;
    error = null;
    _notify();
    VmServiceTarget? selected;
    try {
      final targets = await _discovery.discover();
      if (_generation != generation || _disposed) return;
      discoveredApps = List.unmodifiable(targets);
      selected = targets.where((app) => app.uri == endpoint).firstOrNull;
      if (selected == null) {
        final matching = _preferredProject == null
            ? targets
            : targets
                  .where((app) => app.projectKey == _preferredProject)
                  .toList();
        if (matching.length == 1) selected = matching.single;
      }
    } catch (_) {
      if (_generation == generation && !_disposed) {
        discoveredApps = const [];
        error =
            'Automatic discovery is unavailable. Enter the VM Service URL manually.';
      }
    } finally {
      if (_generation == generation) {
        busy = false;
        _notify();
      }
    }
    if (_generation != generation || _disposed) return;
    if (selected == null) {
      _scheduleRetry();
    } else {
      endpoint = selected.uri;
      _preferredProject = selected.projectKey;
      await _connect();
    }
  }

  Future<void> _connect() async {
    if (_disposed || !_wanted || busy || running) return;
    final generation = _generation;
    busy = true;
    error = null;
    _notify();
    WebSocket? socket;
    VmService? service;
    try {
      socket = await WebSocket.connect(
        endpoint,
      ).timeout(const Duration(seconds: 8));
      if (generation != _generation || _disposed) {
        await socket.close();
        return;
      }
      service = VmService(
        socket.cast<String>(),
        socket.add,
        disposeHandler: () => socket!.close(),
      );
      _socket = socket;
      _service = service;
      // Do not treat opening a socket as success until a VM answers.
      await service.getVM().timeout(const Duration(seconds: 8));
      if (generation != _generation) return;
      final active = service;
      unawaited(service.onDone.then((_) => _lost(active)));
      var consoleQueue = Future<void>.value();
      for (final stream in ['Stdout', 'Stderr', 'Logging']) {
        final events = switch (stream) {
          'Stdout' => service.onStdoutEvent,
          'Stderr' => service.onStderrEvent,
          _ => service.onLoggingEvent,
        };
        _subscriptions.add(
          events.listen((event) {
            final revision = store.revision;
            consoleQueue = consoleQueue.then<void>((_) async {
              if (store.revision != revision) return;
              await _console(active, stream, event);
            });
          }),
        );
        try {
          await service
              .streamListen(stream)
              .timeout(const Duration(seconds: 8));
        } on RPCError catch (e) {
          // Stream subscription may already be active on a shared service.
          if (e.code != 103) rethrow;
        }
      }
      await _poll(active, generation);
      var polling = false;
      if (generation != _generation) return;
      _timer = Timer.periodic(pollInterval, (_) async {
        if (polling || _service != active) return;
        polling = true;
        try {
          await _poll(active, generation);
        } catch (e) {
          if (_service == active) {
            error = 'VM Service: $e';
            _notify();
          }
        } finally {
          polling = false;
        }
      });
    } catch (e) {
      if (generation == _generation && !_disposed) {
        error = 'Cannot connect to Dart VM Service: $e';
        _service = null;
        _socket = null;
        _timer?.cancel();
        for (final subscription in _subscriptions) {
          unawaited(subscription.cancel());
        }
        _subscriptions.clear();
        await service?.dispose();
        await socket?.close();
        _scheduleRetry();
      }
    } finally {
      if (generation == _generation) {
        busy = false;
        _notify();
      }
    }
  }

  void _lost(VmService service) {
    if (_service != service || _disposed) return;
    _generation++;
    busy = false;
    _timer?.cancel();
    _service = null;
    _socket = null;
    discoveredApps = const [];
    error = null;
    _isolates.clear();
    _application = null;
    _cursors.clear();
    _fingerprints.clear();
    _stdoutBuffers.clear();
    for (final subscription in _subscriptions) {
      unawaited(subscription.cancel());
    }
    _subscriptions.clear();
    error = autoDetect ? null : 'VM Service disconnected. Reconnecting…';
    _notify();
    _scheduleRetry();
  }

  void _scheduleRetry() {
    _retry?.cancel();
    if (_wanted && !_disposed) {
      _retry = Timer(
        retryInterval,
        () => unawaited(autoDetect ? _discoverAndConnect() : _connect()),
      );
    }
  }

  Future<void> _poll(VmService service, int generation) async {
    final captureRevision = store.revision;
    final vm = await service.getVM().timeout(const Duration(seconds: 8));
    if (generation != _generation) return;
    final ids = (vm.isolates ?? []).map((i) => i.id!).toSet();
    _isolates.removeWhere((key, _) => !ids.contains(key));
    _cursors.removeWhere((key, _) => !ids.contains(key));
    for (final isolate in vm.isolates ?? <IsolateRef>[]) {
      if (generation != _generation) return;
      final id = isolate.id!;
      try {
        if (!_isolates.containsKey(id)) {
          final detail = await service
              .getIsolate(id)
              .timeout(const Duration(seconds: 8));
          if (!(detail.extensionRPCs ?? []).contains(
            'ext.dart.io.getHttpProfile',
          )) {
            continue;
          }
          final root = detail.rootLib?.uri ?? isolate.name ?? 'Flutter';
          final app = root.startsWith('package:')
              ? root.substring(8).split('/').first
              : root.split('/').last;
          // A VM process is one app. Multiple engines/background isolates and
          // hot restart must not create extra application tabs.
          _application ??= <String, dynamic>{
            'id': jsonEncode([
              Uri.parse(endpoint).authority,
              vm.pid,
              vm.startTime,
            ]),
            'appName': app,
            'deviceName':
                '${vm.operatingSystem ?? 'Flutter'} · ${vm.targetCPU ?? ''}',
            'platform': vm.operatingSystem ?? 'flutter',
          };
          final metadata = <String, dynamic>{
            ..._application!,
            'isolateId': id,
            'isolateName': isolate.name,
          };
          await service
              .callServiceExtension(
                'ext.dart.io.httpEnableTimelineLogging',
                isolateId: id,
                args: {'enabled': 'true'},
              )
              .timeout(const Duration(seconds: 8));
          if (generation != _generation) return;
          _isolates[id] = metadata;
          _ingest(metadata, []);
        }
        final profile = await service
            .callServiceExtension(
              'ext.dart.io.getHttpProfile',
              isolateId: id,
              args: {
                if (_cursors[id] != null) 'updatedSince': '${_cursors[id]}',
              },
            )
            .timeout(const Duration(seconds: 8));
        final json = profile.json ?? {};
        for (final ref in (json['requests'] as List? ?? [])) {
          if (generation != _generation) return;
          final summary = objectOf(ref);
          // updatedSince is inclusive. A profile updated at the cursor boundary
          // can appear twice; do not re-ingest it after Clear or Resume.
          final key = '$id:${summary['id']}';
          final fingerprint = jsonEncode(summary);
          if (_fingerprints[key] == fingerprint) continue;
          _fingerprints[key] = fingerprint;
          while (_fingerprints.length > store.maxRecords * 2) {
            _fingerprints.remove(_fingerprints.keys.first);
          }
          // Paused changes still advance the cursor, so Resume cannot replay them.
          if (store.paused) {
            store.ignoredWhilePaused++;
            continue;
          }
          Json full = summary;
          try {
            final result = await service
                .callServiceExtension(
                  'ext.dart.io.getHttpProfileRequest',
                  isolateId: id,
                  args: {'id': '${summary['id']}'},
                )
                .timeout(const Duration(seconds: 8));
            full = result.json ?? summary;
          } on RPCError {
            // DevTools may clear its remote history between listing and reading.
          }
          if (generation != _generation) return;
          if (captureRevision == store.revision) {
            _ingest(_isolates[id]!, FlutterCaptureAdapter.network(full));
          }
        }
        if (generation != _generation) return;
        if (json['timestamp'] is int) _cursors[id] = json['timestamp'] as int;
      } on RPCError catch (e) {
        // An isolate can exit during polling; other isolates must keep working.
        _isolates.remove(id);
        lastProtocolError = '$e';
      }
    }
    if (generation != _generation) return;
    if (_isolates.isNotEmpty) _ingest(_isolates.values.first, []);
    error = null;
    _notify();
  }

  void _ingest(Json metadata, List<Json> events) => store.ingest({
    'session': {
      for (final entry in metadata.entries)
        if (!['isolateId', 'isolateName'].contains(entry.key))
          entry.key: entry.value,
      'isolates': [
        for (final isolate in _isolates.values)
          {'id': isolate['isolateId'], 'name': isolate['isolateName']},
      ],
    },
    'sdk': {'name': 'dart_vm_service', 'schemaVersion': 1},
    'events': [
      for (final event in events)
        {
          ...event,
          'isolateId': metadata['isolateId'],
          'isolateName': metadata['isolateName'],
          // Profiler IDs are only unique within their originating isolate.
          if (event['requestId'] != null) ...{
            'vmRequestId': event['requestId'],
            'requestId': jsonEncode([
              metadata['isolateId'],
              event['requestId'],
            ]),
          },
        },
    ],
  });

  Future<String> _instanceText(
    VmService service,
    String isolate,
    dynamic value, {
    bool stringify = false,
  }) async {
    final ref = objectOf(value);
    final text = ref['valueAsString'];
    if (stringify &&
        text == null &&
        ref['id'] is String &&
        ref['kind'] != 'Null') {
      try {
        final result = await service
            .invoke(
              isolate,
              ref['id'] as String,
              'toString',
              [],
              disableBreakpoints: true,
            )
            .timeout(const Duration(seconds: 2));
        if (result is InstanceRef && result.valueAsString != null) {
          return result.valueAsString!;
        }
      } catch (_) {
        /* The object may already have been collected. */
      }
      return '${objectOf(ref['class'])['name'] ?? ref['kind'] ?? ''}';
    }
    if (ref['valueAsStringIsTruncated'] == true && ref['id'] is String) {
      try {
        final object = await service
            .getObject(isolate, ref['id'] as String)
            .timeout(const Duration(seconds: 5));
        if (object is Instance && object.valueAsString != null) {
          return object.valueAsString!;
        }
      } catch (_) {
        /* Preserve the preview if the VM has collected the object. */
      }
    }
    return text?.toString() ?? '';
  }

  Future<void> _console(VmService service, String stream, Event event) async {
    if (_service != service) return;
    final revision = store.revision;
    final id = event.isolate?.id;
    if (id == null) return;
    final metadata = _isolates[id];
    if (metadata == null) return;
    try {
      final json = event.json ?? {};
      var message = '';
      var level = stream == 'Stderr' ? 'error' : 'log';
      final args = <Json>[];
      final details = <String, dynamic>{};
      if (stream == 'Logging') {
        final record = objectOf(json['logRecord']);
        message = await _instanceText(service, id, record['message']);
        final severity = record['level'] as num? ?? 0;
        details['severity'] = severity;
        details['loggerName'] = await _instanceText(
          service,
          id,
          record['loggerName'],
        );
        level = severity >= 1000
            ? 'error'
            : severity >= 900
            ? 'warn'
            : severity >= 800
            ? 'info'
            : severity > 0 && severity < 800
            ? 'debug'
            : 'log';
        for (final name in ['error', 'stackTrace']) {
          final text = await _instanceText(
            service,
            id,
            record[name],
            stringify: true,
          );
          if (text.isNotEmpty && text != 'null') {
            args.add({'type': name, 'value': text});
            details[name] = text;
          }
        }
        if (details.containsKey('error')) level = 'error';
      } else {
        final bytes = json['bytes'];
        if (bytes is! String) return;
        // VM stdout chunks are not necessarily individual lines.
        final key = '$id:$stream';
        if (_stdoutRevision != revision) {
          _stdoutBuffers.clear();
          _stdoutRevision = revision;
        }
        final buffer = _stdoutBuffers.putIfAbsent(key, () => []);
        buffer.addAll(base64Decode(bytes));
        final lines = <String>[];
        var start = 0;
        for (var index = 0; index < buffer.length; index++) {
          if (buffer[index] == 10) {
            lines.add(
              utf8.decode(buffer.sublist(start, index), allowMalformed: true),
            );
            start = index + 1;
          }
        }
        buffer.removeRange(0, start);
        if (buffer.length > 65536) {
          lines.add(utf8.decode(buffer, allowMalformed: true));
          buffer.clear();
        }
        for (final line in lines) {
          if (_service != service) return;
          _ingest(metadata, [
            {
              'source': stream == 'Stderr' ? 'dart.stderr' : 'dart.stdout',
              'level': level,
              'message': line,
              'args': [],
              'timestamp': DateTime.fromMillisecondsSinceEpoch(
                event.timestamp ?? DateTime.now().millisecondsSinceEpoch,
                isUtc: true,
              ).toIso8601String(),
            },
          ]);
        }
        return;
      }
      if (_service != service || store.revision != revision) return;
      _ingest(metadata, [
        {
          'source': 'dart.logging',
          'level': level,
          'message': message,
          'args': args,
          ...details,
          'timestamp': DateTime.fromMillisecondsSinceEpoch(
            event.timestamp ?? DateTime.now().millisecondsSinceEpoch,
            isUtc: true,
          ).toIso8601String(),
        },
      ]);
    } catch (e) {
      if (_service != service) return;
      invalidMessages++;
      lastProtocolError = '$e';
      _notify();
    }
  }

  Future<void> stop() async {
    _wanted = false;
    _generation++;
    _retry?.cancel();
    _timer?.cancel();
    final service = _service;
    final socket = _socket;
    _service = null;
    _socket = null;
    discoveredApps = const [];
    error = null;
    _isolates.clear();
    _application = null;
    _cursors.clear();
    _fingerprints.clear();
    _stdoutBuffers.clear();
    busy = false;
    for (final subscription in _subscriptions) {
      unawaited(subscription.cancel());
    }
    _subscriptions.clear();
    await service?.dispose();
    await socket?.close();
    _notify();
  }

  void _notify() {
    if (!_disposed) notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    unawaited(stop());
    super.dispose();
  }
}
