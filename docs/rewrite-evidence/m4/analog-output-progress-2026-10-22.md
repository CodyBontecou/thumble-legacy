# Analog, actual phone route and native output progress — 2026-10-22

This is private implementation evidence, excluded from the public projection. Source review is available in draft PR4; no merge, release, physical output or OS capture is performed.

## Published coherent source

Public head `3c1b15ea6394ca810df59e91e6df3a027433132a`, tree `57a8ad7a40d760e1b07538a4126c657972222f8c`, sole public parent `010ab117d646131a159032ed6efaf0e626b766a7`. All 52 leaf paths/modes/Git blobs were checked against complete recursive before/after trees before the expected-parent nonforce ref update. Private source commit `41b8dbf` stores the same frozen leaves; live agent supersets were preserved.

Capsule `/private/tmp/thumble-analog52-review-source-20261022.json`, SHA256 `0f67c863287398fee4bb1cd5b9664bf878ccfc546f9eb6bf041a3ac993cb8a97`. Exact isolated public projection: `/private/tmp/thumble-native-input-v4-public-projection`. TypeScript verify:266pass/1package-onlyskip/4509assertions; generated files, all3 type checks, boundaries, notices and formatting pass. `/private/tmp/thumble-analog52-verify-escalated.txt`. The initial non-escalated run could not launch its native child correctly under sandbox and was stopped; it is not a passing gate.

Analog integration separately passes129Rustunit+23process, strictClippy, packaging1/67 and3actualSwiftservicecases. Correct Swift filter/parser evidence is `thumble-analog30-service-swift-correct` (initial mistaken filter executed zero and is excluded). Swift management repair:115localSDKcases, bothSDKbuilds; actual catalog2/button3/analog3 service cases pass. Projection focusedclient13 pass. Kotlin122pass/11explicitopt-inskips, AAR/lint pass. Caps preserve host credential expiry/fences and local rollback detection while bounding the local deadline to min(hostExpiry,localNow+60s). No wire/grace/host-expiry extension.

## Actual phone app findings

Android actual saved corner initially failed at catalog lease admission when the emulator clock lagged the host. Controlled diagnostic isolation proved only the old `hostExpiry-localNow>60000` predicate rejected; other checks passed. Focused client regressions fail before/fix after. Clean corner and unsupported app modes each execute exactly1case with zero skips/failures; temporary production diagnostics were removed byte-for-byte. The mode-aware orchestrator also passes both. Private results `/private/tmp/thumble-saved-android-{corner,unsupported}-clean-results`.

Actual iOS saved corner passes28.888s, exactly1case/zero skips: public UI paste/Done/Pair, actual Keychain, delivered native map, saved summary2/A/noB, corner/header geometry, >=1.5s held input with nativeTTL350 and release ACK. Proof `/private/tmp/thumble-saved-corner-react-attempt8-proof.json`. Unsupported also passes17.412s/exactly1case/zero failures/skips with no grant/input counters and nativeexit0. Proof `/private/tmp/thumble-saved-unsupported-react-attempt1-proof.json`. Atomic QA publication remains pending. An earlier attempt hung before test-runner launch; another assertion had duplicate nested AX A labels, corrected without weakening the expected content. Actual analog UI and hardware/gameplay remain separate.

## Hosted findings

010 native mobile SDK all3jobs pass, Android app/production build pass, Mac headless pass. Windows compile39855958818 produces unsigned DLL/EXE/INF evidence only; install/sign/load/SDKdevice/gameplayfalse. Windows data39855959026 verifies96rows/5rejections/6adversarialstates from exact four upstream data declarations, never device-capable SDK execution.

010 iOSapp39855958736/job120094106514 fails with TLSruntime read-deadline/controlprogress after4071ms,246publications/maxgap17ms,hosthealth110/110, nativeexitnull. No timeout increase or root-cause claim. Linux headless fails because the test ceases input publication after SIGCONT while renewing owner;52source repairs that witness and focused8/206pass. Windows headless receives child reply only after parent stdin destruction.

New controlled Windows matrix39861377874/job120112662734: all scriptcells1..4/9/10/13pass; all bun-testpending-rejection-matchercells5..8/11/12fail withreplyOnlyAfterEOFtrue acrossbothparentAPIs/bothchildruntimes/bothwriters. This localizes test-runner/matcher interaction; production transport/timers unchanged. Native-await assertion controls and narrow test repair are assigned. New public52CI runs39861377580(headless),39861377921(nativeSDK),39861377792(app); no pending run is a passing gate.

## Frozen integration prerequisites

Outputclock5 capsule SHA815cb4faf8846061c62c53897310e4e072563012a8a4f4f655bd404ce0472628. Nativebridge7 SHA c9e3f6312e82edb6c39fddfe4d9cbcf520785d42734b95ce176c0653e5163429. TSoutputboundary4 SHA d9aac466594465e3a278105e16fa6df44841deb423368d66220c846028fecd0a, 3safetycases10assertions plus isolated18tests193assertions/all3types pass. Explicit opt-in, exact kernel clock, independent output expiry and confirmed cleanup; fake devices only. Coherent18prerequisite capsule SHA36addf5661d3a17ad60622cecf96c4231c4f0567e59aef7452b2cb22eabaeb31 is not yet public and lacks final service/CLI composition.

Optical TS9 SHA1250286c827d283105c6fe0129d2f03ebef8bfc3a6c2e4289018b5c7d126d568; Mac12 SHA9f37b6ae3f584dfe11dcc92b4f27e630860bc464a49daf246db042515b06c2b7; Windowspure9 SHA930fa10b0740daf4e5f4a644de5e8c102336785691d93f711dd655336a2c25ca. Frozen portable/compile gates pass; no capture/TCC, WindowsAPI provider, authority adoption or phone delivery acceptance. New capture child/PNG/service/CLI integration is in progress and remains excluded until frozen.

Retained stack safety last55 constrained cases and macOS/iOS builds pass. No retained model/startup/codec source changes in this slice. All physical, signing, Windows backend adoption, required optical delivery and launch gates remain pending.
