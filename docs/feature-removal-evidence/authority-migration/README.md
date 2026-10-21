# Reviewed authority migration evidence

The tested source starts at `1423291` plus the isolated source hashes recorded here. Native CLI/helper binaries are reused from the asset/defaults cutover; their unchanged native behavior and stack coverage are documented in [that evidence](../asset-defaults-cutover/README.md). Concurrent phone, WoW, gamepad and design implementation changes are excluded from this isolated Rust build.

The full pinned Rust 1.88 gate passes 668 tests with three historical native-dependent tests ignored, including strict clippy, formatting, fixtures and target checks. The updated CLI/MCP ledger covers 157 operations, 29 roots and 268 spellings.

Actual CLI and local MCP migration/rollback pass all five source layouts through the native converter. Exact private backups, all eight saved appearances, local launch associations, precise metadata and trust survive. Live-owner refusal, required write opt-in, changed-plan refusal, exact replay, later-write protection, absent archive restoration, conflicting archive refusal and rollback replay checks pass. No installed authority is touched.

The actual defaults/assets process check also passes: reviewed CLI/MCP bootstrap, pairing and identity, exact private source backup, source checksums, replay, running/existing authority protection, legacy asset metadata in all three declared slots, bytes/integer preservation, native previews, helper-free replay and fresh startup without preference reads.

Phone journals must be reconciled before migration. The authenticated status field describes the currently connected peer's format offer only. Physical haptics/editing, live WoW, actual app activation, comparable input latency and a clean signed tools-only installation remain separate acceptance gates. This batch does not delete the desktop target or enable automatic migration.

## Shared-checkout qualification

The broader merged gate stops at the frozen owned-projection oracle provenance check: concurrent changes to `Tests/OwnedConfigurationDocumentTests.swift` differ from the manifest source hash. Frozen fixtures and hashes were left intact. The isolated batch above passes that guard. Merged Rust formatting, strict clippy and workspace tests are checked separately; their result is recorded below when complete.

Merged Rust formatting and strict clippy pass; workspace tests pass 686 cases with 3 historical native-dependent cases ignored. This includes the concurrent retained Rust WoW, phone and gamepad changes, but does not validate concurrent native/mobile changes.
