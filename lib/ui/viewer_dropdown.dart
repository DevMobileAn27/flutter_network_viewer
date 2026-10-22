import 'dart:math' as math;

import 'package:flutter/material.dart';

import 'viewer_theme.dart';
import 'native_capture_list.dart';

class DropdownOption<T> {
  const DropdownOption(this.value, this.label, {this.color});

  final T value;
  final String label;
  final Color? color;
}

class ViewerDropdown<T> extends StatefulWidget {
  const ViewerDropdown({
    super.key,
    required this.value,
    required this.options,
    required this.onChanged,
    this.semanticLabel,
  });

  final T value;
  final List<DropdownOption<T>> options;
  final ValueChanged<T>? onChanged;
  final String? semanticLabel;

  @override
  State<ViewerDropdown<T>> createState() => _ViewerDropdownState<T>();
}

class _ViewerDropdownState<T> extends State<ViewerDropdown<T>> {
  final _controller = MenuController();
  final _focus = FocusNode();
  VoidCallback? _releaseNativeLists;

  @override
  void dispose() {
    _focus.dispose();
    _releaseNativeLists?.call();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colors = context.palette;
    final textStyle = Theme.of(
      context,
    ).textTheme.bodyMedium!.copyWith(fontSize: 12, fontWeight: FontWeight.w500);
    final selected = widget.options.where(
      (option) => option.value == widget.value,
    );
    final label = selected.isEmpty ? '' : selected.first.label;
    return LayoutBuilder(
      builder: (context, constraints) {
        final menuWidth = math.max(180.0, constraints.maxWidth);
        return MenuAnchor(
          controller: _controller,
          childFocusNode: _focus,
          crossAxisUnconstrained: false,
          alignmentOffset: const Offset(0, 6),
          onOpen: () {
            _releaseNativeLists ??= NativeListOverlays.acquire();
            setState(() {});
          },
          onClose: () {
            _releaseNativeLists?.call();
            _releaseNativeLists = null;
            setState(() {});
          },
          consumeOutsideTap: true,
          style: MenuStyle(
            alignment: Alignment.bottomLeft,
            backgroundColor: WidgetStatePropertyAll(colors.surface),
            surfaceTintColor: const WidgetStatePropertyAll(Colors.transparent),
            shadowColor: WidgetStatePropertyAll(
              Colors.black.withValues(alpha: .14),
            ),
            elevation: const WidgetStatePropertyAll(8),
            visualDensity: VisualDensity.standard,
            padding: const WidgetStatePropertyAll(EdgeInsets.all(6)),
            minimumSize: WidgetStatePropertyAll(Size(menuWidth, 0)),
            maximumSize: WidgetStatePropertyAll(Size(menuWidth, 340)),
            shape: WidgetStatePropertyAll(
              RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(12),
                side: BorderSide(color: colors.line),
              ),
            ),
          ),
          menuChildren: [
            for (final option in widget.options)
              Padding(
                padding: const EdgeInsets.symmetric(vertical: 2),
                child: MenuItemButton(
                  onPressed: widget.onChanged == null
                      ? null
                      : () => widget.onChanged!(option.value),
                  style: ButtonStyle(
                    minimumSize: WidgetStatePropertyAll(
                      Size(menuWidth - 12, 36),
                    ),
                    tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                    visualDensity: VisualDensity.standard,
                    padding: const WidgetStatePropertyAll(
                      EdgeInsets.symmetric(horizontal: 10),
                    ),
                    shape: WidgetStatePropertyAll(
                      RoundedRectangleBorder(
                        borderRadius: BorderRadius.circular(7),
                      ),
                    ),
                    textStyle: WidgetStatePropertyAll(textStyle),
                    iconColor: WidgetStatePropertyAll(colors.accent),
                    foregroundColor: WidgetStatePropertyAll(
                      option.color ??
                          (option.value == widget.value
                              ? colors.accent
                              : Theme.of(context).colorScheme.onSurface),
                    ),
                    backgroundColor: WidgetStatePropertyAll(
                      option.value == widget.value
                          ? colors.accent.withValues(alpha: .1)
                          : Colors.transparent,
                    ),
                    overlayColor: WidgetStatePropertyAll(
                      colors.accent.withValues(alpha: .08),
                    ),
                  ),
                  trailingIcon: SizedBox(
                    width: 18,
                    child: option.value == widget.value
                        ? const Icon(Icons.check_rounded, size: 16)
                        : null,
                  ),
                  child: Text(
                    option.label,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ),
          ],
          builder: (context, controller, child) => Semantics(
            label: widget.semanticLabel,
            expanded: controller.isOpen,
            child: SizedBox(
              width: double.infinity,
              height: 40,
              child: TextButton(
                focusNode: _focus,
                onPressed: widget.onChanged == null
                    ? null
                    : () => controller.isOpen
                          ? controller.close()
                          : controller.open(),
                style: ButtonStyle(
                  alignment: Alignment.centerLeft,
                  padding: const WidgetStatePropertyAll(
                    EdgeInsets.symmetric(horizontal: 12),
                  ),
                  textStyle: WidgetStatePropertyAll(textStyle),
                  foregroundColor: WidgetStateProperty.resolveWith(
                    (states) => states.contains(WidgetState.disabled)
                        ? colors.muted
                        : Theme.of(context).colorScheme.onSurface,
                  ),
                  backgroundColor: WidgetStatePropertyAll(colors.input),
                  overlayColor: WidgetStatePropertyAll(
                    controller.isOpen
                        ? colors.accent.withValues(alpha: .06)
                        : Colors.transparent,
                  ),
                  side: WidgetStatePropertyAll(
                    BorderSide(
                      color: controller.isOpen
                          ? colors.accent
                          : Colors.transparent,
                    ),
                  ),
                  shape: WidgetStatePropertyAll(
                    RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(8),
                    ),
                  ),
                ),
                child: Row(
                  children: [
                    Expanded(
                      child: Text(
                        label,
                        style: TextStyle(
                          color: selected.isEmpty ? null : selected.first.color,
                        ),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                    const SizedBox(width: 8),
                    AnimatedRotation(
                      turns: controller.isOpen ? .5 : 0,
                      duration: const Duration(milliseconds: 160),
                      child: Icon(
                        Icons.keyboard_arrow_down_rounded,
                        size: 18,
                        color: colors.muted,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}
