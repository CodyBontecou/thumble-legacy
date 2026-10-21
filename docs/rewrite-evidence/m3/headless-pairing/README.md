# Headless native pairing owner evidence

Local source and standalone CLI tests now drive a real native listener without desktop GUI/IPC. An independent Node/OpenSSL peer validates the native certificate/name/ALPN/pin, enrolls and receives the required completion reply. Other tests cancel after hello, require explicit identity, reject configuration mismatch and kill the service during a stalled TLS exchange while verifying receiver neutralization and listener closure.

Native unit tests cover local-only defaults, exclusive owner locks, store/policy responsiveness, committed enrollment, bounded acceptance, stop and retired-offer coherence. Source checks pass 61 Rust and 73 TypeScript tests; standalone packaging passes 42 assertions. Real input remains disabled.

This newer host/service source is separate from frozen phone/vault candidate 8f216c9 and is not published. Hosted Windows evidence for this source, real OS-provider tests, sessions/routing/revocation/reconnect, phone UI and launch acceptance remain pending. Retained Swift stack-safety remains unverified after its earlier compiler macro-probe stall. No private baseline/history, real credentials or build products are included in these local evidence records.
