// Local native JSON fixture, without network/discovery/preferences.
import 'package:flutter/material.dart';
import 'package:flutter_network_viewer/l10n/app_localizations.dart';
import 'package:flutter_network_viewer/ui/components.dart';
import 'package:flutter_network_viewer/ui/native_capture_list.dart';
import 'package:flutter_network_viewer/ui/native_json_view.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await NativeLists.initialize();
  await NativeJson.initialize();
  runApp(
    MaterialApp(
      debugShowCheckedModeBanner: false,
      theme: viewerTheme(Brightness.light),
      supportedLocales: AppLocalizations.supportedLocales,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      home: Scaffold(
        body: SafeArea(
          child: DataSection(
            'Demo',
            {
              'name': 'Tiếng Việt 😀',
              'color': '#ff0000',
              'nested': {'a': 'hello', 'b': 'hello', 'c': 'hello'},
              'rows': List.generate(
                200,
                (i) => {'id': i, 'value': 'Row $i', 'enabled': i.isEven},
              ),
            },
            body: true,
            searchable: true,
            fillAvailable: true,
            showTitle: false,
          ),
        ),
      ),
    ),
  );
}
