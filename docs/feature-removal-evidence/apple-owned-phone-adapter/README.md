# Apple owned-phone wire and journal adapter

The shared Apple adapter now reads the receiver's frozen scalar intent/result contract and keeps exact original intent/recovery bytes in a bounded, deduplicated offline journal. Successful acknowledgments must match the same authenticated authority, profile, edit ID and base revision. Ordinary snapshots, mismatched receipts and conflicts cannot retire work. Historical receipts change only the journal, never the current snapshot.

Verification in an isolated HEAD checkout:

- 47 focused native tests passed, including nine new adapter cases, retained profile format/import/export tests, legacy Rust/Swift wire fixtures, orientation and binding presentation behavior.
- The shared 14-case Rust intent matrix and all five frozen wire frames are read by Apple tests. Full unsigned revisions and lexical integer token kinds are checked. Unknown/mixed input payloads fail closed without changing ordinary opaque snapshot metadata.
- File-backed save/restart preserves exact intent and owned-document bytes, deduplicates recovery, validates profile/digest ownership and refuses duplicate IDs, mismatched receipts, damaged recovery and corrupt-file replacement.
- 42 constrained-stack tests passed, including a new 512 KiB thread case exercising the intent, wire, journal, recovery document and 128-control bound. Existing budgets remain unchanged: intent/journal 8 bytes each, result 88 bytes and ControllerMessage 2872 bytes.
- macOS and iOS Debug builds passed with static network/input stack gates. Frozen fixture and local MCP/CLI parity checks passed.

`verification.json` records source, fixture, binary and normalized/raw log hashes. `native-source-hashes.json` identifies the native source snapshot represented by the staged commit. Concurrent WoW, Android and gamepad changes remain outside this verified batch.

Reproduce the selected native test suites in `nativeTests.txt`, then run `scripts/verify-stack-safety.sh`. The test suite reads existing frozen receiver fixtures; this batch does not regenerate them.

Live owned snapshot transfer, iOS editor diff/application, Android integration/verification, legacy pending-edit conversion and phone recovery UI remain unfinished. Shipping offers remain `[4]`; no authority or phone migration is activated and no user data is modified. Physical haptics, latency and live WoW acceptance still require their own evidence. This adapter is a prerequisite for the skin/portrait/desktop retirement, not completion of those removals.
