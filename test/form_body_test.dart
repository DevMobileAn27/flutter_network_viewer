import 'dart:convert';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/services/form_body.dart';

void main() {
  const boundary = '--dio-boundary-AbC123';
  const mime = 'multipart/form-data; boundary="$boundary"';
  final file = [
    0,
    255,
    254,
    13,
    10,
    ...utf8.encode('--$boundary--not-a-boundary'),
    127,
  ];
  final bytes = <int>[
    ...utf8.encode(
      '--$boundary\r\nContent-Disposition: form-data; name="title"\r\n\r\nCà phê 😀\r\n',
    ),
    ...utf8.encode(
      '--$boundary\r\nContent-Disposition: form-data; name="tags[]"\r\n\r\none\r\n',
    ),
    ...utf8.encode(
      '--$boundary\r\nContent-Disposition: form-data; name="tags[]"\r\n\r\ntwo\r\n',
    ),
    ...utf8.encode(
      '--$boundary\r\nContent-Disposition: form-data; name="empty"\r\n\r\n\r\n',
    ),
    ...utf8.encode(
      '--$boundary\r\nContent-Disposition: form-data; name="attachment"; filename="photo; test.jpg"\r\nContent-Type: image/jpeg\r\n\r\n',
    ),
    ...file,
    ...utf8.encode('\r\n--$boundary--\r\n'),
  ];
  test(
    'binary multipart preserves Unicode, repeats, empty fields and file metadata',
    () {
      final form = parseFormBody({
        'body': base64Encode(bytes),
        'encoding': 'base64',
        'contentType': mime,
      });
      expect(form.incomplete, false);
      expect(form.parts.map((p) => p.name), [
        'title',
        'tags[]',
        'tags[]',
        'empty',
        'attachment',
      ]);
      expect(form.parts.first.value, 'Cà phê 😀');
      expect(form.parts[1].value, 'one');
      expect(form.parts[2].value, 'two');
      expect(form.parts[3].value, '');
      final part = form.parts.last;
      expect(part.isFile, true);
      expect(part.filename, 'photo; test.jpg');
      expect(part.contentType, 'image/jpeg');
      expect(part.byteLength, file.length);
      expect(
        part.value,
        isNull,
        reason: 'Never create text/base64 copies of file parts',
      );
    },
  );
  test('UTF-8 form and extended filenames preserve case and characters', () {
    final form = parseFormBody({
      'body':
          '--$boundary\r\nCONTENT-DISPOSITION: form-data; name="file"; filename="fallback"; filename*=UTF-8\'\'B%E1%BA%A3n%20v%E1%BA%BD.png\r\n\r\nabc\r\n--$boundary--\r\n',
      'contentType': mime,
    });
    expect(form.incomplete, false);
    expect(form.parts.single.filename, 'Bản vẽ.png');
    expect(form.parts.single.byteLength, 3);
  });
  test(
    'form URL encoding retains duplicate fields, plus, Unicode and empty values',
    () {
      final form = parseFormBody({
        'body': 'name=C%C3%A0+ph%C3%AA&tag=a&tag=b&empty=&flag',
        'contentType': 'application/x-www-form-urlencoded; charset=UTF-8',
      });
      expect(form.incomplete, false);
      expect(form.parts.map((p) => p.name), [
        'name',
        'tag',
        'tag',
        'empty',
        'flag',
      ]);
      expect(form.parts.map((p) => p.value), ['Cà phê', 'a', 'b', '', '']);
    },
  );
  test('truncated multipart retains captured parts and reports incomplete', () {
    final source =
        '--$boundary\r\nContent-Disposition: form-data; name="text"\r\n\r\npartial';
    final form = parseFormBody({'body': source, 'contentType': mime});
    expect(form.incomplete, true);
    expect(form.parts.single.value, 'partial');
  });
  test('invalid boundary, base64 and percent escapes never throw', () {
    for (final input in [
      {'body': 'raw', 'contentType': 'multipart/form-data'},
      {'body': '%%%', 'encoding': 'base64', 'contentType': mime},
      {'body': 'bad=%GG', 'contentType': 'application/x-www-form-urlencoded'},
    ]) {
      expect(parseFormBody(input).incomplete, true);
    }
  });
  test('content type detection accepts case-insensitive names and lists', () {
    final value = bodyContentType({
      'CONTENT-TYPE': [mime],
    });
    expect(isFormBody(value), true);
    expect(
      isFormBody('APPLICATION/X-WWW-FORM-URLENCODED; charset=utf-8'),
      true,
    );
    expect(isFormBody('application/json'), false);
  });
}
