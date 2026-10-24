# Classic Hunter Warband — independent visual critique 2

**Verdict: revise.** The four previous major findings are corrected, but actual native keyboard-hint contrast has a further major failure in light pressed/disabled states. No human approval or publication is implied.

Exact reviewed sheet: `native-review-4/contact-sheet.png`, SHA-256 `f9e36e9888782eac957723d6759c474c60ddda2fdf147b2d3f5918bb9f6fc56e`. All sixteen full-resolution PNGs were independently viewed, with `native-layouts.json`, `pixel-evidence.json`, `instrument-glyph-contrast.json`, `quality.json`, revised art direction and execution notes. Compared against `sources/warband-reference.json`, its slot/command map, and the original Warband native landscape reference. This is a local user-authorized derivative and native profile feature; no public catalog or human approval claim applies.

## Pass 1 — composition and exact reference continuity

The requested Warband arrangement is preserved: shoulder pairs, central capsules, lower shortcuts, Move instrument, circular Cursor instrument in the Look frame, and two four-tile diamonds. All eighteen reference controls plus system toggle are present. Four duplicates are truthfully labeled Map/Character/Jump/Self. Hunter identity and the five real supplied icon textures replace Warrior content. Portrait retains the reference's intentional open vertical intervals; these are not a reason to redesign the layout.

I independently compared the eighteen mapped native frames against reference element centers/scales and the current native base-size formula, including View/Menu multipliers and the compensated Cursor frame. Maximum delta of center/width/height is effectively zero (floating-point error below 0.000001 pt), exceeding the required within-2-pt continuity. All four geometry records have nineteen controls including toggle. Minimum hit dimension is 44.10176 pt portrait and 44.1984 pt landscape. No hit rectangles intersect or exceed their respective 402×874/874×402 reference canvases. Minimum hit-frame separation is 11.01955 pt portrait and 10.29547 pt landscape. Physical phone insets remain separate QA.

**Previous Major 1 resolved:** Tab and Shift-Tab now occupy independent exterior positions beneath Target/Prev faces, with complete glyphs and no collisions. Supplied glyph evidence reports a minimum socket clearance of 5.64 pt; full-size frames agree that the tags are visibly separated. Main slot geometry remains unchanged.

## Pass 2 — material, artwork and typography

**Previous Major 2 resolved:** Light frames now use the actual warm ground RGB (232,226,213), while dark retains (17,23,27). Light native faces, passive header and keyboard tags agree with that ground. Both light and dark read as the requested Warband material system rather than unrelated themes. Brass sockets, original icon colors and restrained circular instruments retain the reference hierarchy. No visible raster seam, clipped socket, distorted icon or artwork/frame mismatch appears.

The five tiles visibly correspond to the retained source imagery: Water at left top, Food at left bottom, Auto Shot at right top, Attack at right left, and Raptor Strike at right right. Provenance/CASC identity validation is functional/source QA; this visual review does not independently re-extract game files. There is no Warrior placeholder art, fabricated cooldown state or camera-control legend.

**Previous Major 3 resolved:** The portrait diamond caption now reads the complete `Jump`, with no ellipsis in any of the eight portrait frames. Target/Prev, shoulders, central capsules and duplicate shortcuts remain readable. Native labels retain stable baselines across states.

## Pass 3 — interaction and meaningful contrast

Normal retains thin brass edges and matte faces; pressed is inset with a pressure boundary; active returns to the normal face with a thicker brass boundary; disabled visibly mutes faces/icons while retaining grouping. All four states are distinguishable in both schemes and orientations. The circles remain native instruments visually; their actual movement, tap and release behavior requires functional QA.

**Previous Major 4 resolved:** `quality.json` now reports zero issues for the exact captured artboard, and actual Cursor text agrees with the explicit face. I independently sampled Cursor central glyph interiors in all sixteen PNGs: light normal/pressed/active 7.371:1, light disabled **5.005:1**; dark normal/pressed/active 7.813:1, dark disabled **6.398:1**. The supplied 48-case instrument report covers Move/Cursor/drawer and has minimum 5.005:1, consistent with visible complete glyphs. This does not establish contrast for the smaller secondary keyboard hints.

### Major 1 — native small keyboard hints fail light pressed and disabled contrast

- **Affected panels:** `portrait-light-pressed`, `portrait-light-disabled`, `landscape-light-pressed`, `landscape-light-disabled`. The issue affects native secondary hints on shoulder capsules, central duplicate capsules and lower duplicate shortcuts; confirmed samples include M, Shift-B, F1 and Space.
- **Actual visible/pixel evidence:** The fully covered M hint interior is RGB (76,77,68) on pressed face (194,182,156), **4.267:1**. Disabled M is (100,100,94) on (210,205,193), **3.755:1**; Shift-B/F1/Space are approximately (100,100,95), **3.752:1**. These are repeated modal glyph-interior pixels, not antialiased fringes. Landscape M independently resolves to the same exact pairs. Light normal/active hints are 6.115:1; dark sampled hints are at least 6.315:1. The failed hints look noticeably faint compared with the full-strength passive action tags.
- **Likely cause:** Native secondary keyboard-hint opacity compounds the pressed material/disabled compositing, while the strict checker validates the main foreground and the added instrument evidence covers only Move/Cursor/drawer. The zero-issue style report does not measure this secondary text path.
- **Bounded correction:** Strengthen the resolved small native hint foreground for the light pressed and disabled states, preferably by removing the additional secondary-hint opacity in those states or using the resolved main-caption foreground. Preserve reference centers, fonts, baselines, outputs, icon artwork and state hierarchy. Require actual composited fully covered M, Shift-B, F1 and Space pixels to reach **at least 4.5:1** in both orientations and all four states. Add these hint cases to pixel evidence; do not treat instrument-only or style-token contrast as proof. Dark appearance can retain its current satisfactory treatment.

## Authorization and scope

Near-identical reuse of project-owned Warband design is expressly requested by the user and recorded in the revised brief. It supersedes the generic originality recommendation; it is not a defect. Blizzard imagery retains the documented provenance/attribution. This review neither grants human approval nor verifies package determinism, refresh ownership, asset synchronization, gameplay input, key release, or the headless product boundary. Those remain independent QA requirements.

## Disposition

**revise** until the light native keyboard hints meet the explicit 4.5:1 requirement and a new immutable native sheet is reviewed. Highest priorities: (1) correct light disabled hint contrast; (2) correct light pressed hint contrast; (3) extend pixel checks to the secondary hint path so the strict-style/instrument reports cannot miss it. Reference continuity, prior key-tag collision, actual light ground, complete Jump caption and Cursor contrast all pass this revision.
