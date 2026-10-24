# Classic Era ConsolePort lab — 20 October 2026

Follow-up to [the phone-controller spike](wow-phone-controller-spike-2026-10-20.md). User authorized receiver ownership/signing resolution, ConsolePort installation and a separate phone profile.

## Prepared setup

- Development background-only receiver at `build/ConsolePortLab/Thumble Controller Lab.app`. Uses the already authorized `com.codybontecou.PocketPadMac` identity with the matching Apple Development certificate and embedded macOS development provisioning profile. This is not the normal `com.codybontecou.ThumbleHost` distribution identity or a production release.
- Final artifact passed `scripts/verify-hid-signing.py --require-hid --bundle-id com.codybontecou.PocketPadMac --distribution development`. Helpers are signed without HID privileges. The profile includes this Mac's **provisioning UDID** (which differs from its platform UUID).
- Receiver has separate state under `build/ConsolePortLab/state`, Bonjour discovery and port **9876**. Legacy desktop copies were gracefully quit; another chat that reopened them was paused by the user. Confirm one owner before each run.
- Profile **WoW Classic Era · ConsolePort Lab**, ID `7FEFB7BB-9BF2-573B-851D-7987DA834FAA`, selected in the lab. Based on `wowForever` solely for its physical gamepad contract: controller output, two analog sticks without digital directions, D-pad, face buttons, held triggers, shoulders, stick clicks and menus. No spell-binding assumptions. Lab default was left unchanged.
- Portable profile export at `build/ConsolePortLab/consoleport-lab-profile.json`, revision 2, SHA-256 `55d921c4b164f40afa6a816cb9cd4ff19bbae768e0cd4ca7512d09131c71c95d`.
- [ConsolePort 3.3.11](https://www.curseforge.com/wow/addons/console-port/files/9254142) installed into Classic Era's `Interface/AddOns`. All nine directories were byte-verified against the downloaded archive; existing ThumbleLink retained. TOC includes interface `11509`. Archive SHA-256: `928fd0423447677f455150f6c625eaed1dad18f7b260c6a2e03cc4c5c279d871`.
- Private backups of native preferences, original Rust state, and Classic Era WTF settings are under `build/ConsolePortLab/backups`. Do not commit or expose these files; they can contain credentials.

## Runtime diagnosis and pending gate

Initial signed receiver creation failed before parsing the report descriptor: `IOServiceOpen(IOHIDResource)` returned **0xe00002e2**, “not permitted.” A separately signed no-input probe reproduced this; CoreHID construction also failed. Taskgated accepted the development profile; the probe showed validated kernel entitlements and no process sandbox. These observations do not establish successful virtual-device publication.

The independently Launch Services-started receiver then reported **Accessibility not trusted**, although checks from a process launched through Codex had reported trusted. The actual receiver-owned permission response is the relevant evidence. A permission prompt was requested, Accessibility settings opened, and the exact signed lab bundle revealed in Finder. After the user added/enabled the exact lab bundle in Accessibility, `gamepad retry` succeeded: phase `ready`, signed claim present, initial report submitted. The receiver's separate Accessibility status still reported false at the time of that retry, so do not equate that status with the HID creation outcome. The approval changed the observable HID result; broader keyboard/pointer permission remains a separate check.

Signed/profile verification, Accessibility approval, HID creation, GameController enumeration and WoW input recognition remain separate gates. After approval, the independent probe found **Thumble Virtual Gamepad** with descriptor SHA-256 `48cd0a7664686a06eddcf5b6ebe31e76a3b418cf91c2d960258465a3c4ef6129`. `GCController.supportsHIDDevice` returned **false**, and the GameController list was empty. Thus signed HID publication passed, but Apple GameController recognition did not. Actual WoW native input still requires testing. No gameplay pass or physical-phone profile delivery has been established yet.

## Local operation

`build/ConsolePortLab/lab-host` targets the isolated receiver for `status`, `accessibility`, `gamepad`, `pairing-code`, `controls`, `release-all` and `stop`. Its README contains the launch command and test sequence. Do not start a second owner.

The packaged Swift CLI bridge intentionally discards caller-selected state paths. An initial template install therefore went to the original offline Rust catalog; it was removed through the supported CLI, and the original selection restored. Comparing with the backup confirmed unchanged profiles, active/default IDs, key/output maps, server identity and trusted clients. Revision/history metadata may advance through those reversible transactions. Native preferences were not overwritten.

For the isolated lab, the setup uses its supported typed `cli-profile-transaction` socket endpoint with expected revisions for mutations, rather than writing state JSON. `lab-control.py` is a local diagnostic client for that endpoint. Its export produced the portable artifact above. No application source changes or production packaging changes were made.

Next: pair the physical phone to the new lab server identity, confirm profile synchronization, restart Classic Era to discover the new addon directories, and run ConsolePort's controller setup outside combat. Verify neutral/release on disconnect before gameplay testing.


## Physical Pixel 7 acceptance

Using Argent on the connected Pixel 7, cancelled reconnect to the stopped legacy receiver and paired the installed Android app with the isolated lab. Receiver status confirmed `Thumble Android (Pixel 7)` connected. The phone connection drawer showed **WoW Classic Era · ConsolePort Lab**, practice mode off, and UDP plus WebSocket transport.

Portrait A tap produced `gamepad_button:South:true` then `false`. A Move-stick drag produced increasing left-stick X values (up to approximately 0.514) followed by `gamepad_stick:Left:0.000:0.000`. Final status: HID phase ready, last report result 0, 61 reports, all axes/triggers zero, no held buttons. Evidence files are in `build/ConsolePortLab/pixel-portrait-A.json` and `pixel-stick-result.json`.

Landscape rendering was inspected, but Argent coordinate injection in that orientation hit RT when targeting the displayed A control. Both tap and custom gesture paths showed that discrepancy; portrait input verified the intended mapping. The unexpected trigger was explicitly released and subsequent status confirmed neutral. Do not count this as landscape touch-input acceptance or conclude that the profile itself remaps A to RT. Phone orientation was returned to portrait for the successful input tests.

The Pixel remains paired on the lab profile. Temporary pairing-code secret entries used for Argent were removed without changing unrelated secrets. No WoW gameplay recognition result is claimed. Restart Classic Era to discover the installed ConsolePort folders, then run its controller setup and test recognition in game.


## Compatibility repair — 21 October 2026

The generic v1 HID identity was the first recognition failure. A signed, provisioned
normal-security probe compared it with `045e:028e` using the old generic reports and
then the complete Xbox-compatible report contract. The known identity alone
registered an extended GameController but all 30 tested generic report samples
remained neutral. The compatible 20-byte layout delivered 18 independently checked
face/shoulder/stick-click/D-pad/trigger/stick cases with correct semantic values.
System Home/Menu/Options actions were not sent by that automatic probe.

Both receiver implementations now use the v2 codec and a stable Thumble identity.
The lab bundle was backed up, updated, signed with its existing approved development
identity and verified again. The independent consumer sees `Thumble Virtual Gamepad`
as an extended Xbox-category controller with `supportsHIDDevice=true`. The underlying
HID service reports transport `Virtual`, despite the configured `USB` compatibility
property. A live installed-control A down/up test initially caught the Rust adapter
still submitting the old byte count. It now derives the FFI length from the report
buffer, and the repeated consumer test observes A pressed/released with all final
buttons and axes neutral. This proves the installed receiver's consumer path; it
does not prove physical Pixel or WoW gameplay input.

Source-bound results are recorded in `build/ConsolePortLab/hid-v2-acceptance.json`.
The final Rust run passed 277 host and 36 MCP tests (one existing ignored), with
Clippy clean; Swift HID tests passed 11 cases and the stack gate passed 43 cases plus
macOS/iOS Debug builds. The no-input A/B/A character-sync process test also passes.
The optical decoder now uses a full-width fiducial and normalizes its square atlas;
its tests cover ten scales, including fractional values.

The installed Classic Era ThumbleLink matches the new identity and capture marker.
ConsolePort itself was unchanged. The update backed up the addon and 17 settings
files under `Interface/ThumbleBackups/2abeca13-493d-475f-9def-0524aa5cea17`.
An Argent launch found the Pixel locked. The user was asked to unlock Thumble and
to log into Classic Era and report `C_GamePad.GetAllDeviceIDs()`. Both human checks
remain pending. The native helper reports capture authorization when invoked from
the agent, but capture authorization from the independently launched receiver and
actual game pixels still require verification. Live sync remains disabled on the
legacy lab authority; format-5 migration and phone transfer are still required.
