# Local rich background editing

Gradient, tile and image backgrounds now use typed local authority transactions through `thumble customization set`. The desktop preference-store route and its duplicate mutation implementation are removed. Both live and stopped receivers use the same authority ownership, source revisions, release barrier, atomic persistence and durable retry path.

```sh
thumble customization set --background-gradient '#102030,#405060' --profile PROFILE
thumble customization set --light-background-tile dots --tile-scale 1.5 --profile PROFILE
thumble customization set --dark-background-image image.png --image-mode fit --profile PROFILE
thumble customization set --light-background '#FF0000' --profile PROFILE
thumble customization set --reset-background --profile PROFILE
```

Existing aliases and parsing remain available. A request can combine scalar preferences, background edits and device sizing in one transaction. The CLI preserves the old precedence: scalar/colors, global gradient/tile/image, scoped gradient/tile/image, then background reset. Up to fourteen bounded changes account for that full combination. File-backed image reads use a bounded regular descriptor, refusing symlinks, FIFOs and files over 2,500,000 bytes before opening a transaction. The original file is never modified.

Local MCP extends the existing `customization.set` draft operation with `changes.backgroundEdit = {action:"fill", scope:"all"|"light"|"dark", fill:...}`. Solid/gradient/tile fill fields use the existing safe fill contract. Image fills use `{kind:"image", dataBase64:"...", contentMode:"fill"|"fit"|"tile", fileName?:"display-basename"}`. Data must be nonempty canonical Base64 for at most 2,500,000 bytes. A display name is bounded and cannot contain separators or controls. Tools accept bytes rather than arbitrary host paths or remote URLs. Configuration-write opt-in and exact draft/authority revisions still apply. Local stdio admits the bounded image-bearing edit within its existing large-source ceiling while ordinary edits keep their smaller limit. The same framing admission now covers the existing bounded customization/style import tools and asset-import actions.

Owned backgrounds are edited explicitly in the canonical layout and all eight saved light/dark × Base/LT/RT/LT+RT appearances. All-scope edits replace every saved background; scheme-scoped edits preserve the opposite scheme, including inherited global paint. Solid-color changes and background clear use that same policy, so an older image cannot reappear from a saved trigger state after resetting the background. A change present only in one saved state still produces a correlated profile change and timestamp. Repeating an identical edit at a later clock time is a no-op.

The authority independently predicts the permitted background changes in the canonical layout and every saved state, validates requested image hashes, and refuses residual bank paint or a different valid resource blob. Raw merging patches saved states by their scheme/action identity, preserving row extensions, escaped state-label spellings and exact numbers instead of replacing the whole bank array. A plain owned profile with an explicit null bank retains that representation. Controls, geometry, output bindings, groups, haptics, style/asset libraries, local launch data, selection and unrelated profiles stay outside the background-edit scope.

The retained style-import merger also matches decoded state identities and accepts an explicit null source bank when importing a saved eight-state catalog. Regression fixtures pin both cases.

Binary resources share the existing immutable content-addressed pool. Merging retains references from the complete raw profile, including opaque metadata and launch data, then prunes only unreachable blobs. Repeated maximum-size replacements therefore do not accumulate orphan images. A temporary union is bounded to twice the document budget while reachability is calculated; the final document and resource budgets remain unchanged. Precise metadata is read for traversal without serializing it back through a numeric projection.

Legacy profiles retain their native orientation behavior. Owned profiles still use the canonical landscape contract. No automatic migration, phone-format activation, or desktop/package/portrait runtime deletion is introduced. See [verification evidence](feature-removal-evidence/rich-backgrounds/README.md) and [the remaining cutover gates](cli-mcp-cutover.md).
