import 'dart:async';
import 'dart:io';

import 'package:vm_service/vm_service.dart';

class VmServiceTarget {
  const VmServiceTarget({
    required this.uri,
    required this.name,
    required this.projectKey,
    required this.vmKey,
  });

  final String uri;
  final String name;
  final String projectKey;
  final String vmKey;
}

/// Finds the authenticated VM URLs published by local Dart Development Service
/// processes. No port scanning, SDK path configuration or device log access.
class VmServiceDiscovery {
  VmServiceDiscovery({Future<String> Function()? readProcesses})
    : _readProcesses = readProcesses ?? _processes;

  final Future<String> Function() _readProcesses;

  static Future<String> _processes() async {
    final process = Platform.isWindows
        ? await Process.start('powershell.exe', [
            '-NoProfile',
            '-NonInteractive',
            '-Command',
            r'Get-CimInstance Win32_Process | Where-Object { $_.Name -in @("dart.exe", "dartaotruntime.exe") } | Select-Object -ExpandProperty CommandLine',
          ])
        : await Process.start('/bin/ps', ['-ww', '-axo', 'command=']);
    final output = process.stdout
        .transform(const SystemEncoding().decoder)
        .join();
    final errors = process.stderr.drain<void>();
    try {
      final code = await process.exitCode.timeout(const Duration(seconds: 5));
      await errors;
      if (code != 0) {
        throw const ProcessException(
          'process list',
          [],
          'Discovery unavailable',
        );
      }
      return await output;
    } finally {
      process.kill();
    }
  }

  static bool _local(Uri uri) =>
      ['http', 'ws'].contains(uri.scheme) &&
      ['localhost', '127.0.0.1', '::1'].contains(uri.host) &&
      uri.userInfo.isEmpty &&
      uri.hasPort &&
      uri.port > 0;

  static List<Uri> processUris(String processes) {
    final uris = <String, Uri>{};
    final argument = RegExp(r'''--vm-service-uri(?:=|\s+)["']?([^\s"']+)''');
    for (final line in processes.split('\n')) {
      if (!RegExp(
        r'(?:development-service|dds(?:_aot)?\.dart(?:\.snapshot)?)\b',
      ).hasMatch(line)) {
        continue;
      }
      for (final match in argument.allMatches(line)) {
        final uri = Uri.tryParse(match.group(1)!);
        if (uri != null && _local(uri)) uris[uri.toString()] = uri;
      }
    }
    return uris.values.take(24).toList();
  }

  Future<List<VmServiceTarget>> discover() async {
    final uris = processUris(await _readProcesses());
    final found = await Future.wait(uris.map(_inspect));
    final unique = <String, VmServiceTarget>{};
    for (final target in found.whereType<VmServiceTarget>()) {
      unique[target.vmKey] = target;
    }
    return unique.values.toList()..sort((a, b) => a.name.compareTo(b.name));
  }

  static Uri _webSocket(Uri uri) {
    var path = uri.path;
    if (!path.endsWith('/ws')) {
      path = '${path.endsWith('/') ? path : '$path/'}ws';
    }
    return uri.replace(scheme: 'ws', path: path, query: null, fragment: null);
  }

  /// The original VM redirects to DDS, whose port and authentication token differ.
  static Future<Uri> _ddsUri(Uri source) async {
    final client = HttpClient()..connectionTimeout = const Duration(seconds: 2);
    var uri = source.replace(scheme: 'http');
    if (uri.path.endsWith('/ws')) {
      uri = uri.replace(path: uri.path.substring(0, uri.path.length - 2));
    }
    try {
      for (var i = 0; i < 4; i++) {
        final request = await client
            .getUrl(uri)
            .timeout(const Duration(seconds: 2));
        request.followRedirects = false;
        final response = await request.close().timeout(
          const Duration(seconds: 2),
        );
        final location = response.headers.value(HttpHeaders.locationHeader);
        if (!response.isRedirect || location == null) return _webSocket(uri);
        final next = uri.resolve(location);
        final nested = Uri.tryParse(next.queryParameters['uri'] ?? '');
        if (nested != null && _local(nested)) return _webSocket(nested);
        if (!_local(next)) {
          throw const FormatException('Non-local DDS redirect');
        }
        uri = next;
      }
      throw const FormatException('Too many DDS redirects');
    } finally {
      client.close(force: true);
    }
  }

  static Future<VmServiceTarget?> _inspect(Uri source) async {
    WebSocket? socket;
    VmService? service;
    try {
      final uri = await _ddsUri(source);
      socket = await WebSocket.connect(
        uri.toString(),
      ).timeout(const Duration(seconds: 2));
      service = VmService(socket.cast<String>(), socket.add);
      final vm = await service.getVM().timeout(const Duration(seconds: 2));
      if (vm.pid == pid && vm.operatingSystem == Platform.operatingSystem) {
        return null;
      }
      for (final ref in (vm.isolates ?? <IsolateRef>[]).take(8)) {
        final isolate = await service
            .getIsolate(ref.id!)
            .timeout(const Duration(seconds: 2));
        if (!(isolate.extensionRPCs ?? []).any(
          (rpc) => rpc.startsWith('ext.flutter.'),
        )) {
          continue;
        }
        final root = isolate.rootLib?.uri ?? '';
        if (root.startsWith('package:flutter_network_viewer/')) return null;
        final package = RegExp(r'^package:([^/]+)/').firstMatch(root)?.group(1);
        return VmServiceTarget(
          uri: uri.toString(),
          name: package ?? (isolate.name ?? 'Flutter app'),
          projectKey: root.isEmpty ? (isolate.name ?? 'Flutter app') : root,
          vmKey: '${vm.pid}:${vm.startTime}:${vm.operatingSystem}',
        );
      }
    } catch (_) {
      // Processes can disappear or restart between enumeration and inspection.
    } finally {
      await service?.dispose();
      await socket?.close();
    }
    return null;
  }
}
