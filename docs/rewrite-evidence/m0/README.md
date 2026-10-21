# TypeScript rewrite execution record

Implementation started 2026-10-21 on `codex/typescript-rewrite`, in the dedicated worktree `/Users/codybontecou/.codex/worktrees/typescript-rewrite/Thumble`.

**M0/M1 acceptance is incomplete. M2 implementation has begun under the user-approved headless policy.** The [headless clarification](headless-execution-decision.md) supersedes the earlier restriction on starting implementation without local hardware; physical acceptance still gates release and retirement. No production entry point, user authority, phone installation, addon installation, or native input behavior was changed by this slice.

## Scope and reproducible baseline

The user confirmed **Windows at launch** and **live sync required at launch**. See [the exact decisions](scope-decisions.md) and the updated [plan](../../typescript-rewrite-plan.md). Acceptance therefore covers eight host/phone/game combinations, plus the independent live-sync gate. Cross-compilation cannot fill the existing Windows virtual-gamepad gap.

The worktree initially came from the remote default branch. It was moved to the current checkout's HEAD, `014fce90454e18311fa3d46be990c34a556c979b`, and populated with its tracked diff plus 487 nonignored untracked files. That input is committed separately as **`c7456e2`**. The original tracked binary diff SHA-256 was `0227220665c75f855d796b3b76a6109337c36a99f1911eb0ecbc7eb4434cf804`. The baseline commit preserves the plan and in-flight native/Android/optical work without treating it as release acceptance. Review rewrite changes from `c7456e2`, not from the remote default branch.

[baseline.json](baseline.json) binds the full baseline commit/tree, 203 retained fixture file hashes, compiler/OS versions, and the selected development lab package's file manifest and executable hashes. The existing package totals **210,017,404 bytes**, with nine files; this is a development-artifact byte count, not a shipping size budget. Full source-to-binary provenance is unproven because that package was built before this worktree existed.

## Retained-operation ledger

[retained-operations.json](retained-operations.json) maps 277 surface entries:

| Surface | Entries | Source |
| --- | ---: | --- |
| Swift CLI capabilities plus WoW setup/sync additions | 156 | existing capability ledger and actual dispatch |
| Native CLI command declarations | 43 | host/configuration/migration/portable receiver enums |
| Local stdio MCP tools | 41 | actual `#[tool]` functions |
| iOS behavior families | 17 | current client/editor and phone contracts |
| Android behavior families | 17 | current client/editor and Android parity inventory |
| Retired/excluded surface groups | 3 | desktop windows, hosted relay, accounts/marketplace |

Each entry names its current owner, planned replacement, milestone, and retention decision. Entries remain acceptance-pending. Grouping related mobile behaviors does not certify parity. Replacement paths describe ownership under the future workspace; they are not implemented packages. Native command variants and aliases can map to the same eventual operation.

The generator fails on an unmapped capability family. `--check` detects stale source hashes, added/removed tools or commands, and changed mapping data. The existing CLI parity verifier separately checks source dispatch coverage (154 operations, 29 roots, 264 spellings); the additional WoW routes are explicitly included here rather than silently omitted from the old ledger.

## Initial machine and compatibility matrix

[environment.json](environment.json) contains model/OS metadata without serial numbers or credentials. Pixel 7 / API 37 is connected. iPhone 17 Pro / iOS 27.0.1 and iPhone 13 Pro Max / iOS 26.3.1 are paired over the local network but their device tunnels are disconnected. Simulator results cannot substitute for physical-phone acceptance. A Windows machine, OS version, native virtual-gamepad implementation and driver distribution route remain unresolved.

[installed-clients.json](installed-clients.json) records public bundle metadata and addon file hashes only; no WTF/SavedVariables or character/account data was collected.

| Local provisional test target | Observed build | State |
| --- | --- | --- |
| Classic Era + ConsolePort | client 1.15.9.70003; ConsolePort 3.3.11 | installed; gameplay acceptance pending |
| Forever native interface | client 1.60.1.70245 | installed; gameplay acceptance pending |
| Windows Classic/Forever | not observed | target machine/builds required |

Retained build minima are iOS 17 (some configurations 18) and Android API 26. The new Expo/native combination has not yet been selected or certified against those minima. Supported OS coverage and performance thresholds must be frozen before rewrite comparison.

## Data-preservation and migration inventory

| Requirement | Existing oracle | Rewrite consequence |
| --- | --- | --- |
| Precise integer/decimal tokens and unknown nested metadata | raw JSON edits, strict scanners, owned snapshot/artifact fixtures | retain raw subtrees or a lossless representation; ordinary JS number serialization is insufficient |
| Stable IDs, revisions and durable receipts | owned-phone wire/edit fixtures and authority migration tests | string revisions/IDs, same-operation replay, reject changed payloads, correlate target profile |
| Eight appearance states | `GamepadAppearanceBank`, owned design/interchange fixtures | preserve light/dark × Base/LT/RT/LT+RT, with saved haptics and resources |
| Alternate orientations | legacy preparation/migration and owned layout adapter | retain exact original layouts; inventory legacy portrait before removing any path; owned editing currently uses canonical landscape |
| Bounded external data | legacy preparation and owned phone contracts | retain 8 MiB source/artifact bounds, 40 MiB package bound, 64 KiB phone intents / 128 controls, and duplicate-key checks; specify additional graph/asset/decode limits before new imports |
| Pending journals and selection | native/Android journal adapters | disconnected peers are unresolved; never replace host authority from display cache or redirect an old edit after A/B/A |
| Trust and local launch metadata | authority migration and profile-command contracts | discover through existing owner, private backups, reviewed source/destination hashes, resumable receipts; no credentials in exports |
| Optical integration | retained Lua, native capture, live fixtures and lab record | carry forward without assuming signed independent capture or physical delivery acceptance |

## Checks and honest evidence boundaries

- [tooling-tests.txt](tooling-tests.txt): focused Python regressions for stale inventory, scope omission, missing gameplay cells, invalid budgets, evidence tampering and mock-as-gameplay rejection.
- Existing CLI parity and retired desktop-source checks passed. They establish source consistency, not signed tools-only acceptance.
- [retained-wire-check.txt](retained-wire-check.txt): failed isolated launch using a copied signed lab executable; process exited before the receiver started. Root cause was not diagnosed, and no protocol result is inferred.
- [retained-wire-diagnostic.txt](retained-wire-diagnostic.txt): the same temporary no-input suite passed using the existing unprivileged debug host and retained native bridge. It exercises authenticated edits, explicit revisions/receipts, refusal, replay across restart, all eight appearance states and precise metadata. Binary hashes are in `environment.json`; these binaries were not rebuilt here.
- [M1 evidence](../m1/README.md): current development signing passes and independent GameController enumerates a neutral extended Thumble controller. No input was sent.

No Swift models/startup/codecs were changed, so the Swift stack suite was not rerun for this Python/documentation slice. It remains mandatory for the retained native changes listed in AGENTS.md. No handcrafted skin was authored, approved or published.

## Reproduce the new checks

From this worktree:

```sh
python3 scripts/rewrite/inventory.py --check
python3 -m unittest discover -s scripts/rewrite -p 'test_*.py'
python3 scripts/rewrite/check_gates.py
```

The default gate check exits **2**, meaning valid records with open M0/M1 acceptance gates. `python3 scripts/rewrite/check_gates.py --development` exits **0** when the hash-bound user-approved headless execution policy is valid; it does not mark physical acceptance complete. Exit 1 means malformed/inconsistent evidence; 0 means the recorded prerequisite checks are satisfied, not permission to publish. `launch-contract.json` requires both hosts, eight physical gameplay cells, separate finite numeric budgets and hash-bound reviewed evidence. It cannot infer gameplay from the present codec or development-signing results.

Capture public source/fixture/bundle metadata again with:

```sh
python3 scripts/rewrite/capture_baseline.py --baseline c7456e2 \
  --receiver-bundle '/absolute/path/to/the/existing/lab.app'
```

This reads only the selected package, never adjoining state/backups. It does not launch a receiver. See [launch-contract.json](launch-contract.json) and the generated [open-gate report](gates.json) for unresolved prerequisites. Keep those acceptance records pending while implementing M2 headlessly. Native gamepad/signing and physical beta evidence remain required before real-input/release acceptance.

## Subsequent M1 preparation

Added a bounded read-only native XInput witness and completed [Windows driver feasibility research](../../research/windows-native-gamepad-rewrite-2026-10-21.md). See the [M1 record](../m1/README.md#windows-consumer-witness-added-during-m1-preparation) for commands and verification boundaries. This advances the Windows evaluation tools; it does not close the native-driver or physical acceptance gates.
