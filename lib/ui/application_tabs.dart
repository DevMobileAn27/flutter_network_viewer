import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

import '../l10n/strings.dart';
import '../models/capture.dart';
import 'viewer_theme.dart';

class ApplicationTabs extends StatelessWidget {
  const ApplicationTabs({
    super.key,
    required this.sessions,
    required this.selectedId,
    required this.isOnline,
    required this.onChanged,
  });

  final List<DeviceSession> sessions;
  final String? selectedId;
  final bool Function(String) isOnline;
  final ValueChanged<String?> onChanged;

  @override
  Widget build(BuildContext context) {
    final colors = context.palette;
    return LayoutBuilder(
      builder: (context, constraints) {
        final labelWidth = (constraints.maxWidth - 34).clamp(16.0, 160.0);
        return SizedBox(
          height: 38,
          child: DefaultTabController(
            key: ValueKey(
              Object.hashAll(sessions.map((session) => session.id)),
            ),
            length: sessions.length + 1,
            initialIndex:
                sessions.indexWhere((session) => session.id == selectedId) + 1,
            child: Material(
              color: colors.input,
              borderRadius: BorderRadius.circular(30),
              clipBehavior: Clip.antiAlias,
              child: Padding(
                padding: const EdgeInsets.all(3),
                child: ScrollConfiguration(
                  behavior: ScrollConfiguration.of(
                    context,
                  ).copyWith(dragDevices: {...PointerDeviceKind.values}),
                  child: TabBar(
                    isScrollable: true,
                    tabAlignment: TabAlignment.start,
                    onTap: (index) =>
                        onChanged(index == 0 ? null : sessions[index - 1].id),
                    dividerColor: Colors.transparent,
                    indicatorSize: TabBarIndicatorSize.tab,
                    indicatorAnimation: TabIndicatorAnimation.linear,
                    indicatorPadding: const EdgeInsets.symmetric(horizontal: 2),
                    indicator: BoxDecoration(
                      color: colors.primaryFill,
                      borderRadius: BorderRadius.circular(30),
                    ),
                    overlayColor: const WidgetStatePropertyAll(
                      Colors.transparent,
                    ),
                    splashFactory: NoSplash.splashFactory,
                    labelColor: Colors.white,
                    unselectedLabelColor: colors.muted,
                    labelStyle: const TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.w600,
                    ),
                    unselectedLabelStyle: const TextStyle(
                      fontSize: 12,
                      fontWeight: FontWeight.w500,
                    ),
                    labelPadding: const EdgeInsets.symmetric(horizontal: 14),
                    tabs: [
                      Tab(
                        key: const ValueKey('application-all'),
                        child: Tooltip(
                          message: context.l10n.allSessions,
                          child: ConstrainedBox(
                            constraints: BoxConstraints(maxWidth: labelWidth),
                            child: Text(
                              context.l10n.allApps,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                            ),
                          ),
                        ),
                      ),
                      for (final session in sessions)
                        Tab(
                          key: ValueKey('application-${session.id}'),
                          child: Tooltip(
                            message:
                                '${session.name}\n${session.device} · ${session.id}\n${isOnline(session.id) ? context.l10n.connected : context.l10n.disconnected}',
                            child: ConstrainedBox(
                              constraints: BoxConstraints(maxWidth: labelWidth),
                              child: Row(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  Icon(
                                    Icons.circle,
                                    size: 6,
                                    color: isOnline(session.id)
                                        ? const Color(0xff63ddbb)
                                        : null,
                                  ),
                                  const SizedBox(width: 8),
                                  Flexible(
                                    child: Text(
                                      session.name,
                                      maxLines: 1,
                                      overflow: TextOverflow.ellipsis,
                                    ),
                                  ),
                                ],
                              ),
                            ),
                          ),
                        ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}
