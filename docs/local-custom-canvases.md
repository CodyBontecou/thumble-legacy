# Local custom canvas transactions

Custom dimensions are retained design functionality. `thumble device set` and the custom-size forms of `thumble customization set` now use typed local authority transactions instead of requiring desktop persistence.

```sh
thumble device set custom --size 1024x768 --orientation landscape --profile PROFILE
thumble customization set --device-size 1100x700 --orientation landscape --profile PROFILE
thumble customization set --device custom --device-width 1100 --device-height 700 --orientation landscape --profile PROFILE
thumble device show --profile PROFILE --json
```

The native catalog resolves CLI dimensions using its existing rounding and 240...1800 point bounds. A stored ID uses portrait-ordered integer dimensions, for example `custom-700x1100-landscape`. Wire writes accept only exact canonical IDs with `240 <= W <= H <= 1800`. Leading zeros, fractional spellings, reversed dimensions, out-of-bounds values and suffix garbage are refused. Callers needing aliases or normalization use the native CLI; the wire contract never silently repairs arbitrary IDs. The checked-in built-in catalog remains the same 68 frames.

Local MCP uses the existing `device.set` operation through `edit_configuration_draft`, followed by `save_configuration_draft`; configuration-write opt-in and matching draft/authority revisions remain required. Customization requests can combine the selected frame with supported scalar/color changes in one transaction. The current rich gradient/tile/image CLI route still needs its own cutover.

Owned profiles retain one landscape layout and reject portrait frame targets even when the caller selects the primary slot. Legacy orientation behavior remains available only through the bounded legacy contract pending migration. Changing the canvas preserves normalized control geometry, identities, declared outputs, bindings, groups, haptics, styles, embedded resources, all eight saved appearances and unrelated profiles. It does not automatically arrange controls. Existing layout validation/repair and previews remain the way to inspect or adjust placement.

The native source-mirror overlay now compares pooled source values with pooled canonical values and takes orientation from the typed result. It no longer decodes a resource-reference-bearing customization through the inline Data decoder. This also fixes built-in frame changes on pooled owned profiles. Native setter timestamps are restored before the transaction's semantic comparison, so selecting the same canvas at a later clock time is a no-op rather than an invalid timestamp-only transformation.

Writes use normal raw draft/CAS, input release, atomic persistence and durable replay. Original opaque metadata remains exact. Device inspection returns bounded custom dimensions; local profile export remains the interchange path. No automatic migration, public peer activation or desktop/package/portrait runtime removal is activated by this change. See [verification evidence](feature-removal-evidence/custom-canvases/README.md).
