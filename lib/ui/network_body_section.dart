import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/foundation.dart';

import '../l10n/strings.dart';
import '../models/capture.dart';
import 'components.dart';
import 'capture_scope.dart';
import 'image_body_data.dart';
import 'native_image_view.dart';

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
  bool _decoding = false;
  int _revision = 0;
  bool _preparing = false;
  Map<String, Object?>? _wanted;
  NativeImageData? _native;

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
        _imageMime(oldWidget.data) != _imageMime(widget.data)) {
      _decode();
    }
  }

  bool _imageMime(Json data) => objectOf(data['headers']).entries.any(
    (e) =>
        e.key.toLowerCase() == 'content-type' &&
        '${e.value}'.toLowerCase().contains('image/'),
  );

  void _decode() {
    ++_revision;
    _native?.release();
    _native = null;
    _bytes = null;
    _raw = false;
    _image = _imageMime(widget.data);
    final args = <String, Object?>{
      'body': _body,
      'image': _image,
      'base64': widget.data['bodyEncoding'] == 'base64',
    };
    _decoding = NativeImages.available && mayContainImageBody(args);
    if (_decoding) {
      _wanted = args;
      if (!_preparing) unawaited(_prepareLatest());
    } else {
      _wanted = null;
      _applyImage(decodeImageBody(args));
    }
  }

  Future<void> _prepareLatest() async {
    _preparing = true;
    while (mounted && _wanted != null) {
      final args = _wanted!;
      final revision = _revision;
      _wanted = null;
      try {
        final result = await compute(decodeImageBody, args);
        if (mounted && revision == _revision) {
          setState(() => _applyImage(result));
        }
      } catch (_) {
        if (mounted && revision == _revision) {
          setState(() => _decoding = false);
        }
      }
    }
    _preparing = false;
  }

  void _applyImage(ImageBodyData data) {
    _decoding = false;
    _image = data.image;
    _bytes = data.bytes;
    if (NativeImages.available && _bytes != null && _bytes!.isNotEmpty) {
      _native = NativeImageData(_bytes!);
    }
  }

  @override
  void dispose() {
    _revision++;
    _wanted = null;
    _native?.release();
    super.dispose();
  }

  void _open() {
    final bytes = _bytes;
    if (bytes == null) return;
    final native = _native;
    native?.retain();
    unawaited(
      showCaptureDialog(
        context: context,
        builder: (context) => _ImageDialog(bytes: bytes, native: native),
      ).whenComplete(() => native?.release()),
    );
  }

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
    preview: !_image && !_decoding
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
                        child: _decoding
                            ? const Center(child: CircularProgressIndicator())
                            : _bytes == null || _bytes!.isEmpty
                            ? const _ImageUnavailable()
                            : _native != null
                            ? NativeImageView(
                                image: _native!,
                                onOpen: _open,
                                fallback: (_) => _fallbackImage(),
                              )
                            : _fallbackImage(),
                      ),
                    ),
                ],
              ),
            ),
          ),
  );

  Widget _fallbackImage() => InkWell(
    onTap: _open,
    child: Image(
      image: ResizeImage(
        MemoryImage(_bytes!),
        width: 1600,
        policy: ResizeImagePolicy.fit,
      ),
      fit: BoxFit.contain,
      semanticLabel: context.l10n.imagePreview,
      errorBuilder: (_, _, _) => const _ImageUnavailable(),
    ),
  );
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
  const _ImageDialog({required this.bytes, this.native});
  final NativeImageData? native;
  final Uint8List bytes;

  @override
  State<_ImageDialog> createState() => _ImageDialogState();
}

class _ImageDialogState extends State<_ImageDialog> {
  final _transform = TransformationController();
  final _nativeController = NativeImageController();

  void _zoom(double factor) {
    _nativeController.zoom(factor);
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
    _nativeController.dispose();
    _transform.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => Dialog(
    insetPadding: const EdgeInsets.all(24),
    clipBehavior: Clip.antiAlias,
    child: SizedBox(
      width: MediaQuery.sizeOf(context).width * .9,
      height: MediaQuery.sizeOf(context).height * .9,
      child: widget.native != null
          ? NativeImageView(
              image: widget.native!,
              controller: _nativeController,
              interactive: true,
              modal: true,
              onClose: () => Navigator.pop(context),
              fallback: (_) => _fallbackContent(),
            )
          : _fallbackContent(),
    ),
  );

  Widget _fallbackContent() => Stack(
    fit: StackFit.expand,
    children: [
      ClipRect(child: _fallbackViewer()),
      Positioned(
        right: 16,
        bottom: 16,
        child: Material(
          color: context.palette.surface,
          elevation: 6,
          borderRadius: BorderRadius.circular(10),
          child: Padding(
            padding: const EdgeInsets.all(4),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
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
                  onPressed: () {
                    _nativeController.fit();
                    _transform.value = Matrix4.identity();
                  },
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
        ),
      ),
    ],
  );

  Widget _fallbackViewer() => InteractiveViewer(
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
  );
}
