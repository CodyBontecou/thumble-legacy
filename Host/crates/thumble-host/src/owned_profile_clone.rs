//! Preserve exact source appearance and extension fields when cloning profiles.
use super::*;
use serde_json::value::RawValue;
use std::collections::BTreeMap;
pub(super) fn identities(operation: &ConfigurationOperation) -> Option<(&str, &str)> {
    match operation {
        ConfigurationOperation::ProfileCreate {
            source_profile_id: Some(source),
            new_profile_id,
            ..
        } => Some((source, new_profile_id)),
        ConfigurationOperation::ProfileDuplicate {
            profile_id,
            new_profile_id,
            ..
        } => Some((profile_id, new_profile_id)),
        _ => None,
    }
}
pub(super) fn raw_profile<'a>(
    document: &'a OwnedConfigurationDocument,
    id: &str,
) -> Result<(usize, &'a RawValue), ConfigurationBridgeError> {
    let root: BTreeMap<String, &RawValue> = serde_json::from_slice(document.original_bytes())
        .map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
    let profiles: Vec<&RawValue> = serde_json::from_str(root["profiles"].get())
        .map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
    for (index, profile) in profiles.into_iter().enumerate() {
        let fields: BTreeMap<String, &RawValue> = serde_json::from_str(profile.get())
            .map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
        let actual: String = serde_json::from_str(fields["id"].get())
            .map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
        if actual.eq_ignore_ascii_case(id) {
            return Ok((index, profile));
        }
    }
    Err(ConfigurationBridgeError::InvalidResponse)
}
pub(super) fn merge(
    source: &OwnedConfigurationDocument,
    before: &OwnedConfigurationDocument,
    after: &OwnedConfigurationDocument,
    source_id: &str,
    new_id: &str,
) -> Result<OwnedConfigurationDocument, ConfigurationBridgeError> {
    let original = raw_profile(source, source_id)?.1;
    let old = raw_profile(before, source_id)?.1;
    let (index, new) = raw_profile(after, new_id)?;
    // Only identity/name/time and the explicit new-profile launch exclusion may
    // differ. Copy the exact original graph instead of reserializing its arrays.
    let mut left: Value =
        serde_json::from_str(old.get()).map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
    let mut right: Value =
        serde_json::from_str(new.get()).map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
    for key in [
        "id",
        "name",
        "updatedAt",
        "launchTarget",
        "appearanceResources",
        "appearanceResourceVersion",
    ] {
        left.as_object_mut().unwrap().remove(key);
        right.as_object_mut().unwrap().remove(key);
    }
    if left != right {
        return Err(ConfigurationBridgeError::InvalidResponse);
    }
    let fields: BTreeMap<String, &RawValue> = serde_json::from_str(original.get())
        .map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
    let next: BTreeMap<String, &RawValue> =
        serde_json::from_str(new.get()).map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
    let mut edits: Vec<_> = ["id", "name", "updatedAt"]
        .into_iter()
        .map(|key| thumble_protocol::JsonValueEdit {
            pointer: match key {
                "id" => "/id",
                "name" => "/name",
                _ => "/updatedAt",
            },
            value: Some(next[key].get().as_bytes()),
        })
        .collect();
    if fields.contains_key("launchTarget") && !next.contains_key("launchTarget") {
        edits.push(thumble_protocol::JsonValueEdit {
            pointer: "/launchTarget",
            value: None,
        });
    }
    let raw = thumble_protocol::edit_json_values(original.get().as_bytes(), &edits, MAXIMUM)
        .map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
    let raw = prune_resources(raw)?;
    let merged = source
        .with_projected_changes(before, after)
        .map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
    merged
        .with_edits(&[thumble_protocol::JsonValueEdit {
            pointer: &format!("/profiles/{index}"),
            value: Some(&raw),
        }])
        .map_err(|_| ConfigurationBridgeError::InvalidResponse)
}

pub(super) fn prune_resources(raw: Vec<u8>) -> Result<Vec<u8>, ConfigurationBridgeError> {
    let mut raw = raw;
    let graph: Value =
        serde_json::from_slice(&raw).map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
    if let Some(pool) = thumble_core::ProfileResourceTable::from_profile(&graph)
        .map_err(|_| ConfigurationBridgeError::InvalidResponse)?
    {
        let pruned = pool
            .pruning_unused(&graph)
            .map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
        let bytes = serde_json::to_vec(&pruned["appearanceResources"])
            .map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
        raw = thumble_protocol::edit_json_values(
            &raw,
            &[thumble_protocol::JsonValueEdit {
                pointer: "/appearanceResources",
                value: Some(&bytes),
            }],
            MAXIMUM,
        )
        .map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
    }
    Ok(raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cloned_profiles_retain_exact_opaque_fields_beyond_the_native_projection() {
        let projection: Value = serde_json::from_slice(include_bytes!(
            "../../../fixtures/owned-configuration/v1/document.json"
        ))
        .unwrap();
        let before =
            OwnedConfigurationDocument::decode(&serde_json::to_vec(&projection).unwrap()).unwrap();
        let precise = b"0.123456789012345678901234567890123456789";
        let source = before
            .with_edits(&[
                thumble_protocol::JsonValueEdit {
                    pointer: "/profiles/0/futureCloneNumber",
                    value: Some(precise),
                },
                thumble_protocol::JsonValueEdit {
                    pointer: "/profiles/0/customization/elements/0/futureCloneNumber",
                    value: Some(precise),
                },
                thumble_protocol::JsonValueEdit {
                    pointer: "/profiles/0/appearanceBank/variants/0/futureCloneNumber",
                    value: Some(precise),
                },
            ])
            .unwrap();
        let mut value: Value = serde_json::from_slice(before.original_bytes()).unwrap();
        let mut clone = value["profiles"][0].clone();
        let new_id = "ee000000-0000-4000-8000-000000000001";
        clone["id"] = json!(new_id);
        clone["name"] = json!("Copy");
        clone["updatedAt"] = json!(2000);
        clone.as_object_mut().unwrap().remove("launchTarget");
        value["profiles"].as_array_mut().unwrap().push(clone);
        let after =
            OwnedConfigurationDocument::decode(&serde_json::to_vec(&value).unwrap()).unwrap();
        let id = before.profiles()[0]["id"].as_str().unwrap();
        let actual = merge(&source, &before, &after, id, new_id).unwrap();
        assert_eq!(actual.profiles()[..2], source.profiles()[..2]);
        let text = std::str::from_utf8(actual.original_bytes()).unwrap();
        assert_eq!(
            text.matches(std::str::from_utf8(precise).unwrap()).count(),
            6
        );
        assert_eq!(actual.profiles()[2]["id"], new_id);
        assert!(actual.profiles()[2].get("launchTarget").is_none());
    }
}
