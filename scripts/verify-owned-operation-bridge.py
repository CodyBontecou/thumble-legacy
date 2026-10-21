#!/usr/bin/env python3
"""Exercise owned native edit packets against the real built helper, without state writes."""
import argparse
import base64
import json
from pathlib import Path
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("bridge", type=Path)
args = parser.parse_args()
executable = str(args.bridge.resolve(strict=True))
fixtures = Path(__file__).resolve().parent.parent / "Host/fixtures/owned-native-operations/v1"
names = ["nullable-select", "geometry", "haptic", "add", "delete", "scheme"]

def invoke(request):
    result = subprocess.run([executable], input=json.dumps(request,separators=(",",":")).encode()+b"\n",
                            capture_output=True, env={}, cwd="/", timeout=30)
    assert len(result.stdout) <= 24*1024*1024+1
    assert result.stdout.endswith(b"\n") and result.stdout.count(b"\n") == 1
    return result.returncode, json.loads(result.stdout)

for name in names:
    status, actual = invoke(json.loads((fixtures/f"{name}-request.json").read_bytes()))
    assert status == 0, (name, actual)
    assert actual == json.loads((fixtures/f"{name}-response.json").read_bytes()), name
request = json.loads((fixtures/"geometry-request.json").read_bytes())
for field in ["legacyProfileCommand","profileArtifactCommand","phoneLayoutCommand","skinCommand","path"]:
    status, result = invoke({**request,field:"unused"})
    assert status != 0 and "error" in result, field
portrait = {**request,"operation":{**request["operation"],"variant":"portrait"}}
status, result = invoke(portrait)
assert status != 0 and result["error"]["code"] == "owned_retired_operation"
source = json.loads(base64.b64decode(request["documentData"]))
source["version"] = 4
status, result = invoke({**request,"documentData":base64.b64encode(json.dumps(source).encode()).decode()})
assert status != 0 and "error" in result
print("Native owned editing verified: 6 exact responses, 5 mixed/unexpected requests, portrait and legacy-document rejection")
