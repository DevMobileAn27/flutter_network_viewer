import 'dart:convert';
import 'dart:io';
import 'package:vm_service/vm_service_io.dart';

Future<void> main(List<String> args) async {
  final service = await vmServiceConnectUri(args[0]);
  try {
    final vm = await service.getVM();
    final id = vm.isolates!.firstWhere((i) => i.name == 'main').id!;
    final results = <Object>[];
    final cases = args.length > 2
        ? args[2].split(',')
        : ['console', 'small', 'medium', 'large', 'consoleJson'];
    for (final name in cases) {
      await service.clearCpuSamples(id);
      final start = (await service.getVMTimelineMicros()).timestamp!;
      final response = await service.callServiceExtension(
        'ext.viewerProbe.run',
        isolateId: id,
        args: {'case': name},
      );
      final end = (await service.getVMTimelineMicros()).timestamp!;
      final profile = await service.getCpuSamples(id, start, end - start);
      final functions = profile.functions ?? [];
      functions.sort(
        (a, b) => (b.exclusiveTicks ?? 0).compareTo(a.exclusiveTicks ?? 0),
      );
      final result = {
        ...response.json!,
        'sampleCount': profile.sampleCount,
        'hotFunctions': functions
            .take(20)
            .map(
              (f) => {
                'name': f.function?.name,
                'exclusive': f.exclusiveTicks,
                'inclusive': f.inclusiveTicks,
              },
            )
            .toList(),
      };
      results.add(result);
      File(
        args[1],
      ).writeAsStringSync(const JsonEncoder.withIndent('  ').convert(results));
      stdout.writeln(jsonEncode(result));
    }
    File(
      args[1],
    ).writeAsStringSync(const JsonEncoder.withIndent('  ').convert(results));
  } finally {
    await service.dispose();
  }
}
