#!/usr/bin/env python3
"""Check built CLI pairing/address commands using a correlated test bridge.

This checks command dispatch/output and ownership routing, not real networking,
phone pairing, receiver startup, or operating-system input authorization.
"""
import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tempfile


BRIDGE = '''#!/usr/bin/python3
import json
from pathlib import Path
import sys
root = Path(__file__).parent
request = json.loads(sys.stdin.readline())
assert request["runtimeCommand"] == {"type": "status"}
with (root / "requests.jsonl").open("a") as log:
    log.write(json.dumps(request) + "\\n")
status = {"pid": 42, "port": 9123, "serviceName": "Headless receiver",
          "accessibilityTrusted": True, "inputEnabled": True,
          "output": {"mode": "recording"},
          "urls": ["ws://192.168.1.42:9123"], "pairingCode": "123456",
          "serverID": "11111111-2222-4333-8444-555555555555"}
if (root / "incomplete").exists():
    del status["pairingCode"]
print(json.dumps({"schemaVersion": 1, "ok": True,
                  "invocationID": request["invocationID"], "owner": "rust",
                  "response": {"ok": True, "status": status}}, separators=(",", ":")))
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("cli", type=Path, help="Built standalone thumble executable")
    args = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix="thumble-pairing-cli-") as directory:
        root = Path(directory)
        cli = root / "thumble"
        shutil.copy2(args.cli.resolve(strict=True), cli)
        helper = root / "thumble-cli-bridge"
        helper.write_text(BRIDGE)
        helper.chmod(0o700)

        def run(*command):
            return subprocess.run([str(cli), *command], cwd=root,
                                  capture_output=True, text=True, timeout=15)

        for command, expected in [(("pairing", "code"), "123456\n"),
                                  (("server", "addresses"), "ws://192.168.1.42:9123\n"),
                                  (("server", "urls"), "ws://192.168.1.42:9123\n")]:
            result = run(*command)
            assert result.returncode == 0, (command, result.stderr)
            assert result.stdout == expected, (command, result.stdout)
        result = run("pairing", "payload")
        assert result.returncode == 0, result.stderr
        payload = json.loads(result.stdout)
        assert payload["type"] == "pocketpad-pair"
        assert payload["urls"] == ["ws://192.168.1.42:9123"]
        assert payload["pairingCode"] == "123456"
        assert payload["serverID"] == "11111111-2222-4333-8444-555555555555"
        assert payload["serviceName"] == "Headless receiver"
        assert payload["serviceType"] == "_pocketpad._tcp"
        assert payload["serviceDomain"] == "local"
        (root / "incomplete").touch()
        result = run("pairing", "payload")
        assert result.returncode != 0 and not result.stdout, result
        requests = [json.loads(line) for line in (root / "requests.jsonl").read_text().splitlines()]
        assert len(requests) == 5
        assert len({request["invocationID"] for request in requests}) == 5
        print("Headless pairing CLI dispatch verified: 4 outputs and 1 nonmutating rejection")


if __name__ == "__main__":
    main()
