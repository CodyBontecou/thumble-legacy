# Thumble Link 0.3.0

Export your existing WoW Forever or Classic Era keyboard bindings into a Thumble iPhone setup.
This version targets **Forever interface 16001** and **Classic Era 1.15.9,
interface 11509**, using Blizzard's default action bars. It requires the matching
Thumble Mac app/CLI from this repository (including Classic Era importer support).

## Install

1. Copy this entire `ThumbleLink` folder into your chosen client's
   `Interface/AddOns` directory. `ThumbleLink.toc` must be directly inside
   `Interface/AddOns/ThumbleLink`, with no extra nested folder.
   On this Mac, the client folders are `/Applications/World of Warcraft/_forever_`
   and `/Applications/World of Warcraft/_classic_era_`. Install separately in each
   client's `Interface/AddOns/ThumbleLink` folder when using both.
2. Enable **Thumble Link** in the character-selection AddOns list and enter WoW.
3. Outside combat, type `/thumble`.
4. Select Main bar, Extra bars, and/or Utility, then click **Generate export**.
5. Copy the selected code with Cmd+C on Mac or Ctrl+C on Windows.

## Import on Mac

Click **Thumble Link…** in the Thumble Mac toolbar. Paste the code, optionally
name the setup, choose **Controller** under Layout, and choose **Review Import**,
then **Create Setup** (or **Refresh Setup**). Controller groups movement at the left,
combat and Jump at the right, and targeting, consumables, and menus separately.
Your paired
iPhone receives the setup through Thumble's existing connection. WoW should use
its keyboard/mouse interface for this setup.

You can also paste into the terminal:

```sh
thumble wow import --clipboard --dry-run
thumble wow import --clipboard --layout controller
```

For a file or standard input:

```sh
thumble wow import export.txt --dry-run --json
thumble wow import export.txt --name "My Warrior"
cat export.txt | thumble wow import -
```

New imports use Controller for up to 24 bound commands, with two extra utility rows
when needed. Larger imports use Button grid so every supported binding is retained.
Choose **Controller** explicitly to convert an existing imported grid. Choosing a
layout rebuilds imported positions and appearance while preserving bindings, custom
labels, and deleted controls; leave **Automatic** selected for ordinary refreshes.
The CLI equivalent is `thumble wow import --clipboard --layout controller`.
Controller styling uses native controls in both appearances; it does not borrow a
Warrior skin's artwork or invent native gamepad/trigger-layer assignments.

Forever and Classic Era exports create separate character setups, even for matching
character GUIDs. Re-export after changing your WoW bindings. Importing again refreshes the same
character setup, even if you renamed it. Phone positions, styling, deleted controls,
and manually edited labels/outputs survive refresh. A changed imported binding
updates unless you edited its phone output. An unavailable/removed binding clears
an unchanged imported output. Refresh does not change WoW bindings.

## First-version limits

- Configuration snapshots only. No background connection, live character/form
  detection, cooldowns, or ability pictures. Icon file IDs are retained as metadata;
  they are not image files and are not rendered by this version.
- Main-bar labels describe the page/form active at export time. The same key may
  invoke a different action after WoW changes pages or forms.
- Blizzard bars and common utility commands only. Custom action bars,
  click-casting, pet/vehicle/override bars, and native gamepad layers need adapters.
- Unbound, mouse-wheel, mouse-button, and gamepad-only actions are shown as skipped.
  A supported secondary keyboard binding is used when the primary is unsupported.
- Keyboard output uses Mac physical key positions based on a US layout. Check
  punctuation on other layouts. Cmd/Alt combinations keep their modifier meanings.
- Export selected bars that you actually need. Large imports may need rearranging
  in the editor; refresh can add new controls without moving your existing ones.
- Exports include character GUID, name, realm, class, client build, timestamp,
  command names, and action metadata. Treat the code as your character configuration.

The addon never sends gameplay commands, rewrites bindings, exports macro bodies,
opens network connections, or requires SavedVariables. Each phone press uses the
existing Thumble key-down/key-up input path.

This implementation has mocked exporter/importer coverage for both clients, including
Classic Era paged slots, extra bars, and legacy spell metadata APIs. Installation,
export UI, and actual key behavior still need acceptance testing in each exact
client build; source inspection and mocks do not establish live gameplay results.

## Experimental live ConsolePort transport

The source now includes an opt-in optical sender, enabled with `/thumblelive`
(or `/thumblelive on`) and hidden with `/thumblelive off`. It draws a visible
256 × 320 UI-unit panel containing paged character/action data and an icon atlas.
The normal `/thumble` keyboard export remains available independently.

The native capture bridge reads only a single visible supported WoW window and
requires macOS Screen Recording permission. It returns the bounded telemetry
frame and cropped atlas, without returning or saving the whole window. Two WoW
windows produce an ambiguous-source status instead of selecting a character.

The protocol carries character GUID, realm, class, ConsolePort bindings for all
four trigger banks, action labels, icon identities, and WoW's enumerated gamepad
devices. Snapshot refresh waits until outside combat. Leaving the world hides the
panel. Receiver validation rejects corrupt/oversized messages; repeated frozen
frames do not renew liveness.

**Implementation status:** Lua sender, native capture/decoder, native profile/icon
preparation, Rust reassembly/switch intents, receiver observer scaffolding and
CLI/MCP sync controls are implemented. The isolated end-to-end receiver test
exposed the existing format-4 authority gate: it correctly rejects profile-owned
appearance banks. Owned-authority upsert, format-5 phone transfer and migration
must be connected before automatic switching can work. Neither the in-game
character-swap check nor physical phone acceptance is complete.

The ConsolePort 3.3.11 onboarding adapter is also present, with version/device/
combat checks, character-local preset application, keyboard preservation checks,
rollback and `/thumblesetup restore`. Its repository setup file defaults to
**disabled**; copying the addon manually does not change bindings. CLI
`thumble wow setup` and MCP `setup_wow_consoleport` now install the complete addon
with declarative onboarding enabled. They provide read-only plans, exact plan-hash
checks, running-game protection, private addon/settings backups and rollback for
ordinary failures. See the [onboarding workflow](../../docs/thumble-link.md#consoleport-onboarding).
Existing controller bindings are kept unless replacement is explicitly enabled.
Tutorial completion flags are never forged. `/thumblesetup` reports setup state;
`/thumblesetup apply` explicitly reapplies the preset after onboarding is enabled.

The development controls are `thumble wow sync status|start|stop|request-permission`
and MCP `wow_sync`. Start requires configuration writes and a migrated owned
profile authority. They do not bypass the migration or phone compatibility gates.
Installing this addon alone does not switch the phone's active profile.

Developer checks: `scripts/verify-thumble-live.py`,
`scripts/verify-wow-optical.sh`, `scripts/verify-wow-profile.py --bridge <helper>`,
and the host `wow_live` unit tests. `scripts/verify-wow-runtime.py` exercises a real
no-input receiver against synthetic optical frames; its full acceptance currently
fails at the documented authority-format gate. `verify-wow-setup-cli.py --bin
<tools-directory>` verifies public CLI/MCP plan parity and installation without a
receiver, profile catalog or desktop application.
