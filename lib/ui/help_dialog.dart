import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

import '../l10n/strings.dart';

class HelpDialog extends StatefulWidget {
  const HelpDialog({super.key});

  static final discussionUri = Uri.parse(
    'https://github.com/DevMobileAn27/flutter_network_viewer/discussions',
  );

  @override
  State<HelpDialog> createState() => _HelpDialogState();
}

class _HelpDialogState extends State<HelpDialog> {
  bool _opening = false;
  bool _failed = false;

  Future<void> _openDiscussion() async {
    setState(() {
      _opening = true;
      _failed = false;
    });
    var opened = false;
    try {
      opened = await launchUrl(
        HelpDialog.discussionUri,
        mode: LaunchMode.externalApplication,
      );
    } catch (_) {
      // Keep the guide open and offer the address if no browser is available.
    }
    if (!mounted) return;
    setState(() {
      _opening = false;
      _failed = !opened;
    });
  }

  @override
  Widget build(BuildContext context) {
    final strings = context.l10n;
    return AlertDialog(
      key: const ValueKey('basic-usage-guide'),
      scrollable: true,
      title: Text(strings.helpTitle),
      content: SizedBox(
        width: 610,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _step(
              Icons.radar,
              strings.helpConnectTitle,
              strings.helpConnectBody,
            ),
            _step(
              Icons.swap_vert,
              strings.helpNetworkTitle,
              strings.helpNetworkBody,
            ),
            _step(
              Icons.terminal,
              strings.helpConsoleTitle,
              strings.helpConsoleBody,
            ),
            _step(
              Icons.data_object,
              strings.helpJsonTitle,
              strings.helpJsonBody,
            ),
            _step(
              Icons.save_alt,
              strings.helpCaptureTitle,
              strings.helpCaptureBody,
            ),
            if (_failed) ...[
              Text(
                strings.openDiscussionFailed,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
              const SizedBox(height: 8),
              SelectableText(HelpDialog.discussionUri.toString()),
            ],
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(strings.gotIt),
        ),
        FilledButton.icon(
          key: const ValueKey('go-to-discussion'),
          onPressed: _opening ? null : _openDiscussion,
          icon: const Icon(Icons.open_in_new, size: 18),
          label: Text(strings.goToDiscussion),
        ),
      ],
    );
  }

  Widget _step(IconData icon, String title, String body) => Padding(
    padding: const EdgeInsets.only(bottom: 20),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(icon, size: 20, color: Theme.of(context).colorScheme.primary),
        const SizedBox(width: 12),
        Expanded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(title, style: const TextStyle(fontWeight: FontWeight.w600)),
              const SizedBox(height: 6),
              Text(body),
            ],
          ),
        ),
      ],
    ),
  );
}
