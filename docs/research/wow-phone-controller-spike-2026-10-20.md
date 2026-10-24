# Phone as a WoW gamepad: local spike, 20 October 2026

Companion to [addon research](wow-controller-addons-2026-10-20.md) and the existing [virtual-controller acceptance gates](../virtual-controller-support.md). This records read-only investigation, not a successful gameplay test.

## Recommendation

Try **ConsolePort on Classic Era** first, using Thumble's existing native gamepad output. Separately compare Forever's native gamepad UI with no controller addon. Keep these as separate experiments: the clients and their binding/UI conventions differ. Start with ConsolePort alone; add a dialogue or camera addon only after movement, camera, targeting, and trigger layers work.

The intended path is phone touch → paired local receiver → macOS virtual HID gamepad → WoW native gamepad input → ConsolePort UI/bindings. The addon does not receive Thumble network messages or create the macOS device. ThumbleLink can remain the configuration/export companion; it is not the controller input transport. No external keyboard mapper is needed if WoW recognizes the native device.

## Local evidence

Observed on macOS 26.5.1 (25F80), evening of 20 October 2026 in America/Puerto_Rico:

- Both `/Applications/World of Warcraft/_classic_era_` and `_forever_` exist. Each AddOns directory contains ThumbleLink; no ConsolePort TOC was found.
- A scan of both standard user provisioning-profile caches decoded all 69 profiles. One carries Boolean `com.apple.developer.hid.virtual.device = true`: App ID `67KC823C9A.com.codybontecou.PocketPadMac`, macOS, expiration 4 October 2027 UTC. None of the decoded profiles authorizes `com.codybontecou.ThumbleHost`. This is cached-profile evidence, not final signed-executable authorization or validation of distribution scope.
- `security cms -D` failed to decode the profiles in this environment. The scan used `openssl cms -verify -inform DER -noverify` to inspect their contents. This does not establish Apple certificate-chain trust and does not replace the release verifier.
- `swift -module-cache-path /tmp/thumble-wow-module-cache scripts/probe-gamepad-compatibility.swift 1` opened the HID manager successfully, observed **zero matching Thumble HID devices** and **zero GameController controllers**. This is an idle snapshot; it does not prove the backend cannot work.
- Current CLI `server status --json` returned `legacy_owner_possible`: an authority lock is held without Rust runtime artifacts. Native host `gamepad status --json` found no host control socket. The existing lock was not removed and no competing receiver was started.
- The newest local `wowForever` template was inspected through `build/ThumbleCamera/bin/thumble template show wowForever`. It already provides pure controller output: left/right analog sticks with digital directions disabled, ABXY, D-pad, LT/RT, LB/RB, L3/R3, View/Menu. Its physical input contract is a useful ConsolePort starting point too; its name does not certify Classic compatibility or match ConsolePort bindings.
- The ordinary `xbox` template starts in **keyboard** output mode with digital sticks. Installing that template alone would not establish native controller support.

No user profiles, addon installations, WoW settings, signing settings, or receiver ownership were changed. No gameplay input was injected. The final receiver's HID authorization and WoW recognition remain unverified.

## Concrete experiment

1. Resolve receiver ownership through the supported lifecycle tools. Determine the existing owner before restarting anything. Verify the exact retained headless receiver bundle, App ID, signing certificate, embedded profile and distribution scope with `scripts/verify-hid-signing.py --require-hid`. The cached PocketPadMac grant must not be assumed to authorize ThumbleHost. Preserve required native helpers and the [CLI/MCP product boundary](../cli-mcp-cutover.md).
2. Read/export the current profile and note its active/default identity. Install a separate experimental `wowForever` template named `WoW ConsolePort Lab`, using the supported profile tools; do not overwrite a class profile or change its default. Inspect actual installed control IDs rather than assuming generated IDs or outputs from labels.
3. Confirm `thumble gamepad doctor` and `thumble gamepad status --json` report the owning receiver's actual HID readiness. Repeat the independent compatibility probe with the phone connected, exercising each control. Verify neutral, diagonal directions, stick polarity, independent triggers and disappearance/release on disconnect. A `ready` flag alone does not prove WoW support.
4. Install the **Classic Era** ConsolePort release into `_classic_era_/Interface/AddOns`, after checking the archive's supported game version and folder structure. Use its Xbox-style setup as a glyph/binding convention; Thumble remains a generic HID device. Complete initial bindings outside combat. Confirm the native gamepad enable setting through that client/addon's setup rather than copying Forever UI instructions.
5. In a safe in-game location, test movement and camera first, then jump/interact/targeting, menus, and a small action set. Exercise base, LT, RT and LT+RT inputs. Verify movement + trigger + action simultaneously, release orders, phone disconnect, reconnect and profile switching. WoW/ConsolePort owns spell bindings, secure actions and modifier behavior; Thumble should emit held physical inputs without adding timed spell sequences.
6. Compare two phone interactions using profile-owned controls: ordinary held trigger buttons versus larger reachable trigger regions. Test actual simultaneous touches before considering any optional latch design. A flat phone has no physical shoulder buttons, so modifier reach is likely the main ergonomic constraint. Do not change the binding model just to make a screenshot look like a controller.
7. Run Forever separately with its native interface first. Only add a Forever-specific UI addon after establishing its baseline and checking the exact installed build. Record client build, addon version, receiver/package hash, descriptor hash, mappings, observed failures and successful release behavior.

For cleanup, release all receiver-owned inputs, restore the previous active profile, and disable only experimental addons/settings that were actually changed. Do not reset all WoW keybinds or remove unrelated profiles.

## Success criterion

The phone produces a device WoW recognizes; sticks, buttons and modifier combinations behave correctly; action/target/menu navigation works; and every tested disconnect or configuration transition returns to neutral. Comfortable movement + modifier + action on the physical phone is a separate usability gate. Neither entitlement approval nor addon popularity establishes either result.
