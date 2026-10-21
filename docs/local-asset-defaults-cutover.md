# Asset metadata and desktop defaults cutover

The native CLI no longer owns a preference-backed profile store. Asset import/set/rename/remove use local headless transactions on both legacy schema-3 and owned schema-4 authorities. Asset list/show and saved-profile preview read revision-correlated authority snapshots. There is no preference or native GUI IPC fallback after refusal, stopped runtime or missing authority.

Legacy asset transactions update each declared primary/landscape/portrait customization library without creating absent slots. Owned edits retain their pooled resources and all eight saved appearances. Both paths share input limits, stable import identities, referenced-removal refusal, revision checks and durable retries. Metadata updates preserve bytes, hashes, geometry, outputs, haptics, groups, styles, local application associations and unrelated fields. Removing a referenced asset refuses rather than stripping its uses. Unsupported private metadata deletion can refuse under the existing exact-source merge contract.

## Explicit defaults bootstrap

Receiver startup never invokes `defaults`, reads the desktop plist, or imports desktop preferences automatically. An existing headless state remains authoritative. A fresh receiver can start with no desktop installation and ignores unreadable or malformed desktop preferences.

Before starting a fresh authority, review an exported or saved, user-owned plist and its exact SHA256, then run:

```sh
thumble-host configuration import-desktop-defaults \
  --source-file /absolute/preferences.plist \
  --source-sha256 REVIEWED_LOWERCASE_SHA256
```

Local MCP exposes `import_desktop_defaults` with `sourceFile` and `sourceSHA256`; start MCP with `--allow-config-write`. This works without a running receiver. Custom/relay channels cannot bootstrap a different local authority.

The importer validates bounded supported declarations before acquiring the receiver lease, refuses a running/legacy owner, and never overwrites an existing authority. It preserves supported server identity, pairing, profile selection/defaults, customization, application associations and binding maps. It keeps `desktop-defaults-source.plist` as an exact private backup (0600) before writing state, and stores a checksum receipt. A successful retry returns the receipt without rewriting state. Later edits invalidate replay; missing/corrupt receipts or partial bootstrap artifacts refuse safely rather than replacing data. The source plist is never modified. Tool responses contain counts/checksums, not credentials.

This first step produces a supported legacy authority. Use the separate reviewed `configuration migration plan/apply` workflow to materialize owned appearances and prepare phone admission. Obsolete declarations and numeric extensions beyond the legacy DTO's supported precision refuse before state creation; retain the exact original source and use the existing exact JSON preparation/recovery tools. This does not enable automatic migration or silently replace an existing pairing identity.

## Retained design tooling

Local compile, validate, quality, package-artifact preview and reviewed design workspace tooling remain available. Artifact previews use canonical artboards without a catalog; explicit `--profile` reads the headless authority. Remaining package library/runtime operations require the headless receiver and give an explicit start instruction when it is unavailable. They cannot elect the desktop GUI or write preferences. Required native render/input/signing helpers remain packaged.

Appearance editing, design tools, haptics, on-phone editing and the WoW addon remain retained features. Physical phone haptics/editor behavior, live WoW, real app activation, comparable latency and a clean signed tools-only install remain separate acceptance gates.
