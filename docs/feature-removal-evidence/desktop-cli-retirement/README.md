# Desktop CLI UI command retirement

Removed implementations for desktop app open/launch/quit, onboarding reset/replay, editor-view preference mutation and window screenshot/capture. Retired aliases now return explicit receiver/preview guidance before configuration or UI work. App termination, preference-reset, window-enumeration and screenshot helper code is deleted; game-profile launch associations and ordinary native design rendering remain.

- Actual built `thumble` and `thumbconsole` binaries each refused 11 retired invocations, returned no stdout, preserved a private working-directory fixture, and retained help/setup-guide output for receiver, pairing and preview tools.
- The source guard runs before invoking removed commands and in the Rust gate. It verifies unsafe desktop implementations are absent and only the three retained app setup/link operations remain in the capability ledger. The five removed MCP records were planned entries, not implemented tools.
- Complete Rust gate passed: 612 tests, zero failed, three existing native-dependent ignored; formatting, clippy, target, frozen-fixture and CLI/MCP parity checks passed.
- Six native setup-guide tests and 42 constrained-stack tests passed. Standalone CLI/compatibility alias and macOS/iOS Debug builds succeeded; all existing stack budgets are unchanged.
- README, website docs, agent guidance and the bundled generator skill use saved-layout preview artifacts and headless setup rather than a desktop window capture.

`verification.json` records exact tested source, native-input, binary and log hashes. Logs have trailing whitespace normalized with raw hashes retained. The isolated native input set matches the staged commit, excluding concurrent WoW, Android and gamepad work in the shared checkout.

Reproduce with `scripts/verify-rust-host.sh`, native setup tests and `scripts/verify-stack-safety.sh`, then `python3 scripts/verify-retired-desktop-cli.py --cli /absolute/path/thumble` (and the compatibility alias).

The fixture check proves the retained test directory is unchanged; it does not claim a system-wide filesystem audit. Source inspection proves the retired handlers no longer perform their prior UI/preference work. No real desktop app was launched, terminated or captured by these checks, and no profile authority was migrated. The desktop app target, remaining GUI IPC/legacy authority fallback, full phone-owned snapshot/editor integration, release cutover and deployed-service retirement remain unfinished.
