# Authenticated owned phone intent receiver

This isolated HEAD-based batch connects the staged canonical phone intent to the headless receiver. It does not activate format 5 in shipping mobile clients or migrate user data.

- Complete Rust gate: 612 passed, zero failed, three existing native-dependent ignored; formatting, clippy, target checks, frozen fixtures and CLI/MCP parity passed.
- Native gate: 41 constrained-stack tests and macOS/iOS Debug builds passed with unchanged budgets. The 131 native inputs match HEAD; no Swift change is part of this batch.
- Actual WebSocket process: temporary owned authority, native helper and receiver without local agent write opt-in. Checks unauthorized refusal, scoped commit, eight-state paint/resources, surviving declarations and binding ownership, precise metadata, helper-free replay, stale/changed intent rejection, historical/current revisions, agent-write refusal and receiver restart.
- Unit cases cover bounded queue backpressure, revoked peers, independent phone/agent authorization, held-input release failure without persistence, invalid/misplaced payloads, and unchanged legacy snapshot/input behavior.

`verification.json` records source, binary and log hashes. The receiver uses an isolated target directory to avoid concurrent-build artifact replacement; an earlier shared-target attempt was inconclusive and is not the evidence here. No running user receiver was stopped or user authority modified.

Reproduce with `scripts/verify-rust-host.sh`, `scripts/verify-stack-safety.sh`, then `python3 scripts/verify-owned-phone-wire.py --host /absolute/path/thumble-host --bridge /absolute/path/thumble-bridge`.

The source hashes identify the isolated commit content. The shared checkout also contains concurrent WoW, Android and gamepad work; those changes are excluded from this batch and its verification claim. Physical haptics, end-to-end latency, mobile journals, full format-5 transfer fidelity/bounds, automatic migration and actual skin/portrait/GUI removal remain outstanding.
