#!/usr/bin/env python3
"""Verify the immutable pre-removal fixtures without regenerating expectations."""
import hashlib
import json
import base64
from pathlib import Path

root = Path(__file__).resolve().parent.parent / "Host/fixtures/feature-removal/v1"
manifest = json.loads((root / "manifest.json").read_text())
assert manifest["fixtureVersion"] == 1
for name, expected in manifest["files"].items():
    assert Path(name).name == name
    actual = hashlib.sha256((root / name).read_bytes()).hexdigest()
    assert actual == expected, f"Frozen baseline changed: {name}"
for name, expected in manifest["renders"].items():
    assert Path(name).name == name
    actual = hashlib.sha256((root / "previews" / name).read_bytes()).hexdigest()
    assert actual == expected, f"Frozen native render changed: {name}"
for name, count in [("appearance", "appearanceCases"), ("legacy-profiles", "legacyProfileCases"), ("pending-edits", "pendingEditCases"), ("action-sets", "actionSetCases"), ("migration-cases", "migrationCases")]:
    assert len(json.loads((root / f"{name}.json").read_text())["cases"]) == manifest[count]
print("Frozen feature-removal fixtures verified against checkpoint", manifest["checkpoint"])

# Later native edit oracles are separate from the untouched pre-removal baseline.
repo = root.parents[3]
edits = repo / "Host/fixtures/appearance-edits/v1"
contract = json.loads((edits / "manifest.json").read_text())
assert contract["fixtureVersion"] == 1
assert contract["normalizedColorChannelTolerance"] == 2e-7
assert hashlib.sha256((edits / "fixture.json").read_bytes()).hexdigest() == contract["fixtureSHA256"]
assert hashlib.sha256((repo / contract["nativeGenerator"]).read_bytes()).hexdigest() == contract["nativeGeneratorSHA256"]
oracle = json.loads((edits / "fixture.json").read_text())
assert oracle["schemaVersion"] == 1 and len(oracle["cases"]) == contract["caseCount"]
assert len({case["name"] for case in oracle["cases"]}) == len(oracle["cases"])
expected = {(scheme, action) for scheme in ["light", "dark"] for action in ["base", "left-trigger", "right-trigger", "both-triggers"]}
for case in oracle["cases"]:
    assert len(case["resolutions"]) == 8
    assert {(row["colorScheme"], row["actionSet"]) for row in case["resolutions"]} == expected
    ids = {element["id"].lower() for element in case["profile"]["customization"]["elements"]}
    assert all(set(row["expectedAppearance"]["controls"]) == ids for row in case["resolutions"])
assert sum(len(case["resolutions"]) for case in oracle["cases"]) == contract["resolutionCount"]
print("Native appearance edit oracle verified:", contract["caseCount"], "cases,", contract["resolutionCount"], "results")

# The profile-format matrix is a later hand-authored compatibility contract.
formats = repo / "Host/fixtures/profile-formats"
contract = json.loads((formats / "manifest.json").read_text())
assert contract["fixtureVersion"] == 1 and contract["advertisedFormats"] == [4]
assert hashlib.sha256((formats / "v1.json").read_bytes()).hexdigest() == contract["fixtureSHA256"]
matrix = json.loads((formats / "v1.json").read_text())
assert matrix["schemaVersion"] == 1
for key, count in [("negotiations", "negotiationCount"), ("frames", "frameCount")]:
    rows = matrix[key]
    assert len(rows) == contract[count] and len({row["name"] for row in rows}) == len(rows)
    assert all(isinstance(row["allowed"], bool) for row in rows)
print("Profile-format matrix verified:", contract["negotiationCount"], "negotiations,", contract["frameCount"], "frames")

# Explicit v5 destination specimens; not outputs of an activated migration.
owned = repo / "Host/fixtures/owned-configuration/v1"
contract = json.loads((owned / "manifest.json").read_text())
assert contract["fixtureVersion"] == 1
for name, digest in contract["files"].items():
    assert Path(name).name == name
    assert hashlib.sha256((owned / name).read_bytes()).hexdigest() == digest
for name, digest in contract["sources"].items():
    assert not Path(name).is_absolute() and ".." not in Path(name).parts
    assert hashlib.sha256((repo / name).read_bytes()).hexdigest() == digest
rows = json.loads((owned / "cases.json").read_text())["cases"]
assert len(rows) == contract["caseCount"] and len({row["name"] for row in rows}) == len(rows)
document = json.loads((owned / "document.json").read_text())
assert document["version"] == 5 and all(p["profileFormatVersion"] == 5 for p in document["profiles"])
retired = {"skinReference", "skinBaselineCustomization", "landscapeSkinBaselineCustomization", "portraitSkinBaselineCustomization", "landscapeCustomization", "portraitCustomization", "orientationPreference"}
assert all(not (retired & p.keys()) for p in document["profiles"])
print("Explicit owned-configuration contract verified:", contract["caseCount"], "cases")

# A real Rust writer result, independently rendered/projected by retained peers.
written = repo / "Host/fixtures/owned-configuration-edits/v1"
contract = json.loads((written / "manifest.json").read_text())
assert contract["fixtureVersion"] == 1
for name, digest in contract["files"].items():
    assert Path(name).name == name
    assert hashlib.sha256((written / name).read_bytes()).hexdigest() == digest
for field, digest in [("baseDocument", "baseSHA256"), ("generator", "generatorSHA256")]:
    path = Path(contract[field])
    assert not path.is_absolute() and ".." not in path.parts
    assert hashlib.sha256((repo / path).read_bytes()).hexdigest() == contract[digest]
assert len(json.loads((written / "edits.json").read_text())["edits"]) == contract["operationCount"]
print("Lossless owned-edit destination verified:", contract["operationCount"], "operations")

# Native typed views are merged into original source, never substituted for it.
merged = repo / "Host/fixtures/owned-projection-merge/v1"
contract = json.loads((merged / "manifest.json").read_text())
assert contract["fixtureVersion"] == 1 and contract["activatedMigration"] is False
for name, digest in contract["files"].items():
    assert Path(name).name == name
    assert hashlib.sha256((merged / name).read_bytes()).hexdigest() == digest
for name, digest in contract["sources"].items():
    assert not Path(name).is_absolute() and ".." not in Path(name).parts
    assert hashlib.sha256((repo / name).read_bytes()).hexdigest() == digest
before = json.loads((merged / "before.json").read_text())
after = json.loads((merged / "after.json").read_text())
destination_bytes = (merged / "destination.json").read_bytes()
destination = json.loads(destination_bytes)
assert all(len(value["profiles"]) == contract["profileCount"] for value in [before, after, destination])
assert all(value["version"] == 5 for value in [before, after, destination])
assert "futureRootMetadata" not in before and "futureRootMetadata" not in after
assert "futureRootMetadata" in destination and "futureProfileMetadata" in destination["profiles"][0]
assert b"18446744073709551617" in destination_bytes
assert b"0.12345678901234567890123456789" in destination_bytes
assert len(destination["profiles"][0]["appearanceBank"]["variants"]) == contract["appearanceStates"]
assert destination["profiles"][0]["name"] == "Edited keypad"
print("Native projection merge verified:", contract["profileCount"], "profiles,", contract["appearanceStates"], "appearance states")

# Actual legacy inputs prepared by native code and composed into owned documents.
legacy = repo / "Host/fixtures/legacy-configuration-migration/v1"
contract = json.loads((legacy / "manifest.json").read_text())
assert contract["fixtureVersion"] == 1 and contract["activationEnabled"] is False
for name, digest in contract["files"].items():
    assert Path(name).name == name
    assert hashlib.sha256((legacy / name).read_bytes()).hexdigest() == digest
for name, digest in contract["sources"].items():
    assert not Path(name).is_absolute() and ".." not in Path(name).parts
    assert hashlib.sha256((repo / name).read_bytes()).hexdigest() == digest
for name in contract["convertedCases"]:
    profile_data = (legacy / f"{name}-profile.json").read_bytes()
    request = json.loads((legacy / f"{name}-request.json").read_bytes())
    response = json.loads((legacy / f"{name}-response.json").read_bytes())
    assert base64.b64decode(request["profileData"], validate=True) == profile_data
    assert response["sourceSHA256"] == hashlib.sha256(profile_data).hexdigest()
    assert response["invocationID"].lower() == request["invocationID"].lower()
    if "packageData" in request:
        assert response["packageSHA256"] == hashlib.sha256(base64.b64decode(request["packageData"], validate=True)).hexdigest()
    else:
        assert response.get("packageSHA256") is None
    source = json.loads((legacy / f"{name}-source.json").read_bytes())
    destination_data = (legacy / f"{name}-destination.json").read_bytes()
    destination = json.loads(destination_data)
    assert source["version"] == 4 and destination["version"] == 5
    original = source["profiles"][0]
    converted = destination["profiles"][0]
    assert original == json.loads(profile_data)
    assert converted["profileFormatVersion"] == 5 and not (retired & converted.keys())
    for field in ["id", "name", "updatedAt", "outputMode", "futureMigrationMetadata", "futureNativeMigrationMetadata"]:
        assert converted[field] == original[field]
    for field in ["activeProfileID", "defaultProfileID", "keyBindings", "outputBindings", "profileKeyBindings", "profileOutputBindings", "futureRootMetadata"]:
        assert destination[field] == source[field]
    selected = original[response["sourceLayout"]]
    assert converted["customization"]["futureLayoutMetadata"] == selected["futureLayoutMetadata"]
    assert {item["id"] for item in selected["elements"]} == {item["id"] for item in converted["customization"]["elements"]}
    assert b"18446744073709551617" in destination_data and b"0.12345678901234567890123456789" in destination_data
    if name == "portrait-only":
        assert len(converted["appearanceBank"]["variants"]) == contract["appearanceStates"]
        assert "portraitArranged" in response["reviewReasons"]
    if name == "explicit-landscape":
        assert response["sourceLayout"] == "landscapeCustomization"
        assert converted["customization"]["colorSchemePreference"] == original["customization"]["colorSchemePreference"]
for name in contract["recoveryCases"]:
    assert not (legacy / f"{name}-destination.json").exists()
print("Legacy configuration preparation verified:", len(contract["convertedCases"]), "conversions,", len(contract["recoveryCases"]), "recovery case")

owned_ops = root.parent.parent / "owned-native-operations/v1"
contract = json.loads((owned_ops / "manifest.json").read_bytes())
assert contract["fixtureVersion"] == 1
repository = root.parents[3]
for name, expected in contract["files"].items():
    assert Path(name).name == name
    assert hashlib.sha256((owned_ops / name).read_bytes()).hexdigest() == expected, name
for name, expected in contract["sources"].items():
    assert hashlib.sha256((repository / name).read_bytes()).hexdigest() == expected, name
for name in contract["cases"]:
    request = json.loads((owned_ops/f"{name}-request.json").read_bytes())
    response = json.loads((owned_ops/f"{name}-response.json").read_bytes())
    source_data = base64.b64decode(request["documentData"], validate=True)
    source = json.loads(source_data)
    destination_data = (owned_ops/f"{name}-destination.json").read_bytes()
    destination = json.loads(destination_data)
    projection = json.loads(base64.b64decode(response["afterDocumentData"], validate=True))
    assert response["sourceSHA256"] == hashlib.sha256(source_data).hexdigest()
    assert response["invocationID"].lower() == request["invocationID"].lower()
    assert response["changed"] and response["changedPaths"]
    assert source["version"] == destination["version"] == projection["version"] == 5
    assert [p["id"] for p in source["profiles"]] == [p["id"] for p in destination["profiles"]]
    assert destination["activeProfileID"] == projection["activeProfileID"]
    assert destination["defaultProfileID"] == projection["defaultProfileID"]
    for key in ["futureRootMetadata"]:
        if key in source:
            assert destination[key] == source[key]
    for key in ["keyBindings","outputBindings","profileKeyBindings","profileOutputBindings"]:
        assert destination.get(key,{}) == projection.get(key,{})
    for before, after, known in zip(source["profiles"],destination["profiles"],projection["profiles"]):
        assert after["profileFormatVersion"] == 5
        assert not set(after).intersection(["skinReference","landscapeCustomization","portraitCustomization","orientationPreference"])
        if "futureProfileMetadata" in before:
            assert after["futureProfileMetadata"] == before["futureProfileMetadata"]
        assert after["customization"] == known["customization"]
        assert after.get("appearanceBank") == known.get("appearanceBank")
        if "appearanceBank" in after:
            variants = after["appearanceBank"]["variants"]
            assert len(variants) == 8
            ids = {e["id"].lower() for e in after["customization"]["elements"]}
            for variant in variants:
                assert {key.lower() for key in variant["appearance"]["controls"]} == ids
    for literal in [b"18446744073709551617",b"0.12345678901234567890123456789"]:
        if literal in source_data:
            assert literal in destination_data
    if name == "nullable-select":
        assert source["defaultProfileID"] is None and destination["defaultProfileID"] is None
        assert source["activeProfileID"] != destination["activeProfileID"]
print("Owned native edits verified:",len(contract["cases"]),"operations across eight appearance states")

sequence_root = Path(__file__).resolve().parent.parent / "Host/fixtures/owned-element-sequences/v1"
sequence_contract = json.loads((sequence_root / "manifest.json").read_text())
sequence_bytes = (sequence_root / "after-reset-and-hide.json").read_bytes()
assert sequence_contract["schemaVersion"] == 1
assert hashlib.sha256(sequence_bytes).hexdigest() == sequence_contract["sourceSHA256"]
sequence = json.loads(sequence_bytes)
assert sequence["version"] == 5
profile = sequence["profiles"][0]
removed_id = sequence_contract["removedElementID"].lower()
preset = next(e for e in profile["customization"]["elements"] if e["id"].lower().endswith("0109"))
assert preset["layout"]["isHidden"] and "visualStyle" not in preset["layout"]
assert any(e["id"].lower() == removed_id for e in profile["customization"]["elements"])
assert len(profile["appearanceBank"]["variants"]) == 8
assert all(removed_id in row["appearance"]["controls"] for row in profile["appearanceBank"]["variants"])
print("Owned chained element fixture verified: reset/hide followed by custom removal")
