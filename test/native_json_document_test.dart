import 'dart:convert';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/ui/json_document.dart';
import 'package:flutter_network_viewer/ui/native_json_document.dart';

void main() {
  test(
    'native text offsets and token mappings preserve Unicode and escaped keys',
    () {
      final value = {
        'Tiếng 😀\n"': {'color': '#f00', 'text': '😀hello'},
        'list': [null, true, 2.5],
      };
      final data = prepareNativeJson(jsonEncode(value));
      final doc = JsonDocument.fromValue(value);
      expect(data['text'], doc.text);
      final code = data['code'] as List;
      for (final row in code.cast<Map>()) {
        expect(
          doc.text.substring(row['start'] as int, row['end'] as int),
          row['text'],
        );
      }
      final tree = (data['tree'] as List).cast<Map>();
      for (final row in tree) {
        for (final mapping in row['maps'] as List) {
          final m = (mapping as List).cast<int>();
          expect(
            (row['text'] as String).substring(m[0], m[0] + m[1]),
            doc.text.substring(m[2], m[2] + m[1]),
          );
        }
      }
      expect(tree.first['fold'], tree.length - 1);
      expect(
        tree.firstWhere(
          (r) => (r['text'] as String).contains('#f00'),
        )['swatch'],
        0xffff0000,
      );
    },
  );
  test(
    'all JSON scalar and empty collection documents are native compatible',
    () {
      for (final source in ['null', 'true', '2.5', '"😀"', '{}', '[]']) {
        final data = prepareNativeJson(source);
        expect((data['code'] as List).length, 1);
        expect((data['tree'] as List).length, 1);
        expect(((data['tree'] as List).first as Map)['fold'], 0);
        expect(jsonDecode(data['text'] as String), jsonDecode(source));
      }
    },
  );
  test(
    'Tree retains unquoted keys, array indexes and branch continuation metadata',
    () {
      final data = prepareNativeJsonValue({
        'group': {
          'child': {'color': '#f00'},
          'lastChild': 1,
        },
        'rows': [true],
        'tail': {},
      });
      final rows = (data['tree'] as List).cast<Map>();
      expect(rows.map((r) => r['text']), [
        '{}',
        'group: {}',
        'child: {}',
        'color: "#f00"',
        'lastChild: 1',
        'rows: []',
        '[0]: true',
        'tail: {}',
      ]);
      expect(rows[1]['last'], false);
      expect(rows[2]['branches'], [true]);
      expect(rows[3]['branches'], [true, true]);
      expect(rows[4]['branches'], [true]);
      expect(rows[4]['last'], true);
      expect(rows.last['last'], true);
      expect(rows.first['count'], 3);
      expect(rows.last.containsKey('count'), false);
    },
  );

  test('native width reserves room for wide Unicode text', () {
    final data = prepareNativeJson(jsonEncode({'text': '漢字😀'}));
    final row = (data['code'] as List).cast<Map>().firstWhere(
      (r) => (r['text'] as String).contains('漢字'),
    );
    expect(row['columns'] as int, greaterThan((row['text'] as String).length));
  });
  test(
    'large tree indexes include folded descendants without building widgets',
    () {
      final data = prepareNativeJson(
        jsonEncode({
          'rows': List.generate(5000, (i) => {'id': i}),
        }),
      );
      final tree = (data['tree'] as List).cast<Map>();
      expect(tree.length, 10002);
      expect(tree[1]['fold'], 10001);
      expect(tree[2]['fold'], 3);
      expect(tree.last['depth'], 3);
    },
  );
}
