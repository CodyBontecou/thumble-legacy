# macOS and physical phone review priority — 2026-10-22

The user superseded Windows-at-launch and entirely-headless execution: prioritize a local headless macOS receiver plus iOS and Android, test with the connected iPhone and Pixel through Argent, and prepare store review candidates. Windows/Linux follow while those candidates are in review. Optical live sync remains required. This does not authorize store publication or certify a release.

## Earlier frozen source

Private commit `e281df3` contains 69 curated files. The public source projection excludes the private rewrite plan and history: `/private/tmp/thumble-mac-phone-first68-public-source-20261022.json`, SHA256 `694c08a97a7825ec88ef35db043c700981f3d9142399e4de9fde2d41279d42e5`. Public commit `e53013cb505e9f031e67b00996aa9ab12ade9742` has sole public parent `2dee80c96704d939797dd2df976caca97fa0b93d` and verified tree `58e2e2a69b4a7e34721632daed32a45eae713329`. The projection adds explicit physical phone protocol v9, native output mode selection, a production 18-control starter operation through CLI/MCP, optional capture packaging, developer signing preparation, an output shutdown race fix and physical-device UI corrections. Later QR, orientation, store-profile and reconnect work described below are excluded from that earlier snapshot.

Isolated source gates pass generated contracts, boundaries, notices, formatting, all three TypeScript configurations and 303 tests / 5,063 assertions (two intentional opt-in skips). macOS-feature receiver tests pass 149 unit and 24 process tests. Standalone recording packaging passes 1 test / 67 assertions on unchanged recheck; the initial MCP sample-phase failure and an independent successful SDK debug probe are preserved separately. No production/test deadline changed. Full native SDK acceptance is recorded in the individual frozen capsules; this combined run is not a claim of physical touch acceptance.

## Actual devices and Mac

Both bundled Release phone apps build and install independently of Metro. The iPhone navigation flow passes 21 checks over 24 steps, including safe-area Done and explicit Mac-input versus recording intent. Pixel light-theme labels and outlines are corrected; navigation flows pass, but full dark-mode acceptance remains pending.

The old Mac lab owner was queried using its explicit state directory, released, verified neutral and gracefully stopped. Its authority/configuration remains preserved. The signed rewrite receiver started in recording mode with a separate private identity/runtime. The production starter operation committed revision 1 and the saved layout activated successfully. After recording pairing tool limitations were preserved, the service restarted in explicit macos-hid mode using the same private identity/runtime. Saved activation reports inputEnabled true, generation/revision 1 and a neutral report. An independent read-only IOKit/GameController probe observes exactly one supported Thumble HID device, one extended gamepad, all zero axes/triggers and released buttons. This is development signing and neutral enumeration, not gameplay acceptance.

Manual LAN invitation entry initially failed through the automation surface: landscape taps on the Pixel and a 30-second iPhone keyboard-return watchdog prevented submitting the native forms. Secrets were injected through server-side placeholders and remained outside repository/evidence. These historical tool failures did not establish an enrollment failure. Native QR intake and the explicit local CLI QR image export subsequently established pairing on both physical phones.

### iPhone: actual input and restart/reconnect

The real iPhone scanned the native invitation, paired through pinned TLS, authenticated explicit physical protocol v9, loaded the production saved 18-control starter and drove the development-signed Mac receiver. Independent GameController consumer captures record 35 and then 66 input changes, including actual controller controls. Live QA observed approximately 4.6 minutes of continuity. These are OS-consumer observations, not gameplay or a latency benchmark.

A subsequent full Mac service restart, in-place phone app upgrade and saved-trust Reconnect succeeded. Enrollment remained closed; no fresh pairing offer or credential replacement was required. The independent Mac consumer observed South input after reconnect. Two additional captures each contain two changes. This proves the exercised saved-trust reconnect path and current-authority resolution; it does not prove automatic controller rearming after every clean release.

The installed signed receiver used for these runs does **not** yet contain READY16 local auto-rearm, the first-termination diagnostic supplement or auth-only startup. Local activation/rearm steps in these runs remain explicit operator actions. Those new source policies are frozen for isolated validation and have not been accepted on the physical phones.

### Pixel: actual controls, unresolved disconnect

The real Pixel scanned the QR, paired, received the resolver upgrade and produced actual South and left-stick input in the independent Mac consumer: 60 changes in `pixel-consumer-diagnostic-active.json`. The consumer's final inventory has no remaining controller/HID device after the session ended; the recorded input history, rather than final enumeration, is the evidence for those controls.

The terminal error was logged 76,745 ms after the PhoneConnection object was created, including manual reconnect-dialog and automation time; this is **not an established-session duration**. Connected duration was not measured. The first Connected accessibility observation is at Unix milliseconds `1792712906647`; Android diagnostics identify `PROGRESS_RECEIVE` at `1792712955823`. Neither timestamp establishes when the transport became connected. The first causal failure is still being investigated. Neither source implementation, a successful short run nor iPhone stability resolves this Android result. There is **no physical LT/RT acceptance evidence** from that Pixel run, and sustained Android continuity/reconnect remains open.

### Private consumer artifacts

All paths below are relative to `/private/tmp/thumble-macos-device-beta-20261022/`. Keep the raw captures private; they are not source-publication inputs.

| Capture | Recorded changes | SHA-256 |
| --- | ---: | --- |
| `iphone-consumer-session-first.json` | 35 | `029227736f264469dc3dec9783cf1b4ec1e6bd73414ca549c3a298d92cf8cf4d` |
| `iphone-consumer-controls-second.json` | 66 | `1617804ec96e14ea37c37e9f3fe8f63d41619a4b8000b5b0a96a3a9649be3823` |
| `iphone-consumer-reconnect-third.json` | 2 | `1904e9ebcf77f1fd96739a00d0009a980eef076c70db56ef410378e1624a78e8` |
| `iphone-consumer-restart-resolver.json` | 2 | `fe8e98ff402537dd4b82ca67d7ef13988c02d4643569b9351b52af4c3b352c2f` |
| `pixel-consumer-diagnostic-active.json` | 60 | `0acf02f37234486fc5b143a6efc105c5e027814d325df3ff8614d9c1a330310d` |

Hashes and event counts were checked directly against the private JSON captures. Continuity, app-upgrade and diagnostic phase/timestamp statements are from the coordinated actual-device run record, not inferred from the capture observation-duration setting.

## Store profile and next frozen source

The explicit opt-in store profile is published in public source `68a` (eight curated files). The default `.rewrite` development identifiers remain unchanged. The store profile uses the retained production iOS/Android identifiers and icon, strict explicit review version/build values, phone-first settings, and excludes the native QA plugin/test harness. Its isolated five tests / 67 assertions, strict type checks and offline fresh development/store Expo configuration and prebuild inspections pass. No production app was overwritten by that prebuild review, and no archive, store upload, account change or release-signing acceptance follows from it.

Full retained stack-safety verification passed, including constrained-thread regressions and macOS/iOS build budgets. Private log: `/private/tmp/thumble-rewrite-store-stack-safety.txt`, ending `Stack-safety verification passed.`

The next combined 36-file reconnect/readiness source is frozen at `/private/tmp/thumble-m3-reconnect-ready-source-20261022.json`, SHA-256 `dc84ecf5d23476ef87b0e0880315fadc49345b901c1a454c2e457f27cd4ed099`. Root isolated validation is in progress; do not label the exact combined capsule accepted yet. It brings bounded operator-visible local saved-controller readiness, native first-termination evidence, explicit fixed-port auth-only startup and related reconnect work. Defaults remain no listener/no device until their separate explicit opt-ins. The presently installed signed receiver is older than these changes.

## Outstanding release gates

M1/M3 remain incomplete. Preserve the iPhone progress without upgrading it to four phone/game acceptance combinations. Android's first disconnect cause and sustained recovery, real trigger/multitouch/modifier input on both phones, background/lock/rotation/fallback/host-loss behavior, physical haptics, comparative latency/ergonomics, and both exact target game experiences still need acceptance. None of the OS-consumer captures establishes actual gameplay.

Optical live sync is still mandatory. The extracted capture helpers, framing, pixel/PNG validation and headless tests do not yet prove independently launched signed-helper permission, supported real game window/scaling, receipt freshness/frozen-frame rejection, authority refresh or physical-phone delivery. Those remain release gates alongside client/addon detection/setup and real Classic/Forever operation.

Store production signing/provisioning, iOS archive and Android release bundle inspection, review metadata/privacy/permissions, signed/notarized headless Mac packaging and clean install/update/removal remain open. Rich editor/journal/preview/migration parity and required recovery gates remain tracked by the rewrite plan. Development-signed HID and source-only store configuration do not certify distribution. Windows/Linux remain deferred until the Mac/phone review candidates are ready; their driver/signing/gameplay support is not claimed.
