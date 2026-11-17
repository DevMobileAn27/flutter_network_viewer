#!/usr/bin/env python3
"""Local Rust build packaging. Never publishes or installs. Python >=3.11."""
import argparse
import base64
import hashlib
import html
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import tempfile
import tomllib
import xml.etree.ElementTree as ET
from verify_updater_vendor import verify

ROOT=Path(__file__).resolve().parents[1]
PLATFORMS={'macos-arm64':'aarch64-apple-darwin','macos-x86_64':'x86_64-apple-darwin','windows-x64':'x86_64-pc-windows-msvc'}
SPARKLE='http://www.andymatuschak.org/xml-namespaces/sparkle'
REPOSITORY='https://github.com/DevMobileAn27/flutter_network_viewer'
ET.register_namespace('sparkle',SPARKLE)


def metadata(root):
    version=tomllib.loads((root/'Cargo.toml').read_text(encoding='utf-8'))['package']['version']
    meta=json.loads((root/'packaging/release.json').read_text(encoding='utf-8'))
    if not re.fullmatch(r'\d+\.\d+\.\d+',version) or type(meta['build']) is not int or meta['build']<1:
        raise ValueError('Require numeric X.Y.Z version and positive build')
    if meta['identifier']!='dev.quocan.flutterNetworkViewer.rust' or len(base64.b64decode(meta['public_key'],validate=True))!=32:
        raise ValueError('Invalid Rust identity or trust anchor')
    if not meta['notes'].strip():
        raise ValueError('Require release notes')
    resource=(root/'native/windows.rc').read_text(encoding='utf-8')
    expected=version.replace('.',',')+','+str(meta['build'])
    if f'FILEVERSION {expected}' not in resource or f'PRODUCTVERSION {expected}' not in resource:
        raise ValueError('Windows version resource must match Cargo version/build')
    return dict(meta,version=version,tag=f'rust-v{version}',title=f'Flutter Network Viewer v{version}')


def increased(version,build,previous,previous_build):
    a,b=tuple(map(int,version.split('.'))),tuple(map(int,previous.split('.')))
    if a<b: raise ValueError('Version decreased')
    if a==b: return False
    if build<=previous_build: raise ValueError('Increase macOS build number')
    return True


def filename(meta,platform,installer=False):
    if platform not in PLATFORMS or (installer and platform!='windows-x64'): raise ValueError('Unsupported package')
    return f'flutter-network-viewer-rust-{meta["version"]}-{platform}'+('-setup.exe' if installer else '.zip')


def prepare(meta,output):
    output.mkdir(parents=True,exist_ok=True)
    (output/'release-notes.md').write_text(meta['notes'],encoding='utf-8')
    (output/'actions-output.txt').write_text(f'version={meta["version"]}\nbuild={meta["build"]}\ntag={meta["tag"]}\ntitle={meta["title"]}\n',encoding='utf-8')


def feed(meta,platform,name,length,kind,signature):
    mac=platform.startswith('macos-')
    if name!=filename(meta,platform,installer=not mac) or length<=0 or kind!=('edSignature' if mac else 'dsaSignature') or not signature:
        raise ValueError('Invalid archive identity, length or platform signature')
    try: raw=base64.b64decode(signature,validate=True)
    except Exception as error: raise ValueError('Invalid signature encoding') from error
    if not raw or (mac and len(raw)!=64): raise ValueError('Invalid signature length')
    root=ET.Element('rss',version='2.0'); channel=ET.SubElement(root,'channel')
    ET.SubElement(channel,'title').text='Flutter Network Viewer'
    ET.SubElement(channel,'link').text=REPOSITORY
    item=ET.SubElement(channel,'item'); ET.SubElement(item,'title').text=meta['title']
    ET.SubElement(item,'description').text=''.join(
        '<p>'+'<br>'.join(html.escape(line) for line in paragraph.splitlines())+'</p>'
        for paragraph in meta['notes'].strip().split('\n\n') if paragraph.strip()
    )
    ET.SubElement(item,f'{{{SPARKLE}}}version').text=str(meta['build']) if mac else meta['version']
    ET.SubElement(item,f'{{{SPARKLE}}}shortVersionString').text=meta['version']
    if mac: ET.SubElement(item,f'{{{SPARKLE}}}minimumSystemVersion').text='12.0'
    attributes={'url':f'{REPOSITORY}/releases/download/{meta["tag"]}/{name}','length':str(length),'type':'application/octet-stream',f'{{{SPARKLE}}}os':'macos' if mac else 'windows',f'{{{SPARKLE}}}{kind}':signature}
    if not mac: attributes.update({f'{{{SPARKLE}}}version':meta['version'],f'{{{SPARKLE}}}installerArguments':'/SILENT /NORESTART /SP-'})
    ET.SubElement(item,'enclosure',attributes); ET.indent(root)
    return ET.tostring(root,encoding='utf-8',xml_declaration=True)


def sign_archive(archive,platform,private_key):
    if not private_key.strip(): raise ValueError('Missing private update key; unsigned feeds forbidden')
    meta=metadata(ROOT)
    if platform not in PLATFORMS: raise ValueError('Unsupported signing platform')
    if platform.startswith('macos-'):
        sig=subprocess.check_output(['swift','-module-cache-path',str(ROOT/'target/swift-cache'),str(ROOT/'tools/sign_macos_update.swift'),str(archive),meta['public_key']],input=private_key.encode('utf-8')).decode('utf-8').strip()
        return 'edSignature',sig
    with tempfile.TemporaryDirectory(dir=ROOT/'target') as folder:
        key=Path(folder)/'private.pem'; key.write_text(private_key,encoding='utf-8'); key.chmod(0o600)
        public=subprocess.check_output(['openssl','dsa','-in',str(key),'-pubout'],stderr=subprocess.DEVNULL)
        if public.strip()!=(ROOT/'native/windows_dsa_pub.pem').read_bytes().strip(): raise ValueError('Private key mismatches embedded DSA trust anchor')
        # WinSparkle 0.8.x signs SHA1(archive) with SHA1+DSA, per vendor sign_update.
        digest=hashlib.sha1(archive.read_bytes()).digest()
        signature=subprocess.check_output(['openssl','dgst','-sha1','-sign',str(key)],input=digest)
        sigfile=Path(folder)/'signature'; sigfile.write_bytes(signature)
        subprocess.run(['openssl','dgst','-sha1','-verify',str(ROOT/'native/windows_dsa_pub.pem'),'-signature',str(sigfile)],input=digest,check=True,stdout=subprocess.DEVNULL)
        return 'dsaSignature',base64.b64encode(signature).decode('ascii')


def validate_legacy_appcast(meta, platform, appcast, archive):
    kind = 'dsaSignature' if platform == 'windows-x64' else 'edSignature'
    data = appcast.read_bytes()
    try:
        enclosure = ET.fromstring(data).find('./channel/item/enclosure')
        signature = enclosure.get(f'{{{SPARKLE}}}{kind}', '') if enclosure is not None else ''
        expected = feed(meta, platform, archive.name, archive.stat().st_size, kind, signature)
    except (ET.ParseError, ValueError) as error:
        raise ValueError(f'Appcast is invalid for {platform}') from error
    if data != expected:
        raise ValueError(f'Appcast does not describe the expected {platform} package')


def checksums(paths,output):
    output.write_text(''.join(hashlib.sha256(p.read_bytes()).hexdigest()+'  '+p.name+'\n' for p in sorted(paths)),encoding='utf-8')
    return output


def verify_checksums(manifest):
    lines=manifest.read_text(encoding='utf-8').splitlines()
    if not lines: raise ValueError('Empty checksum manifest')
    seen=set()
    for line in lines:
        if '  ' not in line: raise ValueError('Malformed checksum entry')
        digest,name=line.split('  ',1)
        if not re.fullmatch(r'[0-9a-f]{64}',digest) or not name or name in seen or '/' in name or '\\' in name or name in ('.','..'):
            raise ValueError('Invalid checksum, duplicate or nonlocal filename')
        seen.add(name)
        if hashlib.sha256((manifest.parent/name).read_bytes()).hexdigest()!=digest:
            raise ValueError('Checksum mismatch')


def stage(root,meta,platform,binary,output):
    verify(root/'vendor')
    if platform not in PLATFORMS: raise ValueError('Unsupported platform')
    if platform.startswith('macos-'):
        app=output/'Flutter Network Viewer.app'; contents=app/'Contents'
        resources=contents/'Resources'; executable=contents/'MacOS/flutter-network-viewer'
        executable.parent.mkdir(parents=True,exist_ok=False)
        shutil.copytree(root/'vendor/sparkle/Sparkle.framework',contents/'Frameworks/Sparkle.framework',symlinks=True)
        info={'CFBundleName':'Flutter Network Viewer','CFBundleDisplayName':'Flutter Network Viewer','CFBundleExecutable':'flutter-network-viewer','CFBundleIdentifier':meta['identifier'],'CFBundlePackageType':'APPL','CFBundleVersion':str(meta['build']),'CFBundleShortVersionString':meta['version'],'LSMinimumSystemVersion':'12.0','NSHighResolutionCapable':True,'CFBundleIconFile':'AppIcon.icns','SUPublicEDKey':meta['public_key'],'SUEnableInstallerLauncherService':True,'SUEnableAutomaticChecks':False,'SUAutomaticallyUpdate':False,'SUAllowsAutomaticUpdates':False}
        (contents/'Info.plist').write_bytes(plistlib.dumps(info)); resources.mkdir()
        shutil.copy2(root/'assets/icons/AppIcon.icns',resources/'AppIcon.icns')
    else:
        app=output; app.mkdir(parents=True,exist_ok=False); resources=app
        executable=app/'flutter-network-viewer.exe'
        shutil.copy2(root/'vendor/winsparkle/x64/WinSparkle.dll',app/'WinSparkle.dll')
        shutil.copy2(root/'assets/icons/AppIcon.ico',app/'AppIcon.ico')
    shutil.copy2(binary,executable)
    shutil.copy2(root/'LICENSE',resources/'LICENSE')
    shutil.copy2(root/'packaging/THIRD_PARTY_NOTICES.md',resources/'THIRD_PARTY_NOTICES.md')
    shutil.copytree(root/'assets/fonts',resources/'fonts')
    shutil.copytree(root/'vendor',resources/'updater-licenses',ignore=shutil.ignore_patterns('Sparkle.framework','bin','include','x64','SHA256SUMS'))
    return app


def validate_msvc_environment():
    # Git Bash prepends Git/usr/bin, which also contains an unrelated link.exe.
    # Check PATH before Cargo builds host-side proc macros and build scripts.
    tools=os.environ.get('VCToolsInstallDir')
    linker=shutil.which('link.exe')
    if not tools or not linker:
        raise ValueError('MSVC developer environment required: VCToolsInstallDir and link.exe')
    try:
        Path(linker).resolve().relative_to(Path(tools).resolve())
    except ValueError as error:
        raise ValueError(f'MSVC linker required; PATH resolved link.exe to {linker}. Run from a Visual Studio developer PowerShell, not Git Bash.') from error
    print(f'MSVC linker: {linker}')


def package(platform,output,identity=None):
    meta=metadata(ROOT); target=PLATFORMS[platform]
    if platform=='windows-x64' and os.name=='nt': validate_msvc_environment()
    subprocess.run(['cargo','build','--release','--locked','--offline','--target',target],cwd=ROOT,check=True)
    binary=ROOT/'target'/target/'release'/('flutter-network-viewer.exe' if platform=='windows-x64' else 'flutter-network-viewer')
    if platform.startswith('macos-'):
        subprocess.run(['lipo',str(binary),'-verify_arch','arm64' if platform=='macos-arm64' else 'x86_64'],check=True)
    app=stage(ROOT,meta,platform,binary,output/platform)
    if identity:
        if not platform.startswith('macos-'): raise ValueError('Use Windows Authenticode tooling separately')
        # Sign inside-out. Sparkle helpers and XPC services precede the framework.
        framework=app/'Contents/Frameworks/Sparkle.framework'
        code=framework/'Versions/B'
        for path in [code/'XPCServices/Downloader.xpc',code/'XPCServices/Installer.xpc',code/'Updater.app',code/'Autoupdate',framework,app]:
            subprocess.run(['codesign','--force','--preserve-metadata=entitlements,requirements,flags','--options','runtime','--timestamp','--sign',identity,str(path)],check=True)
    elif platform.startswith('macos-'):
        # The Mach-O linker signature doesn't seal app resources. Keep the
        # vendor signatures on Sparkle and seal the outer bundle before ZIP.
        subprocess.run(['codesign','--force','--sign','-',str(app)],check=True)
    if platform.startswith('macos-'):
        subprocess.run(['codesign','--verify','--deep','--strict',str(app)],check=True)
    archive=output/filename(meta,platform)
    if archive.exists(): raise ValueError('Refusing to overwrite archive')
    if platform.startswith('macos-'): subprocess.run(['ditto','-c','-k','--sequesterRsrc','--keepParent',str(app),str(archive)],check=True)
    else: shutil.make_archive(str(archive.with_suffix('')),'zip',app)
    checksums([archive],archive.with_suffix('.sha256')); prepare(meta,output)
    signing='developer-id' if identity else 'ad-hoc' if platform.startswith('macos-') else 'none'
    (output/f'{platform}-status.json').write_text(json.dumps({'code_signed':signing!='none','signing':signing,'notarized':False,'update_feed_generated':False,'release_ready':False},indent=2),encoding='utf-8')
    return archive


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    sub=parser.add_subparsers(dest='command',required=True)
    p=sub.add_parser('package'); p.add_argument('platform',choices=PLATFORMS); p.add_argument('--output',type=Path,default=ROOT/'dist'); p.add_argument('--identity'); p.add_argument('--github-output',action='store_true',help='Append validated package outputs to GITHUB_OUTPUT after a successful build')
    p=sub.add_parser('feed'); p.add_argument('platform',choices=PLATFORMS); p.add_argument('archive',type=Path); p.add_argument('--legacy',action='store_true',help='Use the appcast filename requested by the Flutter updater')
    p=sub.add_parser('manifest'); p.add_argument('directory',type=Path)
    p=sub.add_parser('verify'); p.add_argument('manifest',type=Path)
    args=parser.parse_args()
    if args.command=='package':
        args.output.mkdir(parents=True,exist_ok=True); print(package(args.platform,args.output,args.identity))
        if args.github_output:
            with Path(os.environ['GITHUB_OUTPUT']).open('a',encoding='utf-8',newline='') as stream:
                stream.write((args.output/'actions-output.txt').read_text(encoding='utf-8'))
    elif args.command=='manifest':
        paths=[p for p in args.directory.iterdir() if p.suffix in ('.zip','.exe','.xml')]
        if not paths: raise ValueError('No artifacts to checksum')
        checksums(paths,args.directory/'SHA256SUMS.txt')
    elif args.command=='verify': verify_checksums(args.manifest)
    else:
        archive=args.archive.resolve(); meta=metadata(ROOT)
        # Validate identity before reading keys or signing anything.
        if archive.name!=filename(meta,args.platform,installer=args.platform=='windows-x64'): raise ValueError('Unexpected Rust update archive filename')
        prefix='appcast' if args.legacy else 'rust-appcast'
        output=archive.parent/f'{prefix}-{args.platform}.xml'
        if output.exists(): raise ValueError('Refusing to overwrite feed')
        kind,sig=sign_archive(archive,args.platform,os.environ.get('UPDATE_SIGNING_KEY',''))
        output.write_bytes(feed(meta,args.platform,archive.name,archive.stat().st_size,kind,sig))
        checksums([archive,output],archive.parent/f'{args.platform}-signed.sha256')
        status=archive.parent/f'{args.platform}-status.json'
        if status.is_file():
            flags=json.loads(status.read_text(encoding='utf-8'))
            flags['update_feed_generated']=True
            status.write_text(json.dumps(flags,indent=2),encoding='utf-8')

if __name__=='__main__': main()
