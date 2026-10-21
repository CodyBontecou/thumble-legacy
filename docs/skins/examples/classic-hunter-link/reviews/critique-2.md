# Classic Hunter Link — independent visual critique 2

Verdict: **visual-pass** for the native 16-command fixture. No human approval or publication is granted.

Evidence: exact `native-review-2/contact-sheet.png`, SHA-256 `01dae6ff48f6896b0c685c3badebff0c2b34011d1a90d37a5fd3e7b1874ddb97`; all sixteen full-resolution PNG panels were inspected independently, together with `native-review-2/native-layouts.json`. Review follows the art-direction acceptance criteria, skill quality bar, revision-1 report, and revised design specification. This is an importer-generated native layout/material feature. No appearance package has been authored, so package compiler, package determinism, and package approval gates are conditional. Original SVG studies are not runtime evidence.

## Pass 1 — composition

Landscape reads as four distinct functional groups: left movement cross, upper utility strips, center recovery pair, and right combat/jump block. Portrait independently recomposes the utility strips into two rows, with a left cross, right combat block, and lower recovery pair. Both avoid the uniform imported grid shown by the user. The layout is text-led and does not claim the illustrated spell richness of Warband.

All 16 supported command controls and the native drawer toggle are visible in every panel. Arithmetic checks against all four native geometry records found no intersecting hit bounds or viewport violations. Minimum native target dimension is 52 pt portrait and 48 pt landscape; minimum pairwise hit-frame separation is 8 pt. The portrait utility-to-main interval is now 96 pt (utility bottom 324, first movement top 420), improved from 128 pt. Recovery ends at 730 pt on the 874 pt viewport; the remaining lower space is balanced against the top system-toggle zone and leaves the recovery row distinct from the thumb clusters. No decorative filler is needed.

**Resolved revision-1 Major 1:** `Up`, `Back`, `Target`, and `Previous` render completely in both orientations, both appearances, and all four states. No ellipsis or midword split remains. Portrait `Strafe L`/`Strafe R` wrap at the word boundary, and `Raptor Strike` wraps at its space. The full `Refreshing Spring Water` caption fits two lines. Native key hints remain visible under captions, including modifiers and punctuation. Image review cannot establish accessibility-name or UUID preservation; functional QA must verify those separately.

## Pass 2 — material and craft

Warm ivory faces and a warm stone ground form a coherent light appearance. Dark uses green-charcoal faces against a darker ground; restrained leaf edges identify movement while brass connects utility, combat, and recovery. Consistent corners, edge widths, and restrained normal shadows establish a single native component family. No visible seams, clipping, misaligned artwork, raster artifacts, or texture interference appear. The short utility captions improve visual rhythm without inventing action imagery.

### Minor 1 — active still uses the depressed face

- **Affected panels:** All active panels, clearest in light.
- **Visible evidence:** Active retains the pressed gray/darker face; its continuous 3.5 pt edge is stronger and flatter than the pressed 2.5 pt inset edge. These states are distinguishable in the contact sheet and at full resolution, but active still suggests depression.
- **Likely cause:** Native active material inherits the pressed fill while overriding border treatment.
- **Concrete optional correction:** If a bounded native style capability is later available, retain the normal face for active and reserve the inset/darker face for pressed. Keep existing centers, caption baselines, and stronger continuous active edge; verify active/pressed remain distinguishable at contact-sheet scale. This is a tracked refinement, not an unresolved major or a request to expand the shared model in this change.

## Pass 3 — interaction and accessibility

All four states differ meaningfully for both leaf and brass materials. Normal is raised and thin-edged; pressed has inset depth and stronger edge; active has a thick uninterrupted edge; disabled loses the colored edge and raised emphasis while remaining readable. Control centers and caption/hint baselines stay stable between states. Active and pressed are adequately distinguishable despite the minor above. The images show no simulated cooldowns or persistent WoW state.

**Resolved revision-1 Major 2:** Actual composited disabled hint pixels now exceed 4.5:1. The sampled fully covered interiors of the Attack `1` hint are RGB (96,94,90) on face (244,239,228), **5.64:1** light; dark is (168,174,175) on (33,48,50), **6.09:1**. The same values were independently sampled in portrait and landscape. Portrait `Shift-B`, `Shift-Tab`, minus, equals, and `Space` interiors resolve to those same respective color pairs. Samples exclude antialiased fringes, borders, and captions; multiple fully covered pixels exist for every sample. The disabled drawer chevron measures **11.50:1** light and **9.36:1** dark. Small hints are clearly subordinate but no longer faint. Main captions have stronger contrast than the hints.

Reference geometry uses an already-safe 402×874 or 874×402 content viewport with zero explicit insets. These native captures do not prove physical Pixel cutout handling, Android typography, touch release, keyboard delivery, or ordinary-refresh preservation. Those remain separate device/functional QA responsibilities, not evidence-based visual defects in this sheet.

## Originality and scope

This work uses a generic movement cross and action grouping with an original restrained native palette. It contains no Blizzard textures or marks, copied console outline, Warrior identity, borrowed Warband art, fabricated spell images, or proprietary control glyph arrangement. Text captions honestly represent the imported snapshot's current capabilities. No new package or compatible-artboard claim is implied.

## Disposition and priorities

**visual-pass**: no blockers or unresolved majors. Portrait and landscape are composed, labels are complete, disabled hints meet the stated contrast target, and all interaction states read distinctly. The highest-priority review outcomes are: (1) caption clipping is resolved; (2) disabled hint contrast is resolved by measured native pixels; (3) the active inherited-face refinement remains minor and tracked. Proceed to independent functional/native QA, including the physical Android viewport; do not infer human approval from this verdict.
