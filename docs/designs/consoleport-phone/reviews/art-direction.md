# ConsolePort phone — art direction

Status: direction only; no human visual approval. Reference: user photo `IMG_6152.JPG`, reviewed 2026-10-21. Delivery contract: a profile-owned design edited through local CLI/MCP, with editable SVG and native-renderer review evidence. Legacy skin packages and desktop GUI are not the authority.

## Concept

An instrument panel of two paired action diamonds: large live ability pictures in original charcoal circular wells, with small controller legends, so a glance between game and phone preserves the same action geography.

The user explicitly requests ConsolePort's functional grouping. Preserve the left directional diamond and right Y/X/B/A diamond, and tuck corresponding shoulder/trigger actions beside their clusters. This is information architecture, not a hardware replica. Use the already imported game icons as local live content; do not redraw, distribute, or bake those icons into the original frame artwork. No ConsolePort artwork, medallions, WoW ornament, Xbox logo, traced hardware silhouette, branded typeface, or manufacturer shell. Original work is the restrained charcoal palette, bevel construction, spacing, and legend system.

## Landscape composition

Target current landscape runtime only. Use a clean dark canvas without an illustrated controller shell. Left and right action diamonds must be immediately recognizable and have equal diameter/rhythm. Preserve their cardinal ordering. Shoulder and trigger actions belong to their respective side, outside the diamonds' silhouette, rather than a detached large top-row stack. Keep movement and camera pads accessible in lower outer thumb zones, with their existing semantics and sensitivity. View/Menu and stick clicks remain subordinate but usable; leave clear central negative space. Prefer shifting whole groups over interleaving movement and action targets. Respect the phone's cutout/insets and control bar.

Do not infer control identifiers, outputs, or action-bank assignments from the photo. Bind the composition to the selected profile's actual element IDs and semantic outputs. Establish the canonical artboard by exporting that profile's current landscape geometry before editing. Retain before/after geometry and an exact profile-state backup. Work on a reviewable owned-design artifact before live application.

## Portrait boundary

Portrait is not supported by the current product contract. Do not restore portrait runtime or stretch this landscape composition. Retain any migration-era portrait data untouched; mark portrait previews not applicable. A future portrait design needs a separately composed artboard and review.

## Palette and material

| Token | Dark appearance | Light appearance | Purpose |
| --- | --- | --- | --- |
| canvas | #101214 | #25292D | Dark game-oriented ground in both appearances, intentionally avoiding the white current canvas |
| well | #191C20 | #202429 | Recessed circle beneath native icon |
| rim | #69717B | #7A848F | Thin, clean boundary |
| upper bevel | #9CA5AD | #AFB8C0 | Restrained upper-left light |
| lower bevel | #080A0D | #111417 | Lower-right depth |
| legend | #F1F3F5 | #FFFFFF | Small, high-contrast identifying labels |
| active | #E2BE69 | #F2D28A | Restrained active-bank emphasis |

Use canvas → shallow recess → narrow bevel → native icon → native legend. Upper-left light is consistent for all controls; no chrome or glossy lens. Rim width approximately 1–2 logical points, bevel no more than 3 points, low shadow offset 1–2 points. No random noise needed. Ability images are the strongest color. Unassigned circles use a quiet original empty well rather than invented game emblems. Native legends stay outside or at the upper rim of the icon disc, never across the ability picture. At least 4.5:1 legend contrast; color only supplements readable labels.

## Native states and control contract

Normal: charcoal well with legible rim. Pressed: darker recess, reduced upper bevel and a crisp brighter outline; no translation or target drift. Active: warm perimeter or bank indicator distinct from pressed, with readable LT/RT identity. Disabled: desaturated/lower-contrast body while legend and silhouette remain readable. Preserve meaningful combined pressed/active feedback. Review both appearances and all Base/LT/RT/both-trigger banks; changes in action imagery must not resize controls.

SVG artwork is passive. Native controls retain hit testing, focus, accessibility names, haptics, and state. Use centered native original-color icon rendering large enough to fill the circle, plus separate small native controller labels where necessary. The existing importer currently emits `.top` icon placement and the Android icon baseline is 0.34 × minimum control dimension: a visual design must override placement/scale while preserving icon identity and ongoing managed icon refresh. Do not accidentally classify user-authored layout, labels, or original SVG as importer-owned content.

## Acceptance criteria

1. On the actual Pixel landscape viewport, two action diamonds maintain cardinal ordering, equal disc size, and consistent center spacing within 2 logical points; no label/icon overlaps or unsafe inset collisions.
2. Each primary action target is at least 48 × 48 dp, with at least 8 dp between non-overlapping adjacent targets. Movement/camera and all existing shoulder, trigger, utility, and stick-click actions remain reachable.
3. Ability pictures occupy 68–82% of the disc diameter, centered within 1 logical point; controller legends are secondary (approximately 11–14 sp at default font scale), readable, and do not obscure imagery.
4. Native light/dark contact sheets show normal, pressed, active, and disabled materials as distinct; legends achieve 4.5:1 contrast and active meaning does not depend only on color.
5. Base/LT/RT/both-trigger bank evidence shows correct live icon identities at stable positions; switching Thumbleapp → Bonteq → Thumbleapp preserves this design and restores existing profiles without duplicates.
6. Before/after semantic mapping comparison proves no output, axis mapping, sensitivity, haptic preference, or inaccessible hidden-control regression. Pixel input still produces jump, movement, camera, and trigger-modified actions.
7. Retain editable SVG/design JSON, native-rendered contact sheets and exact artifact hashes; two independent critiques and QA must pass before declaring the design finished. No publication or human approval is implied.
8. Icon refresh and reconnect preserve the new visual geometry, label treatment, and user overrides. No imported game icon is baked into reusable original frame assets; unrelated/default profiles remain unchanged.

## Strongest risks

Space pressure from paired diamonds plus both analog pads; small legends becoming inaccessible; existing importer `.top` placement defeating icon-centric framing; appearance overrides suppressing future managed icon refresh; platform renderer differences in clipping, bevels, and state treatment. Resolve these in the native renderer and actual Pixel evidence, not a static SVG mockup alone.

## Revision — 2026-10-21: circular live ability pictures

The user's clarification supersedes criterion 3's previous 68–82% diameter target. Scope is presentation only: clip each live ability picture to a true circle and enlarge it to approximately 90–92% of the action disc diameter, with a 1.5–2.5 dp visible frame on the target Pixel. Preserve the existing above-icon controller legends and every control's position, dimensions, output, and hit target. No further composition changes are requested.

Use the authored `content.iconMask: circle` / `-thumble-icon-mask: circle` presentation setting in the native and Android renderers. The default `none` preserves other profiles. Apply a circular clipping path to the rendered picture itself; rounding the surrounding button alone does not satisfy this direction. Preserve asset bytes, identity, imported game content, and managed refresh metadata. Existing square 32 × 32 source pictures may scale using the established fit behavior; aspect-fill is acceptable only without distortion or accidental image displacement. Never edit or regenerate the source bitmap to obtain the mask.

Acceptance for this revision:

- All four square image corners are absent outside the circular image aperture, including at diagonal edges; the circular frame remains visible around the enlarged picture.
- Native and Android previews show a centered, undistorted picture occupying approximately 90–92% of the disc diameter. Where a very small disc makes that ratio incompatible with the visible-frame target, preserve the frame and report the measured difference.
- Normal, pressed, active, and disabled states and all Base/LT/RT/both-trigger banks keep the same circular mask. State feedback remains on the native frame and never exposes square image corners.
- Above-icon controller legends retain their current placement and remain readable without overlap or clipping.
- A before/after comparison reports zero geometry, output, hit-target, source-asset-byte, or icon-identity changes. Character switching and subsequent live icon refresh retain the presentation mask.
