# Bind phone authentication to one native control connection

Control version 3 proves the persisted phone credential directly over pinned native TLS and creates an opaque native authorization bound to that exact server channel, host pin and credential epoch. A public connection identifier is a routing reference, never a transferable credential; each operation rechecks durable trust and bounded expiry. Version 2 remains enrollment-only so older phone clients cannot interpret authentication as a completed enrollment or input grant. See [the protocol](../rewrite-authenticated-control-v3.md).
