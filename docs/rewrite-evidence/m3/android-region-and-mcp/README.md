# Android touch routing and rewrite MCP — in progress

The six-file Android repair is local `22f4e47`, public `09d71c76`. Native capture
uses separate Activity-child regions and one shared engine outside React root
ancestry. An actual two-finger test failed with the old full-screen portal because
it swallowed Done while another control stayed held. The repaired seven-case
suite passes twice locally, including a repeat begun with the synthetic emulator
asleep. Native feedback must advance while the actual JavaScript queue is blocked.
Sleep, background, cancellation and rotation leave neutral input. The selected
emulator is explicitly woken during setup, and primary failures survive cleanup.

The earlier public `b41af1a` app run passes both iOS UIKit cases. Its Android job
failed during wake/setup before any input witness. Deliberately putting the local
emulator to sleep reproduced the setup failure. The repair builds APKs before CI
boots its emulator. Android repair run `39740319712` was superseded during
instrumentation by the MCP lockfile push; its cancellation is not a passing result.
The successor app run is `39741905588`: all seven Android cases ran, with two
stick comparisons differing by one quantization unit; the other five passed.
The iOS repair job `119695700197` passes both hosted cases. iOS concurrency preserves active cold
builds. No real phones or physical output were used.

The 14-file MCP source is local `0a45506`, public `1d923e9f`. It adds a bounded
stdio adapter in the standalone CLI using the same local service client. Default
tools inspect service/pairing and release input; activation and samples require
the launching user's recording-input flag. No endpoint tokens are returned.
Closing MCP leaves service ownership intact. SDK license notices accompany the
binary and audited runtime distribution hashes are checked before verification
and packaging. See [the MCP contract](../../../rewrite-stdio-mcp-v1.md).

Local validation passes 102 TypeScript cases and standalone packaging (1/55).
The first tests used the SDK's default legacy negotiation even in the intended
modern branch. Successor local `f18dd6c` pins the modern revision and asserts each
negotiated era. It passes three MCP tests/74 assertions and packaging 1/57.
Only the successor establishes actual modern-plus-legacy coverage. That
strengthening was published in `d26d7d2`; its later Windows timeout is tracked
below. Exact-source results are recorded in the manifest.

The repair snapshot's native SDK/Keychain/Keystore jobs all pass, as do macOS and
Linux host jobs. Windows passes the stale-grant test alone and fails the
concurrent suite while the sampling worker sends a status barrier. The sanitized
failure is preserved here. The earlier bounded producer did not eliminate this
load-sensitive failure. The MCP workflow bounds outer Rust fixture concurrency
to two threads on every host, preserving every test, internal concurrent
scenario, production deadline, and assertion. The full native suite passes
locally and on hosted Windows at that budget. Subsequent MCP status handling
remains under repair. This is a hosted
fixture-resource experiment, not physical throughput or latency certification.

The manifest records public single-parent trees, frozen capsule hashes and raw
local-log hashes. Public publication contains exact source snapshots only;
private history/evidence, credentials and build products are excluded. Native
phone enrollment/binding UI, continuous renewal, full configuration/MCP parity,
haptics/editor, signing, gameplay and mandatory optical live sync remain open.

The MCP fixture also exposed platform-specific stream behavior. Public
`d26d7d2` explicitly pins/asserts modern versus legacy negotiation and reads
startup pipes concurrently. Windows then reaches modern negotiation but stalls
at listing. Public `1641db6` replaces write-callback waits with the official
writable boolean/drain contract; its regression is observed red before the fix.
Windows advances through listing and stalls at the service status tool. The
new snapshot awaits Windows directory protection asynchronously with a bounded
helper timeout, then performs the same endpoint validation. This remains an
unconfirmed Windows repair until its exact hosted run finishes. Sanitized traces
retain each failure; none of the timed-out runs count as a full pass.

Current source is local `6b3bee1`, public `afaeafe7` (16 exact verified paths,
sole public parent `1641db6`). Both native libraries now decode the existing
seven-field invitation with strict byte/type/identity limits and normalized
rejection errors. The proof stays native. One shared public fixture covers
IPv4/hostname/IPv6 plus malformed inputs, and existing enrollment scenarios now
obtain their bootstrap object through this decoder. Manual/QR UI remains open.

Physical Android stick comparisons now permit one signed 16-bit unit from Float
screen-coordinate conversion. Buttons, triggers, headers, reserved bytes and
complete neutralization remain exact; canonical protocol vectors are unchanged.
Both app jobs preserve active builds across successor pushes. Local verification
passes 104 TypeScript cases, packaging 1/57, 74 Swift and 93 Android cases with
zero skips, both iOS SDK builds, seven Android app cases and two UIKit hosted
cases. The stack gate again passes 55 cases and both builds. Current hosted runs
are [host](https://github.com/CodyBontecou/Thumble/actions/runs/39745233044),
[native mobile](https://github.com/CodyBontecou/Thumble/actions/runs/39745233063)
and [bundled app](https://github.com/CodyBontecou/Thumble/actions/runs/39745232916);
results remain pending. The original public Android failure and all Windows
failures remain explicit, rather than being overwritten by local successes.

## Completed host validation and corrected client driver

Public `60ef45d1` passes all three host jobs in run `39748760932`, including
Windows's isolated/concurrent native tests, all 106 portable cases, source MCP
interoperability and standalone packaging. Its direct child-process test proves
natural EOF and malformed-frame exit plus protocol-only stdout, without an SDK
client killing the server. Windows passes that witness in 397 ms.

The independent Node client had already passed all four protocol/capability
scenarios on public `9bac3f0`, while the Bun-hosted SDK client timed out after the
Bun server accepted and sent its explicit unavailable-tool error. The preserved
trace and receipt distinguish a consumer-runtime compatibility failure from a
missing server response. Tests now use the SDK's supported Node runtime on every
platform against the actual Bun server. No Windows interop case is skipped.

Public `274194d2` expands this to all four combinations against the standalone
binary, with its sibling receiver and a server PATH excluding both Bun and Node.
The driver counts completed scenarios and records actual negotiated eras. This
exact snapshot passes Windows, macOS and Linux in
[run 39749930024](https://github.com/CodyBontecou/Thumble/actions/runs/39749930024).
Local projection checks pass 106 cases and standalone packaging 1/46; the Node
assertions inside the driver are additional to the Bun assertion count. Native
phone APIs remain at 74 Swift/93 Android cases and the unchanged stack gate is 55.

The Android `afaeafe7` artifact shows all seven cases stopping in setup because a
Pixel Launcher boot-ANR dialog covers the running Activity. Its screen is on;
this differs from the earlier sleep failure and the prior physical quantization
failure. Setup can close one exactly identified launcher ANR per target-Activity
wait on emulator hardware. It leaves Thumble app failures visible, and repeated
launcher failures in that wait fail. All seven cases pass locally after this
change. The [bundled-app run](https://github.com/CodyBontecou/Thumble/actions/runs/39748760846)
now passes both jobs: all seven Android witnesses, production-shaped APK assembly
and both UIKit hosted cases. [Native/provider validation](https://github.com/CodyBontecou/Thumble/actions/runs/39749929877)
has a test-only asynchronous-writer race described below. The preceding iOS native run was cancelled by source supersession
and is not counted as a pass.

Private source/history and this evidence were never included in public parents
or capsules. Every published snapshot was checked for its exact changed paths,
modes, Git blob hashes and sole public parent. The dedicated worktree is clean
at each source checkpoint. The unused owned Metro server and its one emulator
reverse mapping were removed after bundled app tests; unrelated devices/processes
were untouched. Native pairing/binding UI, renewal, configuration/editor work,
physical output/signing, gameplay and mandatory optical live sync remain open.

## Native writer fixture follow-up

Android native job `119728156595` on public `274194d2` fails at a test's immediate
`io.inputs().size == 1`: the asynchronous native writer has not yet delivered the
accepted first neutral packet (observed zero). Pausing that exact fixture writer
reproduces the same failure locally. The test now explicitly holds the writer,
requires admission to return while delivery is blocked, verifies metadata
inspection cannot consume/rebind the grant, releases the writer, waits for actual
delivery, and still requires exactly one packet plus a connected engine. Native
production delivery code, socket deadlines, grants and TTLs are unchanged.

The two-file repair is local `d5a9988`, public `a3edca9b`, sole public parent
`274194d2`. Its isolated public projection passes all 93 native Android cases with
zero skips, release AAR assembly and lint. App CI now watches the shared native
production sources rather than unit-only fixture files. Hosted
[host](https://github.com/CodyBontecou/Thumble/actions/runs/39752869738),
[native mobile](https://github.com/CodyBontecou/Thumble/actions/runs/39752869804)
and [app](https://github.com/CodyBontecou/Thumble/actions/runs/39752869832)
runs are pending; the original artifact and forced red witness remain preserved.
The completed host/app snapshots above are passing evidence for their exact
sources, not inferred successes for this repair.

## Connected native surface handoff

Local `811d151`, public `73aab76d` (sole public parent `a3edca9b`, twelve verified
source paths) adds single-use adoption of the already bound neutral engine.
Both native clients preserve the exact admitted epoch and packet sequence through
real TLS/IPC binding. UIKit construction in an unattached container survives first
installation; removal disconnects. Android instrumentation injects a real touch
into the adopted surface and verifies exact neutralization on release/removal.
No credentials, grant keys or engine references cross React.

Local checks pass 77 Swift and 96 Android cases, both iOS SDK builds, AAR/lint,
eight Android app cases, three hosted UIKit cases, and the 55-case stack gate plus
both builds. The isolated public projection passes after the standard generated
fixture preparation. Its first unprepared run failed because fixture resources
were missing; that is not product evidence. The first new Android app run reached
the control and failed a string-versus-enum assertion; the corrected eight-case
run passes. Both failed logs remain hashed in the manifest. Hosted successor
checks are pending; no mobile connection UI or continuous renewal is claimed.

Prior public `a3edca9b` now passes all three host and all three native mobile jobs,
including Keychain/Keystore. Its Android app job `119737486934` passes; the iOS
app job `119737486561` was still running at the last observation.
