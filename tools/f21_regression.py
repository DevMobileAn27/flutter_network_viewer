#!/usr/bin/env python3
"""Sequential native regression on current source, no stale artifacts."""
from pathlib import Path
import os
import subprocess
import sys
ROOT=Path(__file__).resolve().parents[1]
output=Path(sys.argv[1]).resolve(); output.mkdir(parents=True,exist_ok=False)
for feature in range(13,21):
    env=dict(os.environ,FNV_DIAGNOSTICS_DIR=str(output/f'f{feature}-diagnostics'))
    with (output/f'f{feature}.log').open('wb') as log:
        result=subprocess.run([sys.executable,'-B',str(ROOT/f'tools/f{feature}_native_smoke.py'),str(output/f'f{feature}')],cwd=ROOT,env=env,stdout=log,stderr=subprocess.STDOUT,timeout=180)
    if result.returncode: raise SystemExit(f'F{feature} failed: {output}/f{feature}.log')
    print(f'PASS F{feature} native current source',flush=True)
