/** Explicit recording-only companion to the opt-in paired editor app fixture. */
import assert from "node:assert/strict";
import { readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import {
  readEndpoint,
  request,
  requestConfiguration,
  requestSavedRuntime,
} from "@thumble/client/local";
assert(
  process.env.THUMBLE_REWRITE_PAIRED_EDITOR_UI === "1",
  "reconnect_opt_in",
);
const phase = process.argv[2];
assert(phase === "activate" || phase === "edit", "reconnect_phase");
const root = resolve(import.meta.dir, "..");
const bases = join(root, "build/paired-editor-private");
const dirs = readdirSync(bases);
assert(
  dirs.length === 1 && /^\d+-\d+$/.test(dirs[0]!),
  "reconnect_exact_owned_fixture",
);
const receiver = process.env.THUMBLE_EDITOR_RECEIVER;
assert(receiver, "reconnect_receiver_required");
const endpoint = readEndpoint(join(bases, dirs[0]!, "runtime"), receiver);
const state = await request(endpoint, { command: "status" });
assert(
  state.receiver.snapshot.backend === "recording" &&
    state.receiver.snapshot.inputEnabled === false,
  "reconnect_recording_only",
);
const catalog = await requestConfiguration(endpoint, {
  command: "configuration-list",
  expectedRevision: null,
  offset: 0,
});
assert(
  catalog.kind === "catalog" &&
    catalog.profiles.length === 1 &&
    catalog.activeProfileId === "e1000000-0000-0000-0000-000000000001",
  "reconnect_synthetic_catalog",
);
if (phase === "activate") {
  assert(
    catalog.revision === "1" || catalog.revision === "2",
    "reconnect_revision",
  );
  const activated = await requestSavedRuntime(endpoint, {
    command: "activate-saved",
    expectedReceiverRevision: state.receiver.snapshot.revision,
    expectedConfigurationRevision: catalog.revision,
    expectedSourceHash: catalog.contentHash!,
    profileId: catalog.activeProfileId!,
  });
  assert(
    activated.snapshot.active &&
      activated.snapshot.backend === "recording" &&
      !activated.snapshot.inputEnabled &&
      activated.savedBinding?.configurationRevision === catalog.revision,
    "reconnect_activation",
  );
  console.log(
    JSON.stringify({
      version: 1,
      phase,
      revision: catalog.revision,
      recordingOnly: true,
      active: true,
    }),
  );
} else {
  assert(
    catalog.revision === "1" && state.receiver.snapshot.active,
    "reconnect_old_pin_required",
  );
  const receipt = await requestConfiguration(endpoint, {
    command: "configuration-set-control-label",
    operationId: "reconnect-recovery-label",
    expectedRevision: "1",
    profileId: catalog.activeProfileId!,
    controlId: "ce000000-0000-0000-0000-000000000001",
    label: "ReconnectLabel",
  });
  assert(
    receipt.kind === "committed" && receipt.receipt.resultingRevision === "2",
    "reconnect_label_commit",
  );
  const pin = await requestSavedRuntime(endpoint, {
    command: "saved-binding-status",
  });
  assert(
    pin.savedBinding?.configurationRevision === "1" && pin.snapshot.active,
    "reconnect_stale_pin_retained",
  );
  console.log(
    JSON.stringify({
      version: 1,
      phase,
      revision: "2",
      recordingOnly: true,
      oldPinRevision: "1",
    }),
  );
}
