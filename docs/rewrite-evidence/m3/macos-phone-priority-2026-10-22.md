# macOS and physical phone review priority — 2026-10-22

The user superseded Windows-at-launch and entirely-headless execution: prioritize a local headless macOS receiver plus iOS and Android, test with the connected iPhone and Pixel through Argent, and prepare store review candidates. Windows/Linux follow while those candidates are in review. Optical live sync remains required. This does not authorize store publication or certify a release.

## Frozen source

Private commit `e281df3` contains 69 curated files. The public source projection excludes the private rewrite plan and history: `/private/tmp/thumble-mac-phone-first68-public-source-20261022.json`, SHA256 `694c08a97a7825ec88ef35db043c700981f3d9142399e4de9fde2d41279d42e5`. Public commit `e53013cb505e9f031e67b00996aa9ab12ade9742` has sole public parent `2dee80c96704d939797dd2df976caca97fa0b93d` and verified tree `58e2e2a69b4a7e34721632daed32a45eae713329`. The projection adds explicit physical phone protocol v9, native output mode selection, a production 18-control starter operation through CLI/MCP, optional capture packaging, developer signing preparation, an output shutdown race fix and physical-device UI corrections. Later QR work and experimental iOS analog UI probes are excluded.

Isolated source gates pass generated contracts, boundaries, notices, formatting, all three TypeScript configurations and 303 tests / 5,063 assertions (two intentional opt-in skips). macOS-feature receiver tests pass 149 unit and 24 process tests. Standalone recording packaging passes 1 test / 67 assertions on unchanged recheck; the initial MCP sample-phase failure and an independent successful SDK debug probe are preserved separately. No production/test deadline changed. Full native SDK acceptance is recorded in the individual frozen capsules; this combined run is not a claim of physical touch acceptance.

## Actual devices and Mac

Both bundled Release phone apps build and install independently of Metro. The iPhone navigation flow passes 21 checks over 24 steps, including safe-area Done and explicit Mac-input versus recording intent. Pixel light-theme labels and outlines are corrected; navigation flows pass, but full dark-mode acceptance remains pending.

The old Mac lab owner was queried using its explicit state directory, released, verified neutral and gracefully stopped. Its authority/configuration remains preserved. The signed rewrite receiver started in recording mode with a separate private identity/runtime. The production starter operation committed revision 1 and the saved layout activated successfully. After recording pairing tool limitations were preserved, the service restarted in explicit macos-hid mode using the same private identity/runtime. Saved activation reports inputEnabled true, generation/revision 1 and a neutral report. An independent read-only IOKit/GameController probe observes exactly one supported Thumble HID device, one extended gamepad, all zero axes/triggers and released buttons. This is development signing and neutral enumeration, not gameplay acceptance.

Manual LAN invitation entry has not established either phone. Argent landscape taps on the Pixel and a 30-second iPhone keyboard-return watchdog prevented submitting the native forms. Secrets were injected using server-side placeholders and kept outside repository/evidence. These are preserved tool failures, not proven native enrollment failures. Native QR scanners and a local CLI QR artifact path are the next pairing implementation.

## Outstanding release gates

Real phone enrollment, physical multi-control input, consumer interpretation, disconnect/reconnect and fault recovery, both game experiences, mandatory live-sync authority/phone delivery, release signing/notarization, rich editing/migration and store submission checks remain open. A working development HID device does not discharge these gates.
