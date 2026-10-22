// Local native image QA. No discovery or external connections.
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_network_viewer/l10n/app_localizations.dart';
import 'package:flutter_network_viewer/ui/viewer_theme.dart';
import 'package:flutter_network_viewer/ui/network_body_section.dart';
import 'package:flutter_network_viewer/ui/native_capture_list.dart';
import 'package:flutter_network_viewer/ui/native_json_view.dart';
import 'package:flutter_network_viewer/ui/native_image_view.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await NativeLists.initialize();
  await NativeJson.initialize();
  await NativeImages.initialize();
  // Embed fixtures at build time so the standalone QA bundle needs no files.
  const encoded =
      'iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAYAAABzenr0AAAABGdBTUEAALGPC/xhBQAAACBjSFJNAAB6JgAAgIQAAPoAAACA6AAAdTAAAOpgAAA6mAAAF3CculE8AAAAeGVYSWZNTQAqAAAACAAEARoABQAAAAEAAAA+ARsABQAAAAEAAABGASgAAwAAAAEAAgAAh2kABAAAAAEAAABOAAAAAAAAAEgAAAABAAAASAAAAAEAA6ABAAMAAAABAAEAAKACAAQAAAABAAAAIKADAAQAAAABAAAAIAAAAACfCVbEAAAACXBIWXMAAAsTAAALEwEAmpwYAAABWWlUWHRYTUw6Y29tLmFkb2JlLnhtcAAAAAAAPHg6eG1wbWV0YSB4bWxuczp4PSJhZG9iZTpuczptZXRhLyIgeDp4bXB0az0iWE1QIENvcmUgNi4wLjAiPgogICA8cmRmOlJERiB4bWxuczpyZGY9Imh0dHA6Ly93d3cudzMub3JnLzE5OTkvMDIvMjItcmRmLXN5bnRheC1ucyMiPgogICAgICA8cmRmOkRlc2NyaXB0aW9uIHJkZjphYm91dD0iIgogICAgICAgICAgICB4bWxuczp4bXA9Imh0dHA6Ly9ucy5hZG9iZS5jb20veGFwLzEuMC8iPgogICAgICAgICA8eG1wOkNyZWF0b3JUb29sPkZpZ21hPC94bXA6Q3JlYXRvclRvb2w+CiAgICAgIDwvcmRmOkRlc2NyaXB0aW9uPgogICA8L3JkZjpSREY+CjwveDp4bXBtZXRhPgoE/1zIAAAHw0lEQVRYCX1Xa2wc1RX+Zma9Xjt+BnBc23FiSwHRQkgaUVTqFkSjVpSqClF/FLWE0idIhSBVaiqDqvKnUlUhoSCrCNr+Kv+I+gup0D9UvFQSGpwHxEnsPNw4trO73tfszszOo9+5s3d3vNhc7Z37Oud855x7zpkdoK1FUTTBfigMw3c4RmEYsIeN3j7faK334jGWoWQd4nyiDa615GEP+wsEKwiTtBhYC0wqofdk3GgutHo/jIXFz7UoCF7gtEcjGzLhRi/764ZhfCteh/q8bVTkwqF+0EtNxW3V9L6s9ZwTylfHxHqL8++zl03ZoaW/F3AeiDINLiFucgsZm5w1UHgkAhsy1al6JFmSc/Jp+YIlmEJvcHMn+2luyhUoGRs/kmcx8A2P6nB7qDMeW3xCq9E516wNbUVxYlU43mkGQfC1zcE1p4gWgXEXOZ+WgSdPhXj0XR+nsmGbJzR4g0+zypJNDBVMhU1XvMfFvZ9vfcwoTwE/W4rw3FyE+WyEasnHYBTg1fs7sWfYUh5pUctMG5FUSuQoL7wvVyBtPc8mKwE/XQSmPw1xKR8idCK4doDimoshK8RrB3qxd7RDWRiL0HIFXOZ6lNM4KE0mm6zYhGCzHls+WwB+cybAQjZAUAvhVAO4tQDwQlxcquLg35bx0aKnrGtZrmUnR5mTQq5Ccr2xjAf1TGoau+t8LsATH4e4VgoReRE8Wu9RCc/24VUceOUa3KKN0T4D7x2ZwI6hrsR1NCDaPCBQqQRq21SUiC33IwN2sYy5D1dhjY4hpAJ1J0Sd1tdtD/WKC992gFoNhewCavbN8KNupAxJPS02lieub3nagNmMEUUnh5owBj9VjPCTkwFOeBk8v9uBfe4yKkUfTrkOt+wq6/1qDUGlip7yPF46NIoTqa147MNkdmhQARGNWji8goA7mqA1SsDNMuCe/YT3e8NHvRri8K46uq4t4Jl/0vU9w0DdQ0jw0CZ45SKO/ngMxp59mFlIIZer4xaEeOUbnbhrm84ODS6KxG2DGIiLzMlChGmCL66F8OlqSTeHET+9L0L3ygIOv27D6byJbie4PY+Zn+1AdNeXMTNvoUxatxKgXHBxixkxO/qwd0RnhyghTYzlk8UgkpzUUSvz/zLaf3vGx2I+QOiGcGm9y2CrFenyah1/+GYGmfwlPPXX/yFtOHj5iUlEd+zBixcs2GV6iwo7FQZn2UM5X8VoxsCxx4exd3uaMaGzbp0CShmVPmeZYk8y2ldKDXAnICjTrVJXweYWqzBcHzM/GMBgsIK0ZaI0fjv+OGeiSnCfCntVeoDgngRnhdmxZmN7fwpvH5nE5LBkR8sLqZb1opGB5ZU1fHK8iJ6xYfgCTms8m+AS7Yz0sOYivH4dH5/OY+bpKfIEOPTaVabnALZkOhW4R1oBD0ivssOt4drcFaxmB6jAWNPbYjYLUSswZL5vrAO/HMkiN7ek7t0tSaS7tD4WFmZXcf9EAU99bxIrHhV2Tfxqj4HJ1YvILdeYHS3rhSco2+goXMHvHh7EF3f2S/UR3GZriwEDNddDNpfFi29cwdHZXlg9fbSaljuM+BvL2D++hqd/8VW8bY1gNsf6UIkwYdbxgHkVR9/M4nR9DFbkMzuqiCoVpIuXMX1gAI9/dzduHtyKrk6+OpupSJ+3KmGslBRG23GQy2fxp2ML+PPxDNC1BTQP+3cUcITgL1VGcCHHYlQJee8hSgUfYykPz96+hOePLeOizezwHYJfwnMHWRMe2o2Bvn51RaYpf0G0F6QQrWsRTNNAN+9y6+BNeObATjx2RwlYPI/7thfw8uGv4ANvCGeXGBeFOrPCY/l1AQbauQtlnF3rwV9+ug2j/jys/DymHxbwOxW4yGwHF2gjZBqqd2xTK7WNIAxRYZFZXF7Fvz+6iqm9O3Db+DAeecvH8YJJF0tmsDPa61Sgni/jO7cG+Mevd+HN/5zH4koRD967C1v7GZxdGVjrLNdWG3wXqBqgN/QYKYYtXV0Y/8I2PPKg3F2GtZ2eXc2z0PSpVJRo9yXQWAlZddAdBOhId+K+fbfCcV1kyCOWrwNvej+uAykVlZsokWKO93ZL3maU+4LAw9RACW/M1tDRvSUuw/RSVLOpwBL2f4nVkC+uHirencmousIbXd/a1imDrmmlok5JrSbviAxxreCf18jEwbv7ceLcBRw7wxdpaIElEqZzA49OZfDte8Z5dRFEcUsZpeXoUdD1PJYrWdD4SyYlMknQmAu9TPngP3xU6O6r15fxr+PXcHK+zN0A99zWiwfunsDI0BB66DFTgzd5hV9aS75hKMPfN3zf/6FlWX+Pa7RCIqHmbGmr+AUuYHDyvV+2bXh8G4p30uk03d7NzmCz6JWElYpPi9HiKUf4+B76Udvf8uSLQnPF0Mmn/Ieq+/xbxqCTO0rxGlMElhRugWu0z8ppWB//LacmlxkDr8QAwpQUkoBNyBEXd3ZYzIw0ewfSnKssE8ubdDJJrrWsWDHBFGy14mLjTzORoSj40DVc1hpEnWnBjf3P2RPLpRGv+WmmNhqb/DJSH45rJFCt9YGpP0I3GVnMYlr9QZocAy1OPnblw/ezH6dNLWLtJjh8nYQ/p6enODY92W6ccg//eK7ziPZOQ6jBuKCId+nuV7n1DsdLjSM1/B/z6TpTuUDsNQAAAABJRU5ErkJggg==';
  final bytes = base64Decode(encoded);
  final image = NativeImageData(bytes);
  runApp(
    MaterialApp(
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      theme: viewerTheme(Brightness.light),
      home: Builder(
        builder: (context) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            showDialog<void>(
              context: context,
              builder: (dialogContext) => Dialog(
                insetPadding: const EdgeInsets.all(24),
                clipBehavior: Clip.antiAlias,
                child: SizedBox(
                  width: MediaQuery.sizeOf(context).width * .9,
                  height: MediaQuery.sizeOf(context).height * .9,
                  child: NativeImageView(
                    image: image,
                    interactive: true,
                    modal: true,
                    onClose: () => Navigator.pop(dialogContext),
                    fallback: (_) => const Text('Native decoding failed'),
                  ),
                ),
              ),
            );
          });
          return Scaffold(
            body: NetworkBodySection(
              data: {
                'headers': {'content-type': 'image/png'},
                'bodyEncoding': 'base64',
                'body': encoded,
              },
              fillAvailable: true,
            ),
          );
        },
      ),
    ),
  );
}
