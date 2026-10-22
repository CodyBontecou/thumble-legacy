# Native phone app pairing and input — in progress

Local source `2b01f7d`, public `b862c619` adds the app's native pairing/reconnect
owner and the shared React connection route. The twenty-one exact source paths
were verified against the public base tree; the only public parent is `73aab76d`.
Private baseline/history, credentials, generated endpoint ports, images, evidence
and build products are excluded. Publication uses the user's standing CI
authorization, without a new approval prompt.

The native secure field is the only manual invitation entry. Its proof and vault
material stay outside React. The OS provider completes enrollment before the
control client authenticates, fetches the current receipt and binds/adopts its
neutral engine. View installation preserves that receipt. Configuration/practice
bridge calls cannot replace a live-mode owner, even after disconnect. Pending
setup closes on cancellation, removal or backgrounding, and admission requires
an active/key or focused attached view. Saved endpoint metadata supports a fresh
reconnect; it is not a second credential store.

All ten Android app cases pass. The new scenario operates the actual connection
route, enters the native password field, saves through real Keystore, injects
system touches, witnesses held input at the recording receiver and uses a second
finger on Done. It then reconnects with saved endpoint/OS trust on an advanced
receipt and starts neutral. All five UIKit cases pass with actual Keychain and
TLS/IPC; its corresponding scenario calls the same native submit/reconnect
methods used by alert actions and drives semantic input, rather than injecting
iOS system fingers. Both scenarios verify first-recorded and current neutral,
held state, and native release twice. Exact result checkers reject skips or
missing cases.

Other local checks pass 77 Swift and freshly rerun 96 Android SDK cases with all
four real network probes, 107 portable cases, standalone packaging (1 case/46
Bun assertions plus all four Node SDK scenarios), typechecks, formatting,
boundaries, generated vectors, Rust probe build/clippy and the 55-case retained
stack gate plus both builds. The portable runner excludes one package case,
which is separately required and passing. The isolated public projection passes
typechecks, Prettier and Rust formatting.

Failed attempts remain explicit in the manifest. A native-button text lookup
was replaced by stable Android resource IDs. The app's sequence-zero observation
was unsuitable once its native frame sampler advanced; the app witness now
checks the first recorded source sample, while the SDK's original sequence-zero
check remains unchanged. One repeat lost ADB during rotation; the same emulator
recovered, the incomplete run was rejected and the next complete run passes.
Owned port cleanup now preserves the primary failure and verifies already-absent
forwards. The first iOS recording-host failure's exact phase was not captured;
subsequent runs have explicit phase receipts and no precise initial cause is
claimed.

The private light/dark PNGs show the actual UIKit alert over the hosted QA
Home/test surface. Text is legible without clipping, and native entry is masked.
They are not product connected-route screenshots, motion/latency proof or visual
launch approval. The QA-only ad-hoc simulator identity enables Keychain without
configuring production device signing or changing vault policy.

Latest CI is [host](https://github.com/CodyBontecou/Thumble/actions/runs/39761455119),
[native mobile](https://github.com/CodyBontecou/Thumble/actions/runs/39761455105) and
[bundled app](https://github.com/CodyBontecou/Thumble/actions/runs/39761455025).
Exact job outcomes are tracked in the manifest. Discovery/QR, continuous renewal,
full authority/editing/MCP parity, haptics, migration, output/signing, gameplay and
mandatory optical live sync remain open. No acceptance milestone is complete.
