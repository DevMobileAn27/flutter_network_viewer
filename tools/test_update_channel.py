"""Verify compatibility feed preparation without publishing or installing."""
import base64
import json
import io
import tempfile
from pathlib import Path
import unittest
from unittest.mock import patch
import release
import update_channel


class UpdateChannelTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.source = self.root / 'source'
        self.source.mkdir()
        self.meta = release.metadata(release.ROOT)
        self.meta['notes'] += '\nKiểm thử “cập nhật” tiếng Việt.\n'
        self.feeds = []
        for platform in release.PLATFORMS:
            installer = platform == 'windows-x64'
            archive = release.filename(self.meta, platform, installer=installer)
            kind = 'dsaSignature' if installer else 'edSignature'
            path = self.source / f'appcast-{platform}.xml'
            path.write_bytes(release.feed(self.meta, platform, archive, 123, kind,
                                         base64.b64encode(bytes(64)).decode()))
            self.feeds.append(path)
        release.checksums(self.feeds, self.source / 'SHA256SUMS.txt')

    def test_prepares_all_old_client_urls_without_changing_signed_feed_bytes(self):
        output = self.root / 'output'
        paths = update_channel.prepare(self.source, output, self.meta['tag'])
        self.assertEqual(len(paths), 3)
        for platform in release.PLATFORMS:
            self.assertEqual((output / f'rust-appcast-{platform}.xml').read_bytes(),
                             (self.source / f'appcast-{platform}.xml').read_bytes())
        release.verify_checksums(output / 'SHA256SUMS.txt')

    def test_rejects_corrupt_feed_missing_platform_or_wrong_version_before_writing(self):
        for failure in ['corrupt', 'missing', 'version']:
            with self.subTest(failure=failure):
                original = self.feeds[0].read_bytes()
                if failure == 'corrupt':
                    self.feeds[0].write_bytes(original + b' ')
                elif failure == 'missing':
                    self.feeds[0].unlink()
                tag = 'rust-v9.9.9' if failure == 'version' else self.meta['tag']
                with self.assertRaises((ValueError, FileNotFoundError)):
                    update_channel.prepare(self.source, self.root / failure, tag)
                self.assertFalse((self.root / failure).exists())
                self.feeds[0].write_bytes(original)

    def test_rejects_unsigned_or_wrong_package_even_with_valid_checksum(self):
        for old, new in [(b'edSignature', b'unsigned'), (b'rust-v', b'wrong-v')]:
            original = self.feeds[0].read_bytes()
            self.feeds[0].write_bytes(original.replace(old, new))
            release.checksums(self.feeds, self.source / 'SHA256SUMS.txt')
            with self.assertRaises(ValueError):
                update_channel.prepare(self.source, self.root / 'bad', self.meta['tag'])
            self.feeds[0].write_bytes(original)

    def test_publish_never_promotes_channel_to_latest_and_rejects_downgrade(self):
        paths = update_channel.prepare(self.source, self.root / 'output', self.meta['tag'])
        with patch.object(update_channel, 'gh', return_value=json.dumps([[{'tag_name': self.meta['tag'], 'draft': False, 'prerelease': False}]])) as gh:
            update_channel.publish(paths, 'owner/repo', self.meta['tag'])
            create = next(c.args for c in gh.call_args_list if c.args[:2] == ('release', 'create'))
            self.assertIn('--prerelease', create)
            self.assertIn('--latest=false', create)
        newer = self.feeds[0].read_text(encoding='utf-8').replace(self.meta['version'], '9999.0.0')
        with patch.object(update_channel, 'gh', return_value=newer):
            with self.assertRaisesRegex(ValueError, 'downgrade'):
                update_channel.ensure_not_older(paths[0], newer)

    def test_rejects_unpublished_source_without_remote_changes(self):
        paths = update_channel.prepare(self.source, self.root / 'output', self.meta['tag'])
        for draft, prerelease in [(True, False), (False, True)]:
            payload = [[{'tag_name': self.meta['tag'], 'draft': draft, 'prerelease': prerelease}]]
            with patch.object(update_channel, 'gh', return_value=json.dumps(payload)) as gh:
                with self.assertRaises(ValueError):
                    update_channel.publish(paths, 'owner/repo', self.meta['tag'])
                self.assertEqual(gh.call_count, 1)

    def test_existing_public_channel_uploads_only_feeds_after_version_check(self):
        paths = update_channel.prepare(self.source, self.root / 'output', self.meta['tag'])
        source = {'tag_name': self.meta['tag'], 'draft': False, 'prerelease': False}
        assets = [{'name': path.name, 'url': 'asset-' + str(i)} for i, path in enumerate(paths)]
        channel = {'tag_name': 'rust-updates', 'draft': False, 'assets': assets}
        def response(*args):
            if args[1].startswith('repos/'):
                return json.dumps([[source, channel]])
            if args[0] == 'api':
                return paths[int(args[1].split('-')[1])].read_text(encoding='utf-8')
            return ''
        with patch.object(update_channel, 'gh', side_effect=response) as gh:
            update_channel.publish(paths, 'owner/repo', self.meta['tag'])
            calls = [call.args for call in gh.call_args_list]
            upload = calls[-1]
            self.assertEqual(upload[:3], ('release', 'upload', 'rust-updates'))
            self.assertEqual(upload[-3:], tuple(map(str, paths)))
            self.assertFalse(any(call[:2] == ('release', 'create') for call in calls))

    def test_channel_flows_with_windows_cp1252_default_encoding(self):
        # Emulate Windows without UTF-8 mode; explicit encodings still win.
        original_open = Path.open
        def windows_open(path, mode='r', buffering=-1, encoding=None,
                         errors=None, newline=None):
            if 'b' not in mode and encoding is None:
                encoding = 'cp1252'
            return original_open(path, mode, buffering, encoding, errors, newline)
        with patch.object(Path, 'open', windows_open), patch.object(
                io, 'text_encoding', lambda encoding, *args: encoding or 'cp1252'):
            self.test_prepares_all_old_client_urls_without_changing_signed_feed_bytes()
            self.test_publish_never_promotes_channel_to_latest_and_rejects_downgrade()
            self.test_existing_public_channel_uploads_only_feeds_after_version_check()

    def test_public_feed_verification_checks_the_installed_client_urls_and_bytes(self):
        paths = update_channel.prepare(self.source, self.root / 'output', self.meta['tag'])
        fetched = []
        def fetch(url):
            fetched.append(url)
            return next(p.read_bytes() for p in paths if url.endswith('/' + p.name))
        update_channel.verify_public(paths, 'owner/repo', fetch)
        self.assertEqual(set(fetched), {
            'https://github.com/owner/repo/releases/download/rust-updates/' + p.name
            for p in paths})
        with self.assertRaisesRegex(ValueError, 'public feed'):
            update_channel.verify_public(paths, 'owner/repo', lambda _: b'old feed')


if __name__ == '__main__':
    unittest.main()
