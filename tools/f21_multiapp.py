from pathlib import Path
import subprocess
import sys
ROOT=Path(__file__).resolve().parents[1]
output=Path(sys.argv[1]).resolve(); output.mkdir(parents=True,exist_ok=False)
cargo=['cargo','run','--locked','--offline','--example']
processes=[]
try:
    uris=[]
    for _ in range(2):
        process=subprocess.Popen(cargo+['f21_fake_vm'],cwd=ROOT,stdout=subprocess.PIPE,text=True)
        processes.append(process); uri=process.stdout.readline().strip()
        if not uri.startswith('ws://127.0.0.1:'): raise RuntimeError('No fresh VM')
        uris.append(uri)
    subprocess.run(cargo+['f21_multiapp','--',*uris,str(output/'result.json')],cwd=ROOT,check=True,timeout=65)
finally:
    for process in processes:
        process.terminate()
        try: process.wait(timeout=5)
        except subprocess.TimeoutExpired: process.kill(); process.wait()
