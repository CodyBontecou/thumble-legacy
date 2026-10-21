//! Scoped exact customization replacement. Profile identity and local authority
//! metadata never come from a reusable layout file.
use super::*;
use serde_json::value::RawValue;
use thumble_core::{CustomizationArtifact, OwnedConfigurationDocument};
use thumble_protocol::{merge_json_projection, JsonValueEdit};
type Object = BTreeMap<String, Box<RawValue>>;
const MAX: usize = 8 * 1024 * 1024;
fn invalid() -> TransactionFailure {
    TransactionFailure::new(
        "invalid_customization_artifact",
        "customization replacement is invalid or would discard opaque binding metadata",
    )
}
fn object(bytes: &[u8]) -> Result<Object, TransactionFailure> {
    serde_json::from_slice(bytes).map_err(|_| invalid())
}
fn raw<T: Serialize>(value: &T) -> Result<Box<RawValue>, TransactionFailure> {
    serde_json::value::to_raw_value(value).map_err(|_| invalid())
}
fn key(map: &Object, id: &str) -> Option<String> {
    map.keys().find(|k| k.eq_ignore_ascii_case(id)).cloned()
}
fn aligned(value: Value, original: &Object) -> Result<Vec<u8>, TransactionFailure> {
    let value = value.as_object().ok_or_else(invalid)?;
    let mut map = Object::new();
    for (id, v) in value {
        map.insert(key(original, id).unwrap_or_else(|| id.clone()), raw(v)?);
    }
    serde_json::to_vec(&map).map_err(|_| invalid())
}
fn merge_map<T: serde::de::DeserializeOwned + Serialize>(
    original: Option<&RawValue>,
    new: &T,
) -> Result<Box<RawValue>, TransactionFailure> {
    let text = original.map(|v| v.get()).unwrap_or("{}");
    let old = object(text.as_bytes())?;
    let before: T = serde_json::from_str(text).map_err(|_| invalid())?;
    let before = aligned(serde_json::to_value(before).map_err(|_| invalid())?, &old)?;
    let after = aligned(serde_json::to_value(new).map_err(|_| invalid())?, &old)?;
    let merged =
        merge_json_projection(text.as_bytes(), &before, &after, MAX).map_err(|_| invalid())?;
    RawValue::from_string(String::from_utf8(merged).map_err(|_| invalid())?).map_err(|_| invalid())
}
fn prepare(
    artifact: &CustomizationArtifact,
    invocation: Uuid,
) -> Result<OwnedConfigurationDocument, TransactionFailure> {
    if artifact.is_pooled() {
        return artifact.owned_document().map_err(|_| invalid());
    }
    let profile = artifact.synthetic_profile(false).map_err(|_| invalid())?;
    let value: Value = serde_json::from_slice(&profile).map_err(|_| invalid())?;
    let id = value["id"].as_str().ok_or_else(invalid)?;
    let envelope=format!("{{\"schema\":\"com.codybontecou.pocketpad.keypad-configuration\",\"version\":4,\"profiles\":[{}],\"activeProfileID\":\"{}\",\"defaultProfileID\":\"{}\"}}",std::str::from_utf8(&profile).map_err(|_|invalid())?,id,id);
    let bridge = crate::bridge::ConfigurationBridge::discover().map_err(|_| {
        TransactionFailure::new(
            "customization_preparation_required",
            "classic customization import requires the local native helper",
        )
    })?;
    let prepared = thumble_core::prepare_legacy_configuration(envelope.as_bytes(), |bytes| {
        bridge.prepare_portable_legacy_profile(bytes, None, invocation)
    })
    .map_err(|_| invalid())?;
    if prepared.reviews().iter().any(|v| {
        v.source_orientation != "landscape"
            || v.reasons
                .iter()
                .any(|s| s.contains("portrait") || s.contains("divergent"))
    }) {
        return Err(TransactionFailure::new(
            "customization_layout_review_required",
            "prepare and review portrait layouts explicitly before importing into an owned profile",
        ));
    }
    Ok(prepared.destination().clone())
}
pub(super) fn plan(
    source: &OwnedConfigurationDocument,
    command: &CliProfileCommand,
    invocation: Uuid,
    catalog: &CliProfileCatalog,
    now: i64,
) -> Result<Option<assets::Plan>, TransactionFailure> {
    let CliProfileCommand::CustomizationImport {
        target,
        variant,
        artifact_json,
    } = command
    else {
        return Ok(None);
    };
    if *variant == ConfigurationVariant::Portrait {
        return Err(TransactionFailure::new(
            "unsupported_orientation",
            "owned profiles have one landscape layout",
        ));
    }
    let selected = resolve_selector(target, catalog)?;
    let imported =
        CustomizationArtifact::decode(artifact_json.as_bytes()).map_err(|_| invalid())?;
    let incoming = prepare(&imported, invocation)?;
    let incoming_root = object(incoming.original_bytes())?;
    let incoming_profiles: Vec<Box<RawValue>> =
        serde_json::from_str(incoming_root["profiles"].get()).map_err(|_| invalid())?;
    let fields = object(incoming_profiles[0].get().as_bytes())?;
    let root = object(source.original_bytes())?;
    let profiles: Vec<Box<RawValue>> =
        serde_json::from_str(root["profiles"].get()).map_err(|_| invalid())?;
    let index = source
        .profiles()
        .iter()
        .position(|p| {
            p["id"].as_str().and_then(|s| Uuid::parse_str(s).ok()) == Some(selected.profile_id)
        })
        .ok_or_else(invalid)?;
    let original = object(profiles[index].get().as_bytes())?;
    let mut changed = Vec::new();
    for name in ["customization", "appearanceBank"] {
        let value = fields.get(name).ok_or_else(invalid)?;
        if original.get(name).map(|v| v.get()) != Some(value.get()) {
            changed.push((format!("/profiles/{index}/{name}"), value.clone()));
        }
    }
    // Keep old blobs for local launch data and opaque/future references.
    let mut pool = original
        .get("appearanceResources")
        .map(|v| object(v.get().as_bytes()))
        .transpose()?
        .unwrap_or_default();
    let mut pool_changed = !original.contains_key("appearanceResources");
    if let Some(incoming) = fields.get("appearanceResources") {
        for (digest, value) in object(incoming.get().as_bytes())? {
            if let Some(old) = pool.get(&digest) {
                if serde_json::from_str::<String>(old.get()).map_err(|_| invalid())?
                    != serde_json::from_str::<String>(value.get()).map_err(|_| invalid())?
                {
                    return Err(invalid());
                }
            } else {
                pool.insert(digest, value);
                pool_changed = true;
            }
        }
    }
    if let Some(launch) = original.get("launchTarget") {
        let launch = object(launch.get().as_bytes())?;
        for field in ["bookmarkData", "iconPNGData"] {
            if let Some(value) = launch.get(field) {
                if let Ok(text) = serde_json::from_str::<String>(value.get()) {
                    use base64::{engine::general_purpose::STANDARD, Engine};
                    let data = STANDARD.decode(&text).map_err(|_| invalid())?;
                    if STANDARD.encode(&data) != text {
                        return Err(invalid());
                    }
                    let digest = format!("{:x}", Sha256::digest(&data));
                    if !pool.contains_key(&digest) {
                        pool.insert(digest.clone(), raw(&text)?);
                        pool_changed = true;
                    }
                    changed.push((
                        format!("/profiles/{index}/launchTarget/{field}"),
                        raw(&serde_json::json!({"resourceSHA256":digest}))?,
                    ));
                }
            }
        }
    }
    if pool_changed {
        changed.push((
            format!("/profiles/{index}/appearanceResources"),
            raw(&pool)?,
        ));
    }
    if !original.contains_key("appearanceResourceVersion") {
        changed.push((
            format!("/profiles/{index}/appearanceResourceVersion"),
            raw(&1)?,
        ));
    }
    if changed.is_empty() {
        return Ok(Some(assets::Plan {
            document: source.clone(),
            names: vec![selected.name.clone()],
            outcome: crate::draft_operation::ConfigurationOperationOutcome {
                changed: false,
                changed_paths: vec![],
            },
        }));
    }
    // Layout files own executable declarations; rebuild only the target mirrors.
    let mut next_profile = original.clone();
    for name in ["customization", "appearanceBank"] {
        next_profile.insert(name.into(), fields[name].clone());
    }
    let profile_value = serde_json::to_value(&next_profile).map_err(|_| invalid())?;
    let outputs = thumble_core::profile_owned_outputs(&profile_value);
    let mut keys = ButtonBindings::<KeyBinding>::default();
    for (id, output) in outputs.iter_ids() {
        if let Some(binding) = &output.keyboard {
            keys.insert(id, binding.clone());
        }
    }
    for (name, is_output) in [
        ("profileKeyBindings", false),
        ("profileOutputBindings", true),
    ] {
        let mut maps = root
            .get(name)
            .map(|v| object(v.get().as_bytes()))
            .transpose()?
            .unwrap_or_default();
        let owner = key(&maps, &selected.profile_id.to_string())
            .unwrap_or_else(|| selected.profile_id.to_string());
        let value = if is_output {
            merge_map::<ButtonBindings<OutputBinding>>(
                maps.get(&owner).map(|v| v.as_ref()),
                &outputs,
            )?
        } else {
            merge_map::<ButtonBindings<KeyBinding>>(maps.get(&owner).map(|v| v.as_ref()), &keys)?
        };
        maps.insert(owner, value);
        changed.push((format!("/{name}"), raw(&maps)?));
    }
    if source
        .active_profile_id()
        .eq_ignore_ascii_case(&selected.profile_id.to_string())
    {
        changed.push((
            "/keyBindings".into(),
            merge_map::<ButtonBindings<KeyBinding>>(
                root.get("keyBindings").map(|v| v.as_ref()),
                &keys,
            )?,
        ));
        changed.push((
            "/outputBindings".into(),
            merge_map::<ButtonBindings<OutputBinding>>(
                root.get("outputBindings").map(|v| v.as_ref()),
                &outputs,
            )?,
        ));
    }
    changed.push((format!("/profiles/{index}/updatedAt"), raw(&now)?));
    let edits: Vec<_> = changed
        .iter()
        .map(|(pointer, value)| JsonValueEdit {
            pointer,
            value: Some(value.get().as_bytes()),
        })
        .collect();
    let document = source.with_edits(&edits).map_err(|_| invalid())?;
    Ok(Some(assets::Plan {
        document,
        names: vec![selected.name.clone()],
        outcome: crate::draft_operation::ConfigurationOperationOutcome {
            changed: true,
            changed_paths: vec![
                format!("/profiles/{}", selected.profile_id),
                "/profileKeyBindings".into(),
                "/profileOutputBindings".into(),
                "/keyBindings".into(),
                "/outputBindings".into(),
            ],
        },
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scoped_replacement_retains_exact_profiles_and_binding_extensions() {
        let source = OwnedConfigurationDocument::decode(include_bytes!(
            "../../../fixtures/owned-configuration/v1/document.json"
        ))
        .unwrap();
        let root = object(source.original_bytes()).unwrap();
        let profiles: Vec<Box<RawValue>> = serde_json::from_str(root["profiles"].get()).unwrap();
        let file =
            CustomizationArtifact::from_profile(profiles[0].get().as_bytes(), "primary", true)
                .unwrap();
        let mut maps = object(root["profileKeyBindings"].get().as_bytes()).unwrap();
        let id = source.profiles()[1]["id"].as_str().unwrap();
        maps.insert(id.into(),RawValue::from_string("{\"00000000-0000-0000-0000-000000000109\":{\"keyCode\":48,\"modifiersRawValue\":0,\"futurePrecision\":0.12345678901234567890123456789}}".into()).unwrap());
        let map_bytes = serde_json::to_vec(&maps).unwrap();
        let source = source
            .with_edits(&[JsonValueEdit {
                pointer: "/profileKeyBindings",
                value: Some(&map_bytes),
            }])
            .unwrap();
        let catalog = catalog_from_parts(
            source.profiles(),
            source.active_profile_id(),
            source.default_profile_id().unwrap(),
            41,
            std::iter::empty::<&str>(),
        )
        .unwrap();
        let command = CliProfileCommand::CustomizationImport {
            target: ProfileSelector::Id {
                profile_id: Uuid::parse_str(id).unwrap(),
            },
            variant: ConfigurationVariant::Landscape,
            artifact_json: String::from_utf8(file.original_bytes().to_vec()).unwrap(),
        };
        let changed = plan(&source, &command, Uuid::new_v4(), &catalog, 2000)
            .unwrap()
            .unwrap();
        assert!(changed.outcome.changed);
        let root_after = object(changed.document.original_bytes()).unwrap();
        let profiles_after: Vec<Box<RawValue>> =
            serde_json::from_str(root_after["profiles"].get()).unwrap();
        assert_eq!(profiles_after[0].get(), profiles[0].get());
        assert_eq!(root_after["keyBindings"].get(), root["keyBindings"].get());
        assert!(std::str::from_utf8(changed.document.original_bytes())
            .unwrap()
            .contains("0.12345678901234567890123456789"));
        let repeated = plan(&changed.document, &command, Uuid::new_v4(), &catalog, 3000)
            .unwrap()
            .unwrap();
        assert!(!repeated.outcome.changed);
        assert_eq!(
            repeated.document.original_bytes(),
            changed.document.original_bytes()
        );
    }
    #[test]
    fn active_replacement_updates_only_the_declared_primary_mirrors() {
        let source = OwnedConfigurationDocument::decode(include_bytes!(
            "../../../fixtures/owned-configuration/v1/document.json"
        ))
        .unwrap();
        let root = object(source.original_bytes()).unwrap();
        let profiles: Vec<Box<RawValue>> = serde_json::from_str(root["profiles"].get()).unwrap();
        let exported =
            CustomizationArtifact::from_profile(profiles[0].get().as_bytes(), "primary", true)
                .unwrap();
        let mut file: Value = serde_json::from_slice(exported.original_bytes()).unwrap();
        file["customization"]["elements"][0]["output"]["keyboard"]["keyCode"] =
            serde_json::json!(49);
        let catalog = catalog_from_parts(
            source.profiles(),
            source.active_profile_id(),
            source.default_profile_id().unwrap(),
            41,
            std::iter::empty::<&str>(),
        )
        .unwrap();
        let command = CliProfileCommand::CustomizationImport {
            target: ProfileSelector::Active,
            variant: ConfigurationVariant::Primary,
            artifact_json: serde_json::to_string(&file).unwrap(),
        };
        let after = plan(&source, &command, Uuid::new_v4(), &catalog, 2000)
            .unwrap()
            .unwrap();
        let root_after: Value = serde_json::from_slice(after.document.original_bytes()).unwrap();
        assert_eq!(
            root_after["keyBindings"]["00000000-0000-0000-0000-000000000109"]["keyCode"],
            49
        );
        assert_eq!(
            root_after["outputBindings"]["00000000-0000-0000-0000-000000000109"]["keyboard"]
                ["keyCode"],
            49
        );
        assert_eq!(after.document.profiles()[1], source.profiles()[1]);
    }
}
