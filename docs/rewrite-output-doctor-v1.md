# Offline output doctor

`thumble-rewrite output doctor --json` and the read-only MCP tool `output_doctor` inspect the selected native receiver executable without a running service, desktop GUI, runtime directory, identity, pairing or saved configuration. The native command is `thumble-rewrite-receiver output-capabilities`; it accepts no state directory or activation arguments. CLI `--receiver PATH` selects an explicit local helper; packaged use resolves the sibling helper. The MCP tool takes an empty object and is available without input/configuration-write opt-ins.

## What the result establishes

The strict version-1 `output-capabilities` result always reports `backend:"recording"`, `physicalOutputEnabled:false`; these describe the offline inspection/default backend, not an active receiver. `runtimeIntegrated` reports whether the selected executable contains the macOS output runtime path: true only for a macOS `output-macos` build, false for default and inspection-only builds. The doctor does not construct an output owner, create/register a device, load a Windows SDK, inspect/install a driver, prompt for permissions or send an input report.

The platform is `macos`, `windows`, `linux` or `unsupported`. Its macOS section separates `dormantAdapterCompiled` from `signedClaim`. Normal builds report `false` and `not-compiled` on macOS. The explicit native Cargo feature `output-doctor-macos` compiles the output-core macOS module and permits signed-claim inspection, while leaving `runtimeIntegrated:false`; it may report `present`, `absent` or `inspection-failed`. The `output-macos` feature includes that inspection capability and the runtime bridge, reporting `dormantAdapterCompiled:true` and `runtimeIntegrated:true`. The historical `dormantAdapterCompiled` field names compiled adapter availability, not whether the runtime is integrated or active. A true runtime flag requires macOS and a compiled adapter; it never establishes authorization or activation. Other platforms report `false` and `not-applicable`. Inconsistent combinations are rejected by the portable decoder.

Feature-enabled inspection calls only the existing macOS factory's `authorization` method, which reads the executing process's actual Boolean virtual-HID entitlement using SecTask. That method must remain inspection-only for future changes: no device creation, worker startup or permission prompt. A constructor-forbidden fake verifies that the doctor cannot reach `Factory::create`. A present signed claim does not establish matching provisioning, valid distribution authorization, successful IOHID creation or consumer recognition. `osDeviceAuthorization` therefore remains `not-inspected`, and `gameplayVerified` remains false in every result.

The Windows section is static candidate evidence, returned on every platform. It names the pinned HIDMaestro 2.0.3 mapping experiment, upstream commit `63cc6481f9e82fc8600bda8aaa65cb3271113a19` and profile `xbox-360-wired`. It reports `sdkLinked:false`, `backendAvailable:false`, `driver:"not-inspected"`. These values describe compiled integration scope, not a probe for driver absence. The separate mapping crate's tests are not a working Windows backend or gameplay acceptance.

## Bounds, errors and side effects

The CLI adapter starts one asynchronous native child per doctor call with a three-second deadline and independent 4 KiB stdout/stderr limits. The native reply is decoded as strict UTF-8 and duplicate-key-free JSON, then checked against an exact data-only contract. Extra fields, getters/coercion, misleading output claims and inconsistent platform/compile states are rejected. Raw stderr, paths, credentials and arbitrary provider details are not returned. Errors are bounded codes such as `output_doctor_unavailable`, `output_doctor_timeout`, `output_doctor_response_too_large` and `output_doctor_response_invalid`.

The doctor runs outside service authority and input heartbeat queues. It does not read the service endpoint or initialize runtime/trust/configuration state. No returned status can enable physical output. A missing or incompatible helper fails closed rather than falling back to desktop IPC or a different output owner.

## Build and verification scope

Default native builds do not link output-core or its optional macOS dependencies. The optional feature adds those packages to Cargo metadata/lock notices; notices cover every locked registry package, including optional/target/development dependencies. CI explicitly compiles/lints and tests the macOS inspection feature in addition to normal recording-only builds. Packaging uses the default feature set unless explicitly built with `--with-macos-output`. That option compiles the `output-macos` runtime; it does not sign, provision or activate it. Physical output additionally requires explicit service `--output macos-hid --allow-physical-output` and an appropriately authorized helper.

Six exact pinned objc2-family crate archives omit license files. Their supplemental notice records each published package's own VCS commit, repository, SPDX declaration and Cargo metadata/source hashes, retains its pinned upstream license declaration verbatim, and appends the standard SPDX MIT terms with literal placeholders clearly identified. It invents no project-specific copyright holder or year. The offline generator rejects changed versions, metadata, audited source or license text; this exception does not admit arbitrary missing-license packages.

```sh
thumble-rewrite output doctor --json
thumble-rewrite output doctor --receiver /absolute/path/to/helper --json

# Explicit source build for signed-claim inspection; no signing or device creation.
cargo +1.88.0 build --locked --manifest-path Rewrite/native/receiver/Cargo.toml --features output-doctor-macos
# Compile the runtime bridge as well; inspection still creates no device.
cargo +1.88.0 build --locked --manifest-path Rewrite/native/receiver/Cargo.toml --features output-macos
```

Tests cover strict portable admission, native argument rejection, offline native/CLI operation without state creation, both independent official MCP SDK eras, malicious/oversized/duplicate JSON output, sanitized child failures, deadlines and constructor-forbidden signed-claim inspection. Default, inspection-only and runtime-enabled builds are checked through native → CLI → both MCP SDK eras without starting a service. Compiled runtime availability is not signed-helper verification, OS authorization, active output, consumer compatibility or physical phone/gameplay acceptance. Windows driver integration remains separate work. This doctor completes no launch acceptance gate.

The compile-capability correction was verified headlessly on macOS with Cargo
1.88.0: default, `output-doctor-macos`, and `output-macos` builds reported
`runtimeIntegrated` false, false, and true respectively. The default build had no
compiled adapter; both optional builds reported the actual signed claim as
absent on the unsigned development executables. Every build kept physical output
disabled, OS authorization uninspected and gameplay unverified. Focused native
tests passed (2/4/4), including the constructor-forbidden inspector, and all five
offline CLI/contract/MCP tests passed independently for each binary (15 total).
Both official MCP SDK eras returned the selected binary's exact native result.
No service, HID owner/device, physical report, signing/install operation or
permission dialog was started for these checks. The portable typecheck passed;
full mobile-dependent type coverage remains part of the composed repository gate.
