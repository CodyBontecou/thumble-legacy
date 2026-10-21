use super::*;
use crate::wow_live::{self, Observer, SyncAction};
use base64::{engine::general_purpose::STANDARD, Engine};

impl SharedRuntime {
    pub(super) fn wow_sync_control(&self, action: SyncAction) -> ControlResponse {
        if action == SyncAction::RequestPermission {
            let result = ConfigurationBridge::discover()
                .map_err(|e| e.to_string())
                .and_then(|bridge| wow_live::capture(&bridge, "request-permission"));
            match result {
                Ok(reply) => {
                    let mut status = self.wow_sync.lock().expect("WoW sync mutex poisoned");
                    status.phase = reply.status;
                }
                Err(error) => return ControlResponse::error(error),
            }
        }
        let mut status = self.wow_sync.lock().expect("WoW sync mutex poisoned");
        match action {
            SyncAction::Start => {
                if !self.configuration_write_enabled {
                    return ControlResponse::coded_error(
                        "configuration_writes_disabled",
                        "Start the receiver with --allow-config-write before enabling WoW sync",
                    );
                }
                if !self.uses_owned_configuration() {
                    return ControlResponse::coded_error("wow_profile_format_upgrade_required",
                        "WoW live trigger-bank icons require a migrated format-5 profile authority. Automatic migration is not enabled in this build.");
                }
                if !ConfigurationBridge::is_available() {
                    return ControlResponse::coded_error("native_helper_unavailable", "Install the matching native thumble-bridge helper before enabling WoW sync");
                }
                if !status.enabled {
                    status.enabled = true;
                    status.generation = status.generation.wrapping_add(1);
                    status.phase = "waiting-for-game".into();
                    status.last_error = None;
                }
            }
            SyncAction::Stop => {
                status.enabled = false;
                status.generation = status.generation.wrapping_add(1);
                status.phase = "disabled".into();
                status.last_error = None;
            }
            _ => (),
        }
        let mut response = ControlResponse::success();
        response.wow_live = Some(status.clone());
        response
    }

    fn wow_sync_tick(&self, observer: &mut Observer, generation: u64) -> Result<(), String> {
        let bridge = ConfigurationBridge::discover().map_err(|e| e.to_string())?;
        let capture = wow_live::capture(&bridge, "capture")?;
        {
            let mut status = self.wow_sync.lock().expect("WoW sync mutex poisoned");
            if !status.enabled || status.generation != generation {
                return Ok(());
            }
            status.phase = capture.status.clone();
        }
        if capture.status != "frame" {
            return Ok(());
        }
        let frame = STANDARD
            .decode(capture.frame_base64.ok_or("Missing WoW frame")?)
            .map_err(|_| "Invalid WoW frame encoding")?;
        let atlas = STANDARD
            .decode(capture.atlas_png_base64.ok_or("Missing WoW atlas")?)
            .map_err(|_| "Invalid WoW atlas encoding")?;
        let now = self.started_at.elapsed_millis().max(0) as u64;
        observer.observe(
            capture.window_id.ok_or("Missing WoW source")?,
            &frame,
            &atlas,
            now,
        )?;
        {
            let mut status = self.wow_sync.lock().expect("WoW sync mutex poisoned");
            if status.enabled && status.generation == generation && status.last_error.is_none() {
                status.phase = if !observer.is_live(now) {
                    "waiting-for-live-data"
                } else if status.profile_id.is_some() {
                    "synced"
                } else {
                    "assembling"
                }
                .into();
            }
        }
        let Some(update) = observer.pending(now).cloned() else {
            return Ok(());
        };
        let state = self
            .inner
            .lock()
            .expect("runtime mutex poisoned")
            .core
            .persistent_state()
            .clone();
        let summary = self
            .owned_begin(state.configuration_revision)
            .map_err(|e| e.to_string())?;
        let store = crate::owned_drafts::OwnedDraftStore::new(&self.paths);
        let result = (|| -> Result<Option<u64>, String> {
            use sha2::{Digest, Sha256};
            let draft = store
                .get(
                    &summary.draft_id,
                    Some(summary.draft_revision),
                    unix_millis(),
                )
                .map_err(|e| e.to_string())?;
            let plan = wow_live::prepare_profile(&bridge, &update, draft.document())?;
            let candidate = plan.merge(draft.document(), update.select)?;
            // Digest the actual typed sync intent and complete prepared bytes.
            let descriptor = format!("{:x}", Sha256::digest(serde_json::to_vec(&serde_json::json!({
                "type":"wow.sync-profile", "profileID":update.profile_id, "select":update.select,
                "sourceSHA256":format!("{:x}", Sha256::digest(draft.document().original_bytes())),
                "resultSHA256":format!("{:x}", Sha256::digest(candidate.original_bytes())),
            })).map_err(|e| e.to_string())?));
            let edited = store
                .edit_with_descriptor(
                    &summary.draft_id,
                    summary.draft_revision,
                    &Uuid::new_v4().to_string(),
                    &descriptor,
                    unix_millis(),
                    |source| {
                        let changed = source.original_bytes() != candidate.original_bytes();
                        Ok((
                            candidate,
                            crate::draft_operation::ConfigurationOperationOutcome {
                                changed,
                                changed_paths: if changed {
                                    vec![
                                        "/profiles".into(),
                                        "/activeProfileID".into(),
                                        "/profileKeyBindings".into(),
                                        "/profileOutputBindings".into(),
                                        "/keyBindings".into(),
                                        "/outputBindings".into(),
                                    ]
                                } else {
                                    vec![]
                                },
                            },
                        ))
                    },
                )
                .map_err(|e| e.to_string())?;
            let status = self.wow_sync.lock().expect("WoW sync mutex poisoned");
            if !status.enabled
                || status.generation != generation
                || observer
                    .pending(self.started_at.elapsed_millis().max(0) as u64)
                    .is_none()
            {
                return Ok(None);
            }
            // Owned save supplies CAS, raw-source preservation, held-input release
            // barriers, atomic persistence and peer broadcasts.
            let commit_started = self.started_at.elapsed_millis().max(0) as u64;
            self.owned_save(crate::owned_drafts::OwnedCommitRequest {
                draft_id: &summary.draft_id,
                draft_revision: edited.draft.draft_revision,
                expected: state.configuration_revision,
                commit_id: &Uuid::new_v4().to_string(),
                request_digest: Some(&descriptor),
            })
            .map_err(|_| "Profile authority rejected WoW update; retrying against current state")?;
            Ok(Some(commit_started))
        })();
        // Remove only this private draft on stale data, helper failure or CAS
        // conflict. The observer retains unacknowledged intent for the next tick.
        if let Ok(draft) = store.get(&summary.draft_id, None, unix_millis()) {
            let _ = store.discard(&summary.draft_id, draft.draft_revision, unix_millis());
        }
        let Some(commit_started) = result? else {
            return Ok(());
        };
        let mut status = self.wow_sync.lock().expect("WoW sync mutex poisoned");
        // Acknowledgment happens after commit; a failed CAS never loses the intent.
        observer.acknowledge(update.receipt, commit_started);
        status.phase = "synced".into();
        status.profile_id = Some(update.profile_id);
        status.character_name = Some(update.snapshot.character.name);
        status.last_error = None;
        Ok(())
    }
}

pub(super) fn run(shared: Arc<SharedRuntime>, shutdown: watch::Receiver<bool>) {
    let mut observer = Observer::default();
    let mut generation = 0;
    while !*shutdown.borrow() {
        let status = shared
            .wow_sync
            .lock()
            .expect("WoW sync mutex poisoned")
            .clone();
        if status.generation != generation {
            generation = status.generation;
            observer = Observer::default();
        }
        if status.enabled {
            if let Err(error) = shared.wow_sync_tick(&mut observer, generation) {
                let mut current = shared.wow_sync.lock().expect("WoW sync mutex poisoned");
                if current.enabled && current.generation == generation {
                    current.phase = "retrying".into();
                    current.last_error = Some(error.chars().take(256).collect());
                }
            }
        }
        std::thread::sleep(Duration::from_millis(if status.enabled { 75 } else { 250 }));
    }
}
