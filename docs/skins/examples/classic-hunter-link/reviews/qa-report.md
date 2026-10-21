# Classic Hunter Link — independent QA, revision 2

Verdict: **qa-pass** for the native import feature and anonymized 16-command fixture. This is not a physical Android/WoW input test, publication authorization, or human approval.

## Scope and gates

Read the QA role, skin-author skill/workflow/quality bar, art direction, design specification, and both independent critique reports. Latest critic verdict is `visual-pass`, against exact `native-review-2/contact-sheet.png`, SHA-256 `01dae6ff48f6896b0c685c3badebff0c2b34011d1a90d37a5fd3e7b1874ddb97` (independently verified).

The brief explicitly defines an importer-generated native layout and profile-owned materials, with no appearance package, community catalog entry, or canonical-artboard compatibility claim. Consequently package compile-twice, strict package validators, package archive safety/unpack/repack, package budgets and package SHA gates are **not applicable**. Creating a package merely to run these commands would change the authorized scope. No strict command was run with warnings accepted. Source compilation and functional/native equivalents are recorded below.

## Functional and source verification

Reviewed `Sources/Mac/ThumbleLinkImport.swift`, Mac import view, CLI `importThumbleLink`, `Tests/ThumbleLinkTests.swift`, and the actual anonymized `hunter-classic-era.txt` fixture.

- Shared planner is used by Mac and CLI; both expose explicit Controller / Button grid selection. CLI accepts `wow import ... --layout controller|grid` and reports layout. Ordinary refresh retains stored layout; deliberate selection rebuilds imported geometry/appearance with review warning.
- New imports up to 24 commands use Controller; larger inventories default to grid without silently dropping bindings. Explicit oversized Controller requests fail. Existing manual controls participate in collision checks; an unplaceable addition fails with a relayout instruction instead of moving existing controls.
- Independently checked each of the 16 fixture commands in both portrait and primary landscape customizations: stable SHA-derived command UUID, exact physical key code and Shift modifier, empty gamepad outputs, imported baseline equal to output, and full original accessibility name. Captions shorten only known commands. No guessed spell imagery or simulated cooldowns appear.
- Independently generated imports compare equal recursively after excluding `createdAt` / `updatedAt`; timestamps naturally prevent a claim of whole profile JSON byte determinism.
- Existing refresh tests cover positions, bindings, labels, deletion, changed imported outputs, and no longer bound outputs. Explicit grid-to-controller conversion keeps IDs and outputs. Overflow tests cover all 24 commands and rejection/fallback at 25.

Reviewed passing execution logs supplied by the parent:

```sh
xcodebuild test -scheme ThumbleTests ... -only-testing:ThumbleCLITests/ThumbleLinkTests
./scripts/verify-stack-safety.sh
```

`/tmp/thumble-controller-import-tests.log`: 13 tests, zero failures, `TEST SUCCEEDED` (18:49:12 initial revision-2 pass; parent reruns after adding caption assertions). `/tmp/thumble-controller-stack-safety.log`: 39 constrained-stack tests, zero failures; Mac and iOS builds succeed; final `Stack-safety verification passed.` Source is unchanged by QA.

Independent CLI acceptance command:

```sh
python3 scripts/verify-thumble-link-cli.py build/ThumbleControllerImport/bin > /tmp/thumble-controller-qa-cli-tests.log 2>&1
```

Exit 0. `Thumble Link CLI checks passed: review, file/stdin import, selection, refresh, geometry, metadata, and invalid input.` This uses isolated authority state, not the user's live configuration.

## Native rendering and determinism

Ran sequentially from the repository root:

```sh
docs/skins/examples/classic-hunter-link/build/render-native-import Tests/Fixtures/ThumbleLink/hunter-classic-era.txt docs/skins/examples/classic-hunter-link/reviews/qa-native-A /tmp/thumble-controller-hints.json
docs/skins/examples/classic-hunter-link/build/render-native-import Tests/Fixtures/ThumbleLink/hunter-classic-era.txt docs/skins/examples/classic-hunter-link/reviews/qa-native-B /tmp/thumble-controller-hints.json
```

Both exit 0, each emits 16 native 2× PNGs plus contact sheet, profile and geometry JSON. These are the real native SwiftUI controls, not the SVG studies. Hints map to full `custom.UUID` identities and correspond to the fixture's actual outputs. Python comparison `all(p.read_bytes() == (B/p.name).read_bytes() for p in A.glob("*.png"))` passes for **all 17 PNGs**, including the contact sheet. Dimensions: portrait 804×1748; landscape 1748×804; sheet 2176×1714. Exact per-image hashes are in `qa-native-sha256.json`.

QA sheet SHA-256: `310b4f18cadf348e36a1842b14030eb09ae688fe2a22e9cc72331680027c7c4a`.

The QA sheet is not byte-identical to the earlier independently critiqued sheet. Profile contents (excluding timestamps) and geometry JSON match; visible captions, roles, composition and state distinctions match. Rerendering introduces channel-rounding and antialias differences: disabled face light (244,240,228) versus prior (244,239,228), dark (33,48,51) versus prior (33,48,50). In portrait disabled frames, fewer than 0.4% of pixels differ by over 2/channel and only 2–4 of approximately 1.4 million pixels differ over 20/channel. Independent A/B generation is byte stable. QA inspected the full sheet and portrait disabled frame; no material design change, clipping, malformed caption or geometry drift results. Both exact sheets are retained; no false reference-byte equality is claimed.

## Geometry, state and contrast

Independent arithmetic on all four `native-layouts.json` records includes the system drawer hit frame: 17 native controls, no intersections, no authored viewport violations, minimum pairwise hit-frame separation 8 pt. Imported command minimum dimension is 52 pt portrait / 48 pt landscape. The native drawer is 60×44 pt with expanded 80×64 hit frame. Viewports are already authored safe content: 402×874 / 874×402, not whole physical Pixel screens. Pixel cutouts/system insets remain a device check.

Portrait utility-to-main interval is 96 pt; recovery ends at y730. Full accessibility names remain despite `Up`, `Back`, `Target`, `Previous` display captions. Labels and key hints remain in native views. Normal, pressed, active, disabled differ in edge/depth/opacity without shifting control centers or baselines. The inherited depressed active face remains the critic's accepted tracked minor, not an unresolved major.

Independent Pillow sampling in hint-only Attack `1` regions (portrait pixel box 490,940–525,960; landscape 1259,441–1294,480), excluding fringes, found matching fully covered glyph/background colors in both orientations:

| Appearance | Hint RGB | Face RGB | Contrast |
|---|---|---|---|
| Light disabled | 96,94,89 | 244,240,228 | 5.68:1 |
| Dark disabled | 168,174,175 | 33,48,51 | 6.08:1 |

Ratios use sRGB linearization and `(Lmax+.05)/(Lmin+.05)`. They exceed the 4.5:1 acceptance target. Critic 2 independently verified Shift-B, Shift-Tab, punctuation, Space and drawer contrast in the exact reviewed sheet. Main legends have stronger contrast. No visual marks substitute for executable controls.

## Editable sources, safety and remaining limits

Four original editable SVG framing studies remain under `sources/artwork/`, plus editable `design-spec.json` and the native renderer harness. They are workspace studies only, not raster dependencies or distributed skin assets. Runtime changes contain native layout and material values; binding assignment remains a separate importer responsibility. No Warband package/artwork changes are required. No website preview/catalog/publication work is within scope.

`reviews/human-approval.json` is absent; QA created no approval. No public catalog, deployment, staging, commit, push, or distribution occurred.

Physical Pixel display, live authority import and in-WoW keyboard delivery remain **pending parent/device verification**. Live export is currently blocked by preexisting configuration/catalog orphan-map discrepancies; QA did not mutate or repair production state. This does not invalidate isolated planner/native evidence, but must not be represented as a successful live import or gameplay test.

## Final CLI planning delta — independent QA addendum

Verdict remains **qa-pass**. Reviewed final `thumbleLinkPlanningConfiguration` and its call site in `Sources/CLI/ThumbleCLI.swift`, plus the isolated local-artwork regression in `scripts/verify-thumble-link-cli.py`. No native visual or import layout/material changes occurred; prior native evidence remains applicable.

The CLI now reads the authority catalog, then the existing editor snapshot endpoint for each profile, using the catalog configuration revision on every call. It checks returned revision equality and rejects draft snapshots. It composes a local configuration envelope for the shared import planner; the resulting single imported profile is committed through the existing authority `.import` path with the same expected configuration revision. The backend bounds requests, validates typed responses, and carries the revision through its normal request. No portable export policy is relaxed and unrelated local asset payloads are not removed. This matches the Mac's local planning capability while preserving commit concurrency checks.

Independently reran the final acceptance script:

```sh
python3 scripts/verify-thumble-link-cli.py build/ThumbleControllerImport/bin > /tmp/thumble-controller-qa-cli-tests-final.log 2>&1
```

Exit **0**; final log reports all Thumble Link CLI checks passed. The new regression seeds a saved-domain fixture only in a private temporary child-process HOME, verifies portable `profile export --all` still fails with local embedded artwork, verifies WoW dry review and actual refresh succeed, and compares the unrelated asset library unchanged after import. Earlier create/refresh/layout/metadata/invalid-input assertions also pass in the same run. QA neither accesses nor edits live profile state.

Parent reports the real Hunter dry review now passes and live application is in progress; this addendum does not claim it completed. User-authorized orphan-reference repair and same-value modifier alias deduplication are separate production-state operations, outside this isolated code/native QA verdict. Physical Pixel display and WoW input remain parent/device checks.
