import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../l10n/strings.dart';
import 'viewer_theme.dart';

enum ViewerNotificationType { info, success, danger, warning }

final _notifications = Expando<_NotificationSession>();

/// Shows a non-modal notification. Returns true for OK/Confirm, false for
/// Cancel, auto-hide, replacement, or disposal of the overlay.
/// Success and info notifications auto-hide after 3 seconds by default, except
/// confirmations. The delay is measured from when the popup is mounted;
/// [hideAfterMilisecond] must be non-negative.
Future<bool> showViewerNotification(
  BuildContext context, {
  required String description,
  String? title,
  ViewerNotificationType type = ViewerNotificationType.info,
  bool confirmation = false,
  bool? autoHide,
  int hideAfterMilisecond = 3000,
}) {
  if (hideAfterMilisecond < 0) {
    throw ArgumentError.value(
      hideAfterMilisecond,
      'hideAfterMilisecond',
      'Must be non-negative',
    );
  }
  final overlay = Overlay.of(context, rootOverlay: true);
  _notifications[overlay]?.close(false);
  final session = _NotificationSession(
    autoHide:
        autoHide ??
        (!confirmation &&
            (type == ViewerNotificationType.success ||
                type == ViewerNotificationType.info)),
    hideAfterMilisecond: hideAfterMilisecond,
  );
  _notifications[overlay] = session;
  session.entry = OverlayEntry(
    builder: (context) => _NotificationPopup(
      session: session,
      type: type,
      title: title,
      description: description,
      confirmation: confirmation,
    ),
  );
  overlay.insert(session.entry);
  return session.result.future;
}

class _NotificationSession {
  _NotificationSession({
    required this.autoHide,
    required this.hideAfterMilisecond,
  });

  final bool autoHide;
  final int hideAfterMilisecond;
  final result = Completer<bool>();
  late final OverlayEntry entry;
  Timer? _timer;

  void startAutoHide() {
    if (!autoHide || result.isCompleted) return;
    _timer = Timer(
      Duration(milliseconds: hideAfterMilisecond),
      () => close(false),
    );
  }

  void close(bool confirmed) {
    _timer?.cancel();
    if (result.isCompleted) return;
    result.complete(confirmed);
    entry.remove();
    entry.dispose();
  }

  void disposed() {
    _timer?.cancel();
    if (!result.isCompleted) result.complete(false);
  }
}

class _NotificationPopup extends StatefulWidget {
  const _NotificationPopup({
    required this.session,
    required this.type,
    required this.title,
    required this.description,
    required this.confirmation,
  });

  final _NotificationSession session;
  final ViewerNotificationType type;
  final String? title;
  final String description;
  final bool confirmation;

  @override
  State<_NotificationPopup> createState() => _NotificationPopupState();
}

class _NotificationPopupState extends State<_NotificationPopup> {
  @override
  void initState() {
    super.initState();
    widget.session.startAutoHide();
  }

  @override
  void dispose() {
    widget.session.disposed();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final palette = context.palette;
    final strings = context.l10n;
    final (icon, color, defaultTitle) = switch (widget.type) {
      ViewerNotificationType.info => (
        Icons.info_outline_rounded,
        const Color(0xff3b82f6),
        strings.notificationInfo,
      ),
      ViewerNotificationType.success => (
        Icons.check_circle_outline_rounded,
        const Color(0xff22c55e),
        strings.success,
      ),
      ViewerNotificationType.danger => (
        Icons.error_outline_rounded,
        const Color(0xffef4444),
        strings.error,
      ),
      ViewerNotificationType.warning => (
        Icons.warning_amber_rounded,
        const Color(0xffeab308),
        strings.notificationWarning,
      ),
    };
    final size = MediaQuery.sizeOf(context);
    return Positioned(
      left: 20,
      bottom: 20,
      width: math.min(380, math.max(0, size.width - 40)),
      child: Semantics(
        container: true,
        liveRegion: true,
        child: Material(
          key: const ValueKey('viewer-notification'),
          color: palette.surface,
          surfaceTintColor: Colors.transparent,
          elevation: 12,
          shadowColor: Colors.black.withValues(alpha: .25),
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(12),
            side: BorderSide(color: palette.line),
          ),
          clipBehavior: Clip.antiAlias,
          child: ConstrainedBox(
            constraints: BoxConstraints(
              maxHeight: math.max(0, size.height - 40),
            ),
            child: SingleChildScrollView(
              padding: const EdgeInsets.all(18),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Icon(icon, color: color, size: 23),
                      const SizedBox(width: 10),
                      Expanded(
                        child: Semantics(
                          header: true,
                          child: Text(
                            widget.title ?? defaultTitle,
                            style: const TextStyle(
                              fontSize: 15,
                              fontWeight: FontWeight.w700,
                            ),
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 10),
                  SelectableText(
                    widget.description,
                    style: TextStyle(
                      color: palette.description,
                      fontSize: 13,
                      height: 1.5,
                    ),
                  ),
                  const SizedBox(height: 16),
                  Align(
                    alignment: Alignment.centerRight,
                    child: Wrap(
                      alignment: WrapAlignment.end,
                      spacing: 8,
                      runSpacing: 8,
                      children: [
                        if (widget.confirmation)
                          OutlinedButton(
                            key: const ValueKey('notification-cancel'),
                            onPressed: () => widget.session.close(false),
                            child: Text(strings.notificationCancel),
                          ),
                        FilledButton(
                          key: const ValueKey('notification-confirm'),
                          onPressed: () => widget.session.close(true),
                          child: Text(
                            widget.confirmation
                                ? strings.notificationConfirm
                                : 'OK',
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
