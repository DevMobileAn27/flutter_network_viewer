"""Exercise release file I/O with Windows' legacy default text encoding."""

import html
import io
import os
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import xml.etree.ElementTree as ET

import create_update_feed
import prepare_github_release


class ReleaseEncodingTest(unittest.TestCase):
    def setUp(self):
        folder = tempfile.TemporaryDirectory(prefix='fnv-encoding-test-')
        self.addCleanup(folder.cleanup)
        self.root = Path(folder.name)
        previous = Path.cwd()
        os.chdir(self.root)
        self.addCleanup(os.chdir, previous)
        self.title = 'Flutter Network Viewer — Bản cập nhật'
        self.body = '## What’s new\n\n- Hướng dẫn sử dụng và xem ảnh 🖼️\n'
        Path('pubspec.yaml').write_text(
            '# Ứng dụng Flutter\nversion: 0.1.3+4\n', encoding='utf-8',
        )
        Path('github_release_info.md').write_text(
            f'---\ntag: v0.1.3\ntitle: {self.title}\n---\n\n{self.body}',
            encoding='utf-8',
        )

        original_open = io.open

        def windows_open(file, mode='r', buffering=-1, encoding=None, *args, **kwargs):
            if 'b' not in mode and encoding in (None, 'locale'):
                encoding = 'cp1252'
            return original_open(file, mode, buffering, encoding, *args, **kwargs)

        for target in ('io.open', 'builtins.open'):
            patched = patch(target, side_effect=windows_open)
            patched.start()
            self.addCleanup(patched.stop)

    def test_windows_appcast_preserves_utf8_release_notes(self):
        archive = Path('flutter-network-viewer-0.1.3-windows-x64-setup.exe')
        archive.write_bytes(b'test installer')
        with patch('sys.argv', [
            'create_update_feed.py', 'windows-x64', str(archive),
        ]), patch.object(create_update_feed, 'sign_archive', return_value=(
            'dsaSignature', 'test-signature',
        )) as signer:
            create_update_feed.main()
        signer.assert_called_once()
        item = ET.fromstring(Path('appcast-windows-x64.xml').read_bytes()).find('channel/item')
        self.assertEqual(item.find('title').text, self.title)
        self.assertEqual(html.unescape(item.find('description').text),
                         f'<pre>{self.body}</pre>')

    def test_release_notes_and_actions_outputs_are_utf8(self):
        with patch.dict(os.environ, {
            'BEFORE_COMMIT': '',
            'GITHUB_SHA': 'test-commit',
            'GITHUB_OUTPUT': str(self.root / 'output'),
            'GITHUB_STEP_SUMMARY': str(self.root / 'summary'),
        }), patch.object(prepare_github_release, 'verify_tag_commit'):
            prepare_github_release.main()
        self.assertEqual(Path('build/release-notes.md').read_text(encoding='utf-8'),
                         self.body)
        self.assertIn(f'title={self.title}\n', Path('output').read_text(encoding='utf-8'))


if __name__ == '__main__':
    unittest.main()
