# Revisioned saved-profile selection v1

`configuration select-profiles` atomically changes the saved active and default
profile selectors. It does not publish a runtime configuration, change an input
session, enable receiver input, or establish activation compatibility. Existing
owned-v5 structural and activation acceptance gates remain separate.

The command requires **both** selectors explicitly. Read `configuration list`
first, preserve the selector you do not intend to change, and use that catalog's
revision as the compare-and-swap precondition. A null default is supported; the
active selector must name an existing profile.

```sh
thumble configuration select-profiles \
  --runtime-dir /absolute/runtime --receiver /absolute/thumble-rewrite-receiver \
  --operation-id selection-1 --expected-revision 7 \
  --active-profile-id aaaaaaaa-0000-0000-0000-000000000001 \
  --default-profile-id bbbbbbbb-0000-0000-0000-000000000002 --json

# Clear only the saved default while explicitly retaining the active selector.
thumble configuration select-profiles \
  --runtime-dir /absolute/runtime --receiver /absolute/thumble-rewrite-receiver \
  --operation-id selection-2 --expected-revision 8 \
  --active-profile-id aaaaaaaa-0000-0000-0000-000000000001 \
  --default-profile-id none --json
```

The equivalent MCP tool is `configuration_select_profiles`, available only when
the launching user supplies `--allow-configuration-write`. Its strict arguments
are `operationId`, `expectedRevision`, `activeProfileId`, and `defaultProfileId`.
Use JSON null to clear the default. Both modern and legacy MCP protocols use the
same contract, capability guard, service authority, and durable receipts.

IDs use case-insensitive UUID lookup. Command identity normalizes UUID case for
retry equivalence, while new source selectors reuse the selected profile's exact
stored ID spelling and escaping. An unchanged selector retains its existing
source value. Neither profile records nor their IDs/timestamps are rewritten.
Only the two known root selector values can change; opaque metadata, large
numeric lexemes, saved states, bindings, appearance data, external resource
references, and the authority resource manifest are retained. This works for
legacy inline saved documents and canonical authority envelopes without
materializing or rewriting resource files. JSON container whitespace may
normalize during the lossless source edit.

The sole configuration child derives the payload fingerprint from the scoped
command and commits through the existing SQLite authority. Named profiles must
exist before a new receipt can be written. Capability denial precedes reads or
writes. A stale expected revision returns a typed conflict; missing profile IDs
fail without a commit. Selecting an uninitialized authority cannot seed it.

A successful result is the existing `committed` reply with a durable operation
receipt, one resulting revision, content hash, and
`affectedEntities: ["configuration"]`, because the edit affects root selectors
rather than profile contents. Even unchanged selections persist one receipt and
revision. Selection never consults a wall clock or changes profile timestamps.

Retry the same operation ID and complete intent to recover the original receipt,
including after restart or intervening edits. Receipt replay precedes current
source decoding and CAS. Reusing an already committed ID with another revision,
active selector, or default selector is rejected. Import/export jobs retain
exclusive child IO; selection returns `configuration_owner_busy` while one of
those bounded background jobs runs. Native input leases and ordinary request
deadlines are unchanged.

Focused domain/application tests verify named-profile checks, unchanged source,
case-preserving ID selection, opaque data/resources, no clock access,
capability/CAS failures, no-op receipts, restart replay, and payload binding.
Independent Node MCP SDK probes race CLI and MCP submissions for one operation,
exercise default clearing and stale/missing/reused operations, and recover the
original receipt after service restart. The same probes run against copied
standalone binaries with a restricted product PATH and assert receiver input
stays disabled throughout.
