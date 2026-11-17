#!/usr/bin/env python3
"""Build current Rust source and run real Sparkle in an isolated test app bundle.
Only loopback appcast requests are allowed; any archive request fails the test.
No installer is downloaded, launched or accepted.
"""
import functools
import http.server
import os
import pathlib
import plistlib
import shutil
import subprocess
import sys
import threading
root = pathlib.Path(__file__).resolve().parents[1]
output = pathlib.Path(sys.argv[1]).resolve() if len(sys.argv)>1 else root / 'docs/artifacts/f20/native'
output.mkdir(parents=True, exist_ok=True)
(output/'feed.xml').write_text('malformed feed', encoding='utf-8')
requests=[]
class Handler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        requests.append(self.path)
        if self.path != '/feed.xml':
            self.send_error(403)
            return
        # Preserve Checking across at least one production frame.
        import time
        time.sleep(.25)
        super().do_GET()
server=http.server.ThreadingHTTPServer(('127.0.0.1',0), functools.partial(Handler,directory=str(output)))
thread=threading.Thread(target=server.serve_forever,daemon=True)
thread.start()
try:
    subprocess.run(['cargo','build','--locked','--offline','--example','f20_native_smoke'],cwd=root,check=True)
    meta=__import__('json').loads(subprocess.check_output(['cargo','metadata','--locked','--offline','--format-version','1','--no-deps'],cwd=root))
    binary=pathlib.Path(meta['target_directory'])/'debug/examples/f20_native_smoke'
    bundle=output/'F20.app/Contents'
    (bundle/'MacOS').mkdir(parents=True,exist_ok=True)
    (bundle/'Frameworks').mkdir(exist_ok=True)
    shutil.copy2(binary,bundle/'MacOS/f20_native_smoke')
    shutil.copytree(root/'vendor/sparkle/Sparkle.framework',bundle/'Frameworks/Sparkle.framework',symlinks=True,dirs_exist_ok=True)
    info={'CFBundleExecutable':'f20_native_smoke','CFBundleIdentifier':'dev.quocan.fnv.rust.updater.test','CFBundleName':'F20 Updater Native Acceptance','CFBundlePackageType':'APPL','CFBundleVersion':'1','CFBundleShortVersionString':'0.1.0','SUPublicEDKey':'TNPdGQZDTuvlcdgdReTFjir/+sfSLq5+o92dT4IolFY=','SUEnableInstallerLauncherService':True,'SUEnableAutomaticChecks':False,'SUAutomaticallyUpdate':False,'SUAllowsAutomaticUpdates':False}
    with (bundle/'Info.plist').open('wb') as f: plistlib.dump(info,f)
    env=os.environ.copy()
    # Sparkle may persist opt-in settings. Force its defaults domain into a test
    # home inside Rust so all test writes stay in this project.
    env['HOME']=str(output/'home')
    env['CFFIXED_USER_HOME']=str(output/'home')
    (output/'home').mkdir(exist_ok=True)
    subprocess.run([str(bundle/'MacOS/f20_native_smoke'),str(output),f'http://127.0.0.1:{server.server_port}/feed.xml'],cwd=root,env=env,check=True,timeout=70)
    high=(root/'tests/fixtures/f20_current.xml').read_text(encoding='utf-8').replace('<sparkle:version>1</sparkle:version>','<sparkle:version>999999</sparkle:version>').replace('<sparkle:shortVersionString>0.1.0</sparkle:shortVersionString>','<sparkle:shortVersionString>99.0.0</sparkle:shortVersionString>')
    (output/'feed.xml').write_text(high,encoding='utf-8')
    subprocess.run([str(bundle/'MacOS/f20_native_smoke'),str(output),f'http://127.0.0.1:{server.server_port}/feed.xml','available'],cwd=root,env=env,check=True,timeout=70)
    assert len(requests)>=3,requests
    assert all(p=='/feed.xml' for p in requests), requests
    (output/'http-requests.json').write_text(__import__('json').dumps(requests))
finally:
    server.shutdown()
    server.server_close()
    thread.join()
