#!/usr/bin/env python3
"""Capture public artifact/source metadata without reading user authority or credentials.

The caller must explicitly select an existing receiver bundle. Never starts a
receiver, changes ownership, requests permissions, or injects input.
"""
from __future__ import annotations

import argparse
import datetime
import hashlib
import json
import plistlib
import shutil
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def digest(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def bundle_manifest(bundle: Path) -> dict:
    # Hash only explicit package contents, never adjoining state/backups.
    entries = []
    for path in sorted(bundle.rglob("*")):
        if path.is_symlink():
            raise ValueError("Bundle contains symlinks; review manifest scope explicitly")
        if path.is_file():
            entries.append({"path": path.relative_to(bundle).as_posix(),
                            "bytes": path.stat().st_size, "sha256": digest(path)})
    canonical = json.dumps(entries, sort_keys=True, separators=(",", ":")).encode()
    info = plistlib.loads((bundle / "Contents/Info.plist").read_bytes())
    return {"identity": info["CFBundleIdentifier"], "fileCount": len(entries),
            "installedBytes": sum(e["bytes"] for e in entries),
            "manifestEncoding": "thumble-file-manifest-v1: UTF-8 sorted compact JSON array of path/bytes/sha256",
            "manifestSHA256": hashlib.sha256(canonical).hexdigest(),
            "executableSHA256": digest(bundle / "Contents/MacOS" / info["CFBundleExecutable"]),
            "files": entries}


def command(args: list[str]) -> str:
    return subprocess.check_output(args, cwd=ROOT, text=True, stderr=subprocess.STDOUT, timeout=30).strip()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receiver-bundle", type=Path, required=True)
    parser.add_argument("--baseline", required=True, help="Frozen baseline Git commit")
    args = parser.parse_args()
    # Resolve to a commit; captures are not automatically acceptance records.
    commit = command(["git", "rev-parse", args.baseline + "^{commit}"])
    report = {
        "version": 1, "capturedAtUTC": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "source": {"baselineCommit": commit, "baselineTree": command(["git", "rev-parse", commit + "^{tree}"])},
        "host": {"os": command(["sw_vers"]), "architecture": command(["uname", "-m"])},
        "toolchains": {"xcode": command(["xcodebuild", "-version"]),
                       "swift": command(["swift", "--version"]), "node": command(["node", "--version"]),
                       "bun": command(["bun", "--version"]) if shutil.which("bun") else "not installed; M2 toolchain pin pending", "rustPin": "1.88.0"},
        "receiver": bundle_manifest(args.receiver_bundle.resolve()),
        "fixtureHashes": {p.relative_to(ROOT).as_posix(): digest(p)
                          for p in sorted((ROOT / "Host/fixtures").rglob("*")) if p.is_file()},
        "boundary": "Existing development lab artifact; full build provenance to frozen source is unproven. Not release/gameplay/latency acceptance.",
    }
    target = ROOT / "docs/rewrite-evidence/m0/baseline.json"
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(report, indent=2) + "\n")
    print("Captured source, fixture and selected bundle hashes; no input or state mutation")


if __name__ == "__main__":
    main()
