import 'dart:convert';
import 'dart:typed_data';

/// Transport/base64 normalization runs in an isolate on native desktop hosts.
/// Pixel decoding is left to ImageIO / WIC, never Flutter's image cache.
class ImageBodyData {
  const ImageBodyData(this.image, this.bytes);
  final bool image;
  final Uint8List? bytes;
}

bool mayContainImageBody(Map<String, Object?> data) {
  if (data['body'] is! String) return false;
  if (data['image'] == true || data['base64'] == true) return true;
  final body = data['body'] as String;
  var prefix = body.substring(0, body.length.clamp(0, 128)).trimLeft();
  if (prefix.startsWith('"')) prefix = prefix.substring(1).trimLeft();
  if (prefix.toLowerCase().startsWith('data:image/')) return true;
  return _base64ImagePrefix(prefix);
}

ImageBodyData decodeImageBody(Map<String, Object?> data) {
  var image = data['image'] == true;
  Uint8List? bytes;
  if (data['body'] is! String) return ImageBodyData(image, null);
  try {
    var text = (data['body'] as String).trim();
    if (data['base64'] == true) {
      final captured = base64Decode(text.replaceAll(RegExp(r'\s'), ''));
      if (_hasImageSignature(captured)) {
        bytes = captured;
      } else {
        if (image) bytes = captured;
        text = utf8.decode(captured);
      }
    }
    if (bytes == null || !_hasImageSignature(bytes)) {
      if (text.startsWith('"')) {
        final decoded = jsonDecode(text);
        if (decoded is String) text = decoded.trim();
      }
      if (text.toLowerCase().startsWith('data:image/')) {
        image = true;
        bytes = UriData.parse(text).contentAsBytes();
      } else {
        final decoded = _decodeBase64Image(text);
        if (decoded != null) {
          bytes = decoded;
        } else if (image && bytes == null) {
          bytes = Uint8List.fromList(utf8.encode(text));
        }
      }
    }
  } on FormatException {
    // Corrupt captured bodies remain available through Raw.
  }
  if (bytes != null) image = image || _hasImageSignature(bytes);
  return ImageBodyData(image, image ? bytes : null);
}

bool _hasImageSignature(Uint8List bytes) {
  bool starts(List<int> signature, [int offset = 0]) =>
      bytes.length >= offset + signature.length &&
      List.generate(
        signature.length,
        (i) => bytes[offset + i] == signature[i],
      ).every((match) => match);
  return starts([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]) ||
      starts([0xff, 0xd8, 0xff]) ||
      starts(ascii.encode('GIF87a')) ||
      starts(ascii.encode('GIF89a')) ||
      starts(ascii.encode('BM')) ||
      (starts(ascii.encode('RIFF')) && starts(ascii.encode('WEBP'), 8));
}

bool _base64ImagePrefix(String text) {
  // Ordinary JSON/text should not allocate a whitespace-stripped copy merely
  // to discover that it is not an image. Only inspect a bounded prefix first.
  final prefixText = StringBuffer();
  for (var i = 0; i < text.length && prefixText.length < 64; i++) {
    final code = text.codeUnitAt(i);
    if (code == 32 || code == 9 || code == 10 || code == 13) continue;
    if (!(code >= 65 && code <= 90 ||
        code >= 97 && code <= 122 ||
        code >= 48 && code <= 57 ||
        code == 43 ||
        code == 47 ||
        code == 45 ||
        code == 95 ||
        code == 61)) {
      return false;
    }
    prefixText.writeCharCode(code);
  }
  if (prefixText.length < 4) return false;
  try {
    final prefix = base64Decode(prefixText.toString());
    return _hasImageSignature(prefix);
  } on FormatException {
    return false;
  }
}

Uint8List? _decodeBase64Image(String text) {
  if (!_base64ImagePrefix(text)) return null;
  try {
    return base64Decode(text.replaceAll(RegExp(r'\s'), ''));
  } on FormatException {
    return null;
  }
}
