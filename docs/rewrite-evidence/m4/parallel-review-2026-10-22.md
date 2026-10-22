# Parallel implementation and review — 2026-10-22

The user authorized maximum useful subagent parallelism to expedite implementation and review. Standing authorization covers source-only CI publication without repeated questions. Private baseline/history/evidence, credentials and generated builds remain excluded from the public projection. The full rewrite goal stays active; no acceptance milestone is complete.

## Review and source boundary

Draft PR: https://github.com/CodyBontecou/Thumble/pull/4 . It retains the existing product and uses recording output. The review guide is `Rewrite/REVIEWING.md`; the PR explicitly lists unimplemented product and release gates. It is not merge/release readiness.

Local source commits: `3040c84` review/package documentation and bounded harness evidence; `4aa2e98` deterministic contention safety characterization; `1c39824` toolchain-scoped compiler cache; `f18e692` bounded saved import; `fe9bbda` bounded iOS original-error evidence and shared branch app concurrency. All are scoped commits on `codex/typescript-rewrite`.

Public current head: `766f89b66d369a194a758619e93a14360602dac6`, parent `a101340bf37707648af4885e1295114257946ebd`. Import has 17 changed paths, iOS diagnostic has four. Both recursive tree deltas were verified against immutable source capsules and published with a non-force expected-parent update. Neither publication includes private worktree history.

## Import verification

Independent public projection: `/private/tmp/thumble-native-input-v4-public-projection`, using only reviewed source, not concurrent native edits. `bun run verify`: 168 pass, one intentional standalone-test skip, zero failures, 3,190 assertions. `bun run build`: standalone CLI and recording receiver. `bun run test:package`: one pass, 67 assertions, 10.18 seconds. Toolchain: Bun 1.4.3 and Rust 1.88.0. Logs: `/private/tmp/thumble-import-projection-{verify,build,package}.txt`.

The import pins reviewed source hash and authority revision, stages assets before receipt publication, recovers original durable receipts before rereading missing files, bounds parent cache at 32 and bulk concurrency at one, and preserves input/status responsiveness while its child is suspended beyond ordinary request budget. Structural admission explicitly does not certify activation/image/platform compatibility. Normal child, client, owner and input budgets remain unchanged.

## Windows diagnosis and pending fix

Measured failing public workflow: https://github.com/CodyBontecou/Thumble/actions/runs/39822804159 , Windows job 119978203547. Durable trust authorization max 360,913 us; control held max 372,380 us; control wait max 372,288 us; in-flight sample 421,421 us. Status is inactive/neutral/input_expired. Prior eight consecutive witnesses passed on run39820408550, so green runs alone did not resolve the intermittent failure.

Published contention characterization demonstrates independent expiry and no queued resurrection while durable trust/mutation locks are blocked. It is safety evidence, not a responsiveness fix. The native agent is implementing a persisted bounded clock reservation, packet-local authorization epoch guards and revoke/re-enrollment fencing with migration/restart/failure/race coverage. Uncommitted test reports are provisional until frozen review and exact-source CI; no TTL or lease relaxation is authorized.

## iOS evidence

Hosted minimal shared/ephemeral loopback run39820408669 succeeds across three cold launches. Bundled run39818097569 completes enrollment/fresh neutral admission but fails at the first exact held trigger within the existing one-second host witness. Earlier run39811471888 fails its QA HTTP begin before enrollment. These do not establish the same cause. No phone product-routing change is justified yet.

Local bounded diagnostic source passes all five UIKit cases including 75 seconds/two renewals. Injected original URL failure remains failure(-1005) even when a separate diagnostic ephemeral health check succeeds. Five collector tests pass. The strict collector requires exact-test failure metadata, exact known fields and bounded values/bytes, ignores all unrelated raw attachments and only uploads validated JSON. Raw xcresult exports stay private in RUNNER_TEMP. Compiler caches include only exact-toolchain module/stat caches, with fresh product builds, simulator lifecycle and acceptance every run; time savings remain unmeasured.

## Remaining and concurrent work

The configuration agent implements explicit revision-pinned full saved export; the native agent implements the measured Windows fix; the iOS agent triages hosted evidence and prepares an independent portable optical decoding/freshness seam. Root reviews, freezes, verifies isolated projections, publishes source capsules, updates review evidence and integrates workflows.

Retained Rust-host parity check reports unmapped shared-build on run39822813463. Its CLI, capability ledger and verifier blobs match main exactly; this baseline failure is distinct from new rewrite checks. Do not classify active public shared-build commands as retired solely to silence this check.

Physical output, signed Windows driver/host packaging, real-phone gameplay/ergonomics and optical capture/delivery acceptance remain pending under the user-approved headless policy. Rich editing/preview, phone journals/sync, discovery, haptics, WoW experiences, migration and distribution also remain open. Review of this foundation does not certify a usable gameplay beta.
