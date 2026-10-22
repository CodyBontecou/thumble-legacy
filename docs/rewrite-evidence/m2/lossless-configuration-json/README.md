# Lossless configuration source foundation

The retained v5 owned reader keeps original data authoritative and allows full-width numeric transport, while portable artifacts use a separate RFC 8785 integrity mode. New @thumble/contracts/json preserves raw value spans and exact number tokens, supports immutable scoped edits and a separately named canonical source representation. It does not activate a document format, import, migration or runtime configuration.

Limits: 8 MiB UTF-8 source, depth 64, 131072 value nodes, pointer length 4096 code units/bytes. Full revalidation follows edits. Invalid scalar Unicode/duplicate decoded keys/grammar/BOM/non-JSON whitespace fail. Exact safe integer projections avoid IEEE-754 coercion of fractional or out-of-range tokens. Canonical output streams chunks rather than copying the full document at each ancestor.

Verification: 80 portable tests, zero failures, standalone packaging one test/40 assertions, formatter/type/boundary/generated checks and actionlint pass. Codec tests include exact unknown-subtree retention after profile-name edits, full-width and enormous exponents, pointer/prototype names, all byte-range string escapes, UTF-8 size/depth/node limits and an 8 MiB string. No retained Swift/shared model changed; retained stack-safety/cutover acceptance remains open.

The new mobile workflow filters native and mobile-boundary source, generated/schema bindings, shared workspace metadata, fixtures and tooling, rather than all Rewrite files. Its own workflow change triggers a complete native run once; later portable-only edits avoid cancelling independent native SDK/provider jobs. The existing headless matrix still tests TypeScript on all three hosts.

Standing user authorization covers explicit source/docs CI publication without renewed approval requests. Private baseline/history, credentials, builds and this evidence remain excluded. SQLite authority/typed owned model, operation receipts, assets, CLI/MCP configuration tools and launch gates remain unfinished.
