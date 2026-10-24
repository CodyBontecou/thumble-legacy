# Classic Hunter Warband — independent final QA, candidate 6

Verdict: **qa-pass** for the exact local appearance package and captured Hunter profile, with the CLI quality-wrapper limitation explicitly recorded below. No publication or human approval is granted. Live phone/gameplay behavior is not established by these checks.

Read the QA role, project skin-author skill and workflow/quality references, revised art direction, and all three critique reports. The final independent critic verdict is `visual-pass` against `native-review-6/contact-sheet.png`.

- Package: `dist/classic-hunter-warband-1.0.0.pocketpad`, **201,174 bytes**, SHA-256 `ee1851315847080a8190f3b1f562ef9fe6b3bb2abd96a10896de653bfa3ed6e9`.
- Reviewed sheet SHA-256: `a08ab2d25646301f9f9b46766d0ea33ba9b0e82ad5a07038a042db9e4bbc9ea9`.
- Independently computed both hashes. QA logs, exact rerender hashes and direct evaluator output are retained under `reviews/qa-evidence/`.

## Deterministic source compilation and native rendering

Copied `skin-source.json`, `sources/`, `styles/`, and `preview/` into two independent temporary workspaces `/tmp/thumble-hunter-warband-qa-A` and `...-B`. The helper writes captured-artboard/source/preview outputs, so it ran only against copies, preserving the exact reviewed source and live state.

```sh
docs/skins/examples/classic-hunter-warband/build/build-hunter-warband /tmp/thumble-hunter-warband-qa-A native-qa > /tmp/thumble-hunter-warband-qa-A.log 2>&1
docs/skins/examples/classic-hunter-warband/build/build-hunter-warband /tmp/thumble-hunter-warband-qa-B native-qa > /tmp/thumble-hunter-warband-qa-B.log 2>&1
```

Both exit **0**. The existing linked helper calls actual `ThumbleSkinCompiler.compile(... clean:true, strict:true)`, so this is two source/package compiles, not two copies of an existing package. Python byte comparison passes for compile A, compile B, the exact reviewed distribution candidate, and the unpacked/repacked package. All four have the package SHA above.

Each build independently runs the native SwiftUI renderer for portrait/landscape × light/dark × normal/pressed/active/disabled: 16 native 2× frames and a contact sheet. All **17 PNGs** from A and B compare byte-for-byte equal. Per-PNG SHA-256 values are in `qa-evidence/sha256.json`. Individual viewports are 804×1748 and 1748×804. This verifies renderer output without needing the desktop GUI running. No rebuild of app product sources was needed or performed by QA.

## Strict validation and captured-artboard quality

```sh
build/ThumbleControllerImport/bin/thumble skin validate docs/skins/examples/classic-hunter-warband/dist/classic-hunter-warband-1.0.0.pocketpad --strict
```

Exit **0**: `Validation: valid with no warnings`.

The literal CLI wrapper was also attempted:

```sh
build/ThumbleControllerImport/bin/thumble skin quality docs/skins/examples/classic-hunter-warband --artboard classic-hunter-warband-captured-v1 --strict --json
```

It exits **1 before evaluation**, because this independent captured workspace lacks `design-session.json`. `contract/profile.json` exists, but the current CLI wrapper insists on a full controller-design session for any captured artboard. QA did not fabricate a session or substitute the incompatible catalog `wow-forever-v1` artboard. This command is **not reported as passing**, and no quality warning was waived.

Equivalent exact quality gate: the independently rerun helper invokes the same `ThumbleSkinQualityEvaluator.evaluate(package:workspace:capturedProfile:)` API used by the CLI, supplying the actual `preview/profile.json` and its schema-3 captured artboard. Both A and B results are `checkedArtboardID: classic-hunter-warband-captured-v1`, score **100**, **zero issues of any severity**. The strict acceptance predicate (no errors and no warnings) therefore passes through the actual evaluator. `qa-evidence/quality.json` is the independently generated result. The helper itself prints issues rather than exiting nonzero for them; QA explicitly asserted `issues == []`, rather than trusting helper exit alone. This satisfies the substantive captured-profile quality gate while documenting the CLI tooling gap.

## Archive integrity, appearance safety and source media

```sh
build/ThumbleControllerImport/bin/thumble skin unpack docs/skins/examples/classic-hunter-warband/dist/classic-hunter-warband-1.0.0.pocketpad -o /tmp/thumble-hunter-warband-qa-unpacked
build/ThumbleControllerImport/bin/thumble skin pack /tmp/thumble-hunter-warband-qa-unpacked -o /tmp/thumble-hunter-warband-qa-repacked.pocketpad
```

Both exit **0**; repacked bytes exactly match the original. Archive inventory consists only of `manifest.json`, `skin.json`, and eleven PNG assets. No profile payload, executable binding payload, CSS, SVG, script, absolute path or traversal entry is distributed. `skin.json` contains appearance base/variants. Captured compatibility metadata includes geometry, roles and paint contracts; it does not install control bindings. All eleven asset byte counts and SHA-256 values independently match manifest declarations.

Asset dimensions: four full-canvas rasters 1748×804 / 804×1748; five original icons 64×64; two Cursor faces 256×256. Strict validation accepts package/media budgets. Editable CSS, JSON and original SVG files remain outside the archive in the workspace. Original project Warband reuse is explicitly user-authorized in revised art direction; no publication or ownership claim over Blizzard imagery is made.

Independently verified all five `sources/provenance.json` original BLP and converted PNG hashes. Pillow decoding confirms each BLP's exact RGBA pixels equal the corresponding 64×64 PNG, with no resizing or recoloring. Texture IDs match recorded snapshot identities: Attack 135641/spell6603; Raptor Strike132223/spell2973; Auto Shot135493/spell75; Water132794/item159; Jerky133972/item117. Retained originals record installed Classic Era CASC source and Blizzard attribution. QA verifies retained source identity/conversion integrity; it does not claim a fresh independent CASC re-extraction.

## Outputs, geometry and contrast

```sh
python3 docs/skins/examples/classic-hunter-warband/sources/verify-bindings.py build/ThumbleControllerImport/recovery/before-hunter-warband-native.json
```

Exit **0**: all **16 exported outputs preserved in all three stored canvases**, keyboard Move parts match the private before-backup, analog target is none, digital directions are enabled, and no gamepad output remains. The private backup and its character metadata are not copied into review evidence. Also independently compared `preview/profile.json` with `preview/install-profile.json`: every button output/default, direction-part output and joystick mapping is unchanged, normalizing dictionary encoding order. Shift chords retain mask2. Four documented duplicate shortcuts belong to Map/Character/Jump/Self; Cursor is a native relative pointer/tap affordance, not an exported Look/camera binding.

Independent arithmetic over all four rendered geometry records finds **19 controls** (18 reference frames plus system drawer), no intersecting hit rectangles, no viewport violations, minimum hit dimension **44.10176 pt portrait / 44.1984 pt landscape**, minimum rectangle separation **11.01955 / 10.29547 pt**. The captured profile and package geometry are the same custom Hunter contract; the independent critic additionally measured all 18 reference frames against Warband within floating-point tolerance. This does not equate to physical Pixel safe-area or gesture/release verification.

```sh
python3 docs/skins/examples/classic-hunter-warband/sources/measure-native-hints.py /tmp/thumble-hunter-warband-qa-A/reviews/native-qa
python3 docs/skins/examples/classic-hunter-warband/sources/measure-native-instruments.py /tmp/thumble-hunter-warband-qa-A/reviews/native-qa
```

Both exit **0** against the independent fresh renders. All **64 native secondary-hint cases** pass ≥4.5:1 (M, Shift-B, F1, Space across 16 panels), minimum **4.7527368:1**. All **48 Move/Cursor/drawer cases** pass, minimum **6.3982708:1**. The saved evidence includes fully covered glyph/face RGB samples; these are actual composited pixel checks, not only token estimates. Independent critic 3 already visually inspected exact candidate6 and closed both prior contrast majors, with complete Target/Prev tags, Jump caption, stable states and proper light ground.

## Approval and product limits

`reviews/human-approval.json` remains **pending**, bound to the exact reviewed sheet and package SHA above. QA did not create approval, publish, edit public catalog, deploy, stage, commit, push, mutate live app state or install on the phone. This is a local personal derivative; no website listing/readiness claim is made.

Parent reports the final artifact imports successfully in isolated private HOME state. Parent handles authorized live installation and phone rendering/assets. Movement hold/diagonals/release/disconnect, Cursor relative movement/tap, duplicates, ability/Jump/targeting and actual WoW keyboard delivery remain device/gameplay verification. Accessibility permission was reported unavailable, so this report must not be represented as successful in-WoW input testing. The new CLI/MCP product boundary is not proven solely by appearance/package QA; retained import/refresh and receiver behavior require their separate product tests.
