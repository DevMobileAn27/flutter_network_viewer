import 'dart:convert';
import 'dart:io';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/services/viewer_diagnostics.dart';

void main() {
  test(
    'diagnostics persist structured events and rotate within a fixed budget',
    () async {
      final directory = await Directory.systemTemp.createTemp(
        'viewer-diagnostics-test-',
      );
      addTearDown(() => directory.delete(recursive: true));
      final log = ViewerDiagnostics(directory, maxFileBytes: 300);
      for (var i = 0; i < 20; i++) {
        log.record('rpc_failed', {'method': 'getVM', 'sequence': i});
      }
      await log.flush();
      final files = await directory
          .list()
          .where((file) => file is File)
          .cast<File>()
          .toList();
      expect(files, hasLength(3));
      for (final file in files) {
        expect(await file.length(), lessThanOrEqualTo(300));
        final records = (await file.readAsLines()).map(
          (line) => jsonDecode(line) as Map,
        );
        expect(
          records.every(
            (record) => record['time'] != null && record['pid'] == pid,
          ),
          true,
        );
      }
      expect(
        (jsonDecode((await File(log.path).readAsLines()).last)
            as Map)['sequence'],
        19,
      );
    },
  );

  test('unavailable log directory never breaks capture', () async {
    final directory = await Directory.systemTemp.createTemp(
      'viewer-diagnostics-test-',
    );
    addTearDown(() => directory.delete(recursive: true));
    final blocker = File('${directory.path}/file');
    await blocker.writeAsString('not a directory');
    final log = ViewerDiagnostics(Directory('${blocker.path}/nested'));
    log.record('connection_attempt');
    await log.flush();
    expect(await blocker.readAsString(), 'not a directory');
  });
}
