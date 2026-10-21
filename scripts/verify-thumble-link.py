#!/usr/bin/env python3
"""Exercise the real Lua exporter with mocked WoW APIs and check the Swift fixture."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--lua", default=shutil.which("lua5.1") or shutil.which("lua"))
parser.add_argument("--write-fixture", action="store_true")
args = parser.parse_args()
if not args.lua:
    parser.error("Lua is required; pass --lua /path/to/lua (tested with Lua 5.1.5)")
root = Path(__file__).resolve().parent.parent
prefix = "THUMBLELINK1:"
for flavor, filename, interface in [("forever", "warrior.txt", 16001),
                                     ("classic-era", "warrior-classic-era.txt", 11509)]:
    result = subprocess.run([args.lua, str(root / "Tests/ThumbleLinkExporterTests.lua"), str(root), flavor],
                            check=True, text=True, capture_output=True)
    assert result.stdout.startswith(prefix)
    snapshot = json.loads(result.stdout[len(prefix):])
    assert snapshot["schema"] == "thumble.wow-link" and snapshot["version"] == 1
    assert snapshot["client"]["flavor"] == flavor and snapshot["client"]["interface"] == interface
    assert len(snapshot["actions"]) == 5
    assert all(isinstance(action["keys"], list) for action in snapshot["actions"])
    fixture = root / "Tests/Fixtures/ThumbleLink" / filename
    if args.write_fixture:
        fixture.write_text(result.stdout)
    else:
        assert json.loads(fixture.read_text()[len(prefix):]) == snapshot, "Lua output differs from the Swift fixture"
print("Thumble Link Lua checks passed for Forever and Classic Era; snapshots match the Swift fixtures.")
