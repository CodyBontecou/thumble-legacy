//! Style imports patch only catalogs and resource entries. Matching the finite
//! bank-state keys avoids replacing opaque metadata in an unkeyed JSON array.
use super::*;
use serde_json::value::RawValue;
use std::collections::BTreeMap;
use thumble_protocol::{merge_json_projection, JsonValueEdit};
type Object<'a> = BTreeMap<String, &'a RawValue>;
const MAXIMUM: usize = 8 * 1024 * 1024;
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
fn library<'a>(appearance: &'a Object<'a>) -> &'a [u8] {
    appearance
        .get("styleLibrary")
        .map(|value| value.get().as_bytes())
        .unwrap_or(b"{\"styles\":[]}")
}
fn merge_library(
    edits: &mut Vec<(String, Vec<u8>)>,
    path: String,
    source: &Object<'_>,
    before: &Object<'_>,
    after: &Object<'_>,
) -> Result<(), ConfigurationBridgeError> {
    if library(before) == library(after) {
        return Ok(());
    }
    let merged = merge_json_projection(library(source), library(before), library(after), MAXIMUM)
        .map_err(|_| invalid())?;
    edits.push((path, merged));
    Ok(())
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
        let key = format!("{}/{}", row["colorScheme"].get(), row["actionSet"].get());
        let appearance = object(row["appearance"].get().as_bytes())?;
        if result.insert(key, (index, appearance)).is_some() {
            return Err(invalid());
        }
    }
    Ok(result)
}
pub(super) fn merge(
    source: &OwnedConfigurationDocument,
    before: &OwnedConfigurationDocument,
    after: &OwnedConfigurationDocument,
    profile_id: &str,
) -> Result<OwnedConfigurationDocument, ConfigurationBridgeError> {
    let index = source
        .profiles()
        .iter()
        .position(|p| {
            p["id"]
                .as_str()
                .is_some_and(|id| id.eq_ignore_ascii_case(profile_id))
        })
        .ok_or_else(invalid)?;
    let a = profile(source, index)?;
    let b = profile(before, index)?;
    let c = profile(after, index)?;
    for value in [&a, &b, &c] {
        let id: String = serde_json::from_str(value.get("id").ok_or_else(invalid)?.get())
            .map_err(|_| invalid())?;
        if !id.eq_ignore_ascii_case(profile_id) {
            return Err(invalid());
        }
    }
    let mut edits = Vec::new();
    let ac = object(a["customization"].get().as_bytes())?;
    let bc = object(b["customization"].get().as_bytes())?;
    let cc = object(c["customization"].get().as_bytes())?;
    merge_library(
        &mut edits,
        format!("/profiles/{index}/customization/styleLibrary"),
        &ac,
        &bc,
        &cc,
    )?;
    match (
        a.get("appearanceBank"),
        b.get("appearanceBank"),
        c.get("appearanceBank"),
    ) {
        (Some(original), Some(old), Some(new)) => {
            let original = rows(original)?;
            let old = rows(old)?;
            let new = rows(new)?;
            if original.keys().ne(old.keys()) || old.keys().ne(new.keys()) {
                return Err(invalid());
            }
            for (key, (ordinal, source_row)) in &original {
                merge_library(&mut edits,format!("/profiles/{index}/appearanceBank/variants/{ordinal}/appearance/styleLibrary"),source_row,&old[key].1,&new[key].1)?;
            }
        }
        (None, None, Some(bank)) => edits.push((
            format!("/profiles/{index}/appearanceBank"),
            bank.get().as_bytes().to_vec(),
        )),
        (None, None, None) => (),
        _ => return Err(invalid()),
    }
    if let Some(resources) = c.get("appearanceResources") {
        let mut pool = a
            .get("appearanceResources")
            .map(|v| object(v.get().as_bytes()))
            .transpose()?
            .unwrap_or_default();
        for (hash, value) in object(resources.get().as_bytes())? {
            if let Some(old) = pool.get(&hash) {
                let a: String = serde_json::from_str(old.get()).map_err(|_| invalid())?;
                let b: String = serde_json::from_str(value.get()).map_err(|_| invalid())?;
                if a != b {
                    return Err(invalid());
                }
            } else {
                pool.insert(hash, value);
            }
        }
        edits.push((
            format!("/profiles/{index}/appearanceResources"),
            serde_json::to_vec(&pool).map_err(|_| invalid())?,
        ));
        edits.push((
            format!("/profiles/{index}/appearanceResourceVersion"),
            b"1".to_vec(),
        ));
    }
    if let Some(updated) = c.get("updatedAt") {
        edits.push((
            format!("/profiles/{index}/updatedAt"),
            updated.get().as_bytes().to_vec(),
        ));
    }
    let patches: Vec<_> = edits
        .iter()
        .map(|(pointer, bytes)| JsonValueEdit {
            pointer,
            value: Some(bytes),
        })
        .collect();
    source.with_edits(&patches).map_err(|_| invalid())
}
