# Receiver authority migration

The local CLI and MCP can now prepare and apply an exact, backed-up migration from state schema 3 / profile format 4 to state schema 4 / profile format 5. This is an explicit offline operation; automatic startup migration remains disabled. Normal retained phone startup now offers format 5 only after every legacy journal loads and is empty. Before migrating an installed authority, reconcile retained phone edits with their legacy owner, reconnect and inspect the authenticated `core.clientSupportsOwnedProfiles` status. This handshake is not a claim about disconnected phones or changes made afterward; preserve journals if new work blocks reconnection.

Stop the specific receiver through its normal lifecycle command first. Migration holds the same authority lease as the receiver and refuses an active owner. It never stops another process or initializes a missing catalog. Set the normal `THUMBLE_HOST_STATE_DIR` and `THUMBLE_HOST_CONTROL_SOCKET` for the intended receiver when using an isolated installation.

```sh
thumble-host configuration migration plan > /absolute/private/migration-plan.json
thumble-host configuration migration apply \
  --plan-file /absolute/private/migration-plan.json --allow-config-write
```

For each referenced skin, supply `--package PROFILE_UUID=/absolute/original.archive` on both calls. Mappings must exactly match the referenced profiles. The bounded reader rejects symlinks, special files, foreign ownership and changing inputs. The converter materializes all eight appearance states and uses the raw local merger, preserving launch associations, exact numeric metadata, profile identities, active/default selection, trust and bindings. Original alternate orientations remain in the exact source backup; review reasons explicitly identify portrait arrangement and archived alternate layouts. A failed conversion produces no partial state or backup transaction.

MCP `migrate_profile_authority` uses the same implementation. Planning accepts `packages: [{profileId, archivePath}]`. Apply additionally requires `apply: true`, `planJson` containing the complete unchanged plan returned by planning, and MCP `--allow-config-write`. Only the exact configured local Unix authority is eligible. Relay/custom transports and control-socket overrides that do not match the configured filesystem authority are refused rather than falling back to a different catalog.

The plan contains checksums, revisions, preparation time and per-profile layout/package reviews. It contains no receiver state, pairing tokens or asset bytes. Apply checks the original state bytes and revision, reruns native conversion at the reviewed timestamp, and compares the complete resulting receipt. Changed packages, native output, source state or plan fields require a fresh plan.

Before installing state, the transaction durably saves the original state and original referenced archives under private `owned-migrations-v1/<source SHA256>/`, alongside the destination and phase receipt. Repeating the exact plan can finish a recorded installation or return its receipt. It cannot overwrite a subsequent edit. Partial backups are validated before reuse. If pending legacy work blocks the phone after cutover, stop the same receiver and use the original plan for a deliberate rollback:

```sh
thumble-host configuration migration rollback \
  --plan-file /absolute/private/migration-plan.json --allow-config-write
```

MCP `rollback_profile_authority` accepts the original complete `planJson` and requires `--allow-config-write`. It uses the same local-only authority lease. Rollback restores the original profiles, selection, local metadata and trust while advancing the revision beyond the migration; it does not reset the revision or discard phone journals. It restores absent original archives from the private backup, without needing their old source paths or a desktop library. A different installed archive at the same package identity is a conflict and is kept intact.

Rollback refuses any authority write after the migration destination, including changes to trust. Exact retries can resume a recorded rollback, but cannot acknowledge or overwrite edits made after that rollback. Corrupt journals or rollback bytes require review. This is recovery for the unchanged cutover destination, not a general undo of later owned edits. If a newer write prevents rollback, preserve both the phone journal and receiver backup for explicit reconciliation.

`verify-authority-migration.py --host PATH --bridge PATH --mcp PATH` uses private fixture authorities and the real converter. It exercises CLI/MCP planning and apply across five source layouts, write opt-in, active-owner refusal, tampered plans, exact backups, all eight appearance states, local launch metadata, raw decimal retention, credential-free output, replay, later-edit conflicts, exact-plan rollback, original archive restoration and rollback replay protection. It does not migrate the installed lab, reconcile actual phone journals or establish live WoW acceptance.
