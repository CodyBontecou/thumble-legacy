# M3 bundled mobile practice — in progress

Source: local `5c8675d86bba54a70d6ae686d7f2d402d214ef4c`, public
`5d70e1fd29592f2a766df56b593e223b3c45d21e`. All 39 frozen paths/modes/Git blobs
match the published tree. Its sole parent is the existing public `6f82b437`;
private baseline/history, evidence, credentials and build products are excluded.
Standing user authorization covers scoped CI publication without further prompts.

Local validation: 99 portable cases (plus a separate packaging opt-in), standalone
packaging 1/41, five Android actual-touch cases twice, two iOS UIKit hosted cases,
and the iOS navigation/rotation flow twice (18 passed/0 failed/0 skipped each).
The retained stack gate now passes 55 constrained-stack cases plus macOS and iOS
builds; this supersedes the earlier unresolved local macro-probe attempt.

The Android ancestry witness failed against the original React-child surface.
Subsequent real-device tests exposed a Fabric native-tag registration race and
framework-added event fields; native prop/receipt admission fixes the former and
a strict nested receipt fixes the latter. A QA-only iOS explicit dependency scan
stalled; the sampled failure is preserved and the QA target uses normal implicit
module compilation. These are fixes driven by observed failures, not claimed
passing evidence from the earlier attempts. An overlapping local instrumentation
attempt also produced a UID mismatch; final runs were serialized.

Hosted validation is pending:

- [Host/Windows/macOS/Linux](https://github.com/CodyBontecou/Thumble/actions/runs/39731960042)
- [Native SDKs, Keychain and Keystore](https://github.com/CodyBontecou/Thumble/actions/runs/39731960118)
- [Bundled Android/iOS app](https://github.com/CodyBontecou/Thumble/actions/runs/39731960295)

The manifest records exact source and raw local-log hashes. Sanitized witnesses
are committed here; verbose local logs remain outside public source. Practice is
recording/local feedback only. No native app host enrollment, transport binding,
grant renewal, production signing, gameplay, phone-size/visual acceptance or
optical live-sync acceptance is claimed.
