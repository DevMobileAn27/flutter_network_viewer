import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'components.dart';

/// Updates independently so resource sampling does not rebuild captured entries.
class ProcessResourceUsage extends StatefulWidget {
  const ProcessResourceUsage({super.key});

  @override
  State<ProcessResourceUsage> createState() => _ProcessResourceUsageState();
}

class _ProcessResourceUsageState extends State<ProcessResourceUsage> {
  static const _channel = MethodChannel(
    'dev.quocan.flutterNetworkViewer/process_resources',
  );
  Timer? _timer;
  bool _sampling = false;
  double? _memoryMB;

  @override
  void initState() {
    super.initState();
    _timer = Timer.periodic(const Duration(seconds: 2), (_) => _sample());
    _sample();
  }

  Future<void> _sample() async {
    if (_sampling) return;
    _sampling = true;
    double? memory;
    try {
      final values = await _channel.invokeMapMethod<String, num>('sample');
      final bytes = _validNumber(values?['memoryBytes']);
      memory = bytes == null ? null : bytes / 1000000;
    } on MissingPluginException {
      _timer?.cancel();
    } on PlatformException {
      // A failed sample is shown as unavailable rather than stale usage.
    } finally {
      _sampling = false;
    }
    if (!mounted) return;
    setState(() {
      _memoryMB = memory;
    });
  }

  double? _validNumber(num? value) =>
      value != null && value.isFinite && value >= 0 ? value.toDouble() : null;

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => RepaintBoundary(
    child: Text(
      'RAM ${_memoryMB == null ? '—' : '${_memoryMB!.toStringAsFixed(0)} MB'}',
      key: const ValueKey('process-resource-usage'),
      maxLines: 1,
      style: mono.copyWith(fontSize: 10, color: context.palette.muted),
    ),
  );
}
