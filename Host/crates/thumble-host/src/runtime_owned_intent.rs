//! Reliable authenticated owned phone intent dispatch. Native staging is off the
//! input mutex; commit revalidates peer ownership and uses the normal release/CAS path.
use super::*;
use crate::owned_drafts::OwnedCommitRequest;
use crate::owned_phone_edit::{OwnedPhoneStage, OwnedPhoneStageError};
use thumble_protocol::{OwnedPhoneEdit, OwnedPhoneEditResult, OwnedPhoneEditStatus};
impl SharedRuntime {
    pub(super) fn try_handle_owned_phone_intent(
        &self,
        connection: ConnectionId,
        message: &ControllerMessage,
    ) -> Result<bool, String> {
        let owned = message.owned_phone_edit.is_some() || message.owned_phone_edit_result.is_some();
        if message.message_type != ControllerMessageType::OwnedPhoneEdit {
            if owned || message.message_type == ControllerMessageType::OwnedPhoneEditResult {
                return Err("unexpected owned phone edit payload".into());
            }
            return Ok(false);
        }
        if message.owned_phone_edit_result.is_some()
            || message.has_adoption_payload()
            || message.gamepad_customization.is_some()
            || message.gamepad_profiles.is_some()
            || message.skin_reference.is_some()
            || message.skin_packages.is_some()
        {
            return Err("mixed owned phone edit payload".into());
        }
        let intent = message
            .owned_phone_edit
            .as_deref()
            .ok_or("owned phone intent is required")?;
        {
            let mut inner = self.inner.lock().expect("runtime mutex poisoned");
            self.guard_input_liveness(&mut inner);
            if !inner.core.is_authenticated(connection) {
                self.handle_input_locked(&mut inner, connection, message.clone(), false)?;
                return Ok(true);
            }
            if !inner.core.accepts_owned_phone_edits(connection)
                || message.profile_format_version != Some(5)
            {
                drop(inner);
                self.reply_owned_phone_intent(
                    connection,
                    intent,
                    OwnedPhoneEditStatus::Rejected,
                    None,
                    false,
                    Some("owned_phone_format_required"),
                );
                return Ok(true);
            }
        }
        if intent.validate().is_err() {
            self.reply_owned_phone_intent(
                connection,
                intent,
                OwnedPhoneEditStatus::Rejected,
                None,
                false,
                Some("owned_phone_invalid_intent"),
            );
            return Ok(true);
        }
        let queued = {
            let jobs = self.skin_jobs.lock().expect("profile jobs poisoned");
            match jobs.as_ref() {
                Some(jobs) => jobs
                    .try_send((
                        connection,
                        skin_runtime::ProfileJob::OwnedIntent(Box::new(intent.clone())),
                    ))
                    .map_err(|error| match error {
                        std::sync::mpsc::TrySendError::Full(_) => "owned_phone_queue_busy",
                        std::sync::mpsc::TrySendError::Disconnected(_) => {
                            "owned_phone_worker_unavailable"
                        }
                    }),
                None => Err("owned_phone_worker_unavailable"),
            }
        };
        if let Err(code) = queued {
            self.reply_owned_phone_intent(
                connection,
                intent,
                OwnedPhoneEditStatus::Rejected,
                None,
                false,
                Some(code),
            );
        }
        Ok(true)
    }
    fn reply_owned_phone_intent(
        &self,
        connection: ConnectionId,
        intent: &OwnedPhoneEdit,
        status: OwnedPhoneEditStatus,
        committed: Option<u64>,
        changed: bool,
        error: Option<&'static str>,
    ) {
        let mut inner = self.inner.lock().expect("runtime mutex poisoned");
        if !inner.connections.contains_key(&connection) || !inner.core.is_authenticated(connection)
        {
            return;
        }
        let current = inner.core.persistent_state().configuration_revision;
        let mut reply =
            ControllerMessage::new(ControllerMessageType::OwnedPhoneEditResult, unix_millis());
        reply.profile_format_version = Some(5);
        reply.configuration_revision = Some(current);
        reply.owned_phone_edit_result = Some(Box::new(OwnedPhoneEditResult {
            schema_version: 1,
            edit_id: intent.edit_id,
            profile_id: intent.profile_id,
            base_configuration_revision: intent.base_configuration_revision,
            configuration_revision: current,
            committed_revision: committed,
            changed,
            status,
            error_code: error.map(str::to_owned),
        }));
        if self
            .execute_effects(
                &mut inner,
                vec![Effect::SendMessage {
                    connection_id: connection,
                    message: Box::new(reply),
                }],
            )
            .is_err()
        {
            self.log("owned phone edit receipt delivery deferred");
        }
    }
    pub(super) fn handle_owned_phone_intent(
        &self,
        connection: ConnectionId,
        intent: OwnedPhoneEdit,
    ) {
        {
            let inner = self.inner.lock().expect("runtime mutex poisoned");
            if !inner.connections.contains_key(&connection)
                || !inner.core.accepts_owned_phone_edits(connection)
            {
                return;
            }
        }
        let result = (|| {
            let source = self
                .owned_snapshot_source()
                .map_err(OwnedPhoneStageError::Draft)?;
            crate::owned_phone_edit::stage_local(&self.paths, &source, &intent, unix_millis())
        })();
        match result {
            Ok(OwnedPhoneStage::AlreadyCommitted(receipt)) => self.reply_owned_phone_intent(
                connection,
                &intent,
                OwnedPhoneEditStatus::Replayed,
                Some(receipt.configuration_revision),
                receipt.changed,
                None,
            ),
            Ok(OwnedPhoneStage::Prepared(prepared)) => {
                let result = self.owned_save_from_phone(
                    OwnedCommitRequest {
                        draft_id: &prepared.draft.draft_id,
                        draft_revision: prepared.draft.draft_revision,
                        expected: intent.base_configuration_revision,
                        commit_id: &prepared.commit_id.to_string(),
                        request_digest: Some(&prepared.request_digest),
                    },
                    connection,
                );
                match result {
                    Ok(saved) => self.reply_owned_phone_intent(
                        connection,
                        &intent,
                        if saved.idempotent_replay {
                            OwnedPhoneEditStatus::Replayed
                        } else {
                            OwnedPhoneEditStatus::Applied
                        },
                        Some(saved.configuration_revision),
                        saved.changed,
                        None,
                    ),
                    Err(error) => {
                        let (status, code) = match error {
                            SaveConfigurationError::Commit(ConfigurationCommitError::Draft(
                                DraftError::ConfigurationRevisionConflict { .. }
                                | DraftError::DraftIdConflict,
                            )) => (
                                OwnedPhoneEditStatus::Conflict,
                                "owned_phone_revision_conflict",
                            ),
                            SaveConfigurationError::ReleaseFailed(_) => {
                                (OwnedPhoneEditStatus::Rejected, "owned_phone_release_failed")
                            }
                            SaveConfigurationError::Disabled => (
                                OwnedPhoneEditStatus::Rejected,
                                "owned_phone_session_retired",
                            ),
                            _ => (OwnedPhoneEditStatus::Rejected, "owned_phone_save_failed"),
                        };
                        self.reply_owned_phone_intent(
                            connection,
                            &intent,
                            status,
                            None,
                            false,
                            Some(code),
                        );
                    }
                }
            }
            Err(error) => {
                let (status, code) = match error {
                    OwnedPhoneStageError::RevisionConflict { .. } => (
                        OwnedPhoneEditStatus::Conflict,
                        "owned_phone_revision_conflict",
                    ),
                    OwnedPhoneStageError::Draft(
                        DraftError::ConfigurationRevisionConflict { .. }
                        | DraftError::DraftIdConflict,
                    ) => (OwnedPhoneEditStatus::Conflict, "owned_phone_source_changed"),
                    OwnedPhoneStageError::IntentConflict => (
                        OwnedPhoneEditStatus::Rejected,
                        "owned_phone_intent_conflict",
                    ),
                    OwnedPhoneStageError::MissingProfile => (
                        OwnedPhoneEditStatus::Rejected,
                        "owned_phone_profile_missing",
                    ),
                    OwnedPhoneStageError::MissingControl => (
                        OwnedPhoneEditStatus::Rejected,
                        "owned_phone_control_missing",
                    ),
                    OwnedPhoneStageError::InvalidIntent => {
                        (OwnedPhoneEditStatus::Rejected, "owned_phone_invalid_intent")
                    }
                    OwnedPhoneStageError::Draft(DraftError::OperationIdConflict) => (
                        OwnedPhoneEditStatus::Rejected,
                        "owned_phone_intent_conflict",
                    ),
                    _ => (
                        OwnedPhoneEditStatus::Rejected,
                        "owned_phone_preparation_failed",
                    ),
                };
                self.reply_owned_phone_intent(connection, &intent, status, None, false, Some(code));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::owned_runtime::tests::{begin_edit, fixture, hold_key};
    use super::*;
    fn authenticate(shared: &SharedRuntime) -> mpsc::Receiver<Outbound> {
        let (sender, receiver) = mpsc::channel(64);
        shared.register_connection(7, sender);
        let mut inner = shared.inner.lock().unwrap();
        let mut hello = ControllerMessage::new(ControllerMessageType::Hello, 0);
        hello.pairing_code = Some("123456".into());
        hello.supported_profile_formats = Some(vec![5]);
        let RuntimeInner { core, tokens, .. } = &mut *inner;
        let effects = core
            .handle_message(7, hello, unix_millis(), tokens)
            .unwrap();
        shared.execute_effects(&mut inner, effects).unwrap();
        assert!(inner.core.accepts_owned_phone_edits(7));
        receiver
    }
    fn intent() -> OwnedPhoneEdit {
        OwnedPhoneEdit::decode_json(include_bytes!(
            "../../../fixtures/owned-phone-edits/v1/intent.json"
        ))
        .unwrap()
    }
    fn result(receiver: &mut mpsc::Receiver<Outbound>) -> OwnedPhoneEditResult {
        while let Ok(Outbound::Binary(bytes)) = receiver.try_recv() {
            let message = thumble_protocol::ControllerWireCodec::decode(&bytes).unwrap();
            if let Some(reply) = message.owned_phone_edit_result {
                return *reply;
            }
        }
        panic!("no edit receipt")
    }
    #[test]
    fn queue_backpressure_preserves_peer_and_returns_correlated_receipt_without_draft() {
        let (_temp, _lease, shared) = fixture();
        let mut receiver = authenticate(&shared);
        while receiver.try_recv().is_ok() {}
        let (sender, _jobs) = std::sync::mpsc::sync_channel(0);
        *shared.skin_jobs.lock().unwrap() = Some(sender);
        let request = intent();
        let mut message = ControllerMessage::new(ControllerMessageType::OwnedPhoneEdit, 0);
        message.profile_format_version = Some(5);
        message.owned_phone_edit = Some(Box::new(request.clone()));
        let before = storage::read_state_bytes(&shared.paths.state_file).unwrap();
        assert!(shared.try_handle_owned_phone_intent(7, &message).unwrap());
        let reply = result(&mut receiver);
        assert_eq!(reply.edit_id, request.edit_id);
        assert_eq!(reply.status, OwnedPhoneEditStatus::Rejected);
        assert_eq!(reply.error_code.as_deref(), Some("owned_phone_queue_busy"));
        assert_eq!(reply.configuration_revision, 41);
        assert!(shared.inner.lock().unwrap().core.is_authenticated(7));
        assert_eq!(
            storage::read_state_bytes(&shared.paths.state_file).unwrap(),
            before
        );
        assert!(!shared.paths.drafts_dir.exists());
    }
    #[test]
    fn paired_phone_commits_without_agent_write_opt_in_but_revoked_peer_cannot_commit() {
        let (_temp, _lease, mut shared) = fixture();
        let _receiver = authenticate(&shared);
        let draft = begin_edit(&shared, "Phone test");
        shared.configuration_write_enabled = false;
        let commit = Uuid::new_v4().to_string();
        let request = || OwnedCommitRequest {
            draft_id: &draft.draft_id,
            draft_revision: 2,
            expected: 41,
            commit_id: &commit,
            request_digest: None,
        };
        assert!(matches!(
            shared.owned_save(request()),
            Err(SaveConfigurationError::Disabled)
        ));
        {
            let mut inner = shared.inner.lock().unwrap();
            inner.core.disconnect(7);
        }
        let before = storage::read_state_bytes(&shared.paths.state_file).unwrap();
        assert!(matches!(
            shared.owned_save_from_phone(request(), 7),
            Err(SaveConfigurationError::Disabled)
        ));
        assert_eq!(
            storage::read_state_bytes(&shared.paths.state_file).unwrap(),
            before
        );
        let _receiver = authenticate(&shared);
        let saved = shared.owned_save_from_phone(request(), 7).unwrap();
        assert_eq!(saved.configuration_revision, 42);
    }
    #[test]
    fn phone_commit_obeys_release_failure_barrier_and_keeps_prepared_work() {
        let (_temp, _lease, mut shared) = fixture();
        let _receiver = authenticate(&shared);
        let draft = begin_edit(&shared, "Phone release barrier");
        hold_key(&shared);
        shared.configuration_write_enabled = false;
        let before = storage::read_state_bytes(&shared.paths.state_file).unwrap();
        let blocked = shared.paths.state_dir.join("blocked-recording");
        fs::create_dir(&blocked).unwrap();
        shared.inner.lock().unwrap().output = OutputExecutor::new(false, Some(blocked));
        let commit = Uuid::new_v4().to_string();
        let request = || OwnedCommitRequest {
            draft_id: &draft.draft_id,
            draft_revision: 2,
            expected: 41,
            commit_id: &commit,
            request_digest: None,
        };
        assert!(matches!(
            shared.owned_save_from_phone(request(), 7),
            Err(SaveConfigurationError::ReleaseFailed(_))
        ));
        assert_eq!(
            storage::read_state_bytes(&shared.paths.state_file).unwrap(),
            before
        );
        shared.inner.lock().unwrap().output = OutputExecutor::new(false, None);
        assert_eq!(
            shared
                .owned_save_from_phone(request(), 7)
                .unwrap()
                .configuration_revision,
            42
        );
    }
    #[test]
    fn misplaced_payload_cannot_reach_input_and_invalid_intent_cannot_create_draft() {
        let (_temp, _lease, shared) = fixture();
        let mut receiver = authenticate(&shared);
        while receiver.try_recv().is_ok() {}
        let mut request = intent();
        let mut message = ControllerMessage::new(ControllerMessageType::Button, 0);
        message.owned_phone_edit = Some(Box::new(request.clone()));
        assert!(shared.try_handle_owned_phone_intent(7, &message).is_err());
        request.control_edits[0].center_x = Some(2.0);
        message.message_type = ControllerMessageType::OwnedPhoneEdit;
        message.profile_format_version = Some(5);
        message.owned_phone_edit = Some(Box::new(request));
        assert!(shared.try_handle_owned_phone_intent(7, &message).unwrap());
        assert_eq!(
            result(&mut receiver).error_code.as_deref(),
            Some("owned_phone_invalid_intent")
        );
        assert!(!shared.paths.drafts_dir.exists());
    }
}
