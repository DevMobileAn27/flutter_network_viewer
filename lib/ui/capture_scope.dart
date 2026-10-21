import 'package:flutter/material.dart';

import '../services/capture_store.dart';

/// Captured dialogs must release their bodies when capture data is removed.
class CaptureScope extends InheritedWidget {
  const CaptureScope({super.key, required this.store, required super.child});
  final CaptureStore store;

  @override
  bool updateShouldNotify(CaptureScope oldWidget) => store != oldWidget.store;
}

Future<void> showCaptureDialog({
  required BuildContext context,
  required WidgetBuilder builder,
  CaptureStore? store,
}) async {
  store ??= context.getInheritedWidgetOfExactType<CaptureScope>()?.store;
  if (store == null) {
    await showDialog<void>(context: context, builder: builder);
    return;
  }
  final capture = store;
  final revision = capture.revision;
  final navigator = Navigator.of(context, rootNavigator: true);
  final route = DialogRoute<void>(context: context, builder: builder);
  void changed() {
    if (capture.revision != revision && route.isActive) {
      navigator.removeRoute(route);
    }
  }

  capture.addListener(changed);
  try {
    await navigator.push(route);
  } finally {
    capture.removeListener(changed);
  }
}
