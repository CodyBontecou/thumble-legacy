# Native design execution notes

This workspace contains an anonymized sample16-command native layout specification and original passive SVG framing studies. It is an import feature, not a community appearance package; package compiler/validation and a package hash do not apply unless a package is later authored. No publication or human approval is claimed.

## Geometry

`design-spec.json` records every center in safe-content normalized coordinates and reference points, plus exact hit width/height. Portrait402×874 and landscape874×402 are independently composed. The portrait movement cross is52×52pt and the combat cluster76×76pt; landscape movement is72×64pt and combat104×72pt. Utilities never go below48pt high. An arithmetic rectangle check confirms16 controls, no overlap, minimum8pt separation in both orientations. Repeat this check against actual native frames after safe-area transforms. Reference-screen positions must not be normalized against full screen while rendering against safe content.

Movement must remain4 independently held native buttons. Cross centers preserve diagonal combinations and full imported command identities. The16-item sample has no need for a Look stick, gamepad axis, modifier banks, or triggers. Feed imported semantics into these slots; preserve UUID and resolved output. Unknown additional actions need separate native overflow rather than silently replacing these anchors.

## Materials and typography

The material list uses real `ThumbleSkinMaterialSpec` property names. Map role to reusable material; never key the style to sample UUIDs. `menu` and `utility` share quiet brass, `movement` uses leaf edges, `primaryAction` has stronger brass, and `secondaryAction` reduces depth. Pressed scale is1 to avoid target or label drift. Revision2 uses native overall disabled opacity0.8 with neutral#737873 edge and raised shadow off; actual composited hint contrast must be remeasured. Renderer support for a2pt pressed edge should be verified rather than inferred from activeStrokeWidth.

Use two native caption lines,13pt minimum in this design, and a separate12pt monospaced hint. `Raptor Strike` may wrap at the space; do not clip it into `Raptor Strik`. Known movement/menu commands use short honest captions and full accessibility labels. The original water/food action names remain in accessibility and import review. Never use sample hints as outputs.

## Initial advisory-token contrast (superseded for native disabled review)

These were initial sRGB advisory-token ratios. The parent native implementation uses scheme-aware native fills/foregrounds and derived pressed fills without any appearance bank changes. In revision2 the native disabled opacity is0.8, so the ratios below do not establish actual native disabled contrast; review2 must sample composited glyph interiors and face colors.

| Pair | Ratio |
|---|---|
| foregroundColor / baseColor | 12.39:1 |
| darkForegroundColor / darkBaseColor | 10.74:1 |
| foregroundColor / pressedFillColor | 9.63:1 |
| darkForegroundColor / darkPressedFillColor | 13.76:1 |
| disabledForegroundColor / disabledFillColor | 5.47:1 |
| darkDisabledForegroundColor / darkDisabledFillColor | 7.08:1 |

## Editable SVG evidence and provenance

`sources/artwork/framing-*.svg` are original passive source studies with ground and three group recesses. They contain no labels, control faces, game imagery, copied Warband artwork, or fabricated behavior. Each can be reproduced with native rounded rectangles. They are intentionally not runtime media, since native framing/role materials can accommodate arbitrary imported IDs without artboard package dependencies. Do not apply these SVGs to a canonical `wow-forever-v1` artboard.

## Required review evidence

Parent implementation must supply native captures in both orientations, appearances, and normal/pressed/active/disabled state:16 panels. Review measured native frames, caption wrapping, actual colors and labels. The contact sheet is pending implementation; no visual-pass, QA-pass, canonical compatibility, or package approval is implied by these design files. Ordinary refresh must preserve manual placement/labels/outputs/deletion; applying this layout to an existing grid requires an explicit relayout operation in Mac and CLI.


## Exact revision2 changes after parent synthesis

Parent accepted review1 Major1/Major2 and the portrait composition minor. Known display captions are now **Up**, **Back**, **Target**, and **Previous**; full imported action/accessibility names remain. Both raw imported labels and legacy normalized12-character labels count as unchanged defaults eligible for short-caption refresh. Manually edited labels stay intact.

Portrait utility/menu rows move from190/250 to238/298pt. Movement anchors move to446/506/566pt; combat anchors to466/550pt. For this16-command fixture recovery moves from662 to700pt. Dense17–24-command layouts retain the earlier662pt recovery/632–692pt placement envelope to reserve footer space. Landscape geometry is unchanged. These are initial/explicit-relayout placements; ordinary refresh preserves existing positions. Arithmetic checks still find minimum8pt separation and no overlaps.

Disabled native controls use overall opacity0.8, neutral#737873 edges, and no raised shadow. Native hint contrast must reach4.5:1 after compositing. The old advisory-token contrast table cannot verify this; exact second native frames are pending.

The active inherited-face issue from review1 Minor2 is tracked: current native active uses pressed fill plus stronger continuous edge. Parent has not authorized a blanket redesign or wider model change. It remains a minor alignment refinement and must not be reported fixed. No appearance bank, portrait-bank API change, package, or new canonical artboard is required. Existing SVGs remain version1 passive framing studies, not revision2-aligned runtime evidence.
