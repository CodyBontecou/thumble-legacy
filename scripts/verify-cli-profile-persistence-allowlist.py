#!/usr/bin/env python3
"""Fail CI when direct Swift profile-store persistence expands silently."""

from __future__ import annotations

import collections
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SOURCE = ROOT / "Sources/CLI/ThumbleCLI.swift"
EXPECTED = {
    "skin": (3, 2),
    "renderSkinPreview": (2, 0),  # mutually exclusive lazy reads; artifact previews need no store
    "mutateProfileResources": (1, 1),
    "asset": (1, 0),  # legacy asset metadata mutation only; reads use authority snapshots
}

function = "<top-level>"
actual: dict[str, list[int]] = collections.defaultdict(lambda: [0, 0])
for line in SOURCE.read_text(encoding="utf-8").splitlines():
    match = re.match(r"\s*private static func\s+([A-Za-z0-9_]+)", line)
    if match:
        function = match.group(1)
    for index, symbol in enumerate(("loadStore", "persistStore")):
        if re.search(rf"\b{symbol}\s*\(", line) and not re.match(
            rf"\s*private static func {symbol}", line
        ):
            actual[function][index] += 1

normalized = {name: tuple(counts) for name, counts in actual.items()}
if normalized != EXPECTED:
    print("Direct loadStore/persistStore allowlist changed.", file=sys.stderr)
    print(f"Expected: {EXPECTED}", file=sys.stderr)
    print(f"Actual:   {normalized}", file=sys.stderr)
    print(
        "Migrate the call or explicitly review and update this narrow allowlist.",
        file=sys.stderr,
    )
    raise SystemExit(1)

source_text = SOURCE.read_text(encoding="utf-8")
for method in ("validateLayout", "previewLayout"):
    block = source_text.split(f"private static func {method}(", 1)[1].split("private static func ", 1)[0]
    if "captureInspectionProfile" not in block or "loadStore(" in block or "persistStore(" in block:
        raise SystemExit(f"{method} must use read-only authority capture without desktop persistence")
asset_reads = source_text.split("private static func asset(arguments:", 1)[1].split('case "import":', 1)[0]
if asset_reads.count("captureInspectionProfile") != 2 or "loadStore(" in asset_reads or "persistStore(" in asset_reads:
    raise SystemExit("Asset list/show must use read-only authority capture")

profile_block = source_text.split("private static func moveProfiles", 1)[0]
for command in ("select", "set-default", "rename", "duplicate", "delete", "reset"):
    if f'"{command}"' not in profile_block:
        print(f"Missing migrated profile command routing: {command}", file=sys.stderr)
        raise SystemExit(1)

profile_transfer_block = source_text.split(
    "private static func exportProfiles", 1
)[1].split("private static func attachApplicationToProfile", 1)[0]
for forbidden in ("loadStore(", "persistStore(", "Data(contentsOf:"):
    if forbidden in profile_transfer_block:
        print(f"Migrated profile export/import routing still uses {forbidden}", file=sys.stderr)
        raise SystemExit(1)
for command in (".export(", ".import("):
    if command not in profile_transfer_block:
        print(f"Missing Rust-authoritative profile transfer routing: {command}", file=sys.stderr)
        raise SystemExit(1)
for required in (
    "maximumProfileArtifactBytes", "O_NOFOLLOW", "Darwin.open", "Darwin.fstat",
    "S_IFREG", "Darwin.read", "Darwin.close", "String(decoding:",
):
    if required not in profile_transfer_block:
        print(f"Missing bounded profile artifact IO guard: {required}", file=sys.stderr)
        raise SystemExit(1)
for required in ("artifact.artifactJSON", "options: .atomic", "FileHandle.standardOutput"):
    if required not in profile_transfer_block:
        print(f"Missing raw profile export behavior: {required}", file=sys.stderr)
        raise SystemExit(1)
if "attributesOfItem" in profile_transfer_block:
    print("Profile import must inspect the opened descriptor without a path pre-check", file=sys.stderr)
    raise SystemExit(1)
if "hasExplicitProfileArtifactSchema" not in profile_transfer_block:
    print("Missing explicit profile artifact schema routing", file=sys.stderr)
    raise SystemExit(1)

generation_block = source_text.split(
    "private static func generate(arguments:", 1
)[1].split("// MARK: - Profiles", 1)[0]
for forbidden in ("loadStore(", "persistStore(", "requireExplicitUnmigratedProfileAccess("):
    if forbidden in generation_block:
        print(f"Migrated generation routing still uses {forbidden}", file=sys.stderr)
        raise SystemExit(1)
for command in ("generationGenerate", "generationPlanSpec", ".import("):
    if command not in generation_block:
        print(f"Missing Rust-authoritative generation routing: {command}", file=sys.stderr)
        raise SystemExit(1)

spec_generation_block = generation_block.split(
    "private static func generateFromSpec", 1
)[1].split("private static func prepareGeneratedProfile", 1)[0]
plan_index = spec_generation_block.find("generationPlanSpec")
import_index = spec_generation_block.find(".import(")
if plan_index < 0 or import_index <= plan_index:
    print("Spec generation must plan through Rust before backend import", file=sys.stderr)
    raise SystemExit(1)
for required in (
    "plan.artifactJSON", "appendAsCopies: false", "planningResponse.invocationID",
    "expectedConfigurationRevision: plan.configurationRevision", "importResponse.outcome",
):
    if required not in spec_generation_block:
        print(f"Missing transactional spec-generation import guard: {required}", file=sys.stderr)
        raise SystemExit(1)

spec_io_block = generation_block.split(
    "private static func readBoundedGenerationSpec(path:", 1
)[1]
for required in (
    "maximumGenerationSpecBytes", "O_NOFOLLOW", "Darwin.open", "Darwin.fstat",
    "S_IFREG", "Darwin.read", "Darwin.close", "String(data:",
):
    if required not in spec_io_block:
        print(f"Missing bounded generation-spec IO guard: {required}", file=sys.stderr)
        raise SystemExit(1)
if "Data(contentsOf:" in spec_io_block:
    print("Generation spec must be read from the safely opened descriptor", file=sys.stderr)
    raise SystemExit(1)

template_block = source_text.split(
    "private static func template(arguments:", 1
)[1].split("// MARK: - Themes", 1)[0]
if "templateInstall" not in template_block:
    print("Missing Rust-authoritative template install routing", file=sys.stderr)
    raise SystemExit(1)
if "loadStore(" in template_block or "persistStore(" in template_block:
    print("Migrated template install routing still uses direct persistence", file=sys.stderr)
    raise SystemExit(1)

orientation_block = SOURCE.read_text(encoding="utf-8").split(
    "private static func orientation", 1
)[1].split("private static func parseProfileLayoutVariant", 1)[0]
for forbidden in ("loadStore(", "persistStore("):
    if forbidden in orientation_block:
        print(f"Migrated orientation routing still uses {forbidden}", file=sys.stderr)
        raise SystemExit(1)
for command in ("orientationGet", "orientationSet", "orientationCopy"):
    if command not in orientation_block:
        print(f"Missing Rust-authoritative orientation routing: {command}", file=sys.stderr)
        raise SystemExit(1)

binding_output_block = SOURCE.read_text(encoding="utf-8").split(
    "// MARK: - Bindings", 1
)[1].split("// MARK: - Customization", 1)[0]
for forbidden in ("loadStore(", "persistStore("):
    if forbidden in binding_output_block:
        print(f"Migrated binding/output routing still uses {forbidden}", file=sys.stderr)
        raise SystemExit(1)
for command in (
    "bindingList", "bindingDisplay", "bindingSet", "bindingClear", "bindingReset",
    "bindingResetAll", "outputList", "outputModeGet", "outputMode", "outputSet",
    "outputReset", "outputResetAll",
):
    if command not in binding_output_block:
        print(f"Missing Rust-authoritative binding/output routing: {command}", file=sys.stderr)
        raise SystemExit(1)

control_bar_block = SOURCE.read_text(encoding="utf-8").split(
    "private static func controlBar(arguments:", 1
)[1].split("private static func controlBarItem(arguments:", 1)[0]
for forbidden in ("loadStore(", "persistStore(", "mutateCustomization("):
    if forbidden in control_bar_block:
        print(f"Migrated control-bar collection routing still uses {forbidden}", file=sys.stderr)
        raise SystemExit(1)
for command in (
    "controlBarList", "controlBarSet", "controlBarAdd", "controlBarRemove",
    "controlBarMove", "controlBarReset",
):
    if command not in control_bar_block:
        print(f"Missing Rust-authoritative control-bar routing: {command}", file=sys.stderr)
        raise SystemExit(1)

control_bar_item_block = SOURCE.read_text(encoding="utf-8").split(
    "private static func controlBarItem(arguments:", 1
)[1].split("private static func parseControlBarItems", 1)[0]
for forbidden in ("loadStore(", "persistStore(", "mutateCustomization("):
    if forbidden in control_bar_item_block:
        print(f"Migrated control-bar item routing still uses {forbidden}", file=sys.stderr)
        raise SystemExit(1)
for command in ("controlBarItemShow", "controlBarItemSet", "controlBarItemReset"):
    if command not in control_bar_item_block:
        print(f"Missing Rust-authoritative control-bar item routing: {command}", file=sys.stderr)
        raise SystemExit(1)

customization_block = SOURCE.read_text(encoding="utf-8").split(
    "private static func customization(arguments:", 1
)[1].split("private static func setCustomization(", 1)[0]
layer_block = SOURCE.read_text(encoding="utf-8").split(
    "private static func layer(arguments:", 1
)[1].split("private static func group(arguments:", 1)[0]
for command in ("layerList", "layerMove", "layerForward", "layerBackward", "layerFront", "layerBack"):
    if command not in layer_block and "mutateLayerThroughAuthority" not in layer_block:
        print(f"Missing Rust-authoritative layer routing: {command}", file=sys.stderr)
        raise SystemExit(1)
for forbidden in ("loadStore(", "persistStore(", "mutateCustomization("):
    if forbidden in layer_block:
        print(f"Migrated layer routing still uses {forbidden}", file=sys.stderr)
        raise SystemExit(1)

style_block = SOURCE.read_text(encoding="utf-8").split(
    "private static func style(arguments:", 1
)[1].split("private static func layer(arguments:", 1)[0]
for command in ("styleList", "styleShow", "styleCreate", "styleRename", "styleApply", "styleDetach", "styleDelete"):
    if command not in style_block:
        print(f"Missing Rust-authoritative style routing: {command}", file=sys.stderr)
        raise SystemExit(1)

customization_set_block = SOURCE.read_text(encoding="utf-8").split(
    "private static func setCustomization(", 1
)[1].split("private static func orientation(arguments:", 1)[0]
for forbidden in ("loadStore(", "persistStore(", "mutateCustomization("):
    if forbidden in customization_set_block:
        print(f"Safe customization routing still uses {forbidden}", file=sys.stderr)
        raise SystemExit(1)
if "customizationSet" not in customization_set_block:
    print("Missing Rust-authoritative customization set routing", file=sys.stderr)
    raise SystemExit(1)

theme_block = SOURCE.read_text(encoding="utf-8").split(
    "private static func theme(arguments:", 1
)[1].split("private static func resolveThemePreset", 1)[0]
if ".themeApply(" not in theme_block or any(token in theme_block for token in ("requireExplicitUnmigratedProfileAccess", "mutateCustomization(", "loadStore(")):
    raise SystemExit("Theme application must use the typed authority transaction")

repair_block = SOURCE.read_text(encoding="utf-8").split(
    "private static func repairLayout(", 1
)[1].split("private static func parseLayoutRepairKind", 1)[0]
if "customizationFix" not in repair_block:
    print("Missing Rust-authoritative customization repair routing", file=sys.stderr)
    raise SystemExit(1)
if ".issue(target)" not in repair_block or "requireExplicitUnmigratedProfileAccess" in repair_block:
    print("Issue-specific repairs must use typed local authority", file=sys.stderr)
    raise SystemExit(1)

element_block = SOURCE.read_text(encoding="utf-8").split(
    "private static func element(arguments:", 1
)[1].split("private static func duplicateElements", 1)[0]
if "layerList" not in element_block or "loadStore(" in element_block:
    print("Element list must use the sanitized Rust projection", file=sys.stderr)
    raise SystemExit(1)

device_block = SOURCE.read_text(encoding="utf-8").split(
    "private static func device(arguments:", 1
)[1].split("private static func resolveDeviceFrame", 1)[0]
for forbidden in ("loadStore(", "persistStore(", "mutateCustomization(", "UserDefaults.standard"):
    if forbidden in device_block:
        print(f"Migrated device routing still uses {forbidden}", file=sys.stderr)
        raise SystemExit(1)
for command in ("deviceGet", "deviceSet"):
    if command not in device_block:
        print(f"Missing Rust-authoritative device routing: {command}", file=sys.stderr)
        raise SystemExit(1)


for function, command in (("addElement", ".elementAdd("), ("duplicateElements", ".elementDuplicate("), ("alignElements", ".elementAlign("), ("distributeElements", ".elementDistribute("), ("setElement", ".elementSet("), ("nudgeElement", ".elementNudge("), ("deleteElement", ".elementDelete("), ("resetElement", ".elementReset(")):
    block = source_text.split("private static func " + function + "(", 1)[1].split("private static func ", 1)[0]
    if command not in block or any(token in block for token in ("loadStore(", "persistStore(", "mutateCustomization(", "requireExplicitUnmigratedProfileAccess(")):
        raise SystemExit("Element commands must use typed authority transactions: " + function)


customization_block = source_text.split("private static func customization(arguments:", 1)[1].split("private static func setCustomization(", 1)[0]
reset_block = customization_block.split('case "reset":', 1)[1].split("default:", 1)[0]
if ".customizationReset(" not in reset_block or any(token in reset_block for token in ("requireExplicitUnmigratedProfileAccess", "mutateCustomization(", "loadStore(")):
    raise SystemExit("Customization reset must use the typed authority transaction")


for command in ("customizationExport", "customizationImport", "customizationReset"):
    if command not in customization_block or any(token in customization_block for token in ("loadStore(", "mutateCustomization(", "requireExplicitUnmigratedProfileAccess(")):
        raise SystemExit("Customization interchange and reset must use typed authority: " + command)


# Custom sizing is a retained design operation, independent of rich paint's
# remaining legacy route. It must never re-enter the desktop preference branch.
custom_size_block = source_text.split("private static func setCustomization(", 1)[1].split("var scalar", 1)[0]
if "requestsCustomDevice" in custom_size_block or "customDeviceOptions" in custom_size_block:
    raise SystemExit("Custom canvas dimensions must use authority transactions")
device_write = source_text.split("private static func setAuthoritativeDeviceFrame(", 1)[1].split("private static func resolveDeviceFrame(", 1)[0]
if "future bounded configuration transaction" in device_write or ".deviceSet(" not in device_write:
    raise SystemExit("Custom device selection must use the existing typed authority transaction")

print("CLI direct profile/design/element persistence allowlist passed.")

for method in ("createProfile", "attachApplicationToProfile", "detachApplicationFromProfile", "launchAttachedApplication", "repairLayout"):
    block = source_text.split(f"private static func {method}(",1)[1].split("private static func ",1)[0]
    for forbidden in ("loadStore(", "persistStore(", "mutateCustomization(", "requireExplicitUnmigratedProfileAccess("):
        if forbidden in block: raise SystemExit(f"{method} still uses {forbidden}")
