import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';

import '../l10n/strings.dart';
import '../models/capture.dart';
import 'components.dart';
import 'capture_scope.dart';

/// Renders captured bytes only; opening an image never replays the HTTP request.
class NetworkBodySection extends StatefulWidget {
  const NetworkBodySection({
    super.key,
    required this.data,
    this.fillAvailable = false,
    this.showTitle = true,
  });
  final Json data;
  final bool fillAvailable;
  final bool showTitle;

  @override
  State<NetworkBodySection> createState() => _NetworkBodySectionState();
}

class _NetworkBodySectionState extends State<NetworkBodySection> {
  Uint8List? _bytes;
  bool _image = false;
  bool _raw = false;

  dynamic get _body => widget.data['body'] ?? widget.data['bodyPreview'];

  @override
  void initState() {
    super.initState();
    _decode();
  }

  @override
  void didUpdateWidget(NetworkBodySection oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.data['body'] != widget.data['body'] ||
        oldWidget.data['bodyPreview'] != widget.data['bodyPreview'] ||
        oldWidget.data['bodyEncoding'] != widget.data['bodyEncoding'] ||
        oldWidget.data['headers'] != widget.data['headers']) {
      _decode();
    }
  }

  void _decode() {
    _bytes = null;
    _raw = false;
    final headers = objectOf(widget.data['headers']);
    final contentType = headers.entries
        .where((entry) => entry.key.toLowerCase() == 'content-type')
        .map((entry) => '${entry.value}'.toLowerCase())
        .join();
    _image = contentType.contains('image/');
    if (_body is! String) return;
    try {
      var text = (_body as String).trim();
      if (widget.data['bodyEncoding'] == 'base64') {
        final captured = base64Decode(text.replaceAll(RegExp(r'\s'), ''));
        if (_hasImageSignature(captured)) {
          _bytes = captured;
        } else {
          if (_image) _bytes = captured;
          // The captured bytes can themselves contain a base64 response string.
          text = utf8.decode(captured);
        }
      }
      if (_bytes == null || !_hasImageSignature(_bytes!)) {
        if (text.startsWith('"')) {
          final decoded = jsonDecode(text);
          if (decoded is String) text = decoded.trim();
        }
        if (text.toLowerCase().startsWith('data:image/')) {
          _image = true;
          _bytes = UriData.parse(text).contentAsBytes();
        } else {
          final image = _decodeBase64Image(text);
          if (image != null) {
            _bytes = image;
          } else if (_image && _bytes == null) {
            _bytes = Uint8List.fromList(utf8.encode(text));
          }
        }
      }
    } on FormatException {
      // Malformed or truncated bodies remain available in the raw view.
    }
    final bytes = _bytes;
    if (bytes == null) return;
    _image = _image || _hasImageSignature(bytes);
    if (!_image) _bytes = null;
  }

  void _open() => showCaptureDialog(
    context: context,
    builder: (context) => _ImageDialog(bytes: _bytes!),
  );

  String _rawLabel(BuildContext context) {
    final size = _bytes?.length;
    if (size == null) return context.l10n.raw;
    final amount = size < 1024
        ? '$size B'
        : size < 1024 * 1024
        ? '${(size / 1024).toStringAsFixed(1).replaceFirst(RegExp(r'\.0$'), '')} KB'
        : '${(size / (1024 * 1024)).toStringAsFixed(1).replaceFirst(RegExp(r'\.0$'), '')} MB';
    return '${context.l10n.raw} ($amount)';
  }

  @override
  Widget build(BuildContext context) => DataSection(
    widget.data['bodyEncoding'] == 'base64'
        ? '${context.l10n.body} (base64)'
        : context.l10n.body,
    _body,
    body: true,
    showTitle: widget.showTitle,
    searchable: !_image && widget.data['bodyEncoding'] != 'base64',
    fillAvailable: widget.fillAvailable,
    preview: !_image
        ? null
        : LayoutBuilder(
            builder: (context, constraints) => SizedBox(
              height: _raw
                  ? null
                  : constraints.minHeight > 0
                  ? constraints.minHeight
                  : 368,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Row(
                    children: [
                      TextButton.icon(
                        key: const ValueKey('image-preview-toggle'),
                        onPressed: () => setState(() => _raw = !_raw),
                        icon: Icon(
                          _raw ? Icons.image_outlined : Icons.code,
                          size: 17,
                        ),
                        label: Text(
                          _raw ? context.l10n.imagePreview : _rawLabel(context),
                        ),
                      ),
                      const Spacer(),
                      if (_bytes != null && _bytes!.isNotEmpty)
                        IconButton(
                          key: const ValueKey('open-image'),
                          tooltip: context.l10n.openImage,
                          onPressed: _open,
                          icon: const Icon(Icons.open_in_full, size: 18),
                        ),
                    ],
                  ),
                  if (_raw)
                    SelectableText('$_body', style: mono)
                  else
                    Expanded(
                      child: Container(
                        key: const ValueKey('captured-image-preview'),
                        clipBehavior: Clip.antiAlias,
                        decoration: BoxDecoration(
                          color: context.palette.input,
                          borderRadius: BorderRadius.circular(8),
                          border: Border.all(color: context.palette.line),
                        ),
                        child: _bytes == null || _bytes!.isEmpty
                            ? const _ImageUnavailable()
                            : InkWell(
                                onTap: _open,
                                child: Image(
                                  image: ResizeImage(
                                    MemoryImage(_bytes!),
                                    width: 1600,
                                    policy: ResizeImagePolicy.fit,
                                  ),
                                  fit: BoxFit.contain,
                                  semanticLabel: context.l10n.imagePreview,
                                  errorBuilder: (_, _, _) =>
                                      const _ImageUnavailable(),
                                ),
                              ),
                      ),
                    ),
                ],
              ),
            ),
          ),
  );
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

Uint8List? _decodeBase64Image(String text) {
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
      return null;
    }
    prefixText.writeCharCode(code);
  }
  if (prefixText.length < 4) return null;
  try {
    final prefix = base64Decode(prefixText.toString());
    if (!_hasImageSignature(prefix)) return null;
    return base64Decode(text.replaceAll(RegExp(r'\s'), ''));
  } on FormatException {
    return null;
  }
}

class _ImageUnavailable extends StatelessWidget {
  const _ImageUnavailable();

  @override
  Widget build(BuildContext context) => Center(
    child: Padding(
      padding: const EdgeInsets.all(20),
      child: Text(
        context.l10n.imageUnavailable,
        textAlign: TextAlign.center,
        style: TextStyle(color: context.palette.muted),
      ),
    ),
  );
}

class _ImageDialog extends StatefulWidget {
  const _ImageDialog({required this.bytes});
  final Uint8List bytes;

  @override
  State<_ImageDialog> createState() => _ImageDialogState();
}

class _ImageDialogState extends State<_ImageDialog> {
  final _transform = TransformationController();

  void _zoom(double factor) {
    final current = _transform.value.getMaxScaleOnAxis();
    final next = (current * factor).clamp(.1, 10.0);
    // Scale around the center of the visible image area.
    final size = _viewport.currentContext?.size;
    final center = size == null ? Offset.zero : size.center(Offset.zero);
    final focal = _transform.toScene(center);
    _transform.value = Matrix4.identity()
      ..translateByDouble(center.dx, center.dy, 0, 1)
      ..scaleByDouble(next, next, 1, 1)
      ..translateByDouble(-focal.dx, -focal.dy, 0, 1);
  }

  final _viewport = GlobalKey();

  @override
  void dispose() {
    _transform.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Dialog(
    insetPadding: const EdgeInsets.all(24),
    child: SizedBox(
      width: MediaQuery.sizeOf(context).width * .9,
      height: MediaQuery.sizeOf(context).height * .9,
      child: Column(
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 8, 8),
            child: Row(
              children: [
                Expanded(child: Text(context.l10n.imagePreview)),
                IconButton(
                  tooltip: context.l10n.zoomOut,
                  onPressed: () => _zoom(1 / 1.25),
                  icon: const Icon(Icons.zoom_out),
                ),
                IconButton(
                  tooltip: context.l10n.zoomIn,
                  onPressed: () => _zoom(1.25),
                  icon: const Icon(Icons.zoom_in),
                ),
                IconButton(
                  tooltip: context.l10n.fitImage,
                  onPressed: () => _transform.value = Matrix4.identity(),
                  icon: const Icon(Icons.fit_screen),
                ),
                IconButton(
                  tooltip: context.l10n.gotIt,
                  onPressed: () => Navigator.pop(context),
                  icon: const Icon(Icons.close),
                ),
              ],
            ),
          ),
          Expanded(
            child: ClipRect(
              child: InteractiveViewer(
                key: _viewport,
                transformationController: _transform,
                minScale: .1,
                maxScale: 10,
                boundaryMargin: const EdgeInsets.all(1000),
                child: SizedBox.expand(
                  child: Image.memory(
                    widget.bytes,
                    fit: BoxFit.contain,
                    semanticLabel: context.l10n.imagePreview,
                    errorBuilder: (_, _, _) => const _ImageUnavailable(),
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    ),
  );
}
