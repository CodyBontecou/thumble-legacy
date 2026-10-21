use super::tests::{fixture, load, request};
use super::*;
use thumble_core::{OwnedProfileArtifact, ProfileArtifactSelection};
use thumble_protocol::JsonValueEdit;
fn artifact(paths: &HostPaths) -> String {
    let source =
        authority_document(&storage::read_state_bytes(&paths.state_file).unwrap()).unwrap();
    let source = source
        .with_edits(&[
            JsonValueEdit {
                pointer: "/profiles/0/name",
                value: Some(br#""Imported""#),
            },
            JsonValueEdit {
                pointer: "/futureDocument",
                value: Some(
                    br#"{"integer":9007199254740993,"fraction":0.12345678901234567890123456789}"#,
                ),
            },
        ])
        .unwrap();
    String::from_utf8(
        OwnedProfileArtifact::from_configuration(&source, ProfileArtifactSelection::All, 123)
            .unwrap()
            .encode_json()
            .unwrap(),
    )
    .unwrap()
}
fn import(json: String, append: bool) -> CliProfileRequest {
    request(CliProfileCommand::Import {
        artifact_json: json,
        append_as_copies: append,
        select: true,
        make_default: true,
    })
}
#[test]
fn replacement_preserves_local_launch_paint_bindings_and_exact_extensions_across_restart() {
    let (_temp, _lease, paths) = fixture();
    let before = storage::read_state_bytes(&paths.state_file).unwrap();
    let original = authority_document(&before).unwrap();
    let req = import(artifact(&paths), false);
    let response = execute_offline_authority(&paths, &req);
    assert!(response.ok, "{:?}", response.error);
    assert_eq!(
        response.outcome.unwrap().profile_names,
        vec!["Imported", "Arcade Stick"]
    );
    let saved = storage::read_state_bytes(&paths.state_file).unwrap();
    let after = authority_document(&saved).unwrap();
    assert_eq!(load(&paths).configuration_revision, 42);
    assert_eq!(after.profiles().len(), 2);
    for (a, b) in after.profiles().iter().zip(original.profiles()) {
        for key in [
            "launchTarget",
            "customization",
            "appearanceBank",
            "appearanceResources",
        ] {
            assert_eq!(a[key], b[key], "{key}");
        }
    }
    let text = std::str::from_utf8(&saved).unwrap();
    assert!(text.contains("9007199254740993"));
    assert!(text.contains("0.12345678901234567890123456789"));
    assert!(text.contains("ownedConfigurationExtensions"));
    let replay = execute_offline_authority(&paths, &req);
    assert!(replay.ok);
    assert!(replay.outcome.unwrap().idempotent_replay);
    assert_eq!(storage::read_state_bytes(&paths.state_file).unwrap(), saved);
    let exported =
        execute_offline_authority(&paths, &request(CliProfileCommand::Export { target: None }));
    assert!(exported.ok, "{:?}", exported.error);
    let exported =
        OwnedProfileArtifact::decode_json(exported.artifact.unwrap().artifact_json.as_bytes())
            .unwrap();
    let root: BTreeMap<String, &RawValue> =
        serde_json::from_slice(exported.document().original_bytes()).unwrap();
    assert_eq!(
        root["futureDocument"].get(),
        r#"{"integer":9007199254740993,"fraction":0.12345678901234567890123456789}"#
    );
    let mut rename = request(CliProfileCommand::Rename {
        target: ProfileSelector::Active,
        name: "After restart".into(),
    });
    rename.expected_configuration_revision = Some(42);
    let result = execute_offline_authority(&paths, &rename);
    assert!(result.ok, "{:?}", result.error);
    let restarted =
        authority_document(&storage::read_state_bytes(&paths.state_file).unwrap()).unwrap();
    assert!(std::str::from_utf8(restarted.original_bytes())
        .unwrap()
        .contains(root["futureDocument"].get()));
}
#[test]
fn append_copy_ids_selection_and_names_are_deterministic_and_replays_do_not_duplicate() {
    let (_temp, _lease, paths) = fixture();
    let req = import(artifact(&paths), true);
    let response = execute_offline_authority(&paths, &req);
    assert!(response.ok, "{:?}", response.error);
    let outcome = response.outcome.unwrap();
    assert_eq!(outcome.profile_names, vec!["Imported", "Arcade Stick 2"]);
    let saved = storage::read_state_bytes(&paths.state_file).unwrap();
    let state = load(&paths);
    assert_eq!(state.profiles.len(), 4);
    let expected = deterministic_uuid(
        req.invocation_id.unwrap(),
        "profile.import:append:ea000000-0000-0000-0000-000000000001:0",
    );
    assert_eq!(
        state.active_profile_id.to_ascii_lowercase(),
        expected.to_string()
    );
    assert_eq!(
        state.default_profile_id.to_ascii_lowercase(),
        deterministic_uuid(
            req.invocation_id.unwrap(),
            "profile.import:append:0f38b980-9d36-56b6-8425-826ea40b149d:1"
        )
        .to_string()
    );
    assert!(state.profiles[2].get("launchTarget").is_none());
    let response = execute_offline_authority(&paths, &req);
    assert!(response.ok);
    assert_eq!(
        response.outcome.unwrap().profile_names,
        outcome.profile_names
    );
    assert_eq!(storage::read_state_bytes(&paths.state_file).unwrap(), saved);
    let mut conflict = req.clone();
    if let CliProfileCommand::Import { select, .. } = &mut conflict.command {
        *select = false;
    }
    assert_eq!(
        execute_offline_authority(&paths, &conflict)
            .error
            .unwrap()
            .code,
        "commit_id_conflict"
    );
    assert_eq!(storage::read_state_bytes(&paths.state_file).unwrap(), saved);
}
#[test]
fn failed_save_reuses_prepared_import_without_mutating_original_or_adding_another_copy() {
    let (_temp, _lease, paths) = fixture();
    let before = storage::read_state_bytes(&paths.state_file).unwrap();
    let req = import(artifact(&paths), true);
    let state = load(&paths);
    let failed = execute_profile_transaction(&paths, &state, &req, "test", |_, _, _, _, _| {
        Err(TransactionFailure::new("test_save_refused", "test"))
    });
    assert!(!failed.ok);
    assert_eq!(
        storage::read_state_bytes(&paths.state_file).unwrap(),
        before
    );
    let response = execute_offline_authority(&paths, &req);
    assert!(response.ok, "{:?}", response.error);
    assert_eq!(load(&paths).profiles.len(), 4);
    assert_eq!(load(&paths).configuration_revision, 42);
}
#[test]
fn tampered_import_never_initializes_an_absent_authority() {
    let (_temp, _lease, source) = fixture();
    let bytes = artifact(&source);
    let mut wire: serde_json::Value = serde_json::from_str(&bytes).unwrap();
    wire["contentHash"]["value"] = serde_json::json!("0".repeat(64));
    let temp = tempfile::tempdir().unwrap();
    let paths = HostPaths::new(temp.path().join("absent"), temp.path().join("control.sock"));
    let response = execute_offline_authority(
        &paths,
        &import(serde_json::to_string(&wire).unwrap(), false),
    );
    assert!(!response.ok);
    assert!(!paths.state_file.exists());
    assert!(!paths.state_dir.exists());
}
#[test]
fn stale_import_does_not_change_profiles_or_metadata() {
    let (_temp, _lease, paths) = fixture();
    let before = storage::read_state_bytes(&paths.state_file).unwrap();
    let mut req = import(artifact(&paths), false);
    req.expected_configuration_revision = Some(40);
    assert_eq!(
        execute_offline_authority(&paths, &req).error.unwrap().code,
        "configuration_revision_conflict"
    );
    assert_eq!(
        storage::read_state_bytes(&paths.state_file).unwrap(),
        before
    );
}
#[test]
fn extension_storage_refuses_envelope_collisions_and_preserves_other_authority_fields() {
    let (_temp, _lease, paths) = fixture();
    let before = storage::read_state_bytes(&paths.state_file).unwrap();
    let candidate = thumble_protocol::edit_json_values(
        &before,
        &[JsonValueEdit {
            pointer: "/ownedConfigurationExtensions",
            value: Some(br#"{"profiles":[]}"#),
        }],
        16 * 1024 * 1024,
    )
    .unwrap();
    assert!(authority_document(&candidate).is_err());
    let document = authority_document(&before)
        .unwrap()
        .with_edits(&[JsonValueEdit {
            pointer: "/futureRoot",
            value: Some(br#"9007199254740993"#),
        }])
        .unwrap();
    let after = apply_document(&before, &document).unwrap();
    let a: BTreeMap<String, &RawValue> = serde_json::from_slice(&after).unwrap();
    let b: BTreeMap<String, &RawValue> = serde_json::from_slice(&before).unwrap();
    for (key, value) in b {
        assert_eq!(a[&key].get(), value.get(), "{key}");
    }
    assert!(
        std::str::from_utf8(authority_document(&after).unwrap().original_bytes())
            .unwrap()
            .contains("9007199254740993")
    );
}

#[test]
fn valid_owned_import_requires_an_existing_owned_authority_without_initializing_legacy_state() {
    let (_temp, _lease, source) = fixture();
    let req = import(artifact(&source), false);
    let temp = tempfile::tempdir().unwrap();
    let paths = HostPaths::new(temp.path().join("absent"), temp.path().join("control.sock"));
    assert_eq!(
        execute_offline_authority(&paths, &req).error.unwrap().code,
        "owned_authority_required"
    );
    assert!(!paths.state_dir.exists());
}
