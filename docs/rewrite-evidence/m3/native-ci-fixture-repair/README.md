# Native CI fixture repair

The 9cdb65f source passes Linux and both native/mobile provider jobs, including iOS 55 native + 47 simulator library tests, real entitled Keychain, both SDK builds and Android Keystore. macOS hit code22/InvalidInput while setting read timeout after peer stop, before testing observable closure. Windows lost two control replies while the independent receiver correctly expired its unrenewed configuration-owner lease.

A controlled 200ms native-reply delay reproduces Windows's TransportRejected and records lease_expired. The same loop passes after valid transport scenarios use a separate owner heartbeat. The owner-loss test still omits that guard and proves input cannot renew ownership. Re-enrollment now maintains a real native held sampler and requires native_input_released, so neither owner nor input expiry can substitute for revocation. A new 900ms slow-setup regression exceeds the unchanged 750ms lease while the actual owner renews it. All private owner requests remain serialized and generation-bound.

The macOS socket-option failure did not reproduce across 30 local iterations; its exact hosted failure is retained. The test now requires actual EOF/reset via a bounded worker result instead of configuring an already-stopped socket. Server worker/socket bounds and the closure assertion remain enforced.

The isolated CI-only source passes 79 native unit + nine actual-process tests and all-target Clippy. Controlled-delay revocation/re-enrollment loops pass. Temporary diagnostic probes are absent from source. These two files contain only tests/fixtures, and the public snapshot excludes unfinished UDP production code. Publication is authorized by the user's standing instruction to continue CI without repeated approvals.
