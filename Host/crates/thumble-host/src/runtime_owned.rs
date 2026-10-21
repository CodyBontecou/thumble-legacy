//! Owned-schema configuration transactions use raw state as their persistence source.
use super::*;
use crate::owned_drafts::{apply_document, OwnedCommitRequest, OwnedDraft, OwnedDraftStore};

impl SharedRuntime {
    pub(super) fn uses_owned_configuration(&self) -> bool {
        self.inner
            .lock()
            .expect("runtime mutex poisoned")
            .core
            .persistent_state()
            .schema_version
            == thumble_core::OWNED_SCHEMA_VERSION
    }

    fn read_owned_source(&self) -> Result<(Vec<u8>, PersistentState), DraftError> {
        let source = storage::read_state_bytes(&self.paths.state_file)
            .map_err(|_| DraftError::InsecureFile)?;
        let mut stored: PersistentState =
            thumble_protocol::decode_unique_json(&source).map_err(|_| DraftError::Malformed)?;
        stored.normalize().map_err(|_| DraftError::Malformed)?;
        Ok((source, stored))
    }
    fn check_owned_source(
        stored: &PersistentState,
        current: &PersistentState,
    ) -> Result<(), DraftError> {
        if stored.schema_version != thumble_core::OWNED_SCHEMA_VERSION || stored != current {
            return Err(DraftError::ConfigurationRevisionConflict {
                expected: current.configuration_revision,
                actual: stored.configuration_revision,
            });
        }
        Ok(())
    }
    pub(super) fn owned_snapshot_source(&self) -> Result<Vec<u8>, DraftError> {
        // Disk reads and schema validation can be expensive; only the final
        // comparison needs to exclude concurrent input/configuration updates.
        let (source, stored) = self.read_owned_source()?;
        let inner = self.inner.lock().expect("runtime mutex poisoned");
        Self::check_owned_source(&stored, inner.core.persistent_state())?;
        Ok(source)
    }
    fn owned_summary(&self, draft: &OwnedDraft) -> Result<ConfigurationDraftSummary, DraftError> {
        let mut summary = draft.summary()?;
        let inner = self.inner.lock().expect("runtime mutex poisoned");
        let tokens: Vec<_> = inner
            .core
            .persistent_state()
            .trusted_clients
            .keys()
            .map(String::as_str)
            .collect();
        summary.active_profile_id = bounded_redacted(
            &summary.active_profile_id,
            &tokens,
            MAXIMUM_DISPLAY_CHARACTERS,
        );
        summary.default_profile_id = bounded_redacted(
            &summary.default_profile_id,
            &tokens,
            MAXIMUM_DISPLAY_CHARACTERS,
        );
        Ok(summary)
    }

    pub(super) fn owned_begin(
        &self,
        revision: u64,
    ) -> Result<ConfigurationDraftSummary, DraftError> {
        let source = self.owned_snapshot_source()?;
        let draft = OwnedDraftStore::new(&self.paths).begin(
            &source,
            revision,
            &Uuid::new_v4().to_string(),
            unix_millis(),
        )?;
        self.owned_summary(&draft)
    }
    pub(super) fn owned_get(&self, id: &str) -> Result<ConfigurationDraftSummary, DraftError> {
        let draft = OwnedDraftStore::new(&self.paths).get(id, None, unix_millis())?;
        self.owned_summary(&draft)
    }
    pub(super) fn owned_edit(
        &self,
        id: &str,
        revision: u64,
        operation_id: &str,
        operation: &ConfigurationOperation,
    ) -> Result<
        (
            ConfigurationDraftSummary,
            crate::draft_operation::ConfigurationOperationOutcome,
            bool,
        ),
        DraftError,
    > {
        // The native design helper runs without holding the input/runtime mutex.
        let edited = OwnedDraftStore::new(&self.paths).edit(
            id,
            revision,
            operation_id,
            operation,
            unix_millis(),
        )?;
        Ok((
            self.owned_summary(&edited.draft)?,
            edited.outcome,
            edited.idempotent_replay,
        ))
    }
    pub(super) fn owned_rebase(
        &self,
        id: &str,
        revision: u64,
        configuration_revision: u64,
        operation_id: &str,
    ) -> Result<
        (
            ConfigurationDraftSummary,
            crate::draft_operation::ConfigurationOperationOutcome,
            bool,
        ),
        DraftError,
    > {
        let source = self.owned_snapshot_source()?;
        let edited = OwnedDraftStore::new(&self.paths).rebase(
            &source,
            id,
            revision,
            configuration_revision,
            operation_id,
            unix_millis(),
        )?;
        Ok((
            self.owned_summary(&edited.draft)?,
            edited.outcome,
            edited.idempotent_replay,
        ))
    }
    pub(super) fn owned_validate(
        &self,
        id: &str,
        revision: u64,
    ) -> Result<ConfigurationValidationSummary, DraftError> {
        let draft = OwnedDraftStore::new(&self.paths).get(id, Some(revision), unix_millis())?;
        let source = self.owned_snapshot_source()?;
        apply_document(&source, draft.document())?;
        Ok(ConfigurationValidationSummary {
            draft_id: draft.draft_id,
            draft_revision: draft.draft_revision,
            valid: true,
            error_count: 0,
            warning_count: 0,
            // Structural validation does not claim a native render or operation replay.
            validator: "rust-owned-structural-v1".into(),
        })
    }
    pub(super) fn owned_preview(
        &self,
        id: &str,
        revision: u64,
    ) -> Result<(ConfigurationDraftSummary, ControllerSnapshot, String), DraftError> {
        let draft = OwnedDraftStore::new(&self.paths).get(id, Some(revision), unix_millis())?;
        let source = self.owned_snapshot_source()?;
        let candidate: PersistentState =
            thumble_protocol::decode_unique_json(&apply_document(&source, draft.document())?)
                .map_err(|_| DraftError::Malformed)?;
        let snapshot = bounded_controller_snapshot(&candidate).map_err(DraftError::Preview)?;
        Ok((self.owned_summary(&draft)?, snapshot, "primary".into()))
    }
    pub(super) fn owned_discard(&self, id: &str, revision: u64) -> Result<String, DraftError> {
        OwnedDraftStore::new(&self.paths).discard(id, revision, unix_millis())?;
        Uuid::parse_str(id)
            .map(|id| id.to_string())
            .map_err(|_| DraftError::InvalidDraftId)
    }
    pub(super) fn owned_save(
        &self,
        request: OwnedCommitRequest<'_>,
    ) -> Result<ConfigurationSaveSummary, SaveConfigurationError> {
        self.owned_save_with_origin(request, None)
    }
    pub(super) fn owned_save_from_phone(
        &self,
        request: OwnedCommitRequest<'_>,
        connection: ConnectionId,
    ) -> Result<ConfigurationSaveSummary, SaveConfigurationError> {
        self.owned_save_with_origin(request, Some(connection))
    }
    fn owned_save_with_origin(
        &self,
        request: OwnedCommitRequest<'_>,
        connection: Option<ConnectionId>,
    ) -> Result<ConfigurationSaveSummary, SaveConfigurationError> {
        if connection.is_none() && !self.configuration_write_enabled {
            return Err(SaveConfigurationError::Disabled);
        }
        let authorized = |inner: &RuntimeInner| {
            connection.is_none_or(|id| {
                inner.connections.contains_key(&id) && inner.core.accepts_owned_phone_edits(id)
            })
        };
        if !authorized(&self.inner.lock().expect("runtime mutex poisoned")) {
            return Err(SaveConfigurationError::Disabled);
        }
        let now = unix_millis();
        let store = OwnedDraftStore::new(&self.paths);
        let commit_error =
            |error| SaveConfigurationError::Commit(ConfigurationCommitError::Draft(error));
        // Disk reads, draft decoding and candidate validation stay off the input mutex.
        let source = self.owned_snapshot_source().map_err(commit_error)?;
        let mut original: PersistentState = thumble_protocol::decode_unique_json(&source)
            .map_err(|_| commit_error(DraftError::Malformed))?;
        original
            .normalize()
            .map_err(|_| commit_error(DraftError::Malformed))?;
        let id = request.draft_id.to_owned();
        let revision = request.draft_revision;
        let prepared = store
            .prepare_commit(&source, request, now)
            .map_err(commit_error)?;
        let mut summary = prepared.summary;
        let next = prepared
            .candidate
            .as_ref()
            .map(|candidate| {
                let mut state: PersistentState = thumble_protocol::decode_unique_json(candidate)
                    .map_err(|_| commit_error(DraftError::Malformed))?;
                state
                    .normalize()
                    .map_err(|_| commit_error(DraftError::Malformed))?;
                Ok(state)
            })
            .transpose()?;
        let mut inner = self.inner.lock().expect("runtime mutex poisoned");
        if !authorized(&inner) {
            return Err(SaveConfigurationError::Disabled);
        }
        let Some(candidate) = prepared.candidate else {
            return Ok(summary);
        };
        Self::check_owned_source(&original, inner.core.persistent_state()).map_err(commit_error)?;
        if summary.changed {
            let releases = inner.core.release_all_locally();
            let effects_result =
                self.execute_effects_with_output_errors(&mut inner, releases, true);
            let tracked_result = inner.output.release_tracked();
            effects_result.map_err(SaveConfigurationError::ReleaseFailed)?;
            tracked_result.map_err(SaveConfigurationError::ReleaseFailed)?;
        }
        storage::install_private_state_bytes(
            &self.paths.state_file,
            &candidate,
            Some(&source),
            || Ok(()),
        )
        .map_err(SaveConfigurationError::PersistenceFailed)?;
        let effects = inner
            .core
            .install_validated_persisted_state(next.expect("candidate was validated"));
        summary.phone_sync_queued = effects
            .iter()
            .any(|effect| matches!(effect, Effect::SendMessage { .. }));
        if self.execute_effects(&mut inner, effects).is_err() {
            self.log("owned configuration committed; phone delivery deferred");
        }
        drop(inner);
        if store.discard(&id, revision, now).is_err() {
            self.log("owned configuration committed; draft cleanup deferred");
        }
        Ok(summary)
    }
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use serde_json::value::RawValue;
    use thumble_protocol::{edit_json_values, JsonValueEdit};
    pub(in crate::runtime) fn fixture() -> (tempfile::TempDir, AuthorityLock, SharedRuntime) {
        let directory = tempfile::tempdir().unwrap();
        let paths = HostPaths::new(
            directory.path().join("authority"),
            directory.path().join("control.sock"),
        );
        let lease = AuthorityLock::try_acquire(&paths).unwrap().unwrap();
        let mut source =
            serde_json::to_vec(&PersistentState::minimal("owned-runtime-test").unwrap()).unwrap();
        let document: BTreeMap<String, &RawValue> = serde_json::from_slice(include_bytes!(
            "../../../fixtures/owned-configuration/v1/document.json"
        ))
        .unwrap();
        for name in [
            "profiles",
            "activeProfileID",
            "defaultProfileID",
            "keyBindings",
            "outputBindings",
            "profileKeyBindings",
            "profileOutputBindings",
        ] {
            if let Some(raw) = document.get(name) {
                source = edit_json_values(
                    &source,
                    &[JsonValueEdit {
                        pointer: &format!("/{name}"),
                        value: Some(raw.get().as_bytes()),
                    }],
                    16 * 1024 * 1024,
                )
                .unwrap();
            }
        }
        source = edit_json_values(
            &source,
            &[
                JsonValueEdit {
                    pointer: "/schemaVersion",
                    value: Some(b"4"),
                },
                JsonValueEdit {
                    pointer: "/configurationRevision",
                    value: Some(b"41"),
                },
                JsonValueEdit {
                    pointer: "/futureState",
                    value: Some(b"9007199254740993"),
                },
            ],
            16 * 1024 * 1024,
        )
        .unwrap();
        storage::install_private_state_bytes(&paths.state_file, &source, None, || Ok(())).unwrap();
        let mut shared = super::super::tests::failing_controller_runtime(paths);
        shared.configuration_write_enabled = true;
        shared.input_enabled = false;
        let mut state = storage::load(&shared.paths.state_file).unwrap();
        state.normalize().unwrap();
        let mut inner = shared.inner.lock().unwrap();
        inner.core = HostCore::new(state, "123456").unwrap();
        inner.output = OutputExecutor::new(false, None);
        drop(inner);
        (directory, lease, shared)
    }
    pub(in crate::runtime) fn begin_edit(
        shared: &SharedRuntime,
        name: &str,
    ) -> ConfigurationDraftSummary {
        let response = shared.handle(ControlRequest::BeginConfigurationDraft {
            expected_configuration_revision: 41,
        });
        assert!(response.ok, "{:?}", response.error);
        let draft = response.draft.unwrap();
        let id = shared
            .inner
            .lock()
            .unwrap()
            .core
            .persistent_state()
            .active_profile_id
            .clone();
        let edited = shared.handle(ControlRequest::EditConfigurationDraft {
            draft_id: draft.draft_id,
            expected_draft_revision: 1,
            operation_id: Uuid::new_v4().to_string(),
            operation: ConfigurationOperation::ProfileRename {
                profile_id: id,
                name: name.into(),
            },
        });
        assert!(edited.ok, "{:?}", edited.error);
        edited.draft.unwrap()
    }
    #[test]
    fn owned_control_transactions_persist_and_replay_without_legacy_projection() {
        let (_directory, _lease, shared) = fixture();
        let draft = begin_edit(&shared, "Through control socket");
        assert_eq!(draft.draft_revision, 2);
        let validation = shared.handle(ControlRequest::ValidateConfigurationDraft {
            draft_id: draft.draft_id.clone(),
            expected_draft_revision: 2,
        });
        assert!(validation.ok);
        assert_eq!(
            validation.validation.unwrap().validator,
            "rust-owned-structural-v1"
        );
        let preview = shared.handle(ControlRequest::PreviewConfigurationDraft {
            draft_id: draft.draft_id.clone(),
            expected_draft_revision: 2,
        });
        assert!(preview.ok, "{:?}", preview.error);
        assert_eq!(preview.editable_variant.as_deref(), Some("primary"));
        assert_eq!(
            preview.controller.unwrap().profile.name,
            "Through control socket"
        );
        hold_key(&shared);
        {
            let mut inner = shared.inner.lock().unwrap();
            inner
                .output
                .execute(&Effect::KeyDown(thumble_core::KeyBinding::new(37, 0)))
                .unwrap();
            assert_eq!(inner.output.snapshot().held_key_count, 2);
        }
        let request = ControlRequest::SaveConfigurationDraft {
            draft_id: draft.draft_id,
            expected_draft_revision: 2,
            expected_configuration_revision: 41,
            commit_id: Uuid::new_v4().to_string(),
        };
        let save = shared.handle(request.clone());
        assert!(save.ok, "{:?}", save.error);
        assert_eq!(save.configuration_revision, Some(42));
        assert_eq!(
            shared
                .inner
                .lock()
                .unwrap()
                .output
                .snapshot()
                .held_key_count,
            0
        );
        let raw = storage::read_state_bytes(&shared.paths.state_file).unwrap();
        assert!(std::str::from_utf8(&raw)
            .unwrap()
            .contains("9007199254740993"));
        let mut persisted = storage::load(&shared.paths.state_file).unwrap();
        persisted.normalize().unwrap();
        assert_eq!(
            shared.inner.lock().unwrap().core.persistent_state(),
            &persisted
        );
        assert!(shared.handle(request).save.unwrap().idempotent_replay);
        assert_eq!(
            storage::read_state_bytes(&shared.paths.state_file).unwrap(),
            raw
        );
        assert_eq!(persisted.profiles[0]["name"], "Through control socket");
    }
    #[test]
    fn disabled_owned_save_and_disk_memory_divergence_preserve_drafts_and_state() {
        let (_directory, _lease, mut shared) = fixture();
        let draft = begin_edit(&shared, "Staged only");
        let request = ControlRequest::SaveConfigurationDraft {
            draft_id: draft.draft_id.clone(),
            expected_draft_revision: 2,
            expected_configuration_revision: 41,
            commit_id: Uuid::new_v4().to_string(),
        };
        let before = storage::read_state_bytes(&shared.paths.state_file).unwrap();
        shared.configuration_write_enabled = false;
        assert_eq!(
            shared.handle(request.clone()).error_code.as_deref(),
            Some("configuration_write_disabled")
        );
        assert_eq!(
            storage::read_state_bytes(&shared.paths.state_file).unwrap(),
            before
        );
        shared.configuration_write_enabled = true;
        let changed = edit_json_values(
            &before,
            &[JsonValueEdit {
                pointer: "/configurationRevision",
                value: Some(b"42"),
            }],
            16 * 1024 * 1024,
        )
        .unwrap();
        storage::install_private_state_bytes(
            &shared.paths.state_file,
            &changed,
            Some(&before),
            || Ok(()),
        )
        .unwrap();
        let rejected = shared.handle(request);
        assert_eq!(
            rejected.error_code.as_deref(),
            Some("configuration_revision_conflict")
        );
        assert_eq!(
            storage::read_state_bytes(&shared.paths.state_file).unwrap(),
            changed
        );
        assert_eq!(
            shared
                .inner
                .lock()
                .unwrap()
                .core
                .persistent_state()
                .configuration_revision,
            41
        );
        assert!(
            shared
                .handle(ControlRequest::GetConfigurationDraft {
                    draft_id: draft.draft_id
                })
                .ok
        );
    }
    #[test]
    fn owned_rebase_and_discard_use_the_owned_store() {
        let (_directory, _lease, shared) = fixture();
        let draft = begin_edit(&shared, "Local");
        let rebased = shared.handle(ControlRequest::RebaseConfigurationDraft {
            draft_id: draft.draft_id.clone(),
            expected_draft_revision: 2,
            expected_configuration_revision: 41,
            rebase_id: Uuid::new_v4().to_string(),
        });
        assert!(rebased.ok, "{:?}", rebased.error);
        assert_eq!(rebased.draft.unwrap().draft_revision, 2);
        let discarded = shared.handle(ControlRequest::DiscardConfigurationDraft {
            draft_id: draft.draft_id.clone(),
            expected_draft_revision: 2,
        });
        assert!(discarded.ok);
        assert!(
            !shared
                .handle(ControlRequest::GetConfigurationDraft {
                    draft_id: draft.draft_id
                })
                .ok
        );
    }
    pub(in crate::runtime) fn hold_key(shared: &SharedRuntime) {
        let mut inner = shared.inner.lock().unwrap();
        let effects = inner
            .core
            .set_local_output_binding(
                "held",
                Some(OutputBinding {
                    keyboard: Some(thumble_core::KeyBinding::new(36, 0)),
                    gamepad_buttons: Default::default(),
                }),
                true,
                unix_millis(),
            )
            .unwrap();
        shared.execute_effects(&mut inner, effects).unwrap();
        assert_eq!(inner.output.snapshot().held_key_count, 1);
    }
    #[test]
    fn release_failure_prevents_owned_persistence_and_retry_can_commit() {
        let (_directory, _lease, shared) = fixture();
        let draft = begin_edit(&shared, "Retry after release");
        hold_key(&shared);
        let before = storage::read_state_bytes(&shared.paths.state_file).unwrap();
        let blocked = shared.paths.state_dir.join("recording-directory");
        fs::create_dir(&blocked).unwrap();
        shared.inner.lock().unwrap().output = OutputExecutor::new(false, Some(blocked));
        let request = ControlRequest::SaveConfigurationDraft {
            draft_id: draft.draft_id.clone(),
            expected_draft_revision: 2,
            expected_configuration_revision: 41,
            commit_id: Uuid::new_v4().to_string(),
        };
        assert_eq!(
            shared.handle(request.clone()).error_code.as_deref(),
            Some("input_release_failed")
        );
        assert_eq!(
            storage::read_state_bytes(&shared.paths.state_file).unwrap(),
            before
        );
        assert_eq!(
            shared
                .inner
                .lock()
                .unwrap()
                .core
                .persistent_state()
                .configuration_revision,
            41
        );
        assert!(
            shared
                .handle(ControlRequest::GetConfigurationDraft {
                    draft_id: draft.draft_id
                })
                .ok
        );
        shared.inner.lock().unwrap().output = OutputExecutor::new(false, None);
        assert!(shared.handle(request).ok);
        assert_eq!(
            storage::load(&shared.paths.state_file)
                .unwrap()
                .configuration_revision,
            42
        );
    }
    #[test]
    fn non_regular_authority_source_rejects_without_waiting_for_a_writer() {
        let (_directory, _lease, shared) = fixture();
        fs::remove_file(&shared.paths.state_file).unwrap();
        use std::os::unix::ffi::OsStrExt;
        let path = std::ffi::CString::new(shared.paths.state_file.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
        let (sender, receiver) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            sender
                .send(matches!(
                    shared.owned_begin(41),
                    Err(DraftError::InsecureFile)
                ))
                .unwrap();
        });
        assert!(receiver.recv_timeout(Duration::from_secs(2)).unwrap());
        worker.join().unwrap();
    }
}
