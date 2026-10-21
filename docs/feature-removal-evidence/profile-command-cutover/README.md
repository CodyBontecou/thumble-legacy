# Profile command cutover evidence

This batch was built from `bcb8dc1` plus the recorded isolated source changes, excluding concurrent phone/WoW/gamepad/Android work in the shared checkout. `source-sha256.json` and `binary-sha256.json` identify the exact tested inputs and Debug executables. No installed authority or real application was changed/opened.

- The full pinned Rust 1.88 gate passes: formatting, strict clippy, fixture/parity/source checks, 653 tests, three historical native-dependent tests ignored, and retained target checks.
- 83 focused native tests pass, including source/default creation, typed CLI payloads, application scope/no-op behavior, issue-specific locked/unlocked repair and refusal to call legacy runtime callbacks. A new default-creation test protects binding-map UUID spelling across edits.
- The required stack gate passes 52 constrained-stack tests and Mac/iOS Debug builds with unchanged inline/frame budgets and 512 KiB thread coverage.
- `process.json` records final merged CLI/receiver/MCP acceptance in disposable private homes for legacy and owned stores. Copy/blank/default creation, selection/defaults, exact owned extension numbers, unsupported legacy precision refusal, helper-free replay, application attachment/detachment, revision-correlated inspection, scoped issue repair, all eight native previews, private MCP edits/commit/replay and input-opt-in launch refusal pass. Repeated no-op edits can append receipts while leaving profiles and revision unchanged.
- A held legacy lease and listening socket receive no ordinary command traffic. Only explicit migration inspection sends one snapshot request, retains local app association, returns its correlated revision and leaves authority bytes unchanged. It is a DTO recovery projection, not raw preference backup or automatic migration.
- Six historical owned edit responses and the rich-background response replay unchanged. `preview-sha256.json` identifies eight native PNG renders; all eight PNGs are retained here.

The initial acceptance failures caught missing sidecar fallback rules, legacy numeric-copy rounding, the attachment changed-path scope, binding-map UUID case changes causing opaque merge conflicts, and the explicit snapshot's revision requirement. Production fixes retain narrow independent validation. Test corrections distinguish UUID spelling and durable receipts from semantic changes, and use a genuine legacy DTO for recovery.

Reproduce with `scripts/verify-rust-host.sh`, `scripts/verify-stack-safety.sh`, the three focused native test classes and `scripts/verify-command-cutover.py --cli PATH --bridge PATH --cli-bridge PATH --host PATH --mcp PATH --output-dir PRIVATE-DIRECTORY`. See [the contract](../../profile-command-cutover.md).

These are codec/process/build checks. Physical phone haptics/editor flows, actual application activation, live WoW, same-condition input latency and a clean signed tools-only install remain separate acceptance requirements. Remaining legacy asset metadata/defaults owners still gate desktop deletion.

## Merged retained-feature verification

The final merged pinned Rust gate passes 681 tests, with three historical native-dependent tests ignored. The combined MCP router retains all four concurrent tools plus this batch's launch tool (41 total). `merged-source-sha256.json` records concurrent Rust/addon/verifier inputs; `merged-binary-sha256.json` records the actual Rust processes and isolated native CLI/helper used by these checks. This integration evidence includes existing unstaged phone, migration, WoW and gamepad changes; it does not attribute those changes to the isolated cutover commit. Native test/stack/build results above apply to the isolated native source set, excluding concurrent native phone/WoW changes.

`migration.json` passes real plan/apply/replay/rollback across five legacy layouts, retaining local launch data, exact backup/archive restoration, eight appearance states and later-write protection. `phone-wire.json` passes authenticated edits, refusal/replay/restart/receipt checks and exact metadata retention. The addon Lua exporter matches the Forever and Classic Era warrior native fixtures exactly; live transport and ConsolePort onboarding checks pass. `addon-checks.log.gz` records those Lua checks. None uses a running WoW client or physical phone.

The final Rust precision-refusal regression also covers legacy state with its documented revision field omitted. The explicit snapshot uses the authenticated captured revision when none is supplied, while preserving an explicitly requested revision check. Both are included in the final isolated and merged gates.
