# Thumble TypeScript rewrite plan

Status: [draft PR #4](https://github.com/CodyBontecou/Thumble/pull/4) remains open for source review. Public head `d65e499` enables the Unix captured-owner editor/policy seam and native app session APIs. The actual service, TLS/policy host, child-owned SQLite and both Swift/Kotlin SDKs pass together, including queued CLI contention, slow-write retention, exact receipts and four revoke acknowledgements after completion. Root gates pass460 TypeScript tests,189 Rust unit plus33 integration tests, standalone packaging and55 retained stack cases. CI for the new head is pending; separate iOS TLS simulator and Windows package intermittency remain unresolved. Visible editor QA is ongoing. macOS plus iOS/Android remains the launch priority. Physical gameplay/live sync, broader editing/recovery, notices/signing/migration and clean-install/store gates remain open; no launch milestone is complete. See [end-to-end editor evidence](rewrite-evidence/m4/catalog-operations/editor-end-to-end-registration-2026-10-23.md).

## Outcome and scope

Build Thumble around TypeScript, Bun, and one React Native application for iOS and Android. Launch with two fully tested game experiences: **WoW Classic Era with ConsolePort**, and **WoW Forever with its native gamepad interface**. CLI/MCP and the phone controller/editor are the product interfaces. A headless local receiver provides native input without a desktop GUI or cloud account.

This plan authorizes implementation sequencing, not a claim of compatibility or permission to publish a release. The existing [CLI/MCP cutover contract](cli-mcp-cutover.md) remains the authority for retiring desktop dependencies. Preserve the current product until replacement behavior and migration pass their gates.

| Dimension | Launch baseline | Expansion |
| --- | --- | --- |
| Phone platforms | iOS and Android, equal acceptance requirements | Tablets and additional form factors after phone acceptance |
| Computer platform | macOS headless receiver; user reprioritized 2026-10-22 | Windows and Linux follow while phone apps are in review; each needs native input, signing and gameplay acceptance |
| Classic | Classic Era + tested ConsolePort release | Other Classic branches require explicit adapters and gameplay evidence |
| Forever | Native gamepad interface on a pinned supported build | ConsolePort on Forever is a separate certified combination |
| Configuration | Local CLI, stdio MCP, phone editor | No required hosted configuration service |
| Gameplay | Native sticks, triggers, buttons, keyboard/pointer where configured | Additional game integrations after launch gates |
| Integration | Setup, character/build context, snapshot refresh, trigger-bank presentation, and optical live sync; user confirmed 2026-10-21 | Live sync must pass its independent acceptance gate before launch |

The user superseded the earlier Windows-at-launch decision on 2026-10-22: prioritize the local macOS receiver and both phone apps, test with the connected iPhone and Pixel, and prepare store submissions. Continue Windows/Linux implementation after the macOS/phone review candidate is ready. Their driver, installation/removal, signing and gameplay gates remain required before claiming support. Optical live sync remains required for the initial release.

Exclude desktop configuration windows, hosted relays, accounts, marketplaces, and a general plugin framework from the rewrite's launch scope. Preserve general-purpose controller editing and existing user data. Do not rebuild every historical implementation detail merely to reproduce the old module structure.

## Decisions and boundaries

| Decision | Rationale and boundary |
| --- | --- |
| TypeScript with strict checking | Share domain behavior, application operations, game adapters, validation, and mobile editor logic |
| Bun workspaces, runtime, bundler, test runner | One host development toolchain; pin a tested version and lockfile |
| React Native with Expo development builds | Share mobile UI and product behavior while supporting custom Swift/Kotlin modules; Expo Go is not the acceptance environment |
| Retained minimal native receiver | Reuse proven input, HID, signing, sequencing, and release behavior; remove configuration policy from it incrementally |
| Native phone input modules | Track continuous touches, send input, cancel holds, and invoke haptics without depending on React rendering |
| SQLite local authority | Transactional revisions, durable receipts, and migration recovery; one writer owns canonical configuration |
| Language-neutral wire schemas | Generate TypeScript/Swift/Kotlin types and validate all external data at runtime |
| Versioned client adapters | Separate physical input, game assignments, and presentation; exact client/addon compatibility is recorded |

Retaining a small Rust receiver is acceptable and is the initial recommendation. Its dependency graph should stop at transport/input/platform primitives. TypeScript replaces the unified product layer, not the OS input backend. Rewriting that backend in Swift or another language is a later decision supported by measured maintenance or packaging benefit.

Bun compilation embeds its runtime. Faster iteration and smaller source builds are goals; smaller installed artifacts are hypotheses to measure. Native mobile changes still require Xcode/Gradle builds. Keep pure domain code free of Bun, filesystem, React, and OS imports.

## Process architecture

```text
CLI ──────┐
MCP ──────┼── local IPC ── Bun configuration service ── SQLite + asset store
          │                        │
Phone editor ── authenticated control channel
                                   │
                       revisioned runtime configuration
                                   ↓
Phone native input engine ═══ native receiver ═══ OS input ═══ WoW
                                   ↑
                     bounded native capture observations
                                   │
                        TypeScript WoW adapters
```

### Ownership

- The Bun service owns saved profiles, active/default selection, trust policy, edits, game integration state, migration, and configuration receipts.
- The native receiver owns the published virtual device, active input sessions, sequence/replay state, held outputs, expiry, and emergency release. It has no independent editable profile catalog.
- The phone owns transient touches, local credentials, a source cache, and a durable edit journal. Cached projections never replace host authority wholesale.
- The addon owns only supported in-game configuration/export behavior. WoW owns protected actions, spell execution, combat restrictions, and actual action assignments.

Use user-restricted local IPC between host processes. Give the receiver a validated immutable runtime configuration and an activation generation. Acknowledge activation before reporting a profile switch complete; neutralize the previous generation and reject its delayed packets. Receiver restart begins neutral and requires a new session.

The service grants bounded receiver sessions over authenticated local IPC. The receiver verifies incoming input itself, expires session leases independently, and rejects stale generations. Define behavior when trust is revoked or the service disappears: release immediately on explicit revocation; expire on a short configured lease after loss of the owner. Configuration-service failure must never leave an indefinite hold.

Keep the receiver alive independently of CLI/MCP process lifetimes. A user background-service supervisor manages startup/restart; CLI invocation does not create a competing owner. Status reports configuration-service health, receiver health, permissions, device recognition, and phone state separately.

### Mobile implementation

React Native owns pairing screens, profile selection, editor tools, game setup, diagnostics, and controller presentation. Start with a standard React Native renderer and explicit geometry; preserve a renderer interface for artwork-heavy controls without committing to a second rendering framework before measurement.

Swift/Kotlin input modules consume a validated control/hit-region map and emit semantic input independent of React rerenders. They own touch identities, multitouch arbitration, stick sampling, trigger holds, cancellation, and transport scheduling. Feed local press/bank feedback directly from the same input state. React state can observe it for inspection; it is not the per-sample event queue.

Native modules must release on touch cancellation, backgrounding, screen lock, practice-mode entry, rotation, profile replacement, disconnect, and module disposal. Any optional reconnection starts neutral and never replays prior touches. Practice mode produces local feedback without host output.

Native networking modules handle input and platform discovery. TypeScript controls the configuration protocol through a bounded interface. Secure storage uses platform credential facilities. Discovery must support manual address/code fallback; preserve Apple peer-to-peer behavior where supported without claiming equivalent Android transport.

## Proposed repository layout

Introduce the workspace under `Rewrite/` during coexistence. The directories below are proposed; they do not exist as part of this documentation change.

```text
Rewrite/
  apps/
    cli/                      command parsing and output
    mcp/                      stdio MCP adapter and tool catalog
    service/                  Bun composition, IPC, lifecycle, persistence
    mobile/                   React Native iOS/Android application
  packages/
    contracts/                wire schemas, codecs, generated bindings
    domain/                   canonical documents, validation, merge rules
    application/              use cases, transactions, capability checks
    client/                   typed local/control-channel client
    storage/                  SQLite migrations, repositories, receipts
    design/                   geometry, appearance, preview scene description
    game-wow/                 client adapters, import and refresh policy
    fixtures/                 shared contract and behavioral fixtures
  native/
    mobile-input/             Swift/Kotlin input, transport, haptics
    receiver/                 extracted native input process and platform code
    preview/                  retained native render/capture helpers
  tooling/                    schema generation, packaging, verification
```

Dependency direction: adapters depend on application operations; application depends on domain/contracts and abstract ports; platform/storage implementations satisfy those ports. Mobile imports portable domain/design/client code, not host application or storage packages. Game adapters return typed observations/changes and do not write the database directly.

Use one lockfile and native build caches keyed to native inputs. Separate TypeScript-only CI from simulator/device builds. Move the workspace to the repository root only after cutover; avoid two long-lived production implementations.

## Canonical documents and transactions

### Model

Define a versioned controller document containing stable profile/control IDs, semantic outputs, layout, appearance variants, asset references, haptics, and game-context metadata. Model these independently:

1. Physical output: left-stick axes, south button, held trigger, keyboard chord, pointer movement.
2. Game assignment: character, binding command, action slot, ability identity, contextual page/form.
3. Presentation: icon, legend, geometry, appearance, accessible name, trigger-bank state.

Preserve stable IDs through refresh, character A/B/A changes, rank changes, and imports. Keep Base/LT/RT/LT+RT explicit. Preserve the existing eight saved appearance states and alternate layout data during migration; inventory product-supported orientations before removing any editor path. User labels, output overrides, moved controls, and deletion history survive game refresh.

Store assets by content hash outside ordinary document rows; imports stage bytes and commit references atomically. Exports include a bounded resource manifest and no credentials. Define graph/depth, byte, control-count, asset-size, and image-decoding limits before accepting external documents.

Inventory the existing raw-number and opaque-metadata preservation requirements. Use a lossless JSON representation or retain untouched raw subtrees where ordinary JavaScript numbers/serialization would alter imported data. UUIDs and revisions travel as strings where numeric precision could be lost. Canonical hashing needs an explicit versioned byte representation, never incidental `JSON.stringify` output.

### Command contract

Every write carries an operation ID, expected revision, scoped typed changes, and caller capability. The application layer validates, produces a diff, and commits or returns a typed conflict. A committed receipt identifies operation, resulting revision, content hash, and affected entities. Retrying the same operation returns the original receipt; changing its payload under the same ID is rejected.

Use SQLite transactions for authority writes and receipt creation. Coordinate file assets through staging and a durable recovery record; garbage collection follows reachability after commit. The active service is the sole writer. When stopped, CLI edits start a short-lived service under the same authority lease rather than bypassing transaction code.

Phone journals persist original operation IDs and payloads before upload. Only a matching successful receipt retires an entry. Disjoint edits may rebase through documented field rules; overlapping edits require an explicit conflict resolution. Selection changes never redirect an old edit to the newly selected profile. Begin with scoped transactions and three-way merges rather than a general CRDT.

CLI and MCP invoke the same operations. CLI machine output uses versioned JSON and typed errors; MCP exposes bounded schemas, structured receipts, and preview artifacts. Preserve explicit input opt-in, keep emergency release available, and separate configuration writes, addon installation, capture, and input capabilities. MCP stdout contains protocol messages only.

## Transport and security

Keep configuration synchronization separate from the input fast path. Prefer reliable authenticated control traffic and authenticated UDP state updates with reliable fallback. First document the existing wire protocol and retain its golden vectors; change the protocol only with a versioned negotiation/migration story.

Specify pairing authentication, credential provisioning, encryption, nonce/replay rules, session identity, packet limits, and revocation in a protocol ADR before enabling real input. Use established cryptographic implementations and a reviewed pairing construction; a six-digit display code alone is not a transport security design. Never transmit long-lived credentials in logs, preview artifacts, or profile exports.

Input messages include session/generation identity, monotonic sequence, and bounded semantic state. Define how snapshots, edges, duplicates, reordering, packet loss, fallback switching, and held-state expiry interact. A newer full-state sample must repair a lost release. Rate-limit pairing attempts and malformed traffic. Listen only on the intended local network surfaces; no internet relay is required.

Negotiate document, control, input, and capability versions separately. Old peers either use a tested compatibility adapter or receive an actionable upgrade requirement. Unknown mandatory semantics fail before activation. Pairing/discovery compatibility must not silently select the wrong local authority during coexistence.

## WoW launch integrations

### Compatibility records

Identify a target by client flavor, interface/build range, integration mode, and addon version/hash where relevant. Classic Era + ConsolePort and Forever native gamepad get independent fixtures, setup instructions, import logic, and physical acceptance records. A tested Era build does not certify other Classic branches.

| Operation | Classic Era + ConsolePort | Forever native gamepad |
| --- | --- | --- |
| Installation | Detect exact client; plan/backup/install compatible ConsolePort and Thumble Link | Detect exact client; install Thumble Link when using snapshots |
| Input | Native gamepad consumed through WoW/ConsolePort | Native gamepad consumed through the client interface |
| Setup | Preserve existing bindings; explicit reviewed preset replacement | Follow the selected client's native binding/action rules |
| Assignment import | Resolve ConsolePort's actual commands, slots and modifier layers | Separate adapter for native action pages and supported exports |
| Presentation | Reflect observed assignments; preserve user edits | Reflect supported native assignments without assuming keyboard parity |
| Unsupported context | Explain missing/stale data and allow manual setup | Same requirement; never invent an ability mapping |

Setup returns a read-only plan with source/destination hashes and conflicts. Apply rechecks the exact plan, requires the game closed for filesystem changes, backs up affected addon/settings files, and records durable recovery phases. Preserve unrelated addons and bindings. Existing setup has ordinary rollback but documented interruption-recovery limitations; close that gap in the rewrite.

Thumble Link remains a bounded Lua companion. Keep keyboard snapshots distinct from native-gamepad assignments. Respect combat restrictions, protected/restricted data, and supported game APIs. Thumble emits physical state; it does not schedule spell sequences or claim authority over protected gameplay actions.

### Character, build, and live state

Manual character/build selection and explicit snapshot import/refresh are launch requirements. Context includes flavor, character identity, class, and supported spec/build identity. Automatic observation can suggest/select context only under a documented policy that respects manual selection, keeps stable profiles, and handles A/B/A switches without losing edits.

The current tree includes optical live-panel, capture, onboarding, and character-sync work. Preserve its contracts and fixtures as migration inputs; do not describe it as absent or certified. The addon does not provide an arbitrary network socket. Retain explicit snapshot import as the baseline and evaluate optical observation as a separate capability.

To ship optical live sync, prove capture permission from the independently launched signed helper, supported window/scaling behavior, frame identity and freshness, frozen/replayed-frame rejection, and physical phone delivery. Treat captured pixels as untrusted observations. Loss of observation marks icons/context stale without interrupting physical input. State freshness is visible; cooldowns, ranks, forms, pets, and contextual pages are promised only where tested.

Live sync is mandatory launch scope by the user’s M0 decision on 2026-10-21. Its independent optical acceptance gate is a launch blocker. Manual import/refresh must remain usable when capture is unavailable or stale.

## Reuse and migration inventory

Existing documents and source describe a moving implementation. At M0 freeze a commit plus selected worktree diff hashes for reproducible evidence; do not treat uncommitted lab changes as a release baseline.

| Existing area | Rewrite treatment |
| --- | --- |
| `Host/crates/thumble-mcp`, CLI/bridge operations | Inventory retained behaviors; port product policy into shared TypeScript application operations |
| Native receiver and platform output | Extract minimal implementation; retain fixture-based conformance and final-artifact signing checks |
| `Sources/Shared` and Android codecs/journals | Use as behavioral oracles; generate new bindings while preserving lossless and pending-edit behavior |
| Native appearance/rendering helpers | Retain headless previews during migration; compare the new phone renderer against real phone output |
| Thumble Link and WoW setup/live code | Port orchestration, preserve bounded Lua/native pieces and existing edit-preservation semantics |
| Host fixtures and existing verification scripts | Reuse exact vectors where valid; make deviations explicit contract changes |
| Credentials, identities, active/default selection | Discover through existing owners; backup, migrate, verify, and recover under one authority lease |
| Desktop GUI/IPC, Swift configuration transformer | Retire only after their retained operations have replacements and signed clean-install acceptance |

Follow [authority migration](authority-migration.md) and [phone edit admission](owned-phone-edit-contract.md). Preflight every known peer and pending journal. A disconnected peer may contain edits; record it as unresolved rather than assuming its journal is empty. Preserve its recovery route or block destructive migration.

Migration uses a reviewed exact-source plan, private durable backups, conversion report, destination verification, and resumable receipts. Never run both authorities against one state directory. Reuse identities/credentials only through a tested compatible conversion; otherwise provide explicit re-pairing and preserve journals. Rollback after newer writes requires reconciliation/export, not restoring an old database over them.

Use isolated state, discovery identifiers/metadata, ports, and explicit client selection for rewrite development. Installation and upgrade must detect existing owners. The final receiver keeps the exact authorized native identity and required bundle container; a background `.app` can remain necessary without a desktop GUI.

## Milestones and completion gates

All milestones below start pending. Gates require stored evidence, not checked boxes inferred from implementation. Work in small vertical changes; the dependencies below describe sequencing, not authorization to launch parallel agents.

### Headless implementation policy (user decision, 2026-10-21)

The user superseded the entirely-headless execution policy on 2026-10-22. Use the local Mac and connected iPhone and Pixel through Argent for real pairing, input, usability and recovery checks. Keep the macOS receiver headless and independent of the desktop GUI. Use GitHub-hosted runners for deferred Windows build/unit/contract/loopback/package tests; eventual beta testers provide Windows gameplay evidence. M2 and subsequent engineering may proceed while M0/M1 physical evidence remains pending. Those outstanding acceptance gates still block release certification and retirement of existing owners; headless CI never substitutes for gameplay, production driver/signing, physical latency, haptics or optical capture acceptance. See [the recorded user clarification](rewrite-evidence/m0/headless-execution-decision.md).

### M0 — Baseline and launch contract

Deliver: retained-feature ledger; fixed baseline artifacts; supported phone/OS/device matrix; exact Classic/Forever/addon combinations; decisions on macOS-first and live-sync scope; latency/build/size measurements.

Complete when every retained CLI/MCP/phone operation maps to a replacement, explicit deferral, or retirement; both phone platforms and both game experiences have named acceptance devices/builds; open product decisions have recorded outcomes.

### M1 — Prove native gameplay and packaging risk

Depends on M0. Use the current receiver before rebuilding broad UI. Verify the exact release identity, entitlement/provisioning scope, native device creation, independent GameController semantics, and actual game enumeration.

Complete when physical iPhone and Android input passes movement, camera, buttons, both triggers, modifier combinations, target/interact/menu actions, disconnect neutralization, and reconnect in each launch game experience on macOS (four phone/game combinations). Deferred Windows support additionally requires all four combinations, a proven native virtual gamepad driver and signed installation/removal path. Store build/package/descriptor hashes and recordings. Development HID success alone does not satisfy production signing or gameplay gates. An unresolved gamepad or entitlement failure blocks enabling real input and release acceptance. Under the headless policy above, isolated implementation and automated testing may continue with those gates explicitly pending.

### M2 — Workspace, contracts, and receiver seam

Engineering may proceed under the headless policy; final acceptance depends on M1. Create the Bun workspace, strict TypeScript boundaries, contract generation, fixture runner, minimal local IPC, and extracted receiver interface. Build the native mobile input-module skeleton on both phones.

Complete when one semantic controller fixture produces matching TypeScript/Swift/Kotlin/native output, receiver activation is revisioned, invalid/stale packets are rejected, and killing the service neutralizes input within the configured lease. Demonstrate a packaged Bun executable launching the signed helper without a repository checkout.

### M3 — Complete controller vertical slice

Current physical progress: both phones pair through QR and produce independently observed OS-consumer input. The ACK-protected iPhone passes a fresh Connected check after about 254 seconds and sequential full controls; the Pixel passes about 255 seconds against the original slow debug host and a fresh Connected check after about eight minutes against the release receiver. Both finish neutral. Saved-trust reconnection after host restart is demonstrated. These sequential checks do not establish multitouch gameplay, latency or production signing acceptance.

Public `50d53c5` extends the bank integration with strict actual Android/iOS artwork witnesses, store-only notice packaging, unsigned-artifact audits and opt-in bounded debug error evidence. Both actual Kotlin role10 and role11 service witnesses pass UDP and TLS fallback. Android and iOS simulator bank tests each pass one exact case with zero skips/failures, four banks, both themes, appearance CAS, held-input refresh and clean release. The iOS palette oracle now rejects overlapping tolerance boxes; all 15 screenshot boundaries stayed active. The underlying intermittent receiver failure remains unresolved. Earlier hosted eca app witnesses pass five iOS and ten Android cases including 75-second holds, but newer saved-route failures lack enough detail to identify their cause; they are separate open results. The new native diagnostic survives the actual service/fixture chain and an explicitly induced receiver pause without changing deadlines or scheduling. Current-head CI remains pending. Real-game optical testing is deferred until the user is available. Complete notices, signing, gameplay/live-sync and clean-install acceptance remain open. See [the dated reconnect/readiness evidence](rewrite-evidence/m3/reconnect-readiness-2026-10-22.md).

Depends on M2. Deliver CLI startup/status/pairing, mobile discovery/manual pairing, one shared controller layout, two sticks, held triggers, face buttons, local feedback, practice mode, UDP plus reliable fallback, and reconnect.

Complete when both physical phones pass multitouch gameplay in both target experiences under CPU load and intentional JavaScript stalls. Verify no stuck input through backgrounding, lock, rotation, fallback transition, host restart, and profile-generation change. Compare physical latency and ergonomics to M0/M1 baselines.

### M4 — Authority, editing, and recovery

Current authoring progress: public `bcdd3f2` adds explicit profile deletion and retains typed control-label overrides through CLI/MCP, with exact Unicode bounds, explicit blank vs clear-to-generated semantics, lossless unrelated data, final durable revision CAS and replay before current-source decoding. Independent modern/legacy MCP and CLI checks cover concurrent retries, conflicts, changed-intent rejection and service restart. The installed input map/active epochs remain unchanged. Deletion requires explicit active/default replacements, rejects the last profile, removes only its binding-map entries and reindexes retained resource descriptors without garbage-collecting assets. The installed recording pin remains unchanged even when its saved profile is deleted, until explicit release. Phone editing, journals/conflicts, richer appearance/geometry/binding operations and undo remain open.

Phone editor preparation at public `efbba1f` now includes protected bounded journals/caches, strict authenticated SDK coordinators and exact receipt reconciliation, a captured actual-child parent bridge, and strongly retained pending edits with shared orphan drain leases. These components remain unregistered until actual policy acknowledgements and parent failure/shutdown are proven end to end. App factory/UI, verified cross-restart origin reconciliation, richer operations and undo remain open. See the client/drain evidence linked above.

Depends on M3. Implement SQLite authority, revisioned commands/receipts, phone journals, conflicts, profile CRUD, bindings, geometry, assets, appearance variants, haptics, import/export, undo, and deterministic preview scenes. Add CLI/MCP parity around these operations.

Complete when concurrent agent/phone edits, process termination during commit, repeated requests, offline A/B/A selection, asset failures, and restart recovery preserve user edits and unrelated data. Rich export/import round trips retain supported metadata and assets. Native preview and physical-phone render evidence agree within documented tolerances.

### M5 — WoW product experiences

Depends on M4. Deliver exact-client detection, reviewed installation/backup/recovery, Classic ConsolePort onboarding, Forever native setup, character/build selection, snapshot refresh, bank presentation, and user-override preservation. Include diagnostics that distinguish HID, game recognition, addon readiness, and snapshot freshness.

Complete when a clean user can configure and play each experience from CLI/MCP plus either phone, refresh game assignments without losing edits, and recover a failed addon install. Exercise different characters, contextual forms/pets where supported, combat entry/exit, and stale integration data. Live sync additionally needs the optical acceptance described above if included in launch scope.

### M6 — Migration, distribution, and retirement

Depends on M5. Ship reviewed legacy migration, same-owner upgrades, interrupted-install recovery, signed/notarized host packaging, iOS/Android release builds, agent setup instructions, and update/uninstall behavior. Retain necessary native helpers and third-party notices.

Complete when a clean account with no desktop GUI can install, pair, edit, preview, play, update, and uninstall through supported interfaces. Both phone migrations preserve queued edits/trust or provide explicit recoverable re-pairing. Re-run the CLI/MCP cutover acceptance and retire each obsolete owner only after replacement evidence exists.

## Verification and release evidence

### Required suites

| Suite | Evidence |
| --- | --- |
| Portable domain/contracts | Shared vectors, strict bounds, malformed input, deterministic hashes, lossless migration, import/refresh preservation |
| Native input | Codec semantics, ordering, replay, expiry, full-state repair, forced disconnect, profile barriers, JS-stall independence |
| Authority/storage | Concurrent edits, duplicate IDs, receipt replay, crash recovery, assets, old/new database migrations |
| CLI/MCP | Same application results, capability boundaries, machine-output schema, errors, preview artifacts |
| Mobile | Simulator/emulator coverage plus physical iOS/Android touch, haptics, offline journals, reconnection, accessibility |
| WoW | Real installed clients and pinned addons; gameplay and assignment correctness recorded separately |
| Distribution | Clean machine/account, final signed artifacts, permissions, installation/update recovery, no GUI dependency |

Retained Swift model/codec/startup changes still require `./scripts/verify-stack-safety.sh` and existing inline-size/constrained-thread budgets. Replace coverage only with equivalent retained-owner coverage; a larger stack is not a fix. Handcrafted skin creation continues to follow the project skin-author workflow and exact contact-sheet/package approval requirement. This plan does not approve a skin.

Record actual touch-to-receiver and physical touch-to-visible-game latency separately. Include p50/p95/p99, sample count, phone/host builds, network conditions, and load. Do not subtract clocks on separate devices without a validated synchronization method. Synthetic replay results remain codec/scheduling evidence, not end-to-end gameplay measurements.

At M0 set numeric latency, idle CPU/RSS, download/installed size, cold startup, and warm/cold build budgets from measured baselines; freeze thresholds before evaluating the rewrite. At minimum require zero stuck outputs in the fault suite and documented physical latency/ergonomic acceptance on every launch combination. A missing performance budget remains an open gate.

Store evidence under `docs/rewrite-evidence/<milestone>/` when work begins. Each record includes source revision/diff hash, runtime/compiler versions, devices, client/addon builds, artifact hashes, commands, results, failures, and remaining gates. Exclude credentials, private character/account data, and raw backups. Do not mark mock, no-input, or development-signed results as release acceptance.

## Risks and decision triggers

| Risk | Mitigation / decision trigger |
| --- | --- |
| Native recognition or signing fails on final package | Resolve in M1 and repeat on distribution artifacts; do not substitute a CLI readiness flag |
| Shared mobile UI adds input jitter | Native input path; deliberate JS stalls; simplify render work if measured budgets fail |
| Rust remains too coupled to configuration | Extract a bounded runtime contract; resist porting its catalog into the receiver seam |
| Bun/native packaging incompatibility | Out-of-process helpers avoid runtime ABI coupling; test signed distribution in M2 |
| New model loses raw metadata or queued phone edits | Lossless fixtures, explicit admission, durable backups and migration receipts |
| Game/addon update changes mappings | Versioned compatibility records, adapter fixtures, stale-state indication, manual fallback |
| Optical sync unreliable or unavailable | Independent gate and explicit snapshot workflow; mandatory-live decision changes launch scope |
| Rewrite grows into feature parity without a release | Complete M3 before broad editor work; track retained operations and explicit deferrals |
| Multiple authorities or discovery confusion | Separate development identities/state; same-owner installation and lease enforcement |

## First implementation slice

Under the headless implementation policy, implement M2 as one reviewable slice while preserving outstanding M0/M1 physical acceptance: workspace setup, a single contract fixture, an extracted receiver adapter, a Bun CLI that starts/statuses/releases it, and a native-module smoke build on iOS/Android. Keep old production entry points intact. Acceptance is a signed isolated receiver driven by the new contract, with neutralization on service death and no shared-state writes to the existing authority.

## References

- [CLI/MCP cutover](cli-mcp-cutover.md), [stack safety](stack-safety.md), [authority migration](authority-migration.md), [owned phone edits](owned-phone-edit-contract.md).
- [Virtual controller contract and gates](virtual-controller-support.md), [Classic ConsolePort lab](research/wow-consoleport-lab-2026-10-20.md), [Thumble Link/setup/live contracts](thumble-link.md), [Forever template](wow-forever.md).
- [Windows/Linux receiver limitations](cross-platform-receiver.md).
- [Bun standalone executables](https://bun.sh/docs/bundler/executables): runtime embedding and cross-compilation do not remove native signing/build requirements.
- [Expo development builds](https://docs.expo.dev/develop/development-builds/introduction/) and [React Native threading](https://reactnative.dev/architecture/threading-model): basis for shared UI with custom native modules.
- [ConsolePort upstream](https://github.com/seblindfors/ConsolePort): verify compatibility against the exact selected release during M0/M5.
