# Lossless owned-configuration edit result

`v1/input.json` is the existing version-5 specimen with two unknown integers changed to 18446744073709551617 and an unknown precise decimal added. Those values deliberately cannot roundtrip through ordinary floating-point projections. `v1/edits.json` requests seven edits: a name change and matching canonical/mirror geometry, icon and haptic changes. `v1/destination.json` is the real output of `OwnedConfigurationDocument::with_edits`, not a normalized native export.

The Rust integration test checks the resulting eight appearance states and unchanged outputs, compares the full output against this fixture, and leaves the input unchanged. Apple and Android readers independently resolve the destination and assert geometry, icon and haptic parameters across all eight states. Original resource bytes and large numeric spellings remain intact. `manifest.json` pins input/request/output hashes, the prior specimen hash and the Rust generator source.

To deliberately regenerate after reviewing a writer change:

```sh
THUMBLE_REGENERATE_OWNED_EDIT_FIXTURE=1 cargo +1.88.0 test --manifest-path Host/Cargo.toml -p thumble-core --test owned_configuration lossless_geometry_icon_and_haptic_edits_match_shared_destination
```

Update the manifest only after reviewing the output and rerunning the independent peer checks. This is an in-memory edit fixture; it does not prove authority revision checks, disk atomicity, offline reconciliation, live delivery, physical haptics or migrated user data.
