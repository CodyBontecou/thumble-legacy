# Saved-phone reconnect and Mac readiness: isolated validation

Public commit: `03e03f8df91af9d7782ecc2c627017e2a80dc0a4`, tree `3580e04c1bc088cb26b0ed6a7fc4485774451d40`, sole public parent `68a0bfd67706a6b79bb3e4bf1fbf7a5e0e42dc29`. Curated 37-text-file capsule SHA256 `9925e2716ed05c6fd20f4c21736439bb16b693769cb31a4f236f9ce15da37df2`. All public blob hashes/modes and exact recursive tree delta checked; no deletion, private plan, private baseline/history, credentials or builds published.

Private commits `a774c00` and `ace6292` preserve the reviewed frozen files while unrelated live source remains unstaged. Independent review found no safety/credential boundary blocker; its production transport-category finding was fixed and covered through the actual adapter before publication.

Isolated projection `/private/tmp/thumble-mac-phone-first58-proof` with independent offline dependencies: full generated/boundaries/notices/format/all-three-types and 324 TypeScript tests / 5285 assertions pass (two opt-in skips). Mac-feature native receiver 155 unit +24 process tests pass. Standalone CLI/optional capture/QR build and relocated package 1 test /67 assertions pass. Android production-category correction 135 SDK tests pass /11 opt-in skips, AAR/lint and bundled Release build pass.

The initial sandboxed runs lacked loopback socket/cache permissions and failed; those logs remain. Authorized reruns pass without changing source, deadlines or guards. An initial build invocation used unsupported option spellings and was corrected to documented build flags. Initial isolated signing could not locate retained signing verifier; the unchanged project script was then run with its required verifier in place.

New separate signed development bundle manifest SHA256 `ae6d9a06bb438c9a13d149f0d3d092194af30e3eaa6406265b78913c2abb86ff`. Previous neutral owner stopped before new owner launch. Persistent identity and fixed port retained; enrollment remains closed. Explicit saved activation arms optional maintenance, and read-only firstTermination is initially null. Physical startup is development-only, not production signing/gameplay certification.

Logs: `/private/tmp/thumble-m3-reconnect-ready-{verify-authorized,native-authorized,build-authorized,package}.txt`; signing `/private/tmp/thumble-m3-ready-development-signing-recheck.txt`.

Next actual Pixel trace uses in-place APK upgrade SHA256 `4952cf35fb07e6cd309c1820bccfd12d0fdc586aff81bd63ef38b1cf90684d7a`, independent 180-second GameController capture and bounded read-only readiness changes. No result or sustained reliability is asserted before this run completes.

Actual Pixel trace: precise Reconnect tap `1792714036193..1792714036271`; Connected accessibility observation `1792714038989`. Phone terminal `1792714049499` reports `PROGRESS_RECEIVE`, fixed `READ_FAILED/TIMEOUT`, `DELIVERY_CLOSED`, secondary checkpoint `CLOSED`, no lifecycle origin. Timeout category is real, but the shared TLS operation category does not yet distinguish a send watchdog from receive timeout; a narrow supplementary diagnostic is in progress. No transport/root-cause fix is asserted.

Independent capture `pixel-consumer-first-cause.json` completes at `2026-10-23T00:09:58Z`: 79 interpreted events, South and left stick (maximum leftX .704899, leftY .005539), all final controls neutral. LT/RT remain zero despite attempted gestures and are not accepted. Host readiness changes to `armed/clean-release` at `1792714051692` with a fresh neutral generation, demonstrating the optional clean-retirement rearm path in this run. It does not establish sustained Android reliability or input-expiry fault recovery.

## Reliable fallback bottleneck identified

Production activity APK `c3f2d1bc2cefe07f7881ccdc815f86b3d47921d4adf0c6f731f2b3a7bf230017` reconnect tap `1792714457148..1792714457236`, Connected observed `1792714459752`. One South hold followed by 90 seconds without UI automation remains connected at `1792714569827`. Later LT/RT vertical gestures are interpreted: independent 180-second capture `pixel-consumer-tls-activity.json` has 195 events, South, LT maximum .80392158 and RT maximum .42745098, final neutral. This proves partial trigger response/release, not full-range/multitouch/gameplay.

The same session later times out at `1792714611474`: first TLS operation RECEIVE/DEADLINE, 108 control writes completed, 107 replies received, last reply age 2001ms, last write age1900ms, queue depth0. Thus the passing 90-second interval is not sustained reliability acceptance.

Existing debug-only host IO instrumentation was enabled in a separate signed development helper (manifest `8dd070f810f71e60861291e91eb04f276ab4a9a5460a38b2cf9e7bb539c78bd4`) through a private wrapper that redirects only helper stderr. No RPC/input/deadline source behavior was changed. Same identity/port, pairing closed, explicit neutral saved activation. Exact reconnect `1792714914187..1792714914261`; phone timeout at `1792714928306`, about14.1seconds later. All17 control writes complete,16 replies arrive, first TLS operation RECEIVE/DEADLINE.

Host aggregate terminal metrics disprove the proposed authorization-mutex-starvation cause: controlWait20 calls/max5us/total31us; inputWait1135/max28us. Instead1135 reliable TLS input frames consume13,082,597us of inputHeld work, and1166 native IPC receives consume13,339,447us (roughly11.5ms each). The sender writes full-state samples faster than the native receiver consumes them; status requests can queue behind input already written into TCP. A bounded latest-sample application queue does not bound that TCP backlog. Source `host-debug-native-stderr.txt`, derived fixed aggregate JSON `host-debug-metrics-derived.json`, phone fixed-tag log `/private/tmp/thumble-pixel-resolver-physical-diagnostics.txt`.

The Android lane is implementing an acknowledged in-flight input window plus one coalesced newest unsent sample. Existing same-grant progress validation must control credits; stale/forged/future acknowledgements cannot grant credits. UDP and TLS share sequence numbers, so legitimate UDP progress must be checked against overall offered sequence, not only TLS-written sequence. Renewal packet-zero barriers, close cleanup,350ms input TTL,2s transport deadlines and authentication remain unchanged. No fix acceptance is claimed until the new source and actual phone runs pass.

Private commit `1efe50e` additionally freezes store-review9B + PNG binary attributes + six-file TLS activity diagnostics. Exact isolated source passes327 TypeScript tests/5313 assertions (two opt-in skips), all three type configurations, format/generated/boundaries/notices, and six Python metadata tests. Public head is still `03e03f8`; this newer source is awaiting the coherent flow-control publication. Two expired task QR entries were removed from Argent secrets while preserving unrelated entries and permissions.


## Acknowledged input windows and release receiver follow-up

Curated public head `8f860cd595684e671ab3afec7bd3273a818eb4de` (sole public parent `66a09d501a112901a0dd1c78c64dd3aa3a6cb8db`) adds the reviewed 45-file batch: WoW owner composition33, truthful status4 overlay, Swift ACK8, and one Android synthetic-clock setup repair. The exact combined source capsule SHA256 is `77c888661f11996125e2cff3834db2a435f8422e083b3613d50e4cc360c5995f`. All45 Git blob hashes/modes and the complete recursive tree delta were verified, with no deletion or private baseline/history publication.

The Pixel APK SHA256 `f0973878a759105a30d8563253c558ea5c9ab62ced8724d45fd45a216900e677` passes approximately255 seconds against the same original slow debug receiver, including full LT/RT and clean Done. Independent consumer captures contain two South events and33 trigger events, reaching1.0 on each trigger and ending neutral. This isolates the acknowledged dispatch fix from the separate receiver wake optimization.

Against the signed release wake receiver SHA256 `ee2ae7601a273dc81030b525bcf2e6865160b2e07f247e4e4382910fe14cf308`, stored Reconnect begins1792716974676..6974745; Connected is observed1792716977469, then freshly confirmed1792717454884 (about480seconds). Post-soak sequential LT/RT/left-stick/right-stick gestures run1792717494320..7499506; Done1792717501854..7501925, clean detach1792717502310 with no diagnostic failure categories. Independent180-second initial capture contains two South events; a separate180-second tail contains134 interpreted events, both triggers and both horizontal stick axes reach1.0, all axes/buttons end neutral. Home is confirmed but its actual rotation is landscape; portrait restoration is not claimed for this run. No simultaneous multitouch, physical latency or gameplay acceptance is inferred.

The unchanged receiver security/input deadlines remain enforced. Android global8 dispatch credits (including active write), one coalesced latest state, strict progress acknowledgments and control/renewal barriers bound reliable input. The receiver now receives coalesced native wake notifications while retaining its safety tick. Swift parity passes132 native tests, including both75-second real renewal modes, both SDK module compiles and independent read-only review. Its physical run is separate and still pending in this record.

Exact isolated projection gates:359 TypeScript tests, three explicit opt-in skips,5,607 assertions; generated/boundaries/notices/format/allthree types pass. Actual retained helper synthetic refresh:12 tests/133 assertions. Standalone relocated package:1/67. Public-only full retained stack gate:30 constrained-thread tests, macOS and iOS Debug frame budgets pass. Private retained test extensions and artwork were not copied into the public test source.

Public66 CI exposed one remaining Android test advancing its fake35-second clock before initial packet-zero dispatch; reproduced locally red, repaired with a test-only dispatch barrier, allfour renewal cases pass. Public66 iOS native tests/real-service witnesses/bothSDK compiles pass, but isolated runtime fails progress after8,080ms; public8f includes Swift backpressure protection and is undergoing exact local/remote runtime verification. Neither failure is represented as release acceptance.

WoW composition is source/helper acceptance only: the real retained helper consumes synthetic verified PNGs and repeated A/B/A refresh preserves18controls, opaque numeric spelling and explicit empty binding maps. Capture permission, exact game-client optical observations and physical presentation delivery remain open.


## iPhone ACK parity physical follow-up and isolated runtime

The signed in-place iPhone development binary SHA256 `2773a623d14ab1db016c7149725ba196a9394de05215ea6c69d650992c3d73cd` preserves stored trust and reconnects without another QR. Reconnect is observed1792717774959; the150-second untouched interval after the last representative gesture finishes1792717963297. Fresh Connected AX is observed around01:13:49Z (about254seconds after Reconnect); right-stick and remaining sequential D-pad/shoulder/face/menu/stick-press controls follow. Done1792718087644 returns Home. A separate polished34-step full replay passes with no failure/error/skip. This is sequential control/navigation evidence, not simultaneous multitouch or game/latency certification. A later clock-closure cleanup supplement was not part of this installed binary.

Independent initial/tail GameController captures contain46 and133 interpreted events (179total). Both triggers, both horizontal stick axes and both D-pad axes reach full magnitude1.0. Tail records all ten expected buttons (south/east/west/north, both shoulders, both stick presses, Select and Start), all axes/buttons end neutral. Private bounded evidence capsule SHA256 `b2e1a1ffb58c6669f37caa4b32f95c6bd17afa3b1c55518dc96757015e5a3f1f` preserves the timeline and limits.

The exact isolated public45 source passes its owned native iOS runtime:75,139ms, generation2, finish, maximum publication gap16ms, two held/fresh/neutral phases and one verified renewal watch. The watch runs75,049ms with two rotations,4,494samples and zero neutral pulses; native exit0 and owned cleanup pass. Public8f remote native Android/AAR/real-service witnesses/Keystore pass, but native iOS has an intermittent renewal-UDP hold loss and isolated runtime progress/read-failed. Local success does not certify those remote runs. Bounded debug static terminal outcomes are being added to investigate without altering deadlines.

Public8f full retained Stack Safety, dormant output, macOS optical helper, pairing QR, Linux portable and both secondary Windows shards pass. Windows primary shard exposes a test file-URL decoding issue; optional Windows optical packaging reaches its assertions then fails removing the still-mapped temporary executable. Narrow test setup/confirmed cleanup repairs are being prepared. Legacy Rust host CI independently fails its pre-existing shared-build capability-ledger coverage; no legacy parity claim is made.


## Read-only phone presentation composition

Public `7b0716f0371963e437f5e55b45df2ee01d569d20` tree `3817b19893a6a73ff272ca9320e06574ae5f6047` adds the exact47-file batch (capsuleSHA256 `b4ddaf19bd1a859f890c7d352b393453a7e664f9a322752a06ab80e72fe25c33`). Public `567bfe42452d9c2609cb68ed25f4227a3eab3dff` tree `bee2473942e9176da95262647fa43c28c623d4c8` adds exact19 nativeSDK/app callback leaves (capsuleSHA256 `4c3e9e043595ae69942b7e4fa712f2897281e06ece1fbc0dfc94c54d09ea31b7`). Each commit has only its previous curated public head as parent; complete recursive deltas, no deletion, all Git blob hashes/modes verified before CAS publication. Private commits `dba1fe3` and `118ff00` preserve the same frozen source in the dedicated worktree without staging working supersets.

Role10 is a separate authenticated read-only management channel, leaving strict inputv9 untouched. It checks the old installed native binding before/after a trusted configuration-child lookup. The child remembers only successfully served maps (max8/512controls), compares current normalized input semantics before offering a newer presentation revision, and scopes each verified32px PNG to current source/revision/control/hash/theme. Metadata is bounded32KiB/64controls, icons8KiB with16KiB reply, max65sequential requests, maxone authenticated presentation connection; SDK closes each batch rather than parking workers. Missing/stale/changed semantics return fixed cosmetic errors without reactivation/input retirement. Freshness requires applied observation ACK plus a matching advancing live identity.

Swift and Kotlin native clients verify pins, role/auth order, canonical base64, SHA/PNGCRC/chunk/inflation bounds. Foreground/theme polling every2s uses a separate management task, bounded icon cache, and currentconnection/map/viewport gates before callbacks. React validates installedmapSHA/gen/rev, IDs and presentation order; late icons/errors never change input ownership. It marks absent metadata stale after6s of local monotonic time; this is a UI bound, not proof of optical freshness. Independent review found and fixed map-before-ready pin loss;21React tests/141assertions now pass.

Exact isolated combined source:377TypeScript tests/5,812assertions,3opt-in skips; allgenerated/boundary/notices/format/types pass. Host actualhelper+strictPNG4/93; native role10 149unit/25process and focusedrelease4 pass; diagnostic static-symbol test1 passes; nativeClippy default/Mac pass. KotlinSDK155pass/11explicitIOskips+AAR/lint; SwiftSDK145tests including both actual75srenewal modes +bothSDKmodulecompiles; both release apps build. Fullpublic-only retained stack script rerun afterSDK19:30constrained-thread tests and Mac/iOSDebug frame budgets pass. Newcallbacks/actualchanged-icon service/UI delivery remains under unattended verification.

Full isolated Swift realprobes initially had3missingphysical-v9resource failures due root omitting mobile:prepare; all other130cases including both75srenewals passed. Generating existingapprovedfixtures and rerunning onlythe3blockedcases passes3/3. This setup red is preserved and is not a product fix. Optional relocated optical package1/50 passes with explicitlyselectedsynthetichelper; realcapturehelperneverexecuted inthat test.

Newdevelopment macOS package manifestSHA256 `117a91abc1b9adddc991bd6175b6738a903dbdb62731abedc85ac006e64213a1` reuses the existing authorized identity/profile, no account/provisioning/install changes. Priorowner6190/PIDs77918/77920 stoppedandendpointremoved before newauthority34177211-4435-4ca5-b297-5cf3b79a60c2 starts(service26295/native26297). Newclient reports invalid_service_response foroldstop response schema afterstop executed; independentPID/endpoint checks confirmcleanexit. Sameprivatepersistentidentity/trust andcontrolport56249, enrollmentclosed. Explicit savedactivation ofunchangedstarterrev1/hash05d60d…e666 establishesneutralgen1/rev1. Helperlaunch paths are explicit; capture/sync is not started automatically.

User deferred game/login testing while asleep and asked autonomousimplementation tocontinue. InstalledEra addon has autoconfiguration sideeffects onworldentry. A twoflag optical-only replacement, exactbackup/hash and guardedrestore are staged privately, neverapplied. No game launched, account/session/SavedVariables read, addon installedorchanged, or actualgamecapture performed. /thumblelive preference persistence is separatelydisclosed. Optical/gameacceptance remainsopen.


## Unattended presentation acceptance and CI diagnostics

Public `6e249e4dff30ca6b005adccd97096f4f4f0c3d7a`, tree `838833e9a5457be26a75aa174dfd91373fbbfd9e`, has only public `567bfe4` as parent. The frozen 22-file capsule SHA256 is `810349ba9f53b8a2e8b2ba45d906dbf46ad3b4740ec6cf449c4b15faf94e2dc3`. Complete recursive leaf delta, Git blob hashes, modes and absence of deletion were checked before compare-and-swap publication. Private source commit `6149f46` records 21 matching leaves; the public-only legacy capability ledger intentionally remains outside the private index because the retained private CLI differs.

The actual Kotlin SDK/service witness passes both UDP and forced TLS fallback. It imports synthetic icon data through normal CLI/CAS, retains the installed native input pin, observes refreshed metadata and verified PNGs, rejects stale CAS and old icon requests, and holds South for at least 1.5 seconds with zero neutral pulses under the unchanged 350ms TTL. It then releases input and checks revoked-phone authentication stops. Operator revocation intentionally releases input globally and is not represented as cosmetic-only. Root independently reran the actual witness, its fixture test and all TypeScript configurations. The required CI step admits exactly one named case, no failures/errors/skips, and both internal transport modes.

The actual Android app witness also passes one exact case with no skips: mounted React Image pixels change between synthetic initial and refreshed icons while OS-injected South remains held, then Done produces neutral output. A private screenshot was persisted and visually inspected. Native uiMode transitions to night, but the visible React palette remains light; dark palette acceptance stays open. iOS saved-controller tests fail before enrollment: fixed DEBUG enum diagnostics report idle admission, and XCTest invitation entry retains the placeholder instead of the invitation. Even the previous unsupported-route green cannot establish geometry rejection until entry is repaired. No new iOS visual acceptance is claimed.

The Android prepared-renewal fixture race was deterministically reproduced before any fake-clock advance: legitimate progress before initial input caused the fake transport to call last() on an empty list. The test now returns native-compatible null progress and orders pre-bind progress plus packet-zero raw IO before advancing the fake clock. Raw IO proves the packet passed freshness eligibility; it does not prove progress-reply validation completed. All five focused cases, 200 prepared cases over eight independent concurrent instances, and the full default SDK suite pass without production or deadline changes.

Exact isolated combined source passes generated contracts, boundaries, notices, formatting, all three TypeScript configurations and 378 tests / 5,831 assertions with three explicit opt-in skips. Python terminal/stage/existing-runtime tests pass 14 cases; actual-app report admission adds two passing cases. Actionlint passes. An initial sandboxed receiver run could not admit local service sockets; the same unchanged suite with the required local-socket access passes. This is recorded as execution setup, not a product fix.

Public `567bfe4` native iOS/Android, real-service/Keystore, headless, stack safety, Windows and optical-helper CI checks pass. Its app UI workflow still contains failures under investigation. The legacy Rust host fails unmapped shared-build CLI ledger coverage; the public-only ledger repair adds accurate host-CLI-only declarations, not unsupported MCP routes or rewrite certification. New bounded iOS stage journaling preserves diagnostics across a stuck external call while best-effort journal errors never skip owned simulator cleanup. Sanitized terminal outcomes remain post-cleanup observations, not first-cause diagnoses.

The user is asleep; no further QR, unlock, login or game actions are requested. Installed addon backup/replacement remains staged only and unapplied. Game capture, optical acceptance, real multitouch gameplay, physical latency and release/store signing remain open.


## App admission, appearance and modifier-bank follow-up

Public `8cf02404b815614b94eac5c8d06a202f5536941c`, tree `f33b61dd874ede5662c8546faf50494ad9d62f22`, adds exactly 14 reviewed leaves on sole public parent `6e249e4`. Capsule SHA256 `025b8f2270d99b5e849da1efd7ffabc285a07d62b9277664c338ab6b3a1a6ef9`; complete recursive delta, modes and blob hashes verified before CAS publication. Private commits `e27c3bc` and `80917cb` preserve the corresponding source except the public-only legacy fixture reference.

The actual Android app now passes visible dark palette acceptance during held input and verified icon refresh. Module-level PlatformColor descriptors were not reapplied after Android uiMode changed; a reactive semantic color hook uses the existing native token values without changing geometry or native view identity. The screenshot was persisted and visually inspected. Android practice/native-input tests now follow automatic landscape entry; the explicit portrait fault test temporarily changes only its Activity test policy and restores it in finally. All ten actual cases pass, including a 75.17-second hold, two renewals, 3,910 samples, zero neutral pulses and reconnect.

Actual iOS entry through XCTest normal typeText fixes an empty-paste test-driver failure. At the real 750×338 viewport, saved corner geometry then exposed division rounding beyond the strict unit extent. The projector now caps normalized size by the remaining normalized extent after the retained point clamp; the decoder remains strict. Twenty corner/viewport cases preserve point dimensions to 1e-10. Actual iOS corner and unsupported routes both pass one exact case each with no skips; unsupported rejection now reaches the saved-map decoder rather than accidentally failing an empty invitation.

Public8cf native iOS/Android, Keystore, stack safety, macOS optical/QR and Windows compile/source subsets pass. Its actual Android practice witness passes, but its saved routes fail on CI. Actual iOS app holds input for 22,458ms before progress/read-failed with maximum publication gap18ms; the prior public6e explicit runtime fails at11,514ms with maximum gap21ms. These remain intermittent stability failures despite physical and other CI passes. Root preserved the public6e diagnostic artifact. The bounded stage journal proves setup and launch completed. The next diagnostic supplement persists sanitized native terminal outcomes atomically because the native-runtime wrapper's final stdout was redirected to a private runner file. No deadline or success condition is relaxed.

The inherited public Rust test fixture path repair admits portable-core and hosted-builder CI. Its next macOS interoperability failure is the same missing private artwork in two Swift test files. Reusing the unchanged already-public asset-bearing Indigo example passes all64 XCTest cases and four Swift Testing cases in the two affected suites. The public-only two-file capsule SHA256 is `5bd847f0f19882c2adc23a244f237b41b49216362432a3516f7ca2818ab1b3c8`; it is not staged over the retained private test variants. No skin was authored, altered or newly published.

Modifier-bank source is saved privately in `77da976` and remains pending native phone SDK completion before public publication. Frozen host14 SHA256 `be3e7bbf5078f773466ae682cde46851e0a99f4e0cbfc2a28254b89f1fbf8e7a`, native6 `336ef26b881cb519c8fe2850128ef097ecb557ba23e15580f8a37bf35e966854`, React overlay5 `82184f8b918ccb0d5d3fe25be4f2fd489c6517370ce64f05d229badd76f25161`, and actual service acceptance4 `a355f0250982170d6b93246d7891908db7f72e1cd410056aee9e3899724d6803` are independently reviewed. Exact composed root gates pass390 TypeScript tests /6,385 assertions, three explicit opt-in skips, all generated/boundary/notices/format/type checks, 154 Rust unit tests and25 process tests. Actual Kotlin bank reads pass all four banks in both themes over UDP and TLS fallback while both triggers and South remain held under unchanged TTL350; legacy role10 also passes. The full SDK exposed an accepted-callback overload compatibility regression, now assigned to its owner; no full SDK pass is inferred until corrected and rerun. Actual app bank visuals and real-game optical acceptance remain separate open gates.


## Unattended bank composition review — 2026-10-23

Published 65 curated leaves as public `eca50a73e6950f10dd0cdae7fa96ab24d872cbb0`,
tree `27a41df05af643ae9430b5bfad7760726733796f`, sole public parent
`8cf02404b815614b94eac5c8d06a202f5536941c`. Recursive leaf modes and Git blob
hashes matched the frozen composition exactly, with no deletions. Private source
commit `62cc325` contains the remaining 30 reviewed leaves; earlier 37-leaf
source is `77da976`. Public-only Swift fixture variants remain outside private
source commits. Draft PR4 updated; no release/store upload or submission.

New source includes role11 bank metadata/icons, host and both native SDK/app
fences, required real-service bank CI, Android bank QA, terminal artifacts,
public retained-fixture repairs and test-only output retirement wait tracing.
Frozen SDK20 and callback fence1 independently cleared; Android QA5 independently
cleared. Diagnostic3 adds only four audited static native IPC terminal symbols;
eight Python tests pass. Root QA5 SHA256
`4491d4e3c43a258fffd1a231bf0e5a742f5d0a513fd9f471a30e1e9360d282a0`.

Exact isolated composition passes generated/boundary/notices/format and all
three TypeScript checks: 390 tests, three explicit opt-in skips, zero failures,
6,385 assertions. First attempted verification was sandbox-bound and could not
start local receivers; stopped that owned test run and retained its log, then
ran with authorized host-local IPC. No product change followed that setup error.
Both actual Kotlin role10/11 service witnesses pass. Isolated standalone CLI
and recording receiver packaging succeeds. Required full stack-safety script
passes 30 constrained-thread cases and macOS/iOS Debug frame budgets again.

Actual Android bank app runner v5 passes exact one case, zero skips/failures,
five service phases. Three actual pointers hold South/LT/RT; all four banks,
both themes and appearance refresh are checked. Recording evidence confirms
unchanged epochs, full held state and no neutral pulse across refresh. The
visible mounted-image raster oracle is distinct from composited screenshot
pixels. v4 app case passed but wrapper rejected valid Gradle aggregate XML;
the corrected strict aggregate parser has four passing admission tests. Earlier
setup and app failures remain retained. No endurance/physical acceptance claim.
Logs: /private/tmp/thumble-bank-actual-app-proof-v5-run.txt,
/private/tmp/thumble-bank-composition65-verify-host.txt,
/private/tmp/thumble-bank-package-proof-build.txt,
/private/tmp/thumble-bank-stack-safety-20261023.txt.

Full aligned Swift SDK still has two renewal-mode failures. iOS actual bank QA
is underway in additive test-only files, not published in this snapshot.
Intermittent diagnosis has reproduced expiry without Swift/TLS/simulator, with
one producer dispatch gap 376ms against unchanged 350ms TTL and sub4ms packet
RPC. A separate native owner reply-timeout branch and earlier clean-release
transport branch are not yet causally resolved. Native scheduling/activity
parity investigation remains a private prototype. No safety deadline changes.
Real-game login and physical phone-dependent checks remain deferred for user
availability; no new human action requested.

Standalone relocated package acceptance also passes one case / 67 assertions,
with the compiled CLI and sibling receiver outside the checkout and no Bun on
PATH. Log: /private/tmp/thumble-bank-package-proof-test.txt.
The composed required-mobile workflow also passes actionlint.


## Unattended app gates and real unsigned artifacts — 2026-10-23

Private commits e52ea6c/e135e46/db4ce8b/9c0d42e/d9a70e7/000e8b7 add a rehearsed
reviewer guide, isolated Android QA selection, read-only final artifact audit,
strict iOS bank QA, required app-bank CI and timeout-category diagnostics. Public
head remains eca50a73 while the separate first-native-error artifact is being
prepared for a coherent next review batch. No broad analog supersets were staged.
Pending19 capsule SHA ea668604d5882350bbff4c7dfab81967958fd381acd38ee327a38302cb890953;
actual artifact documentation supplement SHA
56dafb22ed6b5f889092c2e4e6f3462639ec45e9df54171f1361e9573d6b8dcf.

Android actual bank v6 complete runner passes against explicit aligned isolated
service/native2bd546d8, one case/zero skips/failures/five phases. Private log
/private/tmp/thumble-bank-actual-app-proof-v6-isolated-run.txt; screenshot reviewed:
connected, correct dark palette, three synthetic controls held, Done visible.
The prior v5 shared-development-binary result is not substituted for this exact
host/native source witness. Reviewer rehearsal uses relocated compiled siblings
with no Bun on PATH: default no listener, starter, explicit fixed-loopback closed
auth listener, saved CAS activation, readiness, bounded offer/cancel and release.
No QR/physical/game claim follows from this rehearsal.

The new iOS bank QA target builds and its five ownership/parser/cleanup tests
pass. Real three-pointer trajectories reached all banks/themes, CAS and held
verification; one full underlying run exposed wrong final fixture response keys.
The corrected strict next run failed late with native firstTermination=input_expired,
already inactive before capture. It is NOT accepted. Earlier malformed screen
captures and interpolated gesture mistakes remain failed attempts. New CI keeps
this strict gate, independent of the existing two QA compile outcomes; all39
existing workflow steps, original budgets/concurrency/artifacts were checked.
Four extracted actual EXIT cleanup shell cases pass. Actionlint/ShellCheck,
format and all three TS configurations pass; 4+5+11 new Python tests pass.

Actual unsigned production-profile artifacts were built in fresh disposable
copies using illustrative1.0.0/build4, no accounts/keys/provisioning/install/upload.
Android arm64 AAB:22,933,026 bytes SHA
4b8dda3b144fb2e2d89a8f6fe8a50917e8cfe001e3820053b082510585ac5ae1.
Read-only metadata audit passes11/11;1192 files/108,520,402 expanded bytes,
minSDK29/target36,24native libraries. signingConfig null/no JAR signature entries.
iOS unsigned generic archive:166files/162,000,155bytes, inventory tree
0721ed054e37625ec1e01e7a9c0d26da63358266b18590071c05a2f868abed03;
10checks pass, notice presence fails (0 shipped notice files),11privacy plists parse.
Original archives stay immutable while actual Metro/Pods/Maven/native provenance
is reconciled for complete notice packaging. Android base/assets count1 is not
the whole ZIP notice inventory; AndroidX also has a base/root/META-INF license.
Both artifacts remain distributionAccepted=false/signingStatus=unverified.

Timeout classification2 SHA92098604dac7f501d2e6df69d1a6c9e620c927c97a924659ecf6b353d4c76d6a
maps WouldBlock/TimedOut only to native_input_timeout. Real withheld-reply test is
red-before/green-after and preserves socket shutdown/no replay; default156+25 and
pinned Mac169+24/strictClippy/fmt pass. Induced~280ms receiver pause proves that
sub350ms IPC timeout can produce cleanup native_input_released, the same signature
as earlier uninjected06. It does not prove the original scheduling cause. Later
closed-client errors/final terminal can still be generic transport_failed.
Four new uninjected75s local runs pass. Latest hosted eca iOS runtime fails at
holding4992ms/progress/read-failed/maxgap19ms after setup completes, terminal
transport_failed elapsed1us; first native error instrumentation is the next
needed discriminator. No deadline, safety, QoS or release acceptance changed.


### Unattended store/bank acceptance and publication — 2026-10-23

Published curated48 from private commits7935579/af78492 as public
50d53c56e2fced5693c05f48bc374d329701dc93, tree
fd40d5ece851c4820c6666cba89199aeb004b0c4, sole public parenteca50a7.
All48 recursive leaf hashes/modes, no deletions, sole parent and CAS verified.
Capsule /private/tmp/thumble-store-bank-review48-source-20261023.json SHA256
5a2b867b78a2d3277f71981b08c530f8b6a72ba4833edf567ac1d959bd386ccc.
Original upstream notice CRLF/blanklines retained verbatim; diff-whitespace checks
exclude only the two byte-pinned notice text bundles.

Exact isolated public projection verify:396pass/3explicit opt-in skips/0fail,
6594assertions, all3TypeScript configurations, generated/boundaries/notices/format.
Root independently passed14notice Node,10Androidinventory,11artifact and5native
error parser tests, actionlint both workflows and Rustfmt. Agent exactdiagnostic13
passed158nativeunit+25process, bothMacClippy variants and a cold75.338s nativeiOS
runtime; independently reviewed positivecontrol pauses only ownedrecordingreceiver
283115us, captures brokerWouldBlock35→packettimeout→receiverEOF/ownedexit before
cleanup in1077bytes/0600. Induced evidence validates plumbing, not originalcause.

Corrected iOSbank attempt12 PASSED exact1case/0skip/0failure/all4banks×2themes,
appearanceCAS/heldrefresh/Done-neutral. Fifteen captures activebefore+after;
rootvisualreview darkboth screenshot PASS. Evidence
/private/tmp/thumble-ios-bank-ui-wide-attempt12 and-run.txt. NativebinarySHA
29c800c6bd47302e1a6113108c1a48eb3032fa445b3b3f63a9cf5ed773a05263.
Attempt11 stopped at Ruby/xcodeproj setup; systemRuby corrected setup without
product/deadline changes. Earlier failed attempts remain failed.

Fresh normal store-parent prebuilds in
/private/tmp/thumble-store-parent-integration-proof register exact2resources iOS
and exact2assets Android; generated prebuild metadata12/12PASS. Later iOSExpoRouter
origin update is helper-verified, not rebuilt into thirdunsignedarchive.
ThirdiOSarchive172files/161604310bytes, tree
f86478ca5c1a55ece215ddfcdf899457cfccaaca6547031cb86e54fde296973f;
11artifactchecksPASS, exact26textbundle29878bytesSHA9cd6ab4d31d89adafd93f46060c4252fb7a367adcc66cc3cdfd40dcb8881eb9f
and then-frozeninventory58040bytesSHA076fd43b193ccab28c9ad499f401e9693c34a82b11b1543c20ce8eea289dee6d.
Its own matchingbuild-emittedmap SHAe7d906e352db412d463d63cc5a2447fce8704e801d8098b9b00b45ae914ad35a
matches priorgraph1142sources, while ownHermesSHA
8e305b270c8fb8c197e71660bccc0190126df35551bd8fa09d2cff3c5da130b0 differs.
FinalAndroidunsignedAAB SHA080efb4cc1aa098d6bccda22100d332e72ac201feb63b51c063411f896c30beb,
22982799bytes,11artifactmetadataPASS, zeroJARsignatureentries, originalunchanged;
33verbatimtextbundle91352bytesSHA9448866c217eb6099a351ab3f5488b7f0b22bffbd706d635d15bda1e77149972,
inventory213985bytesSHAe9bfb70039728741db385f8c1d9742c204f7898badef473995d40aa198518609,
186unresolved. Both completecoveragefalse/signingunverified/distributionfalse.

Newpublic CIruns39965465030(app)/39965465215(native)/39965465195(headless)
pending. Prior eca appiOS5/0 andAndroid10/0 legacywitnesses including75shold PASS;
savedcorneriOSFAIL/unsupportedPASS, bothAndroidsavedroutesFAIL. Artifacts expose
onlyfailed and cannot identify cause; additionalfixedsavedstage evidence inprogress.
No native-stability fix, physical bank/game acceptance, production signing or
store submission claimed. User asleep; phones/game/install flags untouched.


### Typed authoring, actual CI classification and second publication — 2026-10-23

Public c4a05098efc81f3a7367fa15fd9aab844fc0bfbd, tree
0c91bdba1dd7f3490e64502b061a8c2498f58924, sole parent50d53c5.
Curated31 source SHA33b5946a435663638726c47cf5359758ee572263658892c9c06cf8dedb03b3ba
/private/tmp/thumble-control-label-review31-source-20261023.json.
All31recursiveleafhashes/modes/no deletions/solepublicparent/CASverified.
Privatecommits b0dd256/54d7628/4510ba6/4549601 retain9notice/diagnostic
and17authoring and5orchestration/CI source deltas. Broader six privateQA working
variants were preserved;25nonconflicting source files materialized.

Domaincontract7 SHA9ddf280a4d61c8d734c95148fb5546b7fab4359cc72cd1c0bb35885da4586b83;
app4 SHA c5b98a5e70534e93f3fcd059d64b2abdac0c1de3e832ace17b986e5e33a9bd81;
CLI4v2 SHAe721b5cff4e3b6b9c76f6274327928eee2e697de466b9fd4a575422a01fd306b;
MCP2v2 SHA6a867a8226e7868b8669aa32a063507850c816790d4ce05df85c0f2523532c29.
Strict3fieldlabelpatch, string≤128UTF8/blank/nullclear, originalgeneratedlabel
preserved via labelOverrides, rawopaque/numeric/resource/mirror data preserved.
Contract closed6fields; authority normalfingerprint/replay-before-source+clock,
finalSQLiteCAS, canonicalprofile/control affectedEntities. MCPwrite-capability
only; CLIstrictcommand-gatedlabel/clear/help flags. IndependentreviewsCLEAR.

Rootcoherent26v2 fullverify418pass/3optskip/0fail6855assertions/83files69.12s,
all3types/boundaries/generated/notices/format. Root9CLI+SQLitecases121assertions;
independentNodeSDK modern+legacy readonly/write cases/concurrentonecommit+replay/
conflict/mismatch/blank/clear/restart/export pass. Initialfullsuite417green/1red
NEWNodefixture export returned configuration_export_directory_unsafe: ownedalias
red proves uncanonicalizedmkdtemp path, Node-onlyrealpath fix/permanentPOSIXalias
regression green. Nativeexportguard/timers unchanged. No firstfailedtest deleted.

Standalonebuild initialscaffold lacked exactpublic ThirdParty license8990518e…4bae;
copiedverifiedblob, no productsourcechange; build2PASS. Existingpackage1/67PASS.
Relocated compiledNodeSDK authoring proof4scenariosPASS with childPATH/usr/bin:/bin,
6edits/revision7/replayafterrestart/inputActivatedfalse. PackageCLI SHA
832089506555d1f0793490274e1afab357cfa3ca0e44708e714bce94624de958,
releaseReceiverc8ac45244f9e1417f87e3b2dd7366338f24e919fec62dff665b37151ffbe211b.
/private/tmp/thumble-label-relocated-proof/result.json; ownedservice stopped.

Additional9actualupstreamroottexts selected, iOS35texts63751bytes. Completecoverage
stillfalse, no newarchive claimed. NestedRewrite/.gitattributes explicit-text-off
preservesoriginalmixedCRLF bytes;4checkoutcases (both bundles×autocrlffalse/true)
byteexact. Private /private/tmp/thumble-notice-checkout-proof/result.json.
Savedroutediag5v2 seven tests includecontradictoryiOSsummary cannotclaimassertion;
oldv1attributionblockerfixed, statusneverfalsegreen. Exactpublic50actualAndroid
savedcorner+unsupported PASS1caseeach/0skip/0fail onAPI35arm645580; no hostedx86cause
claimed. Evidence /private/tmp/thumble-saved-route-public50-run/acceptance-summary.json.

Public50nativeiOS failure exactstage: compile/sign/create/boot/install/container
complete; launchstarts168.654s andnevercompletesbeforeouter480s. No runtime or
firstnativeerrorartifact; fourSwiftrealservice groupsPASSED. Boundedprivateproof
/private/tmp/thumble-public50-ios-native-failure-summary.json SHAcb7a50903b4716f1a24b8f653c57619ec897c0cf16fb25abeeedee2495e98785.
Neworchestration4 SHA8bd516e83e6687fb6e1bd0d9deb15914924dd9eca5d5e0740b790bce60dd3106
launchwatchdog60s/independentcleanup15each/fixedatomic0600sidecar, preserveprimary;
16runtime/stage testsred-before/green-after+independentreviewCLEAR. Addedonlyone
unitline toexactpublicnativeCI; oldartifactglobcoversnewsidecar; product250/350,
runtime180 andouter480 unchanged. No real simulator/phone actions fornewguard.

Public50AndroidappFULLSUCCESS inclrequiredbank; Androidnative/KeystoreSUCCESS.
iOSapp39965465030 stilloldUIKit/savedroutes IN_PROGRESS; bankpending. Headless
Mac/Linux+Windows2/3pass, Windows1packaging30s timeout; dormantWindowsbridge two
authorization/controls testsfail; retainedMacbridge constrainedprocess5s timeout.
Thosecausesremainopen; no retry/deadlineworkaround. Publicc4 jobs39971094743(app),
39971094975(native),39971094596(headless) queued/pending; optical39971094514 and
Windowsstdio39971094787alreadySUCCESS. Apps/native concurrencyfalse preserves
ongoing public50run. No launch/product/game/store gate declaredcomplete.
