import 'dart:async';
import 'dart:collection';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:vm_service/vm_service.dart';

import '../models/capture.dart';
import 'capture_store.dart';
import 'flutter_capture_adapter.dart';
import 'vm_service_discovery.dart';
import 'viewer_diagnostics.dart';

class _IsolateChanged implements Exception {}

/// A DevTools-compatible VM Service client. The historical name is retained so
/// the copied UI and MCP tools share the same capture/presence interface.
class ViewerServer extends ChangeNotifier {
  ViewerServer(
    this.store, {
    this.pollInterval = const Duration(milliseconds: 1400),
    this.idlePollIncrement = const Duration(milliseconds: 200),
    this.maxPollInterval = const Duration(seconds: 5),
    this.retryInterval = const Duration(seconds: 3),
    this.rpcTimeout = const Duration(seconds: 8),
    this.diagnostics,
    VmServiceDiscovery? discovery,
  }) : _discovery = discovery ?? VmServiceDiscovery(),
       _networkPollInterval = pollInterval,
       _consolePollInterval = pollInterval {
    store.addListener(_captureChanged);
  }
  static const defaultServiceUri = String.fromEnvironment('VM_SERVICE_URI');
  final CaptureStore store;
  final Duration pollInterval;
  final Duration idlePollIncrement;
  final Duration maxPollInterval;
  Duration _networkPollInterval;
  Duration _consolePollInterval;
  @visibleForTesting
  Duration get networkPollInterval => _networkPollInterval;
  @visibleForTesting
  Duration get consolePollInterval => _consolePollInterval;
  final Duration retryInterval;
  final Duration rpcTimeout;
  final ViewerDiagnostics? diagnostics;
  final VmServiceDiscovery _discovery;
  bool _autoDetect = false;
  bool get autoDetect => _adding?.autoDetect ?? _autoDetect;
  set autoDetect(bool value) => _autoDetect = value;
  bool get searching =>
      _adding?.searching ?? (_autoDetect && _wanted && _service == null);
  List<VmServiceTarget> _discoveredApps = const [];
  List<VmServiceTarget> get discoveredApps =>
      _adding?.discoveredApps ?? _discoveredApps;
  set discoveredApps(List<VmServiceTarget> value) => _discoveredApps = value;
  final _additional = <ViewerServer>[];
  ViewerServer? _adding;
  Set<String> Function()? _excludedVmKeys;
  String? _vmKey;
  VmService? _service;
  WebSocket? _socket;
  Timer? _timer;
  Timer? _consoleTimer;
  final _pendingConsole = Queue<({Json metadata, Json event, int bytes})>();
  int _pendingConsoleBytes = 0;
  int _pendingConsoleRevision = 0;
  Timer? _retry;
  final _isolates = <String, Json>{};
  final _isolateChanges = <String, Set<Completer<void>>>{};
  final _exitedIsolates = <String>{};
  int? _pollingGeneration;
  bool _pollRequested = false;
  int _pollFailures = 0;
  Json? _application;
  final _cursors = <String, int>{};
  // Stream positions only; never retain cleared bodies, headers or log events.
  String? _clearedApplicationId;
  final _clearedCursors = <String, int>{};
  final _fingerprints = <String, String>{};
  final _stdoutBuffers = <String, List<int>>{};
  int _stdoutRevision = 0;
  final _subscriptions = <StreamSubscription<dynamic>>[];
  bool _disposed = false;
  bool _wanted = false;
  int _generation = 0;
  bool _busy = false;
  bool get busy => _busy || (_adding?.busy ?? false);
  set busy(bool value) => _busy = value;
  String endpoint = defaultServiceUri;
  String? _error;
  String? get error => _adding?.error ?? _error;
  set error(String? value) => _error = value;
  String? lastProtocolError;
  int invalidMessages = 0;
  bool get running =>
      _service != null || _additional.any((client) => client.running);
  int get connections =>
      (_service != null ? 1 : 0) +
      _additional.fold<int>(0, (count, client) => count + client.connections);
  int get port => Uri.tryParse(endpoint)?.port ?? 0;
  String get localUrl => endpoint;
  bool isOnline(String sessionId) =>
      (_service != null &&
          _isolates.values.any((session) => session['id'] == sessionId)) ||
      _additional.any((client) => client.isOnline(sessionId));

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
    if (automatic && (_service != null || _additional.isNotEmpty)) {
      await _addProject(uri);
      return;
    }
    final requestedGeneration = _generation + 1;
    await stop();
    if (_disposed || _generation != requestedGeneration) return;
    _wanted = true;
    diagnostics?.record('capture_started', {'automatic': automatic});
    autoDetect = automatic;
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

  // Each VM owns its own socket, polling timers, cursors and restart lifecycle.
  // Adding another project must never stop or clear already attached projects.
  Future<void> _addProject(String? uri) async {
    var client = _adding;
    if (client == null) {
      client = ViewerServer(
        store,
        pollInterval: pollInterval,
        idlePollIncrement: idlePollIncrement,
        maxPollInterval: maxPollInterval,
        retryInterval: retryInterval,
        rpcTimeout: rpcTimeout,
        diagnostics: diagnostics,
        discovery: _discovery,
      );
      final candidate = client;
      client._excludedVmKeys = () => {
        ?_vmKey,
        for (final existing in _additional)
          if (existing != candidate && existing._vmKey != null)
            existing._vmKey!,
      };
      _additional.add(client);
      _adding = client;
      client.addListener(_additionalChanged);
    }
    diagnostics?.record('additional_discovery_started', {
      'connections': connections,
    });
    _notify();
    await client.start(automatic: true, uri: uri);
    _additionalChanged();
  }

  void _additionalChanged() {
    final candidate = _adding;
    if (candidate != null &&
        candidate._service != null &&
        !candidate.busy &&
        !candidate.autoDetect) {
      _adding = null;
      _autoDetect = false;
      diagnostics?.record('additional_project_connected', {
        'connections': connections,
      });
    }
    _notify();
  }

  /// Cancel only discovery. Existing projects continue capturing live data.
  Future<void> stopDiscovery() async {
    final candidate = _adding;
    _adding = null;
    _autoDetect = false;
    if (candidate != null) {
      candidate.removeListener(_additionalChanged);
      _additional.remove(candidate);
      await candidate.stop();
      candidate.dispose();
    } else if (_service == null && _additional.isEmpty) {
      await stop();
    }
    _notify();
  }

  Future<void> _discoverAndConnect() async {
    if (_disposed || !_wanted || _busy || _service != null) return;
    final generation = _generation;
    busy = true;
    error = null;
    _notify();
    VmServiceTarget? selected;
    try {
      final found = await _discovery.discover();
      final excluded = _excludedVmKeys?.call() ?? <String>{};
      final targets = found
          .where((target) => !excluded.contains(target.vmKey))
          .toList();
      diagnostics?.record('discovery_completed', {'targets': targets.length});
      if (_generation != generation || _disposed) return;
      discoveredApps = List.unmodifiable(targets);
      selected = targets.where((app) => app.uri == endpoint).firstOrNull;
      if (selected == null && targets.length == 1) selected = targets.single;
    } catch (_) {
      diagnostics?.record('discovery_failed');
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
      await _connect();
    }
  }

  Future<void> _connect() async {
    if (_disposed || !_wanted || _busy || _service != null) return;
    final generation = _generation;
    busy = true;
    error = null;
    _notify();
    WebSocket? socket;
    VmService? service;
    diagnostics?.record('connection_attempt', {
      'host': Uri.tryParse(endpoint)?.host,
      'port': Uri.tryParse(endpoint)?.port,
      'generation': generation,
    });
    try {
      socket = await WebSocket.connect(endpoint).timeout(rpcTimeout);
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
      final active = service;
      unawaited(service.onDone.then((_) => _lost(active)));
      // Do not treat opening a socket as success until a VM answers.
      final vm = await _rpc('getVM', () => active.getVM());
      if (generation != _generation) return;
      _vmKey = '${vm.pid}:${vm.startTime}:${vm.operatingSystem}';
      diagnostics?.record('vm_ready', {'vmPid': vm.pid, 'version': vm.version});
      _subscriptions.add(
        active.onIsolateEvent.listen((event) {
          if (_isActive(active, generation)) _isolateEvent(event);
        }),
      );
      try {
        await _rpc(
          'streamListen:Isolate',
          () => active.streamListen('Isolate'),
        );
      } on RPCError catch (e) {
        if (e.code != 103) rethrow;
      }
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
          await _rpc('streamListen:$stream', () => active.streamListen(stream));
        } on RPCError catch (e) {
          // Stream subscription may already be active on a shared service.
          if (e.code != 103) rethrow;
        }
      }
      // Isolate extensions may be unavailable during a hot restart. They are
      // retried by polling without throwing away an established VM transport.
      try {
        await _poll(active, generation);
        _pollFailures = 0;
      } on TimeoutException {
        _pollFailures = 1;
      } on RPCError {
        _pollFailures = 1;
      }
      if (generation != _generation) return;
      // Discovery is only for finding the initial connection. Once capture is
      // ready, reconnect this endpoint without scanning or switching projects.
      autoDetect = false;
      diagnostics?.record('discovery_locked');
      _retry?.cancel();
      _retry = null;
      discoveredApps = const [];
      _networkPollInterval = pollInterval;
      _consolePollInterval = pollInterval;
      _schedulePoll(active, generation);
      _scheduleConsole(active, generation);
      diagnostics?.record('connection_ready', {'isolates': _isolates.length});
    } catch (e) {
      if (generation == _generation && !_disposed) {
        diagnostics?.record('connection_failed', {
          'errorType': '${e.runtimeType}',
        });
        error = 'Cannot connect to Dart VM Service: $e';
        _service = null;
        _socket = null;
        final sessionId = _application?['id'] as String?;
        if (sessionId != null) store.removeSession(sessionId);
        _isolates.clear();
        _invalidateIsolateCalls();
        _application = null;
        _cursors.clear();
        _fingerprints.clear();
        _stdoutBuffers.clear();
        _pendingConsole.clear();
        _pendingConsoleBytes = 0;
        _timer?.cancel();
        _consoleTimer?.cancel();
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

  Duration _nextInterval(Duration current, bool changed) {
    if (changed) return pollInterval;
    final next = current + idlePollIncrement;
    return next > maxPollInterval ? maxPollInterval : next;
  }

  bool _isActive(VmService service, int generation) =>
      !_disposed && _wanted && _service == service && _generation == generation;

  Future<T> _rpc<T>(
    String method,
    Future<T> Function() call, {
    String? isolate,
  }) async {
    final clock = Stopwatch()..start();
    try {
      final result = await call().timeout(rpcTimeout);
      if (clock.elapsedMilliseconds >= 1000) {
        diagnostics?.record('rpc_slow', {
          'method': method,
          'isolate': isolate,
          'elapsedMs': clock.elapsedMilliseconds,
        });
      }
      return result;
    } catch (e) {
      diagnostics
          ?.record(e is _IsolateChanged ? 'rpc_cancelled' : 'rpc_failed', {
            'method': method,
            'isolate': isolate,
            'elapsedMs': clock.elapsedMilliseconds,
            'errorType': '${e.runtimeType}',
            if (e is RPCError) 'code': e.code,
          });
      rethrow;
    }
  }

  Future<T> _isolateRpc<T>(
    String id,
    String method,
    Future<T> Function() call,
  ) async {
    if (_exitedIsolates.contains(id)) throw _IsolateChanged();
    final pending = _isolateChanges.putIfAbsent(id, () => {});
    final changed = Completer<void>();
    pending.add(changed);
    try {
      return await _rpc(
        method,
        () => Future.any<T>([
          call(),
          changed.future.then<T>((_) => throw _IsolateChanged()),
        ]),
        isolate: id,
      );
    } finally {
      pending.remove(changed);
      if (pending.isEmpty && identical(_isolateChanges[id], pending)) {
        _isolateChanges.remove(id);
      }
    }
  }

  void _discardIsolate(String id, {bool resetCursor = true}) {
    for (final changed in _isolateChanges.remove(id) ?? <Completer<void>>{}) {
      changed.complete();
    }
    _isolates.remove(id);
    if (resetCursor) {
      _cursors.remove(id);
      _clearedCursors.remove(id);
      _fingerprints.removeWhere((key, _) => key.startsWith('$id:'));
    }
    _stdoutBuffers.removeWhere((key, _) => key.startsWith('$id:'));
  }

  void _invalidateIsolateCalls() {
    for (final pending in _isolateChanges.values) {
      for (final changed in pending) {
        changed.complete();
      }
    }
    _isolateChanges.clear();
    _exitedIsolates.clear();
  }

  void _isolateEvent(Event event) {
    final id = event.isolate?.id;
    if (id == null) return;
    final kind = event.kind;
    if (kind == EventKind.kIsolateExit) {
      _discardIsolate(id);
      _exitedIsolates.add(id);
    } else if (kind == EventKind.kIsolateStart) {
      _discardIsolate(id);
      _exitedIsolates.remove(id);
    } else if (kind != EventKind.kIsolateRunnable &&
        !(kind == EventKind.kServiceExtensionAdded &&
            event.extensionRPC == 'ext.dart.io.getHttpProfile')) {
      return;
    }
    diagnostics?.record('isolate_event', {'kind': kind, 'isolate': id});
    _pollRequested = true;
    if (!busy && _pollingGeneration != _generation && _service != null) {
      _schedulePoll(_service!, _generation, immediately: true);
    }
  }

  void _schedulePoll(
    VmService service,
    int generation, {
    bool immediately = false,
  }) {
    if (!_isActive(service, generation)) return;
    _timer?.cancel();
    _timer = Timer(immediately ? Duration.zero : _networkPollInterval, () async {
      if (!_isActive(service, generation) || _pollingGeneration == generation) {
        return;
      }
      _pollingGeneration = generation;
      _pollRequested = false;
      try {
        final changed = await _poll(service, generation);
        if (!_isActive(service, generation)) return;
        if (_pollFailures > 0) diagnostics?.record('capture_recovered');
        _pollFailures = 0;
        _networkPollInterval = _nextInterval(_networkPollInterval, changed);
      } catch (e) {
        if (!_isActive(service, generation)) return;
        _pollFailures++;
        diagnostics?.record('poll_failed', {
          'consecutive': _pollFailures,
          'errorType': '${e.runtimeType}',
        });
        if (_pollFailures >= 2) {
          // A socket can stay open after DDS/VM stops responding. Replace that
          // transport, using the same project endpoint rather than rediscovery.
          diagnostics?.record('connection_unresponsive');
          _lost(service);
          return;
        }
      } finally {
        if (_pollingGeneration == generation) _pollingGeneration = null;
      }
      // Schedule only after completion: slow VM calls must never overlap.
      _schedulePoll(service, generation, immediately: _pollRequested);
    });
  }

  void _scheduleConsole(VmService service, int generation) {
    if (!_isActive(service, generation)) return;
    _consoleTimer = Timer(_consolePollInterval, () {
      if (!_isActive(service, generation)) return;
      final changed = _flushConsole();
      _consolePollInterval = _nextInterval(_consolePollInterval, changed);
      _scheduleConsole(service, generation);
    });
  }

  void _captureChanged() {
    if (_pendingConsoleRevision != store.revision) {
      _fingerprints.clear();
      _stdoutBuffers.clear();
      // Keep only the live stream position, not old request summaries. The VM
      // cursor is inclusive, so exclude the boundary already cleared by user.
      _cursors.updateAll((_, timestamp) => timestamp + 1);
      if (_application != null) {
        _clearedApplicationId = _application!['id'] as String;
        _clearedCursors
          ..clear()
          ..addAll(_cursors);
      }
    }
    if (store.paused || _pendingConsoleRevision != store.revision) {
      _pendingConsole.clear();
      _pendingConsoleBytes = 0;
      _pendingConsoleRevision = store.revision;
    }
  }

  void _queueConsole(Json metadata, Json event) {
    _captureChanged();
    if (store.paused) {
      store.ignoredWhilePaused++;
      return;
    }
    final bytes = utf8.encode(jsonEncode(event)).length;
    _pendingConsole.add((metadata: metadata, event: event, bytes: bytes));
    _pendingConsoleBytes += bytes;
    // Bound pending live logs just like the retained capture history.
    while (_pendingConsole.isNotEmpty &&
        (_pendingConsole.length > store.maxRecords ||
            _pendingConsoleBytes > store.maxBytes)) {
      _pendingConsoleBytes -= _pendingConsole.removeFirst().bytes;
      store.evicted++;
    }
  }

  bool _flushConsole() {
    _captureChanged();
    if (_pendingConsole.isEmpty) return false;
    final pending = _pendingConsole.toList();
    _pendingConsole.clear();
    _pendingConsoleBytes = 0;
    // Batch consecutive logs from the same isolate without changing log order.
    var start = 0;
    while (start < pending.length) {
      final metadata = pending[start].metadata;
      var end = start + 1;
      while (end < pending.length &&
          pending[end].metadata['isolateId'] == metadata['isolateId']) {
        end++;
      }
      _ingest(metadata, [for (var i = start; i < end; i++) pending[i].event]);
      start = end;
    }
    return true;
  }

  void _lost(VmService service) {
    if (_service != service || _disposed) return;
    final socket = _socket;
    diagnostics?.record('connection_lost', {
      'closeCode': socket?.closeCode,
      'pollFailures': _pollFailures,
      'isolates': _isolates.length,
    });
    _consoleTimer?.cancel();
    _pendingConsole.clear();
    _pendingConsoleBytes = 0;
    _generation++;
    busy = false;
    _timer?.cancel();
    _service = null;
    _socket = null;
    final sessionId = _application?['id'] as String?;
    if (sessionId != null) store.removeSession(sessionId);
    discoveredApps = const [];
    error = null;
    _isolates.clear();
    _invalidateIsolateCalls();
    _application = null;
    _cursors.clear();
    _fingerprints.clear();
    _stdoutBuffers.clear();
    for (final subscription in _subscriptions) {
      unawaited(subscription.cancel());
    }
    _subscriptions.clear();
    unawaited(service.dispose());
    unawaited(socket?.close());
    error = _autoDetect ? null : 'VM Service disconnected. Reconnecting…';
    _notify();
    _scheduleRetry();
  }

  void _scheduleRetry() {
    _retry?.cancel();
    if (_wanted && !_disposed) {
      diagnostics?.record('retry_scheduled', {
        'delayMs': retryInterval.inMilliseconds,
      });
      _retry = Timer(
        retryInterval,
        () => unawaited(_autoDetect ? _discoverAndConnect() : _connect()),
      );
    }
  }

  Future<bool> _poll(VmService service, int generation) async {
    var changed = false;
    var profilingFailed = false;
    var profiled = false;
    final captureRevision = store.revision;
    final vm = await _rpc('getVM', () => service.getVM());
    if (generation != _generation) return false;
    final ids = (vm.isolates ?? []).map((i) => i.id!).toSet();
    for (final id in {
      ..._isolates.keys,
      ..._cursors.keys,
    }.where((id) => !ids.contains(id))) {
      _discardIsolate(id);
    }
    _exitedIsolates.removeWhere((id) => !ids.contains(id));
    for (final isolate in vm.isolates ?? <IsolateRef>[]) {
      if (generation != _generation) return false;
      final id = isolate.id!;
      if (_exitedIsolates.contains(id)) continue;
      try {
        if (!_isolates.containsKey(id)) {
          final detail = await _isolateRpc(
            id,
            'getIsolate',
            () => service.getIsolate(id),
          );
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
          await _isolateRpc(
            id,
            'ext.dart.io.httpEnableTimelineLogging',
            () => service.callServiceExtension(
              'ext.dart.io.httpEnableTimelineLogging',
              isolateId: id,
              args: {'enabled': 'true'},
            ),
          );
          if (generation != _generation) return false;
          _isolates[id] = metadata;
          if (_application!['id'] == _clearedApplicationId &&
              _clearedCursors.containsKey(id)) {
            _cursors[id] = _clearedCursors[id]!;
          }
          _ingest(metadata, []);
          diagnostics?.record('isolate_capture_ready', {'isolate': id});
        }
        final metadata = _isolates[id]!;
        final profile = await _isolateRpc(
          id,
          'ext.dart.io.getHttpProfile',
          () => service.callServiceExtension(
            'ext.dart.io.getHttpProfile',
            isolateId: id,
            args: {if (_cursors[id] != null) 'updatedSince': '${_cursors[id]}'},
          ),
        );
        final json = profile.json ?? {};
        if (generation != _generation) return false;
        profiled = true;
        final timestamp = json['timestamp'];
        if (timestamp is int) {
          final next = timestamp + (captureRevision == store.revision ? 0 : 1);
          if (next > (_cursors[id] ?? 0)) _cursors[id] = next;
          if (_application?['id'] == _clearedApplicationId) {
            _clearedCursors[id] = _cursors[id]!;
          }
        }
        for (final ref in (json['requests'] as List? ?? [])) {
          if (generation != _generation || captureRevision != store.revision) {
            return false;
          }
          final summary = objectOf(ref);
          // updatedSince is inclusive. A profile updated at the cursor boundary
          // can appear twice; do not re-ingest it after Clear or Resume.
          final key = '$id:${summary['id']}';
          final fingerprint = jsonEncode(summary);
          if (_fingerprints[key] == fingerprint) continue;
          changed = true;
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
          if (FlutterCaptureAdapter.exceedsBodyLimit(summary)) {
            diagnostics?.record('body_omitted', {
              'isolate': id,
              'reason': 'profile-size-limit',
              'requestBytes': FlutterCaptureAdapter.declaredBodyLength(
                summary['request'],
              ),
              'responseBytes': FlutterCaptureAdapter.declaredBodyLength(
                summary['response'],
              ),
            });
            full = FlutterCaptureAdapter.omitBodies(
              summary,
              'profile-size-limit',
            );
          } else {
            try {
              final result = await _isolateRpc(
                id,
                'ext.dart.io.getHttpProfileRequest',
                () => service.callServiceExtension(
                  'ext.dart.io.getHttpProfileRequest',
                  isolateId: id,
                  args: {'id': '${summary['id']}'},
                ),
              );
              full = result.json ?? summary;
            } on RPCError catch (e) {
              // A bad/cleared body belongs to this request, not the connection.
              full = FlutterCaptureAdapter.omitBodies(
                summary,
                'profile-read-error',
              );
              lastProtocolError = 'HTTP profile ${summary['id']}: $e';
            } on TimeoutException catch (e) {
              // Keep following other requests and console streams on this VM.
              full = FlutterCaptureAdapter.omitBodies(
                summary,
                'profile-read-error',
              );
              lastProtocolError = 'HTTP profile ${summary['id']}: $e';
            }
          }
          if (generation != _generation) return false;
          final bodySize = ['requestBody', 'responseBody'].fold<int>(0, (
            total,
            key,
          ) {
            final body = full[key];
            if (body is List) return total + body.length;
            if (body is String) return total + body.length;
            return total;
          });
          final events = bodySize > 65536
              ? await compute(FlutterCaptureAdapter.network, full)
              : FlutterCaptureAdapter.network(full);
          if (generation != _generation) return false;
          if (!identical(_isolates[id], metadata)) break;
          if (captureRevision == store.revision) {
            _ingest(metadata, events);
          }
        }
        if (generation != _generation) return false;
      } on _IsolateChanged {
        // Exit/Start invalidates old calls immediately, even if the VM never
        // replies to that old isolate. The next iteration can attach the new one.
        continue;
      } on TimeoutException catch (e) {
        _discardIsolate(id, resetCursor: false);
        profilingFailed = true;
        lastProtocolError =
            'HTTP profiling timed out for $id: ${e.runtimeType}';
      } on RPCError catch (e) {
        // An isolate can exit during polling; other isolates must keep working.
        _discardIsolate(id, resetCursor: false);
        profilingFailed = true;
        lastProtocolError = '$e';
      }
    }
    if (generation != _generation) return false;
    if (profilingFailed && !profiled) {
      throw TimeoutException(
        'HTTP profiling is temporarily unavailable',
        rpcTimeout,
      );
    }
    if (_isolates.isNotEmpty &&
        store.sessions.containsKey(_application?['id'])) {
      _ingest(_isolates.values.first, []);
    }
    error = null;
    _notify();
    return changed;
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
          _queueConsole(metadata, {
            'source': stream == 'Stderr' ? 'dart.stderr' : 'dart.stdout',
            'level': level,
            'message': line,
            'args': [],
            'timestamp': DateTime.fromMillisecondsSinceEpoch(
              event.timestamp ?? DateTime.now().millisecondsSinceEpoch,
              isUtc: true,
            ).toIso8601String(),
          });
        }
        return;
      }
      if (_service != service || store.revision != revision) return;
      _queueConsole(metadata, {
        'source': 'dart.logging',
        'level': level,
        'message': message,
        'args': args,
        ...details,
        'timestamp': DateTime.fromMillisecondsSinceEpoch(
          event.timestamp ?? DateTime.now().millisecondsSinceEpoch,
          isUtc: true,
        ).toIso8601String(),
      });
    } catch (e) {
      if (_service != service) return;
      invalidMessages++;
      lastProtocolError = '$e';
      _notify();
    }
  }

  Future<void> stop() async {
    final clients = _additional.toList();
    _additional.clear();
    _adding = null;
    for (final client in clients) {
      client.removeListener(_additionalChanged);
      await client.stop();
      client.dispose();
    }
    diagnostics?.record('capture_stopped');
    _consoleTimer?.cancel();
    _pendingConsole.clear();
    _pendingConsoleBytes = 0;
    _wanted = false;
    _generation++;
    _retry?.cancel();
    _timer?.cancel();
    final service = _service;
    final socket = _socket;
    _service = null;
    _socket = null;
    final sessionId = _application?['id'] as String?;
    if (sessionId != null) store.removeSession(sessionId);
    discoveredApps = const [];
    error = null;
    _isolates.clear();
    _invalidateIsolateCalls();
    _pollFailures = 0;
    _pollRequested = false;
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
    store.removeListener(_captureChanged);
    unawaited(stop());
    super.dispose();
  }
}
