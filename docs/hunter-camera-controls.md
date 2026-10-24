# Hunter camera and mouse controls

The Hunter keypad keeps the imported keyboard commands and game icons. Move retains W/S/Q/E. Camera is a second native joystick on the right: deflect it to hold right mouse and send continuous relative mouse movement; center or release it to stop and release right mouse. Dead zone, sensitivity, axis inversion and cardinal locking use the existing joystick settings. Camera sends neither keyboard directions nor a virtual gamepad axis.

Cursor remains a relative pointer pad with one-finger left click and two-finger right click. Separate **L click** and **R click** pads beneath it click their named mouse button without moving the cursor. They click on a completed tap, rather than holding a button during a drag. A right-click tap during a Camera hold is ignored so it cannot release Camera's right-mouse ownership; left click remains independent.

The portable settings are shared by iOS, Android, the CLI configuration bridge and Rust profile projection:

- Joystick `joystickOutputSettings.analogTarget`: `mouse_look`; normalization forces `sendsDigitalDirections=false`.
- Trackpad `trackpadSettings.clickButton`: optional `left`, `right` or `middle`; omission preserves finger-count click behavior.
- Trackpad `trackpadSettings.movesCursor`: defaults to `true`; `false` makes a discrete click pad. Existing documents retain their old encoded shape when both new settings use their defaults.
- A layout allows two joysticks and three trackpads, accommodating Move/Camera and Cursor/L click/R click.

CLI authoring examples:

```sh
thumble element add joystick --label Camera --thumbstick --target camera --x 0.7 --y 0.58
thumble element add trackpad --label 'R click' --click-button right --moves-cursor false --tap-to-click true
```

MCP element patches accept the same `joystickSettings` and `trackpadSettings` JSON values. Profile import/export preserves them. The headless macOS receiver and retained native receiver both emit held-button drag events with explicit relative mouse deltas. No new wire message type is required.

Phone focus loss, profile replacement, disconnection and gesture cancellation retain the existing release-all behavior. Android camera motion uses the UI frame clock; iOS uses a cancellable frame task. Both cap a single delayed tick at 50 ms.

Skin evidence and pending publication approval are under `docs/skins/examples/classic-hunter-warband/reviews/camera-*`. Local installation does not constitute publication approval.
