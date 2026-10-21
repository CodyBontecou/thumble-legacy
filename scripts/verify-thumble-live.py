#!/usr/bin/env python3
"""Run real Lua live snapshot tests and validate their transport independently."""
import argparse
import json
from pathlib import Path
import shutil
import struct
import subprocess
import zlib

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--lua", default=shutil.which("lua5.1") or shutil.which("lua"))
parser.add_argument("--write-fixture", action="store_true")
args = parser.parse_args()
if not args.lua:
    parser.error("Lua 5.1 is required; pass --lua /path/to/lua")
root = Path(__file__).resolve().parent.parent
result = subprocess.run([args.lua, str(root / "Tests/ThumbleLinkLiveTests.lua"), str(root)],
                        check=True, capture_output=True, text=True)
lines = result.stdout.splitlines()
snapshot = json.loads(lines[0])
assert snapshot["schema"] == "thumble.wow-live"
assert len(snapshot["actions"]) == 6
assert len(snapshot["icons"]) == 2
payloads = []
for index, line in enumerate(lines[1:]):
    frame = bytes.fromhex(line)
    magic, version, tick, digest, page, count, length, total, checksum = struct.unpack("<4sBIIHHHHI", frame[:25])
    assert magic == b"THWL" and version == 1 and tick == 123 and len(frame) == 512
    assert page == index and count == len(lines) - 1
    payload = frame[25:25 + length]
    assert zlib.adler32(frame[:21] + payload) == checksum
    assert not any(frame[25 + length:])
    payloads.append(payload)
message = b"".join(payloads)
assert len(message) == total and zlib.adler32(message) == digest
assert json.loads(message) == snapshot
fixture = root / "Host/fixtures/wow-live/v1.json"
value = {"snapshot": snapshot, "frames": lines[1:]}
if args.write_fixture:
    fixture.parent.mkdir(parents=True, exist_ok=True)
    fixture.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")
else:
    assert json.loads(fixture.read_text()) == value, "live Lua/Rust fixture drift"
subprocess.run([args.lua, str(root / "Tests/ThumbleLinkOnboardingTests.lua"), str(root)], check=True)
print("Live character/action/icon snapshot and paged transport checks passed.")
