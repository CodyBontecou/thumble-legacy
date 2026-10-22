# Native input datagram evidence

Host-only control-v5 implementation, physical output disabled. Protocol and limits: ../../../../docs/rewrite-native-input-control-v5.md.

Local checks: 79 Rust unit + 12 actual-process tests; Clippy all targets; 55 Swift tests including native-process TLS witnesses; Android tests/build/lint; portable verify (74 tests), standalone packaging (1 test, 40 assertions). Retained application stack-safety validation remains unresolved from the previously stalled macro compiler probe; no retained shared models changed here.

UDP probes use repeated identical sealed packets until reliable status confirms native acceptance. Initial one-shot probe assumptions failed under intentional mutation-fence drops; the corrected assertions model lossy UDP rather than weaken production admission. Revocation and listener shutdown require native release acknowledgments, not TTL inference. No physical device output or GUI used.

CI publication is authorized by the user's standing instruction. Publish only explicit source/docs files on the current public parent; exclude this evidence, private baseline/history, credentials and build products.
