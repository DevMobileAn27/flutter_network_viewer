import 'package:flutter/widgets.dart';

import 'app_localizations.dart';

extension ViewerLocalization on BuildContext {
  AppLocalizations get l10n => AppLocalizations.of(this);
}

/// Filter values and capture data stay stable when the UI language changes.
String filterLabel(BuildContext context, String value) => switch (value) {
  'All methods' => context.l10n.allMethods,
  'All statuses' => context.l10n.allStatuses,
  'All levels' => context.l10n.allLevels,
  'All log sources' => context.l10n.allLogSources,
  'dart.stdout' => 'stdout',
  'dart.stderr' => 'stderr',
  'dart.logging' => 'dart:developer',
  'Success' => context.l10n.success,
  'Errors' => context.l10n.errors,
  'Pending' => context.l10n.pending,
  'Error' => context.l10n.error,
  'Aborted' || 'abort' => context.l10n.aborted,
  _ => value,
};
