# Hunter Warband camera and click revision — art direction

This is a bounded addition to the visually accepted Hunter Warband controller: retain its five actual spell/item icons, sixteen exported command bindings, eighteen existing native controls, brass/charcoal material system, and both orientation compositions, while adding a right-thumb camera joystick and explicit left/right mouse buttons. The target is confident simultaneous movement and looking, with independent cursor navigation.

The user authorized local implementation and installation. This is not publication approval for a changed package. Preserve the v1 brief and evidence. Do not edit the original `warband-command` source. Reuse the project’s requested Warband design, not another console’s trade dress, logos, silhouette, or game HUD. No replacement icons, new class palette, new decorative theme, fake cooldowns, or invented WoW actions.

## Geometry contract

Base measurements come from `reviews/native-review-6/native-layouts.json`, not screenshots. Reference canvases remain 874×402 landscape and 402×874 portrait. Keep every existing button frame, action-diamond socket, shoulder capsule, utility capsule, Move frame, and system-toggle frame unchanged. Move only the existing Cursor instrument, add Camera at the specified frame, and add two click controls. The result is **21 native controls plus the existing system toggle**: sixteen keyboard buttons, Move, Cursor, Camera, Left click, Right click.

The table gives exact desired native center points and resolved visual sizes; use actual renderer geometry when translating to layout scales. Retain current 0.72 native instrument scale so the circular frames resolve to approximately 86.8 pt landscape and 69.5 pt portrait. Coordinates are absolute within the existing full reference canvas, not a second safe-area-relative normalization.

| Control | Landscape center / visual size | Portrait center / visual size |
|---|---|---|
| Move — unchanged | (120, 225), 86.8×86.8 | (95, 570), 69.5×69.5 |
| Cursor — relocated | **(440, 280), 86.8×86.8** | **(201, 510), 69.5×69.5** |
| Camera — added | **(580, 230), 86.8×86.8** | **(305, 570), 69.5×69.5** |
| Left click — added | **(385, 355), 52×44** | **(170, 610), 44×44** |
| Right click — added | **(495, 355), 52×44** | **(230, 610), 44×44** |

Use centerX=x/canvasWidth and centerY=y/canvasHeight when normalized coordinates are required. Sizes above are measured native dimensions, not arbitrary `widthScale` values; derive the scale using the relevant native primitive base size and rendered orientation control scale. Click tiles are native trackpad primitives with capsule appearance, so do not reuse button-scale arithmetic without measuring their resolved frame. Click hit regions must never shrink below 44×44 pt. Cap their derived hit expansion at the specified frames where necessary to preserve surrounding gaps. Read-only native measurement must confirm the final geometry; do not infer compliance from model widthScale alone.

**Landscape:** the existing action diamonds remain at (285,275) and (730,255). Camera occupies the open right-center field between the right utility tier and the combat diamond, giving the right thumb its dedicated instrument. Cursor shifts only 50 pt left into the lower central field. Its paired click faces sit immediately beneath and to either side, keeping cursor navigation visually separate from camera control. Cursor bottom is approximately y323.4; click tops are y333, leaving 9.6 pt native clearance. Camera’s right edge is approximately x623.4; Attack’s left edge is approximately x655.6, leaving 32.2 pt. Preserve the lower Jump/Self shortcuts at their existing centers; neither new click intersects them.

**Portrait:** Camera takes the former right-hand Cursor center (305,570), matching Move across the canvas. Cursor relocates to the middle lane above/between these thumb instruments at (201,510). Click faces sit in that central lane below Cursor, at y610; their visual/hit bounds remain x148–192 and x208–252, y588–632. They do not intrude on Move or Camera: each instrument’s current hit region ends before those x-bounds, with additional vertical clearance where needed. The eight action tiles remain at y698/750/802, so click bottoms retain at least 43 pt before the first tile’s hit bounds. The center lane replaces a small part of existing negative space, without moving the action diamonds or top utility tiers.

Preserve existing safe areas, native system toggle, and original control-bar semantics. No new touch target or artwork may enter the device exclusion region. Decorations must remain inside their own allocated visual envelope with at least 8 pt clearance to neighboring control artwork. If a ring exceeds the native visual frame, reduce only the passive ring radius enough to meet that margin; do not enlarge its hit region or move an approved existing button. Any larger conflict discovered by native measurement goes back as a specific designer correction rather than silently shifting the whole controller.

## Native behavior and ownership

`Move` remains the existing four-part keyboard joystick with exact exported W/S/Q/E direction ownership and hold/release behavior. All sixteen imported commands and four intentional duplicate keyboard shortcuts keep their current outputs and refresh semantics.

`Camera` is a **new joystick with analog target `mouse_look`**, not a gamepad axis. Displacement controls continuous relative mouse movement while the right mouse button is held for the camera gesture. Neutral means no pointer movement. The center puck and annular boundary preserve the familiar Warband instrument vocabulary. Use native spring return on release; never use artwork to simulate the interactive puck. Stop motion and release the camera’s right-button ownership on touch end, cancellation, profile change, disconnect, or receiver shutdown.

`Cursor` retains native relative-pointer/tap behavior and its identity. It is not a second camera joystick. Do not add automatic camera-button holds to Cursor.

`Left click` and `Right click` are native trackpad primitives configured with `clickButton=left/right`, `movesCursor=false`, `tapToClick=true`, and `twoFingerScroll=false`, styled as utility capsules. They emit discrete tap-clicks through the existing native pointer route, not macros or held buttons. Use truthful native legends `L click` and `R click` with full accessibility names `Left mouse click` and `Right mouse click`; allow two-line legends within portrait 44 pt faces. A long press must not silently imply a held-button feature. Cursor retains its existing native one-finger-left/two-finger-right clicking and movement. Ensure a discrete Right click event during Camera use cannot accidentally end Camera’s separately owned right-button hold or leave it stuck. Parent owns models, receiver behavior, CLI/MCP operations, and Android/iPhone input delivery.

Camera, Cursor, and explicit mouse clicks are local controller affordances; import review/metadata must distinguish them from the addon’s sixteen resolved keyboard commands. Do not overwrite them when a future export lacks corresponding commands. Preserve editable local placement and per-control settings on refresh. Configuration, review/rendering, install, receiver lifecycle, and pairing must remain usable through the CLI/MCP/headless product boundary specified in current AGENTS.md; no desktop UI dependency is introduced.

## Appearance continuity

Keep current dark ground #11171B, face #263239, well #0C1115, ink #F1E8D8, brass #B8975C; light ground #E8E2D5, face #F5F0E5, well #C9C5BA, ink #282B29, brass #715323. Meaningful legends retain 4.5:1 contrast. Existing five game images remain byte-identical, prominently framed, and attributed. No new raster imagery is required for Camera/click controls.

Camera should match Move’s circular material and native puck treatment, with the native legend `Camera` instead of `Move`. Cursor retains its recognizably flat pointer surface and native `Cursor` legend, now at its relocated center. Click buttons use the existing utility-capsule depth and gold boundary, not icon sockets or oversized shoulder labels. Preserve narrow upper-left highlights, shallow inset/drop shadows, 1–1.5 pt boundaries, and consistent rounded corners. Any supplementary mouse symbol is a native platform symbol and subordinate to the readable click legend. Do not add connection instructions, explanatory paragraphs, or implementation jargon to the controller surface.

Normal is the current matte raised baseline. Pressed/held uses the established inset face and 2 pt pressure edge. Camera activity may show the native displaced puck plus the existing engaged ring while its physical gesture is held; this represents local input, not a WoW cooldown or persistent game state. Active remains distinct from pressed through the existing 3 pt boundary/ordinary face. Disabled is flat/muted with readable labels. State styling must preserve geometry and legend baselines in both appearances. Do not recolor the five game icons.

## Source and evidence

Retain all editable SVG/JSON/CSS/profile source. Update the original orientation-specific SVG framing only where the Cursor/Camera/click frames require it; preserve existing icon socket coordinates and branding. Passive SVG remains appearance only. Capture the revised geometry contract with the actual profile IDs and semantic roles; v1 canonical/captured geometry must not be falsely reused unchanged. Materials may be reused, while new aligned artwork targets the revised captured frames.

Create versioned camera review evidence rather than replacing v1 contact sheets: all 16 orientation × scheme × state native panels, native-layout JSON including hit bounds, and exact source/media hashes. Preserve at least two independent visual-critic passes, a final visual-pass, and independent QA. If the pipeline produces a package, rebuild deterministically twice and run strict validators. Approval remains pending for the new exact sheet/package hash; local installation authorization does not authorize publication.

## Acceptance criteria

1. All 18 original control identities remain, with only Cursor moved as specified; Camera and two click controls bring the total to 21 plus the system toggle. All five icon bytes and sixteen exported command outputs remain unchanged.
2. Native measured centers/sizes match this table within 2 pt at 1× in both orientations. Every touch target is at least 44×44 pt, hit rectangles never overlap, passive decoration has at least 8 pt clearance, and all safe areas remain intact.
3. At thumbnail size, Camera and Move form a balanced thumb pair; Cursor and clicks form a separate central navigation group. Warband hierarchy, material colors, action diamonds, branding, and utility tiers remain recognizable without a whole-controller redesign.
4. Normal/pressed/active/disabled are distinguishable in all 16 native review panels. Labels meet 4.5:1 contrast with no ellipsis, clipping, collision, duplicate hint overlay, or baseline/center movement across states.
5. Native input verification demonstrates continuous displacement-based camera motion during right-button ownership, zero movement at neutral, reliable release after cancellation/disconnection, and safe discrete Right click events during Camera use. Cursor movement/one-finger-left/two-finger-right tap remains functional; each explicit click tile emits its configured discrete button with no cursor movement, scrolling, or unintended hold.
6. CLI/MCP inspection and phone synchronization preserve the revised geometry, mouse targets/buttons, appearance assets, imported keys, and existing manual-edit refresh rules without the desktop GUI running. Android and iPhone renderer/input paths receive appropriate verification for the changed primitives.
7. Versioned editable source, exact capture, all-state contact sheet, two visual critiques, QA, and applicable strict/deterministic results are retained. Human publication approval remains pending for the changed sheet/hash.

The principal risks are crowded center-lane click targets in portrait, a new camera ring accidentally colliding with the combat diamond or replacing the existing Cursor behavior, and discrete Right click events interfering with Camera’s physical mouse-button hold. Native measurements and held-input cancellation tests decide completion; appearance approval alone cannot prove input safety.
