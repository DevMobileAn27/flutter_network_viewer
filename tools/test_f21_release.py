"""Executable contracts written before release implementation; inert fixture binaries."""
import base64
import hashlib
import html
import io
import os
from pathlib import Path
import plistlib
import tempfile
import struct
import unittest
from unittest.mock import patch
import xml.etree.ElementTree as ET
import release

ROOT = Path(__file__).resolve().parents[1]

class ReleaseTests(unittest.TestCase):
    def test_metadata_and_monotonic_version_build(self):
        meta = release.metadata(ROOT)
        self.assertTrue(release.increased(meta['version'], meta['build'], '0.1.8', 9))
        self.assertEqual(meta['tag'], f"rust-v{meta['version']}")
        self.assertEqual(meta['identifier'], 'dev.quocan.flutterNetworkViewer.rust')
        self.assertTrue(release.increased('0.1.10', 11, '0.1.9', 10))
        self.assertFalse(release.increased('0.1.9', 11, '0.1.9', 10))
        for args in [('0.1.8', 12, '0.1.9', 10), ('0.2.0', 10, '0.1.9', 10)]:
            with self.assertRaises(ValueError): release.increased(*args)

    def test_notes_and_outputs_explicit_utf8(self):
        original = io.open
        def cp1252(file, mode='r', buffering=-1, encoding=None, *args, **kwargs):
            if 'b' not in mode and encoding in (None, 'locale'): encoding='cp1252'
            return original(file, mode, buffering, encoding, *args, **kwargs)
        with tempfile.TemporaryDirectory(dir=ROOT/'docs/artifacts/f21') as folder, patch('io.open', side_effect=cp1252):
            path=Path(folder)
            title, body='Bản Rust — 🖼️', '## What’s new\n\nHướng dẫn <script> & ảnh 🖼️\n'
            meta=release.metadata(ROOT)
            meta.update(title=title, notes=body)
            release.prepare(meta,path)
            self.assertIn(title,(path/'actions-output.txt').read_text(encoding='utf-8'))
            self.assertEqual((path/'release-notes.md').read_text(encoding='utf-8'),body)
            xml=release.feed(meta,'windows-x64',release.filename(meta,'windows-x64',installer=True),7,'dsaSignature',base64.b64encode(b'signature').decode())
            item=ET.fromstring(xml).find('channel/item')
            self.assertEqual(item.find('title').text,title)
            self.assertEqual(html.unescape(item.find('description').text),f'<pre>{body}</pre>')

    def test_feed_platform_signatures_and_namespace_fail_closed(self):
        meta=release.metadata(ROOT)
        for platform in release.PLATFORMS:
            mac=platform.startswith('macos')
            filename=release.filename(meta,platform,installer=not mac)
            xml=release.feed(meta,platform,filename,123,'edSignature' if mac else 'dsaSignature',base64.b64encode(bytes(64)).decode())
            item=ET.fromstring(xml).find('channel/item')
            enclosure=item.find('enclosure').attrib
            self.assertIn(f"/download/{meta['tag']}/flutter-network-viewer-rust-",enclosure['url'])
            self.assertEqual(item.find(f'{{{release.SPARKLE}}}version').text,str(meta['build']) if mac else meta['version'])
            if not mac: self.assertEqual(enclosure[f'{{{release.SPARKLE}}}installerArguments'],'/SILENT /NORESTART /SP-')
            for sig,kind,name in [('', 'edSignature',filename),('bad','dsaSignature' if mac else 'edSignature',filename),('bad','edSignature' if mac else 'dsaSignature','flutter-setup.exe')]:
                with self.assertRaises(ValueError): release.feed(meta,platform,name,123,kind,sig)
        with self.assertRaises(ValueError): release.sign_archive(ROOT/'Cargo.toml','macos-arm64','')

    @unittest.skipIf(os.name == 'nt', 'Sparkle symlinks are checked on macOS runners')
    def test_macos_bundle_layout_assets_and_symlinks(self):
        with tempfile.TemporaryDirectory(dir=ROOT/'docs/artifacts/f21') as folder:
            folder=Path(folder)
            binary=folder/'binary'; binary.write_bytes(b'inert fixture'); binary.chmod(0o755)
            meta=release.metadata(ROOT)
            app=release.stage(ROOT,meta,'macos-arm64',binary,folder/'stage')
            info=plistlib.loads((app/'Contents/Info.plist').read_bytes())
            self.assertEqual(app.name, 'Flutter Network Viewer.app')
            self.assertEqual(info['CFBundleName'], 'Flutter Network Viewer')
            self.assertEqual(info['CFBundleDisplayName'], 'Flutter Network Viewer')
            self.assertEqual(info['CFBundleVersion'],str(meta['build']))
            self.assertEqual(info['CFBundleShortVersionString'],meta['version'])
            self.assertEqual(info['CFBundleIdentifier'],meta['identifier'])
            for key in ['SUEnableAutomaticChecks','SUAutomaticallyUpdate','SUAllowsAutomaticUpdates']: self.assertIs(info[key],False)
            self.assertTrue((app/'Contents/Frameworks/Sparkle.framework/Versions/Current').is_symlink())
            for name in ['AppIcon.icns','LICENSE','THIRD_PARTY_NOTICES.md','fonts/cascadia_mono/CascadiaMono-Variable.ttf']:
                self.assertTrue((app/'Contents/Resources'/name).is_file(),name)

    def test_windows_bundle_layout_and_checksums(self):
        with tempfile.TemporaryDirectory(dir=ROOT/'docs/artifacts/f21') as folder:
            folder=Path(folder)
            binary=folder/'binary'; binary.write_bytes(b'inert fixture')
            meta=release.metadata(ROOT)
            win=release.stage(ROOT,meta,'windows-x64',binary,folder/'win')
            self.assertEqual((win/'WinSparkle.dll').read_bytes(),(ROOT/'vendor/winsparkle/x64/WinSparkle.dll').read_bytes())
            manifest=release.checksums([binary],folder/'SHA256SUMS.txt')
            self.assertEqual(manifest.read_text(encoding='utf-8'),hashlib.sha256(binary.read_bytes()).hexdigest()+'  binary\n')
            release.verify_checksums(manifest)
            binary.write_bytes(b'tamper')
            with self.assertRaises(ValueError): release.verify_checksums(manifest)

    def test_manifest_rejects_empty_duplicate_and_path_traversal(self):
        with tempfile.TemporaryDirectory(dir=ROOT/'docs/artifacts/f21') as folder:
            folder=Path(folder); archive=folder/'archive'; archive.write_bytes(b'inert')
            digest=hashlib.sha256(b'inert').hexdigest()
            manifest=folder/'SHA256SUMS.txt'
            for text in ['',digest+'  ../archive\n',(digest+'  archive\n')*2,'bad  archive\n']:
                manifest.write_text(text,encoding='utf-8')
                with self.assertRaises(ValueError): release.verify_checksums(manifest)

    def test_windows_executable_has_version_icon_resource(self):
        source=(ROOT/'native/windows.rc').read_text(encoding='utf-8')
        meta=release.metadata(ROOT)
        version,build=meta['version'],meta['build']
        self.assertIn(f"FILEVERSION {version.replace('.', ',')},{build}",source)
        self.assertIn(f'VALUE "FileVersion", "{version}.{build}\\0"',source)
        self.assertIn(f'VALUE "ProductVersion", "{version}\\0"',source)
        self.assertIn('ICON',source)
        self.assertIn(r'VALUE "ProductName", "Flutter Network Viewer\0"',source)
        self.assertIn('windows.rc',(ROOT/'build.rs').read_text(encoding='utf-8'))

    def test_bundled_font_contains_vietnamese_glyphs_and_icons_are_native(self):
        data=(ROOT/'assets/fonts/cascadia_mono/CascadiaMono-Variable.ttf').read_bytes()
        tables=struct.unpack_from('>H',data,4)[0]
        cmap=None
        for i in range(tables):
            tag,_,offset,_=struct.unpack_from('>4sIII',data,12+16*i)
            if tag==b'cmap': cmap=offset
        self.assertIsNotNone(cmap)
        selected=None
        for i in range(struct.unpack_from('>H',data,cmap+2)[0]):
            _,_,offset=struct.unpack_from('>HHI',data,cmap+4+8*i)
            if struct.unpack_from('>H',data,cmap+offset)[0]==4: selected=cmap+offset
        self.assertIsNotNone(selected)
        count=struct.unpack_from('>H',data,selected+6)[0]//2
        def glyph(char):
            code=ord(char)
            for i in range(count):
                end=struct.unpack_from('>H',data,selected+14+2*i)[0]
                start=struct.unpack_from('>H',data,selected+16+2*count+2*i)[0]
                if start<=code<=end:
                    delta=struct.unpack_from('>h',data,selected+16+4*count+2*i)[0]
                    address=selected+16+6*count+2*i
                    offset=struct.unpack_from('>H',data,address)[0]
                    if not offset: return (code+delta)&65535
                    value=struct.unpack_from('>H',data,address+offset+2*(code-start))[0]
                    return (value+delta)&65535 if value else 0
            return 0
        for char in 'Tiếng Việt Nguyễn Quốc An ăâđêôơư ĂÂĐÊÔƠƯ ắằẳẵặấầẩẫậếềểễệốồổỗộớờởỡợứừửữựỳỷỹỵ':
            if char!=' ': self.assertGreater(glyph(char),0,char)
        self.assertEqual((ROOT/'assets/icons/AppIcon.icns').read_bytes()[:4],b'icns')
        self.assertEqual((ROOT/'assets/icons/AppIcon.ico').read_bytes()[:4],bytes([0,0,1,0]))

    def test_installer_and_ci_contract(self):
        script=(ROOT/'packaging/windows.iss').read_text(encoding='utf-8')
        for text in ['ArchitecturesAllowed=x64compatible','PrivilegesRequired=lowest','CloseApplications=yes','AppId=dev.quocan.flutterNetworkViewer.rust','AppVersion={#AppVersion}','OutputBaseFilename={#OutputName}','WizardStyle=modern']:
            self.assertIn(text,script)
        self.assertIn('AppName=Flutter Network Viewer\n',script)
        workflow=(ROOT/'.github/workflows/checks.yml').read_text(encoding='utf-8')
        for text in ['aarch64-apple-darwin','x86_64-apple-darwin','x86_64-pc-windows-msvc','cargo test --locked','cargo clippy --all-targets --locked','cargo fmt --check','test_f21_release.py','tools/release.py','contents: read']:
            self.assertIn(text,workflow)
        self.assertRegex(workflow, r'(?m)^  push:\s*\n    branches: \[main\]$')
        self.assertNotIn('gh release',workflow)

if __name__=='__main__': unittest.main()
