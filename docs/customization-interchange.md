# Local customization interchange

`thumble customization show`, `export`, and `import` use the local configuration authority. They no longer load the desktop preference store. Both live-receiver and stopped-receiver execution use the same typed high-level requests, source revision checks, and transaction ownership.

```sh
thumble customization show --profile PROFILE
thumble customization export --profile PROFILE -o appearance.json
thumble customization import appearance.json --profile PROFILE --invocation-id UUID
```

These are reusable layout files, including executable control declarations and feedback settings. Import deliberately replaces the selected profile's customization and appearance bank. It preserves the destination profile's identity, name, output mode, local launch association, unrelated profile metadata, active/default selection, and all unrelated profiles. Target binding sidecars are rebuilt from imported control declarations; global mirrors change only for the active target. Opaque binding-row metadata is preserved, or removal is refused if it would discard data outside the supported scope. Use profile interchange when the profile identity or other profile settings should travel too.

## Files and exact reads

A classic file is the existing bare customization object. Export returns its exact stored bytes. A banked file has schema `com.codybontecou.thumble.customization`, version `1`, a `customization` object, and an `appearanceBank`. It carries all eight light/dark × Base/LT/RT/LT+RT appearances. Pooled files also include `appearanceResourceVersion: 1` and `appearanceResources`, using the existing SHA-256 binary table. Valid inline bank images are converted to that table at the explicit interchange boundary. Only known native Data fields are converted; unknown metadata and precise numeric literals retain their source bytes. Repeated image bytes share one resource.

Exports exclude profile identity and local launch fields. Resource pruning removes unreferenced local-only blobs while retaining references from unknown extension metadata. Imports union the destination pool with incoming resources so local launch data and future references survive. Retained inline launch bookmark/icon data is pooled when the destination gains a resource envelope.

Requests and files are bounded to 8 MiB, require strict unique-key JSON, and reject unsupported envelopes. File-backed CLI import accepts a bounded regular descriptor; symlinks and FIFOs are refused. Export is read-only and returns a source revision plus the SHA-256 of exact JSON UTF-8 bytes. The CLI verifies that digest before showing or writing the file. Rust authorities capture the exact state file and check it against the requested snapshot. The transitional native authority uses its authenticated in-memory snapshot rather than opening a competing Rust state file.

Owned authorities accept primary/landscape, with no portrait mirror created. Legacy authorities preserve stored orientation-slot semantics. Classic imports to owned profiles use retained native preparation to create all eight states; banked imports use the resource-aware owned codec directly. Legacy native numeric DTOs support i64 and binary64, including native 17-digit float spellings. Higher-precision extensions and banked files require explicit owned migration; they are refused on legacy authority instead of being rounded or flattened.

## Transactions and local MCP

`export_customization` accepts `profileID` and optional `variant`. It is read-only and works without configuration-write opt-in. `import_customization` accepts `profileID`, `artifactJSON`, `invocationID`, `expectedConfigurationRevision`, and optional `variant`. Write opt-in is checked before socket access. Owned writes use scoped raw drafts and the normal release barrier, revision/CAS, atomic persistence, and durable replay receipt. Committed replay works without the native helper. An identical import creates no new configuration revision. Stale requests fail without mutation.

The generic native draft operation remains available for classic legacy import. Owned import must go through the scoped high-level adapter; it cannot install a display projection through the legacy bridge route. No automatic migration, phone-format activation, skin-runtime deletion, or desktop-target retirement is introduced by this batch.

See [verification evidence](feature-removal-evidence/customization-interchange/README.md), [the cutover gates](cli-mcp-cutover.md), and the frozen native legacy request/response under `Host/fixtures/customization-interchange/v1`.
