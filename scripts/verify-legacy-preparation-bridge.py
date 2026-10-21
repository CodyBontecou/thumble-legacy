#!/usr/bin/env python3
"""Verify the built native legacy-preparation helper without authority writes."""
import argparse
import base64
import json
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("bridge", type=Path)
    args = parser.parse_args()
    executable = str(args.bridge.resolve(strict=True))
    fixtures = Path(__file__).resolve().parent.parent / "Host/fixtures/legacy-configuration-migration/v1"

    def invoke(packet):
        raw = json.dumps(packet, separators=(",", ":")).encode() + b"\n"
        result = subprocess.run([executable], input=raw, capture_output=True,
                                env={}, cwd="/", timeout=30)
        assert result.stdout.endswith(b"\n") and result.stdout.count(b"\n") == 1
        assert len(result.stdout) <= 24 * 1024 * 1024 + 1
        return result.returncode, json.loads(result.stdout)

    for name in ["plain", "portrait-only", "explicit-landscape", "legacy-canvas-alias", "missing-canvas"]:
        request = json.loads((fixtures / f"{name}-request.json").read_bytes())
        status, actual = invoke(request)
        assert status == 0, (name, actual)
        expected = json.loads((fixtures / f"{name}-response.json").read_bytes())
        assert actual == expected, name
    request = json.loads((fixtures / "missing-package-request.json").read_bytes())
    status, actual = invoke(request)
    assert status != 0 and actual["error"]["code"] == "legacy_package_missing"
    good = json.loads((fixtures / "plain-request.json").read_bytes())
    for field, value in [("profileArtifactCommand", "validate"), ("skinCommand", "apply"),
                         ("phoneLayoutCommand", "apply"), ("path", "/tmp/unused")]:
        status, actual = invoke({**good, field: value})
        assert status != 0 and "error" in actual, field
    unknown = {**good}
    profile = json.loads(base64.b64decode(unknown["profileData"]))
    profile["customization"]["deviceCanvas"] = {"frameID": "unknown-future-device"}
    unknown["profileData"] = base64.b64encode(json.dumps(profile,separators=(",", ":")).encode()).decode()
    status, actual = invoke(unknown)
    assert status != 0 and actual["error"]["code"] == "legacy_profile_recovery_required"
    print("Native legacy preparation verified: 5 exact responses, missing-package and unknown-canvas recovery, 4 mixed/unexpected request rejections")


if __name__ == "__main__":
    main()
