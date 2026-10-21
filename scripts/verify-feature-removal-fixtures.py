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
for name, expected in sequence_contract.get("files", {}).items():
    assert Path(name).name == name
    assert hashlib.sha256((sequence_root / name).read_bytes()).hexdigest() == expected
print("Owned duplication packets verified against native sequence fixture hashes")

# Exact-byte owned portable artifacts are distinct from v4/JCS artifacts.
portable = repo / "Host/fixtures/owned-profile-artifact/v1"
contract = json.loads((portable / "manifest.json").read_text())
assert contract["schemaVersion"] == 1 and contract["configurationVersion"] == 5
source = portable / contract["source"]
assert source.resolve().is_relative_to((repo / "Host/fixtures").resolve())
assert hashlib.sha256(source.read_bytes()).hexdigest() == contract["sourceSHA256"]
for name, digest in contract["files"].items():
    assert Path(name).name == name
    assert hashlib.sha256((portable / name).read_bytes()).hexdigest() == digest
artifact_text = (portable / "artifact.json").read_text()
artifact = json.loads(artifact_text)
start = artifact_text.index('"document":') + len('"document":')
_, end = json.JSONDecoder().raw_decode(artifact_text, start)
assert hashlib.sha256(artifact_text[start:end].encode("utf-8")).hexdigest() == artifact["contentHash"]["value"]
assert artifact["schema"] == "com.codybontecou.thumble.owned-profile-artifact"
assert artifact["artifactVersion"] == 1
assert artifact["contentHash"]["canonicalization"] == contract["hashEncoding"] == "exact-json-utf8"
assert artifact["document"]["version"] == 5
assert all("launchTarget" not in p for p in artifact["document"]["profiles"])
print("Owned portable artifact fixture verified; activation remains gated")

# Portable legacy preparation is opt-in; old migration responses stay frozen.
portable_legacy = repo / "Host/fixtures/legacy-portable-preparation/v1"
contract = json.loads((portable_legacy / "manifest.json").read_text())
assert contract["fixtureVersion"] == 1 and contract["portableMode"] is True
assert contract["caseCount"] == 5 and len(contract["files"]) == 6
for name, digest in contract["files"].items():
    assert Path(name).name == name
    assert hashlib.sha256((portable_legacy / name).read_bytes()).hexdigest() == digest
for name, digest in contract["sources"].items():
    assert not Path(name).is_absolute() and ".." not in Path(name).parts
    assert hashlib.sha256((repo / name).read_bytes()).hexdigest() == digest
print("Portable legacy preparation verified: five opt-in native responses; original fixtures unchanged")

# Canonical phone intents are a staging contract, not an activated transfer format.
phone = repo / "Host/fixtures/owned-phone-edits/v1"
contract = json.loads((phone / "manifest.json").read_text())
assert contract["fixtureVersion"] == 1 and contract["schemaVersion"] == 1
assert contract["wireActivated"] is False and contract["advertisedProfileFormats"] == [4]
for name, digest in contract["files"].items():
    assert Path(name).name == name
    assert hashlib.sha256((phone / name).read_bytes()).hexdigest() == digest
assert len(json.loads((phone / "cases.json").read_text())["cases"]) == contract["caseCount"]
print("Owned phone intent fixtures verified; public peer activation remains gated")
schema = Path(contract["schema"])
assert not schema.is_absolute() and ".." not in schema.parts
assert hashlib.sha256((repo / schema).read_bytes()).hexdigest() == contract["schemaSHA256"]

# Receiver wire support does not activate shipping phone profile offers.
phone_wire = repo / "Host/fixtures/owned-phone-wire/v1"
contract = json.loads((phone_wire / "manifest.json").read_text())
assert contract["fixtureVersion"] == 1
assert contract["advertisedPhoneFormats"] == [4]
assert contract["automaticMigrationActivated"] is False
for name, digest in contract["files"].items():
    assert Path(name).name == name
    assert hashlib.sha256((phone_wire / name).read_bytes()).hexdigest() == digest
frames = json.loads((phone_wire / "frames.json").read_text())
assert len(frames) == contract["frameCount"] == 5
assert frames[0]["type"] == "owned_phone_edit"
for frame in frames[1:]:
    result = frame["ownedPhoneEditResult"]
    assert frame["configurationRevision"] == result["configurationRevision"]
assert frames[2]["ownedPhoneEditResult"]["committedRevision"] == 42
assert frames[2]["configurationRevision"] == 43
print("Owned phone wire frames verified; historical receipts retain current authority revision")

# Asset command identity, original-revision retries and migration activation stay explicit.
asset_root = repo / "Host/fixtures/owned-assets/v1"
asset_contract = json.loads((asset_root / "manifest.json").read_text())
assert asset_contract["fixtureVersion"] == 1 and asset_contract["automaticMigrationActivated"] is False
for name, digest in asset_contract["files"].items():
    assert Path(name).name == name
    assert hashlib.sha256((asset_root / name).read_bytes()).hexdigest() == digest
asset_requests = json.loads((asset_root / "requests.json").read_text())
assert len(asset_requests) == asset_contract["requestCount"] == 3
assert [row["expectedConfigurationRevision"] for row in asset_requests] == [41, 42, 43]
assert [row["command"]["type"] for row in asset_requests] == ["asset.import", "asset.set", "asset.remove"]
assert all(row["schemaVersion"] == 8 for row in asset_requests)
assert asset_requests[1]["command"]["assetID"] == asset_contract["importedAssetID"]
assert asset_requests[2]["command"]["assetID"] == asset_contract["importedAssetID"]
print("Owned asset import/set/remove fixture verified; public format activation unchanged")

# Reviewed designs are compiler intermediates, never installed runtime skins.
_owned_design = repo / "Host/fixtures/owned-design/v1"
_owned_design_manifest = json.loads((_owned_design / "manifest.json").read_bytes())
for _name, _hash in _owned_design_manifest["files"].items():
    assert hashlib.sha256((_owned_design / _name).read_bytes()).hexdigest() == _hash, _name
_design_request = json.loads((_owned_design / "apply-request.json").read_bytes())
_design_response = json.loads((_owned_design / "apply-response.json").read_bytes())
assert _design_request["operation"]["type"] == "design.apply"
assert _design_response["invocationID"].lower() == _design_request["invocationID"].lower()
assert _design_response["sourceSHA256"] == hashlib.sha256(base64.b64decode(_design_request["documentData"], validate=True)).hexdigest()
assert _design_request["operation"]["packageSHA256"] == hashlib.sha256(base64.b64decode(_design_request["operation"]["packageBase64"], validate=True)).hexdigest()
_design_after = json.loads(base64.b64decode(_design_response["afterDocumentData"], validate=True))
_design_target = next(p for p in _design_after["profiles"] if p["id"].lower() == _design_request["operation"]["profileID"].lower())
assert _design_target["profileFormatVersion"] == 5 and "skinReference" not in _design_target
assert len(_design_target["appearanceBank"]["variants"]) == _owned_design_manifest["appearanceStates"] == 8
assert len({(v["colorScheme"], v["actionSet"]) for v in _design_target["appearanceBank"]["variants"]}) == 8
assert _owned_design_manifest["automaticMigrationActivated"] is False
print("Owned reviewed-design fixture verified; no runtime package attachment or public format activation")

# Resource-aware style catalogs use the native owned operation boundary.
_style_root = repo / "Host/fixtures/style-library/v1"
_style_manifest = json.loads((_style_root / "manifest.json").read_bytes())
assert _style_manifest["automaticMigrationActivated"] is False
for _name, _digest in _style_manifest["files"].items():
    assert Path(_name).name == _name
    assert hashlib.sha256((_style_root / _name).read_bytes()).hexdigest() == _digest, _name
for _case in _style_manifest["cases"]:
    _request = json.loads((_style_root / (_case + "-request.json")).read_bytes())
    _response = json.loads((_style_root / (_case + "-response.json")).read_bytes())
    assert _request["operation"]["type"] == "style.import"
    assert _response["invocationID"].lower() == _request["invocationID"].lower()
    assert _response["sourceSHA256"] == hashlib.sha256(base64.b64decode(_request["documentData"], validate=True)).hexdigest()
    _after = json.loads(base64.b64decode(_response["afterDocumentData"], validate=True))
    _target = next(p for p in _after["profiles"] if p["id"].lower() == _request["operation"]["profileID"].lower())
    assert len(_target["appearanceBank"]["variants"]) == _style_manifest["appearanceStates"] == 8
    assert all(len(v["appearance"]["styleLibrary"]["styles"]) >= 2 for v in _target["appearanceBank"]["variants"])
print("Native style-library imports verified: plain/banked eight-state catalogs")

_reset_root = repo / "Host/fixtures/customization-reset/v1"
_reset_manifest = json.loads((_reset_root / "manifest.json").read_bytes())
assert _reset_manifest["automaticMigrationActivated"] is False
for _name, _digest in _reset_manifest["files"].items():
    assert Path(_name).name == _name
    assert hashlib.sha256((_reset_root / _name).read_bytes()).hexdigest() == _digest
for _case in _reset_manifest["cases"]:
    _request = json.loads((_reset_root / (_case + "-request.json")).read_bytes())
    _response = json.loads((_reset_root / (_case + "-response.json")).read_bytes())
    assert _request["operation"]["type"] in ("customization.reset", "profile.reset")
    assert _response["sourceSHA256"] == hashlib.sha256(base64.b64decode(_request["documentData"], validate=True)).hexdigest()
    assert _request["invocationID"].lower() == _response["invocationID"].lower()
print("Owned reset fixtures verified: plain/banked customization and profile reset")

_customization_root = repo / "Host/fixtures/customization-interchange/v1"
_customization_manifest = json.loads((_customization_root / "manifest.json").read_bytes())
for _name,_digest in _customization_manifest["files"].items():
    assert Path(_name).name == _name
    assert hashlib.sha256((_customization_root / _name).read_bytes()).hexdigest() == _digest
_customization_file = json.loads((_customization_root / "banked.json").read_bytes())
assert _customization_file["schema"] == "com.codybontecou.thumble.customization"
assert len(_customization_file["appearanceBank"]["variants"]) == _customization_manifest["appearanceStates"] == 8
assert _customization_manifest["automaticMigrationActivated"] is False
print("Customization interchange fixtures verified: pooled graph and native legacy operation")

rich = repo / "Host/fixtures/rich-background/v1"
contract = json.loads((rich / "manifest.json").read_text())
assert contract["fixtureVersion"] == 1 and contract["appearanceStates"] == 8 and contract["maximumImageBytes"] == 2500000
for name, digest in contract["files"].items():
    assert Path(name).name == name and hashlib.sha256((rich / name).read_bytes()).hexdigest() == digest
for name, digest in contract["sources"].items():
    assert not Path(name).is_absolute() and ".." not in Path(name).parts
    assert hashlib.sha256((repo / name).read_bytes()).hexdigest() == digest
assert json.loads((rich / "request.json").read_text())["operation"]["changes"]["backgroundEdit"]["fill"]["kind"] == "image"
print("Native rich-background fixture verified; owned metadata/resource merge and eight-state paint remain scoped")
