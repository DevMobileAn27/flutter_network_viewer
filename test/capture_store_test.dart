import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/models/capture.dart';
import 'package:flutter_network_viewer/services/capture_store.dart';

Json fixture() => objectOf(
  jsonDecode(File('test/fixtures/sdk_envelope.json').readAsStringSync()),
);

void main() {
  test(
    'SDK batch merges fetch/XHR phases and preserves full bodies and args',
    () {
      final store = CaptureStore();
      addTearDown(store.dispose);
      store.ingest(fixture());
      expect(store.sessions.values.single.name, 'Example Shop');
      expect(store.network.length, 3);
      final request = store.network.values.first;
      expect(request.status, 201);
      expect(request.duration, 124);
      expect(request.request['body'], contains('quantity'));
      expect(request.response['body'], contains('created'));
      expect(request.curl, contains('--data-raw'));
      expect(store.network.values.elementAt(1).failed, isTrue);
      expect(store.network.values.last.pending, isTrue);
      expect(
        store.logs.single.event['args'][1]['value']['customer']['name'],
        'An',
      );
      expect(jsonDecode(jsonEncode(store.export()))['network'].length, 3);
    },
  );

  test(
    'same request IDs from two devices do not collide; replay is idempotent',
    () {
      final store = CaptureStore();
      addTearDown(store.dispose);
      final first = fixture();
      store.ingest(first);
      store.ingest(first);
      expect(store.network.length, 3);
      final second = fixture();
      second['session']['id'] = 'android-session';
      second['session']['platform'] = 'android';
      second['sdk']['name'] = 'rnv_network_sdk_android';
      store.ingest(second);
      expect(store.network.length, 6);
      expect(store.sessions.length, 2);
    },
  );

  test('response may arrive before its request in a later batch', () {
    final store = CaptureStore();
    addTearDown(store.dispose);
    final batch = fixture();
    final events = batch['events'] as List;
    store.ingest({
      ...batch,
      'events': [events[1]],
    });
    expect(store.network.values.single.status, 201);
    store.ingest({
      ...batch,
      'events': [events[0]],
    });
    expect(store.network.values.single.url, contains('/orders'));
    expect(store.network.values.single.pending, isFalse);
  });

  test(
    'pause discards events, clear removes captures, resume accepts events',
    () {
      final store = CaptureStore();
      addTearDown(store.dispose);
      store.togglePause();
      store.ingest(fixture());
      expect(store.network, isEmpty);
      expect(store.ignoredWhilePaused, 6);
      store.togglePause();
      store.ingest(fixture());
      store.clear();
      expect(store.network, isEmpty);
      expect(store.logs, isEmpty);
      expect(store.sessions, isNotEmpty);
      store.ingest(fixture());
      expect(store.network.length, 3);
    },
  );

  test(
    'retention bounds records and bytes even when full bodies are large',
    () {
      final store = CaptureStore(maxRecords: 2);
      addTearDown(store.dispose);
      store.ingest(fixture());
      expect(store.logs.length + store.network.length, 2);
      expect(store.evicted, 2);
      final small = CaptureStore(maxBytes: 20);
      addTearDown(small.dispose);
      small.ingest(fixture());
      expect(small.network, isEmpty);
      expect(small.logs, isEmpty);
      small.clear();
      small.ingest(fixture());
      expect(small.evicted, greaterThan(0));
    },
  );

  test(
    'unsupported envelopes fail; malformed events do not poison valid events',
    () {
      final store = CaptureStore();
      addTearDown(store.dispose);
      expect(() => store.ingest([]), throwsFormatException);
      expect(() => store.ingest({'events': []}), throwsFormatException);
      expect(
        () => store.ingest({
          ...fixture(),
          'sdk': {'schemaVersion': 2},
        }),
        throwsFormatException,
      );
      final batch = fixture();
      (batch['events'] as List).insertAll(0, [
        null,
        {
          'requestId': 'bad',
          'phase': 'response',
          'response': {'statusCode': 'bad'},
        },
        {'source': 'future'},
      ]);
      store.ingest(batch);
      expect(store.rejected, 3);
      expect(store.network.length, 3);
    },
  );

  test('cURL uses shell-safe quotes for captured text', () {
    final record = NetworkRecord('s', 'r')
      ..apply({
        'phase': 'request',
        'request': {
          'method': 'POST',
          'url': "https://example.com/a'b",
          'headers': {'x-test': "a'b"},
          'body': "a'b",
        },
      });
    expect(record.curl, contains("'a'\\''b'"));
  });
}
