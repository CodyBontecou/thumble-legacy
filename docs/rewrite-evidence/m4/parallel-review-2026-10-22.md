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

## Follow-up reviewed publication: 990e99b4

Current public source `990e99b48a5c66c8cc4692acf62a090b86126e4d`, tree `b2b556a92128db41a91d2e886f001b08acd45e32`, sole parent `b92ce94a9119b88a3bc430a83738249541c6cc0b`. Exact recursive delta34paths verified against immutable combined capsule before expected-parent non-force publication. Local source commits include native2000540/test3ef176c, optical37e9fcf + scalar-bank278d09d + literal-typee9dce9f + pixels694e252, export5839d3f + anchored Unix traversald37bc5b, TLS diagnostic7f77d55 and docs4caa2eb. No private history/evidence/assets entered the public tree.

Frozen capsule `/private/tmp/thumble-review-combined-with-pixels-capsule.json`. Independent projection gates:197source tests/3,571assertions, one intentional package skip; Rust108unit+4asset+4export+14ingress=130tests; cargo fmt check and debug/release all-target Clippy; standalone1test/67assertions; six Python harness tests. Logs `/private/tmp/thumble-review-pixels-final-verify.txt`, `/private/tmp/thumble-review-combined-{fmt,clippy,release-clippy,native-tests,build,package,python}.txt`. Swift diagnostic source separately passes88SDKtests, both SDK compilations, UIKit5actualapp tests and retained55stackcases+bothbuilds; no TLS policy/deadline retry change.

Review corrected two real issues before publication: bank enum validation had coerced array values to strings; supplemental scalar-type regression now rejects them. Export's initial Unix parent check was vulnerable to intermediate symlink substitution between path inspection and open; it now walks anchored component descriptors with O_NOFOLLOW and has an intermediate-symlink test. Pixel decoder additionally passed independent retained Swift comparison and bounded adversarial scan tests. Synthetic optical assets contain no private character/account data or skin packages.

The b92 isolated lib variant had an extra trailing blank; dispatched exact-head run39827483029 failed formatting before native tests. No behavior resolution is inferred. A normalized immutable variant is archived; next34path source instead includes formatted export routing, and root format check passes. Exact990host PR run39828595699, Windowsjob119998407820, is under review. Automatic push39828588324was cancelled by branch concurrency in favor of PR run; no manual redispatch needed for990.

Retained stack workflow39824999880failed because solar-sumi archive fixture is absent from public repository. StackSafetyRegressionTests.swift blob5c48e2f1bff943dd9af936a755c57556b5154a12matchesmain exactly; constrained budget checks were not weakened. Private skin archive is not published as a workaround. This is distinct from a measured stack-budget regression. Retained shared-build parity failure also remains visible.

Saved profile selection8ff8666/0ddcb61is local only, not in990. Initial focused/full tests pass, but retained Swift audit found selecting disjoint controlIDs also requires synchronizing known root binding mirrors from per-profile maps. The selector-only design is incomplete; correction and a disjoint-profile export/import regression take precedence over further CRUD. Do not publish or present this increment as complete until that invariant is fixed.

## Exact990CI terminal result

Run39828595699: Windows119998407820SUCCESS and Ubuntu119998407144SUCCESS. Windows platform-applicable native105unit+4asset+5export+14ingress=128tests, eight stale-grant witnesses, source194pass/3skip/1todo and standalone67assertions pass. These conditional counts differ from macOS local totals; the in-process contended-store/TLS witness is Unix-only and was not executed on Windows. Full native Windows forwarding/migration/export cases ran where applicable. Native timing maxima in this particular run were below the350msTTL, so this does not reproduce slow storage physically.

macOS119998407594FAILS the new contention witness at sender queue barrier: line177recv_timeout2sec; sender's status receive reports authenticated_control_transport_failed atline165.107unitcases pass, one fails; later suites do not run. Native agent is diagnosing elapsed loop/status roundtrip time and committed reservation remainder without weakening leases or assertions. A fixed20iterations×20mssleep does not itself bound wall-clock elapsed when status roundtrips run each iteration; this is a hypothesis, not a certified cause. No full-matrix green is claimed.
