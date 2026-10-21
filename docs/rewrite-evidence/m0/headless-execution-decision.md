# Headless execution decision — 2026-10-21

The user clarified: “i dont have a windows test machine, this work will be done entirely headless, relying on github action runners for windows-related tests and eventual beta testers”.

Continue implementation without requiring a local Windows computer or connected physical phone. Use GitHub-hosted Windows runners for build, unit, contract, loopback, packaging and other automation that can actually run headlessly. Preserve physical gameplay, signing/driver installation, latency, haptics and optical/live delivery as explicit beta/release acceptance gates. An absent controller on a hosted runner is an observation, not a virtual-gamepad pass.

This supersedes the original M0/M1 restriction on starting M2 implementation while physical acceptance is pending. It does not mark M0/M1 complete, certify the eight host/phone/game combinations, permit publication, or remove Windows/required-live-sync launch scope. Production input remains disabled in new development seams until its protocol, native backend and signing gates pass.
