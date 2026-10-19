import os
from pathlib import Path
import subprocess
import tempfile
import unittest

from prepare_github_release import app_version, release_info, version_increased

SCRIPT = Path(__file__).with_name('prepare_github_release.py').resolve()


class ReleaseMetadataTest(unittest.TestCase):
    def test_flutter_build_number_is_not_part_of_tag(self):
        self.assertEqual(app_version('name: viewer\nversion: 0.1.2+3\n'), '0.1.2')

    def test_only_public_version_increase_releases(self):
        self.assertTrue(version_increased('0.1.10', '0.1.9'))
        self.assertTrue(version_increased('0.2.0', '0.1.9'))
        self.assertFalse(version_increased('0.1.2', '0.1.2'))
        with self.assertRaises(ValueError):
            version_increased('0.1.1', '0.1.2')

    def test_title_and_markdown_are_preserved(self):
        source = '---\ntag: v0.1.2\ntitle: Viewer: bản mới\n---\n\n## Changes\n\n- Fix #42\n'
        self.assertEqual(
            release_info(source, '0.1.2'),
            ('v0.1.2', 'Viewer: bản mới', '## Changes\n\n- Fix #42\n'),
        )

    def test_invalid_metadata_is_rejected(self):
        invalid = [
            'tag: v0.1.2\ntitle: Viewer\nDescription',
            '---\ntag: v0.1.2\ntitle: Viewer\nDescription',
            '---\ntag: v0.1.1\ntitle: Viewer\n---\nDescription',
            '---\ntag: v0.1.2\n---\nDescription',
            '---\ntag: v0.1.2\ntitle:\n---\nDescription',
            '---\ntag: v0.1.2\ntitle: Viewer\n---\n',
            '---\ntag: v0.1.2\ntag: v0.1.2\ntitle: Viewer\n---\nDescription',
        ]
        for source in invalid:
            with self.subTest(source=source), self.assertRaises(ValueError):
                release_info(source, '0.1.2')


class ReleasePreparationTest(unittest.TestCase):
    def setUp(self):
        self.folder = tempfile.TemporaryDirectory(prefix='fnv-release-test-')
        self.addCleanup(self.folder.cleanup)
        self.root = Path(self.folder.name)
        self.git('init', '-q')
        self.write_version('0.1.1+2')
        self.git('add', 'pubspec.yaml')
        self.git('-c', 'user.name=Release Test', '-c', 'user.email=test@example.invalid',
                 'commit', '-qm', 'previous version')
        self.before = self.git('rev-parse', 'HEAD')
        self.write_version('0.1.2+3')
        (self.root / 'github_release_info.md').write_text(
            '---\ntag: v0.1.2\ntitle: Viewer v0.1.2\n---\n\n## Changes\n\n- Fix #42\n'
        )

    def git(self, *arguments):
        return subprocess.check_output(['git', *arguments], cwd=self.root, text=True).strip()

    def write_version(self, version):
        (self.root / 'pubspec.yaml').write_text(f'version: {version}\n')

    def prepare(self, before=None):
        env = dict(os.environ, GITHUB_SHA=self.before,
                   BEFORE_COMMIT=self.before if before is None else before,
                   GITHUB_OUTPUT=str(self.root / 'output'),
                   GITHUB_STEP_SUMMARY=str(self.root / 'summary'))
        return subprocess.run(
            ['python3', str(SCRIPT)], cwd=self.root, env=env,
            capture_output=True, text=True, check=False,
        )

    def test_bump_creates_outputs_and_exact_description(self):
        result = self.prepare()
        self.assertEqual(result.returncode, 0, result.stderr)
        output = (self.root / 'output').read_text()
        self.assertIn('should_release=true\n', output)
        self.assertIn('tag=v0.1.2\ntitle=Viewer v0.1.2\n', output)
        self.assertEqual((self.root / 'build/release-notes.md').read_text(),
                         '## Changes\n\n- Fix #42\n')

    def test_build_number_only_skips_release(self):
        self.write_version('0.1.1+3')
        result = self.prepare()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual((self.root / 'output').read_text(), 'should_release=false\n')
        self.assertFalse((self.root / 'build').exists())

    def test_version_bump_requires_increased_macos_build_number(self):
        for version in ('0.1.2+2', '0.1.2+1', '0.1.2'):
            with self.subTest(version=version):
                self.write_version(version)
                result = self.prepare()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('build number', result.stderr)
                self.assertFalse((self.root / 'build').exists())

    def test_manual_run_can_prepare_without_previous_commit(self):
        result = self.prepare(before='')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn('should_release=true', (self.root / 'output').read_text())

    def test_existing_tag_on_another_commit_stops_release(self):
        self.git('tag', 'v0.1.2')
        self.git('add', 'pubspec.yaml')
        self.git('-c', 'user.name=Release Test', '-c', 'user.email=test@example.invalid',
                 'commit', '-qm', 'new version')
        self.before = self.git('rev-parse', 'HEAD')
        result = self.prepare(before='')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('already points to another commit', result.stderr)
        self.assertFalse((self.root / 'build').exists())


if __name__ == '__main__':
    unittest.main()
