# Native phone control-v5 decoding seam

Strict internal Swift/Kotlin grant and receiver-progress decoders, one shared synthetic fixture. Existing clients still negotiate 3/4; no UDP phone network path or automatic fallback enabled yet. Null values require explicit parser opt-in; old control/enrollment codecs retain rejecting null by default.

Native checks: 59 Swift tests with all native-process witnesses enabled; 78 Android tests, zero skipped/failures/errors, AAR and lint; all three native modules compile for both arm64 iOS Simulator and iOS device SDKs; portable verify 74 passed. Four new cases per phone platform cover explicit UDP opt-in, exact grant fields/port bounds, no acknowledgment of unsent traffic, nondecreasing progress, scope/configuration binding, terminal inactive status and null/duplicate parsing.

This batch also applies the same 1750ms/1500ms transport fixture allowance to the revocation/reenrollment UDP ordering scenario on both activations. Windows e20c345 passed mixed UDP/TLS but the separate reenrollment ordering probe observed neutral before its held-state assertion. That scenario performs reliable control round trips without periodic samples; it must not conflate TTL expiry with ordering. It still requires native_input_released on explicit stop/revocation. Dedicated owner-loss and replay/350ms TTL tests retain original timing. Local all 12 real-process tests pass.

Standing user CI authorization applies; publish only explicit source/docs capsule. Private evidence/history, credentials and builds remain excluded. Hosted OS-provider execution pending for this source.
