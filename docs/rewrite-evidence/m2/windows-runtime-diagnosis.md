# Windows Bun runtime failure — 2026-10-21

Runtime: Bun 1.4.3 (`c6da4a4d3`), Windows hosted runner x64, OS 10.26100. Native receiver builds, format, Clippy, unit tests and TypeScript typechecks passed before the source integration test crashed in Bun. Linux/macOS passed the real service/package seam.

The initial [run 39646838389](https://github.com/CodyBontecou/Thumble/actions/runs/39646838389) segfaulted in `bun run test:ci`. Credential-free phase markers showed startup, endpoint protection, status, activation, stale revision rejection, sample/replay and repair progressing before failure at varying points. One attempt hung and was cancelled. No runtime payloads, endpoint tokens or signed download URLs were recorded.

## Reduction and experiments

- Plain repeated loopback request/reply and socket destruction passed on Windows.
- Plain loopback mixed with repeated Bun child-process pipe capture also passed ([run 39649197997](https://github.com/CodyBontecou/Thumble/actions/runs/39649197997), dedicated runtime-probe job). Native process execution alone was not sufficient to reproduce the crash.
- A native-free synthetic loopback server using the actual typed local client reproduced the crash on the second success/rejection cycle ([run 39649558213](https://github.com/CodyBontecou/Thumble/actions/runs/39649558213), runtime-probe job). Command: `bun test tests/client.test.ts`. The version used `await expect(request(...)).rejects.toThrow(...)`.
- Ranked predictions were: socket timeout teardown; Bun asynchronous rejection matcher; destruction during a socket data callback. Replacing only the client's per-socket timeout with an independent cleared JavaScript timer did not fix the reproduction ([run 39649855289](https://github.com/CodyBontecou/Thumble/actions/runs/39649855289)). That experiment was reverted.
- With the original production client restored, replacing only the asynchronous matcher with explicit rejection capture passed all 30 success/rejection cycles ([run 39650129701](https://github.com/CodyBontecou/Thumble/actions/runs/39650129701), runtime-probe job). This isolates a Bun test-matcher interaction with socket-originated rejections; it does not establish the upstream C++/Zig defect's precise root cause.

## Retained workaround and regression

`Rewrite/tests/helpers/rejection.ts` awaits the outcome, then asserts both `Error` identity and the exact error code. A successfully resolved promise produces `null` and fails those assertions. All five service fault assertions retain their rejection expectations through this helper. `tests/client.test.ts` retains the 30-cycle native-free regression using real sockets and the real client. Production transport, timeouts, authentication, native lease logic and Windows coverage are unchanged.

Temporary phase logs, plain-socket/child-process probes and the diagnostic CI job were removed. Test and package steps now have two-minute deadlines and the matrix job has a fifteen-minute deadline so runtime hangs fail visibly. The final full-matrix result is recorded in the adjacent receiver-seam evidence, separately from these diagnosis runs. Windows gameplay, driver installation and signing remain physical beta/release gates.

## Independent Windows safety-owner lifetime

After the matcher workaround, the full [run 39650478627](https://github.com/CodyBontecou/Thumble/actions/runs/39650478627) reached the service-death fault and failed its native disconnect/neutralization assertion. All other source tests passed. In [run 39651006652](https://github.com/CodyBontecou/Thumble/actions/runs/39651006652), a credential-free liveness/audit probe after termination showed both the service and receiver dead, with the last recorded report still held and no disconnect/process-exit neutral report.

Ranked explanations were a shared Windows process/job lifetime; descendant targeting by Bun's kill implementation; or native pipe failure bypassing cleanup. The first experiment gives the native child `detached: true` on Windows while preserving private stdio, normal shutdown monitoring and all existing safety assertions. Bun documents detached children as able to outlive their parent: [Bun detached process reference](https://bun.sh/reference/bun/Spawn/BaseOptions/detached). The full [run 39651775999](https://github.com/CodyBontecou/Thumble/actions/runs/39651775999) passed on Windows, macOS and Linux. Windows passed the unchanged service-death/disconnect/neutralization/restart test and the standalone package test. This confirms the detached native lifetime addresses the observed fault; it does not identify the upstream process/job implementation in detail. No acceptance assertion or deadline was relaxed.
