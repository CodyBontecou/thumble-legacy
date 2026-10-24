# Classic Hunter Link — independent visual critique 1

Verdict: **revise**. No human approval is granted.

Evidence: exact `native-review-1/contact-sheet.png` (SHA-256 `7de86642a04f6bacd77b0aa1323996bc8a4ea4b8c16d14c21ef9f6c9e4b5335e`), all sixteen individual native PNGs viewed, and `native-review-1/native-layouts.json`. Reviewed against `art-direction.md` and the skill quality bar. This is native import layout/material work; community package, SVG, canonical package compatibility, and package hash gates are conditional, not findings against this implementation.

## Pass 1 — composition

The landscape composition separates left movement cross, central recovery, and right action block immediately. Portrait recomposes the menu and target controls into two rows rather than squeezing landscape. Sixteen imported commands and the native system toggle remain visible. Measured target minimums are 52 pt portrait and 48 pt landscape; minimum hit-frame separation is 8 pt in all four geometry records. There are no overlapping hit bounds or reference-viewport boundary violations. This does not verify a physical Pixel's cutout or system insets.

### Major 1 — movement captions break and utility captions truncate

- **Affected:** All portrait panels for movement; all landscape panels for targeting, in both appearances and every state.
- **Evidence:** Portrait Forward renders `Forwa` / `rd`; Move backward becomes `Move` / `back…`. Landscape target controls render `Target` / `nearest e…` and `Target` / `previous…`. These are visible in full-size individual frames, not contact-sheet scaling artifacts. Dynamic Raptor Strike wraps cleanly and need not change.
- **Likely cause:** Long imported action strings are used directly in narrow control captions despite the brief allowing short native display captions.
- **Bounded fix:** Use generated display captions `Back`, `Target`, and `Previous` for these known command purposes. For Forward either use an unambiguous `Up` display caption or make its portrait native label fit `Forward` without character wrapping (for example a measured smaller caption font confined to movement, at least 12 pt). Retain complete imported labels in accessibility names and command metadata. Do not change UUIDs or outputs. Acceptance: every sample control displays its complete chosen caption with no ellipsis, no midword break, and at most two caption lines in all sixteen rerenders.

### Minor 1 — portrait vertical gaps weaken the hand-zone composition

- **Affected:** All portrait panels.
- **Evidence:** Utility row ends at y=276 pt; the next control starts at y=404 pt, leaving 128 pt empty. Recovery ends at y=692 pt on an 874 pt authored safe viewport, leaving 182 pt blank below. The movement and combat clusters are recognizable, but the content reads as floating groups.
- **Likely cause:** Fixed independent vertical anchors do not distribute the portrait safe viewport deliberately.
- **Bounded fix:** During an explicit initial/re-layout operation only, bring the utility-to-main empty interval down to at most 96 pt and distribute the recovered height into a lower thumb region/recovery strip, keeping at least 8 pt gaps and current action target sizes. Do not move existing user controls during refresh. This is a secondary composition refinement; do not solve it with decorative filler.

## Pass 2 — material and craft

Light uses warm ivory faces against a muted warm ground; dark uses charcoal-green faces with muted green movement borders and brass action borders. Edge radii and shading agree across roles. Decoration stays subordinate and no raster seams appear. This fulfills a bounded text-led native feature, although it does not provide the spell-image richness in the user's Warband reference.

### Minor 2 — active face repeats pressed depression

- **Affected:** All active panels, most obvious in light.
- **Evidence:** Active surfaces retain the same gray/darker face as pressed; the thick edge is what distinguishes them. The state is visibly distinguishable, but active feels depressed rather than a sustained selection on an ordinary face.
- **Likely cause:** Active derives its surface from the pressed native fill.
- **Bounded fix:** Use ordinary face color for active with its existing continuous stronger edge; keep the inset depressed fill exclusive to pressed. Confirm active/pressed still differ at contact-sheet scale, with stable centers and label baselines. This is an alignment refinement to the art direction, not a demand for simulated persistent WoW state.

## Pass 3 — interaction and accessibility

Normal, pressed, active, and disabled are distinguishable in the contact sheet for movement and brass materials. Pressed has an inset depth cue; active has a stronger continuous border; disabled is muted. Labels do not visibly jump between control groups. Native hit frames remain the behavioral authority; visual review cannot prove release handling or keyboard delivery.

### Major 2 — disabled key hints fail the stated contrast target

- **Affected:** All disabled panels, both orientations and appearances.
- **Evidence:** The complete interior color of the portrait Attack `1` hint is approximately RGB (140,137,130) on (240,236,224) in light, a contrast ratio about 2.96:1; dark is (123,130,131) on (28,41,43), about 3.83:1. These are interior glyph pixels rather than antialiased fringe samples. Disabled main captions remain readable (light about 4.57:1; dark about 5.58:1), so the issue specifically concerns the smaller truthful key hints used during setup/testing.
- **Likely cause:** Disabled opacity compounds the normal secondary-hint opacity.
- **Bounded fix:** Give disabled hints an explicit resolved foreground that reaches at least 4.5:1 against the disabled face in each appearance. Reusing the disabled primary-caption resolved color is sufficient for this fixture; preserve the disabled material treatment to communicate unavailability. Verify actual native pixel colors after rendering, including Shift-Tab, Shift-B, punctuation and system legend if present.

## Originality and scope check

No Blizzard logos, borrowed spell textures, console marks, proprietary glyph arrangement, or Warband Warrior identity appear. The cross and two-by-two action group are generic ergonomics; this arrangement and restrained palette are distinct from the provided Warband shell. Text-led ability captions accurately represent current protocol capabilities. Stable identities, exact modifier outputs, refresh preservation, and Android delivery need independent functional QA and cannot receive approval from these images.

## Final disposition

**revise** until Major 1 and Major 2 are corrected and the exact next native sheet is independently reviewed. The two minor refinements are bounded opportunities, with no invented artwork or package required. Highest priorities: eliminate malformed/truncated captions; restore disabled hint contrast; improve portrait space distribution without altering normal refresh behavior.
