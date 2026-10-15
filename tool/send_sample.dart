import 'dart:async';
import 'dart:convert';
import 'dart:developer' as developer;
import 'dart:io';

/// Real VM Service smoke fixture; no sample data is injected into the viewer.
/// dart --enable-vm-service=0 tool/send_sample.dart
/// Paste the printed VM Service / DevTools URL into Flutter Network Viewer.
Future<void> main() async {
  final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
  server.listen((request) async {
    await request.drain<void>();
    request.response.statusCode = request.uri.path == '/error' ? 500 : 201;
    request.response.headers.contentType = ContentType.json;
    request.response.write(
      jsonEncode({'id': 42, 'name': 'Cà phê', 'path': request.uri.path}),
    );
    await request.response.close();
  });
  final client = HttpClient();
  var sequence = 0;
  Timer.periodic(const Duration(seconds: 1), (_) async {
    final current = ++sequence;
    try {
      final request = await client.postUrl(
        Uri.parse(
          'http://127.0.0.1:${server.port}/${current % 3 == 0 ? 'error' : 'orders'}',
        ),
      );
      request.headers.contentType = ContentType.json;
      request.write(jsonEncode({'quantity': current}));
      final response = await request.close();
      await response.drain<void>();
      stdout.writeln('VM fixture request $current: ${response.statusCode}');
      developer.log(
        'VM fixture log $current',
        name: 'viewer.fixture',
        level: 900,
      );
    } catch (e) {
      stderr.writeln('VM fixture error: $e');
    }
  });
}
