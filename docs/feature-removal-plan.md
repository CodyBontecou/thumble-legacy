# Thumble feature removal plan

Remove skins and their catalog, hosted AI systems, portrait controller mode, thumb calibration, and the desktop application/UI. The target is local CLI/MCP configuration through users' agents, phone controller/editor UI, and a headless local receiver. Preserve fast controller/keypad input, direct appearance editing, design operations and previews, haptics, on-phone editing, and the WoW addon.

This plan covers implementation and migration work; it does not initiate removals or shut down deployed services. Execute in small, independently verified changes. The working tree already contains substantial ongoing work, including Android editing and synchronization; preserve it and establish the implementation baseline before starting.

## Product boundary

| Remove | Preserve |
| --- | --- |
| Skin package installation, selection, detachment, storage, synchronization, and updates | Profile-owned appearance, colors, gradients, effects, light/dark variants, text, icons, embedded assets, and reusable styles |
| Skin catalog, downloads, submissions, publishing, and skin-specific authoring pipeline | Agent-accessible canvas operations, layers, groups, alignment, snapping, undo/redo, device sizing, previews, and layout validation |
| Desktop windows, menu bar/Dock UI, onboarding screens, interactive desktop editor, and GUI IPC | Local CLI/MCP setup and receiver lifecycle, phone editing, headless rendering, and required native input/signing helpers |
| Cloud controller builder and remote Mac relay, including OAuth and tunnel infrastructure | Local CLI, local stdio MCP, local controller generation, local design sessions, and profile import/export |
| Hosted shared-build links, pickup, quarantine, and remote artifact adoption | Local backups, ordinary profile import, paired profile synchronization, and offline on-phone edits |
| Portrait controller runtime and separate portrait design variants | Landscape controller and editing, both landscape directions, and portrait-capable connection/onboarding/settings screens |
| Thumb reach tracing, calibration storage, scoring, and calibration suggestions | Manual positioning, touch-target checks, safe-area checks, overlap checks, and deliberate layout repair |

Android and Windows/Linux support remain in scope as existing platforms. Trackpad controls, app launching, general templates, accessibility, support links, and developer capture tools are unaffected by this removal plan.

“Hosted AI systems” means the hosted builder, shared-build pickup, remote relay, OAuth/tunnel infrastructure, and their deployment machinery. It does not remove a user's local agent, local CLI/MCP, local generation, or ordinary design tools. Removing skins means removing the package product and distribution system; it does not remove visual customization, imported artwork, reusable styles, or light/dark appearance.

## Starting point and remaining decisions

Use [implementation progress](feature-removal-progress.md) for evidence and [the ownership inventory](feature-removal-inventory.md) for code boundaries. Existing work has retired public catalog/distribution source, hosted gateway/relay source and phone pickup flows, receiver adoption uploads, and phone calibration. This is source-level progress, not evidence of a deployed shutdown or a completed migration.

Active skin libraries/runtime and portrait variants still require removal. Package-independent appearance, explicit version-5 readers, and lossless in-memory edits are preparation; version 5 is not activated in live authority or phone negotiation. No skin-backed or portrait user profile should be treated as converted merely because these helpers exist. Complete and verify the persistence, synchronization, and recovery path before enabling migration.

Record these decisions before activation: which previous releases can upgrade directly, how long original backups and legacy import decoding remain supported, and how users resolve missing packages/assets or divergent orientation layouts. An unresolved profile stays recoverable rather than being replaced with a default. Existing landscape and portrait variants can contain different controls; choosing landscape must not silently delete controls that occur only in another variant.

## Preservation requirements

- Keep keyboard chords/sequences, gamepad buttons, analog sticks/triggers, pointer input, multitouch, and stable element/input identities.
- Preserve authenticated pairing, discovery, reconnect, UDP/WebSocket behavior, generation checks, held-input expiry, release-all, and latency diagnostics.
- Keep appearance editable without installing a package. Imported artwork and assets must belong to the profile or its ordinary asset resources.
- Preserve global and per-control haptic settings, patterns, intensity, sharpness, and duration on supported devices.
- Keep on-phone move/resize/rotate/delete, grouped edits, undo/cancel, offline persistence, and trusted-host reconciliation.
- Keep Practice/Test mode and suppression of all output paths while testing or editing requires it.
- Preserve Thumble Link export/import/refresh and WoW Forever native controller outputs, including simultaneous trigger holds. Static class-themed skin content is removable; addon behavior and bindings are not.
- Every retained operation must work through CLI/MCP without a Thumble desktop GUI installed or running. Preserve local MCP contracts where the underlying feature remains, and move retained desktop-only behavior into tools or the headless receiver before deleting its owner.
- Preserve immutable/COW storage and the constrained-stack coverage required by AGENTS.md and docs/stack-safety.md.

## Phase 1 Establish the baseline and migration contract

1. Inventory references across Swift, Rust, Android, CLI/MCP schemas, protocol capabilities, project files, fixtures, workflows, release packaging, URL handlers, website, and docs. Classify each as removable, shared, or migration-only.
2. Capture representative profiles and native-rendered previews: plain and richly styled profiles, skin-backed profiles with user overrides, artwork/assets, light/dark differences, reusable styles, per-control haptics, portrait-only layouts, and profiles with both orientations. Include WoW bindings and pending phone edits.
3. Record current input behavior and latency using existing verification tools. Use the same device/network conditions for later comparison.
4. Define a new profile/configuration schema and explicit old-data decoding rules. Do not reuse removed wire identifiers or reinterpret an old field as a new feature.
5. Establish a migration fixture shared across Swift and Rust, with corresponding Android expectations. Verify repeat runs, interrupted writes, and rollback before depending on it.

Exit: a concrete reference map, representative fixtures, and a written migration contract. No user data has been discarded.

## Desktop retirement workstream

Follow [the CLI/MCP cutover](cli-mcp-cutover.md) alongside the phases below. Inventory GUI dependencies, establish one headless authority and migrate legacy configuration/trust safely, expose retained setup/design/runtime operations through CLI/MCP, then remove desktop UI/IPC and its release path. A signed background-only helper container may remain where required for native input authorization; it must not provide or require the desktop GUI. Retire interactive desktop design-session surfaces while retaining their design operations, validation, transactions, and preview artifacts.

Exit: a clean account can install, configure, pair, edit, run input, recover, and update using only local tools and the phone. The old GUI cannot be required as a fallback. Transitional desktop builds are regression evidence only, not the intended release product.

## Phase 2 Separate appearance from skin packages

The most important dependency is in Sources/Shared/ThumbleSkin.swift: it contains general visual roles and artwork types alongside package definitions. GamepadCustomization and ThumbleArtworkLayerView consume these types. Deleting all files named Skin would break retained design capabilities.

1. Extract the general appearance, artwork, visual-role, rendering, and asset types needed by direct customization into ordinary design modules. Preserve serialization compatibility through explicit decoding where necessary.
2. Audit shared renderer, SVG/vector, preview, and compiler dependencies. Retain machinery used by ordinary artwork or editor previews; remove package-only compilation and CSS skin authoring after migration no longer needs them.
3. Materialize each resolvable skin-backed profile into profile-owned appearance before detaching it. Preserve user overrides, both color schemes, relevant normal/pressed/disabled appearances, artwork, asset references, and haptics. Verify every referenced asset survives export and reload without the skin library.
4. Account explicitly for held-trigger action sets. The current WoW display foundation can resolve different artwork for Base, LT, RT, and LT+RT through skin rules. Move this behavior into direct profile appearance if needed to preserve existing designs; flattening only the Base state is insufficient. This does not add live spell synchronization.
5. If the referenced package or required asset is missing, preserve the original data and report the unresolved migration. Do not silently substitute a default appearance and mark the conversion successful.
6. Use atomic, idempotent migration with an original-data backup and migration version. Never change bindings, control identities, active/default profile identities, or output mode as a side effect.

Exit: migrated fixtures render correctly and remain editable after package lookup is disabled. Profile export/import retains the appearance and assets supported by the existing interchange contract; any gap in that contract is addressed before removing the only copy of an asset.

## Phase 3 Remove skin product surfaces and distribution

1. Remove MacSkinLibraryView, IOSSkinLibraryView, Android skin-library/import/export surfaces, skin menu entries, external skin file handlers, and CLI skin commands.
2. Remove skin catalog pages, catalog scripts/data, downloadable packages, submission guidance, and catalog build/verification jobs. Keep the main website and app release download endpoints.
3. Remove runtime package transfer, selection/removal mutations, stores, compatibility resolution, and skin-specific host/bridge commands after Phase 2 conversion works on each retained platform.
4. Keep a bounded legacy decoder/converter only as long as required by the supported upgrade path. Remove active skin capabilities; explicitly reject unsupported legacy mutations without changing profiles. Define and test behavior with older paired clients.
5. Remove skin-only fixtures, authoring skills, review workflows, examples, previews, packaging code, and build dependencies after checking for shared design-tool consumers. Update obsolete skin instructions in AGENTS.md as part of implementation.

Exit: no supported UI, CLI, MCP tool, advertised capability, website link, or release package offers skin functionality. Migrated appearance remains usable offline with no installed packages.

## Phase 4 Remove hosted AI systems

1. Remove cloud-builder and relay entry points from apps, CLI, MCP arguments, plugins, website, and docs. Keep local stdio MCP, local profile generation, preview/editor tools, and local profile transactions.
2. Remove iOS IOSBuilderArtifactPickup and IOSPendingBuilderArtifactStore and equivalent Android shared-build screens, activities, pending stores, URL handling, and cloud-only Practice Preview adoption state.
3. Separate ordinary profile artifact codecs/import/export from hosted pickup/adoption transport. Keep any paired adoption transaction still required by a retained local import workflow; remove only consumers and messages that are exclusive to hosted sharing.
4. Remove thumble-gateway and thumble-tunnel. Audit thumble-builder before deleting it: move any retained local templates/session logic to an appropriate local module first. Likewise, preserve local design-session and MCP editor assets despite builder-related names.
5. Remove relay mode from thumble-mcp, OAuth/link/approval code, tunnel routing, relay LaunchAgent installers, hosted deployment files, and hosted-only tests and capability ledgers. Update Host/Cargo.toml, lockfile, scripts/verify-rust-host.sh, CI, and release packaging together.
6. Inventory deployed services and resources separately from source removal. Prepare an exact retirement manifest: services, routes/domains, jobs, OAuth clients/tokens, storage/retention requirements, backups, and shared dependencies. Deploy the retirement behavior, revoke access, stop services, and verify that the core app and main website need no cloud connection. Execute deployment/destruction only as a separately authorized operational step.
7. Preserve already adopted local profiles. Offer a local export path for recoverable pending builds before clearing hosted-only caches; do not silently discard user work.

Exit: controller use, direct editing, local CLI/MCP, and local import/export work with the hosted infrastructure unavailable. No release installs a relay background process or advertises a hosted connection.

## Phase 5 Make controller mode landscape only

1. Make the live controller and phone layout editor landscape-only on iOS and Android. Keep both landscape rotations. Connection, onboarding, and settings can remain portrait-capable.
2. Simplify CLI/MCP and phone editing to one landscape layout per profile. Remove rotation preferences, portrait canvas selection, automatic variant switching, orientation copy/arrange/compare workflows, and portrait template generation. Keep device dimensions and landscape canvas sizing.
3. Migrate to one canonical layout. Prefer an existing explicit landscape variant; otherwise preserve a landscape base layout. Convert portrait-only layouts through a deterministic arrangement that preserves all control IDs, outputs, style, and group membership. Keep the original layout in the migration backup and flag layouts needing manual review.
4. Resolve pending phone edits against their original orientation before eliminating variant storage. Preserve unmatched edits for recovery; do not apply portrait coordinates directly to a landscape layout.
5. Preserve input release before geometry transitions. A geometry request can be rejected or interrupted; suppress input while the controller cannot present its supported layout and provide a recoverable rotation state.
6. Update profile schemas, bridge operations, templates, CLI/MCP contracts, device metrics, runtime routing, and fixtures consistently. Legacy portrait input/import behavior must be explicit and must never silently drop controls.

Exit: retained controller/editing flows offer one landscape layout, old profiles migrate without binding loss, and rotation/background/foreground transitions produce no stuck inputs.

## Phase 6 Remove thumb calibration

1. Remove ThumbPlacementCalibrationView, the calibration entry points and overlays, reach tracing/scoring/suggestion types in IOSLocalKeypadUX, and corresponding Android components.
2. Remove calibration-specific persistence, settings, tests, and protocol fields if any. Existing accepted calibration adjustments are ordinary saved layout edits and must remain.
3. Preserve general layout-quality checks, repair commands, touch routing, manual edits, and Practice Mode. Remove calibration-only state transitions without weakening output suppression during editing.

Exit: no calibration prompt or stored reach model remains, and manual editing plus layout validation still work.

## Phase 7 Verify and finish

Run checks at each relevant phase, then perform final end-to-end acceptance:

| Area | Required evidence |
| --- | --- |
| Migration | Original backup, atomic/repeatable conversion, missing-package recovery, old export import, and Swift/Rust/Android fixture agreement |
| Appearance and design | Native preview comparison; edit/save/reload/export/import of styles, assets, text, layers, groups, effects, and light/dark appearance; held-trigger states where retained |
| Phone editing | Online/offline changes, undo/cancel, process restart, reconnect conflicts, and retained edits after migration |
| Haptics | Device-level intensity and per-control patterns/settings on physical supported phones |
| Input and latency | Buttons, multitouch, sticks, triggers, pointer, fallback transport, disconnect recovery, release-all, and comparison with the recorded baseline |
| WoW | Existing addon exporter/importer checks, template outputs and trigger combinations; live client acceptance where the required client and signing are available |
| Surface parity | Retained operations work without the desktop GUI; local MCP schemas match surviving CLI commands; removed operations fail clearly without mutation |
| Build and packaging | CLI/native bridge, headless receiver, phone targets, Rust workspace, Android checks, website links, CI, release scripts, and clean installation without the desktop GUI or hosted services |
| Stack safety | Run ./scripts/verify-stack-safety.sh for shared model/codec/sync/startup changes; preserve meaningful constrained-stack coverage and inline-size budgets |

Use scripts/verify-thumble-link.py and scripts/verify-thumble-link-cli.py for the addon boundary. Update scripts/verify-rust-host.sh to verify the surviving workspace, rather than disabling the gate because hosted checks were removed. Retain migration regression tests even after obsolete feature tests are deleted.

Update README, CLI help, local agent instructions, platform parity docs, screenshots, marketing, URL/file associations, and capability catalogs to describe the final feature set. Audit remaining references: permit only intentional migration handling and clearly marked historical records. Do not claim live WoW, physical haptic, or latency acceptance from unit tests alone.

## Suggested change sequence

1. Baseline fixtures and migration contract.
2. General appearance extraction and skin materialization.
3. Skin UI, catalog, and package-runtime removal.
4. Hosted UI, relay/cloud source, and packaging removal; separate deployed-service retirement.
5. Landscape-only profile migration and controller/editor changes.
6. Calibration removal.
7. Complete headless CLI/MCP ownership and tools-only installation, then remove desktop UI, GUI IPC, and desktop release targets.
8. Remaining compatibility cleanup, docs, and final acceptance.

Phases 4 and 6 are relatively independent after the baseline; the ordering above keeps review focused. Phase 2 must precede skin-runtime deletion, and migration fixtures must precede orientation-model deletion. Keep original backups until acceptance is complete and the supported recovery window is defined.

## Remaining reviewable batches

These batches refine the phase ordering around the current repository state. Each should leave the retained app usable and carry its own relevant verification; do not combine migration activation with wholesale deletion of its fallback readers.

| Batch | Deliverable | Gate before proceeding |
| --- | --- | --- |
| 1. Finish the migration transaction | Lossless authoritative version-5 writes; complete configuration, resource, and pending-edit bounds; backup, revision check, atomic commit, receipt, restart recovery, and rollback | Cross-platform fixtures agree; interrupted/concurrent/repeated conversions preserve source data and unknown metadata; missing assets and variant-only controls enter recovery |
| 2. Integrate retained peers | Authority snapshots, local import/export, CLI/MCP edits, and both phone editors use the owned appearance and single-layout contract | Actual transfer fits bounds; unsupported older peers receive update-required errors; offline edits reconcile in their original coordinates and remain recoverable |
| 3. Activate and detach packages | Migrate resolvable profiles, preserve eight light/dark × Base/LT/RT/LT+RT appearances, and remove active skin selection/transfer/library commands and UI | Designs render and remain editable after the package library is unavailable; all IDs, outputs, assets, groups, and haptics survive save/export/reload |
| 4. Retire portrait controller/editing | One landscape layout, both landscape directions, deterministic portrait conversion with review, and removal of variant controls/schema paths | No variant-only controls disappear; phone offline changes survive; rotation, cancellation, reconnect, and backgrounding release held inputs |
| 5. Finish source and release cleanup | Complete the CLI/MCP cutover; remove desktop UI/IPC/release entries, remaining package-only tooling, obsolete hosted/calibration references, associations, capabilities, and dependencies; update documentation | CLI/MCP work with no desktop GUI installed or running; clean installation works offline from hosted services; preserved design tools, phone editing, and WoW tests pass |
| 6. Accept the product and retire deployments | Native preview review, supported-phone haptics, same-condition latency comparison, WoW client checks, then separately authorized service retirement | Record actual device/client evidence and unresolved limits; deployed-resource inventory and backups precede shutdown |

Calibration needs a regression and reference audit in the final cleanup, rather than a second implementation of its already recorded removal. Hosted source retirement likewise does not authorize deleting deployed resources. Keep the [operational retirement checklist](hosted-service-retirement.md) separate from source changes.
