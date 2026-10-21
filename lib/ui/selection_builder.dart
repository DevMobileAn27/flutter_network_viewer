import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

/// Only the old and new selected rows rebuild when selection changes.
class SelectionBuilder<T> extends StatefulWidget {
  const SelectionBuilder({
    super.key,
    required this.selection,
    required this.value,
    required this.builder,
  });

  final ValueListenable<T?> selection;
  final T value;
  final Widget Function(BuildContext context, bool selected) builder;

  @override
  State<SelectionBuilder<T>> createState() => _SelectionBuilderState<T>();
}

class _SelectionBuilderState<T> extends State<SelectionBuilder<T>> {
  late bool _selected;

  @override
  void initState() {
    super.initState();
    _selected = widget.selection.value == widget.value;
    widget.selection.addListener(_changed);
  }

  @override
  void didUpdateWidget(SelectionBuilder<T> oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.selection != oldWidget.selection) {
      oldWidget.selection.removeListener(_changed);
      widget.selection.addListener(_changed);
    }
    _selected = widget.selection.value == widget.value;
  }

  void _changed() {
    final selected = widget.selection.value == widget.value;
    if (selected != _selected) setState(() => _selected = selected);
  }

  @override
  void dispose() {
    widget.selection.removeListener(_changed);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => widget.builder(context, _selected);
}
