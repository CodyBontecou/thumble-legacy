# Owned high-level profile planning

The existing high-level CLI protocol now routes explicit owned authorities through the raw owned draft store. It reuses the established selector, deterministic-ID and operation planner, then applies retained native operations through the owned bridge. Live saves use the receiver's release/atomic-install path; stopped-receiver saves use raw source-checked installation while the CLI bridge holds the existing authority lease. The legacy authority path remains unchanged.

Uncaptured retries resolve selectors and planned destinations against the original authority, so a staged rename does not invalidate its own name selector. Operation logs and durable request digests prevent an invocation from changing meaning. Saved retries replay from the receipt even after draft cleanup. Design capture creates or resumes an owned draft at an exact revision and returns original raw profile JSON, including appearance resources, all saved appearance states and precise unknown numeric tokens. Binding capture remains the bounded known input/output projection, not a portable export.

Read projections resolve owned appearance using the profile's light/dark preference and Base action set in a disposable display state. That paint is never persisted. Orientation reads report the canonical landscape layout; portrait-specific reads reject instead of falling back silently.

Four new unit cases cover staging failure and name-selector retry, lossless commit and invocation conflict, exact design capture/resume and revision refusal, dark appearance style reads without persistence, landscape reporting, portrait refusal, stale configuration rejection and unavailable owned interchange. The process verifier is extended with optional native CLI/CLI-bridge binaries to exercise online/offline high-level routing and design capture alongside its existing local MCP/native-operation flow.

The public native CLI still guards `element set` as unmigrated. The process verifier checks that refusal leaves the authority unchanged, then exercises its supported appearance command through the owned native bridge. Remaining native element-command routing must be completed before desktop retirement.

Owned portable import/export remains explicitly unavailable. It requires a coordinated raw v5 artifact codec and retained peer readers; this batch does not disguise v5 profiles as legacy artifacts. Automatic migration, phone format negotiation, complete design-operation acceptance, user-profile conversion, active skin/portrait deletion and desktop UI retirement remain unfinished. No existing user profiles or deployed services are changed by verification.

The process harness uses a temporary account home and canonical Application Support paths because the CLI bridge intentionally clears caller-provided path overrides. It retains that restriction. Geometry is compared to the frozen native destination, preserving the existing point-based nudge semantics.

Final verification passes the isolated Rust gate (567 tests, 3 existing native-dependent ignored), formatting/Clippy/target checks, 41 constrained-stack cases and both Apple Debug builds, a fresh native CLI build, and the real temporary-account native CLI/bridge/MCP process flow. Source, binary and log hashes are in `verification.json`.
