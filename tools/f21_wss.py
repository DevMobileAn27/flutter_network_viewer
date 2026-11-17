from pathlib import Path
import socket
import os
import shutil
import subprocess
import sys
import time
ROOT=Path(__file__).resolve().parents[1]
output=Path(sys.argv[1]).resolve(); output.mkdir(parents=True,exist_ok=False)
openssl=os.environ.get('OPENSSL_BIN') or ('/opt/homebrew/bin/openssl' if Path('/opt/homebrew/bin/openssl').exists() else shutil.which('openssl'))
subprocess.run([openssl,'req','-x509','-newkey','rsa:2048','-nodes','-keyout',str(output/'test-key.pem'),'-out',str(output/'test-cert.pem'),'-days','1','-subj','/CN=localhost'],check=True,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
with socket.socket() as sock:
    sock.bind(('127.0.0.1',0)); port=sock.getsockname()[1]
with (output/'tls-server.log').open('wb') as log:
    process=subprocess.Popen([openssl,'s_server','-accept',f'127.0.0.1:{port}','-cert',str(output/'test-cert.pem'),'-key',str(output/'test-key.pem'),'-www'],stdout=log,stderr=log)
    try:
        for attempt in range(100):
            try:
                with socket.create_connection(('127.0.0.1',port),timeout=.2): break
            except OSError: time.sleep(.02)
        else: raise RuntimeError('TLS server not listening')
        subprocess.run(['cargo','run','--locked','--offline','--example','f21_wss','--',f'wss://localhost:{port}/auth=/ws'],cwd=ROOT,check=True,timeout=60)
        # Ensure the test reached the TLS server, not merely a failed TCP dial.
        time.sleep(.1)
    finally:
        process.terminate(); process.wait(timeout=5)
if 'ssl routines' not in (output/'tls-server.log').read_text(encoding='utf-8').lower():
    raise AssertionError('No TLS failure observed at server')
