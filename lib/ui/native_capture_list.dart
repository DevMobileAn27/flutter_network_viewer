import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';

import 'viewer_theme.dart';

/// The native hosts own scrolling, text layout, painting and row hit testing.
/// Only summary fields cross this channel; request bodies remain in Dart.
class NativeLists {
  static const channel = MethodChannel(
    'dev.quocan.flutterNetworkViewer/native_lists',
  );
  static bool available = false;
  // AppKitView participates in Flutter compositing. Win32 child HWNDs need a
  // Flutter fallback while a Flutter overlay is above the native list.
  static bool overlayFallback = false;
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
      overlayFallback = defaultTargetPlatform == TargetPlatform.windows;
      channel.setMethodCallHandler((call) async {
        final args = call.arguments as Map;
        _handlers[args['id']]?.call(call);
      });
    } on MissingPluginException {
      available = false;
    } on PlatformException {
      available = false;
    }
  }
}

/// Child native windows cannot overlap Flutter menus/dialogs. While one is
/// open, show the existing Flutter list underneath it, with the same selection.
class NativeListOverlays {
  static final blocked = ValueNotifier<bool>(false);
  static int _count = 0;
  static final depth = ValueNotifier<int>(0);

  static VoidCallback acquire() {
    _count++;
    depth.value = _count;
    blocked.value = true;
    var released = false;
    return () {
      if (released) return;
      released = true;
      _count--;
      depth.value = _count;
      blocked.value = _count > 0;
    };
  }
}

class NativeListRouteObserver extends NavigatorObserver {
  final _releases = <Route<dynamic>, VoidCallback>{};

  void dispose() {
    for (final release in _releases.values) {
      release();
    }
    _releases.clear();
  }

  @override
  void didPush(Route<dynamic> route, Route<dynamic>? previousRoute) {
    if (previousRoute != null) _releases[route] = NativeListOverlays.acquire();
  }

  @override
  void didPop(Route<dynamic> route, Route<dynamic>? previousRoute) {
    final release = _releases.remove(route);
    if (route is TransitionRoute<dynamic>) {
      unawaited(route.completed.then((_) => release?.call()));
    } else {
      release?.call();
    }
  }

  @override
  void didRemove(Route<dynamic> route, Route<dynamic>? previousRoute) =>
      _releases.remove(route)?.call();

  @override
  void didReplace({Route<dynamic>? newRoute, Route<dynamic>? oldRoute}) {
    final release = _releases.remove(oldRoute);
    if (release != null && newRoute != null) {
      _releases[newRoute] = release;
    } else {
      release?.call();
    }
  }
}

/// A diff based on stable keys and immutable record-event identities. Selection
/// has a separate message, so opening detail never serializes all rows again.
class NativeListDiff<T> {
  final _versions = <String, Object>{};
  final _records = <String, T>{};
  List<String> _order = [];
  T? record(String key) => _records[key];

  Map<String, Object?>? update(
    List<T> records,
    String Function(T) keyOf,
    Object Function(T) versionOf,
    Map<String, Object?> Function(T) rowOf,
  ) {
    final order = <String>[];
    final changed = <Map<String, Object?>>[];
    for (final record in records) {
      final key = keyOf(record);
      _records[key] = record;
      order.add(key);
      final version = versionOf(record);
      if (_versions[key] != version) {
        _versions[key] = version;
        changed.add({'key': key, ...rowOf(record)});
      }
    }
    final retained = order.toSet();
    final removed = _versions.keys
        .where((key) => !retained.contains(key))
        .toList();
    for (final key in removed) {
      _versions.remove(key);
      _records.remove(key);
    }
    final reordered = !listEquals(order, _order);
    _order = order;
    if (!reordered && changed.isEmpty && removed.isEmpty) return null;
    return {'rows': changed, 'removed': removed, if (reordered) 'order': order};
  }
}

class NativeCaptureList<T> extends StatefulWidget {
  const NativeCaptureList({
    super.key,
    required this.kind,
    required this.records,
    required this.keyOf,
    required this.versionOf,
    required this.rowOf,
    required this.selection,
    required this.selectedKey,
    required this.onSelected,
    required this.fallback,
    this.visible = true,
    this.titles = const [],
    this.widths = const [],
    this.onWidthsChanged,
    this.onScroll,
  });

  final String kind;
  final List<T> records;
  final String Function(T) keyOf;
  final Object Function(T) versionOf;
  final Map<String, Object?> Function(T) rowOf;
  final Listenable selection;
  final String? Function() selectedKey;
  final ValueChanged<T> onSelected;
  final WidgetBuilder fallback;
  final bool visible;
  final List<String> titles;
  final List<double> widths;
  final ValueChanged<List<double>>? onWidthsChanged;
  final ValueChanged<Offset>? onScroll;

  @override
  State<NativeCaptureList<T>> createState() => _NativeCaptureListState<T>();
}

class _NativeCaptureListState<T> extends State<NativeCaptureList<T>>
    with WidgetsBindingObserver {
  final int _id = NativeLists._nextId++;
  final _diff = NativeListDiff<T>();
  Future<void> _pending = Future<void>.value();
  bool _failed = false;
  bool _scheduled = false;
  bool _currentRoute = true;
  bool _resumed = true;
  Rect? _rect;
  String? _configuration;
  String? _selection;
  bool _hasSelection = false;
  (Rect, bool, double)? _lastFrame;
  bool get _visible =>
      widget.visible &&
      _currentRoute &&
      _resumed &&
      (!NativeLists.overlayFallback || !NativeListOverlays.blocked.value) &&
      !_failed;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    NativeLists._handlers[_id] = _receive;
    widget.selection.addListener(_select);
    NativeListOverlays.blocked.addListener(_overlayChanged);
    _enqueue('create', {'kind': widget.kind});
  }

  void _enqueue(String method, Map<String, Object?> args) {
    _pending = _pending
        .then(
          (_) => NativeLists.channel.invokeMethod<void>(method, {
            'id': _id,
            ...args,
          }),
        )
        .catchError((Object error) {
          if (mounted && !_failed) {
            setState(() => _failed = true);
            // A renderer failure must not leave an invisible native hit target.
            unawaited(
              NativeLists.channel
                  .invokeMethod<void>('dispose', {'id': _id})
                  .catchError((_) {}),
            );
          }
        });
  }

  void _receive(MethodCall call) {
    if (!mounted) return;
    final args = call.arguments as Map;
    if (call.method == 'selected') {
      final key = args['key'] as String;
      final record = _diff.record(key);
      if (record != null) {
        widget.onSelected(record);
      }
    } else if (call.method == 'widthsChanged') {
      widget.onWidthsChanged?.call(
        (args['widths'] as List).map((n) => (n as num).toDouble()).toList(),
      );
    } else if (call.method == 'scrolled') {
      widget.onScroll?.call(
        Offset((args['x'] as num).toDouble(), (args['y'] as num).toDouble()),
      );
    }
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _currentRoute = ModalRoute.isCurrentOf(context) ?? true;
    _scheduleSync();
    _sendFrame();
  }

  @override
  void didUpdateWidget(NativeCaptureList<T> oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.selection != widget.selection) {
      oldWidget.selection.removeListener(_select);
      widget.selection.addListener(_select);
    }
    _scheduleSync();
    _sendFrame();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _resumed =
        state != AppLifecycleState.hidden &&
        state != AppLifecycleState.detached;
    _sendFrame();
  }

  void _overlayChanged() {
    if (!mounted || !NativeLists.overlayFallback) return;
    _sendFrame();
    setState(() {});
  }

  void _select() {
    final key = widget.selectedKey();
    if (_hasSelection && _selection == key) return;
    _hasSelection = true;
    _selection = key;
    _enqueue('selection', {'key': key});
  }

  void _scheduleSync() {
    if (_scheduled || _failed) return;
    _scheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _scheduled = false;
      if (!mounted || _failed) return;
      final p = context.palette;
      final config = <String, Object?>{
        'titles': widget.titles,
        'widths': widget.widths,
        'palette': {
          'background': p.background.toARGB32(),
          'surface': p.surface.toARGB32(),
          'text': Theme.of(context).colorScheme.onSurface.toARGB32(),
          'muted': p.muted.toARGB32(),
          'line': p.line.toARGB32(),
          'selected': p.selectedRow.toARGB32(),
          'selectedText': p.onSelectedRow.toARGB32(),
          'selectedMuted': p.onSelectedRowMuted.toARGB32(),
          'marker': p.selectedRowMarker.toARGB32(),
          'accent': p.accent.toARGB32(),
        },
      };
      final fingerprint = config.toString();
      if (_configuration != fingerprint) {
        _configuration = fingerprint;
        _enqueue('configure', config);
      }
      final patch = _diff.update(
        widget.records,
        widget.keyOf,
        widget.versionOf,
        widget.rowOf,
      );
      if (patch != null) _enqueue('update', patch);
      _select();
      _sendFrame();
    });
  }

  void _bounds(Rect rect) {
    _rect = rect;
    _sendFrame();
  }

  void _sendFrame() {
    if (!mounted || _rect == null) return;
    final scale = View.of(context).devicePixelRatio;
    final next = (_rect!, _visible, scale);
    if (_lastFrame == next) return;
    _lastFrame = next;
    final rect = _rect!;
    _enqueue('frame', {
      'x': rect.left,
      'y': rect.top,
      'width': rect.width,
      'height': rect.height,
      'scale': scale,
      'visible': _visible,
    });
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    widget.selection.removeListener(_select);
    NativeListOverlays.blocked.removeListener(_overlayChanged);
    NativeLists._handlers.remove(_id);
    _enqueue('dispose', const {});
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final fallback = !_visible && widget.visible;
    return _BoundsReporter(
      onBounds: _bounds,
      child: Stack(
        fit: StackFit.expand,
        children: [
          if (defaultTargetPlatform == TargetPlatform.macOS && !_failed)
            AppKitView(
              viewType: 'dev.quocan.flutterNetworkViewer/capture_list',
              creationParams: {'id': _id, 'kind': widget.kind},
              creationParamsCodec: const StandardMessageCodec(),
            ),
          if (fallback) widget.fallback(context),
        ],
      ),
    );
  }
}

class _BoundsReporter extends SingleChildRenderObjectWidget {
  const _BoundsReporter({required this.onBounds, required super.child});
  final ValueChanged<Rect> onBounds;
  @override
  RenderObject createRenderObject(BuildContext context) =>
      _RenderBounds(onBounds);
  @override
  void updateRenderObject(BuildContext context, _RenderBounds renderObject) =>
      renderObject.onBounds = onBounds;
}

class _RenderBounds extends RenderProxyBox {
  _RenderBounds(this.onBounds);
  ValueChanged<Rect> onBounds;
  @override
  void paint(PaintingContext context, Offset offset) {
    super.paint(context, offset);
    final rect = localToGlobal(Offset.zero) & size;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (attached) onBounds(rect);
    });
  }
}
