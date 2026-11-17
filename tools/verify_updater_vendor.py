#!/usr/bin/env python3
"""Verify the pinned updater runtime binaries/headers before builds and bundling."""
import hashlib
from pathlib import Path

def verify(root):
    root=Path(root).resolve()
    for line in (root/'SHA256SUMS').read_text(encoding='utf-8').splitlines():
        expected,name=line.split('  ',1)
        path=(root/name).resolve()
        if not path.is_relative_to(root):
            raise ValueError('Checksum path escapes vendor root')
        if hashlib.sha256(path.read_bytes()).hexdigest()!=expected:
            raise ValueError(f'Updater vendor checksum mismatch: {name}')

if __name__=='__main__':
    verify(Path(__file__).resolve().parents[1]/'vendor')
    print('PASS updater vendor SHA-256 checksums')
