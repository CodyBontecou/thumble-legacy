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
#[path = "owned_style_import.rs"]
mod style_import;
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
                &operation_view(&before, fallback, &canonical)?,
                &operation_view(&after, after_fallback, &canonical)?,
                &canonical,
            )
            || !banks_preserved(&before, &after, &canonical)
        {
            return Err(Error::InvalidResponse);
        }
        let merged = if let ConfigurationOperation::StyleImport { profile_id, .. } = &canonical {
            if response.changed {
                style_import::merge(source, &before, &after, profile_id)?
            } else {
                source.clone()
            }
        } else {
            source
                .with_projected_changes(&before, &after)
                .map_err(|_| Error::InvalidResponse)?
        };
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
    if matches!(
        operation,
        ConfigurationOperation::CustomizationImport { .. }
    ) {
        return Err(ConfigurationBridgeError::OperationNotSupported);
    }

    let mut value =
        serde_json::to_value(operation).map_err(|_| ConfigurationBridgeError::EncodingFailed)?;
    if value["type"] == "device.set"
        && value["frameID"]
            .as_str()
            .is_some_and(|id| !id.ends_with("-landscape"))
    {
        return Err(ConfigurationBridgeError::OperationNotSupported);
    }
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
fn operation_view(
    document: &OwnedConfigurationDocument,
    fallback: &str,
    operation: &ConfigurationOperation,
) -> Result<ConfigurationDocument, ConfigurationBridgeError> {
    let mut view = legacy_view(document, fallback)?;
    if matches!(operation, ConfigurationOperation::DesignApply { .. }) {
        // The design layout plan names the single canonical slot. An invented
        // landscape mirror must not demand a second, unreviewed layout edit.
        for profile in &mut view.profiles {
            if let Some(object) = profile.as_object_mut() {
                object.remove("landscapeCustomization");
            }
        }
    }
    Ok(view)
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
        if let ConfigurationOperation::CustomizationReset { profile_id, .. }
        | ConfigurationOperation::ProfileReset { profile_id } = operation
        {
            if profile["id"]
                .as_str()
                .is_some_and(|id| id.eq_ignore_ascii_case(profile_id))
            {
                if profile.get("appearanceBank").is_none() && next.get("appearanceBank").is_none() {
                    continue;
                }
                if profile.get("appearanceBank").is_none() {
                    return false;
                }
                let Ok(Some(bank)) = ProfileAppearanceBank::from_profile(next) else {
                    return false;
                };
                if !bank
                    .is_canonical_appearance(&next["customization"])
                    .unwrap_or(false)
                {
                    return false;
                }
                continue;
            }
        }
        if profile.get("appearanceBank") == next.get("appearanceBank") {
            continue;
        }
        if let ConfigurationOperation::DesignApply { profile_id, .. }
        | ConfigurationOperation::StyleImport { profile_id, .. } = operation
        {
            if profile["id"]
                .as_str()
                .is_some_and(|id| id.eq_ignore_ascii_case(profile_id))
            {
                if !design_states_preserved(before, after, profile, next, operation) {
                    return false;
                }
                continue;
            }
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
/// Design may replace paint, but never executable declarations or saved haptic
/// settings hidden in a non-base appearance state.
fn design_states_preserved(
    before: &OwnedConfigurationDocument,
    after: &OwnedConfigurationDocument,
    profile: &Value,
    next: &Value,
    operation: &ConfigurationOperation,
) -> bool {
    let (Ok(old_bank), Ok(Some(new_bank))) = (
        ProfileAppearanceBank::from_profile(profile),
        ProfileAppearanceBank::from_profile(next),
    ) else {
        return false;
    };
    let (Ok(mut left), Ok(mut right)) = (
        operation_view(
            before,
            before
                .default_profile_id()
                .unwrap_or(before.active_profile_id()),
            operation,
        ),
        operation_view(
            after,
            after
                .default_profile_id()
                .unwrap_or(after.active_profile_id()),
            operation,
        ),
    ) else {
        return false;
    };
    let Some(index) = before
        .profiles()
        .iter()
        .position(|value| value["id"] == profile["id"])
    else {
        return false;
    };
    for scheme in ["light", "dark"] {
        for action in ["base", "left-trigger", "right-trigger", "both-triggers"] {
            let old = match &old_bank {
                Some(bank) => {
                    match bank.resolve_layout(&profile["customization"], scheme, action) {
                        Ok(value) => value,
                        Err(_) => return false,
                    }
                }
                None if matches!(operation, ConfigurationOperation::StyleImport { .. }) => {
                    match ProfileAppearanceBank::canonical_display_layout(&profile["customization"])
                    {
                        Ok(value) => value,
                        Err(_) => return false,
                    }
                }
                None => profile["customization"].clone(),
            };
            let Ok(new) = new_bank.resolve_layout(&next["customization"], scheme, action) else {
                return false;
            };
            left.profiles[index]["customization"] = old;
            right.profiles[index]["customization"] = new;
            if !valid_operation_delta(&left, &right, operation) {
                return false;
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

#[cfg(test)]
mod design_tests {
    use super::*;
    #[test]
    fn owned_design_cannot_change_trigger_haptics_geometry_outputs_or_other_profiles() {
        let request: Value = serde_json::from_slice(include_bytes!(
            "../../../fixtures/owned-design/v1/apply-request.json"
        ))
        .unwrap();
        let response: Response = serde_json::from_slice(include_bytes!(
            "../../../fixtures/owned-design/v1/apply-response.json"
        ))
        .unwrap();
        let operation: ConfigurationOperation =
            serde_json::from_value(request["operation"].clone()).unwrap();
        let before = projection(&response.before_document_data).unwrap();
        let after_bytes = STANDARD.decode(&response.after_document_data).unwrap();
        for path in [
            "/profiles/1/appearanceBank/variants/1/appearance/controls/00000000-0000-0000-0000-000000000105/hapticFeedback/intensity",
            "/profiles/1/customization/elements/0/layout/centerY",
            "/profiles/1/customization/elements/0/output/keyboard/keyCode",
            "/profiles/0/customization/elements/0/layout/centerX",
        ] {
            let mut forged: Value = serde_json::from_slice(&after_bytes).unwrap();
            *forged.pointer_mut(path).expect(path) = serde_json::json!(0.123);
            let forged = OwnedConfigurationDocument::decode(&serde_json::to_vec(&forged).unwrap());
            if let Ok(forged) = forged {
                assert!(
                    !banks_preserved(&before, &forged, &operation)
                        || !valid_operation_delta(&operation_view(&before,before.active_profile_id(),&operation).unwrap(), &operation_view(&forged,forged.active_profile_id(),&operation).unwrap(), &operation),
                    "forged design passed {path}"
                );
            }
        }
    }
    #[test]
    fn reviewed_owned_design_preserves_executable_declarations_and_raw_metadata() {
        let request: Value = serde_json::from_slice(include_bytes!(
            "../../../fixtures/owned-design/v1/apply-request.json"
        ))
        .unwrap();
        let response: Response = serde_json::from_slice(include_bytes!(
            "../../../fixtures/owned-design/v1/apply-response.json"
        ))
        .unwrap();
        let operation: ConfigurationOperation =
            serde_json::from_value(request["operation"].clone()).unwrap();
        let source = projection(request["documentData"].as_str().unwrap()).unwrap();
        let before = projection(&response.before_document_data).unwrap();
        let after = projection(&response.after_document_data).unwrap();
        assert!(
            response
                .changed_paths
                .iter()
                .all(|path| allowed_changed_path(&operation, path)),
            "paths"
        );
        assert!(banks_preserved(&before, &after, &operation), "banks");
        assert!(
            valid_operation_delta(
                &operation_view(&before, before.active_profile_id(), &operation).unwrap(),
                &operation_view(&after, after.active_profile_id(), &operation).unwrap(),
                &operation
            ),
            "delta"
        );
        let merged = source
            .with_projected_changes(&before, &after)
            .expect("lossless source merge");
        assert!(std::str::from_utf8(merged.original_bytes())
            .unwrap()
            .contains("0.12345678901234567890123456789"));
    }
}

#[cfg(test)]
mod style_tests {
    use super::*;
    #[test]
    fn captured_style_import_preserves_owned_state() {
        let request: Value = serde_json::from_slice(include_bytes!(
            "../../../fixtures/style-library/v1/plain-request.json"
        ))
        .unwrap();
        let response: Response = serde_json::from_slice(include_bytes!(
            "../../../fixtures/style-library/v1/plain-response.json"
        ))
        .unwrap();
        let operation: ConfigurationOperation =
            serde_json::from_value(request["operation"].clone()).unwrap();
        let source = projection(request["documentData"].as_str().unwrap()).unwrap();
        let before = projection(&response.before_document_data).expect("before");
        let after = projection(&response.after_document_data).expect("after");
        assert!(banks_preserved(&before, &after, &operation), "banks");
        assert!(
            valid_operation_delta(
                &operation_view(&before, before.default_profile_id().unwrap(), &operation).unwrap(),
                &operation_view(&after, after.default_profile_id().unwrap(), &operation).unwrap(),
                &operation
            ),
            "delta"
        );
        let ConfigurationOperation::StyleImport { profile_id, .. } = operation else {
            panic!()
        };
        style_import::merge(&source, &before, &after, &profile_id).expect("merge");
    }
}

#[cfg(test)]
mod banked_style_tests {
    use super::*;
    #[test]
    fn banked_import_retains_precise_metadata_and_refuses_unrelated_mutation() {
        let request: Value = serde_json::from_slice(include_bytes!(
            "../../../fixtures/style-library/v1/banked-request.json"
        ))
        .unwrap();
        let response: Response = serde_json::from_slice(include_bytes!(
            "../../../fixtures/style-library/v1/banked-response.json"
        ))
        .unwrap();
        let operation: ConfigurationOperation =
            serde_json::from_value(request["operation"].clone()).unwrap();
        let source = projection(request["documentData"].as_str().unwrap()).unwrap();
        let before = projection(&response.before_document_data).unwrap();
        let after = projection(&response.after_document_data).unwrap();
        assert!(banks_preserved(&before, &after, &operation));
        let ConfigurationOperation::StyleImport { ref profile_id, .. } = operation else {
            panic!()
        };
        let merged = style_import::merge(&source, &before, &after, profile_id).unwrap();
        assert_eq!(
            std::str::from_utf8(merged.original_bytes())
                .unwrap()
                .matches("0.12345678901234567890123456789")
                .count(),
            9
        );
        assert_eq!(merged.profiles()[1], source.profiles()[1]);
        for path in [
            "/profiles/0/appearanceBank/variants/1/appearance/controls/00000000-0000-0000-0000-000000000109/shadowStrength",
            "/profiles/0/customization/elements/0/layout/centerX",
            "/profiles/1/customization/elements/0/layout/centerX",
        ] {
            let mut forged: Value = serde_json::from_slice(after.original_bytes()).unwrap();
            if let Some(value) = forged.pointer_mut(path) { *value = json!(0.123); } else { continue; }
            if let Ok(forged) = OwnedConfigurationDocument::decode(&serde_json::to_vec(&forged).unwrap()) {
                assert!(!banks_preserved(&before,&forged,&operation) || !valid_operation_delta(&operation_view(&before,before.default_profile_id().unwrap(),&operation).unwrap(),&operation_view(&forged,forged.default_profile_id().unwrap(),&operation).unwrap(),&operation),"{path}");
            }
        }
    }
}

#[cfg(test)]
mod reset_tests {
    use super::*;
    #[test]
    fn owned_resets_merge_exact_sources_and_refuse_leftover_state_paint() {
        let cases: &[(&[u8], &[u8])] = &[
            (
                include_bytes!("../../../fixtures/customization-reset/v1/plain-request.json"),
                include_bytes!("../../../fixtures/customization-reset/v1/plain-response.json"),
            ),
            (
                include_bytes!("../../../fixtures/customization-reset/v1/banked-request.json"),
                include_bytes!("../../../fixtures/customization-reset/v1/banked-response.json"),
            ),
            (
                include_bytes!(
                    "../../../fixtures/customization-reset/v1/plain-profile-request.json"
                ),
                include_bytes!(
                    "../../../fixtures/customization-reset/v1/plain-profile-response.json"
                ),
            ),
            (
                include_bytes!(
                    "../../../fixtures/customization-reset/v1/banked-profile-request.json"
                ),
                include_bytes!(
                    "../../../fixtures/customization-reset/v1/banked-profile-response.json"
                ),
            ),
        ];
        for (request, response) in cases {
            let request: Value = serde_json::from_slice(request).unwrap();
            let response: Response = serde_json::from_slice(response).unwrap();
            let operation: ConfigurationOperation =
                serde_json::from_value(request["operation"].clone()).unwrap();
            let source = projection(request["documentData"].as_str().unwrap()).unwrap();
            let before = projection(&response.before_document_data).unwrap();
            let after = projection(&response.after_document_data).unwrap();
            assert!(banks_preserved(&before, &after, &operation));
            assert!(valid_operation_delta(
                &operation_view(&before, before.default_profile_id().unwrap(), &operation).unwrap(),
                &operation_view(&after, after.default_profile_id().unwrap(), &operation).unwrap(),
                &operation
            ));
            let merged = source.with_projected_changes(&before, &after).unwrap();
            assert!(std::str::from_utf8(merged.original_bytes())
                .unwrap()
                .contains("0.12345678901234567890123456789"));
            let target = request["operation"]["profileID"].as_str().unwrap();
            let index = after
                .profiles()
                .iter()
                .position(|p| p["id"].as_str().unwrap().eq_ignore_ascii_case(target))
                .unwrap();
            if after.profiles()[index].get("appearanceBank").is_none() {
                continue;
            }
            for all in [false, true] {
                let mut forged: Value = serde_json::from_slice(after.original_bytes()).unwrap();
                let rows = forged["profiles"][index]["appearanceBank"]["variants"]
                    .as_array_mut()
                    .unwrap();
                for row in rows.iter_mut().take(if all { 8 } else { 1 }) {
                    row["appearance"]["controls"]["00000000-0000-0000-0000-000000000109"]
                        ["cornerRadius"] = json!(44);
                }
                let forged =
                    OwnedConfigurationDocument::decode(&serde_json::to_vec(&forged).unwrap())
                        .unwrap();
                assert!(!banks_preserved(&before, &forged, &operation));
                assert!(
                    !banks_preserved(&forged, &forged, &operation),
                    "equal-bank shortcut must not bypass reset validation"
                );
            }
        }
    }
}

#[cfg(test)]
mod custom_canvas_tests {
    use super::*;
    use crate::draft_operation::ConfigurationVariant;
    #[test]
    fn owned_canvas_targets_remain_landscape_even_in_primary_slot() {
        for id in ["iphone-16-pro-portrait", "custom-390x844-portrait"] {
            let operation = ConfigurationOperation::DeviceSet {
                profile_id: Uuid::nil().to_string(),
                variant: ConfigurationVariant::Primary,
                frame_id: id.into(),
            };
            assert!(canonical_operation(&operation).is_err());
        }
        let operation = ConfigurationOperation::DeviceSet {
            profile_id: Uuid::nil().to_string(),
            variant: ConfigurationVariant::Landscape,
            frame_id: "custom-390x844-landscape".into(),
        };
        assert!(matches!(
            canonical_operation(&operation).unwrap(),
            ConfigurationOperation::DeviceSet {
                variant: ConfigurationVariant::Primary,
                ..
            }
        ));
    }
}
