"""Draft release orchestration with inert assets and a mocked GitHub CLI."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import draft_release
import release


class DraftReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.artifacts = self.root / 'artifacts'
        self.output = self.root / 'output'
        self.meta = release.metadata(release.ROOT)
        self.expected = []
        for platform in release.PLATFORMS:
            folder = self.artifacts / f'rust-preview-{self.meta["version"]}-{platform}'
            folder.mkdir(parents=True)
            names = [release.filename(self.meta, platform)]
            if platform == 'windows-x64':
                names.append(release.filename(self.meta, platform, installer=True))
            paths = []
            for name in names:
                path = folder / name
                path.write_bytes(('inert ' + name).encode())
                paths.append(path)
            self.expected.extend(names)
            release.checksums(paths, folder / 'SHA256SUMS.txt')
            (folder / f'{platform}-status.json').write_text('{}')

    def run_release(self, existing):
        calls = []
        def gh(*args):
            calls.append(args)
            return json.dumps([existing]) if args[0] == 'api' else ''
        with patch.object(draft_release, 'gh', side_effect=gh):
            draft_release.create_draft(self.artifacts, self.output, 'owner/repo', 'a' * 40)
        return calls

    def test_creates_draft_with_all_platform_assets_and_combined_checksums(self):
        calls = self.run_release([])
        create = next(c for c in calls if c[:2] == ('release', 'create'))
        self.assertIn('--draft', create)
        self.assertEqual(create[create.index('--target') + 1], 'a' * 40)
        self.assertEqual(create[2], self.meta['tag'])
        self.assertEqual(create[create.index('--title') + 1], f'Flutter Network Viewer v{self.meta["version"]}')
        upload = next(c for c in calls if c[:2] == ('release', 'upload'))
        self.assertTrue(set(self.expected).issubset({Path(s).name for s in upload}))
        release.verify_checksums(self.output / 'SHA256SUMS.txt')
        self.assertEqual(len((self.output / 'SHA256SUMS.txt').read_text().splitlines()), 4)
        notes = (self.output / 'release-notes.md').read_text(encoding='utf-8')
        self.assertIn('## Có gì mới', notes)
        self.assertIn(self.meta['notes'].strip(), notes)
        self.assertNotIn('bản thử nghiệm', notes.casefold())
        self.assertIn('## Bản tải', notes)
        self.assertIn('### Nếu macOS chặn ứng dụng', notes)
        for name in self.expected:
            self.assertIn(f'`{name}`', notes)
        self.assertIn('SHA256SUMS.txt', notes)
        self.assertIn('flutter-network-viewer.exe', notes)
        self.assertIn('xattr -dr com.apple.quarantine "/Applications/Flutter Network Viewer.app"', notes)
        self.assertIn('open "/Applications/Flutter Network Viewer.app"', notes)
        self.assertIn('GitHub Releases chính thức', notes)
        self.assertFalse(any('publish' in call for call in calls))

    def test_release_notes_follow_version_and_changelog_metadata(self):
        meta = dict(self.meta, version='2.3.4', notes='- Sửa lỗi kết nối mới.\n')
        notes = draft_release.release_notes(meta)
        self.assertIn('- Sửa lỗi kết nối mới.', notes)
        self.assertNotIn(self.meta['notes'].strip(), notes)
        for platform in release.PLATFORMS:
            self.assertIn(f'flutter-network-viewer-rust-2.3.4-{platform}.zip', notes)
        self.assertIn('flutter-network-viewer-rust-2.3.4-windows-x64-setup.exe', notes)

    def test_rerun_updates_existing_draft_instead_of_creating_duplicate(self):
        calls = self.run_release([{'tag_name': self.meta['tag'], 'draft': True}])
        self.assertFalse(any(c[:2] == ('release', 'create') for c in calls))
        edit = next(c for c in calls if c[:2] == ('release', 'edit'))
        self.assertIn('--draft=true', edit)
        self.assertIn('--target', edit)
        self.assertEqual(edit[edit.index('--title') + 1], f'Flutter Network Viewer v{self.meta["version"]}')
        self.assertIn('--clobber', calls[-1])

    def test_published_version_is_never_modified(self):
        with self.assertRaisesRegex(ValueError, 'published'):
            self.run_release([{'tag_name': self.meta['tag'], 'draft': False}])

    def test_missing_platform_or_bad_checksum_stops_before_github(self):
        archive = next(self.artifacts.rglob('*.zip'))
        archive.write_bytes(b'tampered')
        with patch.object(draft_release, 'gh') as gh:
            with self.assertRaisesRegex(ValueError, 'Checksum'):
                draft_release.create_draft(self.artifacts, self.output, 'owner/repo', 'a' * 40)
            gh.assert_not_called()
        archive.unlink()
        with patch.object(draft_release, 'gh') as gh:
            with self.assertRaises(FileNotFoundError):
                draft_release.create_draft(self.artifacts, self.output, 'owner/repo', 'a' * 40)
            gh.assert_not_called()

    def test_manifest_must_cover_installer_too(self):
        folder = next(self.artifacts.glob('*windows-x64'))
        release.checksums(list(folder.glob('*.zip')), folder / 'SHA256SUMS.txt')
        with patch.object(draft_release, 'gh') as gh:
            with self.assertRaisesRegex(ValueError, 'cover'):
                draft_release.create_draft(self.artifacts, self.output, 'owner/repo', 'a' * 40)
            gh.assert_not_called()

    def test_github_api_failure_is_not_treated_as_missing_release(self):
        with patch.object(draft_release, 'gh', side_effect=subprocess.CalledProcessError(1, 'gh')) as gh:
            with self.assertRaises(subprocess.CalledProcessError):
                draft_release.create_draft(self.artifacts, self.output, 'owner/repo', 'a' * 40)
            self.assertEqual(gh.call_count, 1)

    def test_finds_existing_draft_on_later_api_page(self):
        pages = [[], [{'tag_name': self.meta['tag'], 'draft': True}]]
        with patch.object(draft_release, 'gh', side_effect=[json.dumps(pages), '', '']) as gh:
            draft_release.create_draft(self.artifacts, self.output, 'owner/repo', 'a' * 40)
            self.assertIn('--paginate', gh.call_args_list[0].args)
            self.assertEqual(gh.call_args_list[1].args[:2], ('release', 'edit'))

    def test_workflow_waits_for_all_builds_and_limits_write_permission(self):
        workflow = (release.ROOT / '.github/workflows/checks.yml').read_text()
        job = workflow.split('  draft-release:', 1)[1]
        self.assertIn('needs: desktop', job)
        self.assertIn("github.ref == 'refs/heads/main'", job)
        self.assertIn("github.event_name != 'pull_request'", job)
        self.assertNotIn('always()', job)
        self.assertIn('contents: write', job)
        self.assertIn('actions/download-artifact@v4', job)
        self.assertIn('pattern: rust-preview-*', job)
        self.assertIn('tools/draft_release.py', job)
        self.assertIn('GH_TOKEN: ${{ github.token }}', job)
        self.assertIn('cancel-in-progress: false', job)


if __name__ == '__main__':
    unittest.main()
