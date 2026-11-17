"""macOS only: exercise the actual release signing helper with inert bytes/test keys."""
import base64
from pathlib import Path
import subprocess
import tempfile
import unittest
ROOT=Path(__file__).resolve().parents[1]
class Signer(unittest.TestCase):
    def test_real_helper_signs_verifiable_bytes_and_rejects_wrong_anchor(self):
        with tempfile.TemporaryDirectory(dir=ROOT/'docs/artifacts/f21') as folder:
            folder=Path(folder); archive=folder/'inert'; archive.write_bytes('Rust test — không phải installer'.encode('utf-8'))
            private=base64.b64encode(bytes([7])*32)
            source=folder/'public.swift'
            source.write_text('import CryptoKit\nimport Foundation\nprint(try Curve25519.Signing.PrivateKey(rawRepresentation:Data(repeating:7,count:32)).publicKey.rawRepresentation.base64EncodedString())',encoding='utf-8')
            swift=['swift','-module-cache-path',str(ROOT/'docs/artifacts/f21/signer-cache')]
            public=subprocess.check_output(swift+[str(source)],text=True).strip()
            helper=swift+[str(ROOT/'tools/sign_macos_update.swift'),str(archive)]
            signed=subprocess.check_output(helper+[public],input=private).decode('ascii').strip()
            key=folder/'key'; key.write_bytes(private)
            verify=[str(ROOT/'vendor/sparkle/bin/sign_update'),'--verify','--ed-key-file',str(key),str(archive)]
            subprocess.run(verify+[signed],check=True,stdout=subprocess.DEVNULL)
            wrong=subprocess.run(helper+[base64.b64encode(bytes(32)).decode('ascii')],input=private,capture_output=True)
            self.assertNotEqual(wrong.returncode,0)
            self.assertNotIn(signed.encode('ascii'),wrong.stdout)
            archive.write_bytes(b'tampered')
            self.assertNotEqual(subprocess.run(verify+[signed],capture_output=True).returncode,0)
if __name__=='__main__': unittest.main()
