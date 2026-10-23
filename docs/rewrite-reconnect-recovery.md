# Saved-profile reconnect recovery

A saved edit advances the configuration revision/source hash without replacing an
activated input pin. Saved-map reads correctly return
`saved_map_reactivation_required` for that stale activation. Previously, both app
wrappers reduced the result to generic unavailability, and React suggested
reopening pairing. Pairing cannot refresh an input pin.

The app now preserves this one typed SDK failure as exact admission status
`reactivation-required`. Swift forwards errors from both existing pair/reconnect
catches; Android classifies the exact SDK exception/code on its worker before
posting a Boolean to the main thread. Closed guards and cleanup precede terminal
status as before. Other errors retain generic handling. No raw error text,
endpoint or credential enters the bridge.

The existing Expo event carries version/status only. Its strict TypeScript
decoder admits the fixed status and refuses extra fields. React clears installed
map/session/presentation, disables input, and displays: “Your saved profile
changed. Ask your agent to reactivate it on your Mac, then close and reconnect.
Your phone is still paired.” This performs no activation, trust forgetting,
enrollment or input retry. Release, scope, counter, grant, ownership and deadline
rules are unchanged.

## Recording Android acceptance

Use the default release-derived nativeQa entry on an owned emulator, never a
physical phone. Build/install both APKs with the existing native QA plugin.
Start `tooling/paired-editor-app-host.ts` with
`THUMBLE_REWRITE_PAIRED_EDITOR_UI=1` and `THUMBLE_EDITOR_RECEIVER` naming the built
receiver. Reverse only its safe emitted fixture/TLS ports. Its existing
`EditorPairedAppTest` enrollment phase establishes actual target-UID production
Keystore trust and the descriptor. This fixture is a prerequisite separate from
the production recovery change; no production hook or test map is installed.

From `Rewrite`, with the same environment, the narrow opt-in local companion
`tooling/reconnect-recovery-operator.ts` uses production local APIs. It requires
exactly one owned private fixture directory, the one-profile synthetic catalog,
and recording output with `inputEnabled:false`. It emits fixed safe summaries.

1. Run `bun tooling/reconnect-recovery-operator.ts activate`. It explicitly
   activates the current profile with saved hash/revision and native revision CAS.
2. Run the app test below with `thumbleExpectedAdmission ready`.
3. After the connection closes, explicitly `activate` revision 1 again. Run the
   operator's `edit` action, which commits synthetic control label
   `ReconnectLabel` at expected revision 1. It verifies the old native pin stays
   active at revision 1 while the catalog advances to revision 2.
4. Run the app test expecting `reactivation-required`. It requires exact recovery
   guidance, no connected message, and no generic pairing instruction.
5. Explicitly `activate` current revision 2, then run the app test expecting
   `ready` again. The recovered product UI displays the new exact label.

```sh
adb -s OWNED_EMULATOR shell am instrument -w \
  -e class com.thumble.rewrite.qa.ReconnectRecoveryAppTest \
  -e thumbleReconnectRecovery 1 -e thumbleExpectedAdmission ready \
  com.codybontecou.thumble.rewrite.qa.test/androidx.test.runner.AndroidJUnitRunner
```

Without explicit opt-in the test skips. It drives existing `home-connect` and
`android:id/button3` resource IDs, the actual native factory/SDK, and real React
text. Require `OK (1 test)`; adb can exit zero despite JUnit failure. Optional
`-e thumbleObserveAdmission 1` holds the asserted screen ten seconds for Argent
screenshots. Finish with the paired fixture cleanup phase, which requires a
production vault read after forget to return null. Stop the actual fixture and
observe exit, remove only its reverses, and stop scoped Argent services.

## Evidence and limits

The owned API35 arm64 emulator passed baseline ready (16.178s), stale guidance
(15.103s), and recovered ready (15.61s), including observation windows. Argent
screenshots show the full 18-control ready layout, exact warning with no controls,
and recovered layout with `ReconnectLabel`. Actual Keystore enrollment/cleanup
also passed. No OS input or physical phone was used.

This gate commits its edit through the local sole-owner CAS API. The separate
paired editor gate proves phone UI edits, exact original receipts and duplicate
replay; those claims are not repeated here.

Freshly described normalized Argent Reconnect taps produced unavailability,
while resource-ID automation of the same default route passed. Exact AX frame,
tap coordinates and result are preserved as unresolved interaction evidence;
this is not a passing interactive Argent gesture replay or an established
cause. A describe call concurrent with UiDevice polling also failed to parse its
dump. Independent screenshots succeeded. No projection or timing rule changed.

Validation: contract tests 8/176 assertions; actual React tests 29/190 assertions;
root/mobile TypeScript checks; Android release build 599 tasks and focused
instrumentation build; actual Expo iOS product Release compilation including the
final pair catch; retained `./scripts/verify-stack-safety.sh` passed unchanged
budgets. The copied iOS test scheme lacked unrelated test resources, so the
actual product scheme supplied compilation evidence. iOS UI/fresh-pair stale
admission, physical output, physical-device behavior and gameplay remain outside
this gate. Synthetic vault/descriptor and private fixture state were cleaned;
the owned emulator remains booted for later work.
