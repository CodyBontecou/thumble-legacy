# M3 native authenticated input packet slice

The native phone engines now have a bounded encrypted output adapter, and the Rust receiver library has authenticated packet admission through its existing native safety runtime. No LAN listener, trust enrollment, production OS input or full M3 acceptance is enabled.

[ADR 0002](../../../adr/0002-rewrite-input-packet-v1.md) fixes the 108-byte v1 wire layout: a 40-byte authenticated session/generation/sequence header, 52-byte normalized semantic state and 16-byte AES-256-GCM tag. The sender owns nonce progression and a bounded packet budget. Keys must come from a fresh authenticated native grant; JavaScript never owns a sample queue or encryption counter. The fixed semantic bit order is checked during generation so enum changes cannot silently alter the protocol.

## Implemented and tested

- Swift CryptoKit, Android JCE and RustCrypto match **33 exact packet vectors** (32 normalized retained report states plus one all-button/fractional state). A separate actual **Node 26.8.1 / OpenSSL 3.6.4** verifier reproduces the ciphertext, tag and header bytes. Public deterministic fixture keys are not credentials.
- All three native implementations reject tampering at each of **108 byte positions**, replay/reordering, wrong keys, invalid lengths, unsupported header semantics, stale session/generation, exhausted sender budgets and terminal close. Five correctly authenticated malformed payloads cover unknown bits, nonfinite/out-of-range axes, negative triggers and short payloads. Rejected packets never advance replay state.
- Native `InputPacketChannel` adapters connect directly to each native touch engine output callback. They validate generation/contiguous sample sequence, serialize encryption and reject reentrancy, sink failure and sequence gaps. Actual touch-engine tests show encrypted neutral/held states and neutral local disconnect on transport failure without replay.
- Rust `PacketRuntime` admits decrypted states through the existing safety owner. Tests cover full-state repair after a lost release, replay not refreshing held-input TTL, a **350 ms input TTL**, independent **750 ms owner lease**, delayed old generation/renewal rejection, immediate explicit release, neutral restart, output failure, clock rollback and a **ten-minute grant cap** despite continued input/owner renewals. Stale/invalid configuration preserves the old binding; a failed activation barrier retires it.
- Native dependency notices are included in development packages and bound to the exact Cargo.lock hash. The source notice file includes build-time and platform-specific dependencies and retains required Unicode/BSD/copyright texts. The build refuses a changed lockfile until notices are refreshed.

## Verification

- **15 Rust tests** passed; format and Clippy with warnings denied passed.
- **13 Swift test functions**, including **15 parameterized touch scenarios**, passed. Generated state, control-map and packet-grant inline-size checks pass. All three modules compiled against iPhone and Simulator SDK 26.5, targeting iOS 17.
- **28 Android JUnit tests** passed; Android SDK 35 AAR compilation and lint passed.
- **65 TypeScript tests** passed, one package test intentionally skipped in that phase. The separate standalone package test passed outside the checkout with Bun removed from PATH. Strict host/portable types, generation/fixture drift and formatting passed.

Logs, JUnit, SDK build hashes and [verification.json](verification.json) are adjacent. The independent Node oracle is also wired into the proposed portable CI workflow. The initial Android channel test used the wrong signedness for a touch ID and was corrected. aes-gcm 0.11.1 resolved a dependency needing Rust 1.89; patched 0.10.3 was selected for pinned Rust 1.88. Generated fixtures use exact deterministic drift checking rather than Prettier ownership.

## Remaining work and evidence limits

The prior approved pairing source is published as `160418187157fea978a9368f12cf831c16168cb6`; Windows/macOS/Linux and unchanged iOS/Android CI passed. That source excludes this newer native packet implementation. [ci-publication-candidate.json](ci-publication-candidate.json) records the exact new 28-file source/notices/workflow/protocol candidate for a separate scoped approval. It excludes private baseline/history, actual credentials/session keys, caches/build products and local evidence logs.

Pinned native TLS enrollment, transactional trust and secure credential storage, discovery/manual invitation construction, actual UDP/reliable socket adapters, routing changes and reconnect orchestration remain unfinished. The codec/channel and native receiver admission tests do not prove socket scheduling, physical latency, RN-stall behavior on a phone, game recognition, production signing or protocol security review. Retained stack-safety verification remains open after the previously recorded local Xcode compiler-probe wait; retained model tests/budgets were not weakened. Required optical live sync and all later rewrite gates remain open.
