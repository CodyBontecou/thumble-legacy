# Hunter camera and mouse input QA — 2026-10-20

The revised Hunter profile and skin were locally installed on the existing native receiver and the physical Pixel 7. The phone's package hash independently matches **77fe5da8064c82dcec6bfbb2ab757fefe2f81ae9e3ba2df5f2dba43c9b6afb0a**. The native snapshot has 21 controls; every other profile and the default profile ID compare unchanged against the private pre-update backup. Accessibility remained granted after the update. No game abilities were automated.

## Functional evidence

A focused AppKit test application received actual OS mouse events from the physical phone:

- Camera held at a stationary displacement: one `rightMouseDown`, **39 `rightMouseDragged` events**, one `rightMouseUp`; summed horizontal movement **158 points**, vertical 0. Final pressed-mouse mask 0.
- L click: exactly one left down/up pair; final pressed-mouse mask 0.
- R click: exactly one right down/up pair; final pressed-mouse mask 0.
- Camera hold followed by Android Home: one right down, **32 right-drag events**, one right up; final pressed-mouse mask 0. This verifies window-loss cancellation without sending a finger-up first.

The first bare command-line probe could not reliably obtain foreground status over WoW. It was replaced with a private proper `.app` wrapper. Input assertions were gated by the focused responder's recorded foreground state. Two later supplemental hold attempts produced no events after the receiver closed normally; they are not passing tests. The native capture ends with `Mac helper quitting` and release-all, not a crash. The original hold, clicks, and cancellation assertions above passed before that close. A longer hold, resumed input, and cursor check are recorded below only if subsequently completed.

Argent supplied phone screenshots and accessibility targets. Due its previously reproduced normalized landscape input mapping failure on this Pixel, native ADB input used the observed 2400×1080 logical viewport. `camera-pixel-7-landscape.png` is the physical phone's new layout. All private raw snapshots, credentials, OS probe logs and receiver diagnostics remain outside the distributable workspace.

## Code and model verification

- Android full unit suite: **214 tests, zero failures**; `lintDebug` and `assembleDebug` passed.
- Android focused cases cover frame-clock camera continuation without additional MOVE events, simultaneous keyboard movement, neutral/cancel release, inversion, camera right-button ownership, independent left click, and click-pad movement/cancellation behavior.
- Focused Swift model/fixture parity tests passed, including old trackpad decode/encoded-default compatibility, explicit click settings, and camera digital suppression.
- Rust core/projection/generation and receiver tests passed, including the three-trackpad capacity and safe preservation of camera/click settings.
- `./scripts/verify-stack-safety.sh` passed, including constrained-thread tests, unchanged inline-size budgets, Mac Debug and iOS Debug builds. Updated CLI and configuration bridge builds passed.
- Skin review remains separately documented in `camera-qa.md` and the two independent visual critiques. Publication approval remains pending.

No in-game camera angle assertion is claimed: OS delivery was checked in the focused test app, with the user retaining their game state.

## Final reconnect limitation

A second receiver shutdown occurred while waiting for a final supplemental test. The capture also records another native configuration commit during this period; no receiver crash report was found. The receiver was restored again and the temporary focused test app closed. Supplemental longer-hold/cursor-after-reconnect checks remain unverified; they do not replace or invalidate the earlier passing camera/click/cancellation OS event assertions. A phone reconnect may be required if another local operation closes the receiver.
