# Authority-backed design inspection

Native CLI `asset list/show`, `layout validate` and `layout preview` now capture the selected profile at the exact catalog revision through the local authority. They no longer read the desktop preference store. Captures cannot create/resume a draft. Owned profiles use the explicit resource-aware reader; exact source bytes are retained separately from the disposable native view.

`layout preview --native` renders existing native views without a desktop window. The default remains a diagnostic diagram. `--appearance light|dark` and `--action-set base|left-trigger|right-trigger|both-triggers` select owned paint, with missing/duplicate option values rejected. Owned portrait previews and portrait canvas overrides refuse without an artifact. Legacy default/base and explicit variant selection remain available.

Evidence:

- Actual native CLI, bridge and receiver run in a private temporary account home, with the bridge's canonical path/ownership checks intact. Asset list/show retains fixture bytes. Layout validation, diagnostic PNGs and eight native appearance states work with the receiver stopped and running. Every corresponding PNG is byte-identical across the two modes.
- Authority bytes, including precise future metadata, remain unchanged at revision 41. No edit drafts are created. Missing assets and malformed appearance option invocations fail; owned portrait requests produce no file.
- 48 focused native tests passed, including three new inspection cases covering exact capture retention, eight-state/resource decoding, stale revision/draft refusal, unsupported formats, missing resources and legacy reads.
- 43 constrained-stack cases passed, including a new 512 KiB projection case. The inspection result occupies 249 inline bytes with existing budgets unchanged. macOS/iOS Debug and standalone CLI builds passed.
- Full Rust verification passed 612 tests, zero failed and three existing native-dependent ignored, plus formatting/clippy/target checks, frozen fixtures, capability parity and the reduced direct-persistence allowlist.

`verification.json` pins source/native-input, binary, log and artifact hashes. `previews/` contains all actual render outputs. Both held-trigger light/dark native artifacts were visually inspected. The structural fixture intentionally uses an opaque binary resource; these renders prove the native rendering route and paint-state selection, not image-asset fidelity or production design quality.

Reproduce with the selected native test suites in `nativeTests.txt`, `scripts/verify-stack-safety.sh`, `scripts/verify-rust-host.sh`, then `python3 scripts/verify-authority-design-reads.py --cli /absolute/path/thumble --cli-bridge /absolute/path/thumble-cli-bridge --host /absolute/path/thumble-host --output-dir /private/tmp/preview-output`.

The isolated source snapshot excludes concurrent WoW, Android and gamepad work. File-backed asset mutation, full owned design-workspace begin/apply, native MCP asset parity, live phone-owned snapshot/editor integration, automatic migration, and skin/portrait/desktop runtime removal remain unfinished. Public phone format offers remain `[4]`. No user authority was migrated or modified.
