import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:mcp_dart/mcp_dart.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';
import 'package:flutter_network_viewer/services/viewer_mcp_server.dart';
import 'package:flutter_network_viewer/services/viewer_server.dart';

Map<String, dynamic> fixture() =>
    jsonDecode(File('test/fixtures/sdk_envelope.json').readAsStringSync())
        as Map<String, dynamic>;

Future<Map<String, dynamic>> call(
  McpClient client,
  String tool, [
  Map<String, dynamic> arguments = const {},
]) async {
  final result = await client.callTool(
    CallToolRequest(name: tool, arguments: arguments),
    options: RequestOptions(timeout: const Duration(seconds: 5)),
  );
  expect(result.isError, isFalse, reason: result.content.toString());
  return result.structuredContent ??
      jsonDecode((result.content.single as TextContent).text)
          as Map<String, dynamic>;
}

void main() {
  for (final protocol in [McpProtocol.legacy, McpProtocol.stable]) {
    test(
      'HTTP MCP exposes live captures and five read-only tools ($protocol)',
      () async {
        final store = CaptureStore();
        final viewer = ViewerServer(store);
        final mcp = ViewerMcpServer(store, viewer);
        final client = McpClient(
          const Implementation(
            name: 'viewer-integration-test',
            version: '1.0.0',
          ),
          options: McpClientOptions(protocol: protocol),
        );
        addTearDown(() async {
          await client.close();
          await mcp.stop();
          mcp.dispose();
          await viewer.stop();
          viewer.dispose();
          store.dispose();
        });
        expect(mcp.running, isFalse);
        store.ingest(fixture());
        expect(store.network.length, 3);
        store.ingest({
          ...fixture(),
          'session': {...fixture()['session'] as Map, 'id': 'second-session'},
        });
        await mcp.configure(enabled: true, port: 0);
        expect(mcp.running, isTrue);
        await client.connect(
          StreamableHttpClientTransport(Uri.parse(mcp.endpoint)),
        );
        final tools = (await client.listTools()).tools;
        expect(
          tools.map((t) => t.name),
          unorderedEquals([
            'list_sessions',
            'list_requests',
            'get_request',
            'list_logs',
            'get_capture_summary',
          ]),
        );
        for (final tool in tools) {
          expect(tool.annotations!.readOnlyHint, isTrue);
          expect(tool.annotations!.destructiveHint, isFalse);
        }
        final sessions = await call(client, 'list_sessions');
        expect(sessions['total'], 2);
        expect((sessions['items'] as List).first['online'], isFalse);
        expect((sessions['items'] as List).last['online'], isFalse);
        final requests = await call(client, 'list_requests', {
          'session_id': 'ios-session-1',
          'limit': 1,
        });
        expect(requests['total'], 3);
        expect(requests['next_offset'], 1);
        expect((requests['items'] as List).single['request_id'], 'fetch-3');
        expect(
          (requests['items'] as List).single.containsKey('request'),
          isFalse,
        );
        final page = await call(client, 'list_requests', {
          'session_id': 'ios-session-1',
          'offset': 1,
          'limit': 1,
        });
        expect((page['items'] as List).single['request_id'], 'xhr-2');
        final filtered = await call(client, 'list_requests', {
          'session_id': 'second-session',
          'method': 'post',
          'url': '/ORDERS',
          'status_class': 2,
          'status_code': 201,
          'min_duration_ms': 100,
          'since': '2026-10-14T08:00:00Z',
          'until': '2026-10-14T08:00:01Z',
        });
        expect(filtered['total'], 1);
        expect(
          (filtered['items'] as List).single['session_id'],
          'second-session',
        );
        final failed = await call(client, 'list_requests', {'failed': true});
        expect(failed['total'], 2);
        final pending = await call(client, 'list_requests', {'pending': true});
        expect(pending['total'], 2);
        final details = await call(client, 'get_request', {
          'session_id': 'ios-session-1',
          'request_id': 'fetch-1',
        });
        expect(details['request']['headers']['authorization'], '[REDACTED]');
        expect(jsonDecode(details['response']['body']['text'] as String), {
          'id': 42,
          'status': 'created',
        });
        final bodyText = details['request']['body']['text'] as String;
        final bodyPage = await call(client, 'get_request', {
          'session_id': 'ios-session-1',
          'request_id': 'fetch-1',
          'body_offset': 5,
          'body_limit': 10,
        });
        expect(bodyPage['request']['body']['text'], bodyText.substring(5, 15));
        expect(bodyPage['request']['body']['next_offset'], 15);
        final noBody = await call(client, 'get_request', {
          'session_id': 'ios-session-1',
          'request_id': 'fetch-1',
          'include_body': false,
        });
        expect((noBody['response'] as Map).containsKey('body'), isFalse);
        final logs = await call(client, 'list_logs', {
          'session_id': 'second-session',
          'level': 'info',
          'query': 'COFFEE',
          'include_arguments': true,
          'text_limit': 8,
        });
        expect(logs['total'], 1);
        expect((logs['items'] as List).single['message']['text'], 'Order cr');
        expect((logs['items'] as List).single['arguments']['has_more'], isTrue);
        final nextLogText = await call(client, 'list_logs', {
          'log_id': (logs['items'] as List).single['id'],
          'text_offset': 8,
          'text_limit': 8,
        });
        expect(nextLogText['total'], 1);
        expect(
          (nextLogText['items'] as List).single['message']['text'],
          'eated {"',
        );
        final summary = await call(client, 'get_capture_summary', {
          'session_id': 'ios-session-1',
        });
        expect(summary['request_count'], 3);
        expect(summary['failed_count'], 1);
        expect(summary['pending_count'], 1);
        expect(summary['log_count'], 1);
        expect(summary['max_duration_ms'], 124);
        expect(
          (summary['slowest_requests'] as List).first['request_id'],
          'fetch-1',
        );
        for (final args in [
          {'session_id': 'missing', 'request_id': 'fetch-1'},
          {
            'session_id': 'ios-session-1',
            'request_id': 'fetch-1',
            'body_limit': 1000000,
          },
        ]) {
          final error = await client.callTool(
            CallToolRequest(name: 'get_request', arguments: args),
          );
          expect(error.isError, isTrue);
        }
        final badDate = await client.callTool(
          const CallToolRequest(
            name: 'list_requests',
            arguments: {'since': 'yesterday'},
          ),
        );
        expect(badDate.isError, isTrue);
        store.clear();
        final cleared = await call(client, 'list_requests');
        expect(cleared['total'], 0);
        expect(store.paused, isFalse);
        expect(viewer.running, isFalse);
      },
    );
  }

  test(
    'MCP stays off by default, handles conflicts, races and releases its port',
    () async {
      final store = CaptureStore();
      final viewer = ViewerServer(store);
      final mcp = ViewerMcpServer(store, viewer);
      final occupied = await ServerSocket.bind(InternetAddress.loopbackIPv4, 0);
      addTearDown(() async {
        await occupied.close();
        await mcp.stop();
        mcp.dispose();
        viewer.dispose();
        store.dispose();
      });
      await mcp.configure(enabled: false, port: occupied.port);
      expect(mcp.running, isFalse);
      expect(mcp.error, isNull);
      await mcp.configure(enabled: true, port: occupied.port);
      expect(mcp.running, isFalse);
      expect(mcp.error, isNotNull);
      expect(mcp.busy, isFalse);
      await mcp.configure(enabled: true, port: 0);
      expect(mcp.running, isTrue);
      final port = mcp.port;
      final off = mcp.stop();
      final on = mcp.configure(enabled: true, port: port);
      final finalOff = mcp.stop();
      await Future.wait([off, on, finalOff]);
      expect(mcp.running, isFalse);
      expect(mcp.busy, isFalse);
      final released = await ServerSocket.bind(
        InternetAddress.loopbackIPv4,
        port,
      );
      await released.close();
      final closing = ViewerMcpServer(store, viewer);
      final start = closing.configure(enabled: true, port: port);
      closing.dispose();
      await start;
      await Future<void>.delayed(Duration.zero);
      final afterDispose = await ServerSocket.bind(
        InternetAddress.loopbackIPv4,
        port,
      );
      await afterDispose.close();
    },
  );

  test(
    'HTTP rejects foreign origins, hosts, unknown routes and unsupported versions',
    () async {
      final store = CaptureStore();
      final viewer = ViewerServer(store);
      final mcp = ViewerMcpServer(store, viewer);
      final http = HttpClient();
      addTearDown(() async {
        http.close(force: true);
        await mcp.stop();
        mcp.dispose();
        viewer.dispose();
        store.dispose();
      });
      await mcp.configure(enabled: true, port: 0);
      final url = Uri.parse(mcp.endpoint);
      for (final scenario in [
        ({'Origin': 'https://untrusted.example'}, url, 403),
        ({'Host': 'untrusted.example'}, url, 403),
        (<String, String>{}, url.replace(path: '/unknown'), 404),
        ({'MCP-Protocol-Version': '2099-01-01'}, url, 400),
      ]) {
        final request = await http.postUrl(scenario.$2);
        scenario.$1.forEach(request.headers.set);
        request.headers.contentType = ContentType.json;
        request.headers.set('Accept', 'application/json, text/event-stream');
        request.write(
          jsonEncode({'jsonrpc': '2.0', 'id': 1, 'method': 'ping'}),
        );
        final response = await request.close();
        expect(response.statusCode, scenario.$3);
        await response.drain<void>();
      }
      final request = await http.postUrl(url);
      request.headers.contentType = ContentType.json;
      request.headers.set('Accept', 'application/json, text/event-stream');
      request.write(
        jsonEncode({
          'jsonrpc': '2.0',
          'id': 1,
          'method': 'initialize',
          'params': {
            'protocolVersion': '2025-03-26',
            'capabilities': {},
            'clientInfo': {'name': 'raw-http-test', 'version': '1'},
          },
        }),
      );
      final response = await request.close();
      expect(response.statusCode, 200);
      final body = jsonDecode(await utf8.decoder.bind(response).join()) as Map;
      expect(body['result']['protocolVersion'], '2025-03-26');
      expect(response.headers.value('mcp-session-id'), isNotEmpty);
    },
  );
}
