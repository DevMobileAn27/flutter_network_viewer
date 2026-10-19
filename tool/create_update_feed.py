#!/usr/bin/env python3
"""Sign one desktop update and create its platform-specific Sparkle appcast."""

import argparse
import base64
from datetime import datetime, timezone
from email.utils import format_datetime
import hashlib
import html
import os
from pathlib import Path
import subprocess
import tempfile
import xml.etree.ElementTree as ET

from prepare_github_release import app_build, app_version, release_info

SPARKLE = 'http://www.andymatuschak.org/xml-namespaces/sparkle'
REPOSITORY = 'https://github.com/DevMobileAn27/flutter_network_viewer'
ET.register_namespace('sparkle', SPARKLE)


def sign_archive(archive, platform, private_key):
    if not private_key.strip():
        raise ValueError('Update signing key is missing; unsigned updates are not allowed')
    if platform.startswith('macos-'):
        result = subprocess.check_output(
            ['swift', 'tool/sign_macos_update.swift', str(archive)],
            input=private_key.encode(),
        )
        return 'edSignature', result.decode().strip()
    with tempfile.TemporaryDirectory(prefix='fnv-sign-') as folder:
        key = Path(folder) / 'private.pem'
        key.write_text(private_key)
        key.chmod(0o600)
        public = subprocess.check_output(['openssl', 'dsa', '-in', str(key), '-pubout'],
                                         stderr=subprocess.DEVNULL)
        if public.strip() != Path('updates/windows_dsa_pub.pem').read_bytes().strip():
            raise ValueError('Windows signing key does not match the embedded public key')
        # WinSparkle 0.8.x sign_update.bat signs a SHA-1 digest of the file.
        digest = hashlib.sha1(archive.read_bytes()).digest()
        signature = subprocess.check_output(
            ['openssl', 'dgst', '-sha1', '-sign', str(key)], input=digest,
        )
        return 'dsaSignature', base64.b64encode(signature).decode()


def feed_xml(platform, version, build, title, description, filename, length, signature_type, signature):
    if not signature:
        raise ValueError('Missing update signature')
    root = ET.Element('rss', {'version': '2.0'})
    channel = ET.SubElement(root, 'channel')
    ET.SubElement(channel, 'title').text = 'Flutter Network Viewer'
    ET.SubElement(channel, 'link').text = REPOSITORY
    item = ET.SubElement(channel, 'item')
    ET.SubElement(item, 'title').text = title
    ET.SubElement(item, 'pubDate').text = format_datetime(datetime.now(timezone.utc))
    ET.SubElement(item, 'description').text = '<pre>' + html.escape(description) + '</pre>'
    ET.SubElement(item, f'{{{SPARKLE}}}version').text = build if platform.startswith('macos-') else version
    ET.SubElement(item, f'{{{SPARKLE}}}shortVersionString').text = version
    if platform.startswith('macos-'):
        ET.SubElement(item, f'{{{SPARKLE}}}minimumSystemVersion').text = '12.0'
    enclosure = {
        'url': f'{REPOSITORY}/releases/download/v{version}/{filename}',
        'length': str(length),
        'type': 'application/octet-stream',
        f'{{{SPARKLE}}}os': 'macos' if platform.startswith('macos-') else 'windows',
        f'{{{SPARKLE}}}{signature_type}': signature,
    }
    if platform == 'windows-x64':
        enclosure[f'{{{SPARKLE}}}version'] = version
        enclosure[f'{{{SPARKLE}}}installerArguments'] = '/SILENT /NORESTART /SP-'
    ET.SubElement(item, 'enclosure', enclosure)
    ET.indent(root)
    return ET.tostring(root, encoding='utf-8', xml_declaration=True)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('platform', choices=['macos-arm64', 'macos-x86_64', 'windows-x64'])
    parser.add_argument('archive', type=Path)
    args = parser.parse_args()
    source = Path('pubspec.yaml').read_text()
    version = app_version(source)
    build = str(app_build(source))
    _, title, body = release_info(Path('github_release_info.md').read_text(), version)
    extension = '-setup.exe' if args.platform == 'windows-x64' else '.zip'
    expected = f'flutter-network-viewer-{version}-{args.platform}{extension}'
    if args.archive.name != expected:
        raise ValueError(f'Expected update filename {expected}')
    signature_type, signature = sign_archive(args.archive, args.platform, os.environ.get('UPDATE_SIGNING_KEY', ''))
    output = args.archive.parent / f'appcast-{args.platform}.xml'
    output.write_bytes(feed_xml(args.platform, version, build, title, body,
                                args.archive.name, args.archive.stat().st_size,
                                signature_type, signature))
    print(f'Signed update feed: {output.name}')


if __name__ == '__main__':
    main()
