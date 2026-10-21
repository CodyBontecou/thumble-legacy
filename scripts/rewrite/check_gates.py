#!/usr/bin/env python3
"""Report M0/M1 readiness; exit 2 for open gates and 1 for invalid evidence.

This is a consistency check, not a certifier. Only measured, reviewed acceptance
records can close a gate; package/codec success does not imply gameplay success.
"""
from __future__ import annotations

import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
BUDGETS = {"touchToReceiverP95Ms", "touchToVisibleGameP95Ms", "idleCPUPercent",
           "idleRSSBytes", "downloadBytes", "installedBytes", "coldStartupMs",
           "coldBuildSeconds", "warmBuildSeconds"}
BASELINE_ITEMS = {"retainedOperations", "deviceMatrix", "compatibilityBuilds", "scopeDecisions", "measuredBudgets"}
NATIVE_ITEMS = {"productionSigning", "windowsDriver", "independentConsumer", "gameEnumeration"}
EXERCISES = {"movement", "camera", "buttons", "bothTriggers", "modifierCombinations",
             "targetInteractMenu", "disconnectNeutralization", "reconnect", "physicalLatency"}


def review_record(root: Path, record: dict) -> list[str]:
    if record.get("status") == "pending":
        if not record.get("reason"):
            raise ValueError("Pending gate requires a reason")
        return [record["reason"]]
    if record.get("status") != "passed":
        raise ValueError("Gate status must be pending or passed")
    if not record.get("reviewedBy") or not record.get("command"):
        raise ValueError("Passed record needs reviewer and reproduction command/procedure")
    refs = record.get("evidence", [])
    if not refs:
        raise ValueError("Passed record has no evidence")
    for ref in refs:
        path = (root / ref["path"]).resolve()
        if not path.is_relative_to(root.resolve()) or not path.is_file():
            raise ValueError("Evidence must be a file inside this checkout")
        if hashlib.sha256(path.read_bytes()).hexdigest() != ref["sha256"]:
            raise ValueError("Evidence hash mismatch")
    return []


def open_gates(data: dict, root: Path = ROOT) -> list[str]:
    if data.get("version") != 1 or data.get("hostPlatforms") != ["macos", "windows"] or data.get("liveSync") != "required":
        raise ValueError("Launch scope must preserve the user's Windows and required-live-sync decisions")
    reasons = []
    for section, required in [("m0", BASELINE_ITEMS), ("native", NATIVE_ITEMS)]:
        if set(data[section]) != required:
            raise ValueError(f"Incomplete {section} gate set")
        for name, record in data[section].items():
            reasons.extend(f"{section}.{name}: {r}" for r in review_record(root, record))
    # Freeze separate numeric budgets on each host, with baseline provenance.
    if set(data["budgets"]) != {"macos", "windows"}:
        raise ValueError("Both host platforms need budget records")
    for host, budgets in data["budgets"].items():
        if set(budgets) != BUDGETS:
            raise ValueError("Incomplete performance budget set")
        for metric, value in budgets.items():
            if value is None:
                reasons.append(f"budgets.{host}.{metric}: measurement and threshold pending")
            elif isinstance(value, bool) or not isinstance(value, (float, int)) or not 0 < value < float('inf'):
                raise ValueError("Budget must be positive finite numeric data or null")
    expected = {f"{host}/{phone}/{game}" for host in ["macos", "windows"]
                for phone in ["ios", "android"] for game in ["classic-era-consoleport", "forever-native"]}
    if set(data["gameplay"]) != expected:
        raise ValueError("All eight host/phone/game combinations are required")
    for cell, record in data["gameplay"].items():
        if record.get("status") == "passed":
            if record.get("evidenceKind") != "physical-gameplay" or set(record.get("exercises", [])) != EXERCISES:
                raise ValueError("Gameplay pass requires complete physical exercises; mocks and codec evidence do not count")
            if not all(record.get(k) for k in ["hostBuild", "phoneBuild", "clientBuild", "packageSHA256", "recording"]):
                raise ValueError("Gameplay record needs exact builds/package hash/recording")
            recording = (root / record["recording"]).resolve()
            refs = {(root / ref["path"]).resolve() for ref in record.get("evidence", [])}
            if recording not in refs:
                raise ValueError("Gameplay recording must be hash-bound evidence")
        reasons.extend(f"gameplay.{cell}: {r}" for r in review_record(root, record))
    return reasons


def main() -> int:
    try:
        data = json.loads((ROOT / "docs/rewrite-evidence/m0/launch-contract.json").read_text())
        reasons = open_gates(data)
        print(json.dumps({"m2Ready": not reasons, "openGates": reasons,
                          "note": "Required live sync additionally gates M5/M6 release; no release approval is inferred."}, indent=2))
        return 2 if reasons else 0
    except (ValueError, KeyError, OSError, TypeError) as error:
        print(json.dumps({"invalidEvidence": str(error)}))
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
