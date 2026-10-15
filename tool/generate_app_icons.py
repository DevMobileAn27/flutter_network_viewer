#!/usr/bin/env python3
"""Generate desktop launcher icons from logo-app.png on macOS (no packages)."""

import json
from pathlib import Path
import struct
import subprocess


def main():
    root = Path(__file__).resolve().parent.parent
    source = root / 'assets/images/logo/logo-app.png'
    catalog = root / 'macos/Runner/Assets.xcassets/AppIcon.appiconset'
    manifest = json.loads((catalog / 'Contents.json').read_text())
    sizes = {
        image['filename']: round(
            float(image['size'].split('x')[0]) * float(image['scale'][:-1])
        )
        for image in manifest['images']
    }
    for filename, size in sizes.items():
        subprocess.run(
            ['sips', '-z', str(size), str(size), str(source),
             '--out', str(catalog / filename)],
            check=True, stdout=subprocess.DEVNULL,
        )

    # Modern Windows ICO files support PNG entries at each launcher size.
    entries = []
    images = []
    windows_sizes = [16, 32, 64, 128, 256]
    offset = 6 + 16 * len(windows_sizes)
    for size in windows_sizes:
        png = (catalog / f'app_icon_{size}.png').read_bytes()
        entries.append(struct.pack(
            '<BBBBHHII', size % 256, size % 256, 0, 0, 1, 32, len(png), offset,
        ))
        images.append(png)
        offset += len(png)
    icon = root / 'windows/runner/resources/app_icon.ico'
    icon.write_bytes(
        struct.pack('<HHH', 0, 1, len(entries)) + b''.join(entries + images)
    )
    print('Desktop icons updated from assets/images/logo/logo-app.png')


if __name__ == '__main__':
    main()
