#!/usr/bin/env python3
"""Verify actual macOS zip output, symlinks, metadata, executable architecture."""
from pathlib import Path
import json
import plistlib
import stat
import subprocess
import sys
import zipfile
import release
ROOT=Path(__file__).resolve().parents[1]
meta=release.metadata(ROOT)
results=[]
for directory,platform in [(Path(sys.argv[1]),'macos-arm64'),(Path(sys.argv[2]),'macos-x86_64')]:
    archive=directory/release.filename(meta,platform)
    release.verify_checksums(archive.with_suffix('.sha256'))
    app=directory/platform/'Flutter Network Viewer Rust.app'
    binary=app/'Contents/MacOS/flutter-network-viewer'
    subprocess.run(['lipo',str(binary),'-verify_arch','arm64' if platform=='macos-arm64' else 'x86_64'],check=True)
    with zipfile.ZipFile(archive) as package:
        base='Flutter Network Viewer Rust.app/Contents/'
        plist=plistlib.loads(package.read(base+'Info.plist'))
        assert plist['CFBundleIdentifier']==meta['identifier']
        assert plist['CFBundleVersion']==str(meta['build'])
        assert plist['SUPublicEDKey']==meta['public_key']
        for key in ['SUEnableAutomaticChecks','SUAutomaticallyUpdate','SUAllowsAutomaticUpdates']: assert plist[key] is False
        link=package.getinfo(base+'Frameworks/Sparkle.framework/Versions/Current')
        assert stat.S_ISLNK(link.external_attr>>16)
        assert package.read(link)==b'B'
        for name in ['MacOS/flutter-network-viewer','Frameworks/Sparkle.framework/Versions/B/Autoupdate','Frameworks/Sparkle.framework/Versions/B/Updater.app/Contents/MacOS/Updater','Frameworks/Sparkle.framework/Versions/B/XPCServices/Installer.xpc/Contents/MacOS/Installer','Resources/AppIcon.icns','Resources/LICENSE','Resources/fonts/cascadia_mono/OFL.txt','Resources/THIRD_PARTY_NOTICES.md']:
            assert package.getinfo(base+name).file_size>0,name
    results.append({'platform':platform,'archive':str(archive),'sha256_verified':True,'architecture_verified':True,'sparkle_symlinks_helpers':True,'metadata_assets_licenses':True,'code_signed':False,'release_ready':False})
print(json.dumps(results,ensure_ascii=False,indent=2))
