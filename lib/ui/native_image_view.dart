import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';

import '../l10n/strings.dart';
import 'native_capture_list.dart';
import 'viewer_theme.dart';

class NativeImages {
  static const channel = MethodChannel(
    'dev.quocan.flutterNetworkViewer/native_image',
  );
  static bool available = false;
  static int _nextId = 0;
  static final _handlers = <int, void Function(MethodCall)>{};
  static Future<void> initialize() async {
    if (kIsWeb ||
        ![
          TargetPlatform.macOS,
          TargetPlatform.windows,
        ].contains(defaultTargetPlatform)) {
      return;
    }
    try {
      available = await channel.invokeMethod<int>('capabilities') == 1;
      channel.setMethodCallHandler((call) async {
        _handlers[(call.arguments as Map)['id']]?.call(call);
      });
    } on MissingPluginException {
      available = false;
    } on PlatformException {
      available = false;
    }
  }
}

/// The inline view and enlarged dialog share one decoded native resource.
/// There is no global image cache: the last owner explicitly releases it.
class NativeImageData {
  NativeImageData(this.bytes) {
    ready = NativeImages.channel.invokeMethod<void>('load', {
      'id': id,
      'bytes': bytes,
    });
    // Keep an observed future even if disposed before a host is mounted.
    unawaited(ready.catchError((Object _) {}));
  }
  final Uint8List bytes;
  final int id = NativeImages._nextId++;
  late final Future<void> ready;
  int _references = 1;
  void retain() {
    assert(_references > 0);
    _references++;
  }

  void release() {
    assert(_references > 0);
    if (--_references != 0) return;
    unawaited(
      ready
          .catchError((Object _) {})
          .then(
            (_) =>
                NativeImages.channel.invokeMethod<void>('release', {'id': id}),
          )
          .catchError((Object _) {}),
    );
  }
}

class NativeImageController extends ChangeNotifier {
  double? factor;
  void zoom(double value) {
    factor = value;
    notifyListeners();
  }

  void fit() {
    factor = null;
    notifyListeners();
  }
}

class NativeImageView extends StatefulWidget {
  const NativeImageView({
    super.key,
    required this.image,
    required this.fallback,
    this.controller,
    this.onOpen,
    this.onClose,
    this.interactive = false,
    this.modal = false,
  });
  final NativeImageData image;
  final WidgetBuilder fallback;
  final NativeImageController? controller;
  final VoidCallback? onOpen;
  final VoidCallback? onClose;
  final bool interactive;
  final bool modal;
  @override
  State<NativeImageView> createState() => _NativeImageViewState();
}

class _NativeImageViewState extends State<NativeImageView>
    with WidgetsBindingObserver {
  final _id = NativeImages._nextId++;
  final _boundsKey = GlobalKey();
  final _scrollPositions = <ScrollPosition>[];
  Future<void> _pending = Future.value();
  bool _failed = false, _resumed = true, _current = true, _scheduled = false;
  bool _created = false;
  late int _overlayBaseline;
  Rect? _rect, _clip;
  String? _frame, _config;
  Animation<double>? _routeAnimation;
  bool get _visible =>
      !_failed &&
      _created &&
      _current &&
      _resumed &&
      (!widget.modal ||
          _routeAnimation == null ||
          _routeAnimation!.status == AnimationStatus.completed) &&
      (!NativeLists.overlayFallback ||
          NativeListOverlays.depth.value <= _overlayBaseline);

  @override
  void initState() {
    super.initState();
    _overlayBaseline = widget.modal ? NativeListOverlays.depth.value : 0;
    WidgetsBinding.instance.addObserver(this);
    NativeListOverlays.depth.addListener(_overlay);
    widget.controller?.addListener(_control);
    widget.image.retain();
    NativeImages._handlers[_id] = (call) {
      if (!mounted) return;
      if (call.method == 'open') widget.onOpen?.call();
      if (call.method == 'close') widget.onClose?.call();
      if (call.method == 'error') _fail();
    };
    _create();
  }

  void _create() {
    final image = widget.image;
    _pending = _pending
        .then((_) => image.ready)
        .then<void>((_) async {
          if (!mounted) return;
          await NativeImages.channel.invokeMethod<void>('create', {
            'id': _id,
            'image': image.id,
          });
          if (!mounted) return;
          setState(() => _created = true);
          _sendFrame();
        })
        .catchError((Object _) => _fail());
  }

  void _fail() {
    if (!mounted || _failed) return;
    setState(() => _failed = true);
    _send('dispose', {});
  }

  void _send(String method, Map<String, Object?> args) {
    _pending = _pending
        .then<void>((_) async {
          if (_failed && method != 'dispose') return;
          await NativeImages.channel.invokeMethod<void>(method, {
            'id': _id,
            ...args,
          });
        })
        .catchError((Object _) {
          if (method != 'dispose') _fail();
        });
  }

  void _control() => _send('control', {'factor': widget.controller?.factor});

  @override
  void didUpdateWidget(NativeImageView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) {
      oldWidget.controller?.removeListener(_control);
      widget.controller?.addListener(_control);
    }
    if (oldWidget.image != widget.image) {
      widget.image.retain();
      oldWidget.image.release();
      _failed = false;
      final image = widget.image;
      _pending = _pending
          .then((_) => image.ready)
          .catchError((Object _) => _fail());
      _send('bind', {'image': image.id});
    }
    _schedule();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    for (final position in _scrollPositions) {
      position.removeListener(_scrollChanged);
    }
    _scrollPositions.clear();
    context.visitAncestorElements((element) {
      if (element is StatefulElement && element.state is ScrollableState) {
        final position = (element.state as ScrollableState).position;
        if (!_scrollPositions.contains(position)) {
          _scrollPositions.add(position);
          position.addListener(_scrollChanged);
        }
      }
      return true;
    });
    _current = ModalRoute.isCurrentOf(context) ?? true;
    final animation = ModalRoute.of(context)?.animation;
    if (animation != _routeAnimation) {
      _routeAnimation?.removeStatusListener(_routeChanged);
      _routeAnimation = animation;
      animation?.addStatusListener(_routeChanged);
    }
    _schedule();
    _sendFrame();
  }

  void _routeChanged(AnimationStatus _) => _sendFrame();
  void _scrollChanged() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      final render = _boundsKey.currentContext?.findRenderObject();
      if (render is _ImageBoundsRender) render.report();
    });
  }

  void _schedule() {
    if (_scheduled) return;
    _scheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _scheduled = false;
      if (!mounted) return;
      final config = <String, Object?>{
        'interactive': widget.interactive,
        'background': context.palette.input.toARGB32(),
        'label': context.l10n.imagePreview,
        'surface': context.palette.surface.toARGB32(),
        'line': context.palette.line.toARGB32(),
        'text': Theme.of(context).colorScheme.onSurface.toARGB32(),
        'tools': [
          context.l10n.zoomOut,
          context.l10n.zoomIn,
          context.l10n.fitImage,
          context.l10n.gotIt,
        ],
      };
      if (_config != config.toString()) {
        _config = config.toString();
        _send('configure', config);
      }
    });
  }

  void _overlay() {
    if (mounted && NativeLists.overlayFallback) {
      _sendFrame();
      setState(() {});
    }
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _resumed =
        state != AppLifecycleState.hidden &&
        state != AppLifecycleState.detached;
    _sendFrame();
  }

  void _sendFrame() {
    if (!mounted || _rect == null || _clip == null) return;
    final r = _rect!, clip = _clip!;
    final args = <String, Object?>{
      'x': r.left,
      'y': r.top,
      'width': r.width,
      'height': r.height,
      'clipX': clip.left - r.left,
      'clipY': clip.top - r.top,
      'clipWidth': clip.width,
      'clipHeight': clip.height,
      'scale': View.of(context).devicePixelRatio,
      'visible': _visible && !clip.isEmpty,
    };
    if (_frame == args.toString()) return;
    _frame = args.toString();
    _send('frame', args);
  }

  @override
  void dispose() {
    for (final position in _scrollPositions) {
      position.removeListener(_scrollChanged);
    }
    _routeAnimation?.removeStatusListener(_routeChanged);
    WidgetsBinding.instance.removeObserver(this);
    NativeListOverlays.depth.removeListener(_overlay);
    widget.controller?.removeListener(_control);
    NativeImages._handlers.remove(_id);
    _send('dispose', {});
    widget.image.release();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => _ImageBounds(
    key: _boundsKey,
    onBounds: (rect, clip) {
      _rect = rect;
      _clip = clip;
      _sendFrame();
    },
    child: Stack(
      fit: StackFit.expand,
      children: [
        ColoredBox(color: context.palette.input),
        if (defaultTargetPlatform == TargetPlatform.macOS && !_failed)
          AppKitView(
            viewType: 'dev.quocan.flutterNetworkViewer/image_view',
            creationParams: {'id': _id},
            creationParamsCodec: const StandardMessageCodec(),
          ),
        if (_failed) widget.fallback(context),
        // Win32 child windows stay hidden under overlays; do not decode a second
        // Flutter image merely to fill an obscured viewport.
      ],
    ),
  );
}

class _ImageBounds extends SingleChildRenderObjectWidget {
  const _ImageBounds({super.key, required this.onBounds, required super.child});
  final void Function(Rect, Rect) onBounds;
  @override
  RenderObject createRenderObject(BuildContext context) =>
      _ImageBoundsRender(onBounds);
  @override
  void updateRenderObject(
    BuildContext context,
    _ImageBoundsRender renderObject,
  ) => renderObject.onBounds = onBounds;
}

class _ImageBoundsRender extends RenderProxyBox {
  _ImageBoundsRender(this.onBounds);
  void Function(Rect, Rect) onBounds;
  @override
  void paint(PaintingContext context, Offset offset) {
    super.paint(context, offset);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (attached) report();
    });
  }

  void report() {
    final rect = localToGlobal(Offset.zero) & size;
    var clip = rect;
    RenderObject? ancestor = parent;
    while (ancestor != null) {
      if (ancestor is RenderAbstractViewport && ancestor is RenderBox) {
        final viewport = ancestor as RenderBox;
        clip = clip.intersect(
          viewport.localToGlobal(Offset.zero) & viewport.size,
        );
      }
      ancestor = ancestor.parent;
    }
    final visible = clip.isEmpty
        ? Rect.fromLTWH(rect.left, rect.top, 0, 0)
        : clip;
    onBounds(rect, visible);
  }
}
