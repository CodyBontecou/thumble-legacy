# Native enrollment evidence

Local Rust coordinator tests execute actual pinned TLS and durable trust storage. Native window policy operations remain available during TLS reads. The shared synthetic fixture verifies TypeScript/native proof semantics. See `docs/rewrite-enrollment-v1.md` for exact protocol and integration boundaries.

This new source is local only and depends on the unpublished persistent identity and trust ledger. Earlier hosted phone TLS results do not cover it. Existing service CLI pairing windows are not yet forwarded to the native coordinator. Phone secure storage, network listener admission, safety-worker integration and physical acceptance remain pending. The retained Swift stack-safety compiler probe remains unverified.

The separately approved 24-file snapshot from 7d51154 is published as 9e2f643. All host and native jobs passed in Actions runs 39679838153 / 39679838124. The host run executes 55 Rust tests on Unix and 53 on Windows (two Unix-only filesystem tests), 68 TypeScript tests and standalone packaging. See `../enrollment/github-actions.json` for scope, job logs and artifact digests. New phone vault source remains excluded.
