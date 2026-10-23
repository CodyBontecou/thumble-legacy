# Recording MCP capability boundary

`--allow-recorded-input` exposes exactly three tools: `receiver_activate`,
`receiver_activate_saved`, and `receiver_sample`. Previously those callbacks
used the normal local dispatch ports without verifying the selected service's
backend. In particular, saved activation shares the explicit CLI physical path,
so its recording-only MCP description was not an enforced capability boundary.

Each callback now captures one endpoint, requests strict identity-bound status,
and requires `backend:recording` and `inputEnabled:false` before sending its
operation to that same endpoint. A physical backend returns the fixed error
`recording_receiver_required`; no activation or sample request is sent. A failed
status check also prevents dispatch. `receiver_release` remains available without
this precondition so an agent can request emergency neutralization. Explicit CLI
physical launch/activation paths are unchanged.

## Owner and concurrency assumptions

`NativeReceiver` captures the launch backend in its constructor and rejects every
native reply reporting a different backend. The service holds that same native
receiver object for its lifetime; it cannot switch recording to physical mode.
Changing launch options requires a restart. Therefore this check need not reserve
the input queue between read and dispatch; ordinary generation/revision CAS and
all existing input deadlines still apply.

The guard never looks up an endpoint again after status. Local client status and
mutation decoders check the captured service instance, service PID and native PID;
requests carry its private capability token. A replacement service rotates its
instance/token and must not receive a fallback command after the captured owner
fails. This is an immutable-owner assumption, not a claim that a status snapshot
reserves an activation generation. Same-user malicious replacement of the native
program itself is outside the existing local capability model.

## Verification

Actual stdio MCP and the official SDK client exercise three scoped TCP service
doubles without constructing a physical receiver or OS device. Tests prove all
three tools send only status to physical metadata, recording activation/saved
activation/sample retain dispatch, release still dispatches to physical metadata,
and replacing the endpoint file between status and command cannot retarget the
operation. The old unguarded callbacks fail this regression; restored guarded
callbacks pass. Existing independent Node clients for both MCP eras and real
recording CLI/MCP label-author parity also pass: 10 tests, 87 assertions across
three files. Root TypeScript checking passes.

The complete `mcp.ts` leaf retains the independently reviewed output-doctor
wording correction about compiled runtime wiring versus authorization. Compose
this leaf after that doctor overlay; it intentionally includes that single
metadata overlap. No service, native input protocol, desktop route, physical
backend or signing behavior changed.
