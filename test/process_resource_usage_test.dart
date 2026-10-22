import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_network_viewer/ui/process_resource_usage.dart';

void main() {
  const channel = MethodChannel(
    'dev.quocan.flutterNetworkViewer/process_resources',
  );
  final binding = TestWidgetsFlutterBinding.ensureInitialized();

  tearDown(() {
    binding.defaultBinaryMessenger.setMockMethodCallHandler(channel, null);
  });

  testWidgets('samples memory periodically and stops when disposed', (
    tester,
  ) async {
    var samples = 0;
    binding.defaultBinaryMessenger.setMockMethodCallHandler(channel, (
      call,
    ) async {
      expect(call.method, 'sample');
      samples++;
      return {'memoryBytes': 170000000 + samples * 1000000};
    });
    await tester.pumpWidget(
      const MaterialApp(home: Scaffold(body: ProcessResourceUsage())),
    );
    await tester.pump();
    expect(find.text('RAM 171 MB'), findsOneWidget);
    await tester.pump(const Duration(seconds: 2));
    await tester.pump();
    expect(find.text('RAM 172 MB'), findsOneWidget);
    expect(samples, 2);
    await tester.pumpWidget(const SizedBox());
    await tester.pump(const Duration(seconds: 4));
    expect(samples, 2);
    expect(tester.takeException(), isNull);
  });

  testWidgets('clears failed memory samples and recovers', (tester) async {
    var samples = 0;
    binding.defaultBinaryMessenger.setMockMethodCallHandler(channel, (_) async {
      samples++;
      if (samples == 2) throw PlatformException(code: 'unavailable');
      return {'memoryBytes': 256000000};
    });
    await tester.pumpWidget(
      const MaterialApp(home: Scaffold(body: ProcessResourceUsage())),
    );
    await tester.pump();
    expect(find.text('RAM 256 MB'), findsOneWidget);
    await tester.pump(const Duration(seconds: 2));
    await tester.pump();
    expect(find.text('RAM —'), findsOneWidget);
    await tester.pump(const Duration(seconds: 2));
    await tester.pump();
    expect(find.text('RAM 256 MB'), findsOneWidget);
    await tester.pumpWidget(const SizedBox());
  });
}
