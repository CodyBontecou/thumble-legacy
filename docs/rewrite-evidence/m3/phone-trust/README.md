# Native phone trust ledger evidence

Local isolated native store verification; see ADR 0005 for scope and remaining integration. The credential ledger requires persistent host identity from local commit d52b438. These logs do not establish hosted Windows support or physical power-loss behavior. No real credentials are retained here.

The approved phone TLS publication ac9cafe contains the frozen ten-file f15960a snapshot, excluding this ledger and persistent identity. Its hosted runs must be recorded separately.

The retained Swift stack-safety script remains unverified after its earlier compiler macro-probe stall; these Rust/package tests do not replace that gate. Production input, enrollment coordination, phone secure storage and session revocation remain disabled or unconnected.

The separately approved 24-file snapshot from 7d51154 is published as 9e2f643. All host and native jobs passed in Actions runs 39679838153 / 39679838124. The host run executes 55 Rust tests on Unix and 53 on Windows (two Unix-only filesystem tests), 68 TypeScript tests and standalone packaging. See `../enrollment/github-actions.json` for scope, job logs and artifact digests. New phone vault source remains excluded.
