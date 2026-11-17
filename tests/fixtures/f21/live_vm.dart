import 'dart:async';
import 'dart:convert';
import 'dart:developer' as developer;
import 'dart:io';

Future<void> main() async {
  final server = await HttpServer.bind(InternetAddress.loopbackIPv4, 0);
  server.listen((request) async {
    await utf8.decoder.bind(request).join();
    request.response.statusCode = 201;
    request.response.headers.contentType = ContentType.json;
    request.response.write(jsonEncode({'item': 'Cà phê 🖼️'}));
    await request.response.close();
  });
  final client = HttpClient();
  while (true) {
    final request = await client.postUrl(Uri.parse('http://127.0.0.1:${server.port}/coffee'));
    request.headers.contentType = ContentType.json;
    request.write(jsonEncode({'quantity': 2}));
    final response = await request.close();
    await response.drain<void>();
    print('F21 live stdout — tiếng Việt');
    developer.log('F21 live developer log', name: 'f21');
    await Future<void>.delayed(const Duration(milliseconds: 400));
  }
}
