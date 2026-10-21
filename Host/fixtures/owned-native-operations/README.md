# Owned native operation fixtures

Six exact native request/response packets cover geometry, per-control haptics, control creation/removal, appearance preference, and profile selection with no default. Inputs use the existing owned-document specimen. The nullable case changes only its default field without re-encoding opaque metadata; the deletion case begins with the native creation result so it exercises an actual custom control rather than built-in hiding.

`Tests/OwnedConfigurationDocumentTests.swift` regenerates packets into `build/OwnedOperationBridge` and compares exact bytes. `Host/crates/thumble-host/tests/owned_operation_bridge.rs` uses those native responses through the real bounded host adapter, validates operation ownership and bank reconciliation, merges into original source, and compares frozen destinations. Both packet correlation failures and output changes disguised as geometry are rejected. Android independently decodes all destinations and compares display preparation with native staging; package lookup throws.

The manifest pins generator source and every frozen output. The older projection-merge manifest's generator-file hash is refreshed because that file gained these tests; its original data files are unchanged. Run `python3 scripts/verify-owned-operation-bridge.py BUILT_BRIDGE` for actual executable acceptance, including mixed-command, portrait and legacy-document rejection.

The native protocol returns same-codec before/after views, never a replacement authority file. The caller retains raw source and owns CAS, backup, commit identity and runtime publication. These fixtures do not activate owned CLI/MCP transactions, phone offers or automatic migration.
