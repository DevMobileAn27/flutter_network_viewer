// Development-only entry point. No discovery, live traffic or saved settings.
// Build in profile mode with -t tool/profile_viewer.dart, then run
// dart run tool/measure_viewer.dart <VM service WebSocket URI> <output.json>.
import 'dart:convert';
import 'dart:developer';

import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/services.dart';
import 'package:flutter_network_viewer/l10n/app_localizations.dart';
import 'package:flutter_network_viewer/models/capture.dart';
import 'package:flutter_network_viewer/ui/inspector.dart';
import 'package:flutter_network_viewer/ui/viewer_theme.dart';

void main() {
  _ProfileBinding();
  final key = GlobalKey<_ProbeState>();
  runApp(
    MaterialApp(
      theme: viewerTheme(Brightness.light),
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(
        body: Center(
          child: SizedBox(width: 680, child: _Probe(key: key)),
        ),
      ),
    ),
  );
  registerExtension('ext.viewerProbe.run', (_, params) async {
    final result = await key.currentState!.run(params['case']!);
    return ServiceExtensionResponse.result(jsonEncode(result));
  });
}

class _ProfileBinding extends WidgetsFlutterBinding {
  void resumeForMeasurement() =>
      handleAppLifecycleStateChanged(AppLifecycleState.resumed);
}

class _Probe extends StatefulWidget {
  const _Probe({super.key});
  @override
  State<_Probe> createState() => _ProbeState();
}

class _ProbeState extends State<_Probe> {
  static const resources = MethodChannel(
    'dev.quocan.flutterNetworkViewer/process_resources',
  );
  Widget content = const Text('Ready for profile run');
  bool running = false;

  Future<Map<String, Object?>> run(String name) async {
    if (running) throw StateError('A measurement is already running');
    running = true;
    final count = name == 'large'
        ? 1000
        : name == 'medium'
        ? 100
        : 5;
    final bodies = List.generate(
      2,
      (variant) => jsonEncode({
        'items': List.generate(
          count,
          (i) => {
            'id': i,
            'name': 'Demo item $i variant $variant',
            'enabled': i.isEven,
            'color': '#3366ff',
            'description': 'Synthetic response for repeatable profiling.',
          },
        ),
      }),
    );
    final requests = List.generate(
      2,
      (i) => NetworkRecord('probe', '$i')
        ..apply({
          'phase': 'request',
          'request': {'url': 'https://example.test/$i', 'method': 'GET'},
        })
        ..apply({
          'phase': 'response',
          'response': {
            'statusCode': 200,
            'headers': {'content-type': 'application/json'},
            'body': bodies[i],
          },
        }),
    );
    final logs = List.generate(
      2,
      (i) => ConsoleRecord(i, 'probe', {
        'source': 'dart.stdout',
        'message': name == 'console'
            ? 'Demo log $i: completed operation successfully.'
            : bodies[i],
      }),
    );
    final frames = <FrameTiming>[];
    void collect(List<FrameTiming> timings) => frames.addAll(timings);
    SchedulerBinding.instance.addTimingsCallback(collect);
    // Keep this synthetic workload independent of desktop focus/occlusion.
    // Both renderers are measured with the same resumed scheduler.
    (WidgetsBinding.instance as _ProfileBinding).resumeForMeasurement();
    final latencies = <int>[];
    await resources.invokeMethod<Object?>('sample');
    final total = Stopwatch()..start();
    try {
      for (var i = 0; i < 12; i++) {
        final clock = Stopwatch()..start();
        final task = TimelineTask()..start('select-$name');
        setState(
          () => content = name.startsWith('console')
              ? ConsoleInspector(record: logs[i % 2])
              : NetworkInspector(record: requests[i % 2], initialTab: 4),
        );
        SchedulerBinding.instance.scheduleForcedFrame();
        // Occluded macOS windows can stop receiving frames. Reject that run
        // instead of reporting a misleading low CPU average over idle time.
        await SchedulerBinding.instance.endOfFrame.timeout(
          const Duration(seconds: 2),
        );
        latencies.add(clock.elapsedMicroseconds);
        task.finish();
        await Future<void>.delayed(const Duration(milliseconds: 150));
      }
      final sample = await resources.invokeMapMethod<String, dynamic>('sample');
      total.stop();
      latencies.sort();
      return {
        'case': name,
        'bodyBytes': bodies[0].length,
        'selections': latencies.length,
        'elapsedMs': total.elapsedMilliseconds,
        'cpuPercent': sample?['cpuPercent'],
        'memoryBytes': sample?['memoryBytes'],
        'selectionMedianMs': latencies[latencies.length ~/ 2] / 1000,
        'selectionMaxMs': latencies.last / 1000,
        'buildTotalMs':
            frames.fold<int>(0, (n, f) => n + f.buildDuration.inMicroseconds) /
            1000,
        'rasterTotalMs':
            frames.fold<int>(0, (n, f) => n + f.rasterDuration.inMicroseconds) /
            1000,
      };
    } finally {
      SchedulerBinding.instance.removeTimingsCallback(collect);
      running = false;
    }
  }

  @override
  Widget build(BuildContext context) => content;
}
