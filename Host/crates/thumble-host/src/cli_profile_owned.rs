//! High-level planning on owned authorities never serializes a legacy document.
use super::*;
use crate::owned_drafts::{apply_document, authority_document, OwnedDraft, OwnedDraftStore};
use serde_json::value::RawValue;
use thumble_core::{OwnedConfigurationDocument, ProfileAppearanceBank};

fn source(paths: &HostPaths, state: &PersistentState) -> Result<Vec<u8>, TransactionFailure> {
    let bytes = storage::read_state_bytes(&paths.state_file).map_err(|_| {
        TransactionFailure::new(
            "state_load_failed",
            "owned authority could not be read safely",
        )
    })?;
    let mut stored: PersistentState =
        thumble_protocol::decode_unique_json(&bytes).map_err(|_| {
            TransactionFailure::new(
                "invalid_configuration",
                "owned authority could not be decoded",
            )
        })?;
    stored.normalize().map_err(|_| {
        TransactionFailure::new("invalid_configuration", "owned authority is invalid")
    })?;
    if &stored != state {
        return Err(TransactionFailure::new(
            "configuration_revision_conflict",
            "owned authority changed after the configuration snapshot",
        )
        .with_revision(state.configuration_revision, stored.configuration_revision));
    }
    Ok(bytes)
}
fn catalog(
    document: &OwnedConfigurationDocument,
    state: &PersistentState,
) -> Result<CliProfileCatalog, TransactionFailure> {
    catalog_from_parts(
        document.profiles(),
        document.active_profile_id(),
        document.default_profile_id().ok_or_else(|| {
            TransactionFailure::new(
                "invalid_configuration",
                "owned authority requires a default profile",
            )
        })?,
        state.configuration_revision,
        state.trusted_clients.keys().map(String::as_str),
    )
}
fn with_draft(mut failure: TransactionFailure, draft: &OwnedDraft) -> TransactionFailure {
    failure.error.draft_id = Uuid::parse_str(&draft.draft_id).ok();
    failure.error.draft_revision = Some(draft.draft_revision);
    failure
}
fn display_state(state: &PersistentState) -> Result<PersistentState, TransactionFailure> {
    let mut display = state.clone();
    for profile in &mut display.profiles {
        if let Some(bank) = ProfileAppearanceBank::from_profile(profile).map_err(|_| {
            TransactionFailure::new(
                "invalid_configuration",
                "owned appearance could not be validated",
            )
        })? {
            let scheme =
                if profile["customization"]["colorSchemePreference"].as_str() == Some("dark") {
                    "dark"
                } else {
                    "light"
                };
            let resolved = bank
                .resolve_layout(&profile["customization"], scheme, "base")
                .map_err(|_| {
                    TransactionFailure::new(
                        "invalid_configuration",
                        "owned appearance could not be resolved",
                    )
                })?;
            profile["customization"] = resolved;
        }
    }
    Ok(display)
}
fn read_projection(
    state: &PersistentState,
    command: &CliProfileCommand,
    catalog: &CliProfileCatalog,
) -> Result<Option<TransactionResult>, TransactionFailure> {
    let result = match command {
        CliProfileCommand::OrientationGet { target } => {
            let selected = resolve_selector(target, catalog)?;
            TransactionResult {
                orientation: Some(CliOrientationSummary {
                    configuration_revision: state.configuration_revision,
                    profile_id: selected.profile_id,
                    profile_name: selected.name.clone(),
                    orientation: ConfigurationOrientationPreference::Landscape,
                }),
                ..Default::default()
            }
        }
        CliProfileCommand::ControlBarList { target, variant } => TransactionResult {
            control_bar: Some(control_bar_projection_from_state(
                state, target, *variant, catalog,
            )?),
            ..Default::default()
        },
        CliProfileCommand::ControlBarItemShow {
            target,
            variant,
            item,
        } => TransactionResult {
            control_bar_item: Some(control_bar_item_projection_from_state(
                state, target, *variant, *item, catalog,
            )?),
            ..Default::default()
        },
        CliProfileCommand::DeviceGet { target, variant } => TransactionResult {
            device: Some(device_projection_from_state(
                state, target, *variant, catalog,
            )?),
            ..Default::default()
        },
        CliProfileCommand::StyleList { target } => TransactionResult {
            styles: Some(style_projection_from_state(state, target, None, catalog)?),
            ..Default::default()
        },
        CliProfileCommand::StyleShow { target, style_id } => TransactionResult {
            styles: Some(style_projection_from_state(
                state,
                target,
                Some(style_id),
                catalog,
            )?),
            ..Default::default()
        },
        CliProfileCommand::LayerList { target, variant } => TransactionResult {
            layers: Some(layer_projection_from_state(
                state, target, *variant, catalog,
            )?),
            ..Default::default()
        },
        CliProfileCommand::GroupList { target, variant } => TransactionResult {
            groups: Some(group_projection_from_state(
                state, target, *variant, catalog,
            )?),
            ..Default::default()
        },
        command if command.is_binding_output_read() => TransactionResult {
            projection: Some(binding_output_projection_from_state(
                state, command, catalog,
            )?),
            ..Default::default()
        },
        _ => return Ok(None),
    };
    Ok(Some(result))
}
fn reject_portrait_read(command: &CliProfileCommand) -> Result<(), TransactionFailure> {
    let variant = match command {
        CliProfileCommand::ControlBarList { variant, .. }
        | CliProfileCommand::ControlBarItemShow { variant, .. }
        | CliProfileCommand::DeviceGet { variant, .. }
        | CliProfileCommand::LayerList { variant, .. }
        | CliProfileCommand::GroupList { variant, .. } => Some(*variant),
        _ => None,
    };
    if variant == Some(ConfigurationVariant::Portrait) {
        return Err(TransactionFailure::new(
            "unsupported_orientation",
            "owned profiles have one landscape layout",
        ));
    }
    Ok(())
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct Sidecars {
    #[serde(default)]
    key_bindings: ButtonBindings<KeyBinding>,
    #[serde(default)]
    output_bindings: ButtonBindings<OutputBinding>,
    #[serde(default)]
    profile_key_bindings: BTreeMap<String, ButtonBindings<KeyBinding>>,
    #[serde(default)]
    profile_output_bindings: BTreeMap<String, ButtonBindings<OutputBinding>>,
}
fn design_snapshot(
    paths: &HostPaths,
    state: &PersistentState,
    request: &CliProfileRequest,
    source: &[u8],
) -> Result<TransactionResult, TransactionFailure> {
    let CliProfileCommand::DesignSnapshot {
        target,
        draft_id,
        expected_draft_revision,
        create_draft,
    } = &request.command
    else {
        unreachable!()
    };
    if request.expected_configuration_revision != Some(state.configuration_revision) {
        return Err(TransactionFailure::new(
            "configuration_revision_conflict",
            "design capture requires the exact current configuration revision",
        ));
    }
    let store = OwnedDraftStore::new(paths);
    let draft = if *create_draft {
        if draft_id.is_some() || expected_draft_revision.is_some() {
            return Err(TransactionFailure::new(
                "invalid_request",
                "createDraft cannot resume an existing draft",
            ));
        }
        Some(
            store
                .begin(
                    source,
                    state.configuration_revision,
                    &Uuid::new_v4().to_string(),
                    now_millis(),
                )
                .map_err(draft_failure)?,
        )
    } else if let Some(id) = draft_id {
        let draft = store
            .get(&id.to_string(), *expected_draft_revision, now_millis())
            .map_err(draft_failure)?;
        if expected_draft_revision.is_none() {
            return Err(TransactionFailure::new(
                "draft_revision_conflict",
                "design capture requires the exact draft revision",
            ));
        }
        if draft.base_configuration_revision != state.configuration_revision {
            return Err(TransactionFailure::new(
                "configuration_revision_conflict",
                "rebase the captured draft explicitly before design capture",
            ));
        }
        Some(draft)
    } else {
        if expected_draft_revision.is_some() {
            return Err(TransactionFailure::new(
                "invalid_request",
                "draft revision requires draftID",
            ));
        }
        None
    };
    let document = match &draft {
        Some(draft) => draft.document().clone(),
        None => authority_document(source).map_err(draft_failure)?,
    };
    apply_document(source, &document).map_err(draft_failure)?;
    let snapshot_catalog = self::catalog(&document, state)?;
    let selected = resolve_selector(target, &snapshot_catalog)?;
    let root: BTreeMap<String, &RawValue> = serde_json::from_slice(document.original_bytes())
        .map_err(|_| {
            TransactionFailure::new(
                "design_capture_failed",
                "owned capture could not be decoded",
            )
        })?;
    let profiles: Vec<&RawValue> = serde_json::from_str(root["profiles"].get()).map_err(|_| {
        TransactionFailure::new("design_capture_failed", "owned profiles could not be read")
    })?;
    let profile_json = profiles[selected.index].get().to_owned();
    let sidecars: Sidecars = serde_json::from_slice(document.original_bytes()).map_err(|_| {
        TransactionFailure::new("design_capture_failed", "owned bindings could not be read")
    })?;
    let find_keys = sidecars
        .profile_key_bindings
        .iter()
        .find(|(id, _)| id.eq_ignore_ascii_case(&selected.profile_id.to_string()))
        .map(|(_, map)| map);
    let find_outputs = sidecars
        .profile_output_bindings
        .iter()
        .find(|(id, _)| id.eq_ignore_ascii_case(&selected.profile_id.to_string()))
        .map(|(_, map)| map);
    let binding_json = serde_json::to_string(&serde_json::json!({
        "profileID": selected.profile_id,
        "keyBindings": find_keys.or_else(|| selected.active.then_some(&sidecars.key_bindings)).cloned().unwrap_or_default(),
        "outputBindings": find_outputs.or_else(|| selected.active.then_some(&sidecars.output_bindings)).cloned().unwrap_or_default(),
        "inputSettings": design_input_settings(&document.profiles()[selected.index]),
        "outputMode": document.profiles()[selected.index].get("outputMode"),
    })).map_err(|_| TransactionFailure::new("design_capture_failed", "owned bindings could not be encoded"))?;
    if profile_json.len() > 8 * 1024 * 1024 || binding_json.len() > 8 * 1024 * 1024 {
        return Err(TransactionFailure::new(
            "design_capture_too_large",
            "owned capture exceeded its size bound",
        ));
    }
    Ok(TransactionResult {
        design_snapshot: Some(CliDesignSnapshot {
            configuration_revision: state.configuration_revision,
            draft_id: draft
                .as_ref()
                .and_then(|d| Uuid::parse_str(&d.draft_id).ok()),
            draft_revision: draft.map(|d| d.draft_revision),
            profile_json,
            binding_json,
        }),
        ..Default::default()
    })
}

pub(super) fn execute<F>(
    paths: &HostPaths,
    state: &PersistentState,
    request: &CliProfileRequest,
    invocation_id: Uuid,
    catalog: &CliProfileCatalog,
    save: &mut F,
) -> Result<TransactionResult, TransactionFailure>
where
    F: FnMut(&str, u64, u64, &str, &str) -> Result<ConfigurationSaveSummary, TransactionFailure>,
{
    let source = source(paths, state)?;
    authority_document(&source).map_err(draft_failure)?;
    reject_portrait_read(&request.command)?;
    if matches!(request.command, CliProfileCommand::DesignSnapshot { .. }) {
        return design_snapshot(paths, state, request, &source);
    }
    if matches!(
        request.command,
        CliProfileCommand::Import { .. } | CliProfileCommand::Export { .. }
    ) {
        return Err(TransactionFailure::new("owned_interchange_unavailable", "owned portable import/export is not enabled; the authority and original data remain unchanged"));
    }
    let display = display_state(state)?;
    if let Some(result) = read_projection(&display, &request.command, catalog)? {
        return Ok(result);
    }
    let draft_id = match &request.command {
        CliProfileCommand::DesignApply {
            draft_id: Some(id), ..
        } => *id,
        _ => deterministic_uuid(invocation_id, "configuration-draft"),
    };
    let commit_id = deterministic_uuid(invocation_id, "configuration-commit");
    let digest = request_digest(&request.command)?;
    if let Some(record) = state.recent_configuration_commit(&commit_id.to_string()) {
        if record.draft_id != draft_id.to_string()
            || record.client_request_digest.as_deref() != Some(&digest)
            || request
                .expected_configuration_revision
                .is_some_and(|expected| expected != record.base_configuration_revision)
        {
            return Err(TransactionFailure::new(
                "commit_id_conflict",
                "invocation ID was already used for different profile transaction content",
            ));
        }
        return Ok(TransactionResult {
            outcome: Some(CliProfileOutcome {
                operation: request.command.kind().into(),
                profile_names: replay_profile_names(&request.command, catalog, invocation_id),
                destination: replay_destination(&request.command),
                removed_every_profile: replay_removed_every_profile(
                    &request.command,
                    catalog,
                    invocation_id,
                ),
                changed: record.result_configuration_revision != record.base_configuration_revision,
                configuration_revision: record.result_configuration_revision,
                draft_id,
                commit_id,
                idempotent_replay: true,
            }),
            ..Default::default()
        });
    }
    if let Some(expected) = request.expected_configuration_revision {
        if expected != state.configuration_revision {
            return Err(TransactionFailure::new(
                "configuration_revision_conflict",
                "authoritative configuration changed after the revision-tagged read",
            )
            .with_revision(expected, state.configuration_revision));
        }
    }
    let store = OwnedDraftStore::new(paths);
    let mut draft = if let CliProfileCommand::DesignApply {
        draft_id: Some(id),
        expected_draft_revision,
        ..
    } = &request.command
    {
        let draft = store
            .get(&id.to_string(), None, now_millis())
            .map_err(draft_failure)?;
        let operation_id =
            deterministic_uuid(invocation_id, "design.apply:operation:0").to_string();
        let own_retry = draft.operation_records().iter().any(|record| {
            record.operation_id == operation_id
                && Some(record.base_draft_revision) == *expected_draft_revision
                && record.result_draft_revision == draft.draft_revision
        });
        if *expected_draft_revision != Some(draft.draft_revision) && !own_retry {
            return Err(with_draft(
                TransactionFailure::new(
                    "draft_revision_conflict",
                    "captured draft changed since review",
                ),
                &draft,
            ));
        }
        if draft.base_configuration_revision != state.configuration_revision {
            return Err(with_draft(
                TransactionFailure::new(
                    "configuration_revision_conflict",
                    "rebase the captured draft before applying design edits",
                ),
                &draft,
            ));
        }
        draft
    } else {
        store
            .begin(
                &source,
                state.configuration_revision,
                &draft_id.to_string(),
                now_millis(),
            )
            .map_err(draft_failure)?
    };
    let captured = matches!(
        request.command,
        CliProfileCommand::DesignApply {
            draft_id: Some(_),
            ..
        }
    );
    let draft_catalog = self::catalog(draft.document(), state)?;
    // Uncaptured retries must resolve selectors and deterministic destinations
    // against the original authority, even after this draft has renamed/copied it.
    let (planning_catalog, planning_profiles) = if captured {
        (&draft_catalog, draft.document().profiles())
    } else {
        (catalog, state.profiles.as_slice())
    };
    let plan = plan_transaction(
        &request.command,
        planning_catalog,
        planning_profiles,
        invocation_id,
    )
    .map_err(|failure| with_draft(failure, &draft))?;
    for (ordinal, operation) in plan.operations.iter().enumerate() {
        let operation_id = deterministic_uuid(
            invocation_id,
            &format!("{}:operation:{ordinal}", request.command.kind()),
        )
        .to_string();
        if let Some(record) = draft
            .operation_records()
            .iter()
            .find(|record| record.operation_id == operation_id)
        {
            if record.operation_digest
                != DraftStore::operation_digest(operation).map_err(draft_failure)?
            {
                return Err(with_draft(
                    TransactionFailure::new(
                        "operation_id_conflict",
                        "deterministic operation ID was reused with different content",
                    ),
                    &draft,
                ));
            }
            continue;
        }
        draft = store
            .edit(
                &draft.draft_id,
                draft.draft_revision,
                &operation_id,
                operation,
                now_millis(),
            )
            .map_err(|error| with_draft(draft_failure(error), &draft))?
            .draft;
    }
    apply_document(&source, draft.document())
        .map_err(|error| with_draft(draft_failure(error), &draft))?;
    self::catalog(draft.document(), state)?;
    let summary = save(
        &draft.draft_id,
        draft.draft_revision,
        state.configuration_revision,
        &commit_id.to_string(),
        &digest,
    )
    .map_err(|failure| with_draft(failure, &draft))?;
    Ok(TransactionResult {
        outcome: Some(CliProfileOutcome {
            operation: request.command.kind().into(),
            profile_names: plan.profile_names,
            destination: plan.destination,
            removed_every_profile: plan.removed_every_profile,
            changed: summary.changed,
            configuration_revision: summary.configuration_revision,
            draft_id,
            commit_id,
            idempotent_replay: summary.idempotent_replay,
        }),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use thumble_protocol::{edit_json_values, JsonValueEdit};
    const PRECISE: &str =
        "{\"integer\":9007199254740993,\"fraction\":0.12345678901234567890123456789}";
    fn fixture() -> (tempfile::TempDir, crate::runtime::AuthorityLock, HostPaths) {
        let temp = tempfile::tempdir().unwrap();
        let paths = HostPaths::new(
            temp.path().join("authority"),
            temp.path().join("control.sock"),
        );
        let lease = crate::runtime::AuthorityLock::try_acquire(&paths)
            .unwrap()
            .unwrap();
        let mut source =
            serde_json::to_vec(&PersistentState::minimal("owned-high-level").unwrap()).unwrap();
        let raw: BTreeMap<String, &RawValue> = serde_json::from_slice(include_bytes!(
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
            if let Some(value) = raw.get(name) {
                source = edit_json_values(
                    &source,
                    &[JsonValueEdit {
                        pointer: &format!("/{name}"),
                        value: Some(value.get().as_bytes()),
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
                    pointer: "/profiles/0/futureOpaque",
                    value: Some(PRECISE.as_bytes()),
                },
            ],
            16 * 1024 * 1024,
        )
        .unwrap();
        storage::install_private_state_bytes(&paths.state_file, &source, None, || Ok(())).unwrap();
        (temp, lease, paths)
    }
    fn request(command: CliProfileCommand) -> CliProfileRequest {
        let read = command.is_binding_output_read()
            || matches!(
                command,
                CliProfileCommand::OrientationGet { .. } | CliProfileCommand::Export { .. }
            );
        CliProfileRequest {
            schema_version: CLI_PROFILE_SCHEMA_VERSION,
            invocation_id: Some(Uuid::new_v4()),
            expected_configuration_revision: if read { None } else { Some(41) },
            command,
        }
    }
    fn load(paths: &HostPaths) -> PersistentState {
        let mut state = storage::load(&paths.state_file).unwrap();
        state.normalize().unwrap();
        state
    }
    #[test]
    fn name_selector_retry_after_staged_failure_then_commit_and_replay_are_lossless() {
        let (_temp, _lease, paths) = fixture();
        let before = storage::read_state_bytes(&paths.state_file).unwrap();
        let state = load(&paths);
        let req = request(CliProfileCommand::Rename {
            target: ProfileSelector::Name {
                name: "Resource fixture".into(),
            },
            name: "Raid".into(),
        });
        let failed = execute_profile_transaction(&paths, &state, &req, "test", |_, _, _, _, _| {
            Err(TransactionFailure::new(
                "configuration_write_disabled",
                "test",
            ))
        });
        let error = failed.error.unwrap();
        assert_eq!(error.code, "configuration_write_disabled");
        assert_eq!(error.draft_revision, Some(2));
        assert_eq!(
            storage::read_state_bytes(&paths.state_file).unwrap(),
            before
        );
        let saved = execute_offline_authority(&paths, &req);
        assert!(saved.ok, "{:?}", saved.error);
        assert_eq!(saved.outcome.unwrap().configuration_revision, 42);
        let committed = storage::read_state_bytes(&paths.state_file).unwrap();
        assert!(std::str::from_utf8(&committed).unwrap().contains(PRECISE));
        assert_eq!(load(&paths).profiles[0]["name"], "Raid");
        assert!(
            execute_offline_authority(&paths, &req)
                .outcome
                .unwrap()
                .idempotent_replay
        );
        let mut changed = req;
        changed.command = CliProfileCommand::Rename {
            target: ProfileSelector::Active,
            name: "Other".into(),
        };
        assert_eq!(
            execute_offline_authority(&paths, &changed)
                .error
                .unwrap()
                .code,
            "commit_id_conflict"
        );
        assert_eq!(
            storage::read_state_bytes(&paths.state_file).unwrap(),
            committed
        );
    }
    #[test]
    fn design_capture_preserves_raw_profile_and_resumes_the_exact_owned_draft() {
        let (_temp, _lease, paths) = fixture();
        let req = request(CliProfileCommand::DesignSnapshot {
            target: ProfileSelector::Active,
            draft_id: None,
            expected_draft_revision: None,
            create_draft: true,
        });
        let captured = execute_offline_authority(&paths, &req);
        assert!(captured.ok, "{:?}", captured.error);
        let snapshot = captured.design_snapshot.unwrap();
        assert!(snapshot.profile_json.contains(PRECISE));
        let draft_id = snapshot.draft_id.unwrap();
        let source = load(&paths);
        OwnedDraftStore::new(&paths)
            .edit(
                &draft_id.to_string(),
                1,
                &Uuid::new_v4().to_string(),
                &ConfigurationOperation::ProfileRename {
                    profile_id: source.active_profile_id,
                    name: "Staged".into(),
                },
                now_millis(),
            )
            .unwrap();
        let resumed = request(CliProfileCommand::DesignSnapshot {
            target: ProfileSelector::Active,
            draft_id: Some(draft_id),
            expected_draft_revision: Some(2),
            create_draft: false,
        });
        let snapshot = execute_offline_authority(&paths, &resumed)
            .design_snapshot
            .unwrap();
        assert!(snapshot.profile_json.contains(PRECISE));
        assert_eq!(
            serde_json::from_str::<Value>(&snapshot.profile_json).unwrap()["name"],
            "Staged"
        );
        assert_eq!(load(&paths).profiles[0]["name"], "Resource fixture");
        let stale = request(CliProfileCommand::DesignSnapshot {
            target: ProfileSelector::Active,
            draft_id: Some(draft_id),
            expected_draft_revision: Some(1),
            create_draft: false,
        });
        assert_eq!(
            execute_offline_authority(&paths, &stale)
                .error
                .unwrap()
                .code,
            "draft_revision_conflict"
        );
    }
    #[test]
    fn read_projections_resolve_dark_owned_styles_without_persisting_display_paint() {
        let (_temp, _lease, paths) = fixture();
        let original = storage::read_state_bytes(&paths.state_file).unwrap();
        let mut profile: Value = load(&paths).profiles[0].clone();
        let light = serde_json::json!({"id":"owned-style","name":"Light","appliesTo":["button"],"visualStyle":{"normal":{"scale":1}}});
        profile["customization"]["styleLibrary"] = serde_json::json!({"styles":[light.clone()]});
        profile["customization"]["colorSchemePreference"] = serde_json::json!("dark");
        for row in profile["appearanceBank"]["variants"]
            .as_array_mut()
            .unwrap()
        {
            let mut style = light.clone();
            if row["colorScheme"] == "dark" {
                style["name"] = serde_json::json!("Dark");
            }
            row["appearance"]["styleLibrary"] = serde_json::json!({"styles":[style]});
        }
        let encoded = serde_json::to_vec(&profile).unwrap();
        let source = edit_json_values(
            &original,
            &[JsonValueEdit {
                pointer: "/profiles/0",
                value: Some(&encoded),
            }],
            16 * 1024 * 1024,
        )
        .unwrap();
        storage::install_private_state_bytes(
            &paths.state_file,
            &source,
            Some(&original),
            || Ok(()),
        )
        .unwrap();
        let result = execute_offline_authority(
            &paths,
            &request(CliProfileCommand::StyleList {
                target: ProfileSelector::Active,
            }),
        );
        assert!(result.ok, "{:?}", result.error);
        assert_eq!(result.styles.unwrap().styles[0].name, "Dark");
        let result = execute_offline_authority(
            &paths,
            &request(CliProfileCommand::OrientationGet {
                target: ProfileSelector::Active,
            }),
        );
        assert_eq!(
            result.orientation.unwrap().orientation,
            ConfigurationOrientationPreference::Landscape
        );
        let result = execute_offline_authority(
            &paths,
            &request(CliProfileCommand::DeviceGet {
                target: ProfileSelector::Active,
                variant: ConfigurationVariant::Portrait,
            }),
        );
        assert_eq!(result.error.unwrap().code, "unsupported_orientation");
        assert_eq!(
            storage::read_state_bytes(&paths.state_file).unwrap(),
            source
        );
    }
    #[test]
    fn stale_configuration_and_interchange_fail_without_authority_mutation() {
        let (_temp, _lease, paths) = fixture();
        let before = storage::read_state_bytes(&paths.state_file).unwrap();
        let mut req = request(CliProfileCommand::Rename {
            target: ProfileSelector::Active,
            name: "Stale".into(),
        });
        req.expected_configuration_revision = Some(40);
        assert_eq!(
            execute_offline_authority(&paths, &req).error.unwrap().code,
            "configuration_revision_conflict"
        );
        let req = request(CliProfileCommand::Export { target: None });
        assert_eq!(
            execute_offline_authority(&paths, &req).error.unwrap().code,
            "owned_interchange_unavailable"
        );
        assert_eq!(
            storage::read_state_bytes(&paths.state_file).unwrap(),
            before
        );
    }
    #[test]
    fn element_plans_resolve_exact_labels_and_validate_typed_changes() {
        let (_temp, _lease, paths) = fixture();
        let state = load(&paths);
        let catalog = catalog_from_state(&state).unwrap();
        let element = state.profiles[0]["customization"]["elements"][0]["label"]
            .as_str()
            .unwrap();
        let command = CliProfileCommand::ElementSet {
            target: ProfileSelector::Active,
            variant: ConfigurationVariant::Primary,
            element: element.into(),
            changes_json: r#"{"centerX":0.55,"haptic":{"style":"light","intensity":0.42}}"#.into(),
        };
        let plan = plan_transaction(&command, &catalog, &state.profiles, Uuid::new_v4()).unwrap();
        assert!(
            matches!(&plan.operations[0], ConfigurationOperation::ElementSet { element_id, changes, .. } if element_id.eq_ignore_ascii_case(state.profiles[0]["customization"]["elements"][0]["id"].as_str().unwrap()) && changes.center_x == Some(0.55))
        );
        let bad = request(CliProfileCommand::ElementSet {
            target: ProfileSelector::Active,
            variant: ConfigurationVariant::Primary,
            element: element.into(),
            changes_json: r#"{"centerX":0.2,"centerX":0.3}"#.into(),
        });
        assert_eq!(
            execute_offline_authority(&paths, &bad).error.unwrap().code,
            "invalid_element_changes"
        );
        assert!(!paths.drafts_dir.exists());
        let bad = request(CliProfileCommand::ElementSet {
            target: ProfileSelector::Active,
            variant: ConfigurationVariant::Primary,
            element: element.into(),
            changes_json: r#"{"authToken":"forbidden"}"#.into(),
        });
        assert_eq!(
            execute_offline_authority(&paths, &bad).error.unwrap().code,
            "invalid_element_changes"
        );
        assert_eq!(load(&paths).configuration_revision, 41);
    }
    #[test]
    fn element_nudge_delete_and_reset_plans_keep_the_selected_identity() {
        let (_temp, _lease, paths) = fixture();
        let state = load(&paths);
        let catalog = catalog_from_state(&state).unwrap();
        let id = state.profiles[0]["customization"]["elements"][0]["id"]
            .as_str()
            .unwrap();
        for command in [
            CliProfileCommand::ElementNudge {
                target: ProfileSelector::Active,
                variant: ConfigurationVariant::Primary,
                element: id.into(),
                delta_x: 4.,
                delta_y: -2.,
            },
            CliProfileCommand::ElementDelete {
                target: ProfileSelector::Active,
                variant: ConfigurationVariant::Primary,
                element: id.into(),
            },
            CliProfileCommand::ElementReset {
                target: ProfileSelector::Active,
                variant: ConfigurationVariant::Primary,
                element: id.into(),
            },
        ] {
            let plan =
                plan_transaction(&command, &catalog, &state.profiles, Uuid::new_v4()).unwrap();
            match &plan.operations[0] {
                ConfigurationOperation::ElementNudge { element_ids, .. } => {
                    assert!(element_ids[0].eq_ignore_ascii_case(id))
                }
                ConfigurationOperation::ElementDelete { element_id, .. }
                | ConfigurationOperation::ElementReset { element_id, .. } => {
                    assert!(element_id.eq_ignore_ascii_case(id))
                }
                _ => panic!("unexpected operation"),
            }
        }
    }
}
