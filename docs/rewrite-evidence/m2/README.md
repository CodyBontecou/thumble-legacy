# M2 headless workspace and contract slice — in progress

Implemented under the user's [headless execution policy](../m0/headless-execution-decision.md). Windows and required live sync remain launch requirements; no local Windows machine or connected phone is required for ongoing engineering. No milestone/release acceptance is marked complete.

## Implemented

- `Rewrite/` Bun workspaces with Bun 1.4.3, TypeScript 6.0.3, exact dependency versions and one frozen text lockfile. The local Darwin ARM64 Bun archive was fetched from the official `oven-sh/bun` release; archive SHA-256 `80afd4c069b46a6fa8f3ec520bebc750a7843e4a8d5b9583d29ad436c00a3403` was verified before execution.
- One language-neutral normalized semantic gamepad schema; deterministic generated TypeScript, Swift Codable and Kotlin DTOs. DTOs are not validators or authenticated envelopes.
- Strict immutable TypeScript state validation: bounded/unique known buttons, required finite normalized axes/triggers, no unknown or accessor fields, and independent ownership. The local receiver seam now adds byte limits and duplicate-key JSON scanning; authenticated phone transport remains separate.
- A pure portable encoder matching all **34** retained Swift/Rust v2 report vectors, including both triggers, Y polarity, D-pad cancellation, clamping and native signed rounding. Exhaustive semantic button mapping makes unimplemented enum additions fail typechecking.
- Portable packages compile without Bun/Node/DOM ambient globals. AST dependency checks reject host/storage/platform imports, relative package escapes and dynamic imports; negative regression cases accompany the check.
- A headless GitHub Actions matrix for Windows/macOS/Linux. Windows receiver CI no longer attempts an interactive foreground-window witness; it runs unit/contract/loopback tests, captures read-only XInput availability and builds the probe for later beta tests. Linux retains its separate native uinput witness.

- An independent Rust recording receiver with generated DTOs, shared codec vectors, revision/generation/session barriers, exact u64 counters, full-state repair, replay checks and native lease/input expiry.
- A serialized application interface, a detached headless service, authenticated same-user loopback IPC and CLI lifecycle/activation/sample/release commands. Runtime state is isolated from all retained authority; no production input is emitted. See [receiver-seam.md](receiver-seam.md).
- Standalone compiled Bun CLI plus sibling receiver packaging and a no-checkout/no-Bun-on-PATH integration test. Signing and production output remain open gates.

## Initial workspace verification

Local Bun verification: **41 tests passed**, strict general and portable typechecks passed, generated bindings matched, generated Swift DTOs typechecked, both workflow files passed `actionlint`. The gate checker has **12 passing regression tests**, including proof that headless implementation permission does not certify physical acceptance. Logs and JUnit evidence are adjacent.

Hosted [rewrite run 39642030587](https://github.com/CodyBontecou/Thumble/actions/runs/39642030587) passed on **Windows, macOS and Linux**, with JUnit artifacts. Tested commit: `c14f3803f0016f852ba5af353542256babe0e01f`.

Hosted [receiver run 39641444003](https://github.com/CodyBontecou/Thumble/actions/runs/39641444003) passed on Windows and Linux, including the read-only Windows probe and release build. Tested commit: `34ba0f503770dc99afd19f1a4097fcdebfb7fcf6`. The current receiver manifest, probe source and workflow were compared byte-for-byte with that commit and match. One foreground native-delivery test is intentionally ignored in the Windows ordinary suite; it is not represented as passed.

The CI branch `codex/typescript-rewrite-headless-ci` was built from remote main with only rewrite sources, the retained v2 fixture/MIT notice and required Windows witness/workflow changes. It excludes unrelated uncommitted design work preserved in the local baseline. **These checks do not certify the full copied baseline.** Exact run IDs, job steps, artifact IDs/digests and tested file hashes are in [github-actions.json](github-actions.json). Artifact downloads expire; no signed download URLs or credentials are stored. The raw XInput observation is retained as a hosted artifact; its device state was not inspected locally and is not a game-recognition result.

## Receiver/service/CLI verification

The [complete seam run 39651775999](https://github.com/CodyBontecou/Thumble/actions/runs/39651775999), commit `43971b702aa9224b7c170fb771f165bea2a09e57`, passed on **Windows, macOS and Linux**. Each platform passed six native tests, 48 source tests (the compiled-package test is intentionally skipped in that phase), and the separate compiled-package test. Strict TypeScript/portable checks, generation drift, dependency boundaries, Rust format/Clippy and packaging passed; macOS also typechecked generated Swift DTOs.

The Windows suite caught and resolved two failures: Bun's asynchronous rejection matcher crashed around real socket failures, and abrupt service termination also killed the native safety owner. Tests now capture errors explicitly with exact error assertions, and the Windows receiver has an independent detached process lifetime. The unchanged service-death test now observes native disconnect neutralization and neutral restart. See [the diagnosis and failed-run evidence](windows-runtime-diagnosis.md).

Local verification has the same six native/48 source/one package passes, with final formatting verification. [Local logs](receiver-seam-local.txt), [source JUnit](receiver-seam-junit.xml), [package JUnit](receiver-seam-package-junit.xml), [development artifact sizes/hashes](receiver-seam-local-artifacts.json), and [retained inventory/gate checks](receiver-seam-gate-checks.txt) are saved here. The inventory still maps all 277 retained operations; all 12 evidence-tool regression tests pass. No retained Swift model/startup/codec source changed in this seam, so its existing stack budgets and coverage remain unchanged.

The final snapshot also normalizes Windows glob separators, exempts only the explicit host client adapter, removes temporary diagnostics, and adds pinned Prettier checking to CI. The [final snapshot run 39652182641](https://github.com/CodyBontecou/Thumble/actions/runs/39652182641), commit `970b74ffcd880226636ad5d955e78ee187a8a49d`, passed on Windows and Linux; its macOS job was queued at evidence capture. Local macOS checks and the preceding full hosted matrix passed. All 53 curated source/fixture/notice/workflow files match the final CI snapshot. Exact source hashes, job statuses and hosted artifact metadata are in [receiver-seam-github-actions.json](receiver-seam-github-actions.json).

## Remaining M2 implementation and acceptance

Native mobile input-module skeletons and a separate SDK/library workflow are implemented. Local native tests and both iOS SDK/Android library builds passed; hosted native CI awaits approval for public source publication. See [mobile-native/README.md](mobile-native/README.md). Signed-helper invocation remains an acceptance gate. The receiver/service/CLI seam and unsigned standalone recording package are implemented and their evidence is recorded above. The Kotlin DTO now compiles inside the Android AAR and passes the native codec fixtures. An explicitly started loopback control listener is available for development. Pairing, phone input transport and production native output are not enabled.

Physical multitouch, real game enumeration and gameplay, driver/signing/install, haptics, physical latency, native preview parity, migration, and required optical live sync remain beta/release evidence requirements. Continue implementing those capabilities headlessly and keep their acceptance records pending until the appropriate evidence exists.
