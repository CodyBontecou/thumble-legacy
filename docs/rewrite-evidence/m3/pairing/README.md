# M3 pairing window foundation

This slice implements the authority's local pairing window and portable client through the existing authenticated same-user service IPC. It preserves the recording-only receiver and retained owners. It does not complete phone enrollment or controller M3 acceptance.

The [protocol decision](../../../adr/0001-rewrite-pairing-and-input-transport.md) separates a pairing window, committed trusted phone and leased input session. It records high-entropy out-of-band bootstrap, host pinning with native TLS 1.3, later UDP/reliable input sessions, independent version negotiation and the legacy wire boundary. The native endpoint must bind the invitation to the host pin/address before a phone can use it.

Implemented behavior:

- One pending 120-second window per authority instance, five failed proof attempts, one-use admission, explicit replacement/cancel, expiry and rollback rejection. Restart loses unfinished windows. The service generates 32 random secret bytes, retains a domain-separated SHA-256 verifier in memory, and uses constant-time verification.
- CLI `pairing open --reveal-secret --json`, `pairing status --json`, `pairing cancel --json`, using the service's existing private runtime directory. Secret issuance requires explicit CLI opt-in. Status, endpoint metadata and native recording logs contain no bootstrap material. Replies explicitly report `networkEnabled: false`.
- Strict immutable portable request/reply validation and a portable pairing client bound to the expected authority instance. Secret-bearing status replies or mismatched authorities are refused. The shared application module has no OS, Bun or filesystem imports.
- Admission is an application precondition for the future native TLS trust transaction. There is no public IPC admission command, LAN listener, trusted phone record or newly authorized input. Cancellation/replacement of an unfinished window is not revocation of an existing trusted phone.

## Verification

62 source tests passed with one intentionally skipped package test in that phase; the separate standalone package test passed with 22 assertions. The latter copies the compiled CLI and native receiver outside the checkout, removes Bun from PATH and exercises service lifecycle plus pairing issuance/status/cancel. Strict host and portable typechecks, generation drift, boundaries, formatting and frozen installation passed.

The tests cover proof replay, all five failure attempts, a correct final allowed attempt, exact expiry boundary, replacement, cancellation, restart, entropy-generation failure, invalid/rolled-back clocks, verifier domain separation, malformed data/accessors, authority mismatch, secret-output opt-in and actual same-user IPC authentication. The existing service-death, JS-stall neutralization and receiver generation/sequence tests continue to pass.

[verification.json](verification.json) records development artifact hashes and outstanding work. [ci-publication-candidate.json](ci-publication-candidate.json) lists the exact 15-file public CI candidate. The user approved this exact slice and it was published at `160418187157fea978a9368f12cf831c16168cb6`. [Portable run 39664550715](https://github.com/CodyBontecou/Thumble/actions/runs/39664550715) passed Windows, macOS and Linux, including all six native tests, 62 source tests and the standalone package test. Both unchanged mobile native jobs passed in [run 39664550915](https://github.com/CodyBontecou/Thumble/actions/runs/39664550915). [github-actions.json](github-actions.json) records the exact scope, terminal Windows summary, job steps and artifact digests. These results do not certify the subsequent local encrypted packet implementation. Local evidence, private baseline/history, credentials and build products are excluded from that candidate.

Retained Swift/native sources were unchanged in this slice. The previously stalled retained stack-safety verification remains open; no new pass is claimed. Native TLS enrollment, secure credential storage, durable trust, discovery, UDP/fallback, React Native application wiring and physical acceptance remain required before M3 completion. Windows remains a launch target; its hosted engineering regression passed while production and physical acceptance remain pending.
