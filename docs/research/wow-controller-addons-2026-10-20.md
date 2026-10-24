# WoW controller addons — 20 October 2026

Primary-source follow-up to [the October 9 controller research](wow-controller-support-2026-10-09.md). Sources retrieved **2026-10-20**. This note researches addon selection and an experiment; it does not establish successful Thumble-to-WoW input or install addons.

## Recommendation and popularity evidence

**Start with ConsolePort alone on Classic Era; compare Forever's native gamepad UI separately.** ConsolePort is the established full controller suite found in this research. Its owner documents native gamepad input since Shadowlands, controller UI navigation and targeting, with no external mapper required. That establishes the addon's role, not compatibility of Thumble's virtual device on macOS. [ConsolePort README](https://github.com/seblindfors/ConsolePort/blob/master/README.md)

| Addon | CurseForge lifetime downloads observed | Current release/update observed | Role in the experiment |
| --- | ---: | --- | --- |
| [ConsolePort](https://www.curseforge.com/wow/addons/console-port) | 4,253,384 (~4.25M) | 3.3.11, Oct 19 | Main controller UI, bindings, targeting and navigation |
| [Immersion](https://www.curseforge.com/wow/addons/immersion) | 8,409,887 (~8.41M) | 1.4.61, Sep 24 | Optional quest/gossip replacement with configurable hotkeys |
| [Dialogue UI](https://www.curseforge.com/wow/addons/dialogueui) | 7,940,332 (~7.94M) | 1.0.6, Oct 4 | Alternative quest UI; explicitly supports gamepad selection and quest acceptance |
| [DynamicCam](https://www.curseforge.com/wow/addons/dynamiccam) | 2,717,220 (~2.72M) | 2.21.4, Oct 16 | Optional situation-based camera framing and transitions |
| [Easy Controller — Forever](https://www.curseforge.com/wow/addons/easy-controller-forever) | 3,839 (~3.8K) | 1.18.2, Oct 15 on indexed page | New Forever-only extension of Blizzard's native gamepad interface |

These are **project lifetime downloads**, including repeated updates and supported flavors, not unique people, active users, or a measured controller market share. Project age and audience differ. Immersion and Dialogue UI also serve keyboard/mouse players; their larger counts do not make them competing full controller solutions. Counts vary between cached project/file pages. Easy Controller's page was indexed five days before retrieval, so its count/release is a dated observation.

Immersion and Dialogue UI replace overlapping quest surfaces: compare them one at a time. DynamicCam modifies camera behavior, so leave it off until raw right-stick/camera behavior is understood. These are experiment-design recommendations, not published interoperability guarantees. [Immersion description](https://www.curseforge.com/wow/addons/immersion), [Dialogue UI features](https://www.curseforge.com/wow/addons/dialogueui), [DynamicCam description](https://www.curseforge.com/wow/addons/dynamiccam)

## Exact client compatibility and setup

ConsolePort's [3.3.11 file, ID 9254142](https://www.curseforge.com/wow/addons/console-port/files/9254142) explicitly lists Classic **1.15.9**, Forever **1.60.1**, Retail **12.1.0**, MoP **5.5.4**, Titan Reforged **3.80.1** and TBC **2.5.6**. The same listed archive is `ConsolePort-3.3.11.zip`; this is stronger evidence than assuming a Retail download works on Classic. Verify the installed client's build and the extracted TOCs before use; an addon flavor listing is not an in-game pass.

Use the selected client's own `Interface/AddOns` directory: `_classic_era_` for the installed Era client and `_forever_` for the installed Forever client. Avoid blindly following a beta-path example or installing into another flavor. Preserve existing addon folders and back up the selected client's WTF bindings/settings before an experimental setup. Extract the archive's addon directories at the AddOns root, then restart the client and verify its AddOns list. ConsolePort's published gallery shows an initial controller guide, binding/action-loadout editor, action bars, rings and keyboard. [ConsolePort project/gallery](https://www.curseforge.com/wow/addons/console-port)

The optional companion projects currently advertise Retail, Forever and Classic variants; inspect the specific file's build metadata before installing. Immersion also lists Titan Reforged; Dialogue UI and DynamicCam list TBC. [Immersion files](https://www.curseforge.com/wow/addons/immersion/files/all), [Dialogue UI](https://www.curseforge.com/wow/addons/dialogueui), [DynamicCam](https://www.curseforge.com/wow/addons/dynamiccam)

Easy Controller is specifically for Forever's built-in UI, with trigger latching, button mapping, wheels and chat input. Its author recommends Steam Input for standard mapping and extra controls; **that is not proof that Steam Input is necessary or appropriate for Thumble on Mac**. Its current page warns of a gamepad bar disappearing with custom Edit Mode layouts after the October 9 patch and recommends the default gamepad layout as a workaround. Treat that as an author report requiring confirmation on the installed build. Test this route separately from ConsolePort. [Easy Controller project](https://www.curseforge.com/wow/addons/easy-controller-forever)

## What the entitlement does and does not establish

Blizzard documents native gamepad APIs, button icons, mixed mouse/gamepad controls, Interact Key and action targeting. Those features make a phone-as-gamepad approach plausible, but the article does not certify arbitrary virtual HID descriptors on macOS. [Blizzard accessibility documentation](https://news.blizzard.com/en-us/article/23876525/accessibility-updates-in-dragonflight)

The user's reported virtual HID entitlement approval removes an authorization prerequisite. The running receiver still needs the exact signed Boolean claim and matching provisioning authorization. HID publication, Apple GameController recognition, WoW enumeration and correct semantic input are separate gates. No official source inspected guarantees that Thumble's generic HID will pass all of them. The local [virtual-controller acceptance guide](../virtual-controller-support.md) already makes this distinction and provides the independent probe. Its older “no approved profile found” statement describes its inspection date, not a contradiction of a later approval.

## Minimal phone experiment

1. Use one authoritative headless receiver and CLI/MCP configuration. Record receiver identity, signed/profile authorization, controller descriptor and client build. Run `gamepad status`/`doctor` and the existing read-only compatibility probe while the phone is paired. Do not launch a second receiver to work around an ownership problem.
2. Reuse the existing `wowForever` template for the native input experiment, including with ConsolePort on Era: local CLI inspection confirms controller output, Move on `left_stick`, Look on `right_stick`, and pure gamepad face/D-pad/trigger/bumper/stick-click/menu outputs. ConsolePort should own the in-game binding choices. A familiar Xbox-looking template can still be configured as keyboard output; inspect output mode and analog targets rather than assuming its appearance proves gamepad output. Record the actual installed controls in the [local experiment note](wow-phone-controller-spike-2026-10-20.md).
3. Before addon changes, prove device visibility and neutral/released values. In a safe in-game location test every control, simultaneous movement/camera, diagonal movement, both triggers, held-button release and phone disconnect. Record observed gamepad button names and Y polarity.
4. Classic Era: enable ConsolePort alone, run its controller guide, and map the observed gamepad semantics. Test movement, camera, jump, targeting, interact, one ability, menu navigation and a trigger modifier. Keep bindings and action layouts per character during experimentation.
5. Forever: compare its default native gamepad layout without ConsolePort, then optionally Easy Controller. Native gamepad action storage and ConsolePort binding behavior must not be assumed identical; the project has a specific [Forever gamepad-slot request](https://github.com/seblindfors/ConsolePort/issues/264).
6. Add Immersion **or** Dialogue UI, then optionally DynamicCam, only after input passes. Test combat entry/exit, forms/stances and reconnect before considering the profile usable. Preserve releases/neutralization on every transition.

Success means the **phone's output is recognized and behaves correctly inside the selected Mac WoW client**. CLI `ready`, a visible HID entry, an installed addon, or a correct-looking phone HUD alone is insufficient. If recognition fails, keep keyboard/mouse mapping as an explicitly different fallback while investigating the consumer mapping; do not describe it as proven native-controller compatibility.

Addon snapshot exports, class/loadout import and live icon synchronization remain separate from this first input test; the earlier research's transport and protected-action constraints still apply. ConsolePort's own export surface has an [extended-slot issue report](https://github.com/seblindfors/ConsolePort/issues/266), so do not assume an action-layout export covers every flavor/slot without checking the current implementation.
