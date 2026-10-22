# Native bootstrap metadata and Windows fixture — in progress

The frozen 11-file snapshot is local `b52ce852` (metadata `0758bff`) and public
`b41af1aaaaa2e7765daabce67bcf42eace08a2bb`. Its sole public parent is `54a3c816`;
all changed paths, modes and Git blob hashes match the frozen source. Private
history/evidence, credentials and build products were excluded.

`inputConfiguration()` exposes authenticated native configuration epochs before a
one-shot grant attempt. It returns no keys, bearer or geometry. The later binding
operation queries again, so a revision change cannot be admitted from a stale
inspection. Inactive/full-range counters, malformed replies, cancellation,
concurrent ownership, and inspect-to-bind changes are covered. Real native
TLS/native-IPC scenarios query metadata before input in TLS, UDP and blackhole
fallback modes.

Local results: 73 Swift and 92 Android cases (zero Android skips), both iOS SDKs,
99 portable cases, standalone packaging 1 test/41 assertions, two iOS hosted app
cases, five Android actual-touch app cases, the full Rust suite/Clippy and 12
isolated paced-fixture repeats. The retained stack script passes 55 constrained
cases plus macOS/iOS builds in 23.93 seconds. See the manifest for raw local-log
hashes and the sanitized witnesses here.

Windows diagnostics on public `54a3c816` showed the stale-grant case passing alone
and failing under concurrency at the status barrier after an unchecked producer
had queued input. The fixture now allows one input/status round trip at a time
and proves its first native ACK before challenging a second peer. Generation,
revocation and stale-packet assertions and production TLS/lease/TTL limits are
unchanged. This is a bounded fixture repair, not physical-input performance
certification. Earlier failures remain in the sibling app evidence directory.

Hosted results for this exact source:

- [Windows/macOS/Linux host run](https://github.com/CodyBontecou/Thumble/actions/runs/39735127322): all three pass, including the isolated and concurrent Windows native tests.
- [Native mobile run](https://github.com/CodyBontecou/Thumble/actions/runs/39735127446): all three pass, including iOS/Keychain and Android Keystore.
- [Bundled app run](https://github.com/CodyBontecou/Thumble/actions/runs/39735127408): iOS passes both hosted UIKit cases; Android fails during emulator wake/setup before the input witnesses. The preserved failure led to a wake guard, preboot APK build, and separate per-control Android capture regions. See [the successor evidence](../android-region-and-mcp/README.md).

App workflow dependencies now exclude receiver-only test edits, so future Windows
fixture work does not restart an unrelated iOS app build. The phone app still runs
local practice; host enrollment/binding, continuous renewal and launch acceptance
remain open.
