# Thumble Link v1

See [installation and usage](../Addons/ThumbleLink/README.md). This is a keyboard
snapshot integration, separate from the `wowForever` native-controller template.
The addon is in `Addons/ThumbleLink`; no addon-to-host network service is required.

## Wire contract

The clipboard format is `THUMBLELINK1:` followed by UTF-8 JSON. The importer also
accepts bare JSON. The complete input is bounded to 256 KiB, with at most 128
actions, four key alternatives per action, and 32 warnings. Duplicate JSON keys,
duplicate commands, invalid strings/identities, unsupported schemas, and unsupported
client interfaces are rejected before a profile mutation. Version 1 accepts the explicit client/interface pairs `forever` / 16001 and
`classic-era` / 11509 (Classic Era 1.15.9). Other pairs are rejected; a new
client needs a tested adapter/contract update.

```json
{
  "schema": "thumble.wow-link",
  "version": 1,
  "client": { "flavor": "forever", "version": "1.60.1", "build": "99999", "interface": 16001 },
  "character": { "id": "Player-1234-00000001", "name": "Example", "realm": "Test Realm", "classTag": "WARRIOR" },
  "exportedAt": 1792310400,
  "actions": [
    { "command": "ACTIONBUTTON1", "label": "Heroic Strike", "keys": ["1"], "slot": 1, "kind": "spell", "actionID": 78, "iconFileID": 132282 }
  ],
  "warnings": []
}
```

`exportedAt` is Unix seconds. `slot`, `actionID`, `iconFileID`, and
`unavailableReason` are optional. No macro text or executable Lua is accepted.
Key alternatives retain WoW order; the importer uses the first supported keyboard
chord. `SHIFT`, `CTRL`, `ALT`, and `META` map to the existing shared Mac modifier
bits. WoW `DELETE` maps to forward delete, distinct from `BACKSPACE`.

## Export behavior

The addon reads each selected Blizzard button's resolved `action` slot and binding
command, and verifies each candidate key against its effective binding with
override bindings included. Missing metadata falls back to a bar/button label.
Export is blocked in combat and required API/build failures are explicit. Restricted
values are not inspected. Export never persists or changes protected game state.

The binding/slot pattern follows Blizzard's
[action button source](https://github.com/Gethe/wow-ui-source/blob/live/Interface/AddOns/Blizzard_ActionBar/Shared/ActionButton.lua).
Classic Era compatibility was checked against the 1.15.9 source tag's
[action-button implementation](https://github.com/Gethe/wow-ui-source/blob/1.15.9/Interface/AddOns/Blizzard_ActionBar/Shared/ActionButton.lua)
and [Classic button template](https://github.com/Gethe/wow-ui-source/blob/1.15.9/Interface/AddOns/Blizzard_ActionBar/Classic/ActionButtonTemplate.xml).
Both clients use resolved button slots and binding commands. Classic Era can use
the existing legacy spell/item/texture fallbacks. The addon TOC declares both
interface versions. Source inspection does not establish live client behavior.
Native controller bindings are a separate interface; see Blizzard's
[Forever beta notes](https://us.forums.blizzard.com/en/wow/t/wow-forever-beta-development-notes-%E2%80%93-updated-october-15/2360696).

## Controller layout

Mac and CLI share `ThumbleLinkImport.Layout`. New setups default to `controller`
for up to 24 supported commands: four held directional buttons at the left,
combat/Jump at the right, menu/target strips, central consumables, and bounded
additional rows. Each input still owns its original command UUID and keyboard
output; geometry never derives a new output. Full imported legends use native
presentation metadata so the legacy 12-character editable label does not truncate
ability names. Movement/menu captions are shortened with full accessibility names.

The layout uses scheme-aware native fills and state styles, without a package,
canonical gamepad-template claim, or class artwork. Adjacent controls use explicit
hit insets to prevent invisible overlap. More than 24 bound commands default to
grid, with a warning; explicitly requesting Controller rejects that overflow.
Ordinary refresh preserves geometry. A selected layout in the Mac import sheet or
`thumble wow import --layout controller|grid` deliberately rebuilds positions and
appearance while preserving edits to labels/outputs and deletion history. New
controls occupy free controller positions; a refresh fails without mutation if a
user's moved controls leave no space. Native evidence and independent review live
in `docs/skins/examples/classic-hunter-link/reviews`.

## Import and refresh

`ThumbleLinkSnapshot` is a bounded shared Codable value. `ThumbleLinkImport`
prepares the same ordinary configuration envelope for the Mac sheet and Swift CLI.
The normal profile authority owns persistence and phone synchronization. The CLI
uses the exported configuration revision for its atomic import; a concurrent change
fails instead of overwriting new edits. The Mac sheet rejects changed configurations
between review and import and registers the normal editor undo snapshot.

Profile identity is a deterministic, namespaced SHA-256-derived UUID based on game
flavor and character GUID. Button identity additionally includes the binding command,
so spell/rank replacements do not rearrange the phone. Matching an unrelated name or
an ID with missing integration metadata fails without replacing that setup.

The raw profile's `thumbleLink` field retains the last snapshot, import timestamp,
and known command labels through the existing forward-compatible metadata path.
No stored fields are added to the large shared profile/customization aggregates.
Each element's `defaultOutput` records its last imported binding baseline:

- If its current output equals that baseline, refresh updates it.
- If the user changed the output or label, refresh preserves the edit and reports it.
- Missing/unsupported bindings clear unchanged imported outputs.
- Previously deleted controls stay deleted; newly discovered bound commands get
  new controls. This applies independently to existing orientation layouts.

Refresh is scoped to one character setup, preserves unrelated controls and setup
metadata, and leaves the existing default setup selected as default. Mac imports
select the imported setup; CLI `--no-select` preserves selection.

## Verification

```sh
python3 scripts/verify-thumble-link.py --lua /path/to/lua
python3 scripts/verify-thumble-link-cli.py /path/to/packaged/bin
xcodebuild test -project Thumble.xcodeproj -scheme ThumbleTests \
  -destination 'platform=macOS,arch=arm64' \
  -only-testing:ThumbleCLITests/ThumbleLinkTests CODE_SIGNING_ALLOWED=NO
./scripts/verify-stack-safety.sh
```

The Lua harness runs the real exporter and slash-command window with mocked APIs;
its output must match the fixture decoded by Swift. Swift tests cover validation,
keyboard conversion, profile construction, alternate binding selection, refresh,
manual edits, deletion, clearing stale outputs, name conflicts, and concurrent edits.
Stack regression coverage imports and refreshes a maximum-size 128-action snapshot
on a 512 KiB thread, with separate inline-size budgets.

Live acceptance remains: install in each supported Forever / Classic Era build, export main/extra
bars and utility actions, review on Mac, pair an iPhone, and verify actual keys,
modifiers, holds/releases, orientation changes, rebinding refreshes, and page/form
label limitations. No live WoW recognition or gameplay result is implied by mocks.


## ConsolePort onboarding

Use this branch for native gamepad input with ConsolePort. The older `wow import`
keyboard snapshot workflow above remains separate. The supported adapter is pinned
to [ConsolePort 3.3.11, file 9254142](https://www.curseforge.com/wow/addons/console-port/files/9254142),
whose release lists Classic Era 1.15.9 and Forever 1.60.1. Obtain the publisher's
archive and extract it to a temporary directory containing the nine `ConsolePort*`
addon folders. The installer embeds Thumble Link itself, so a released host does
not need a repository checkout or Thumble desktop application to install it.

1. Locate the actual `_classic_era_` or `_forever_` client root. Run a read-only
   plan with the extracted release directory:

   ```sh
   thumble wow setup --client-root '/Applications/World of Warcraft/_classic_era_' \
     --consoleport-source /path/to/extracted/ConsolePort --json
   ```

   MCP exposes the same operation as `setup_wow_consoleport` with `clientRoot`,
   `consolePortSource`, and `apply: false`. The plan records folder hashes,
   conflicts, selected settings-backup count, running-client state and
   `planSHA256`. No files are written by planning. A source interface/version
   mismatch must be resolved before installation. Differing existing ConsolePort
   folders require explicit `replaceAddons`/`--replace-addons`; inspect their
   version and changes before selecting that replacement within the user's scope.

2. With the client closed, apply the exact plan:

   ```sh
   thumble wow setup --client-root '/Applications/World of Warcraft/_classic_era_' \
     --consoleport-source /path/to/extracted/ConsolePort \
     --apply --expected-plan-sha256 HASH --json
   ```

   In MCP use `apply: true` and `expectedPlanSHA256`; applying requires MCP's
   configuration-write capability. Keep the other request fields identical to the
   plan. Replan after changed files, option changes or a running-client failure.
   Completion requires `applied: true` and either an unchanged result or a
   committed receipt in the returned backup directory.

   Replaced addon contents and relevant `bindings-cache.wtf`, ConsolePort and
   Thumble Link SavedVariables files are copied into a private
   `Interface/ThumbleBackups/<id>` directory before replacement. Installation
   leaves WTF files unchanged. Ordinary failures roll back already replaced
   folders; an external edit during rollback is preserved and reported for
   recovery using the receipt and backup. Those files also remain available for
   agent-assisted undo. The current installer does not automatically recover an
   interrupted process on a later invocation.

3. Restart the client and enable the addons. The generated declarative setup file
   opts into first-time live-panel initialization and the ConsolePort adapter.
   `--no-live`/`enableLive: false` omits that initialization; an existing in-game
   live-panel preference is retained. The adapter waits for a logged-in character,
   supported ConsolePort API, loaded bindings, and WoW's actual enumeration of
   Thumble compatibility VID `0x045e`, PID `0x028e`. It defers binding writes during combat.
   Inspect `/thumblesetup`: `configured` means that a character-local preset was
   saved; `keeping-existing-controller-bindings` means existing bindings were
   deliberately retained. Neither result proves physical input recognition.
   `waiting-for-thumble-gamepad` requires resolving HID recognition before retrying.

   Existing gamepad bindings are preserved by default. The explicit
   `replaceBindings`/`--replace-bindings` option, or `/thumblesetup apply` after
   enabling onboarding, requests the Xbox trigger preset. The adapter captures
   previous pad bindings, keyboard bindings and CVars, checks keyboard retention,
   saves only the character set, and restores after an apply failure.
   `/thumblesetup restore` uses its character backup. Tutorial-completion flags
   are left for ConsolePort's real setup/test flow.

4. Use the headless receiver with configuration writes enabled. Pair the phone
   through the normal receiver pairing workflow. Inspect `thumble wow sync status`
   or MCP `wow_sync` with `action: status`; explicitly request the native capture
   helper's Screen Recording permission with `request-permission` when needed.
   macOS approval is a human step. `/thumblelive on` enables the visible optical
   panel; `off` disables it. Then start the observer with `thumble wow sync start`.

   **Current activation gate:** live profiles contain profile-owned appearance
   banks. The legacy format-4 authority and retained peers must be migrated and
   connected to format-5 transfer before this can work end to end on the phone.
   The observer now commits through owned drafts and the normal revision-checked
   save path on an already-migrated authority.
   Current builds reject start on a legacy authority. Preserve that authority and
   pending phone edits when this gate is reported; use the backed-up
   [authority migration and recovery workflow](authority-migration.md) after
   checking the authenticated phone's format offer. Installation is complete but
   live controller onboarding remains incomplete. Never edit the state JSON to
   bypass a format marker or use a second catalog as a fallback.

   Android now has an explicit canonical format-5 cache reader and an atomic,
   Mac-scoped pending-edit journal. Scalar requests retain their original UUIDs
   and bytes; only a matching successful receiver receipt can retire an entry.
   Recovery profiles stay local and share one copy per snapshot. Legacy cache
   saves and expired-credential cleanup preserve the journal. The gated Android
   transport now uploads scalar intents, retries only after an explicit conflict
   and advancing authority snapshot, and preserves edits across A/B/A selections.
   The Pixel client has exercised geometry, appearance and A/B/A selection against
   an isolated real headless receiver plus native helper with input disabled.
   The retained iOS path now has equivalent atomic source/journal caching, editor
   intents and receipt/retry integration. Its actual hosted client has also
   committed edits and confirmed A/B/A selection against the isolated receiver.
   Normal format-5 offers now require successfully loaded, empty legacy
   journals. The actual Pixel client has also drained a legacy layout edit,
   survived an exact backed-up receiver migration and reconnected with the same
   trust before committing an owned edit. Disconnected peers, edits during
   cutover and incomplete recovery still require reconciliation. These tests do not establish live WoW capture,
   game input, or automatic character selection from a running game.

   Phone profiles are display projections. The local cache preserves numeric
   tokens as received, but the Mac's original document remains the exact source.
   The phone never uploads its cached profile as replacement authority.

5. Complete physical acceptance before claiming onboarding success: confirm WoW
   receives the Pixel's native A/Jump and stick input; check Base, LT, RT and
   LT+RT icons against the actual actions; verify neutral/release on disconnect;
   log into a second character and verify its own profile becomes active; return
   to the first character and verify the same profile identity, phone layout and
   user edits remain. Preserve the default profile and unrelated setups. A native
   helper build, addon install, HID publication or mock snapshot alone does not
   satisfy this acceptance.

Developer verification uses `verify-thumble-live.py` (Lua sender and onboarding),
`verify-wow-optical.sh` (native optical decoder), `verify-wow-profile.py` (native
profile/icon preparation and edit preservation), and host `wow_setup`/`wow_live`
tests. `verify-wow-runtime.py` exercises the real no-input receiver with synthetic
capture data against an explicit isolated owned fixture. It verifies character
creation, A/B/A selection and profile reuse, manual-selection preservation during
refresh, frozen-frame rejection, stop isolation, and preservation of unrelated
raw state. This does not establish live game capture or phone delivery.
Use a separate Cargo target directory when another task is building this checkout,
so a binary from a different source tree cannot replace the tool under test.

The Android owned-transport checks are
`OwnedPhoneEditsTest`, `OwnedPhoneEditPersistenceDeviceTest`, and
`OwnedPhoneEditTransportDeviceTest`. After building/installing the debug app and
instrumentation APK in place, `scripts/verify-owned-phone-android.py --host …
--bridge … --adb … --serial …` runs the actual phone client against a temporary
receiver with `--no-input`, disposable trust, and a scoped ADB reverse mapping.
It verifies committed edits, explicit receipts, A/B/A profile selection, and exact
unrelated Mac source preservation; it does not change the phone's real pairing.

The retained iOS checks use the `ThumbleiOSProtocolTests` hosted XCTest scheme on a
dedicated simulator. `scripts/verify-owned-phone-ios.py --host … --bridge …
--xctestrun … --simulator …` supplies a temporary no-input receiver to the real
client test. It restores simulator preferences, uses a disposable owned cache,
and leaves the actual lab authority and phone pairing untouched. The shared
snapshot/session/cache adapters also run on the existing 512 KiB stack test.
