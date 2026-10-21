//! Canonical phone intents stage private owned drafts. Callers must authenticate
//! the peer, hold the authority lease and use the ordinary release/CAS commit
//! path before acknowledging success. No peer format is activated here.
use crate::{
    bridge::ConfigurationBridge,
    draft_operation::{
        ConfigurationBackgroundEdit, ConfigurationColorScheme, ConfigurationOperation,
        ConfigurationOperationOutcome, ConfigurationVariant, CustomizationChanges, ElementChanges,
    },
    drafts::{DraftError, DraftStore},
    owned_drafts::{authority_document, OwnedDraft, OwnedDraftStore},
    paths::HostPaths,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use thumble_core::{OwnedConfigurationDocument, PersistentState, OWNED_SCHEMA_VERSION};
use thumble_protocol::{OwnedPhoneColorScheme, OwnedPhoneEdit};
use uuid::Uuid;
#[derive(Debug)]
pub enum OwnedPhoneStageError {
    InvalidIntent,
    InvalidAuthority,
    MissingProfile,
    MissingControl,
    RevisionConflict { expected: u64, actual: u64 },
    IntentConflict,
    Draft(DraftError),
}
impl From<DraftError> for OwnedPhoneStageError {
    fn from(value: DraftError) -> Self {
        Self::Draft(value)
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnedPhoneCommittedEdit {
    pub configuration_revision: u64,
    pub changed: bool,
    pub idempotent_replay: bool,
}
pub enum OwnedPhoneStage {
    Prepared(Box<PreparedOwnedPhoneEdit>),
    AlreadyCommitted(OwnedPhoneCommittedEdit),
}
pub struct PreparedOwnedPhoneEdit {
    pub draft: OwnedDraft,
    pub commit_id: Uuid,
    pub request_digest: String,
}
fn id(intent: &OwnedPhoneEdit, slot: &str) -> Uuid {
    Uuid::new_v5(
        &intent.edit_id,
        format!("owned-phone-edit:{slot}").as_bytes(),
    )
}
fn digest(intent: &OwnedPhoneEdit) -> Result<String, OwnedPhoneStageError> {
    serde_json::to_vec(intent)
        .map(|v| format!("{:x}", Sha256::digest(v)))
        .map_err(|_| OwnedPhoneStageError::InvalidIntent)
}
/// Geometry/color intent cannot carry outputs, arbitrary appearance banks, paths,
/// profile selection, orientation variants, or package ownership.
fn plan(
    document: &OwnedConfigurationDocument,
    intent: &OwnedPhoneEdit,
) -> Result<Vec<ConfigurationOperation>, OwnedPhoneStageError> {
    let profile = document
        .profiles()
        .iter()
        .find(|p| p["id"].as_str().and_then(|v| v.parse::<Uuid>().ok()) == Some(intent.profile_id))
        .ok_or(OwnedPhoneStageError::MissingProfile)?;
    let profile_id = profile["id"]
        .as_str()
        .ok_or(OwnedPhoneStageError::InvalidAuthority)?
        .to_owned();
    let controls = profile["customization"]["elements"]
        .as_array()
        .ok_or(OwnedPhoneStageError::InvalidAuthority)?;
    for requested in intent
        .control_edits
        .iter()
        .map(|e| e.control_id)
        .chain(intent.removed_control_ids.iter().copied())
    {
        if !controls
            .iter()
            .any(|c| c["id"].as_str().and_then(|v| v.parse::<Uuid>().ok()) == Some(requested))
        {
            return Err(OwnedPhoneStageError::MissingControl);
        }
    }
    let mut operations = Vec::new();
    for edit in &intent.control_edits {
        operations.push(ConfigurationOperation::ElementSet {
            profile_id: profile_id.clone(),
            variant: ConfigurationVariant::Primary,
            element_id: edit.control_id.to_string(),
            changes: Box::new(ElementChanges {
                center_x: edit.center_x,
                center_y: edit.center_y,
                width_scale: edit.width_scale,
                height_scale: edit.height_scale,
                rotation_degrees: edit.rotation_degrees,
                is_hidden: edit.is_hidden,
                is_location_locked: edit.is_location_locked,
                z_index: edit.z_index,
                ..Default::default()
            }),
        });
    }
    for control in &intent.removed_control_ids {
        operations.push(ConfigurationOperation::ElementDelete {
            profile_id: profile_id.clone(),
            variant: ConfigurationVariant::Primary,
            element_id: control.to_string(),
        });
    }
    if let Some(scheme) = intent.color_scheme {
        operations.push(ConfigurationOperation::CustomizationSet {
            profile_id,
            variant: ConfigurationVariant::Primary,
            changes: CustomizationChanges {
                layout_mode: None,
                control_scale: None,
                color_scheme: Some(match scheme {
                    OwnedPhoneColorScheme::System => ConfigurationColorScheme::System,
                    OwnedPhoneColorScheme::Light => ConfigurationColorScheme::Light,
                    OwnedPhoneColorScheme::Dark => ConfigurationColorScheme::Dark,
                }),
                accent_style: None,
                shows_button_labels: None,
                background_edit: ConfigurationBackgroundEdit::Keep,
            },
        });
    }
    for operation in &operations {
        operation
            .validate_bridge_input()
            .map_err(|_| OwnedPhoneStageError::InvalidIntent)?;
    }
    Ok(operations)
}
pub fn stage(
    paths: &HostPaths,
    source: &[u8],
    intent: &OwnedPhoneEdit,
    bridge: &ConfigurationBridge,
    now: i64,
) -> Result<OwnedPhoneStage, OwnedPhoneStageError> {
    stage_with(paths, source, intent, now, |document, operation| {
        bridge
            .apply_owned(document, operation, now, Uuid::new_v4())
            .map_err(DraftError::Bridge)
    })
}
/// Replay and invalid/stale requests do not require native helper discovery.
pub fn stage_local(
    paths: &HostPaths,
    source: &[u8],
    intent: &OwnedPhoneEdit,
    now: i64,
) -> Result<OwnedPhoneStage, OwnedPhoneStageError> {
    let mut bridge = None;
    stage_with(paths, source, intent, now, |document, operation| {
        if bridge.is_none() {
            bridge = Some(ConfigurationBridge::discover().map_err(DraftError::Bridge)?);
        }
        bridge
            .as_ref()
            .unwrap()
            .apply_owned(document, operation, now, Uuid::new_v4())
            .map_err(DraftError::Bridge)
    })
}
fn stage_with(
    paths: &HostPaths,
    source: &[u8],
    intent: &OwnedPhoneEdit,
    now: i64,
    mut apply: impl FnMut(
        &OwnedConfigurationDocument,
        &ConfigurationOperation,
    ) -> Result<
        (OwnedConfigurationDocument, ConfigurationOperationOutcome),
        DraftError,
    >,
) -> Result<OwnedPhoneStage, OwnedPhoneStageError> {
    intent
        .validate()
        .map_err(|_| OwnedPhoneStageError::InvalidIntent)?;
    let document = authority_document(source)?;
    let state: PersistentState = thumble_protocol::decode_unique_json(source)
        .map_err(|_| OwnedPhoneStageError::InvalidAuthority)?;
    if state.schema_version != OWNED_SCHEMA_VERSION || now < 0 {
        return Err(OwnedPhoneStageError::InvalidAuthority);
    }
    let digest = digest(intent)?;
    let draft_id = id(intent, "draft");
    let commit_id = id(intent, "commit");
    if let Some(record) = state.recent_configuration_commit(&commit_id.to_string()) {
        if record.draft_id != draft_id.to_string()
            || record.base_configuration_revision != intent.base_configuration_revision
            || record.client_request_digest.as_deref() != Some(&digest)
        {
            return Err(OwnedPhoneStageError::IntentConflict);
        }
        return Ok(OwnedPhoneStage::AlreadyCommitted(OwnedPhoneCommittedEdit {
            configuration_revision: record.result_configuration_revision,
            changed: record.base_configuration_revision != record.result_configuration_revision,
            idempotent_replay: true,
        }));
    }
    if state.configuration_revision != intent.base_configuration_revision {
        return Err(OwnedPhoneStageError::RevisionConflict {
            expected: intent.base_configuration_revision,
            actual: state.configuration_revision,
        });
    }
    let operations = plan(&document, intent)?;
    let store = OwnedDraftStore::new(paths);
    let mut draft = store.begin(
        source,
        intent.base_configuration_revision,
        &draft_id.to_string(),
        now,
    )?;
    // Bind the complete intent before any operation. A changed later operation
    // must not be accepted merely because its shared prefix already replayed.
    draft = store
        .edit_with_descriptor(
            &draft.draft_id,
            1,
            &id(intent, "intent").to_string(),
            &digest,
            now,
            |document| {
                Ok((
                    document.clone(),
                    ConfigurationOperationOutcome {
                        changed: false,
                        changed_paths: vec![],
                    },
                ))
            },
        )?
        .draft;
    for (ordinal, operation) in operations.iter().enumerate() {
        let operation_id = id(intent, &format!("operation:{ordinal}")).to_string();
        let descriptor = DraftStore::operation_digest(operation)?;
        if let Some(record) = draft
            .operation_records()
            .iter()
            .find(|r| r.operation_id == operation_id)
        {
            if record.operation_digest != descriptor {
                return Err(OwnedPhoneStageError::IntentConflict);
            }
            continue;
        }
        draft = store
            .edit_with(
                &draft.draft_id,
                draft.draft_revision,
                &operation_id,
                operation,
                now,
                |document| apply(document, operation),
            )?
            .draft;
    }
    Ok(OwnedPhoneStage::Prepared(Box::new(
        PreparedOwnedPhoneEdit {
            draft,
            commit_id,
            request_digest: digest,
        },
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        owned_drafts::{apply_document, OwnedCommitRequest},
        runtime::AuthorityLock,
        storage,
    };
    use serde_json::{value::RawValue, Value};
    use thumble_protocol::{edit_json_values, JsonValueEdit, OwnedPhoneControlEdit};
    fn fixture() -> (
        tempfile::TempDir,
        AuthorityLock,
        HostPaths,
        Vec<u8>,
        OwnedPhoneEdit,
    ) {
        let temp = tempfile::tempdir().unwrap();
        let paths = HostPaths::new(
            temp.path().join("authority"),
            temp.path().join("control.sock"),
        );
        let lease = AuthorityLock::try_acquire(&paths).unwrap().unwrap();
        let mut source =
            serde_json::to_vec(&PersistentState::minimal("owned-phone-fixture").unwrap()).unwrap();
        let raw: std::collections::BTreeMap<String, &RawValue> = serde_json::from_slice(
            include_bytes!("../../../fixtures/owned-configuration/v1/document.json"),
        )
        .unwrap();
        for key in [
            "profiles",
            "activeProfileID",
            "defaultProfileID",
            "keyBindings",
            "outputBindings",
            "profileKeyBindings",
            "profileOutputBindings",
        ] {
            source = edit_json_values(
                &source,
                &[JsonValueEdit {
                    pointer: &format!("/{key}"),
                    value: Some(raw[key].get().as_bytes()),
                }],
                16 * 1024 * 1024,
            )
            .unwrap();
        }
        source=edit_json_values(&source,&[JsonValueEdit{pointer:"/schemaVersion",value:Some(b"4")},JsonValueEdit{pointer:"/configurationRevision",value:Some(b"41")},JsonValueEdit{pointer:"/ownedConfigurationExtensions",value:Some(br#"{"futurePhoneMetadata":{"integer":9007199254740993,"fraction":0.12345678901234567890123456789}}"#)}],16*1024*1024).unwrap();
        storage::install_private_state_bytes(&paths.state_file, &source, None, || Ok(())).unwrap();
        let document = authority_document(&source).unwrap();
        let intent = OwnedPhoneEdit {
            schema_version: 1,
            edit_id: Uuid::new_v4(),
            profile_id: document.profiles()[0]["id"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap(),
            base_configuration_revision: 41,
            control_edits: vec![OwnedPhoneControlEdit {
                control_id: document.profiles()[0]["customization"]["elements"][0]["id"]
                    .as_str()
                    .unwrap()
                    .parse()
                    .unwrap(),
                center_x: Some(0.6),
                center_y: None,
                width_scale: None,
                height_scale: None,
                rotation_degrees: None,
                is_hidden: None,
                is_location_locked: None,
                z_index: None,
            }],
            removed_control_ids: vec![],
            color_scheme: Some(OwnedPhoneColorScheme::Dark),
        };
        (temp, lease, paths, source, intent)
    }
    fn apply(
        document: &OwnedConfigurationDocument,
        operation: &ConfigurationOperation,
    ) -> Result<(OwnedConfigurationDocument, ConfigurationOperationOutcome), DraftError> {
        let (pointer, value) = match operation {
            ConfigurationOperation::ElementSet { changes, .. } => (
                "/profiles/0/customization/elements/0/layout/centerX",
                serde_json::to_vec(&changes.center_x.unwrap()).unwrap(),
            ),
            ConfigurationOperation::CustomizationSet { changes, .. } => (
                "/profiles/0/customization/colorSchemePreference",
                serde_json::to_vec(&changes.color_scheme.unwrap()).unwrap(),
            ),
            _ => panic!("unexpected test operation"),
        };
        let next = document
            .with_edits(&[JsonValueEdit {
                pointer,
                value: Some(&value),
            }])
            .unwrap();
        let changed = next.original_bytes() != document.original_bytes();
        Ok((
            next,
            ConfigurationOperationOutcome {
                changed,
                changed_paths: if changed {
                    vec![pointer.into()]
                } else {
                    vec![]
                },
            },
        ))
    }
    fn prepared(result: OwnedPhoneStage) -> PreparedOwnedPhoneEdit {
        match result {
            OwnedPhoneStage::Prepared(p) => *p,
            _ => panic!("expected staged draft"),
        }
    }
    #[test]
    fn staging_preserves_authority_and_every_appearance_state_then_commits_once() {
        let (_temp, _lease, paths, source, intent) = fixture();
        let before = authority_document(&source).unwrap();
        let staged = prepared(stage_with(&paths, &source, &intent, 100, apply).unwrap());
        assert_eq!(
            storage::read_state_bytes(&paths.state_file).unwrap(),
            source
        );
        for (a, b) in staged
            .draft
            .document()
            .profiles()
            .iter()
            .zip(before.profiles())
        {
            for key in ["appearanceBank", "appearanceResources", "launchTarget"] {
                assert_eq!(a[key], b[key], "{key}");
            }
        }
        assert!(
            std::str::from_utf8(staged.draft.document().original_bytes())
                .unwrap()
                .contains("0.12345678901234567890123456789")
        );
        let replay = prepared(
            stage_with(&paths, &source, &intent, 101, |_, _| {
                panic!("staged replay invoked native work")
            })
            .unwrap(),
        );
        assert_eq!(
            replay.draft.document().original_bytes(),
            staged.draft.document().original_bytes()
        );
        let committed = OwnedDraftStore::new(&paths)
            .prepare_commit(
                &source,
                OwnedCommitRequest {
                    draft_id: &staged.draft.draft_id,
                    draft_revision: staged.draft.draft_revision,
                    expected: 41,
                    commit_id: &staged.commit_id.to_string(),
                    request_digest: Some(&staged.request_digest),
                },
                102,
            )
            .unwrap();
        let candidate = committed.candidate.unwrap();
        storage::install_private_state_bytes(&paths.state_file, &candidate, Some(&source), || {
            Ok(())
        })
        .unwrap();
        match stage_with(&paths, &candidate, &intent, 103, |_, _| {
            panic!("committed replay invoked native work")
        })
        .unwrap()
        {
            OwnedPhoneStage::AlreadyCommitted(receipt) => {
                assert_eq!(receipt.configuration_revision, 42);
                assert!(receipt.idempotent_replay);
            }
            _ => panic!("missing commit replay"),
        }
        assert_eq!(
            storage::read_state_bytes(&paths.state_file).unwrap(),
            candidate
        );
    }
    #[test]
    fn interrupted_batch_resumes_only_unfinished_operations() {
        let (_temp, _lease, paths, source, intent) = fixture();
        let mut calls = 0;
        assert!(stage_with(&paths, &source, &intent, 100, |d, o| {
            calls += 1;
            if calls == 2 {
                return Err(DraftError::Malformed);
            }
            apply(d, o)
        })
        .is_err());
        assert_eq!(
            storage::read_state_bytes(&paths.state_file).unwrap(),
            source
        );
        let mut resumed = 0;
        let staged = prepared(
            stage_with(&paths, &source, &intent, 101, |d, o| {
                resumed += 1;
                apply(d, o)
            })
            .unwrap(),
        );
        assert_eq!(resumed, 1);
        assert_eq!(staged.draft.draft_revision, 3);
    }
    #[test]
    fn changed_later_intent_is_refused_before_any_prefix_can_replay_or_mutate() {
        let (_temp, _lease, paths, source, mut intent) = fixture();
        let first = prepared(stage_with(&paths, &source, &intent, 100, apply).unwrap());
        let file = paths
            .drafts_dir
            .join("owned-v1")
            .join(format!("{}.json", first.draft.draft_id));
        let before = std::fs::read(&file).unwrap();
        intent.color_scheme = Some(OwnedPhoneColorScheme::Light);
        assert!(stage_with(&paths, &source, &intent, 101, |_, _| panic!(
            "different intent invoked native work"
        ))
        .is_err());
        assert_eq!(std::fs::read(file).unwrap(), before);
        assert_eq!(
            storage::read_state_bytes(&paths.state_file).unwrap(),
            source
        );
    }
    #[test]
    fn stale_profile_control_and_invalid_intent_fail_before_private_storage() {
        let (_temp, _lease, paths, source, intent) = fixture();
        let mut altered = intent.clone();
        altered.base_configuration_revision = 40;
        assert!(matches!(
            stage_with(&paths, &source, &altered, 100, |_, _| panic!()),
            Err(OwnedPhoneStageError::RevisionConflict {
                expected: 40,
                actual: 41
            })
        ));
        altered = intent.clone();
        altered.profile_id = Uuid::new_v4();
        assert!(matches!(
            stage_with(&paths, &source, &altered, 100, |_, _| panic!()),
            Err(OwnedPhoneStageError::MissingProfile)
        ));
        altered = intent.clone();
        altered.control_edits[0].control_id = Uuid::new_v4();
        assert!(matches!(
            stage_with(&paths, &source, &altered, 100, |_, _| panic!()),
            Err(OwnedPhoneStageError::MissingControl)
        ));
        altered = intent.clone();
        altered.control_edits[0].center_x = Some(2.0);
        assert!(stage_with(&paths, &source, &altered, 100, |_, _| panic!()).is_err());
        assert!(!paths.drafts_dir.exists());
        assert_eq!(
            storage::read_state_bytes(&paths.state_file).unwrap(),
            source
        );
    }
    #[test]
    fn removed_owner_and_same_profile_scalar_plans_have_no_binding_or_variant_fields() {
        let (_temp, _lease, _paths, source, mut intent) = fixture();
        let document = authority_document(&source).unwrap();
        intent.control_edits.clear();
        intent.removed_control_ids.push(
            document.profiles()[0]["customization"]["elements"][0]["id"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap(),
        );
        let ops = plan(&document, &intent).unwrap();
        assert_eq!(ops.len(), 2);
        for op in ops {
            let value = serde_json::to_value(op).unwrap();
            assert_eq!(value["variant"], "primary");
            assert_eq!(
                value["profileID"]
                    .as_str()
                    .unwrap()
                    .parse::<Uuid>()
                    .unwrap(),
                intent.profile_id
            );
            assert!(value.get("output").is_none());
            assert!(value.get("skinReference").is_none());
        }
        let candidate = apply_document(&source, &document).unwrap();
        assert_eq!(
            serde_json::from_slice::<Value>(&candidate).unwrap()["profiles"],
            serde_json::from_slice::<Value>(&source).unwrap()["profiles"]
        );
    }
}
