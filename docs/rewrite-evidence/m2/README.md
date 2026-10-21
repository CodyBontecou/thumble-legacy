# M2 headless workspace and contract slice — in progress

Implemented under the user's [headless execution policy](../m0/headless-execution-decision.md). Windows and required live sync remain launch requirements; no local Windows machine or connected phone is required for ongoing engineering. No milestone/release acceptance is marked complete.

## Implemented

- `Rewrite/` Bun workspaces with Bun 1.4.3, TypeScript 6.0.3, exact dependency versions and one frozen text lockfile. The local Darwin ARM64 Bun archive was fetched from the official `oven-sh/bun` release; archive SHA-256 `80afd4c069b46a6fa8f3ec520bebc750a7843e4a8d5b9583d29ad436c00a3403` was verified before execution.
- One language-neutral normalized semantic gamepad schema; deterministic generated TypeScript, Swift Codable and Kotlin DTOs. DTOs are not validators or authenticated envelopes.
- Strict immutable TypeScript state validation: bounded/unique known buttons, required finite normalized axes/triggers, no unknown or accessor fields, and independent ownership. Byte limits and duplicate-key JSON scanning still belong to the future transport codec.
- A pure portable encoder matching all **34** retained Swift/Rust v2 report vectors, including both triggers, Y polarity, D-pad cancellation, clamping and native signed rounding. Exhaustive semantic button mapping makes unimplemented enum additions fail typechecking.
- Portable packages compile without Bun/Node/DOM ambient globals. AST dependency checks reject host/storage/platform imports, relative package escapes and dynamic imports; negative regression cases accompany the check.
- A headless GitHub Actions matrix for Windows/macOS/Linux. Windows receiver CI no longer attempts an interactive foreground-window witness; it runs unit/contract/loopback tests, captures read-only XInput availability and builds the probe for later beta tests. Linux retains its separate native uinput witness.

## Verification

Local Bun verification: **41 tests passed**, strict general and portable typechecks passed, generated bindings matched, generated Swift DTOs typechecked, both workflow files passed `actionlint`. The gate checker has **12 passing regression tests**, including proof that headless implementation permission does not certify physical acceptance. Logs and JUnit evidence are adjacent.

Hosted [rewrite run 39642030587](https://github.com/CodyBontecou/Thumble/actions/runs/39642030587) passed on **Windows, macOS and Linux**, with JUnit artifacts. Tested commit: `c14f3803f0016f852ba5af353542256babe0e01f`.

Hosted [receiver run 39641444003](https://github.com/CodyBontecou/Thumble/actions/runs/39641444003) passed on Windows and Linux, including the read-only Windows probe and release build. Tested commit: `34ba0f503770dc99afd19f1a4097fcdebfb7fcf6`. The current receiver manifest, probe source and workflow were compared byte-for-byte with that commit and match. One foreground native-delivery test is intentionally ignored in the Windows ordinary suite; it is not represented as passed.

The CI branch `codex/typescript-rewrite-headless-ci` was built from remote main with only rewrite sources, the retained v2 fixture/MIT notice and required Windows witness/workflow changes. It excludes unrelated uncommitted design work preserved in the local baseline. **These checks do not certify the full copied baseline.** Exact run IDs, job steps, artifact IDs/digests and tested file hashes are in [github-actions.json](github-actions.json). Artifact downloads expire; no signed download URLs or credentials are stored. The raw XInput observation is retained as a hosted artifact; its device state was not inspected locally and is not a game-recognition result.

## Remaining M2 implementation and acceptance

The native receiver seam, isolated same-user IPC and lifecycle CLI/service, generation/lease neutralization integration, native mobile input module skeletons and mobile build CI, standalone Bun packaging and signed-helper invocation are not yet implemented by this slice. The Kotlin DTO is generated but has not yet been compiled in a native mobile build. No new listener, pairing, input transport or production native output is enabled.

Physical multitouch, real game enumeration and gameplay, driver/signing/install, haptics, physical latency, native preview parity, migration, and required optical live sync remain beta/release evidence requirements. Continue implementing those capabilities headlessly and keep their acceptance records pending until the appropriate evidence exists.
