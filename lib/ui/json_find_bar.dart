import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../l10n/strings.dart';
import 'json_find_controller.dart';
import 'viewer_theme.dart';

class JsonFindBar extends StatefulWidget {
  const JsonFindBar({
    super.key,
    required this.controller,
    required this.focusNode,
  });
  static const width = 400.0;
  final JsonFindController controller;
  final FocusNode focusNode;

  @override
  State<JsonFindBar> createState() => _JsonFindBarState();
}

class _JsonFindBarState extends State<JsonFindBar> {
  late final _input = TextEditingController(text: widget.controller.query);

  @override
  void dispose() {
    _input.dispose();
    super.dispose();
  }

  void _clear() {
    _input.clear();
    widget.controller.update(query: '');
    widget.focusNode.unfocus();
  }

  Widget _button(
    String key,
    String tooltip,
    Widget icon,
    VoidCallback? action, {
    bool selected = false,
  }) => Tooltip(
    message: tooltip,
    child: SizedBox(
      width: 24,
      height: 24,
      child: IconButton(
        key: ValueKey(key),
        padding: EdgeInsets.zero,
        constraints: const BoxConstraints(),
        style: IconButton.styleFrom(
          backgroundColor: selected
              ? context.palette.accent.withValues(alpha: .18)
              : Colors.transparent,
          foregroundColor: selected
              ? context.palette.accent
              : context.palette.muted,
          shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(3)),
        ),
        onPressed: action,
        icon: icon,
        iconSize: 16,
      ),
    ),
  );

  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: widget.controller,
    builder: (context, _) {
      final find = widget.controller;
      final options = [
        _button(
          'json-find-case',
          context.l10n.findMatchCase,
          const Text('Aa', style: TextStyle(fontSize: 11)),
          () => find.update(matchCase: !find.matchCase),
          selected: find.matchCase,
        ),
        _button(
          'json-find-word',
          context.l10n.findWholeWord,
          const Text(
            'ab',
            style: TextStyle(
              fontSize: 11,
              decoration: TextDecoration.underline,
            ),
          ),
          () => find.update(wholeWord: !find.wholeWord),
          selected: find.wholeWord,
        ),
        _button(
          'json-find-regex',
          context.l10n.findRegex,
          const Text('.*', style: TextStyle(fontSize: 12)),
          () => find.update(regex: !find.regex),
          selected: find.regex,
        ),
      ];
      final navigation = [
        SizedBox(
          width: 68,
          child: Tooltip(
            message: find.error ?? '',
            child: Text(
              find.error != null
                  ? context.l10n.findInvalidRegex
                  : find.matches.isEmpty
                  ? context.l10n.findNoResults
                  : context.l10n.findMatchCount(
                      find.index + 1,
                      find.matches.length,
                    ),
              key: const ValueKey('json-find-count'),
              textAlign: TextAlign.right,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: TextStyle(
                fontSize: 10,
                color: find.error != null
                    ? context.palette.danger
                    : Theme.of(context).colorScheme.onSurface,
              ),
            ),
          ),
        ),
        const SizedBox(width: 8),
        _button(
          'json-find-previous',
          context.l10n.findPrevious,
          const Icon(Icons.arrow_upward),
          find.matches.isEmpty ? null : () => find.move(-1),
        ),
        _button(
          'json-find-next',
          context.l10n.findNext,
          const Icon(Icons.arrow_downward),
          find.matches.isEmpty ? null : () => find.move(1),
        ),
        _button(
          'json-find-clear',
          context.l10n.clearSearch,
          const Icon(Icons.close),
          _clear,
        ),
      ];
      return CallbackShortcuts(
        bindings: {
          const SingleActivator(LogicalKeyboardKey.enter): () => find.move(1),
          const SingleActivator(LogicalKeyboardKey.enter, shift: true): () =>
              find.move(-1),
          const SingleActivator(LogicalKeyboardKey.escape): _clear,
        },
        child: SizedBox(
          key: const ValueKey('json-find-bar'),
          width: JsonFindBar.width,
          height: 30,
          child: Row(
            children: [
              SizedBox(
                width: 240,
                height: 26,
                child: ListenableBuilder(
                  listenable: widget.focusNode,
                  builder: (context, _) => Container(
                    key: const ValueKey('json-find-input-box'),
                    padding: const EdgeInsets.only(right: 2),
                    decoration: BoxDecoration(
                      color: context.palette.input,
                      borderRadius: BorderRadius.circular(3),
                      border: Border.all(
                        color: find.error != null
                            ? context.palette.danger
                            : widget.focusNode.hasFocus
                            ? context.palette.accent
                            : context.palette.line,
                      ),
                    ),
                    child: Row(
                      children: [
                        Expanded(
                          child: TextField(
                            key: const ValueKey('json-find-input'),
                            controller: _input,
                            focusNode: widget.focusNode,
                            onChanged: (text) => find.update(query: text),
                            style: const TextStyle(fontSize: 11),
                            decoration: InputDecoration(
                              hintText: context.l10n.findInJson,
                              contentPadding: const EdgeInsets.symmetric(
                                horizontal: 6,
                                vertical: 6,
                              ),
                              isDense: true,
                              border: InputBorder.none,
                              enabledBorder: InputBorder.none,
                              focusedBorder: InputBorder.none,
                              filled: false,
                            ),
                          ),
                        ),
                        ...options,
                      ],
                    ),
                  ),
                ),
              ),
              const SizedBox(width: 8),
              ...navigation,
              const SizedBox(width: 4),
            ],
          ),
        ),
      );
    },
  );
}
