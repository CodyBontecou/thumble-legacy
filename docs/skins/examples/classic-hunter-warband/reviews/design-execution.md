# Designer execution — Classic Hunter Warband

This local derivative follows the user's explicit request to reuse the project's Warband design with their Hunter bindings and real Hunter icons. It preserves the supplied native Warband reference centers, control scales, face shapes, radii, palette and framing in both orientations. The original `warband-command` workspace/package is untouched.

## Binding ownership

`preview/profile.json` is a Hunter profile derived from the supplied reference. Its12primary buttons retain deterministic command UUIDs from the user's actual exported snapshot. The Move joystick has a deterministic `THUMBLE_MOVEMENT` UUID and four keyboard parts W/S/Q/E. All native button/default outputs are exact first supported exported keyboard bindings; no gamepad buttons or analog axes remain. The command map records explicit direction-part ownership. Actual character identity, realm, and snapshot metadata are not persisted here.

The4shoulder capsules carry Map,Character,Bags,Self; the8diamond positions carry Water,Food,Target,Previous,Jump,Attack,Raptor Strike,Auto Shot. All18reference native frames remain: View/Menu become duplicateMap/Character shortcuts; L3/R3 become duplicateJump/Self shortcuts, each with a distinct stable shortcut UUID and actual key caption. Look becomes a real relative-pointer/tap-to-click circular native Cursor trackpad. It is an added navigation affordance, not an exported binding or camera/right-button drag feature. Interact remains unbound. No extra game actions, modifier banks, or live cooldown/activity state are invented.

This is a custom native profile for the current snapshot, not a generic import default. Existing importer refresh cannot be represented as automatically preserving/updating this new command-to-direction grouping and custom imagery without dedicated integration; do not promise that behavior. Parent preserves existing private Link metadata separately during local authority installation.

## Exact artwork and package boundary

The five64×64PNGs were decoded from installed Classic Era CASC textures by their exported iconFileIDs. Original BLP bytes, conversion operations, dimensions and hashes are retained under `sources/originals/` and `sources/provenance.json`. No filename guess, generated image, Warrior stand-in, or CDN substitution is used.

The four editable WarbandSVGs are derivative sources: WARRIOR becomesHUNTER, obsolete bank ledger is removed, and input tags become the actual keys−,=,Tab,⇧Tab,Space,1,2,3. Passive gold ring/puck art recreates the reference concentric Cursor instrument behind a transparent native trackpad; native text, hit testing, state boundary, relative motion and taps remain real. CSS retains the Warband material language, remaps all five actual texture assets by command action semantics, removes fictitious bank rules, and keeps the native Move ring and puck. Target/Previous font is11pt to fit the small native diamonds; shoulders retain13pt; actual final rendered caption legibility remains subject to independent critique.

Portable profile import rejects embedded image payloads. A local schema3package therefore owns the icon/frame media. The helper captures exact generated Hunter geometry without claiming canonical `wow-forever-v1` compatibility, recalculates the passive full-canvas SVG anchor from the surviving native command bounds, compiles a `.pocketpad`, and emits a dehydrated `preview/install-profile.json` with package asset references. This is local installation evidence, not a public catalog submission.

## Evidence and state constraints

Native rendering must cover portrait/landscape × light/dark × normal/pressed/active/disabled. Actual outputs remain immutable across these appearances; icon sources retain original colors. Active is a synthetic native review state only. Disabled opacity is0.85forordinaryfaces/0.65foricons to preserve detail while muting the face; actual meaningful text/tag contrast must be inspected.

Captured native frames must compare within2pt to the supplied reference for every retained slot. No runtime geometry is supplied by the SVG. Parent installation, synchronization, touch/keyup/diagonal checks and actual WoW keyboard delivery remain distinct functional QA. No human approval, publication, staging, commit or push is performed.


## Eighteen-frame geometry amendment

The parent/director amended the brief before this revision to retain every reference position. Slot8/9 were builtin View/Menu controls with special base-size multipliers; changing them to command-owned custom UUIDs requires compensating widthScale×1.48/1.66 and heightScale×0.72 to retain exact visual dimensions. Central capsule hit-only insets expand their sub44pt heights without moving visuals. Portrait other buttons use1.2pt hit insets to bring41.808pt visual faces above44pt; landscape44.72pt faces need no expansion.

Cursor changes kind from joystick to trackpad. Its scale compensates the differing native base sizes: landscape width0.45,height0.7714285714; portrait width0.4114285714,height0.72. Expected final visual sizes are86.832pt and69.4656pt, matching the reference Look instrument. No gamepad axis or old joystick mapping remains on Cursor. Its gold rings/puck are explicitly passive SVG framing, not a fake interactive joystick.

Duplicate ownership is recorded in command-map.json with `isDuplicateShortcut` and distinct IDs. Generic importer automatic refresh of these shortcuts is not implemented by this workspace and is not claimed. Parent installation/QA must compare current exported outputs independently.


## Parent-synthesized corrections — native reviews3/4

Accepted critique1 corrections retain native geometry/outputs: Tab andShift-Tab have independent glyph boxes below the Target/Prev tiles; all inherited profile background fill/color fields are removed so CSS supplies the actual light#E8E2D5/dark#11171B canvas. Primary-action Jump alone uses11pt/2pt padding/one line, while the duplicate utility Jump retains13pt. Cursor uses two theme-specific opaque original ring/puck SVG face assets so strict contrast analysis samples its actual central face. It is not a white-legend workaround and does not alter pointer behavior.

Review4 sets all three source canvas colorSchemePreference values tosystem for offscreen light/dark evidence. Parent may set the installed user profile toDark afterQA as a display preference; the immutable review fixture stays system so light evidence remains independently renderable.

`native-review-4/pixel-evidence.json` records all16frame Cursor contrast and target-tag glyph bounds. Actual ground pixels are exactlyRGB232/226/213light and17/23/27dark. Cursor glyph contrast minimum is5.005:1light and6.398:1dark. Target/Prev key-tag complete glyph/socket clearance is at least5.64pt, above4pt. `instrument-glyph-contrast.json` separately records all48Move/Cursor/drawer glyph cases; every ratio is≥4.5:1. Portrait/light normal and landscape/light normal were visually inspected: Jump,Move,Cursor,anddrawerchevronare complete/visible.

Native review4 strict source compilation passed and exact captured-profile quality evaluator reports zero issues; CLI strict package validation passes. Existing CLI workspace-quality additionally requires a design-session authority record and cannot run against this standalone captured workspace; package-only strict quality warns unverified-artboard because it lacks the supplied captured-profile context. No authority session is fabricated and no quality warning is suppressed. The direct shared evaluator was given this exact captured profile/artboard and returned zero issues, including native geometry/safe-area checks.


## Parent-synthesized secondary-hint correction — native review6

Candidate4 package/source/profile/CSS are archived under `native-review-4/source-snapshot`. Failed candidate5 is preserved with its native frames and20strict contrast issues; it strengthened all Light pressed/disabled foregrounds, unintentionally changing blank icon-legend declarations against fixed nearblack icon faces. The parent approved the exact correction: retain Light pressed#171B18 anddisabled#101410 for native hint-bearing controls, restore original#F1E8D8 only for the abilities group in those two states. NoappSwift,geometry,font,baseline,media,Darkappearanceoroutputchangeswere made.

`native-review-6/native-hint-contrast.json` now measures64actual secondary keyboard-hint cases: Mcentralshortcut,Shift-Bshoulder,F1shoulder,Spacebottomshortcut × bothorientations × bothschemes × allfourstates. Fully covered repeated glyph interiors reach4.753:1minimum (Lightpressed5.174,Lightdisabled4.753,normal/active6.115;Dark≥6.315). No contrast claim relies on main-caption colors alone. Reproducible source is `sources/measure-native-hints.py`.

`instrument-glyph-contrast.json` rechecksall48Move/Cursor/drawercases;minimum6.398:1. Target-tag glyph/socketclearance remains≥5.64pt. Exact native captured-profile quality returnszeroissues and strictpackagevalidationpasses. Allnative-layoutrecords are identical toreview4 and all8DarkPNGs are byte-identical; only bounded Light foregrounds changed. Final source/package snapshot is preserved under `native-review-6/source-snapshot`. Independent critic recheck andQAremainrequired; noapprovalclaimed.
