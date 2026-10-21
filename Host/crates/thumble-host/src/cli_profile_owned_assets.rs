//! Typed asset edits on raw owned documents; never decode/re-encode paint.
use super::*;
use crate::draft_operation::ConfigurationOperationOutcome;
use base64::{engine::general_purpose::STANDARD, Engine};
use thumble_protocol::JsonValueEdit;
type Object = BTreeMap<String, Box<RawValue>>;
const MAXIMUM_BYTES: usize = 2_500_000;
fn invalid() -> TransactionFailure {
    TransactionFailure::new(
        "invalid_asset_edit",
        "asset edit is malformed or outside supported bounds",
    )
}
fn object(bytes: &[u8]) -> Result<Object, TransactionFailure> {
    serde_json::from_slice(bytes).map_err(|_| invalid())
}
fn raw<T: Serialize>(value: &T) -> Result<Box<RawValue>, TransactionFailure> {
    serde_json::value::to_raw_value(value).map_err(|_| invalid())
}
fn text(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.trim() == value
        && value.chars().count() <= maximum
        && !value.chars().any(char::is_control)
}
fn identifier(value: &str) -> bool {
    text(value, 128)
        && !value.starts_with(['.', '-'])
        && !value.ends_with(['.', '-'])
        && value
            .chars()
            .all(|c| c.is_alphanumeric() || "-_.".contains(c))
}
fn role(value: &str) -> bool {
    matches!(value, "background" | "icon" | "texture" | "reference")
}
pub(super) fn is_asset(command: &CliProfileCommand) -> bool {
    matches!(
        command,
        CliProfileCommand::AssetImport { .. }
            | CliProfileCommand::AssetSet { .. }
            | CliProfileCommand::AssetRemove { .. }
    )
}
pub(super) fn validate(command: &CliProfileCommand) -> Result<(), TransactionFailure> {
    match command {
        CliProfileCommand::AssetImport {
            name,
            file_name,
            content_type,
            data_base64,
            role: value,
            ..
        } => {
            if !text(name, 64)
                || !text(content_type, 80)
                || !content_type.is_ascii()
                || !role(value)
                || file_name.as_ref().is_some_and(|v| {
                    !text(v, 255) || v.contains(['/', '\\']) || matches!(v.as_str(), "." | "..")
                })
                || data_base64.len() > MAXIMUM_BYTES.div_ceil(3) * 4
            {
                return Err(invalid());
            }
            let bytes = STANDARD.decode(data_base64).map_err(|_| invalid())?;
            if bytes.len() > MAXIMUM_BYTES || STANDARD.encode(&bytes) != *data_base64 {
                return Err(invalid());
            }
        }
        CliProfileCommand::AssetSet {
            asset_id,
            name,
            role: value,
            ..
        } => {
            if !identifier(asset_id)
                || name.is_none() && value.is_none()
                || name.as_ref().is_some_and(|v| !text(v, 64))
                || value.as_ref().is_some_and(|v| !role(v))
            {
                return Err(invalid());
            }
        }
        CliProfileCommand::AssetRemove { asset_id, .. } if !identifier(asset_id) => {
            return Err(invalid())
        }
        _ => {}
    }
    Ok(())
}
pub(super) struct Plan {
    pub document: OwnedConfigurationDocument,
    pub outcome: ConfigurationOperationOutcome,
    pub names: Vec<String>,
}
fn referenced(profile: &Value, id: &str) -> bool {
    let mut pending = vec![profile];
    while let Some(node) = pending.pop() {
        if node.get("assetID").and_then(Value::as_str) == Some(id)
            || node.get("source").and_then(Value::as_str) == Some("asset")
                && node.get("value").and_then(Value::as_str) == Some(id)
        {
            return true;
        }
        match node {
            Value::Object(fields) => pending.extend(fields.values()),
            Value::Array(items) => pending.extend(items),
            _ => {}
        }
    }
    false
}
fn library(
    bytes: Option<&RawValue>,
    command: &CliProfileCommand,
    imported: Option<&RawValue>,
    id: &str,
) -> Result<(Vec<u8>, bool, bool), TransactionFailure> {
    let mut value = bytes
        .map(|v| object(v.get().as_bytes()))
        .transpose()?
        .unwrap_or_default();
    let mut assets: Vec<Box<RawValue>> = value
        .get("assets")
        .map(|v| serde_json::from_str(v.get()).map_err(|_| invalid()))
        .transpose()?
        .unwrap_or_default();
    let index = assets.iter().position(|v| {
        serde_json::from_str::<Value>(v.get())
            .ok()
            .is_some_and(|v| v["id"].as_str() == Some(id))
    });
    let found = index.is_some();
    let mut changed = false;
    match command {
        CliProfileCommand::AssetImport { .. } => {
            if found {
                return Err(TransactionFailure::new(
                    "asset_id_conflict",
                    "asset ID is already present",
                ));
            }
            if assets.len() >= 512 {
                return Err(TransactionFailure::new(
                    "asset_limit",
                    "profile asset library is full",
                ));
            }
            assets.push(raw(&imported.ok_or_else(invalid)?)?);
            changed = true;
        }
        CliProfileCommand::AssetSet { name, role, .. } => {
            if let Some(index) = index {
                let mut entry = object(assets[index].get().as_bytes())?;
                for (key, requested) in [("name", name), ("role", role)] {
                    if let Some(requested) = requested {
                        if entry
                            .get(key)
                            .and_then(|v| serde_json::from_str::<String>(v.get()).ok())
                            .as_ref()
                            != Some(requested)
                        {
                            entry.insert(key.into(), raw(requested)?);
                            changed = true;
                        }
                    }
                }
                if changed {
                    assets[index] = raw(&entry)?;
                }
            }
        }
        CliProfileCommand::AssetRemove { .. } => {
            if let Some(index) = index {
                assets.remove(index);
                changed = true;
            }
        }
        _ => return Err(invalid()),
    }
    value.insert("assets".into(), raw(&assets)?);
    Ok((
        serde_json::to_vec(&value).map_err(|_| invalid())?,
        found,
        changed,
    ))
}
/// Materialize a plain profile with the retained read-only native resolver.
/// Only appearance fields enter the original document; authority and output data
/// never cross the helper boundary or get replaced by a legacy projection.
fn prepare_plain_appearance(
    source: &OwnedConfigurationDocument,
    index: usize,
    invocation: Uuid,
) -> Result<OwnedConfigurationDocument, TransactionFailure> {
    let failure = || {
        TransactionFailure::new(
            "asset_appearance_preparation_required",
            "plain appearance requires the local native helper; no asset was committed",
        )
    };
    let root = object(source.original_bytes())?;
    let profiles: Vec<Box<RawValue>> =
        serde_json::from_str(root["profiles"].get()).map_err(|_| failure())?;
    let mut profile = object(profiles[index].get().as_bytes())?;
    profile.remove("profileFormatVersion");
    let id = profile.get("id").ok_or_else(failure)?.get();
    let envelope = format!(
        "{{\"schema\":\"com.codybontecou.pocketpad.keypad-configuration\",\"version\":4,\"profiles\":[{}],\"activeProfileID\":{},\"defaultProfileID\":{}}}",
        serde_json::to_string(&profile).map_err(|_| failure())?, id, id
    );
    let bridge = crate::bridge::ConfigurationBridge::discover().map_err(|_| failure())?;
    let prepared = thumble_core::prepare_legacy_configuration(envelope.as_bytes(), |bytes| {
        bridge.prepare_portable_legacy_profile(bytes, None, invocation)
    })
    .map_err(|_| failure())?;
    let prepared_root = object(prepared.destination().original_bytes())?;
    let profiles: Vec<Box<RawValue>> =
        serde_json::from_str(prepared_root["profiles"].get()).map_err(|_| failure())?;
    let prepared_profile = object(profiles[0].get().as_bytes())?;
    let mut fields = Vec::new();
    for name in [
        "customization",
        "appearanceBank",
        "appearanceResources",
        "appearanceResourceVersion",
    ] {
        if let Some(value) = prepared_profile.get(name) {
            fields.push((format!("/profiles/{index}/{name}"), value.get().as_bytes()));
        }
    }
    let edits: Vec<_> = fields
        .iter()
        .map(|(pointer, bytes)| JsonValueEdit {
            pointer,
            value: Some(bytes),
        })
        .collect();
    source.with_edits(&edits).map_err(|_| failure())
}

pub(super) fn plan(
    source: &OwnedConfigurationDocument,
    command: &CliProfileCommand,
    invocation: Uuid,
    catalog: &CliProfileCatalog,
    now: i64,
) -> Result<Option<Plan>, TransactionFailure> {
    let target = match command {
        CliProfileCommand::AssetImport { target, .. }
        | CliProfileCommand::AssetSet { target, .. }
        | CliProfileCommand::AssetRemove { target, .. } => target,
        _ => return Ok(None),
    };
    validate(command)?;
    let selected = resolve_selector(target, catalog)?;
    let index = source
        .profiles()
        .iter()
        .position(|v| {
            v["id"].as_str().and_then(|v| Uuid::parse_str(v).ok()) == Some(selected.profile_id)
        })
        .ok_or_else(invalid)?;
    let profile = &source.profiles()[index];
    if matches!(command, CliProfileCommand::AssetImport { .. })
        && profile.get("appearanceBank").is_none()
    {
        let prepared = prepare_plain_appearance(source, index, invocation)?;
        if prepared.profiles()[index].get("appearanceBank").is_none() {
            return Err(invalid());
        }
        return plan(&prepared, command, invocation, catalog, now);
    }
    let id = match command {
        CliProfileCommand::AssetImport { .. } => imported_asset_id(invocation).to_string(),
        CliProfileCommand::AssetSet { asset_id, .. }
        | CliProfileCommand::AssetRemove { asset_id, .. } => asset_id.clone(),
        _ => unreachable!(),
    };
    if matches!(command, CliProfileCommand::AssetRemove { .. }) && referenced(profile, &id) {
        return Err(TransactionFailure::new(
            "asset_in_use",
            "asset is referenced by saved appearance; detach its uses before removal",
        ));
    }
    let root = object(source.original_bytes())?;
    let profiles: Vec<Box<RawValue>> =
        serde_json::from_str(root["profiles"].get()).map_err(|_| invalid())?;
    let raw_profile = object(profiles[index].get().as_bytes())?;
    let customization = object(raw_profile["customization"].get().as_bytes())?;
    let mut edits: Vec<(String, Vec<u8>)> = vec![];
    let mut imported = None;
    if let CliProfileCommand::AssetImport {
        name,
        file_name,
        content_type,
        data_base64,
        role,
        ..
    } = command
    {
        let bytes = STANDARD.decode(data_base64).map_err(|_| invalid())?;
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let mut asset = serde_json::json!({"id":id,"name":name,"contentType":content_type,"role":role,"byteCount":bytes.len(),"hash":digest,"data":{"resourceSHA256":digest}});
        if let Some(name) = file_name {
            asset["fileName"] = Value::String(name.clone());
        }
        imported = Some(raw(&asset)?);
        let mut resources: Object = raw_profile
            .get("appearanceResources")
            .map(|v| object(v.get().as_bytes()))
            .transpose()?
            .unwrap_or_default();
        if let Some(existing) = resources.get(&digest) {
            if serde_json::from_str::<String>(existing.get()).map_err(|_| invalid())?
                != *data_base64
            {
                return Err(invalid());
            }
        } else {
            resources.insert(digest, raw(data_base64)?);
        }
        edits.push((
            format!("/profiles/{index}/appearanceResources"),
            serde_json::to_vec(&resources).map_err(|_| invalid())?,
        ));
        edits.push((
            format!("/profiles/{index}/appearanceResourceVersion"),
            b"1".to_vec(),
        ));
    }
    let (value, mut found, changed) = library(
        customization.get("assetLibrary").map(|v| v.as_ref()),
        command,
        imported.as_deref(),
        &id,
    )?;
    if changed {
        edits.push((
            format!("/profiles/{index}/customization/assetLibrary"),
            value,
        ));
    }
    if let Some(bank) = raw_profile.get("appearanceBank") {
        let bank = object(bank.get().as_bytes())?;
        let variants: Vec<Box<RawValue>> =
            serde_json::from_str(bank["variants"].get()).map_err(|_| invalid())?;
        for (ordinal, variant) in variants.iter().enumerate() {
            let variant = object(variant.get().as_bytes())?;
            let appearance = object(variant["appearance"].get().as_bytes())?;
            let (value, present, changed) = library(
                appearance.get("assetLibrary").map(|v| v.as_ref()),
                command,
                imported.as_deref(),
                &id,
            )?;
            found |= present;
            if changed {
                edits.push((format!("/profiles/{index}/appearanceBank/variants/{ordinal}/appearance/assetLibrary"), value));
            }
        }
    }
    if !matches!(command, CliProfileCommand::AssetImport { .. }) && !found {
        return Err(TransactionFailure::new(
            "asset_not_found",
            "asset is not present in the selected profile",
        ));
    }
    let changed = !edits.is_empty();
    if changed {
        edits.push((
            format!("/profiles/{index}/updatedAt"),
            now.max(0).to_string().into_bytes(),
        ));
    }
    let patch: Vec<_> = edits
        .iter()
        .map(|(pointer, bytes)| JsonValueEdit {
            pointer,
            value: Some(bytes),
        })
        .collect();
    let document = source.with_edits(&patch).map_err(|_| {
        TransactionFailure::new(
            "invalid_asset_result",
            "asset edit would exceed bounds or invalidate owned appearance",
        )
    })?;
    Ok(Some(Plan {
        document,
        outcome: ConfigurationOperationOutcome {
            changed,
            changed_paths: if changed {
                vec![format!("/profiles/{}/assets/{id}", selected.profile_id)]
            } else {
                vec![]
            },
        },
        names: vec![selected.name.clone()],
    }))
}
