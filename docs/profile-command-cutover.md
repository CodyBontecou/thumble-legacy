# Profile command cutover

Profile creation, application attachment, issue-specific repair and inspection use the headless configuration authority. Normal configuration and runtime commands do not elect the desktop GUI through its IPC endpoint or defaults. Explicit schema-3 and schema-4 stores remain supported; no command silently initializes an absent authority.

## Creation

`thumble profile create NAME --from UUID --no-select --default` clones the source's controls, bindings, groups, haptics, artwork and all eight appearance states. New profiles exclude the source's local application association; the separate duplicate operation retains its existing semantics. `--blank` produces an empty canvas. With neither source nor blank, creation restores the standard layout and bindings within one draft/commit. `--from`, `--template` and `--blank` are mutually exclusive. An invocation ID makes retries deterministic and committed replay does not need the native helper.

MCP uses `edit_configuration_draft` with `profile.create`, including `sourceProfileID`, `newProfileID`, `name`, `select` and `makeDefault`. Private draft creation/edits leave authority bytes unchanged until save. Owned copies transplant exact raw source data rather than re-encoding unknown extensions. A legacy profile with numbers beyond the native DTO's supported precision must migrate to owned configuration before copying; copying refuses before the native transform rather than rounding the extension.

## Local applications

`profile attach-app --profile UUID --path /Applications/Example.app` resolves local application metadata and submits `profile.application-set`. `profile detach-app --profile UUID` clears it. MCP accepts an attachment with `displayName` and at least one of an absolute `filePath` or `bundleIdentifier`, or null to clear. Repeated attachment/detachment preserves profile data and configuration revision; the authority may record a new no-op receipt. Same-application renames retain bookmarks and opaque launch metadata. Replacing or clearing an association removes its obsolete launch metadata and prunes only resources no longer referenced anywhere in the profile.

`profile launch` and MCP `launch_profile_application` require the running headless receiver. MCP requires `--allow-input`. The receiver validates pooled bookmark data on its launch worker, outside the input mutex, and retains the resolve/release-input/open ordering. Failure to validate/resolve never opens an application. Tool responses expose correlated profile/launch IDs and phase, without application paths or bookmark data.

## Inspection and repair

`profile show --profile UUID --json` correlates profile catalog flags/bindings and the native snapshot at one configuration revision. Its local `thumble.profile-inspection` version-1 envelope contains the exact profile JSON; it is a local inspection response, not a portable export. Portable interchange commands keep their existing contract.

`layout fix ISSUE-CODE --profile UUID` and MCP `customization.fix` with target `{ "kind": "issue", "code": "small-control" }` apply the report's suggested repair for matching issues. Unknown codes refuse. Locked controls and unrelated appearance/output fields remain protected. Existing all and named-repair targets remain available.

## Explicit legacy recovery

`profile show --legacy-desktop --profile UUID --json` is a read-only migration snapshot from an already running legacy owner holding its lease. Its envelope declares `origin: legacy-desktop-migration`. It never starts the GUI or writes/commits into it. This native DTO projection retains local application associations but is not an exact byte backup of preference storage and does not perform authority or trust migration. Normal commands refuse a legacy-owned lease without contacting desktop IPC. Runtime status, launch and release do not invoke the old GUI/defaults fallback.

Desktop retirement remains gated on remaining legacy asset metadata owners, migration/device acceptance and clean signed tools-only packaging. Appearance editing, design tools/previews, haptics, phone editing and WoW import/refresh remain retained functionality. Codec/process checks do not replace physical haptics, live WoW, app activation, or comparable input-latency measurements.
