#!/usr/bin/env python3
"""Validate release metadata and prepare GitHub Actions outputs (stdlib only)."""

import os
from pathlib import Path
import re
import subprocess


def app_version(source):
    match = re.search(r'^version:\s*(\d+\.\d+\.\d+)(?:\+\d+)?\s*$', source, re.M)
    if not match:
        raise ValueError('Expected version: X.Y.Z+BUILD in pubspec.yaml')
    return match.group(1)


def version_increased(current, previous):
    current_parts = tuple(map(int, current.split('.')))
    previous_parts = tuple(map(int, previous.split('.')))
    if current_parts < previous_parts:
        raise ValueError(f'App version decreased from {previous} to {current}')
    return current_parts > previous_parts


def app_build(source):
    match = re.search(r'^version:\s*\d+\.\d+\.\d+\+(\d+)\s*$', source, re.M)
    if not match:
        raise ValueError('A numeric Flutter build number is required for updates')
    return int(match.group(1))


def release_info(source, version):
    lines = source.splitlines()
    if not lines or lines[0] != '---':
        raise ValueError('github_release_info.md must start with ---')
    try:
        end = lines.index('---', 1)
    except ValueError as error:
        raise ValueError('Missing closing --- in release metadata') from error
    fields = {}
    for line in lines[1:end]:
        key, separator, value = line.partition(':')
        key, value = key.strip(), value.strip()
        if not separator or key not in ('tag', 'title') or key in fields:
            raise ValueError('Release metadata must contain exactly tag: and title:')
        if not value:
            raise ValueError(f'Release {key} cannot be empty')
        fields[key] = value
    if set(fields) != {'tag', 'title'}:
        raise ValueError('Both tag: and title: are required')
    if fields['tag'] != f'v{version}':
        raise ValueError(f'Release tag {fields["tag"]} must match app version v{version}')
    body = '\n'.join(lines[end + 1:]).strip()
    if not body:
        raise ValueError('Release description cannot be empty')
    return fields['tag'], fields['title'], body + '\n'


def verify_tag_commit(tag, commit):
    result = subprocess.run(
        ['git', 'rev-parse', '--verify', '--quiet', f'refs/tags/{tag}^{{commit}}'],
        capture_output=True, text=True, check=False,
    )
    if result.returncode not in (0, 1):
        raise ValueError(f'Cannot check existing tag {tag}: {result.stderr.strip()}')
    if result.returncode == 0 and result.stdout.strip() != commit:
        raise ValueError(
            f'Tag {tag} already points to another commit. '
            'Use a new version; this workflow will not move existing tags.'
        )


def main():
    source = Path('pubspec.yaml').read_text()
    version = app_version(source)
    build = app_build(source)
    before = os.environ.get('BEFORE_COMMIT', '')
    if before and set(before) != {'0'}:
        previous_source = subprocess.check_output(
            ['git', 'show', f'{before}:pubspec.yaml'], text=True,
        )
        previous = app_version(previous_source)
        if not version_increased(version, previous):
            with open(os.environ['GITHUB_OUTPUT'], 'a') as output:
                output.write('should_release=false\n')
            print(f'Version remains {version}; no release build needed.')
            return
        if build <= app_build(previous_source):
            raise ValueError('Increase the Flutter build number so installed macOS apps detect the update')

    tag, title, body = release_info(Path('github_release_info.md').read_text(), version)
    commit = os.environ['GITHUB_SHA']
    verify_tag_commit(tag, commit)
    body_path = Path('build/release-notes.md')
    body_path.parent.mkdir(parents=True, exist_ok=True)
    body_path.write_text(body)
    with open(os.environ['GITHUB_OUTPUT'], 'a') as output:
        output.write(f'should_release=true\nversion={version}\ntag={tag}\ntitle={title}\n')
    with open(os.environ['GITHUB_STEP_SUMMARY'], 'a') as summary:
        summary.write(f'## {tag}\n\nCommit: `{commit}`\n\n')
        summary.write('Build all three desktop apps, then create a draft from github_release_info.md.\n')


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        raise SystemExit(f'Release preparation failed: {error}') from error
