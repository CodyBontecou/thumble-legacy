# Native phone enrollment clients

Local tests execute Swift Network.framework and Kotlin/JDK TLS clients against the synthetic native Rust enrollment coordinator/SQLite ledger. Native storage policy sinks verify save-before-ack; separate Keychain/Keystore tests compile but require hosted OS-provider execution. Control protocol v2 adds required final completion; old control v1 peers fail before proof. No product listener or real input is enabled.

The private logs preserve 56 Rust, 36 Swift, 55 Android and 68 TypeScript tests, standalone packaging, SDK/test-bundle/APK builds, Clippy and workflow verification. Provider-result verifier tests pass normally and under optimized Python; missing/skipped/empty provider evidence fails.

This source and the previously local phone vaults are newer than approved all-green 9e2f643. Publication requires a separate scoped source snapshot, excluding private baseline/history, real credentials, build products and all local evidence. Actual provider tests, CLI forwarding, session/reconnect integration, phone application, physical gameplay/signing and optical acceptance remain pending. Retained Swift stack-safety remains unverified after its earlier compiler macro-probe stall.
