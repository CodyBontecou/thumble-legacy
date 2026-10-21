# Keep enrollment outside the input safety process

The service delegates pairing windows and enrollment sockets to a separate native owner using bounded private stdio, while the existing receiver keeps its independent monotonic input lease. Four fixed network workers and a bounded socket queue absorb enrollment waits without putting TLS, identity storage or database work on the safety loop. The extra native process preserves responsive local lifecycle commands and independent neutralization when the service dies.

Both roles ship in the existing native helper binary. Listening requires explicit persistent identity setup and manual addresses; default startup retains local-only windows. TypeScript owns agent-facing validation/routing and does not retain a competing active window authority. No GUI/desktop IPC is required, and real input remains disabled until the separate session/signing/gameplay gates pass. See [the CLI and lifecycle contract](../rewrite-headless-pairing.md).
