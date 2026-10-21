//! Legacy inline libraries use the same local transaction/replay owner as owned assets.
use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::json;

pub(super) struct Plan {
    pub candidate: ConfigurationDocument,
    pub outcome: crate::draft_operation::ConfigurationOperationOutcome,
    pub profile_names: Vec<String>,
}

pub(super) fn plan(
    document: &ConfigurationDocument,
    command: &CliProfileCommand,
    invocation: Uuid,
    catalog: &CliProfileCatalog,
    now: i64,
) -> Result<Plan, TransactionFailure> {
    owned::validate_asset_request(command)?;
    let target = match command {
        CliProfileCommand::AssetImport { target, .. }
        | CliProfileCommand::AssetSet { target, .. }
        | CliProfileCommand::AssetRemove { target, .. } => target,
        _ => unreachable!(),
    };
    let selected = resolve_selector(target, catalog)?;
    let index = document
        .profiles
        .iter()
        .position(|p| {
            p["id"].as_str().and_then(|v| Uuid::parse_str(v).ok()) == Some(selected.profile_id)
        })
        .ok_or_else(|| TransactionFailure::new("profile_not_found", "asset profile is absent"))?;
    let mut candidate = document.clone();
    let profile = &mut candidate.profiles[index];
    let id = match command {
        CliProfileCommand::AssetImport { .. } => imported_asset_id(invocation).to_string(),
        CliProfileCommand::AssetSet { asset_id, .. }
        | CliProfileCommand::AssetRemove { asset_id, .. } => asset_id.clone(),
        _ => unreachable!(),
    };
    if matches!(command, CliProfileCommand::AssetRemove { .. })
        && owned::assets::referenced(profile, &id)
    {
        return Err(TransactionFailure::new(
            "asset_in_use",
            "asset is referenced by saved appearance; detach its uses before removal",
        ));
    }
    let imported = if let CliProfileCommand::AssetImport {
        name,
        file_name,
        content_type,
        data_base64,
        role,
        ..
    } = command
    {
        let bytes = STANDARD.decode(data_base64).map_err(|_| {
            TransactionFailure::new("invalid_asset_edit", "asset bytes are invalid")
        })?;
        let mut asset = serde_json::json!({"id":id,"name":name,"contentType":content_type,"role":role,"byteCount":bytes.len(),"hash":format!("{:x}",Sha256::digest(&bytes)),"data":data_base64});
        if let Some(name) = file_name {
            asset["fileName"] = json!(name);
        }
        Some(serde_json::value::to_raw_value(&asset).map_err(|_| {
            TransactionFailure::new("invalid_asset_edit", "asset could not be encoded")
        })?)
    } else {
        None
    };
    let mut changed = false;
    let mut found = false;
    for slot in [
        "customization",
        "landscapeCustomization",
        "portraitCustomization",
    ] {
        let Some(layout) = profile.get_mut(slot).filter(|v| v.is_object()) else {
            continue;
        };
        let bytes = layout
            .get("assetLibrary")
            .map(serde_json::value::to_raw_value)
            .transpose()
            .map_err(|_| {
                TransactionFailure::new("invalid_asset_edit", "asset library is invalid")
            })?;
        let (next, present, edited) =
            owned::assets::library(bytes.as_deref(), command, imported.as_deref(), &id)?;
        found |= present;
        if edited {
            layout["assetLibrary"] = serde_json::from_slice(&next).map_err(|_| {
                TransactionFailure::new("invalid_asset_edit", "asset library could not be decoded")
            })?;
            changed = true;
        }
    }
    if !matches!(command, CliProfileCommand::AssetImport { .. }) && !found {
        return Err(TransactionFailure::new(
            "asset_not_found",
            "asset is not present in the selected profile",
        ));
    }
    if changed {
        profile["updatedAt"] = json!(now.max(0));
    }
    candidate.validate().map_err(|_| {
        TransactionFailure::new(
            "invalid_asset_edit",
            "asset edit exceeds the configuration limits or violates saved references",
        )
    })?;
    Ok(Plan {
        candidate,
        outcome: crate::draft_operation::ConfigurationOperationOutcome {
            changed,
            changed_paths: if changed {
                vec![format!("/profiles/{}", selected.profile_id)]
            } else {
                vec![]
            },
        },
        profile_names: vec![selected.name.clone()],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, os::unix::fs::PermissionsExt};
    #[test]
    fn legacy_assets_commit_all_declared_slots_preserve_extensions_and_replay() {
        let directory = tempfile::tempdir().unwrap();
        let paths = HostPaths::new(
            directory.path().to_owned(),
            directory.path().join("control.sock"),
        );
        paths.ensure_state_dir().unwrap();
        let mut state = PersistentState::minimal("fixture-server").unwrap();
        let selected = state.active_profile_id.clone();
        for slot in ["landscapeCustomization", "portraitCustomization"] {
            state.profiles[0][slot] = state.profiles[0]["customization"].clone();
        }
        let original = state.profiles[0].clone();
        storage::save_atomic(&paths.state_file, &state).unwrap();
        let invocation = Uuid::new_v4();
        let asset_id = imported_asset_id(invocation).to_string();
        let mut request = CliProfileRequest {
            schema_version: CLI_PROFILE_SCHEMA_VERSION,
            invocation_id: Some(invocation),
            expected_configuration_revision: Some(1),
            command: CliProfileCommand::AssetImport {
                target: ProfileSelector::Id {
                    profile_id: Uuid::parse_str(&selected).unwrap(),
                },
                name: "Artwork".into(),
                file_name: Some("art.svg".into()),
                content_type: "image/svg+xml".into(),
                data_base64: STANDARD.encode(b"<svg/>"),
                role: "reference".into(),
            },
        };
        let result = execute_offline_authority(&paths, &request);
        assert!(result.ok, "{:?}", result.error);
        let saved = fs::read(&paths.state_file).unwrap();
        assert!(execute_offline_authority(&paths, &request).ok);
        assert_eq!(fs::read(&paths.state_file).unwrap(), saved);
        let mut value: Value = serde_json::from_slice(&saved).unwrap();
        for slot in [
            "customization",
            "landscapeCustomization",
            "portraitCustomization",
        ] {
            let entry = &mut value["profiles"][0][slot]["assetLibrary"]["assets"][0];
            assert_eq!(entry["data"], STANDARD.encode(b"<svg/>"));
            assert_eq!(entry["id"], asset_id);
            entry["futureExact"] = json!("PRECISE");
        }
        let precise = "0.123456789012345678901234567890123456789";
        fs::write(
            &paths.state_file,
            serde_json::to_string(&value)
                .unwrap()
                .replace("\"PRECISE\"", precise),
        )
        .unwrap();
        fs::set_permissions(&paths.state_file, fs::Permissions::from_mode(0o600)).unwrap();
        request.invocation_id = Some(Uuid::new_v4());
        request.expected_configuration_revision = Some(2);
        request.command = CliProfileCommand::AssetSet {
            target: ProfileSelector::Active,
            asset_id: asset_id.clone(),
            name: Some("Renamed".into()),
            role: Some("texture".into()),
        };
        let result = execute_offline_authority(&paths, &request);
        assert!(result.ok, "{:?}", result.error);
        let bytes = fs::read_to_string(&paths.state_file).unwrap();
        assert_eq!(bytes.matches(precise).count(), 3);
        let changed: Value = serde_json::from_str(&bytes).unwrap();
        for slot in [
            "customization",
            "landscapeCustomization",
            "portraitCustomization",
        ] {
            let mut layout = changed["profiles"][0][slot].clone();
            layout.as_object_mut().unwrap().remove("assetLibrary");
            let mut before = original[slot].clone();
            before.as_object_mut().unwrap().remove("assetLibrary");
            assert_eq!(layout, before);
        }
        request.invocation_id = Some(Uuid::new_v4());
        request.expected_configuration_revision = Some(3);
        let no_op = execute_offline_authority(&paths, &request);
        assert!(no_op.ok);
        assert!(!no_op.outcome.unwrap().changed);
        let mut value: Value =
            serde_json::from_slice(&fs::read(&paths.state_file).unwrap()).unwrap();
        value["profiles"][0]["futureReference"] = json!({"assetID":asset_id});
        fs::write(&paths.state_file, serde_json::to_vec(&value).unwrap()).unwrap();
        let before = fs::read(&paths.state_file).unwrap();
        request.invocation_id = Some(Uuid::new_v4());
        request.command = CliProfileCommand::AssetRemove {
            target: ProfileSelector::Active,
            asset_id,
        };
        let refused = execute_offline_authority(&paths, &request);
        assert!(!refused.ok);
        assert_eq!(refused.error.unwrap().code, "asset_in_use");
        assert_eq!(fs::read(&paths.state_file).unwrap(), before);
    }
}
