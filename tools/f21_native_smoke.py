#!/usr/bin/env python3
"""Current production host: one fake VM -> filter/inspect/pause/MCP/export/clear."""
import os
from pathlib import Path
import subprocess
import sys
ROOT=Path(__file__).resolve().parents[1]
output=Path(sys.argv[1]).resolve()
output.mkdir(parents=True,exist_ok=False)
env=dict(os.environ,FNV_DIAGNOSTICS_DIR=str(output/'diagnostics'))
cargo=['cargo','run','--locked','--offline','--example']
fixture=subprocess.Popen(cargo+['f13_fake_vm'],cwd=ROOT,env=env,stdout=subprocess.PIPE,text=True)
try:
    uri=fixture.stdout.readline().strip()
    if not uri.startswith('ws://127.0.0.1:'): raise RuntimeError('No fresh VM')
    subprocess.run(cargo+['f21_native_smoke','--',uri,str(output)],cwd=ROOT,env=env,check=True,timeout=90)
finally:
    fixture.terminate()
    try: fixture.wait(timeout=5)
    except subprocess.TimeoutExpired: fixture.kill(); fixture.wait()
