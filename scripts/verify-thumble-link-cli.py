#!/usr/bin/env python3
"""Verify a packaged Thumble Link CLI against a private child-process home, with no input output."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("binary_directory", type=Path)
args = parser.parse_args()
binary = args.binary_directory.resolve() / "thumble"
for name in ("thumble", "thumble-cli-bridge", "thumble-bridge"):
    assert os.access(args.binary_directory.resolve() / name, os.X_OK), f"Missing executable: {name}"
root = Path(__file__).resolve().parent.parent
fixture = root / "Tests/Fixtures/ThumbleLink/warrior.txt"

with tempfile.TemporaryDirectory(prefix="thumble-link-cli-") as temporary:
    private_home = Path(temporary) / "account"
    private_home.mkdir(mode=0o700)
    # Only test subprocesses see this home; the caller's shell and real account are unchanged.
    authority = private_home / "Library/Application Support/ThumbleHost"
    environment = dict(os.environ, HOME=str(private_home),
                       THUMBLE_HOST_STATE_DIR=str(authority),
                       THUMBLE_HOST_CONTROL_SOCKET=str(authority / "control.sock"))

    def run(*arguments, data=None, success=True):
        result = subprocess.run([str(binary), *arguments], input=data, text=True,
                                capture_output=True, env=environment, timeout=60)
        if success:
            assert result.returncode == 0, result.stderr
        else:
            assert result.returncode != 0, result.stdout
        return result

    def export():
        return json.loads(run("profile", "export", "--all").stdout)

    initial = export()
    review = json.loads(run("wow", "import", str(fixture), "--dry-run", "--json").stdout)
    assert not review["imported"] and not review["isRefresh"]
    assert review["layout"] == "controller"
    assert initial["profiles"] == export()["profiles"], "dry run changed profiles"
    imported = json.loads(run("wow", "import", "-", "--no-select", "--json", data=fixture.read_text()).stdout)
    assert imported["imported"] and not imported["isRefresh"]
    first = export()
    assert len(first["profiles"]) == len(initial["profiles"]) + 1
    assert first["activeProfileID"] == initial["activeProfileID"]
    assert first["defaultProfileID"] == initial["defaultProfileID"]
    profile = next(p for p in first["profiles"] if p["id"].lower() == imported["profileID"].lower())
    assert "thumbleLink" in profile
    first_element = next(e for e in profile["customization"]["elements"] if e["output"]["keyboard"]["keyCode"] == 18)
    run("profile", "rename", profile["id"], "Warrior Phone")
    # Use the authority's supported profile import to simulate an editor move.
    # Legacy CLI element geometry writes are intentionally blocked under Rust authority.
    moved = export()
    moved_profile = next(p for p in moved["profiles"] if p["id"] == profile["id"])
    for collection in ("elements", "customButtons"):
        for element in moved_profile["customization"].get(collection, []):
            if element["id"] == first_element["id"]:
                element["layout"]["centerX"] = 0.24
                element["layout"]["centerY"] = 0.54
    native = {key: moved[key] for key in ("schema", "version", "exportedAt", "profileKeyBindings", "profileOutputBindings")}
    native.update(profiles=[moved_profile], activeProfileID=profile["id"], defaultProfileID=None)
    for field in ("profileKeyBindings", "profileOutputBindings"):
        native[field] = {key: value for key, value in native[field].items() if key.lower() == profile["id"].lower()}
    moved_file = Path(temporary) / "moved.json"
    moved_file.write_text(json.dumps(native))
    run("profile", "import", str(moved_file), "--no-select")
    before_refresh = next(p for p in export()["profiles"] if p["id"] == profile["id"])
    before_layout = next(e["layout"] for e in before_refresh["customization"]["elements"] if e["id"] == first_element["id"])
    snapshot = json.loads(fixture.read_text().removeprefix("THUMBLELINK1:"))
    snapshot["actions"][0]["keys"] = ["E"]
    snapshot["actions"][0]["label"] = "New Ability"
    refreshed = json.loads(run("wow", "import", "-", "--json", data=json.dumps(snapshot)).stdout)
    assert refreshed["isRefresh"] and refreshed["profileName"] == "Warrior Phone"
    after = export()
    assert len(after["profiles"]) == len(first["profiles"])
    updated = next(p for p in after["profiles"] if p["id"] == profile["id"])
    updated_element = next(e for e in updated["customization"]["elements"] if e["id"] == first_element["id"])
    assert updated_element["layout"] == before_layout
    assert updated_element["output"]["keyboard"]["keyCode"] == 14
    assert updated_element["label"] == "New Ability"
    assert after["defaultProfileID"] == initial["defaultProfileID"]
    era_fixture = root / "Tests/Fixtures/ThumbleLink/warrior-classic-era.txt"
    era_review = json.loads(run("wow", "import", str(era_fixture), "--dry-run", "--json").stdout)
    assert era_review["client"] == "Classic Era" and not era_review["isRefresh"]
    era_import = json.loads(run("wow", "import", str(era_fixture), "--json").stdout)
    assert era_import["imported"] and era_import["profileID"] != imported["profileID"]
    converted = json.loads(run("wow", "import", str(era_fixture), "--layout", "grid", "--json").stdout)
    assert converted["isRefresh"] and converted["layout"] == "grid"
    era_refresh = json.loads(run("wow", "import", str(era_fixture), "--json").stdout)
    assert era_refresh["isRefresh"] and era_refresh["layout"] == "grid"
    after = export()
    assert len(after["profiles"]) == len(first["profiles"]) + 1
    run("wow", "import", "-", data="x" * (256 * 1024 + 1), success=False)
    run("wow", "import", "-", data="{}", success=False)
    assert export()["profiles"] == after["profiles"], "invalid import changed profiles"
    # Existing local artwork need not be portable for an unrelated WoW import.
    # Seed a valid saved-domain fixture in this private account only.
    state_path = private_home / "Library/Application Support/ThumbleHost/state.json"
    state = json.loads(state_path.read_text())
    artwork_owner = state["profiles"][0]
    artwork_owner["customization"]["assetLibrary"] = {"assets": [{
        "id": "local-review-icon", "name": "Local review icon", "fileName": "icon.png",
        "contentType": "image/png", "role": "icon", "byteCount": 1,
        "data": "AA==", "hash": "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d"}]}
    expected_library = artwork_owner["customization"]["assetLibrary"]
    state_path.write_text(json.dumps(state))
    run("profile", "export", "--all", success=False)
    local_review = json.loads(run("wow", "import", str(era_fixture), "--dry-run", "--json").stdout)
    assert local_review["isRefresh"] and not local_review["imported"]
    local_import = json.loads(run("wow", "import", str(era_fixture), "--json").stdout)
    assert local_import["isRefresh"] and local_import["imported"]
    saved = json.loads(state_path.read_text())
    owner_after = next(p for p in saved["profiles"] if p["id"] == artwork_owner["id"])
    assert owner_after["customization"]["assetLibrary"] == expected_library
print("Thumble Link CLI checks passed: review, file/stdin import, selection, refresh, geometry, metadata, and invalid input.")
