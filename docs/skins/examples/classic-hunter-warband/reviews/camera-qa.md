# Hunter Warband Camera 1.1.0 — independent QA, candidate 2

Verdict: **qa-pass** for the exact source/package/captured native profile. Physical phone and real WoW input behavior remain separate parent-owned tests. No human approval or publication authorization is granted.

Read the project skin-author workflow/quality bar and independent QA role, `art-direction-camera.md`, designer execution notes and both camera critiques. Both independent critics issued **visual-pass** on the exact candidate below.

- Package `dist/classic-hunter-warband-1.1.0.pocketpad`: SHA-256 **77fe5da8064c82dcec6bfbb2ab757fefe2f81ae9e3ba2df5f2dba43c9b6afb0a**.
- Sheet `camera-review-2/contact-sheet.png`: SHA-256 **a3db8bf5921c52bdbfc537d3cc42dbcef38fdaebad5738963a5353c42a9f8dc1**.

Both independently recomputed and matched. Supporting logs, geometry, direct quality and pixel evidence are retained in `camera-qa-evidence/`. QA changed no implementation, source styling, geometry or live app state.

## Determinism, strict package and quality

Copied only editable source/CSS/profile assets into independent temporary A/B workspaces before invoking the helper, because it writes captured artboard/source/profile outputs. The reviewed original workspace is preserved.

```sh
docs/skins/examples/classic-hunter-warband/build/build-hunter-warband /tmp/thumble-camera-qa-A camera-qa
docs/skins/examples/classic-hunter-warband/build/build-hunter-warband /tmp/thumble-camera-qa-B camera-qa
build/ThumbleControllerImport/bin/thumble skin validate docs/skins/examples/classic-hunter-warband/dist/classic-hunter-warband-1.1.0.pocketpad --strict
```

Both helper invocations exit **0** and run actual `ThumbleSkinCompiler.compile(clean:true, strict:true)` independently, not copies of an existing archive. Both generated packages compare **byte-for-byte identical** to the exact reviewed candidate SHA above. Strict package validation exits **0**, `valid with no warnings`.

Each helper runs the actual `ThumbleSkinQualityEvaluator.evaluate(package:workspace:capturedProfile:)` API on the correct **classic-hunter-warband-camera-captured-v2** artboard/profile. Independently asserted both reports have **zero issues of any severity**, score **100**. This satisfies the strict evaluator predicate (no errors or warnings); helper exit alone was not taken as proof, as the helper prints rather than throws quality findings.

The literal wrapper command was attempted:

```sh
build/ThumbleControllerImport/bin/thumble skin quality docs/skins/examples/classic-hunter-warband --artboard classic-hunter-warband-camera-captured-v2 --strict --json
```

It exits **1 before quality evaluation**, because it requires `design-session.json` for captured workspaces. This known tooling limitation is recorded in `camera-qa-evidence/wrapper.log`; it is **not reported as a passing CLI command**. No design-session metadata was fabricated, incompatible catalog artboard substituted, or warning waived. The same underlying evaluator was exercised directly with the real captured profile and passed strictly as described above.

## Native evidence and integrity

Each independent source compile rendered all **16 native SwiftUI panels**: portrait/landscape × light/dark × normal/pressed/active/disabled. All sixteen PNGs plus contact sheet (**17 files**) compare byte-identical between A and B. Native rerender hashes are retained in `camera-qa-evidence/native-sha256.json`.

```sh
build/ThumbleControllerImport/bin/thumble skin unpack docs/skins/examples/classic-hunter-warband/dist/classic-hunter-warband-1.1.0.pocketpad -o /tmp/thumble-camera-qa-unpacked
build/ThumbleControllerImport/bin/thumble skin pack /tmp/thumble-camera-qa-unpacked -o /tmp/thumble-camera-qa-repacked.pocketpad
```

Both exit **0**. Repacked archive compares byte-for-byte identical to the candidate. Archive inventory remains appearance-only: manifest, skin base/variants, eleven PNGs. No executable bindings/profile payload, scripts, CSS or SVG are distributed. All manifest asset SHA-256 and byte-count declarations match actual bytes. Dimensions/budgets pass strict validation. Editable CSS/JSON/SVG and icon originals remain outside the distributable.

All **five real game-icon PNGs are byte-identical** between 1.0.0 and 1.1.0 packages: Attack, Raptor Strike, Auto Shot, Spring Water and Tough Jerky. The earlier independently verified original-BLP/PNG hashes, exact 64×64 conversion and Classic Era CASC provenance/Blizzard attribution remain applicable. No replacement art or generated action imagery was introduced.

## Binding ownership, identities and special settings

```sh
python3 docs/skins/examples/classic-hunter-warband/sources/verify-bindings.py build/ThumbleControllerImport/recovery/before-hunter-warband-native.json
```

Exit **0**: all **16 exported command outputs** match the private pre-change backup across primary, landscape and portrait stored canvases; Move direction parts and W/S/Q/E outputs match, and no gamepad output remains. Private character metadata was not copied to review evidence.

Additional independent JSON assertions compare candidate to the exact v1 reviewed profile in all three stored canvases: **21 controls**, all **18 old IDs retained**, old button outputs/defaults and Move joystick/part mappings unchanged (normalizing dictionary ordering). All four intentional duplicate shortcuts retain their actual output owners. New identities are stable separate Camera/Left-click/Right-click affordances rather than fabricated addon actions.

Verified in all three canvases:

- Camera analog target `mouse_look`; `sendsDigitalDirections=false`; dead zone0.12; no gamepad axis mapping.
- Left/Right click native trackpads: `clickButton=left/right`, `movesCursor=false`, `tapToClick=true`, `twoFingerScroll=false`, full accessibility names and truthful legends.
- Cursor retains native relative movement, tap and scrolling settings; no camera target is attached to it.

These settings assertions establish artifact correctness, not actual OS mouse-button ownership or release behavior.

## Geometry and actual-pixel contrast

```sh
python3 docs/skins/examples/classic-hunter-warband/sources/measure-camera-evidence.py /tmp/thumble-camera-qa-A/reviews/camera-qa
python3 docs/skins/examples/classic-hunter-warband/sources/measure-native-hints.py /tmp/thumble-camera-qa-A/reviews/camera-qa
```

Both exit **0** against independently regenerated images. Four geometry records each contain **22 native controls including the system toggle**, no intersecting hit rectangles or out-of-viewport bounds, minimum hit dimension **44 pt** (floating-point representation43.99999999999999), minimum separation **11.01955 pt portrait /9.584 pt landscape**. Every old native record except intentionally relocated Cursor matches v1; Camera/Cursor/click centers and visual sizes match the brief within0.001 pt. Reference canvas compliance does not establish physical device exclusion handling.

All **96** Move/Cursor/Camera/L-click/R-click/drawer legend cases pass actual composited pixel contrast ≥4.5:1, minimum **5.4929767:1**. All **64** secondary native key-hint cases pass, minimum **4.7527368:1**. Full case evidence is retained. Both independent visual critics inspected the exact accepted sheet and found complete captions, stable native states, clear grouping and no unresolved majors.

## Functional regression evidence and remaining boundary

Reviewed supplied logs, without claiming QA reran those suites: `/tmp/thumble-camera-stack-safety.log` reports **41 tests, zero failures**, successful Mac/iOS builds and `Stack-safety verification passed`; `/tmp/thumble-camera-swift-focused-tests.log` reports **5 tests, zero failures**; `/tmp/thumble-camera-android-full.log` ends `BUILD SUCCESSFUL` including lint; Rust core/host logs report passing suites with their explicit ignored cases retained. These supplement, rather than replace, physical verification. Existing ignored tests were not silently counted as passes.

Parent owns phone installation, asset synchronization, headless receiver operation and actual input tests: displacement-based camera motion, neutral stop, independent right-button ownership/cancellation/disconnect release, safe Right-click interleaving, Cursor movement/tap, and discrete click semantics. No app installation/restart or live input was performed by QA, and **no successful WoW input test is claimed** here.

`human-approval.json` and `camera-human-approval.json` remain **pending**, tied to the exact new package/sheet hashes; the prior pending record is preserved in QA evidence. Local installation authorization does not constitute publication approval. No website/public catalog update, publication, deployment, staging, commit or push occurred.
