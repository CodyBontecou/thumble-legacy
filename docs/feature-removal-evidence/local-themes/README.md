# Local theme verification

`result.json` records actual private-home CLI/MCP acceptance using an input-disabled headless receiver. Both presets run on legacy and explicit owned authorities. Assertions retain exact geometry, IDs, labels, primary/part outputs, all binding sidecars, active/default identities, saved banks and unrelated profiles. Committed retries run with the native helper parked. Invalid presets and owned portrait requests do not write authority. MCP edits remain private until save and committed save replay returns the original receipt.

`rust-gate.log.gz` records formatting, clippy, 627 passed tests/3 historical native-dependent ignored and all-target checks. `native-tests.log.gz` records 45 passing CLI backend tests. `stack-gate.log.gz` records 47 passing constrained-stack cases, macOS/iOS Debug builds and unchanged frame budgets. The schema-v8 theme request planner/validation has its own Rust regression.

Run `python3 scripts/verify-local-themes.py --help` for the required binary arguments. The manifest hashes the tested source and built artifacts. These checks do not exercise physical haptics, network latency or a live WoW client, and do not activate migration or production format offers.
