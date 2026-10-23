# Read-only final mobile artifact metadata audit

`Rewrite/tooling/check-mobile-store-artifact.py` inspects an existing `.xcarchive`
directory or Android App Bundle ZIP. It does not run build tools, applications,
signers, account clients, installers or upload commands. Explicit version/build
inputs follow the store profile's canonical review limits. It prints fixed check
names, content hashes and inventory counts; artifact paths, certificate contents,
provisioning payloads and arbitrary parser exceptions are not printed.

```sh
python3 Rewrite/tooling/check-mobile-store-artifact.py --platform ios \
  --artifact /absolute/private/Thumble.xcarchive --version 1.0.0 --build 4
python3 Rewrite/tooling/check-mobile-store-artifact.py --platform android \
  --artifact /absolute/private/Thumble.aab --version 1.0.0 --build 4
python3 -m unittest discover -s Rewrite/tooling -p test_mobile_store_artifact.py -v
```

These illustrative version inputs do not assign a release. `ok:true` means only
the implemented metadata checks passed. Every result reports
`signingStatus:unverified` and `distributionAccepted:false`. A signed file's
presence does not prove its signer, entitlement, provisioning or store eligibility.
The existing prebuild auditor remains useful before an artifact exists; this
command reads actual artifact contents rather than generated project settings.

## Checks and explicit limits

The iOS check requires exactly one main archived app and a matching archive
application path, retained production identifier, exact version/build, iPhone-only
family, declared minimum OS, nonempty camera/local-network purpose strings,
executable and icon-resource presence, no visible QA test bundles, and the app's
required privacy reasons. Every bundled privacy plist is parsed. Privacy values
are not a complete third-party manifest or store-label review. Icon resource
presence does not prove the curated icon's rendered appearance. Debug entitlement
and signature verification remain explicitly unverified.

The Android check supports a single base module and the bounded scalar XML subset
of [AAPT2's Resources.proto](https://github.com/aosp-mirror/platform_frameworks_base/blob/master/tools/aapt2/Resources.proto),
verified against the official schema on 2026-10-23. It uses no generated or
vendored protobuf code. Compiled scalar values take precedence over misleading
raw strings. Referenced or unsupported critical values fail closed. It verifies
production package/version, declared min/target SDK order, debuggable=false,
backup disabled, the exact audited permission set, optional camera, no manifest
instrumentation/obvious QA paths, and declared icon/dex presence. Target SDK
presence is not a claim of compliance with a current store deadline. Text XML,
extra feature manifests, ambiguous protobuf values and unsupported containers
are rejected instead of accepted by fallback.

Both platforms inventory notice-file presence and content hashes without claiming
complete dependency or license coverage. Runtime QA bytecode exclusion, complete
notices, privacy labels, icon identity, signing and functional/gameplay/live-sync
acceptance remain listed as unverified even after metadata passes. A container
with no nonempty bundled notice file fails the presence check.

Directory traversal uses no-follow directory/file descriptors, rejects symlinks,
hardlinks and special files, and checks files/directories for concurrent mutation.
ZIPs are never extracted. Absolute/traversing/backslash/NUL/duplicate or colliding
names, symlinks, encrypted entries, unsupported compression and ZIP64/multi-disk
are rejected. Limits are 20,000 entries, 32 directory/XML levels, 256MiB per file,
2GiB total expanded content, 2MiB per metadata file, 32MiB retained metadata,
512MiB compressed AAB and a 200:1 per-entry compression ratio. Hashing uses bounded
chunks. These conservative limits can reject a legitimate unsupported artifact;
that is an unresolved inspection result, not permission to bypass a check.

## Current evidence and remaining artifact gap

Eleven synthetic test cases cover passing metadata, wrong identities/builds/families,
empty QA directories, duplicate plist keys, privacy schema errors, extra permissions/instrumentation, compiled/raw conflicts,
critical references, malformed/duplicate/deep protobuf, unsafe ZIP/filesystem
entries and fixed CLI failures. Fixtures contain placeholder executable/dex bytes:
they test metadata policy, never binary execution or signing.

A private AAPT2 conversion of the current development Release APK successfully
parsed its real generated 10,299-byte protobuf manifest. It retained the development
identifier and version 0.0.0/build1. This is parser compatibility evidence, not a
final store AAB or archive. No current Thumble `.aab` or `.xcarchive` was available
in the inspected rewrite output/store proof directories. Final store artifacts
still need their first read-only audit; unrelated products' archives were not used.
