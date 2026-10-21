//! Scope a native rich-background projection back into exact owned source.
//! Bank arrays are keyed by scheme/action; original resource blobs survive.
use super::*;
use serde_json::value::RawValue;
use std::collections::BTreeMap;
use thumble_protocol::{merge_json_projection, JsonValueEdit};
type Object<'a> = BTreeMap<String, &'a RawValue>;
const BACKGROUND: [&str; 5] = [
    "backgroundFillStyle",
    "backgroundLightFillStyle",
    "backgroundDarkFillStyle",
    "backgroundLightColor",
    "backgroundDarkColor",
];
fn invalid() -> ConfigurationBridgeError {
    ConfigurationBridgeError::InvalidResponse
}
fn object(bytes: &[u8]) -> Result<Object<'_>, ConfigurationBridgeError> {
    serde_json::from_slice(bytes).map_err(|_| invalid())
}
fn profile(
    document: &OwnedConfigurationDocument,
    index: usize,
) -> Result<Object<'_>, ConfigurationBridgeError> {
    let root = object(document.original_bytes())?;
    let profiles: Vec<&RawValue> =
        serde_json::from_str(root["profiles"].get()).map_err(|_| invalid())?;
    object(profiles.get(index).ok_or_else(invalid)?.get().as_bytes())
}
fn rows<'a>(
    bank: &'a RawValue,
) -> Result<BTreeMap<String, (usize, Object<'a>)>, ConfigurationBridgeError> {
    let bank = object(bank.get().as_bytes())?;
    let variants: Vec<&RawValue> =
        serde_json::from_str(bank["variants"].get()).map_err(|_| invalid())?;
    let mut result = BTreeMap::new();
    for (index, row) in variants.into_iter().enumerate() {
        let row = object(row.get().as_bytes())?;
        let scheme: String =
            serde_json::from_str(row["colorScheme"].get()).map_err(|_| invalid())?;
        let action: String = serde_json::from_str(row["actionSet"].get()).map_err(|_| invalid())?;
        let key = format!("{scheme}/{action}");
        let appearance = object(row["appearance"].get().as_bytes())?;
        if result.insert(key, (index, appearance)).is_some() {
            return Err(invalid());
        }
    }
    Ok(result)
}
fn value<'a>(fields: &'a Object<'_>, key: &str) -> &'a [u8] {
    fields
        .get(key)
        .map(|v| v.get().as_bytes())
        .unwrap_or(b"null")
}
pub(super) fn merge(
    source: &OwnedConfigurationDocument,
    before: &OwnedConfigurationDocument,
    after: &OwnedConfigurationDocument,
    id: &str,
) -> Result<OwnedConfigurationDocument, ConfigurationBridgeError> {
    let index = source
        .profiles()
        .iter()
        .position(|p| p["id"].as_str().is_some_and(|v| v.eq_ignore_ascii_case(id)))
        .ok_or_else(invalid)?;
    let a = profile(source, index)?;
    let b = profile(before, index)?;
    let c = profile(after, index)?;
    for fields in [&a, &b, &c] {
        let actual: String = serde_json::from_str(fields.get("id").ok_or_else(invalid)?.get())
            .map_err(|_| invalid())?;
        if !actual.eq_ignore_ascii_case(id) {
            return Err(invalid());
        }
    }
    let mut edits: Vec<(String, Option<Vec<u8>>)> = Vec::new();
    for field in ["customization", "updatedAt"] {
        if value(&b, field) != value(&c, field) {
            let merged = merge_json_projection(
                value(&a, field),
                value(&b, field),
                value(&c, field),
                MAXIMUM,
            )
            .map_err(|_| invalid())?;
            edits.push((format!("/profiles/{index}/{field}"), Some(merged)));
        }
    }
    match (
        a.get("appearanceBank")
            .filter(|value| value.get() != "null"),
        b.get("appearanceBank")
            .filter(|value| value.get() != "null"),
        c.get("appearanceBank")
            .filter(|value| value.get() != "null"),
    ) {
        (Some(original), Some(old), Some(new)) => {
            let original = rows(original)?;
            let old = rows(old)?;
            let new = rows(new)?;
            if original.keys().ne(old.keys()) || old.keys().ne(new.keys()) {
                return Err(invalid());
            }
            for (key, (ordinal, raw)) in &original {
                for field in BACKGROUND {
                    let previous = value(&old[key].1, field);
                    let next = value(&new[key].1, field);
                    if previous == next {
                        continue;
                    }
                    let merged = merge_json_projection(value(raw, field), previous, next, MAXIMUM)
                        .map_err(|_| invalid())?;
                    if next == b"null" && merged != b"null" {
                        return Err(invalid());
                    }
                    edits.push((format!("/profiles/{index}/appearanceBank/variants/{ordinal}/appearance/{field}"), (next != b"null").then_some(merged)));
                }
            }
        }
        (None, None, None) => {}
        _ => return Err(invalid()),
    }
    if let Some(incoming) = c.get("appearanceResources") {
        let mut pool = a
            .get("appearanceResources")
            .map(|p| object(p.get().as_bytes()))
            .transpose()?
            .unwrap_or_default();
        let mut changed = !a.contains_key("appearanceResources");
        for (digest, entry) in object(incoming.get().as_bytes())? {
            if let Some(existing) = pool.get(&digest) {
                let x: String = serde_json::from_str(existing.get()).map_err(|_| invalid())?;
                let y: String = serde_json::from_str(entry.get()).map_err(|_| invalid())?;
                if x != y {
                    return Err(invalid());
                }
            } else {
                pool.insert(digest, entry);
                changed = true;
            }
        }
        if changed {
            edits.push((
                format!("/profiles/{index}/appearanceResources"),
                Some(serde_json::to_vec(&pool).map_err(|_| invalid())?),
            ));
        }
        if !a.contains_key("appearanceResourceVersion") {
            edits.push((
                format!("/profiles/{index}/appearanceResourceVersion"),
                Some(b"1".to_vec()),
            ));
        }
    }
    let edits: Vec<_> = edits
        .iter()
        .map(|(pointer, value)| JsonValueEdit {
            pointer,
            value: value.as_deref(),
        })
        .collect();
    // A temporary union can contain both the old and incoming resource tables.
    // Prune only after collecting references from the complete raw profile,
    // including opaque metadata. Persisted documents keep the original 8 MiB cap.
    let mut staged =
        thumble_protocol::edit_json_values(source.original_bytes(), &edits, MAXIMUM * 2)
            .map_err(|_| invalid())?;
    let root = object(&staged)?;
    let profiles: Vec<&RawValue> =
        serde_json::from_str(root["profiles"].get()).map_err(|_| invalid())?;
    let fields = object(profiles[index].get().as_bytes())?;
    if let Some(resources) = fields.get("appearanceResources") {
        let mut pool = object(resources.get().as_bytes())?;
        let graph: Value = serde_json::from_str(profiles[index].get()).map_err(|_| invalid())?;
        let mut pending: Vec<_> = graph
            .as_object()
            .ok_or_else(invalid)?
            .iter()
            .filter(|(key, _)| *key != "appearanceResources")
            .map(|(_, v)| v)
            .collect();
        let mut used = std::collections::BTreeSet::new();
        while let Some(node) = pending.pop() {
            match node {
                Value::Object(o) if o.contains_key("resourceSHA256") => {
                    let digest = o["resourceSHA256"]
                        .as_str()
                        .filter(|_| o.len() == 1)
                        .ok_or_else(invalid)?;
                    if !pool.contains_key(digest) {
                        return Err(invalid());
                    }
                    used.insert(digest.to_owned());
                }
                Value::Object(o) => pending.extend(o.values()),
                Value::Array(a) => pending.extend(a),
                _ => {}
            }
        }
        let old_count = pool.len();
        pool.retain(|digest, _| used.contains(digest));
        if pool.len() != old_count {
            let bytes = serde_json::to_vec(&pool).map_err(|_| invalid())?;
            staged = thumble_protocol::edit_json_values(
                &staged,
                &[JsonValueEdit {
                    pointer: &format!("/profiles/{index}/appearanceResources"),
                    value: Some(&bytes),
                }],
                MAXIMUM * 2,
            )
            .map_err(|_| invalid())?;
        }
    }
    OwnedConfigurationDocument::decode(&staged).map_err(|_| invalid())
}

#[cfg(test)]
mod optional_tests {
    use super::*;
    #[test]
    fn a_valid_explicit_null_bank_remains_null_after_plain_background_edits() {
        let mut source: Value = serde_json::from_slice(include_bytes!(
            "../../../fixtures/owned-configuration/v1/document.json"
        ))
        .unwrap();
        source["profiles"][1]["appearanceBank"] = Value::Null;
        let source =
            OwnedConfigurationDocument::decode(&serde_json::to_vec(&source).unwrap()).unwrap();
        let mut before: Value = serde_json::from_slice(source.original_bytes()).unwrap();
        before["profiles"][1]
            .as_object_mut()
            .unwrap()
            .remove("appearanceBank");
        let mut after = before.clone();
        after["profiles"][1]["customization"]["backgroundFillStyle"] =
            json!({"kind":"solid","color":{"red":1,"green":0,"blue":0,"alpha":1}});
        after["profiles"][1]["updatedAt"] = json!(2000);
        let id = source.profiles()[1]["id"].as_str().unwrap();
        let before =
            OwnedConfigurationDocument::decode(&serde_json::to_vec(&before).unwrap()).unwrap();
        let after =
            OwnedConfigurationDocument::decode(&serde_json::to_vec(&after).unwrap()).unwrap();
        let result = merge(&source, &before, &after, id).unwrap();
        assert_eq!(
            result.profiles()[1].get("appearanceBank"),
            Some(&Value::Null)
        );
        assert_eq!(result.profiles()[0], source.profiles()[0]);
        assert_eq!(
            result.profiles()[1]["customization"]["backgroundFillStyle"]["kind"],
            "solid"
        );
    }
}
