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
