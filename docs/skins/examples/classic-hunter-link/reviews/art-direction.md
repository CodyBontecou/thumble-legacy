# Classic Hunter Link — art direction

## Concept and scope

A Hunter’s field keypad: thumb-sized command islands in matte charcoal and warm brass, with movement at the left hand and the character’s actual combat commands at the right. The target is confident, immediate play, with the calm physical hierarchy the user values in Warband.

This is principally an import-layout feature. The importer must generate a useful native keyboard controller from the snapshot; decorative skin artwork must never supply missing layout, semantics, or behavior. Keep layout generation, outputs, and appearance separate. Existing `warband-command` source, package, and approval remain untouched. Its Warrior artwork, fixed UUID assignments, gamepad outputs, trigger banks, and `WARBAND / WARRIOR` identity must not carry into an imported Hunter profile.

Originality boundaries: reuse the project’s native material capabilities, not a copied artwork composition. Author a new, spare arrangement of group edges if a review skin is needed. No Blizzard HUD tracing, ability textures taken from another class, console silhouettes, decorative runes, brand marks, invented cooldowns, or fabricated spell images. Text-led combat tiles are honest for the current snapshot protocol, which explicitly does not synchronize pictures.

## Binding inventory and hierarchy

The supplied example contains 16 supported commands, within a 17-row export that may include an unavailable command. Count resolved commands rather than blindly reserving seventeen tiles. Preserve each imported command ID and exact resolved output, including modifiers. Unsupported/unbound commands remain in import review, never acquire a guessed output.

| Group | Commands | Native treatment |
|---|---|---|
| Move | Forward W, Backward S, Strafe left Q, Strafe right E | Four individually held digital buttons in a left cross; movement semantic role |
| Fight | Attack 1, Raptor Strike 2, Auto Shot 3 | Three large right-side tiles; primary action semantic role |
| Jump | Space | A fourth right-side tile, below the combat trio |
| Target | Nearest Tab, Previous Shift-Tab, Self F1 | Upper right utility strip |
| Recover | Refreshing Spring Water -, Tough Jerky = | Two central lower secondary tiles |
| Character | World map M, Bags Shift-B, Character C | Upper left menu strip |

The output is whatever the export actually resolves, including punctuation; the table is the provided example, not a replacement binding specification. Use short, readable display captions where needed (`Forward`, `Back`, `Strafe L`, `Strafe R`, `Water`, `Food`, `Target`, `Previous`, `Self`), while accessibility names retain the full imported action label. Dynamic ability labels remain native text and may wrap to two lines. Key hints remain truthful, subordinate, and visible during setup/testing.

Do not add a gamepad Move/Look stick or LB/LT/RT controls without exported keyboard equivalents. Four directional buttons preserve individual snapshot identities, hold/release behavior, diagonals through simultaneous touches, and refresh semantics. If the existing native keyboard joystick already supports independent part outputs and release behavior, a single left Move instrument may replace the cross, provided its four parts retain exactly the imported W/S/Q/E outputs, command-to-part ownership is explicit, and refresh/manual-edit preservation is proven. Never convert those commands to gamepad axes. No modifier banks are needed for this sixteen-command example.

## Orientation composition

**Landscape:** use the safe content rectangle, excluding device cutouts and system UI. Organize the upper band as Map/Bags/Character at left and Target/Previous/Self at right, leaving the system drawer toggle centered. The main left cross occupies approximately x=0.09–0.37, y=0.34–0.88. The combat cluster occupies x=0.61–0.93, y=0.34–0.88; Attack and Raptor Strike are the strongest pair, with Auto Shot and Jump directly beneath. Water/Food occupy the center below the utility tier, approximately x=0.40–0.58; stack them if needed rather than shrinking combat controls. Negative space between the two hand zones communicates grouping. Do not spread all commands into two full-width rows.

**Portrait:** compose anew. Two rows of three utility/menu controls sit below the top safe area and system toggle. Movement occupies the left half of the middle/lower area and the two-by-two Fight/Jump cluster the right half; both remain in reachable thumb regions. Water/Food form a quiet lower strip above the home-indicator safe area. Avoid a large empty vertical center and avoid stretching landscape positions into a narrow canvas. Use measured native frames to settle the exact normalized coordinates; the above describes hierarchy, not binding or hit-frame source data.

Maintain at least 44×44 pt native touch targets and 8 pt gaps at the review viewport in each orientation, preferably 52–64 pt for movement and combat. Caption text must wrap instead of shrinking into illegibility. If additional bound bar actions arrive, keep the fixed movement/target/menu anchors and create clearly grouped native overflow/bank behavior only if already supported; never silently discard actions or guess WoW bar modifier behavior. A bounded first implementation may use deterministic additional rows with an explicit import review warning.

## Palette and material system

| Token | Dark | Light | Purpose |
|---|---|---|---|
| ground | #111A1C | #E8E4D8 | Quiet canvas |
| face | #253638 | #F7F2E7 | Touchable surfaces |
| well | #0B1213 | #CBCBBE | Recess below movement/action groups |
| ink | #F4ECDA | #242F2C | Native captions |
| muted ink | #C2C7B8 | #4D5B52 | Secondary key hints |
| brass | #C6A467 | #725021 | Thin structural borders |
| leaf | #A9C6A6 | #3E6243 | Movement and restrained Hunter identity |
| pressure | #FFE0A2 | #44300E | Clear held feedback |

Verify actual rendered text contrast at 4.5:1; large group edges at 3:1. Light is warm ivory with forest-colored instruments, not an inverted dark skin. The dark surface is matte green-charcoal, with brass used sparingly to separate native controls. Five depth tiers: ground, group recess, native face, upper-left edge highlight, native legend. Borders 1–1.5 pt, 6–10 pt radii, shallow shadows at most 2–3 pt. One upper-left light direction. No gloss, glow, giant rings, rivets, or texture behind captions. Optional texture is authored SVG hatching below 4% contrast, outside touch faces.

## Native states

Normal uses the matte face and thin brass edge. Pressed/held uses a darker inset face, 2 pt pressure edge, and inset shadow, without moving or scaling the label. Active uses a continuous 3 pt leaf/brass edge and ordinary face to distinguish it from depression; do not portray WoW Auto Shot as persistently active without actual state data. Disabled uses a flat muted surface and readable ink, no raised shadow. Native accessibility and hit testing are preserved for all states. SVG supplies only passive group framing, never simulated controls, labels, or state marks.

## Implementation and canonical compatibility

The current importer creates equal-grid positions in `update`; replace initial placement with explicit command-group layout and semantic roles. Preserve deterministic command IDs, exact keyboard outputs, default-output baselines, and the existing refresh rule that keeps positions, deleted controls, labels, and edited outputs. Restyling an already imported grid must be an explicit reset/re-layout operation, with Mac and CLI parity; ordinary refresh must not unexpectedly move a user’s controls.

Reusable native materials or profile-owned appearance should work for arbitrary imported IDs by role/purpose/group. Prefer this bounded implementation; no new public skin, package installation, or general-purpose template registry is necessary to solve the import. Warband's ergonomic geometry can inform placement, including its existing keyboard-capable native joystick, while dynamic imported controls retain their own identities. Native colors, borders, roles, and readable legends are sufficient for the first implementation. Passive original artwork is optional and should remain local and profile-owned if added.

Never apply the existing Warband package as though `wow-forever-v1` were a compatible keyboard artboard. That canonical template includes gamepad identity and different geometry. If the feature adds a canonical imported keyboard template, register and commit its separate geometry contract and capture both orientations before drawing. Otherwise use schema 3 captured artboards for the exact generated fixture profile, retaining fixture snapshot and profile JSON with the workspace. Captured control semantics must match those imported IDs. Aligned artwork must hide on incompatible geometry; semantic material styling may continue.

If a new appearance package is authored, retain `skin-source.json`, `sources/artwork/*.svg` (separate portrait/landscape and light/dark), optional CSS, exact captured artboard/fixture, and original provenance. Native evidence is required even for text-led controls: a contact sheet of portrait and landscape × light and dark × normal, pressed, active, disabled. Do not claim the package applies to every future character snapshot merely because the example fixture passes.

## Acceptance criteria

1. All 16 supported sample commands have distinct stable native controls with the exact imported key/modifier output. Unsupported export rows are clearly reported; no invented actions, gamepad outputs, spell art, or cooldown state appears.
2. Movement and combat groups are identifiable at thumbnail size; neither orientation presents a uniform sixteen-button grid. Movement is a four-button digital cross or a verified four-part keyboard Move instrument, and combat/jump remains a distinct right-hand group.
3. Native hit targets measure at least 44×44 pt with 8 pt separation at each review viewport; no overlapping bounds, safe-area intrusion, clipped caption, or illegible text shrink remains. Long labels wrap to two lines or use an honest short display caption.
4. All native legends meet 4.5:1 contrast. Both appearances show authored material hierarchy, consistent upper-left lighting, and subordinate decoration.
5. Normal, pressed, active, and disabled are visually distinguishable for every semantic material in a full 16-panel native contact sheet. State changes preserve control centers and label baselines.
6. Re-export refresh preserves user positions, deleted controls, manual labels, and manual outputs; any re-layout uses an explicit Mac and CLI operation. Both orientation customizations remain consistent with the same command inventory.
7. If a package is produced, strict compiler/quality validators pass, two rebuilds match byte-for-byte, editable source remains, and two independent visual critiques reach visual-pass before independent QA. Final contact-sheet path and exact package SHA-256 are recorded with human approval pending.
8. Existing Warband assets and package remain untouched. New framing aligns to the actual captured role frames within 2 pt, and incompatible canvas artwork is suppressed rather than stretched or silently misapplied.

## Strongest risks

Grouping alone will not fix text if the renderer’s default width scales remain too narrow; measure final native frames on both phone orientations. A text-led keypad can meet the user’s custom setup request, but does not yet reproduce Warband’s spell-image richness; disclose that current protocol limitation. Automatic migration of the already imported grid conflicts with preserving user edits, so expose a deliberate layout action. A digital cross must release each held output reliably when touches end or the controller disconnects. Schema/package evidence is separate from native product behavior: passing appearance QA does not prove keyboard delivery in WoW.
