# Owned asset transaction evidence

The isolated source at the base commit in `verification.json` adds typed `asset.import`, `asset.set` and `asset.remove` transactions, native CLI adaptation and the opt-in local MCP `edit_profile_asset` tool. Source hashes identify the tested files rather than concurrent work in the shared checkout.

- Rust workspace gate: formatting, clippy, all-target checks, parity/retirement/frozen fixtures, 621 passing tests and three native-dependent ignored tests.
- Native CLI backend: 42 passing tests; maximum-asset encode/strict scan on the existing constrained thread is included in the 44-test stack gate. Both macOS and iOS Debug builds pass with unchanged budgets.
- Actual CLI and MCP processes operate in one private, securely owned account. Import/set/remove, portable export/import, failed/stale/changed requests, referenced-asset refusal and receipt replay after restarting both processes pass. Asset data is pooled across all eight saved states.
- A plain profile without a bank uses the retained local native helper. Missing-helper refusal leaves authority bytes untouched; retry succeeds after restoring it. Committed replay then succeeds while the helper is unavailable. All original canonical values, including explicit haptic parameters, geometry, outputs and opaque precise paint metadata, survive.
- The sixteen native PNGs compare before/after the plain-profile import/removal. Light/dark × Base/LT/RT/LT+RT are byte-identical. The fixture has no trigger-specific visual differences; existing banked fixtures cover distinct states. These are renderer checks, not physical haptic or network latency measurements.

Reproduce with `scripts/verify-rust-host.sh`, the native backend tests, `scripts/verify-stack-safety.sh`, and `scripts/verify-owned-assets.py --cli PATH --bridge PATH --cli-bridge PATH --host PATH --mcp PATH --output-dir DIRECTORY`. The bridge argument is the retained `ThumbleBridge` executable; no desktop GUI is launched. Native build/test invocations and private paths are preserved in the logs.

Automatic migration, shipping phone activation, image/style application, full design-workspace apply, active skin/portrait deletion and desktop release retirement remain unfinished. No deployed service was stopped by this batch.
