#!/usr/bin/env python3
"""Deterministic M0 source inventory. No receiver, preferences or network access."""
from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
LEDGER = "docs/mcp/cli-capabilities-v1.json"
MCP = "Host/crates/thumble-mcp/src/server.rs"
NATIVE_CLI = ["Host/crates/thumble-host/src/cli.rs", "Host/crates/thumble-host/src/cli_configuration.rs",
              "Host/crates/thumble-host/src/migration_command.rs", "Host/crates/thumble-receiver/src/main.rs"]

# Replacement ownership is a plan, never an implementation/acceptance claim.
FAMILIES = {
    "generation": ("M4", "application/generation"),
    "profile": ("M4", "application/profiles"),
    "template": ("M4", "application/templates"),
    "theme": ("M4", "domain/appearance"),
    "skin": ("M4", "application/design-interchange"),
    "binding": ("M4", "domain/outputs"),
    "output": ("M4", "domain/outputs"),
    "customization": ("M4", "application/appearance"),
    "orientation": ("M4", "design/layout"),
    "device": ("M4", "design/geometry"),
    "control-bar": ("M4", "application/controls"),
    "element": ("M4", "application/controls"),
    "style": ("M4", "application/styles"),
    "layer": ("M4", "design/layers"),
    "group": ("M4", "application/groups"),
    "asset": ("M4", "application/assets"),
    "runtime": ("M3", "service/receiver-supervisor"),
    "app": ("M3", "application/setup"),
    "controller-design": ("M4", "application/design-workspace"),
    "wow": ("M5", "game-wow/snapshots"),
}
MOBILE = {
    "pairing": ("M3", "discovery/manual address, code, trust, revoke and reconnect"),
    "input": ("M3", "native multitouch, sticks, buttons, triggers and pointer"),
    "feedback": ("M3", "native press/bank feedback independent of JS rendering"),
    "practice": ("M3", "local feedback with no host output; release on entry"),
    "transport": ("M3", "authenticated UDP with reliable fallback and loss repair"),
    "lifecycle": ("M3", "background, lock, rotation, dispose, replacement and disconnect neutralize"),
    "accessibility": ("M4", "labels, actions, scaling, contrast and screen-reader inspection"),
    "profiles": ("M4", "selection/default, create/duplicate/delete and A/B/A identity"),
    "editor": ("M4", "bindings, geometry, groups, layers, all control kinds and undo/cancel"),
    "appearance": ("M4", "eight saved states, alternate layouts, styles, artwork and assets"),
    "interchange": ("M4", "bounded profile/design import/export and local review"),
    "haptics": ("M4", "native per-control haptics and persistent intensity preferences"),
    "journals": ("M4", "durable original operation IDs, conflict receipts and offline recovery"),
    "settings": ("M4", "appearance, keep-awake, onboarding and recoverable diagnostics"),
    "game-context": ("M5", "character/build, snapshot refresh and Base/LT/RT/LT+RT presentation"),
    "live-sync": ("M5", "required signed capture, freshness, replay rejection and phone delivery"),
    "migration": ("M6", "all pending journals admitted or recoverable; trust/re-pairing preserved"),
}


def sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def mcp_tools(source: str) -> list[str]:
    return sorted(set(re.findall(r"#\[tool\(.*?\)\]\s+pub async fn (\w+)", source, re.S)))


def tool_owner(name: str) -> tuple[str, str]:
    if name in {"setup_wow_consoleport", "wow_sync"}:
        return "M5", "game-wow/setup-and-live"
    if "migrate" in name or "rollback" in name:
        return "M6", "application/migration"
    if name in {"host_status", "accessibility_status", "pairing_code", "press_control", "release_all"}:
        return "M3", "service/receiver-supervisor"
    return "M4", "application/" + ("design-workspace" if "design" in name or "preview" in name or "render" in name else "configuration")


def inventory(root: Path = ROOT) -> dict:
    ledger = json.loads((root / LEDGER).read_text())
    operations = []
    for family in ledger["families"]:
        milestone, owner = FAMILIES[family["id"]]  # New families require an explicit decision.
        for item in family["operations"]:
            operations.append({
                "id": "cli:" + item["id"], "surface": "cli", "commands": item["cli"],
                "currentOwner": family["executor"], "currentStatus": item["status"],
                "capability": item.get("gate", family["gate"]), "replacement": owner,
                "milestone": milestone, "disposition": "retain", "acceptance": "pending",
            })
    # These additions are present in source but absent from the older capability ledger.
    for name in ["setup", "sync"]:
        if f'rest.first == "{name}"' not in (root / "Sources/CLI/ThumbleCLI.swift").read_text():
            raise ValueError(f"WoW {name} source dispatcher changed; review inventory")
        operations.append({"id": f"cli:wow.{name}", "surface": "cli", "commands": [f"wow {name}"],
                           "currentOwner": "rust-host", "replacement": "game-wow/setup-and-live",
                           "milestone": "M5", "disposition": "retain", "acceptance": "pending"})
    for name in mcp_tools((root / MCP).read_text()):
        milestone, owner = tool_owner(name)
        operations.append({"id": "mcp:" + name, "surface": "mcp", "currentOwner": "rust-host-and-native-bridge",
                           "replacement": owner, "milestone": milestone, "disposition": "retain", "acceptance": "pending"})
    for source_path in NATIVE_CLI:
        source = (root / source_path).read_text()
        for enum_name, body in re.findall(r"(?:pub )?enum (\w*Command) \{(.*?)^\}", source, re.S | re.M):
            for variant in re.findall(r"^    ([A-Z]\w*)\b", body, re.M):
                milestone = "M6" if "Migration" in enum_name or variant == "Migration" else "M4" if enum_name == "ConfigurationCommand" or variant in {"Configuration", "ImportConfiguration"} else "M3"
                operations.append({"id": f"native-cli:{source_path}:{enum_name}.{variant}",
                                   "surface": "native-cli", "currentOwner": source_path,
                                   "declaredVariant": f"{enum_name}.{variant}",
                                   "replacement": "application/migration" if milestone == "M6" else "application/configuration" if milestone == "M4" else "service/receiver-supervisor",
                                   "milestone": milestone, "disposition": "retire" if variant == "SharedBuild" else "retain", "acceptance": "pending"})
    for platform in ["ios", "android"]:
        for name, (milestone, behavior) in MOBILE.items():
            operations.append({"id": f"{platform}:{name}", "surface": platform,
                               "currentOwner": "Sources/iOS" if platform == "ios" else "Android/app",
                               "replacement": "mobile/native-input" if milestone == "M3" else "mobile/shared-application",
                               "behavior": behavior, "milestone": milestone,
                               "disposition": "retain", "acceptance": "pending"})
    for name, behavior in {
        "desktop-windows": "retire configuration windows after tools-only replacement acceptance",
        "hosted-relay": "retired; no replacement service",
        "accounts-marketplace": "excluded from rewrite launch",
    }.items():
        operations.append({"id": "retired:" + name, "surface": "legacy",
                           "disposition": "retire", "milestone": "M6", "behavior": behavior,
                           "replacement": "none", "acceptance": "pending"})
    return {"version": 1, "note": "Replacement destinations are planned, not implemented or certified.",
            "sourceHashes": {p: sha(root / p) for p in [LEDGER, MCP, "Sources/CLI/ThumbleCLI.swift", "Android/PARITY.md", "Sources/iOS/ControllerClient.swift", "Sources/iOS/IOSContentView.swift", "Android/app/src/main/java/com/thumble/android/ControllerClient.kt", "Android/app/src/main/java/com/thumble/android/ControllerSurface.kt", *NATIVE_CLI]},
            "operations": sorted(operations, key=lambda x: x["id"])}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    output = ROOT / "docs/rewrite-evidence/m0/retained-operations.json"
    expected = inventory()
    if args.check:
        if json.loads(output.read_text()) != expected:
            raise SystemExit("Rewrite inventory is stale; review source changes and regenerate it")
        print(f"Rewrite inventory verified: {len(expected['operations'])} mapped operations")
    else:
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(expected, indent=2) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
