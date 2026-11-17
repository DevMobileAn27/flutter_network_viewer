#!/usr/bin/env python3
"""Real Dart VM integration; all fixture/output files stay inside Rust."""
from pathlib import Path
import os
import re
import subprocess
import sys
import threading
ROOT=Path(__file__).resolve().parents[1]
output=Path(sys.argv[1]).resolve(); output.mkdir(parents=True,exist_ok=False)
dart=os.environ.get('DART_BIN','/Users/quocan/develop/flutter/bin/cache/dart-sdk/bin/dart')
process=subprocess.Popen([dart,'--enable-vm-service=0','--disable-service-auth-codes',str(ROOT/'tests/fixtures/f21/live_vm.dart')],cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
lines=[]
uri=[]
ready=threading.Event()
def read():
    for line in process.stdout:
        lines.append(line)
        match=re.search(r'http://127\.0\.0\.1:\d+/\S*',line)
        if match and not uri: uri.append(match[0]); ready.set()
thread=threading.Thread(target=read,daemon=True); thread.start()
try:
    if not ready.wait(20): raise RuntimeError('SDK did not advertise service')
    subprocess.run(['cargo','run','--locked','--offline','--example','f21_live_vm','--',uri[0],str(output)],cwd=ROOT,check=True,timeout=90)
finally:
    process.terminate()
    try: process.wait(timeout=5)
    except subprocess.TimeoutExpired: process.kill(); process.wait()
    thread.join(timeout=5)
    (output/'dart.log').write_text(''.join(lines),encoding='utf-8')
