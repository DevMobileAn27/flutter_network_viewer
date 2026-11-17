#!/usr/bin/env python3
"""Run the real desktop host against a fresh local VM fixture; no Dart SDK needed."""
import pathlib
import subprocess
import sys

root = pathlib.Path(__file__).resolve().parents[1]
output = pathlib.Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else root / "docs/artifacts/f14"
cargo = ["cargo", "run", "--locked", "--offline", "--example"]
fixture = subprocess.Popen(cargo + ["f14_fake_vm"], cwd=root, stdout=subprocess.PIPE, text=True)
try:
    uri = fixture.stdout.readline().strip()
    if not uri.startswith("ws://127.0.0.1:"):
        raise RuntimeError(f"fixture did not advertise a loopback VM: {uri!r}")
    subprocess.run(cargo + ["f14_native_smoke", "--", uri, str(output)], cwd=root, check=True, timeout=60)
finally:
    fixture.terminate()
    try:
        fixture.wait(timeout=5)
    except subprocess.TimeoutExpired:
        fixture.kill()
        fixture.wait()
