#!/usr/bin/env python3
"""Compile current source and exercise F17 against a fresh loopback VM."""
import pathlib
import subprocess
import sys
root = pathlib.Path(__file__).resolve().parents[1]
output = pathlib.Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else root / "docs/artifacts/f17"
cargo = ["cargo", "run", "--locked", "--offline", "--example"]
fixture = subprocess.Popen(cargo + ["f16_fake_vm"], cwd=root, stdout=subprocess.PIPE, text=True)
try:
    uri = fixture.stdout.readline().strip()
    if not uri.startswith("ws://127.0.0.1:"):
        raise RuntimeError(f"fixture did not advertise a loopback VM: {uri!r}")
    subprocess.run(cargo + ["f17_native_smoke", "--", str(output), uri], cwd=root, check=True, timeout=90)
finally:
    fixture.terminate()
    try:
        fixture.wait(timeout=5)
    except subprocess.TimeoutExpired:
        fixture.kill()
        fixture.wait()
