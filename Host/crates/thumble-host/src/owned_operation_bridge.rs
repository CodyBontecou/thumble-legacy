//! Correlated owned-document native edits; persistence/CAS stays in the authority.
use super::{
    allowed_changed_path, valid_operation_delta, ConfigurationBridge, ConfigurationBridgeError,
};
use crate::draft_operation::{ConfigurationOperation, ConfigurationOperationOutcome};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use thumble_core::{ConfigurationDocument, OwnedConfigurationDocument, ProfileAppearanceBank};
use uuid::Uuid;
const MAXIMUM: usize = 8 * 1024 * 1024;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Response {
    schema_version: u32,
    #[serde(rename = "invocationID")]
    invocation_id: Uuid,
    #[serde(rename = "sourceSHA256")]
    source_sha256: String,
    before_document_data: String,
    after_document_data: String,
    changed: bool,
    changed_paths: Vec<String>,
}
impl ConfigurationBridge {
    pub fn apply_owned(
        &self,
        source: &OwnedConfigurationDocument,
        operation: &ConfigurationOperation,
        now_millis: i64,
        invocation: Uuid,
    ) -> Result<(OwnedConfigurationDocument, ConfigurationOperationOutcome), ConfigurationBridgeError>
    {
        use ConfigurationBridgeError as Error;
        if now_millis < 0 || !operation.requires_bridge() {
            return Err(Error::OperationNotSupported);
        }
        let canonical = canonical_operation(operation)?;
        canonical
            .validate_bridge_input()
            .map_err(|_| Error::InvalidOperation)?;
        super::validate_operation_bounds(&canonical)?;
        let mut input=serde_json::to_vec(&json!({"schemaVersion":1,"invocationID":invocation,
            "ownedConfigurationCommand":"apply","documentData":STANDARD.encode(source.original_bytes()),
            "operation":&canonical,"nowMillis":now_millis})).map_err(|_|Error::EncodingFailed)?;
        if input.len() > 24 * 1024 * 1024 {
            return Err(Error::InputTooLarge);
        }
        input.push(b'\n');
        let output = self.execute_with_limits(input, 24 * 1024 * 1024, 24 * 1024 * 1024 + 1)?;
        let response: Response =
            thumble_protocol::decode_unique_json(&output).map_err(|_| Error::InvalidResponse)?;
        if response.schema_version != 1
            || response.invocation_id != invocation
            || response.source_sha256 != format!("{:x}", Sha256::digest(source.original_bytes()))
            || response.changed_paths.len() > 128
            || response.changed_paths.iter().any(|path| {
                path.len() > 512
                    || path.chars().any(char::is_control)
                    || !allowed_changed_path(&canonical, path)
            })
        {
            return Err(Error::InvalidResponse);
        }
        let before = projection(&response.before_document_data)?;
        let after = projection(&response.after_document_data)?;
        let fallback = before
            .default_profile_id()
            .unwrap_or(before.active_profile_id());
        let after_fallback = if after.profiles().iter().any(|profile| {
            profile["id"]
                .as_str()
                .is_some_and(|id| id.eq_ignore_ascii_case(fallback))
        }) {
            fallback
        } else {
            after.active_profile_id()
        };
        if response.changed != (before.original_bytes() != after.original_bytes())
            || response.changed == response.changed_paths.is_empty()
            || !valid_operation_delta(
                &legacy_view(&before, fallback)?,
                &legacy_view(&after, after_fallback)?,
                &canonical,
            )
            || !banks_preserved(&before, &after, &canonical)
        {
            return Err(Error::InvalidResponse);
        }
        let merged = source
            .with_projected_changes(&before, &after)
            .map_err(|_| Error::InvalidResponse)?;
        Ok((
            merged,
            ConfigurationOperationOutcome {
                changed: response.changed,
                changed_paths: response.changed_paths,
            },
        ))
    }
}
fn projection(encoded: &str) -> Result<OwnedConfigurationDocument, ConfigurationBridgeError> {
    if encoded.len() > MAXIMUM.div_ceil(3) * 4 {
        return Err(ConfigurationBridgeError::InvalidResponse);
    }
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
    if STANDARD.encode(&bytes) != encoded {
        return Err(ConfigurationBridgeError::InvalidResponse);
    }
    OwnedConfigurationDocument::decode(&bytes)
        .map_err(|_| ConfigurationBridgeError::InvalidResponse)
}
fn canonical_operation(
    operation: &ConfigurationOperation,
) -> Result<ConfigurationOperation, ConfigurationBridgeError> {
    let mut value =
        serde_json::to_value(operation).map_err(|_| ConfigurationBridgeError::EncodingFailed)?;
    if value["type"]
        .as_str()
        .is_some_and(|kind| kind.starts_with("orientation."))
    {
        return Err(ConfigurationBridgeError::OperationNotSupported);
    }
    if let Some(variant) = value.get_mut("variant") {
        if !matches!(variant.as_str(), Some("primary" | "landscape")) {
            return Err(ConfigurationBridgeError::OperationNotSupported);
        }
        *variant = json!("primary");
    }
    serde_json::from_value(value).map_err(|_| ConfigurationBridgeError::InvalidOperation)
}
fn legacy_view(
    document: &OwnedConfigurationDocument,
    default_fallback: &str,
) -> Result<ConfigurationDocument, ConfigurationBridgeError> {
    let root: Value = thumble_protocol::decode_unique_json(document.original_bytes())
        .map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
    let mut fields = serde_json::Map::new();
    for key in [
        "profiles",
        "activeProfileID",
        "defaultProfileID",
        "keyBindings",
        "outputBindings",
        "profileKeyBindings",
        "profileOutputBindings",
    ] {
        if let Some(value) = root.get(key) {
            fields.insert(key.into(), value.clone());
        }
    }
    if fields.get("defaultProfileID").is_some_and(Value::is_null) {
        fields.insert("defaultProfileID".into(), json!(default_fallback));
    }
    if let Some(profiles) = fields.get_mut("profiles").and_then(Value::as_array_mut) {
        for profile in profiles {
            if let Some(profile) = profile.as_object_mut() {
                if let Some(layout) = profile.get("customization").cloned() {
                    profile.insert("landscapeCustomization".into(), layout);
                }
                profile.insert("orientationPreference".into(), json!("landscape"));
                for field in [
                    "profileFormatVersion",
                    "appearanceBank",
                    "appearanceResources",
                    "appearanceResourceVersion",
                ] {
                    profile.remove(field);
                }
            }
        }
    }
    serde_json::from_value(Value::Object(fields))
        .map_err(|_| ConfigurationBridgeError::InvalidResponse)
}
fn banks_preserved(
    before: &OwnedConfigurationDocument,
    after: &OwnedConfigurationDocument,
    operation: &ConfigurationOperation,
) -> bool {
    for profile in before.profiles() {
        let Some(next) = after
            .profiles()
            .iter()
            .find(|next| next["id"] == profile["id"])
        else {
            continue;
        };
        if profile.get("appearanceBank") == next.get("appearanceBank") {
            continue;
        }
        if let ConfigurationOperation::ElementDuplicate {
            profile_id,
            element_ids,
            new_element_ids,
            ..
        } = operation
        {
            if profile["id"]
                .as_str()
                .is_some_and(|id| id.eq_ignore_ascii_case(profile_id))
            {
                let Some(mut expected) = profile.get("appearanceBank").cloned() else {
                    return false;
                };
                let Some(rows) = expected["variants"].as_array_mut() else {
                    return false;
                };
                if element_ids.len() != new_element_ids.len() {
                    return false;
                }
                for row in rows {
                    let Some(controls) = row["appearance"]["controls"].as_object_mut() else {
                        return false;
                    };
                    for (source, destination) in element_ids.iter().zip(new_element_ids) {
                        let Some(paint) = controls.get(&source.to_ascii_lowercase()).cloned()
                        else {
                            return false;
                        };
                        if controls
                            .insert(destination.to_ascii_lowercase(), paint)
                            .is_some()
                        {
                            return false;
                        }
                    }
                }
                if !paint_equal(&expected, &next["appearanceBank"]) {
                    return false;
                }
                continue;
            }
        }
        let mut old = profile.clone();
        if let Some(resources) = next.get("appearanceResources") {
            old["appearanceResources"] = resources.clone();
        }
        let (Ok(Some(old_bank)), Ok(Some(new_bank))) = (
            ProfileAppearanceBank::from_profile(&old),
            ProfileAppearanceBank::from_profile(next),
        ) else {
            return false;
        };
        let ids = |layout: &Value| {
            layout["elements"].as_array().map(|items| {
                items
                    .iter()
                    .map(|item| item["id"].as_str().unwrap_or("").to_ascii_lowercase())
                    .collect::<std::collections::BTreeSet<_>>()
            })
        };
        if ids(&profile["customization"]) == ids(&next["customization"]) {
            return false;
        }
        for scheme in ["light", "dark"] {
            for action in ["base", "left-trigger", "right-trigger", "both-triggers"] {
                let (Ok(expected), Ok(actual)) = (
                    old_bank.resolve_layout(&next["customization"], scheme, action),
                    new_bank.resolve_layout(&next["customization"], scheme, action),
                ) else {
                    return false;
                };
                if !paint_equal(&expected, &actual) {
                    return false;
                }
            }
        }
    }
    true
}
fn paint_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Object(a), Value::Object(b)) => {
            if a.len() != b.len() {
                return false;
            }
            let color = ["red", "green", "blue", "opacity"]
                .iter()
                .all(|key| a.contains_key(*key) && b.contains_key(*key));
            a.iter().all(|(key, value)| {
                b.get(key).is_some_and(|other| {
                    if color && ["red", "green", "blue", "opacity"].contains(&key.as_str()) {
                        value
                            .as_f64()
                            .zip(other.as_f64())
                            .is_some_and(|(a, b)| (a - b).abs() <= 2e-7)
                    } else {
                        paint_equal(value, other)
                    }
                })
            })
        }
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| paint_equal(a, b))
        }
        (Value::Number(_), Value::Number(_)) => super::json_semantically_equal(a, b),
        _ => a == b,
    }
}

#[cfg(test)]
mod arrangement_tests {
    use super::*;
    fn documents() -> (
        OwnedConfigurationDocument,
        OwnedConfigurationDocument,
        ConfigurationOperation,
    ) {
        let request: Value = serde_json::from_slice(include_bytes!(
            "../../../fixtures/owned-element-sequences/v1/duplicate-request.json"
        ))
        .unwrap();
        let response: Response = serde_json::from_slice(include_bytes!(
            "../../../fixtures/owned-element-sequences/v1/duplicate-response.json"
        ))
        .unwrap();
        (
            projection(&response.before_document_data).unwrap(),
            projection(&response.after_document_data).unwrap(),
            serde_json::from_value(request["operation"].clone()).unwrap(),
        )
    }
    #[test]
    fn duplicate_inherits_each_saved_state_and_preserves_input_ownership() {
        let (before, after, operation) = documents();
        assert!(banks_preserved(&before, &after, &operation));
        assert!(valid_operation_delta(
            &legacy_view(&before, before.active_profile_id()).unwrap(),
            &legacy_view(&after, after.active_profile_id()).unwrap(),
            &operation
        ));
    }
    #[test]
    fn forged_duplicate_paint_and_unrelated_geometry_are_rejected() {
        let (before, after, operation) = documents();
        let mut value: Value = serde_json::from_slice(after.original_bytes()).unwrap();
        value["profiles"][0]["appearanceBank"]["variants"][0]["appearance"]["controls"]
            ["ac000000-0000-0000-0000-000000000003"]["icon"]["value"] = json!("forged-state");
        let forged =
            OwnedConfigurationDocument::decode(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(!banks_preserved(&before, &forged, &operation));
        let mut value: Value = serde_json::from_slice(after.original_bytes()).unwrap();
        let original = value["profiles"][0]["customization"]["elements"][0]["layout"]["centerX"]
            .as_f64()
            .unwrap_or(0.5);
        value["profiles"][0]["customization"]["elements"][0]["layout"]["centerX"] =
            json!(original + 0.01);
        let forged =
            OwnedConfigurationDocument::decode(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(!valid_operation_delta(
            &legacy_view(&before, before.active_profile_id()).unwrap(),
            &legacy_view(&forged, forged.active_profile_id()).unwrap(),
            &operation
        ));
    }
}
