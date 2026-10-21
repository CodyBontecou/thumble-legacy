# Native mobile M2 engineering slice

This slice adds real Swift and Kotlin native engines and SDK touch surfaces, a bounded portable configuration/inspection interface, generated control-map schema, Swift package and Android library builds, and separate native CI. It does not complete full M2 acceptance or the full rewrite.

## Verified locally

- **53 TypeScript tests passed**, one compiled-package test intentionally skipped in this phase. Strict host/portable typechecks, generated-contract drift, dependency boundaries, frozen Bun install and formatting passed.
- **7 Swift test functions passed**, including **15 parameterized native scenarios**. These cover 102 shared touch event expectations, with six additional test functions for fixture count, 34 retained codec vectors, invalid maps, touch bounds, transport failure, terminal disposal and inline-size budgets.
- **21 Android tests passed**, including the same 15 scenarios and 34 retained codec vectors, plus bounds, immutable map ownership, native transport failure/exception, terminal disposal and wrong-thread mutation. Android SDK 35 compiled the actual `View`, built the AAR, and passed lint.
- All three Swift modules (generated contracts, input core, UIKit surface) compiled as static libraries/modules against **iPhone and Simulator SDK 26.5**, targeting arm64 iOS 17. These are SDK compile artifacts, not signed installable apps or physical-phone delivery evidence.

The canonical source fixtures are `Host/fixtures/gamepad/v2.json` and `Rewrite/packages/fixtures/mobile-input-v1.json`. Swift test resources are prepared copies in ignored build inputs. The TypeScript and native languages check identical semantic report bytes. Native touch state and sampling stay outside React; the portable bridge has configuration, practice, release and snapshot methods only. React Native app/bridge wiring and authenticated native networking remain M3 work.

Source ownership is explicit: the configuration service remains profile authority; these modules have no editable catalog or disk persistence. Activation rejects malformed/stale maps before releasing current state, and requires matching local receipts. The native engines own touch IDs, circular sticks, simultaneous holds, reference counts, native sampling, practice, lifecycle cancellation and disposal. A retired sender requires a fresh generation, so queued touches cannot replay after reconnect.

## Incomplete verification

The required retained `./scripts/verify-stack-safety.sh` was started. Its Xcode compiler probe stopped making progress before application compilation; the generic native iOS Xcode package build hit the same wait. Sampling the actual compiler showed a blocking `write` in Clang's verbose probe output; direct compiler invocation and the independent SDK module builds succeeded. Only these task-owned stalled build processes were terminated, and both tracked build handles returned 143. This is **not a stack-safety pass** or a successful generic Xcode package build. Their logs are retained here. The existing retained stack tests and budgets were not modified. New native module tests assert the generated state and control-map inline-size budgets independently.

The hosted native workflow passed Swift tests, direct SDK compilation and generic iPhone/Simulator Xcode package builds on Xcode 26.3 / Swift 6.2.4. This closes the package integration check despite the local Xcode 26.5 probe stall. Local SDK compilation and hosted package builds do not substitute for signing, physical touch/haptics, JS-stall timing or gameplay acceptance. The retained stack-safety script remains incomplete.

## Approved public CI publication

The user explicitly approved the scoped 41-file source delta after automatic approval review rejected a broader source snapshot. The approved delta was published as `92820df9808bbf7116c479639978181d70c25234` on the existing public CI branch. [ci-publication-candidate.json](ci-publication-candidate.json) preserves the exact approved source hashes and approval receipt. Unrelated baseline/history, credentials, build products and local evidence logs were excluded.

[Portable run 39659489404](https://github.com/CodyBontecou/Thumble/actions/runs/39659489404) passed on Windows, macOS and Linux, including source/native tests, strict checks and the separate standalone package test. [Native run 39659489513](https://github.com/CodyBontecou/Thumble/actions/runs/39659489513) passed all iOS checks. Android stopped during setup because the setup action requested the removed legacy `tools` package, before compilation. A scoped workflow fix requests only `platform-tools`; follow-up commit `994ce1a21a428aa04c3a2cb9ea5f1800373af06c` passed Android native tests, AAR build and lint, plus every iOS check, in [run 39659933913](https://github.com/CodyBontecou/Thumble/actions/runs/39659933913). [github-actions.json](github-actions.json) records jobs and artifact digests without ephemeral download URLs.

Adjacent logs, Android JUnit, SDK artifact hashes and [verification.json](verification.json) bind the current local verification. The root implementation manifest binds source and evidence hashes. Windows, iOS/Android and required optical live sync remain launch scope under the user's headless-development policy.
