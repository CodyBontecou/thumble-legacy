# Local theme transactions

`thumble theme apply PRESET [--profile PROFILE] [--variant landscape] --invocation-id UUID` and the `theme set` alias use the typed local authority path. Canonical presets are `cavern-glow` and `soft-white-controller`; native CLI aliases resolve to those IDs before the authority request. The high-level request rejects unsupported preset IDs before drafting. Selection may use the active/default profile, a UUID or a catalog-resolved name.

Both legacy and explicit owned authorities use normal revision-checked drafts, native transformation, input release, atomic persistence and durable commit receipts. A committed exact retry works without the native helper. Invalid presets and owned portrait requests fail without authority writes. This does not activate authority migration or add a GUI fallback.

The local MCP already exposes the same `theme.apply` transformation through `begin_configuration_draft`, `edit_configuration_draft` and `save_configuration_draft`. Agents use the returned exact revisions and operation/commit IDs; edits remain private until save. No additional tool or write opt-in bypass is introduced.

Theme application changes appearance using the existing native preset definitions, including their reusable visual styles and tactile-feedback values. It preserves control identities, labels, part outputs, geometry, executable bindings, active/default selection and unrelated profiles. Existing owned appearance banks remain saved; native three-way appearance resolution combines canonical theme edits with those states. This is distinct from removing skin packages or discarding their migration data.

Evidence is in [local theme verification](feature-removal-evidence/local-themes/README.md). The verifier exercises both presets, owned and legacy authorities, a banked target, unchanged binding sidecars, helper-free replay, invalid requests and actual local MCP draft/save/replay with input disabled. Unit/codec evidence does not establish physical haptic feel or live game acceptance.
