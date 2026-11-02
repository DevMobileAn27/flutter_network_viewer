import 'dart:async';
import 'dart:collection';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

/// Connection metadata only: never pass VM URLs, headers, bodies or app logs.
/// Batched writes and rotation keep diagnostics inexpensive and bounded.
class ViewerDiagnostics {
  ViewerDiagnostics(this.directory, {this.maxFileBytes = 512 * 1024});
  final Directory directory;
  final int maxFileBytes;
  final _pending = Queue<String>();
  int _pendingBytes = 0;
  Timer? _timer;
  Future<void>? _writing;
  String get path => '${directory.path}${Platform.pathSeparator}viewer.log';

  factory ViewerDiagnostics.desktop() {
    final env = Platform.environment;
    final String root;
    if (Platform.isMacOS) {
      root = '${env['HOME'] ?? Directory.systemTemp.path}/Library/Logs';
    } else if (Platform.isWindows) {
      root = env['LOCALAPPDATA'] ?? Directory.systemTemp.path;
    } else {
      root =
          env['XDG_STATE_HOME'] ??
          '${env['HOME'] ?? Directory.systemTemp.path}/.local/state';
    }
    return ViewerDiagnostics(Directory('$root/Flutter Network Viewer'));
  }

  void record(String event, [Map<String, Object?> fields = const {}]) {
    final line =
        '${jsonEncode({'time': DateTime.now().toUtc().toIso8601String(), 'pid': pid, 'event': event, ...fields})}\n';
    _pending.add(line);
    _pendingBytes += utf8.encode(line).length;
    while (_pendingBytes > 128 * 1024 && _pending.isNotEmpty) {
      _pendingBytes -= utf8.encode(_pending.removeFirst()).length;
    }
    _timer ??= Timer(const Duration(milliseconds: 250), () {
      _timer = null;
      unawaited(flush());
    });
  }

  Future<void> flush() async {
    _timer?.cancel();
    _timer = null;
    if (_writing case final Future<void> writing) {
      await writing;
      if (_pending.isNotEmpty) await flush();
      return;
    }
    if (_pending.isEmpty) return;
    final lines = _pending.toList();
    _pending.clear();
    _pendingBytes = 0;
    _writing = _write(lines);
    await _writing;
    _writing = null;
  }

  Future<void> _write(List<String> lines) async {
    try {
      await directory.create(recursive: true);
      var file = File(path);
      var length = await file.exists() ? await file.length() : 0;
      final batch = BytesBuilder(copy: false);
      Future<void> append() async {
        if (batch.isNotEmpty) {
          await file.writeAsBytes(batch.takeBytes(), mode: FileMode.append);
        }
      }

      for (final line in lines) {
        final bytes = utf8.encode(line);
        if (bytes.length > maxFileBytes) continue;
        if (length + bytes.length > maxFileBytes) {
          await append();
          final oldest = File('$path.2');
          if (await oldest.exists()) await oldest.delete();
          final previous = File('$path.1');
          if (await previous.exists()) await previous.rename(oldest.path);
          if (await file.exists()) await file.rename(previous.path);
          file = File(path);
          length = 0;
        }
        batch.add(bytes);
        length += bytes.length;
      }
      await append();
    } on FileSystemException {
      // A full disk or unavailable log folder must not interrupt capture.
    }
  }
}
