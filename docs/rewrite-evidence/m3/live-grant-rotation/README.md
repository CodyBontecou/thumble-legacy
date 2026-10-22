# Receiver live grant continuation — in progress

Local `628f0ef`, public `ca7be33f` adds the receiver/private native IPC primitive
needed for future continuous phone renewal. Seven exact source paths and blobs
were verified; the sole public parent is `b862c619`. No private baseline/history,
credentials, builds or evidence were published.

Only the exact live connection/session and configuration can stage one fresh
receiver-issued grant. Staging publishes nothing and extends no current deadline.
A valid next-key counter-zero full state commits the handoff, preserving held
buttons and both triggers without an intermediate neutral output. The old decoder
is then retired. This differs from ordinary new-source installation, whose neutral
barrier and reconnect budget remain unchanged.

Local and isolated public validation pass all 89 Rust unit cases and 14 process
cases, zero skips, and clippy. Standalone sibling-binary packaging passes one
case/46 Bun assertions plus all four Node SDK scenarios. Process tests use the
actual independent recording receiver and capability-authenticated native sockets.
They verify held-state continuity, old-key replay rejection, stale release and
peer cleanup fencing, pending cancellation, and disconnect before/after commit.

Safety cases cover malformed lengths, bad AEAD, incorrect first counter, foreign
scope, overlapping staging, abandoned response delivery, output failure, owner
lease/input TTL/grant expiry, rollback, explicit release and replacement. Pending
material has at most two seconds to commit. Eighty rotations keep constant staging
memory and do not consume ordinary reconnect history or extend the owner lease.

[Host CI](https://github.com/CodyBontecou/Thumble/actions/runs/39765297014),
[native SDK/provider CI](https://github.com/CodyBontecou/Thumble/actions/runs/39765296810)
and [bundled app CI](https://github.com/CodyBontecou/Thumble/actions/runs/39765296632)
are pending for this source. Prior phone app `b862c619` passes all three host and
all three SDK/provider jobs plus Android app; its UIKit app job remains in
progress at the last observation. The earlier surface `73aab76d` now passes all
eight jobs, including UIKit.

This does not implement phone authorization renewal, revocation-registry transfer,
native sender/route replacement or a negotiated phone renewal protocol. Current
apps retain their sixty-second limits. Physical output is disabled. No signing,
physical phone, gameplay, haptic, latency, migration, optical or launch acceptance
is claimed, and no existing GUI owner is retired.
