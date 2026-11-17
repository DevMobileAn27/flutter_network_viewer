"""Test the actual vendored Sparkle verifier on inert bytes; never an installer."""
import base64
import hashlib
import pathlib
import os
import subprocess
import tempfile
import unittest
from verify_updater_vendor import verify
ROOT=pathlib.Path(__file__).resolve().parents[1]
ARTIFACTS=pathlib.Path(os.environ.get('FNV_INTEGRITY_ARTIFACTS',ROOT/'docs/artifacts/f20'))
ARTIFACTS.mkdir(parents=True,exist_ok=True)
class Integrity(unittest.TestCase):
    def test_vendor_checksum_and_tamper_rejection(self):
        verify(ROOT/'vendor')
        with tempfile.TemporaryDirectory(dir=ARTIFACTS) as folder:
            root=pathlib.Path(folder)
            (root/'bytes').write_bytes(b'original')
            (root/'SHA256SUMS').write_text(hashlib.sha256(b'original').hexdigest()+'  bytes\n')
            verify(root)
            (root/'bytes').write_bytes(b'tampered')
            with self.assertRaises(ValueError): verify(root)
    def test_native_signature_accepts_valid_rejects_modified_unsigned_and_wrong_key(self):
        with tempfile.TemporaryDirectory(dir=ARTIFACTS) as folder:
            root=pathlib.Path(folder)
            source=root/'sign.swift'
            source.write_text('''import CryptoKit
import Foundation
let key=try Curve25519.Signing.PrivateKey(rawRepresentation:Data(repeating:7,count:32))
let root=CommandLine.arguments[1]
try key.rawRepresentation.base64EncodedString().write(toFile:root+"/key",atomically:true,encoding:.utf8)
let bytes=Data("inert fixture — không phải installer 🖼️".utf8)
try bytes.write(to:URL(fileURLWithPath:root+"/archive"))
print(try key.signature(for:bytes).base64EncodedString())
''',encoding='utf-8')
            signed=subprocess.check_output(['swift','-module-cache-path',str(ARTIFACTS/'swift-cache'),str(source),str(root)],text=True).strip()
            def run(sig):
                return subprocess.run([str(ROOT/'vendor/sparkle/bin/sign_update'),'--verify','--ed-key-file',str(root/'key'),str(root/'archive'),sig],capture_output=True)
            self.assertEqual(run(signed).returncode,0)
            self.assertNotEqual(run('').returncode,0)
            self.assertNotEqual(run(base64.b64encode(bytes(64)).decode()).returncode,0)
            (root/'key').write_text(base64.b64encode(bytes([8])*32).decode())
            self.assertNotEqual(run(signed).returncode,0)
            (root/'key').write_text(base64.b64encode(bytes([7])*32).decode())
            (root/'archive').write_bytes(b'tampered archive')
            self.assertNotEqual(run(signed).returncode,0)
if __name__=='__main__': unittest.main()
