# Classic Hunter Warband — independent visual critique 1

**Verdict: revise.** No human approval or publication is implied.

Reviewed exact `native-review-1/contact-sheet.png`, SHA-256 `acb1ad4e966e0ac59ce108f90b17e37d21f62b707ae930837ca48d85af5d2331`; all sixteen full-resolution individual PNGs; `native-layouts.json`; `quality.json`; current art direction; original Warband landscape reference and source/profile context; and the five source PNGs with `sources/provenance.json`. Native evidence governs this critique. The task is a local native/profile-owned derivative, with an implementation package used for assets; it is not a community publication request.

## Pass 1 — composition and reference continuity

The landscape normal view closely preserves the chosen Warband arrangement: shoulder capsules, central View/Menu positions now truthfully labeled Map/Character, Move circle, retained Look circle now labeled Cursor, both icon diamonds, and bottom shortcuts. The former generic grid and movement cross are gone. Hunter identity and five appropriate textures replace the Warrior content. Portrait preserves the reference's authored arrangement rather than stretching landscape. Its large open vertical intervals are intentional reference continuity and are not requested for redesign in this pass.

The geometry capture has 19 controls including the system toggle, thus eighteen reference frames. Four command shortcuts are visibly and truthfully duplicated. Minimum measured hit dimension is 44.10176 pt in portrait and 44.1984 pt in landscape; no pair of recorded hit rectangles intersects. Minimum edge-to-edge separation, using Euclidean rectangle distance, is 11.01955 pt portrait and 10.29547 pt landscape. Frame dimensions pass this reference-canvas check; physical device insets remain functional QA.

### Major 1 — key tags collide with native target faces

- **Panels:** Both orientations, both appearances, all four states; clearest in landscape-dark normal and portrait-dark normal.
- **Visible evidence:** `Tab` sits immediately against Target, and `⇧Tab` runs into/under the Previous tile's left edge. Part of the previous-target hint disappears. In portrait this reads as a merged `Target Tab … Prev` band across the diamond. These are genuine overlaps in the full-size frames, despite nonoverlapping native hit bounds.
- **Likely cause:** The original single-glyph diamond center tags were replaced with multi-character keyboard strings without relocating their artwork anchors/available widths. The artwork and native faces have distinct clipping/stacking behavior.
- **Bounded correction:** Place `Tab` and `⇧Tab` below their respective Target/Previous faces in the available lower exterior space, centered on each control; reserve their own text rectangles rather than filling the diamond center. Preserve all native centers, UUIDs and hit frames. Require every tag's full glyph bounds to remain at least 4 pt from any other face/socket or tag at 1×, with no clipped glyphs in all sixteen rerenders. If native key hints can achieve this reliably, prefer those over hardcoded SVG strings.

## Pass 2 — material, artwork and legend craft

Dark brass, charcoal faces, shallow native inset states and socket framing carry the reference convincingly. The five original 64×64 source images visually match their rendered positions: Water at left top, Food at left bottom, Auto Shot bow at right top, Attack sword at right left, Raptor Strike sword at right right. Their PNG SHA-256 values match provenance. Recorded CASC texture IDs are respectively 132794, 133972, 135493, 135641, 132223; the provenance records spell/item identity and retained BLP originals. This review verifies presented source/evidence consistency, not an independent re-extraction of the installed CASC archive.

### Major 2 — light appearance uses dark ground beneath dark artwork legends

- **Panels:** All eight light frames.
- **Visible evidence:** The light native faces are ivory, but the full background remains approximately #11171B, the dark ground. `WARBAND`, `HUNTER`, portrait identity text, and all passive key tags use dark light-mode ink on that dark background. The action keys `1`, `2`, `3`, `Space`, and recovery punctuation are nearly invisible at ordinary size. This is visible across the whole light row of the sheet and in the individual PNGs.
- **Likely cause:** The light CSS/material tokens update native controls and SVG ink, while the background/appearance path still resolves the profile's dark ground. Declaring the intended CSS ground alone has not changed the actual canvas.
- **Bounded correction:** Resolve the native preview/client canvas to the brief's light ground #E8E2D5 for light appearance, preserving dark ground #11171B for dark. Keep the authored light SVG ink; do not work around the mismatch by globally turning all legends white. Require rendered light background pixels to match the intended light canvas, meaningful tags/header ink ≥4.5:1, and native controls/socket edges visibly consistent in all states.

### Major 3 — portrait Jump caption is truncated

- **Panels:** All eight portrait frames.
- **Visible evidence:** The right diamond bottom native face reads `Ju…` instead of Jump. The separate Space key tag does not identify the action completely. The duplicate bottom shortcut above the instruments reads Jump fully and demonstrates the intended caption.
- **Likely cause:** The ordinary 13 pt caption/padding is too wide for the portrait 41.808 pt visual diamond face. Target and Previous already have smaller purpose-specific treatment, but Jump does not.
- **Bounded correction:** Apply a purpose-specific Jump diamond caption size/padding that renders the complete word `Jump` at no less than 11 pt without scaling the native face or changing geometry. Keep the duplicate Jump shortcut's normal typography. Require no caption ellipses in either orientation/state set, retaining full accessibility labels.

## Pass 3 — states, interaction and accessibility

Normal, pressed, active and disabled differ visibly: inset/pressure edges in pressed, strong brass boundaries on ordinary faces in active, flatter muted icons/faces when disabled. Icons remain prominent, retain their original content, and carry no fabricated cooldown information. The actual native Cursor circle has a readable central legend in the reviewed PNGs and does not claim camera behavior. Center/baseline alignment appears stable across the sheet.

### Major 4 — Cursor style's declared contrast fails strict evidence

- **Panels:** Light appearances and base Cursor style, all four states; `quality.json` contains 24 low-style-contrast errors at 1.47:1 for the Cursor-associated style.
- **Visible evidence and limitation:** The actual native light Cursor text is dark on the ivory puck and looks readable in this exact sheet; it is not visually honest to describe that particular PNG label as 1.47:1. The strict style model nevertheless reports 1.47:1, so the declared effective face and actual SVG-backed puck disagree or are modeled inconsistently. This blocks treating the implementation package as quality-passing.
- **Likely cause:** An SVG-backed/transparent trackpad surface does not declare a corresponding solid effective face for contrast evaluation, or the validator evaluates the wrong underlying face.
- **Bounded correction:** Give Cursor an explicit solid scheme-specific effective native face and foreground matching the central puck, or correct the contrast model to evaluate the actual declared artwork-backed face. Keep the concentric circle geometry and pointer/tap behavior. Require the existing strict checker to return no contrast errors, and separately measure the complete rendered Cursor glyph interior against its puck at ≥4.5:1 in each state. Do not silence the validator or invent a white text workaround over an ivory puck.

## Originality and authorization check

Near-identical reuse of the project-owned Warband arrangement/materials is explicitly authorized by the user and recorded in art direction; it supersedes the skill's generic non-copying recommendation for this task. This is therefore not a finding against originality. Actual Blizzard textures retain attribution/provenance. No new console marks, unrelated Warrior art, fictitious bank states, invented activity, or camera claim appears. Existing source/package preservation and output/refresh ownership require separate QA.

## Disposition

**revise**. Resolve all four major findings before a visual pass. Highest priorities: make the light ground and passive legends agree; give Tab/Shift-Tab independent unclipped tag positions; fit the full portrait Jump caption. Cursor strict contrast/effective-face consistency must also pass before final QA. The next review must use a newly versioned immutable native sheet; this sheet is not approved.
