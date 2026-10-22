# Native control renewal

Local source `3666484`, public source `fd2059c8447e9100c1fed229b231b2ac3186adba`, adds explicit control version 6, bounded trust reauthorization, receiver key staging and reliable packet-zero commitment. A single registry entry covers both native references through the handoff and retires both on revocation or uncertain admission.

Local and isolated public projection checks pass 97 Rust unit cases and 14 native process cases, with zero skips, strict Clippy and formatting. The rebuilt standalone package passes its checkout-free CLI/service/receiver/MCP test (46 Bun assertions, including the official Node SDK interoperability scenarios). New tests use real pinned TLS, SQLite and private native receiver IPC with a controlled wall clock. They preserve held reports through two renewals beyond initial authorization, switch UDP routes/progress only after reliable commitment, and release on revocation before/after commitment, early/expired requests or legacy-protocol misuse. The exact source-only leaf diff, modes and Git blob hashes were verified against the sole public parent before a non-force leased branch update. Private baseline/history and this evidence were excluded.

Hosted checks for this snapshot are pending. Earlier receiver staging source `ca7be33f` now passes all three host jobs (including Windows), all three native SDK/provider jobs and Android app; its UIKit app remains running at the recorded observation. Earlier phone-app source `b862c619` has all eight jobs green, including UIKit.

Automatic phone renewal, sender/transport handoff, app opt-in and a real-duration headless phone hold remain open. This evidence does not establish physical output, gameplay, signing, latency, or mandatory optical live sync acceptance.
