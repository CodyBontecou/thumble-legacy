# M2 receiver, service and CLI seam

Engineering scope: isolated development runtime, recording output only. Production input, phone transport, profile persistence and migration remain disabled/unimplemented here. The retained GUI/native authority is untouched.

## Ownership and protocol

`Rewrite/apps/cli` provides JSON lifecycle and receiver commands. `apps/service` owns ephemeral activation configuration through `packages/application`; the standalone Rust receiver owns native time, report encoding, active generation, replay protection, owner lease and held-input expiry. `packages/client` exposes a portable transport interface; its explicit `/local` subpath implements host loopback access.

The native request/response protocol is version 1 NDJSON over inherited private pipes, with 64 KiB frames, bounded queues and correlated IDs. Native request structs deny unknown/duplicate fields. The local channel has a separate strict JSON scanner rejecting escaped duplicate keys, malformed syntax and excessive nesting, then validates exact command shapes. Counters are canonical decimal u64 strings to preserve precision above JavaScript's safe integer range.

Activation requires the current configuration revision, allocates a greater generation/revision and new private session identity, releases the prior report, and commits only after native acknowledgment. Samples require the active generation/session and increasing sequence; each is a full semantic state. A new generation, explicit release, owner disconnect, expired owner lease, expired input state or output failure retires the session and records neutral output. Old packets cannot revive it.

Service defaults are a 750 ms owner lease, 350 ms input TTL and 150 ms renewal interval. Native safety ticks every 10 ms using sleep-inclusive platform clocks. Renewal does not extend stale input's TTL. Native stdout writes run separately behind a bounded queue so stalled JavaScript reads cannot block the safety tick. These intervals are development defaults, not accepted physical latency budgets.

## Isolation and security

The runtime directory is absolute, owned by the current user, and isolated with a version marker. Nonempty unrelated directories are rejected before modifying permissions. The helper refuses symlink/reparse ancestors. Unix state uses 0700/0600; Windows applies user/SYSTEM ACLs inherited by endpoint files. An exclusive native lock prevents competing receiver owners.

The service binds an ephemeral loopback port and requires a random per-start credential on every request. The endpoint descriptor is private, bounded and validated. Credentials do not appear in CLI responses, audit logs or checked-in evidence. Each connection accepts one bounded request, has a timeout, and the listener limits simultaneous connections. Errors expose bounded codes rather than raw paths or internal payloads.

The recording log contains only report bytes and release reasons. Status explicitly distinguishes recording/input-disabled, unevaluated OS permissions/game recognition, disconnected phone and ephemeral configuration. No new code edits a retained catalog, launches a desktop app, installs a driver or emits OS input.

## Test coverage and limits

The source integration suite exercises the actual CLI, detached service, authenticated loopback socket and standalone native process. It covers CLI exit survival; stale revisions; generation barriers; large sequence precision and replay; full-state repair; explicit release; restart neutrality; concurrent starts; unrelated-directory refusal; authentication; explicit recording opt-in; malformed/duplicate JSON; abrupt service death; and a 900 ms busy JavaScript owner while native expiry continues.

The kill test observes native `owner_disconnected`, neutral output and process exit within a 1000 ms test allowance. It checks owner-loss behavior on real processes; deterministic native clock tests separately prove exact lease/TTL boundaries. The observational allowance is not a claim of a sub-750 ms physical delivery bound.

Package tests copy the compiled Bun executable and sibling native receiver to a temporary directory outside the checkout, remove Bun from PATH, then verify start/status/stop and endpoint removal. Packages include the retained descriptor's MIT notice. These artifacts are unsigned and use recording output; the signed-helper M2 gate remains open.

Native mobile input-module skeletons/build CI, signed release identity, production HID/driver output and physical phone/game evidence remain outstanding. Windows and required optical live sync remain launch scope. Headless engineering continues while those beta/release gates are pending.
