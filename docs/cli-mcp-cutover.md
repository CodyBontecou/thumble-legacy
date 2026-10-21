# CLI/MCP cutover

The target product is local agent configuration through CLI/MCP, a phone controller/editor, and a headless receiver. Desktop UI retirement is part of feature removal, not an optional later redesign. This document defines the cutover gates; it does not claim they already pass.

## Current dependencies

| Owner | Required transition |
| --- | --- |
| `Sources/CLI/ThumbleCLI.swift` | Lifecycle/status/pairing inspection now use the headless receiver. Desktop launch/quit, onboarding replay, editor-window preferences and window-capture commands are retired; other legacy runtime/defaults/notification fallbacks still need cutover. Retain profile/design operations, local generation, preview capture/rendering, diagnostics, pairing, permissions guidance, and game-app launch targets. |
| `Host/crates/thumble-cli-bridge/src/main.rs` | It currently prefers the native configuration endpoint before Rust online/offline authority. Establish explicit authority ownership and migrate legacy data before removing that route. Preserve its bounded request and authority-lock guarantees. |
| `Sources/Mac/MacControllerServer.swift` and native output helpers | Transfer essential pairing, trust, synchronization, release barriers, and keyboard/pointer/gamepad behavior to the retained headless runtime before deleting GUI ownership. Compare against `thumble-host`, rather than assuming parity from its existence. |
| Shared design models/renderers and helpers compiled from `Sources/Mac` | Extract non-UI operations used by CLI/bridge, including WoW import/refresh and binding conversion. SwiftUI/AppKit used for headless image rendering does not itself require a desktop GUI product. |
| `Host/crates/thumble-mcp/src/controller_design.rs` | Retain agent design/preview/validation/transaction tools; remove package ownership and interactive desktop-editor dependencies. Audit CLI discovery, source formats, and tool schemas together. |
| `scripts/release/macos-host.sh` | Existing universal CLI/MCP/host/bridge packaging is a starting point. Its background-only signed helper bundle supports native HID authorization; retain the minimum required container and test final-artifact authorization. |
| `scripts/release/macos-cloudflare.sh`, desktop project targets, desktop website downloads | Retire desktop application releases after the headless install/update path passes. Update CI, downloads, setup text, and rollback guidance together. |

Legacy `UserDefaults`, credentials, active/default profile identities, game launch metadata, and pending phone edits are data migration inputs. Discover and back them up through their existing owners. Establish one authority with revision checks and locks; never seed competing catalogs by independently copying a display projection into Rust storage. Keep credential material outside profile exports and ordinary migration receipts.

## Ordered cutover gates

1. **Inventory retained operations.** Map CLI commands and MCP schemas to their real execution owners. For each GUI dependency, identify the headless replacement, data source, and acceptance check. GUI-window operations may retire; their underlying editing/inspection capabilities need tool equivalents.
2. **Prove headless authority and runtime.** Exercise live and stopped-receiver configuration changes, startup/restart, concurrent writes, pairing/trust/revocation, discovery, reconnection, synchronization, and output release. Migrate legacy configuration/trust with backup, revision checks, restart recovery, and rollback. Coordinate this with version-5 profile migration so neither migration can overwrite the other's newer authority.
3. **Complete agent tools.** Expose setup/status/diagnostics, receiver lifecycle, permission guidance, profile CRUD/import/export, appearance/assets/artwork/styles, geometry/layers/groups/alignment, validation, preview artifacts, reversible transactions, haptic configuration, and WoW import/refresh. Keep CLI and MCP behavior equivalent. Tools return useful errors and artifacts without opening a desktop editor.
4. **Remove GUI routing and surfaces.** Delete window/menu/onboarding/editor UI, GUI launch/screenshot commands, distributed-notification IPC, and stale GUI runtime-status reads after their retained replacements pass. Update command catalogs and MCP schemas atomically with removals. Extract shared helpers before dropping desktop source targets.
5. **Ship and verify tools-only installation.** Release CLI/MCP, the receiver, and required native helpers; remove desktop GUI installers/downloads. Run the acceptance below on a clean account with no Thumble desktop GUI installed or running.

## Acceptance evidence

- Install/update/uninstall and receiver start/stop/status/autostart work through tools. Permission or signing failures give actionable guidance; operating-system permission dialogs may still require the user.
- Pair a phone using the available code/QR/text workflow, approve/revoke trust, reconnect after restart, and inspect status without GUI IPC or desktop preferences as a runtime dependency.
- Exercise keyboard chords/sequences, buttons, analog sticks/triggers, pointer, multitouch, fallback transport, held-input expiry, and release-all. Compare latency under the recorded baseline conditions.
- Create and revise a richly styled profile through an agent, obtain native preview images, undo/cancel changes, and export/import with retained assets, all eight appearance states, groups, bindings, and haptic settings intact.
- Edit on both phones online and offline; restart, reconnect, resolve conflicts, and recover pending changes across migration. Verify supported physical-phone haptics separately from codec tests.
- Import and refresh WoW exports, retain actual output bindings and simultaneous trigger holds, and run live client acceptance where available.
- Run retained native CLI/bridge and phone builds, Rust/Android gates, MCP/CLI parity checks, and stack-safety checks. Replace desktop-specific stack coverage with equivalent headless-owner coverage before removing that build target; keep size/stack budgets unchanged.
- A clean release and its instructions contain no required desktop GUI, hosted relay, skin marketplace, calibration, or portrait controller setup. Native helper signing and game-launch `.app` paths are evaluated by their function rather than removed by filename matching.

Track implementation evidence in `feature-removal-progress.md`. Historical desktop verification remains historical; only tools-only acceptance proves this cutover complete.

Native element set/nudge/delete/reset now use typed live/offline authority transactions. Rich non-file set changes use the bounded MCP-compatible changes object; unsupported legacy flags fail explicitly. Preset deletion preserves identity by hiding, custom deletion reconciles all eight appearance states without persisting display paint. Remaining creation/duplication/arrangement, file-backed artwork import, owned portable interchange and phone reconciliation still gate desktop retirement.

Element creation/duplication/alignment/distribution have now joined the typed authority path. All eight primary element-edit handlers avoid legacy persistence, with stable retry IDs and source-preserving paint reconciliation. File-backed artwork workflows, owned portable interchange, phone reconciliation and migration activation still gate retirement.

The pure owned-profile artifact codec now preserves exact version-5 data and validates portable resources. It is not yet wired into native CLI/MCP transactions or phone transfer; those integration gates remain before desktop retirement.

Owned portable import/export is now available through native CLI profile commands and dedicated MCP tools for explicit schema-4 authorities, with revision checks, deterministic retry identities and persistent raw metadata. Explicit legacy conversion, phone reconciliation/format activation, file-backed artwork workflows and automatic migration still gate desktop retirement.

Version-4 file preparation is now exposed as a read-only host CLI/MCP step, returning an owned artifact and exact-source/layout reviews. It handles plain inline artwork and explicitly supplied package archives without authorizing authority migration or phone activation. See [legacy import preparation](legacy-import-preparation.md).

The owned-phone intent and private-draft staging backend now preserve native geometry/deletion/color behavior with revision and replay checks. Authenticated wire dispatch, matching acknowledgments and retained mobile journal adapters remain required before paired format-5 or automatic migration activation. See [the staged contract](owned-phone-edit-contract.md).

Authenticated owned-phone wire dispatch now commits scoped intents through the headless release/CAS path and returns correlated revision receipts without requiring local agent-write opt-in. Shipping mobile journal integration and profile-format activation remain gated. See [the wire contract](owned-phone-edit-contract.md).

The shared Apple owned-phone wire/journal adapter now preserves exact offline intents and recovery documents, with authenticated receipt reconciliation. It does not yet replace the iOS editor's legacy sync path; live owned snapshots, editor application, Android integration and migration activation remain required before desktop retirement.

Desktop UI command implementations and their planned MCP ledger entries are now removed. Old aliases fail with local receiver/preview guidance before reading configuration, writing preferences, opening/terminating apps or capturing a window. Setup help and bundled agent instructions use headless receiver setup and saved-layout preview artifacts. This does not remove the desktop target or remaining native authority fallback.
