#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
umask 077
swift tool/generate_update_keys.swift
python3 - <<'PY'
from pathlib import Path
import re

public_key = Path('updates/macos_ed25519.pub').read_text().strip()
plist = Path('macos/Runner/Info.plist')
source = plist.read_text()
source, count = re.subn(r'(<key>SUPublicEDKey</key>\s*<string>)[^<]*(</string>)',
                        lambda match: match[1] + public_key + match[2], source)
if count != 1:
    raise SystemExit('Expected exactly one SUPublicEDKey in macos/Runner/Info.plist')
plist.write_text(source)
PY
if [ ! -f .local/update-signing/windows_dsa.pem ]; then
  openssl dsaparam -out .local/update-signing/dsa-parameters.pem 4096
  openssl gendsa -out .local/update-signing/windows_dsa.pem .local/update-signing/dsa-parameters.pem
  rm .local/update-signing/dsa-parameters.pem
fi
openssl dsa -in .local/update-signing/windows_dsa.pem -pubout -out updates/windows_dsa_pub.pem
echo 'Keys ready. Back up .local/update-signing privately; never commit private keys.'
