# ConsolePort phone final QA — version 1

Verdict: **qa-pass for the landscape owned-profile contract**, with runtime acceptance pending. **Legacy directory `skin quality --strict` fails**, exit 1, score 96, exactly one warning `source-incomplete-orientation-matrix`. This is not a zero-warning strict legacy-package pass. Portrait is N/A under the current owned product contract in `SKILL.md`, `docs/owned-design-workspace.md`, and the approved art-direction scope; no portrait runtime was restored to satisfy the retired directory matrix.

Both independent visual critiques passed. No source, live authority, application, signing, or publication changes were made by QA.

## Exact candidate

Workspace `build/ConsolePortLab/consoleport-design-final`, session `735E2ECF-50A1-4B77-9EDC-33E9EDACDA3F`, revision 3, review 2, target configuration revision 7. Target Bonteq profile `A4E1DDD3-9767-504E-AC27-099919F511CB`.

- Private compiler artifact SHA-256: `b34d337bc66da6e1336d7b5e1c0c38472db3146dcd49603330b128bebb024259`.
- Exact native contact sheet SHA-256: `901b056c59d4a81502a7f2e30ff3bba413177f2cd35c4ab91aa2cdf64ede0b8d`.
- Source SHA-256: `4e66ef581b7a677840f657c5ee145a4f739c988e24113af15c0d6484010766e2`.
- Binding contract SHA-256: `79ad44961b50a80a874f39100e30881b85c037fbaadfe90a46314d3d6afccd73`.

## Executed checks

CLI `build/ConsolePortDesignTools/thumble`; independent temporary output `/private/tmp/consoleport-qa-y90vk7m_` (abbreviated Q), workspace abbreviated W. Exact stdout/stderr and machine-readable findings are preserved in `qa-evidence/`.

1. `design inspect --workspace W`: exit 0; source synchronized, expected session/revision/target, reviewed sources immutable. Verified all 48 source, package, sheet and frame hashes against review manifest.
2. `skin compile W --build-directory Q/build-a -o Q/a.pocketpad --clean --json`, repeated independently for b: both exit 0. Both outputs byte-identical to each other and the exact reviewed artifact above.
3. `skin validate Q/a.pocketpad --strict --json`, repeated b: both exit 0, no issues.
4. `skin quality W --strict --json`: exit 1, score 96, only the explicit portrait matrix warning described above. No other warning or error was waived.
5. `skin unpack Q/a.pocketpad -o Q/unpacked` then `skin pack Q/unpacked -o Q/repacked.pocketpad`: both exit 0; repacked bytes identical. Every declared asset size and SHA-256 matches.
6. Reviewed immutable native generation evidence: 40 landscape 874×402 frames = two schemes × four action banks × normal/pressed/active/disabled/mixed. All hashes match. No hit-frame rectangle overlap among interactive controls in any frame. All rendered safe insets are explicitly zero: hardware-inset coverage is pending.
7. Frozen original/reviewed contract comparison: 144 differences, every difference restricted to centerX/centerY/widthScale/heightScale. All non-customization fields and all eight appearance-bank declarations remain exact. This preserves saved outputs, bindings, joystick/axis settings, labels and haptic declarations through the layout plan; apply must still enforce the authority's all-eight-state nonvisual/haptic guards. QA did not apply or infer physical haptics.
8. Package safety: only manifest.json, skin.json and five PNGs. No SVG/CSS authoring source, executable payload, complete profile, or bindings shipped. Captured compatibility geometry is permitted appearance metadata. Three game icons are 32×32; two original passive recess textures are 256×256. Package 82,350 bytes. Source SVG/CSS retained outside artifact. This intermediate contains local game imagery and is not approved for redistribution.
9. Nominal rendered foreground/solid-face contrast minimum 8.09:1. This does not measure icon-image contrast, compositor effects, Android accessibility adaptations or actual screen brightness. Independent critics found readable legends/states and only minor square-icon/rim craft limitations.

## Remaining acceptance and approval

Physical Pixel landscape insets, native Android parity, touch reach, movement/camera/jump and modifier input, reconnect, populated LT/RT/both icon identity, and Bonteq → Thumbleapp → Bonteq design persistence remain for parent live QA. Captured modified banks are empty; no placeholders or invented mappings were added. Unrelated/default profile preservation must be checked on local application. Web/public catalog preview readiness is N/A: this is a private owned-profile design, with no catalog or publication requested.

Human approval remains pending for the exact contact sheet and artifact hash. QA passes the bounded local owned-design candidate; it does not approve publication or represent human approval. A user-authorized reversible local apply is separate from that publication gate.

## Follow-up runtime defect — candidate supersession required

Read-only follow-up found that this exact artifact contains 24 explicit state `content.icon` declarations. Swift `GamepadDesignSystem.swift` resolves layout.icon and then content.icon last; Android `ControlPresentation.kt` uses the same precedence. `WoWLiveProfile.swift` refreshes layout.icon/mirrors only. Therefore future imported identity changes can be masked by the reviewed CSS state icon. The static checks above remain accurate, but this artifact must not be treated as passing ongoing icon-refresh acceptance. Parent is replacing fixed identity declarations with presentation-only inherited-icon scale/placement and will require a refreshed deterministic artifact and actual resolved-paint regression. No source or live changes were made by QA.

## Final candidate addendum — version 2, inherited icon presentation

Final verdict: **qa-pass for the landscape owned-profile contract**. Supersedes the stale-identity artifact above. New workspace `build/ConsolePortLab/consoleport-design-inherited`, session revision 2, review 1, target revision 7. Both critics independently bound their visual passes to this exact final artifact.

- Artifact SHA-256 `1810f3fa3239a9a276783356a9701a28edee40f87b3e3379778aef0a3277c0db`.
- Evidence identifier `7d649f60bb52ef7a8061dd8f9e392d2711f124677f04156751564463cedcb49c` (parent/design evidence; QA independently checks its file manifest).
- Exact sheet remains `901b056c59d4a81502a7f2e30ff3bba413177f2cd35c4ab91aa2cdf64ede0b8d`.

Repeated the original QA commands on this workspace in independent `/private/tmp/consoleport-inherited-qa-2im352wh` build directories. Both compiles exit 0 and are byte-identical to each other and the exact reviewed final artifact. Strict validators both exit 0 with no issues. Unpack and repack exit 0 and preserve exact bytes. Asset sizes/hashes pass. Source inspect reports synchronized. All 45 declared source/package/sheet/frame hashes match. All 40 named native frame hashes match the previous candidate byte-for-byte; the contact sheet, binding digest and layout-plan digest are unchanged. Prior geometry/contrast measurements therefore remain applicable.

Strict directory quality still exits 1, score 96, with exactly `source-incomplete-orientation-matrix`; this remains an honest legacy strict failure and portrait remains N/A under the landscape owned contract. No additional warning was waived.

Unpacked final archive contains only manifest.json, skin.json and two original recess PNGs. It contains **zero game PNG assets and zero content.icon identity declarations**. There are 144 content objects with independent iconPlacement/iconScale (288 individual fields), inheriting live image identity. Native `GamepadDesignSystem.swift` applies these presentation properties only to an existing resolved icon; Android `ControlPresentation.kt` does the same. Thus presentation does not invent an image after a managed removal. Parent reports focused Swift resolved-paint identity/removal fixtures, Android tests and actual native importer fixtures pass; those runtime fixture executions were performed by parent, not rerun by this read-only QA. Physical Pixel sync/input and populated modifier-bank acceptance remain separate.

Exact final logs and machine findings: `qa-evidence-inherited/`. No source/live authority changes, publication or human approval were performed by QA. Pending approval record is updated to this artifact.
