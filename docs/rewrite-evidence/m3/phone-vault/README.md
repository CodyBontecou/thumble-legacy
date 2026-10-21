# Native phone credential vault evidence

Local SDK builds, native policy tests and real JCE encryption tests. The fixture uses synthetic public credentials. See `verification.json` for counts and Android build-product hashes; build products themselves are not tracked.

Actual iOS Keychain and AndroidKeyStore runtime tests are compiled, not executed. Native phone enrollment-client wiring remains pending. This source is newer than approved all-green enrollment snapshot 9e2f643 and is not published. Retained Swift stack-safety remains unverified after its earlier compiler macro-probe stall; the isolated credential's immutable storage/inline-size test is separate coverage.
