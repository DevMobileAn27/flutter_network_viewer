#!/usr/bin/env python3
"""Create or refresh a draft from verified CI artifacts; never publish it."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess

import release


def gh(*args):
    return subprocess.check_output(['gh', *args], text=True, encoding='utf-8')


def release_notes(meta):
    return f'''## Có gì mới

{meta['notes'].strip()}

## Bản tải

- **macOS Apple Silicon (M-series):** `{release.filename(meta, 'macos-arm64')}`
- **macOS Intel:** `{release.filename(meta, 'macos-x86_64')}`
- **Windows x64 installer:** `{release.filename(meta, 'windows-x64', installer=True)}`
- **Windows x64 portable:** `{release.filename(meta, 'windows-x64')}`

**macOS 12 trở lên:** giải nén ZIP và kéo **Flutter Network Viewer.app** vào **Applications**.

**Windows:** chạy installer. Với bản portable, giải nén toàn bộ ZIP rồi mở **flutter-network-viewer.exe**; giữ các tệp đi kèm cạnh tệp thực thi.

Dùng `SHA256SUMS.txt` để kiểm tra file tải xuống.

### Nếu macOS chặn ứng dụng

Sau khi chuyển ứng dụng vào **Applications**, nếu macOS báo nhà phát triển chưa được xác minh hoặc ứng dụng bị hỏng, chạy các lệnh sau trong **Terminal**:

```sh
xattr -dr com.apple.quarantine "/Applications/Flutter Network Viewer.app"
open "/Applications/Flutter Network Viewer.app"
```

Các lệnh trên gỡ thuộc tính cách ly (quarantine) và mở lại ứng dụng. Chỉ sử dụng với bản tải xuống đáng tin cậy từ GitHub Releases chính thức của kho mã nguồn này.
'''


def create_draft(artifacts, output, repository, commit):
    meta = release.metadata(release.ROOT)
    assets = []
    checksummed = []
    # Keep artifacts separate: each build has its own SHA256SUMS.txt.
    for platform in release.PLATFORMS:
        folder = artifacts / f'rust-preview-{meta["version"]}-{platform}'
        manifest = folder / 'SHA256SUMS.txt'
        release.verify_checksums(manifest)
        covered = {line.split('  ', 1)[1] for line in manifest.read_text(encoding='utf-8').splitlines()}
        names = [release.filename(meta, platform)]
        if platform == 'windows-x64':
            names.append(release.filename(meta, platform, installer=True))
        appcast = folder / f'appcast-{platform}.xml'
        if covered != {*names, appcast.name}:
            raise ValueError(f'Manifest must cover all expected packages and appcast for {platform}')
        release.validate_legacy_appcast(meta, platform, appcast, folder / names[-1])
        package_assets = [*(folder / name for name in names), appcast]
        checksummed.extend(package_assets)
        assets.extend(package_assets)
        status = folder / f'{platform}-status.json'
        json.loads(status.read_text(encoding='utf-8'))
        assets.append(status)

    # Only contact GitHub after all three platform artifacts validate.
    pages = json.loads(gh('api', f'repos/{repository}/releases', '--paginate', '--slurp'))
    matching = [item for page in pages for item in page if item['tag_name'] == meta['tag']]
    if any(not item['draft'] for item in matching):
        raise ValueError('This version is already published; increase the version before releasing again')

    output.mkdir(parents=True, exist_ok=True)
    for asset in assets:
        shutil.copy2(asset, output / asset.name)
    manifest = release.checksums([output / path.name for path in checksummed], output / 'SHA256SUMS.txt')
    release.verify_checksums(manifest)
    notes = output / 'release-notes.md'
    notes.write_text(release_notes(meta), encoding='utf-8')
    common = ('--repo', repository, '--target', commit, '--title', meta['title'], '--notes-file', str(notes))
    if matching:
        gh('release', 'edit', meta['tag'], '--draft=true', *common)
    else:
        gh('release', 'create', meta['tag'], '--draft', *common)
    gh('release', 'upload', meta['tag'], '--repo', repository, '--clobber',
       *(str(output / path.name) for path in assets), str(manifest), str(notes))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('artifacts', type=Path)
    parser.add_argument('--output', type=Path, default=Path('draft-assets'))
    args = parser.parse_args()
    create_draft(args.artifacts, args.output, os.environ['GITHUB_REPOSITORY'], os.environ['GITHUB_SHA'])


if __name__ == '__main__':
    main()
