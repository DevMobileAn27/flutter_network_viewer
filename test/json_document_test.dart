import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/ui/json_document.dart';
import 'package:flutter_network_viewer/ui/json_find_controller.dart';

class _EncodeProbe {
  int encodes = 0;
  Object toJson() {
    encodes++;
    return {'ok': true};
  }
}

void main() {
  test('clear releases all parsed documents and resets the cache budget', () {
    final cache = JsonDocumentCache();
    final previous = cache.decode('{"secret":"captured body"}');
    cache.clear();
    expect(cache.length, 0);
    expect(cache.estimatedBytes, 0);
    expect(
      identical(cache.decode('{"secret":"captured body"}'), previous),
      isFalse,
    );
  });
  test('empty search does not serialize or index a body', () {
    final value = _EncodeProbe();
    final find = JsonFindController()..setValue(value);
    addTearDown(find.dispose);
    expect(value.encodes, 0);
    find.update(query: 'ok');
    expect(find.matches.length, 1);
    expect(value.encodes, 2); // formatted text and the search index
    find.update(query: 'true');
    expect(value.encodes, 2);
  });

  test(
    'cache reuses immutable source and evicts least recently read entries',
    () {
      final cache = JsonDocumentCache(maxEntries: 2);
      final first = cache.decode('{"a":1}');
      final second = cache.decode('{"a":2}');
      expect(identical(cache.decode('{"a":1}'), first), isTrue);
      cache.decode('{"a":3}');
      expect(cache.length, 2);
      expect(identical(cache.decode('{"a":1}'), first), isTrue);
      expect(identical(cache.decode('{"a":2}'), second), isFalse);
      expect(cache.decode('{"a":4}').value, {'a': 4});
    },
  );

  test('cache budget is bounded and oversized bodies are not retained', () {
    final cache = JsonDocumentCache(maxBytes: 10000);
    final a = cache.decode('{"a":1}');
    cache.decode('{"b":2}');
    cache.decode('{"c":3}');
    expect(cache.estimatedBytes, lessThanOrEqualTo(10000));
    expect(cache.length, 2);
    expect(identical(cache.decode('{"a":1}'), a), isFalse);
    final source = '"${'large' * 1000}"';
    final large = cache.decode(source);
    expect(identical(cache.decode(source), large), isFalse);
    expect(cache.estimatedBytes, lessThanOrEqualTo(10000));
  });
}
