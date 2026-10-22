import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';

import '../l10n/app_localizations.dart';
import 'json_document.dart';
import 'json_find_controller.dart';
import 'native_capture_list.dart';
import 'native_json_document.dart';
import 'viewer_theme.dart';

class NativeJson {
  static const channel = MethodChannel(
    'dev.quocan.flutterNetworkViewer/native_json',
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

/// One native document host survives mode and search changes. Only a new
/// document is prepared/transferred; folding, selection and scrolling are native.
class NativeJsonView extends StatefulWidget {
  const NativeJsonView({
    super.key,
    required this.document,
    required this.tree,
    required this.fallback,
    this.find,
    this.onFind,
    this.initiallyExpanded = true,
  });
  final JsonDocument document;
  final bool tree;
  final bool initiallyExpanded;
  final JsonFindController? find;
  final VoidCallback? onFind;
  final WidgetBuilder fallback;
  @override
  State<NativeJsonView> createState() => _NativeJsonViewState();
}

class _NativeJsonViewState extends State<NativeJsonView>
    with WidgetsBindingObserver {
  final _id = NativeJson._nextId++;
  Future<void> _pending = Future.value();
  bool _failed = false, _ready = false, _preparing = false, _scheduled = false;
  bool _resumed = true, _current = true;
  JsonDocument? _wanted;
  int _revision = 0;
  String? _config;
  List<JsonFindMatch>? _matches;
  Rect? _rect, _clip;
  final _boundsKey = GlobalKey();
  final _scrollPositions = <ScrollPosition>[];
  String? _frame;
  bool get _visible =>
      _ready &&
      !_failed &&
      _current &&
      _resumed &&
      (!NativeLists.overlayFallback || !NativeListOverlays.blocked.value);

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    NativeListOverlays.blocked.addListener(_overlay);
    widget.find?.addListener(_search);
    NativeJson._handlers[_id] = (call) {
      if (!mounted) return;
      if (call.method == 'find') widget.onFind?.call();
      if (call.method == 'next') {
        widget.find?.move((call.arguments as Map)['delta'] as int);
      }
    };
    _send('create', {});
    _prepare();
  }

  void _send(String method, Map<String, Object?> args) {
    _pending = _pending
        .then((_) async {
          if (_failed && method != 'dispose') return;
          await NativeJson.channel.invokeMethod<void>(method, {
            'id': _id,
            ...args,
          });
        })
        .catchError((Object error) {
          if (mounted && !_failed) {
            setState(() => _failed = true);
            unawaited(
              NativeJson.channel
                  .invokeMethod<void>('dispose', {'id': _id})
                  .catchError((_) {}),
            );
          }
        });
  }

  void _prepare() {
    _wanted = widget.document;
    _revision++;
    _ready = false;
    _send('clear', {});
    _sendFrame();
    if (!_preparing) unawaited(_prepareLatest());
  }

  Future<void> _prepareLatest() async {
    _preparing = true;
    while (mounted && _wanted != null && !_failed) {
      final document = _wanted!;
      final revision = _revision;
      _wanted = null;
      try {
        final data = await compute(prepareNativeJsonValue, document.value);
        if (!mounted || revision != _revision) continue;
        _send('document', {...data, 'expanded': widget.initiallyExpanded});
        _matches = null;
        _search();
        setState(() => _ready = true);
        _schedule();
        _sendFrame();
      } catch (_) {
        if (mounted && revision == _revision) {
          setState(() => _failed = true);
          _send('dispose', {});
        }
      }
    }
    _preparing = false;
  }

  void _search() {
    final find = widget.find;
    if (!identical(_matches, find?.matches)) {
      _matches = find?.matches;
      _send('matches', {
        'ranges': [
          for (final m in find?.matches ?? <JsonFindMatch>[]) [m.start, m.end],
        ],
      });
    }
    _send('active', {'index': find?.index ?? -1});
  }

  @override
  void didUpdateWidget(NativeJsonView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.find != widget.find) {
      oldWidget.find?.removeListener(_search);
      widget.find?.addListener(_search);
      _matches = null;
      _search();
    }
    if (!identical(oldWidget.document, widget.document)) _prepare();
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
    _schedule();
    _sendFrame();
  }

  void _scrollChanged() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) return;
      final render = _boundsKey.currentContext?.findRenderObject();
      if (render is _JsonBoundsRender) render.report();
    });
  }

  void _schedule() {
    if (_scheduled) return;
    _scheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _scheduled = false;
      if (!mounted) return;
      final p = context.palette;
      final config = <String, Object?>{
        'tree': widget.tree,
        'countPattern':
            Localizations.of<AppLocalizations>(
              context,
              AppLocalizations,
            )?.itemCount(314159).replaceFirst('314159', '{count}') ??
            '{count} items',
        'fontScale': MediaQuery.textScalerOf(context).scale(12) / 12,
        'palette': {
          'surface': p.surface.toARGB32(),
          'text': Theme.of(context).colorScheme.onSurface.toARGB32(),
          'muted': p.muted.toARGB32(),
          'line': p.line.toARGB32(),
          'key': p.info.toARGB32(),
          'accent': p.accent.toARGB32(),
          'string': p.string.toARGB32(),
          'bool': p.method.toARGB32(),
          'number': p.warning.toARGB32(),
          'selected': p.selectedRow.toARGB32(),
          'selectedText': p.onSelectedRow.toARGB32(),
        },
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
    _revision++;
    _wanted = null;
    for (final position in _scrollPositions) {
      position.removeListener(_scrollChanged);
    }
    _scrollPositions.clear();
    WidgetsBinding.instance.removeObserver(this);
    NativeListOverlays.blocked.removeListener(_overlay);
    widget.find?.removeListener(_search);
    NativeJson._handlers.remove(_id);
    _send('dispose', {});
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => _JsonBounds(
    key: _boundsKey,
    onBounds: (rect, clip) {
      _rect = rect;
      _clip = clip;
      _sendFrame();
    },
    child: Stack(
      fit: StackFit.expand,
      children: [
        ColoredBox(color: context.palette.surface),
        if (defaultTargetPlatform == TargetPlatform.macOS && !_failed)
          AppKitView(
            viewType: 'dev.quocan.flutterNetworkViewer/json_view',
            creationParams: {'id': _id},
            creationParamsCodec: const StandardMessageCodec(),
          ),
        if (_failed || (_ready && !_visible)) widget.fallback(context),
      ],
    ),
  );
}

class _JsonBounds extends SingleChildRenderObjectWidget {
  const _JsonBounds({super.key, required this.onBounds, required super.child});
  final void Function(Rect, Rect) onBounds;
  @override
  RenderObject createRenderObject(BuildContext context) =>
      _JsonBoundsRender(onBounds);
  @override
  void updateRenderObject(
    BuildContext context,
    _JsonBoundsRender renderObject,
  ) => renderObject.onBounds = onBounds;
}

class _JsonBoundsRender extends RenderProxyBox {
  _JsonBoundsRender(this.onBounds);
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
