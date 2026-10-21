# Profile-owned asset transactions

Explicit owned authorities support `asset.import`, `asset.set` and `asset.remove` through the typed CLI profile protocol and the opt-in local MCP `edit_profile_asset` tool. Normal startup and shipping phones still use the existing profile-format gate; this does not migrate or activate an authority.

Imports accept at most 2,500,000 bytes, bounded names/content types, an optional basename and one of background/icon/texture/reference. The native CLI reads a bounded regular descriptor without following symlinks or blocking on FIFOs. MCP accepts canonical Base64 content rather than a host path. The asset ID is UUIDv5 under the invocation UUID with label `asset.import:asset`. Exact original invocation/content/revision retries use durable commit receipts.

For a plain owned profile without an appearance bank, the first import uses the retained local native helper to materialize the existing appearance across all eight states. Only appearance fields are patched into the original profile; the authority schema, output bindings and profile identity remain unchanged. If preparation fails, nothing is staged or committed. This does not require the desktop GUI.

The document adapter patches only canonical/eight-state asset-library definitions, resource-table entries and the selected profile timestamp. New bytes are pooled once by SHA-256; every appearance state receives the same asset definition. Metadata changes update matching definitions across states, preserving opaque asset metadata and unrelated paint, controls, outputs, groups, haptics, launch associations and precise numbers. Asset writes do not resolve transient display paint.

Removal refuses known asset/image references in any saved state or extension. Removing an unused definition retains pooled bytes so unrelated or future resource references are not discarded. Normal portable export can prune unreferenced resources using the existing reference-aware rules. Metadata changes on a missing asset fail; unchanged changes are no-ops. Complete document/state bounds are validated before staging.

The normal raw owned draft, revision/CAS, input-release and atomic persistence path owns the commit. Prepared retries resume without adding another asset; changed invocation content and stale revisions fail. The native CLI keeps its prior guarded legacy-owner path only after an explicit owned-format refusal; other errors never authorize fallback. An absent authority is not initialized by a rejected typed asset request.

CLI examples:

```sh
thumble asset import ./art.png --name "Local art" --role icon --invocation-id UUID
thumble asset set ASSET_ID --name "New name" --role texture --invocation-id UUID
thumble asset remove ASSET_ID --invocation-id UUID
```

Local MCP requires `--allow-config-write` on MCP and the receiver, a canonical profile UUID, positive expected configuration revision and stable invocation UUID. Its `edit` is a tagged import/set/remove request; the tool returns transaction metadata and `assetID`, never imported bytes. Asset list/show remain native CLI inspection operations.

These operations manage the asset library. Remaining file-backed paint/style operations and complete owned design-workspace apply still need cutover; importing an asset alone does not attach it to a control.
