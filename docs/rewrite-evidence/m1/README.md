# M1 native receiver acceptance — incomplete

Read-only observations captured 2026-10-21 on macOS 26.5.1 (25F80), Apple silicon. Source baseline, runtime/compiler versions and artifact/descriptor fixture hashes are recorded in [M0](../m0/baseline.json). The final package was not rebuilt from that baseline.

Commands run against the existing isolated lab:

```sh
python3 scripts/verify-hid-signing.py '/absolute/path/to/Thumble Controller Lab.app' \
  --require-hid --bundle-id com.codybontecou.PocketPadMac --distribution development
swift -module-cache-path /tmp/thumble-rewrite-swift-cache \
  scripts/probe-gamepad-compatibility.swift 1
```

[Development signing verification](development-signing.txt) passed. [Independent consumer observation](consumer-observation.json) enumerates an extended Thumble GameController with neutral axes/buttons. The owning receiver's separate `gamepad status --json` also reported ready, no held buttons, and zero axes/triggers. That status was inspected without retaining timestamps/counters as performance evidence. No controls were pressed and no release or lifecycle command was sent.

This establishes only the current development artifact's signature/profile authorization, enumeration and observed neutral state. It does not establish a production Developer ID identity, a Windows driver, phone-driven transitions, game enumeration, gameplay, disconnection safety, latency, haptics or live capture.

## Remaining acceptance procedure

Use the pinned host/phone/client/addon builds and a separately recorded package/descriptor hash for **each of eight combinations**: macOS/Windows × physical iPhone/Android × Classic Era/ConsolePort and Forever/native. Confirm one input owner and preserve the existing user's state. Use a distinct test profile and isolated authority/discovery metadata. Do not run a second input-enabled receiver beside the installed owner.

1. Verify final native identity, entitlements/provisioning or Windows driver signature, installation permissions, publication, clean installation/removal and independent consumer semantics. A Windows keyboard/pointer `SendInput` result cannot substitute for a native virtual gamepad.
2. Confirm the game enumerates the actual controller; record the client/addon build and settings outside combat. Record Thumble's device separately from compatibility-layer/Steam synthesized devices.
3. Exercise both sticks, camera polarity/diagonals, face/shoulder/stick-click/menu buttons, independent held triggers and combined modifier states, targeting/interact/menu, disconnect neutralization, and neutral reconnect. Check the independent consumer and visible game; save recordings and exact procedure.
4. Measure touch-to-receiver and physical touch-to-visible-game separately, p50/p95/p99 and sample counts, with network/load/build/device metadata. Use validated clock synchronization or a common measurement clock. Freeze budgets during M0 before comparing rewrite results.
5. Repeat on final distribution artifacts, including Windows driver install/update/remove and macOS independently launched permission behavior. Development observations here cannot close that gate.

Required live sync additionally needs independently launched signed-helper capture authorization, supported scaling/window behavior, frame identity/freshness, frozen/replayed-frame rejection and delivery to each physical phone. Lost observation must mark presentation stale without interrupting physical input. Snapshot/manual refresh remains available. This is a mandatory M5/M6 launch gate following the user's scope decision.

`../m0/launch-contract.json` deliberately leaves all physical acceptance entries pending. Windows hardware/OS, its native driver, and its exact client builds still need to be supplied/established. The network-paired iPhones have no connected device tunnel in this session. The existing lab documentation remains historical evidence, not an M1 pass.
