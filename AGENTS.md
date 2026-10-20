# AGENTS.md

## CLI/MCP product boundary

The target product is a local CLI/MCP configured through users' agents, phone controller/editor UI, and a headless local input receiver. Migrate retained functionality from the desktop application to CLI/MCP before retiring its UI and IPC. Configuration, pairing, receiver lifecycle, appearance/design tools, previews, haptics, and WoW import/refresh must work without the Thumble desktop GUI installed or running. Preserve required native input/signing helpers and shared rendering code. See `docs/cli-mcp-cutover.md` when changing desktop dependencies, authority routing, setup, or release packaging.

## Swift stack safety

Changes to shared models, Codable paths, profile synchronization, skins, or app startup must preserve the inline-size budgets and constrained-thread coverage in `Tests/StackSafetyRegressionTests.swift`. Keep large aggregate fields behind immutable/COW storage, split heavy work into phases, and run `./scripts/verify-stack-safety.sh` before considering the change complete. Do not treat a larger thread stack as the primary fix for application-owned stack growth. See `docs/stack-safety.md`.

## Handcrafted skin authoring

When creating, redesigning, critiquing, or preparing a community Thumble skin, load and follow the project `thumble-skin-author` skill. Use the separate art-director, designer, visual-critic, and QA agents in sequence. Preserve editable SVG source and native-renderer review evidence. Never publish a skin or mark human approval without the user's explicit approval of the exact contact sheet and package hash.
