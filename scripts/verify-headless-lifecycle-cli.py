#!/usr/bin/env python3
"""Exercise native CLI lifecycle against a real isolated recording-only receiver.

Requires built CLI, host, and CLI bridge binaries. Uses a private temporary HOME
under this workspace (short, trusted socket ancestry), disables input and Bonjour,
and never reads/writes the signed-in account's profiles or trust state.
"""
import argparse
import fcntl
import hashlib
import json
from pathlib import Path
import plistlib
import shutil
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--cli", type=Path, required=True)
    parser.add_argument("--host", type=Path, required=True)
    parser.add_argument("--bridge", type=Path, required=True)
    args = parser.parse_args()
    workspace = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryDirectory(prefix=".hl-", dir=workspace) as directory:
        root = Path(directory)
        binaries = root / "bin"
        binaries.mkdir(mode=0o700)
        for source, name in [(args.cli, "thumble"), (args.host, "thumble-host"),
                             (args.bridge, "thumble-cli-bridge")]:
            shutil.copy2(source.resolve(strict=True), binaries / name)
        environment = {"HOME": str(root), "PATH": "/usr/bin:/bin"}
        cli = binaries / "thumble"

        def run(*command, ok=True):
            result = subprocess.run([str(cli), *command], env=environment, cwd=root,
                                    capture_output=True, text=True, timeout=30)
            if ok:
                assert result.returncode == 0, (command, result.stdout, result.stderr)
            return result

        def status():
            frame = json.loads(run("server", "status", "--json").stdout)
            assert frame["ok"] and frame["owner"] == "rust", frame
            return frame["response"]["status"]

        try:
            assert json.loads(run("server", "status", "--json").stdout) == {"running": False}
            # Invalid options cannot launch or initialize the receiver.
            assert run("server", "start", "--port", "65536", ok=False).returncode != 0
            assert not (root / "Library/Application Support/ThumbleHost").exists()
            run("server", "start", "--port", "0", "--no-bonjour", "--no-input", "--allow-config-write")
            first = status()
            assert first["port"] > 0 and first["requestedPort"] == 0
            assert not first["inputEnabled"] and not first["bonjour"]["enabled"]
            assert first["configurationWriteEnabled"]
            state_path = root / "Library/Application Support/ThumbleHost/state.json"
            assert Path(first["statePath"]) == state_path
            original_state = hashlib.sha256(state_path.read_bytes()).hexdigest()
            code = run("pairing", "code").stdout.strip()
            assert len(code) == 6 and code.isascii() and code.isdigit()
            payload = json.loads(run("pairing", "payload").stdout)
            assert payload["pairingCode"] == code
            assert payload["serverID"] == first["serverID"]
            assert payload["urls"] == first["urls"]
            duplicate = run("server", "start", ok=False)
            assert duplicate.returncode != 0
            assert status()["pid"] == first["pid"]
            run("server", "restart")
            second = status()
            assert second["pid"] != first["pid"]
            assert second["serverID"] == first["serverID"]
            assert second["requestedPort"] == 0 and second["port"] > 0
            assert not second["inputEnabled"] and not second["bonjour"]["enabled"]
            assert second["configurationWriteEnabled"]
            assert hashlib.sha256(state_path.read_bytes()).hexdigest() == original_state
            run("server", "stop")
            assert json.loads(run("server", "status", "--json").stdout) == {"running": False}
            assert run("pairing", "payload", ok=False).returncode != 0
            run("server", "stop")  # Already-stopped requests are idempotent.
            # Simulate another native authority holding the common lock. Neither
            # a restart nor a status failure may create a competing receiver.
            with (state_path.parent / "runtime.lock").open("rb") as lock:
                fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
                for command in [("server", "start"), ("server", "restart"),
                                ("server", "status", "--json")]:
                    rejected = run(*command, ok=False)
                    assert rejected.returncode != 0, (command, rejected)
                assert not (state_path.parent / "runtime.pid").exists()
            assert hashlib.sha256(state_path.read_bytes()).hexdigest() == original_state
        finally:
            # Use the direct host for cleanup even if frontend verification fails.
            cleanup = subprocess.run([str(binaries / "thumble-host"), "stop"], env=environment,
                                     cwd=root, capture_output=True, text=True, timeout=20)
            assert cleanup.returncode == 0, ("receiver cleanup failed", cleanup.stderr)

        # An unreadable old profile must not become a fresh empty catalog when
        # the user starts the receiver through the new frontend.
        legacy_home = root / "legacy"
        preferences = legacy_home / "Library/Preferences"
        preferences.mkdir(parents=True, mode=0o700)
        legacy_home.chmod(0o700)
        source = preferences / "com.codybontecou.PocketPadMac.plist"
        original = plistlib.dumps({"PocketPadMac.serverIdentity.v1": "11111111-2222-4333-8444-555555555555",
                                  "PocketPad.gamepadConfigurationProfiles.v1": b"unreadable profile JSON"})
        source.write_bytes(original)
        environment = {"HOME": str(legacy_home), "PATH": "/usr/bin:/bin"}
        try:
            rejected = run("server", "start", "--port", "0", "--no-bonjour", "--no-input", ok=False)
            assert rejected.returncode != 0
            state_directory = legacy_home / "Library/Application Support/ThumbleHost"
            assert not (state_directory / "state.json").exists()
            assert not (state_directory / "runtime.pid").exists()
            assert source.read_bytes() == original
            assert "no new state was written" in (state_directory / "host.log").read_text()
        finally:
            cleanup = subprocess.run([str(binaries / "thumble-host"), "stop"], env=environment,
                                     cwd=root, capture_output=True, text=True, timeout=20)
            assert cleanup.returncode == 0, ("legacy receiver cleanup failed", cleanup.stderr)
        print("Headless lifecycle verified: start/status/pairing/restart/stop; options/state preserved; held authority and corrupt legacy import rejected; input disabled")


if __name__ == "__main__":
    main()
