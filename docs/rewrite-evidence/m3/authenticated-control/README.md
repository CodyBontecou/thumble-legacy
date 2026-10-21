# Local authenticated control checkpoint

Control version 3 authenticates an enrolled phone directly in native TLS, producing an opaque authorization bound to the exact native server channel, host and credential epoch. Status/disconnect recheck durable trust, expiry and clock; no input grants exist yet. Version 2 enrollment is preserved. See the protocol document for exact messages and limits.

Local verification passes 67 Rust tests, 74 TypeScript tests (916 assertions), all-target Clippy, and standalone packaging (one test, 53 assertions). Independent Node/OpenSSL enrollment and authenticated reconnect run through both source and compiled CLI; compiled CLI runs outside the checkout without Bun on PATH. Native tests cover revoked/wrong credentials, retired/mismatched epochs, connection ID substitution, exact socket/client/closed-channel binding, expiry/rollback, strict fields, 128-operation budget and disabled input grants.

The rebuilt Rust TLS/enrollment witnesses also pass 36 Swift tests and a forced fresh Android unit run: 55 tests, zero failures/errors/skips. Gradle initially reported UP-TO-DATE; that result was replaced by `--rerun-tasks` execution. These are version-2 phone compatibility tests, not production version-3 phone clients or OS-provider execution. No Swift source changed, and no retained stack-safety pass is claimed.

This remains local newer source; no Windows/hosted result is claimed. The frozen 25-file phone/vault candidate was revalidated against all manifest hashes/byte lengths without modification. Production native phone authentication, receiver session grants/packet routing, real input, UI, optical sync and launch acceptance remain open. The full rewrite goal stays active.
