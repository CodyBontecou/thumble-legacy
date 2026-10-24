# Classic Hunter Warband — revised art direction

## Intent and authorization

Keep the existing Warband controller’s geometry, brass-edged charcoal materials, quiet thumb instruments, and icon-filled action diamonds; replace its Warrior identity and example bindings with this Hunter’s actual exported spells, items, and keys. The emotional target is the same finished controller the user selected, personalized to their character.

The user explicitly requested near-identical reuse of the project’s Warband design. That request supersedes the skill’s general recommendation against copying another existing skin. Reuse the project-owned framing and composition deliberately; do not invent a different text-led keypad, green theme, or new hardware silhouette. Keep the existing `warband-command` workspace/package unchanged and author this derivative locally. Preserve Blizzard attribution for actual game textures; no console trade dress, unrelated Warrior icons, newly generated spell art, or invented cooldown state.

## Native geometry and command mapping

Use the existing `wowForever`/Warband native geometry as the starting layout in both orientations. The reference landscape canvas is 874×402 and portrait 402×874. Preserve the 52 pt action-diamond radii, 0.52 tile scale, 0.72 stick scale, relative positions, breathing room, system toggle, and orientation-specific safe areas. Inspect the supplied `preview/profile.json` and current native template rather than redrawing the screenshot approximately.

Landscape anchor centers: Move (120,225), Look (490,280), left diamond (285,275), right diamond (730,255). Portrait anchors: Move (95,570), Look (305,570), left diamond (95,750), right diamond (305,750). Shoulder capsules keep their reference positions at x=0.23/0.77 and landscape y=0.17/0.31, portrait y=0.17/0.25. Preserve the actual native resolved frames and scale from the reference, not merely these center summaries.

Map the Hunter fixture as follows; outputs come from each exported action’s resolved key, never from the icon or slot label:

| Position | Display | Exact example output | Visual |
|---|---|---|---|
| Move up/down/left/right parts | Move | W / S / Q / E | Existing circular Move instrument |
| Right diamond top | Auto Shot | 3 | Actual exported Auto Shot texture |
| Right diamond left | Attack | 1 | Actual exported Attack texture |
| Right diamond right | Raptor Strike | 2 | Actual exported Raptor Strike texture |
| Right diamond bottom | Jump | Space | Native readable Jump legend |
| Left diamond top | Water | - | Actual Refreshing Spring Water texture |
| Left diamond bottom | Food | = | Actual Tough Jerky texture |
| Left diamond left | Target | Tab | Native target symbol/short legend |
| Left diamond right | Previous | Shift-Tab | Native previous-target symbol/short legend |
| Upper left capsule | Map | M | Native text |
| Lower left capsule | Bags | Shift-B | Native text |
| Upper right capsule | Character | C | Native text |
| Lower right capsule | Self | F1 | Native text |
| Central View frame | Map | M | Native text; duplicate Map shortcut |
| Central Menu frame | Character | C | Native text; duplicate Character shortcut |
| Bottom L3 frame | Jump | Space | Native text; duplicate Jump shortcut |
| Bottom R3 frame | Self | F1 | Native text; duplicate Self shortcut |
| Original Look instrument frame | Cursor | Native relative pointer / tap to click | Circular native trackpad with reference gold ring |

This maps all 16 supported commands: four movement parts plus twelve distinct button commands. Retain all 18 original native control frames: sixteen buttons, Move instrument, and Cursor instrument. Four button positions intentionally duplicate existing commands as ergonomic shortcuts: View→Map, Menu→Character, L3→Jump, R3→Self. Their labels and key hints describe those actual outputs, never the old gamepad identities. This preserves the near-identical Warband composition the user explicitly prioritized and supplies useful central/bottom shortcuts without inventing game actions. Record duplicate ownership explicitly so a refreshed imported output updates every unedited shortcut while preserving each individually edited output. The unbound Interact command remains visible in import review and gains no guessed binding.

The keyboard Move instrument is required in this revision; the prior four-button movement cross is rejected. Assign each exported direction to its corresponding joystick part. Retain diagonals, held keys, immediate release on touch end/cancel/disconnect, explicit command-to-part import metadata, and per-part manual-output preservation on refresh. No analog gamepad output survives from the Forever template.

The existing Forever Look instrument emits a gamepad right stick and is incompatible with Era keyboard output as-is. Retain its exact circle frame as a real native circular trackpad labeled `Cursor`, using the already supported relative pointer and tap-to-click behavior. Preserve its gold ring and instrument styling with native trackpad primitives; do not leave a dead joystick or substitute a conspicuous rectangle. This is an added pointer/navigation affordance, not a WoW-exported binding. Include that fact in import/review notes. Do not claim mouse camera control, right-button camera drag, or an imported Look binding, and do not expand this change into a new camera-input feature.

## Artwork, icons, and legend treatment

Reuse the four editable Warband SVG canvas files as derivative source, changing only `WARRIOR` to `HUNTER`, misleading A/B/X/Y or directional input labels to the actual mapped action/key identity, and required framing details where controls genuinely differ. Retain `WARBAND` if desired; it is the requested design’s identity. No Warrior ability art, bank-specific color behavior, LT/RT hold labels, or bank-reference ledger remains. Preserve the current open ground between clusters and do not add a shell, header card, or extra ornament.

All five exported actions require real, matched icon assets, with this snapshot identity preserved:

| Action | Kind / ID | Exported iconFileID |
|---|---|---|
| Attack | spell 6603 | 135641 |
| Raptor Strike | spell 2973 | 132223 |
| Auto Shot | spell 75 | 135493 |
| Refreshing Spring Water | item 159 | 132794 |
| Tough Jerky | item 117 | 133972 |

Resolve artwork through verified game metadata or the actual addon-exported texture identity. Do not guess a filename from an English label when a numeric texture identity is available. Prefer the exact installed Classic texture or a trusted original/CDN mapping with independently verified spell/item and texture IDs. Preserve original downloads, URL/source, decoded dimensions, texture identity, SHA-256, attribution, and conversion operations in provenance. A filename guess, working CDN URL, or bigger image canvas alone is not identity/resolution proof. Do not reuse Warrior spell textures as placeholders in a finished review. If a source is only 56×56, render it honestly with the reference’s original-image sampling and fit; do not sharpen, redraw, or generate replacements. Keep transparent padding/crop and output scale consistent across the five tiles.

Profile-owned icons should travel with the profile to Android/iPhone and remain attached to the stable imported command/slot. Native accessibility names retain the full exported action label. Icon controls may suppress the large overlaid text just as Warband does, but must retain readable key tags or nearby native captions during setup and full accessibility metadata. Gameplay rendering must show the icons prominently, not merely tiny icons alongside the old grid labels. Static icons represent an imported snapshot; no charge counts, cooldown sweeps, proc glows, or persistently active Auto Shot indication are justified.

## Exact material continuity and native states

Copy the reference material values rather than interpreting them as a new palette: dark ground #11171B, face #263239, well #0C1115, ink #F1E8D8, muted ink #B8B5A9, brass #B8975C. Light ground #E8E2D5, face #F5F0E5, well #C9C5BA, ink #282B29, muted ink #55594F, brass #715323. Preserve the reference thin gold sockets, 5–7 pt tile radii, shallow shadows, upper-left lighting, and concentric native Move ring/puck language. Verify actual meaningful legend contrast at 4.5:1 rather than assuming copied colors guarantee it.

Normal retains the reference matte face/thin edge. Pressed uses native inset shadow and 2 pt pressure boundary with no scale or label movement. Active uses the distinct 3 pt boundary/ordinary face; it is a native input/review state, never a claim of live WoW activity. Disabled is flatter and visibly muted while remaining readable. Icons retain their source colors in both appearances; only native framing changes. Review every material in normal, pressed, active, disabled × light/dark × portrait/landscape, matching the existing Warband’s restraint.

## Architecture and review boundary

Derive a new native profile from the reference geometry with keyboard bindings, per-part movement mappings, profile-owned appearance, and actual icon media. This is an import/controller product feature, not a request for another public skin. Prefer native profile-owned assets and presentation; a new canonical catalog entry or distributable `.pocketpad` is unnecessary unless the current rendering architecture truly requires one. Keep appearance, geometry, and executable outputs separate.

Do not falsely stamp a Hunter profile with the canonical `wow-forever-v1` identity merely because centers match. Imported command IDs differ, the control kinds/outputs differ, and the artwork’s semantic assumptions differ. Reuse source/framing with explicit profile-owned anchors or capture a schema-3 contract for the generated Hunter fixture if skin machinery is needed. Applying the original Warrior package without remapping its identity-specific icon rules and labeling is unacceptable. Keep native renderer evidence tied to the exact derived profile and media hashes.

Ordinary re-export refresh continues preserving positions, deleted controls, manual labels, manual button outputs, and now manual direction-part outputs. Applying the new Warband layout to an existing generic grid is a deliberate layout choice, implemented in Mac and CLI with identical behavior. Preserve command-owned metadata so future exports update spell/item art honestly when an action changes, while preserving explicitly user-selected custom imagery. Do not erase the previous layout until the new profile is concrete and reviewable.

Retain derivative editable SVG/JSON/CSS or profile appearance source, anonymized fixture, exact geometry capture/profile, downloaded icon originals and provenance, and all native sheets. Designer, two independent visual-critic passes, and QA remain sequential. If a package is produced, deterministic compilation and strict package/quality validators are required; no publication or human approval is implied by native product testing. Approval stays pending for the exact final sheet and package hash, if any.

## Acceptance criteria

1. Side-by-side normal landscape and portrait renders unmistakably preserve Warband geometry, materials, socket framing, hierarchy, and large icon presentation. No generic text grid, new green theme, four-button Move cross, or Warrior identity remains.
2. All five Hunter spell/item tiles show independently verified artwork matching the exported action and texture IDs, with provenance/hashes retained. No illustrative placeholders or fabricated cooldown/activity state is present.
3. All 16 supported exported command outputs are retained exactly; movement is a four-part keyboard joystick. Keyup/diagonal/manual-part refresh behavior is verified. Interact stays unbound. No gamepad button or axis output remains.
4. Native safe areas, at least 44 pt touch dimensions, and nonoverlapping hit regions are preserved in both orientations. Labels are readable, key tags truthful, and imported action accessibility names complete.
5. Native contact sheets cover both orientations, both appearances, and all four states; icons remain prominent and state differences visible without center/baseline drift. Matched reference frames align within 2 pt at 1×.
6. The exact generated profile’s icon media and appearance survive Mac-to-Android/iPhone synchronization and render on the client. All eighteen reference frames remain present, with the four documented duplicate shortcuts and functional circular Cursor trackpad. Gameplay testing verifies an ability key, Jump, targeting, movement hold/release, Cursor relative movement/tap, and duplicate shortcut outputs. No camera-control claim is made.
7. Refresh preserves edits and deliberate re-layout has Mac/CLI parity. Existing Warband source/package remains unchanged. Review source, evidence, and provenance are retained; applicable strict checks pass and final approval remains pending.

## Main risks

The biggest risk is delivering another layout interpretation when the user requested the actual Warband look: compare directly against the reference before polishing. Texture-file identity mapping and phone asset synchronization are substantive requirements, not decorative finishing. A gamepad template can silently leak incompatible axis/button output into the keyboard profile. Combining four imported commands into a joystick and mapping four commands to duplicate shortcuts changes refresh ownership and needs explicit part/shortcut metadata. A copied Warrior package can render convincing but false spell art even when geometry appears right. The Cursor instrument must retain the reference circular presentation while performing actual supported relative pointer/tap behavior; it does not imply camera control.
