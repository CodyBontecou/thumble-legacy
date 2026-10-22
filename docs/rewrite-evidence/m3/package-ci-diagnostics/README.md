# Standalone package CI diagnosis

Standing user authorization covers CI source publication without repeated approval requests. Preserve private baseline/history, credentials, build products and local evidence exclusions.

Public base a0203d60838f676cbe6ba29eee8d373518f59b9b passed Windows/Linux host and all three mobile jobs. macOS failed on first identity initialization with exit 1, empty stderr, after 9828ms. The former test discarded the structured error. Ranked hypotheses: native helper startup deadline, native launch error, native identity-store rejection. No cause established yet.

This two-file source capsule reports only a bounded public error code plus action, exit and elapsed time; successful replies and caller options may contain secrets and are never included. The CLI distinguishes ETIMEDOUT without changing its existing five-second deadline. Local verify: 74 passed; standalone package: 1 passed, 40 assertions. Native UDP work is excluded from this diagnostic publication.

Frozen capsule: /private/tmp/thumble-package-diagnostics.json, 16903 bytes, SHA256 e4c5e480e0967a51e695c71163191e8e01a97ecd0b90ebb2272fc5c0c098fc41.
