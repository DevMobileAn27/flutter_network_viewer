#!/usr/bin/env python3
"""Prepare signed compatibility feeds; publishing requires --publish explicitly."""
import argparse
import base64
import hashlib
import json
from pathlib import Path
import re
import subprocess
import xml.etree.ElementTree as ET
from urllib.request import urlopen

import release

CHANNEL = 'rust-updates'


def gh(*args):
    return subprocess.check_output(['gh', *args], text=True, encoding='utf-8')


def prepare(source, output, tag):
    if not re.fullmatch(r'rust-v\d+\.\d+\.\d+', tag):
        raise ValueError('Require a numeric Rust release tag')
    version = tag.removeprefix('rust-v')
    manifest = {}
    for line in (source / 'SHA256SUMS.txt').read_text().splitlines():
        digest, name = line.split('  ', 1)
        if not re.fullmatch(r'[a-f0-9]{64}', digest) or name in manifest:
            raise ValueError('Invalid checksum manifest')
        manifest[name] = digest
    feeds = []
    for platform in release.PLATFORMS:
        name = f'appcast-{platform}.xml'
        data = (source / name).read_bytes()
        if hashlib.sha256(data).hexdigest() != manifest.get(name):
            raise ValueError(f'Feed checksum mismatch: {name}')
        item = ET.fromstring(data).find('./channel/item')
        if item is None or item.findtext(f'{{{release.SPARKLE}}}shortVersionString') != version:
            raise ValueError('Appcast version does not match source release')
        enclosure = item.find('enclosure')
        installer = platform == 'windows-x64'
        filename = f'flutter-network-viewer-rust-{version}-{platform}' + ('-setup.exe' if installer else '.zip')
        expected_url = f'{release.REPOSITORY}/releases/download/{tag}/{filename}'
        kind = 'dsaSignature' if installer else 'edSignature'
        if enclosure is None or enclosure.get('url') != expected_url or int(enclosure.get('length', '0')) <= 0:
            raise ValueError('Appcast must point to the versioned release package')
        signature = base64.b64decode(enclosure.get(f'{{{release.SPARKLE}}}{kind}', ''), validate=True)
        if not signature or (not installer and len(signature) != 64):
            raise ValueError('Missing platform signature')
        feeds.append((f'rust-appcast-{platform}.xml', data))
    # No output or remote writes before every platform validates.
    output.mkdir(parents=True, exist_ok=True)
    paths = []
    for name, data in feeds:
        path = output / name
        path.write_bytes(data)
        paths.append(path)
    release.checksums(paths, output / 'SHA256SUMS.txt')
    return paths


def ensure_not_older(path, existing):
    def version(data):
        item = ET.fromstring(data).find('./channel/item')
        return tuple(map(int, item.findtext(f'{{{release.SPARKLE}}}shortVersionString').split('.')))
    if version(path.read_bytes()) < version(existing):
        raise ValueError('Refusing to downgrade the compatibility channel')


def publish(paths, repository, tag):
    pages = json.loads(gh('api', f'repos/{repository}/releases', '--paginate', '--slurp'))
    releases = [r for page in pages for r in page]
    source = next((r for r in releases if r['tag_name'] == tag), None)
    if source is None or source['draft'] or source['prerelease']:
        raise ValueError('Only a published stable release can update the channel')
    channel = next((r for r in releases if r['tag_name'] == CHANNEL), None)
    if channel is not None:
        if channel['draft']:
            raise ValueError('Compatibility channel must be public')
        for path in paths:
            asset = next((a for a in channel['assets'] if a['name'] == path.name), None)
            if asset:
                existing = gh('api', asset['url'], '-H', 'Accept: application/octet-stream')
                ensure_not_older(path, existing)
    else:
        gh('release', 'create', CHANNEL, '--repo', repository, '--target', source['tag_name'],
           '--prerelease', '--latest=false', '--title', 'Rust update feeds',
           '--notes', 'Compatibility feeds for installed Rust clients. Download the app from the latest stable release.',
           *(str(p) for p in paths))
        return
    gh('release', 'upload', CHANNEL, '--repo', repository, '--clobber', *(str(p) for p in paths))


def verify_public(paths, repository, fetch=None):
    if fetch is None:
        def fetch(url):
            with urlopen(url, timeout=30) as response:
                return response.read()
    for path in paths:
        url = f'https://github.com/{repository}/releases/download/{CHANNEL}/{path.name}'
        if fetch(url) != path.read_bytes():
            raise ValueError(f'Unexpected public feed at {url}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('tag')
    parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--repository', default='DevMobileAn27/flutter_network_viewer')
    parser.add_argument('--publish', action='store_true')
    args = parser.parse_args()
    paths = prepare(args.source, args.output, args.tag)
    if args.publish:
        publish(paths, args.repository, args.tag)
        verify_public(paths, args.repository)
    print(f'Validated {len(paths)} compatibility feeds for {args.tag}')


if __name__ == '__main__':
    main()
