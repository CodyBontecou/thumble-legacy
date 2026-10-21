use super::tests::{fixture, load, request};
use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
fn import() -> CliProfileRequest {
    request(CliProfileCommand::AssetImport {
        target: ProfileSelector::Active,
        name: "Local art".into(),
        file_name: Some("art.png".into()),
        content_type: "image/png".into(),
        data_base64: STANDARD.encode(b"private fixture image"),
        role: "icon".into(),
    })
}
fn libraries(profile: &Value) -> Vec<&Value> {
    std::iter::once(&profile["customization"]["assetLibrary"])
        .chain(
            profile["appearanceBank"]["variants"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| &v["appearance"]["assetLibrary"]),
        )
        .collect()
}
fn contains(library: &Value, id: &str) -> bool {
    library["assets"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["id"].as_str() == Some(id))
}
#[test]
fn import_pools_one_blob_across_all_saved_states_and_replays_without_native_helper() {
    let (_temp, _lease, paths) = fixture();
    let before = load(&paths);
    let req = import();
    let id = imported_asset_id(req.invocation_id.unwrap()).to_string();
    let response = execute_offline_authority(&paths, &req);
    assert!(response.ok, "{:?}", response.error);
    let after = load(&paths);
    assert_eq!(after.configuration_revision, 42);
    let a = &after.profiles[0];
    let b = &before.profiles[0];
    assert!(libraries(a).iter().all(|v| contains(v, &id)));
    assert_eq!(
        a["appearanceResources"].as_object().unwrap().len(),
        b["appearanceResources"].as_object().unwrap().len() + 1
    );
    let mut expected = b.clone();
    expected["customization"]["assetLibrary"] = a["customization"]["assetLibrary"].clone();
    for (old, new) in expected["appearanceBank"]["variants"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .zip(a["appearanceBank"]["variants"].as_array().unwrap())
    {
        old["appearance"]["assetLibrary"] = new["appearance"]["assetLibrary"].clone();
    }
    expected["appearanceResources"] = a["appearanceResources"].clone();
    expected["appearanceResourceVersion"] = a["appearanceResourceVersion"].clone();
    expected["updatedAt"] = a["updatedAt"].clone();
    assert_eq!(a, &expected);
    assert_eq!(after.profiles[1], before.profiles[1]);
    assert_eq!(after.key_bindings, before.key_bindings);
    assert_eq!(after.output_bindings, before.output_bindings);
    let saved = storage::read_state_bytes(&paths.state_file).unwrap();
    assert!(std::str::from_utf8(&saved)
        .unwrap()
        .contains("0.12345678901234567890123456789"));
    let replay = execute_offline_authority(&paths, &req);
    assert!(replay.ok);
    assert!(replay.outcome.unwrap().idempotent_replay);
    assert_eq!(storage::read_state_bytes(&paths.state_file).unwrap(), saved);
    let mut changed = req;
    if let CliProfileCommand::AssetImport { name, .. } = &mut changed.command {
        *name = "Changed request".into();
    }
    assert_eq!(
        execute_offline_authority(&paths, &changed)
            .error
            .unwrap()
            .code,
        "commit_id_conflict"
    );
    assert_eq!(storage::read_state_bytes(&paths.state_file).unwrap(), saved);
}
#[test]
fn metadata_and_unused_removal_reconcile_definitions_without_touching_paint_or_resources() {
    let (_temp, _lease, paths) = fixture();
    let req = import();
    let id = imported_asset_id(req.invocation_id.unwrap()).to_string();
    assert!(execute_offline_authority(&paths, &req).ok);
    let imported = load(&paths);
    let mut set = request(CliProfileCommand::AssetSet {
        target: ProfileSelector::Active,
        asset_id: id.clone(),
        name: Some("Renamed art".into()),
        role: Some("texture".into()),
    });
    set.expected_configuration_revision = Some(42);
    assert!(execute_offline_authority(&paths, &set).ok);
    let changed = load(&paths);
    for library in libraries(&changed.profiles[0]) {
        let entry = library["assets"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["id"] == id)
            .unwrap();
        assert_eq!(entry["name"], "Renamed art");
        assert_eq!(entry["role"], "texture");
    }
    let mut noop = set.clone();
    noop.invocation_id = Some(Uuid::new_v4());
    noop.expected_configuration_revision = Some(43);
    let outcome = execute_offline_authority(&paths, &noop).outcome.unwrap();
    assert!(!outcome.changed);
    assert_eq!(outcome.configuration_revision, 43);
    let mut remove = request(CliProfileCommand::AssetRemove {
        target: ProfileSelector::Active,
        asset_id: id.clone(),
    });
    remove.expected_configuration_revision = Some(43);
    assert!(execute_offline_authority(&paths, &remove).ok);
    let after = load(&paths);
    assert!(libraries(&after.profiles[0])
        .iter()
        .all(|v| !contains(v, &id)));
    assert_eq!(
        after.profiles[0]["appearanceResources"],
        imported.profiles[0]["appearanceResources"]
    );
    let before = storage::read_state_bytes(&paths.state_file).unwrap();
    let mut used = request(CliProfileCommand::AssetRemove {
        target: ProfileSelector::Active,
        asset_id: "one".into(),
    });
    used.expected_configuration_revision = Some(44);
    assert_eq!(
        execute_offline_authority(&paths, &used).error.unwrap().code,
        "asset_in_use"
    );
    assert_eq!(
        storage::read_state_bytes(&paths.state_file).unwrap(),
        before
    );
}
#[test]
fn failed_save_preserves_original_and_resumes_the_prepared_asset_edit_once() {
    let (_temp, _lease, paths) = fixture();
    let before = storage::read_state_bytes(&paths.state_file).unwrap();
    let req = import();
    let state = load(&paths);
    let response = execute_profile_transaction(&paths, &state, &req, "test", |_, _, _, _, _| {
        Err(TransactionFailure::new("test_refused", "test"))
    });
    assert!(!response.ok);
    assert_eq!(
        storage::read_state_bytes(&paths.state_file).unwrap(),
        before
    );
    assert!(execute_offline_authority(&paths, &req).ok);
    assert_eq!(load(&paths).configuration_revision, 42);
}
#[test]
fn malformed_asset_requests_and_stale_revisions_never_create_private_drafts() {
    let (_temp, _lease, paths) = fixture();
    let before = storage::read_state_bytes(&paths.state_file).unwrap();
    for (name, field) in [
        ("../private/file", "file"),
        ("unknown", "role"),
        ("not-base64", "bytes"),
    ] {
        let mut req = import();
        if let CliProfileCommand::AssetImport {
            file_name,
            role,
            data_base64,
            ..
        } = &mut req.command
        {
            match field {
                "file" => *file_name = Some(name.into()),
                "role" => *role = name.into(),
                _ => *data_base64 = name.into(),
            }
        }
        assert_eq!(
            execute_offline_authority(&paths, &req).error.unwrap().code,
            "invalid_asset_edit"
        );
        assert!(!paths.drafts_dir.exists());
    }
    let mut stale = import();
    stale.expected_configuration_revision = Some(40);
    assert_eq!(
        execute_offline_authority(&paths, &stale)
            .error
            .unwrap()
            .code,
        "configuration_revision_conflict"
    );
    assert!(!paths.drafts_dir.exists());
    assert_eq!(
        storage::read_state_bytes(&paths.state_file).unwrap(),
        before
    );
}

#[test]
fn asset_mutation_never_initializes_an_absent_authority() {
    let temp = tempfile::tempdir().unwrap();
    let paths = HostPaths::new(temp.path().join("absent"), temp.path().join("control.sock"));
    let response = execute_offline_authority(&paths, &import());
    assert_eq!(response.error.unwrap().code, "owned_configuration_required");
    assert!(!paths.state_dir.exists());
}
#[test]
fn metadata_edit_preserves_exact_future_asset_fields() {
    let (_temp, _lease, paths) = fixture();
    let source = storage::read_state_bytes(&paths.state_file).unwrap();
    let updated = thumble_protocol::edit_json_values(
        &source,
        &[thumble_protocol::JsonValueEdit {
            pointer: "/profiles/0/customization/assetLibrary/assets/0/futureAsset",
            value: Some(b"0.12345678901234567890123456789"),
        }],
        16 * 1024 * 1024,
    )
    .unwrap();
    storage::install_private_state_bytes(&paths.state_file, &updated, Some(&source), || Ok(()))
        .unwrap();
    let req = request(CliProfileCommand::AssetSet {
        target: ProfileSelector::Active,
        asset_id: "one".into(),
        name: Some("Renamed one".into()),
        role: None,
    });
    let response = execute_offline_authority(&paths, &req);
    assert!(response.ok, "{:?}", response.error);
    let raw = String::from_utf8(storage::read_state_bytes(&paths.state_file).unwrap()).unwrap();
    assert!(raw.contains("\"futureAsset\":0.12345678901234567890123456789"));
}

#[test]
fn frozen_asset_flow_matches_typed_cli_requests_and_owned_commits() {
    let (_temp, _lease, paths) = fixture();
    let requests: Vec<CliProfileRequest> = thumble_protocol::decode_unique_json(include_bytes!(
        "../../../fixtures/owned-assets/v1/requests.json"
    ))
    .unwrap();
    assert_eq!(requests.len(), 3);
    for (ordinal, request) in requests.iter().enumerate() {
        let response = execute_offline_authority(&paths, request);
        assert!(response.ok, "{:?}", response.error);
        assert_eq!(
            response.outcome.unwrap().configuration_revision,
            42 + ordinal as u64
        );
    }
    assert_eq!(
        load(&paths).profiles[0]["appearanceBank"]["variants"]
            .as_array()
            .unwrap()
            .len(),
        8
    );
}

#[test]
fn maximum_asset_remains_pooled_through_metadata_update_and_removal() {
    let (_temp, _lease, paths) = fixture();
    let mut req = import();
    if let CliProfileCommand::AssetImport { data_base64, .. } = &mut req.command {
        *data_base64 = STANDARD.encode(vec![42; 2_500_000]);
    }
    let id = imported_asset_id(req.invocation_id.unwrap()).to_string();
    let result = execute_offline_authority(&paths, &req);
    assert!(result.ok, "{:?}", result.error);
    let mut set = request(CliProfileCommand::AssetSet {
        target: ProfileSelector::Active,
        asset_id: id.clone(),
        name: Some("Large asset".into()),
        role: None,
    });
    set.expected_configuration_revision = Some(42);
    let result = execute_offline_authority(&paths, &set);
    assert!(result.ok, "{:?}", result.error);
    let mut remove = request(CliProfileCommand::AssetRemove {
        target: ProfileSelector::Active,
        asset_id: id,
    });
    remove.expected_configuration_revision = Some(43);
    let result = execute_offline_authority(&paths, &remove);
    assert!(result.ok, "{:?}", result.error);
}
