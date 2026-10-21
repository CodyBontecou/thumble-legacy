//! Exact reusable layout/appearance interchange, independent of profile identity.
use crate::{OwnedConfigurationDocument, MAXIMUM_CONFIGURATION_DOCUMENT_BYTES};
use serde_json::{value::RawValue, Value};
use std::{collections::BTreeMap, sync::Arc};
pub const CUSTOMIZATION_ARTIFACT_SCHEMA: &str = "com.codybontecou.thumble.customization";
type Object = BTreeMap<String, Box<RawValue>>;
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CustomizationArtifactError {
    Invalid,
    TooLarge,
    Portrait,
    RequiresPreparation,
}
impl std::fmt::Display for CustomizationArtifactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "customization interchange failed: {self:?}")
    }
}
impl std::error::Error for CustomizationArtifactError {}
#[derive(Clone)]
pub struct CustomizationArtifact {
    storage: Arc<Storage>,
}
struct Storage {
    bytes: Vec<u8>,
    fields: Object,
    pooled: bool,
}
fn object(bytes: &[u8]) -> Result<Object, CustomizationArtifactError> {
    thumble_protocol::validate_unique_json(bytes)
        .map_err(|_| CustomizationArtifactError::Invalid)?;
    serde_json::from_slice(bytes).map_err(|_| CustomizationArtifactError::Invalid)
}
impl CustomizationArtifact {
    pub fn decode(bytes: &[u8]) -> Result<Self, CustomizationArtifactError> {
        if bytes.len() > MAXIMUM_CONFIGURATION_DOCUMENT_BYTES {
            return Err(CustomizationArtifactError::TooLarge);
        }
        let fields = object(bytes)?;
        let pooled = fields.contains_key("schema");
        if pooled {
            let value: Value =
                serde_json::from_slice(bytes).map_err(|_| CustomizationArtifactError::Invalid)?;
            if value["schema"] != CUSTOMIZATION_ARTIFACT_SCHEMA
                || value["version"] != 1
                || !value["customization"].is_object()
                || !value["appearanceBank"].is_object()
                || fields.keys().any(|k| {
                    ![
                        "schema",
                        "version",
                        "customization",
                        "appearanceBank",
                        "appearanceResourceVersion",
                        "appearanceResources",
                    ]
                    .contains(&k.as_str())
                })
            {
                return Err(CustomizationArtifactError::Invalid);
            }
        } else if fields.is_empty() {
            return Err(CustomizationArtifactError::Invalid);
        }
        let result = Self {
            storage: Arc::new(Storage {
                bytes: bytes.to_vec(),
                fields,
                pooled,
            }),
        };
        if pooled {
            result.owned_document()?;
        }
        Ok(result)
    }
    pub fn original_bytes(&self) -> &[u8] {
        &self.storage.bytes
    }
    pub fn is_pooled(&self) -> bool {
        self.storage.pooled
    }
    pub fn layout_bytes(&self) -> &[u8] {
        if self.storage.pooled {
            self.storage.fields["customization"].get().as_bytes()
        } else {
            self.original_bytes()
        }
    }
    pub fn synthetic_profile(&self, owned: bool) -> Result<Vec<u8>, CustomizationArtifactError> {
        let mut profile = Object::new();
        let id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, self.layout_bytes());
        for (key, text) in [
            ("id", format!("\"{id}\"")),
            ("name", "\"Reusable layout\"".into()),
            ("outputMode", "\"custom\"".into()),
            ("updatedAt", "0".into()),
        ] {
            profile.insert(
                key.into(),
                RawValue::from_string(text).map_err(|_| CustomizationArtifactError::Invalid)?,
            );
        }
        profile.insert(
            "customization".into(),
            RawValue::from_string(
                String::from_utf8(self.layout_bytes().to_vec())
                    .map_err(|_| CustomizationArtifactError::Invalid)?,
            )
            .map_err(|_| CustomizationArtifactError::Invalid)?,
        );
        if owned {
            profile.insert(
                "profileFormatVersion".into(),
                RawValue::from_string("5".into()).unwrap(),
            );
        } else {
            profile.insert(
                "orientationPreference".into(),
                RawValue::from_string("\"landscape\"".into()).unwrap(),
            );
        }
        for name in [
            "appearanceBank",
            "appearanceResourceVersion",
            "appearanceResources",
        ] {
            if let Some(value) = self
                .storage
                .fields
                .get(name)
                .filter(|_| self.storage.pooled)
            {
                profile.insert(name.into(), value.clone());
            }
        }
        serde_json::to_vec(&profile).map_err(|_| CustomizationArtifactError::Invalid)
    }
    pub fn owned_document(&self) -> Result<OwnedConfigurationDocument, CustomizationArtifactError> {
        if !self.storage.pooled {
            return Err(CustomizationArtifactError::RequiresPreparation);
        }
        let profile = pool_inline_resources(&self.synthetic_profile(true)?)?;
        let id = uuid::Uuid::new_v5(&uuid::Uuid::NAMESPACE_OID, self.layout_bytes());
        let text=format!("{{\"schema\":\"com.codybontecou.pocketpad.keypad-configuration\",\"version\":5,\"activeProfileID\":\"{id}\",\"defaultProfileID\":null,\"profiles\":[{}]}}",std::str::from_utf8(&profile).map_err(|_|CustomizationArtifactError::Invalid)?);
        OwnedConfigurationDocument::decode(text.as_bytes())
            .map_err(|_| CustomizationArtifactError::Invalid)
    }
    pub fn from_profile(
        bytes: &[u8],
        variant: &str,
        owned: bool,
    ) -> Result<Self, CustomizationArtifactError> {
        if bytes.len() > MAXIMUM_CONFIGURATION_DOCUMENT_BYTES {
            return Err(CustomizationArtifactError::TooLarge);
        }
        if !["primary", "landscape", "portrait"].contains(&variant) {
            return Err(CustomizationArtifactError::Invalid);
        }
        if owned && variant == "portrait" {
            return Err(CustomizationArtifactError::Portrait);
        }
        let normalized = pool_inline_resources(bytes)?;
        let fields = object(&normalized)?;
        let layout = if variant == "primary" {
            None
        } else {
            fields
                .get(&format!("{variant}Customization"))
                .filter(|v| v.get() != "null")
        }
        .or_else(|| fields.get("customization"))
        .ok_or(CustomizationArtifactError::Invalid)?;
        if fields
            .get("appearanceBank")
            .is_some_and(|v| v.get() != "null")
        {
            let mut result = Object::new();
            result.insert(
                "schema".into(),
                serde_json::value::to_raw_value(CUSTOMIZATION_ARTIFACT_SCHEMA).unwrap(),
            );
            result.insert("version".into(), RawValue::from_string("1".into()).unwrap());
            result.insert("customization".into(), layout.clone());
            for name in [
                "appearanceBank",
                "appearanceResourceVersion",
                "appearanceResources",
            ] {
                if let Some(v) = fields.get(name) {
                    result.insert(name.into(), v.clone());
                }
            }
            if let Some(pool) = result.get("appearanceResources") {
                let value = serde_json::to_value(&result)
                    .map_err(|_| CustomizationArtifactError::Invalid)?;
                let table = crate::ProfileResourceTable::from_profile(&value)
                    .map_err(|_| CustomizationArtifactError::Invalid)?
                    .ok_or(CustomizationArtifactError::Invalid)?;
                let pruned = table
                    .pruning_unused(&value)
                    .map_err(|_| CustomizationArtifactError::Invalid)?;
                let mut entries = object(pool.get().as_bytes())?;
                let used = pruned["appearanceResources"]
                    .as_object()
                    .ok_or(CustomizationArtifactError::Invalid)?;
                entries.retain(|digest, _| used.contains_key(digest));
                result.insert(
                    "appearanceResources".into(),
                    serde_json::value::to_raw_value(&entries)
                        .map_err(|_| CustomizationArtifactError::Invalid)?,
                );
            }
            let bytes =
                serde_json::to_vec(&result).map_err(|_| CustomizationArtifactError::Invalid)?;
            Self::decode(&bytes)
        } else {
            Self::decode(layout.get().as_bytes())
        }
    }
}

/// Pool only native Data fields. Opaque extensions are traversed by neither
/// this schema walk nor the native Data encoder, so their exact bytes survive.
fn pool_inline_resources(bytes: &[u8]) -> Result<Vec<u8>, CustomizationArtifactError> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use sha2::{Digest, Sha256};
    use thumble_protocol::{edit_json_values, JsonValueEdit};
    #[derive(Clone, Copy)]
    enum Kind {
        Profile,
        Surface,
        Control,
        Visual,
        State,
        Fill,
        Data,
    }
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| CustomizationArtifactError::Invalid)?;
    if !value["appearanceBank"].is_object() {
        return Ok(bytes.to_vec());
    }
    crate::ProfileResourceTable::from_profile(&value)
        .map_err(|_| CustomizationArtifactError::Invalid)?;
    let mut pool = object(bytes)?
        .remove("appearanceResources")
        .map(|v| object(v.get().as_bytes()))
        .transpose()?
        .unwrap_or_default();
    let mut pending = vec![(&value, String::new(), Kind::Profile)];
    let mut replacements: Vec<(String, Vec<u8>)> = Vec::new();
    while let Some((node, path, kind)) = pending.pop() {
        let mut child = |name: &str, kind| {
            if let Some(v) = node.get(name) {
                pending.push((v, format!("{path}/{name}"), kind));
            }
        };
        match kind {
            Kind::Profile => {
                child("customization", Kind::Surface);
                if let Some(variants) = node["appearanceBank"]["variants"].as_array() {
                    for (i, row) in variants.iter().enumerate() {
                        pending.push((
                            &row["appearance"],
                            format!("{path}/appearanceBank/variants/{i}/appearance"),
                            Kind::Surface,
                        ));
                    }
                }
                for name in ["bookmarkData", "iconPNGData"] {
                    if let Some(v) = node["launchTarget"].get(name) {
                        pending.push((v, format!("{path}/launchTarget/{name}"), Kind::Data));
                    }
                }
            }
            Kind::Surface => {
                for name in [
                    "backgroundFillStyle",
                    "backgroundLightFillStyle",
                    "backgroundDarkFillStyle",
                ] {
                    child(name, Kind::Fill);
                }
                child("topBar", Kind::Control);
                child("topBarActivationRegion", Kind::Control);
                for name in ["controls", "controlBar"] {
                    if let Some(rows) = node[name].as_object() {
                        for (id, row) in rows {
                            let escaped = id.replace('~', "~0").replace('/', "~1");
                            pending.push((row, format!("{path}/{name}/{escaped}"), Kind::Control));
                        }
                    }
                }
                for (name, field, kind) in [
                    ("elements", "layout", Kind::Control),
                    ("customButtons", "layout", Kind::Control),
                    ("artworkLayers", "fillStyle", Kind::Fill),
                    ("controlBarItemCustomizations", "appearance", Kind::Control),
                ] {
                    if let Some(rows) = node[name].as_array() {
                        for (i, row) in rows.iter().enumerate() {
                            if let Some(v) = row.get(field) {
                                pending.push((v, format!("{path}/{name}/{i}/{field}"), kind));
                            }
                        }
                    }
                }
                if let Some(rows) = node["buttonCustomizations"].as_array() {
                    for (i, row) in rows.iter().enumerate().skip(1).step_by(2) {
                        pending.push((
                            row,
                            format!("{path}/buttonCustomizations/{i}"),
                            Kind::Control,
                        ));
                    }
                }
                for (library, collection, field, kind) in [
                    ("assetLibrary", "assets", "data", Kind::Data),
                    ("styleLibrary", "styles", "visualStyle", Kind::Visual),
                ] {
                    if let Some(rows) = node[library][collection].as_array() {
                        for (i, row) in rows.iter().enumerate() {
                            if let Some(v) = row.get(field) {
                                pending.push((
                                    v,
                                    format!("{path}/{library}/{collection}/{i}/{field}"),
                                    kind,
                                ));
                            }
                        }
                    }
                }
            }
            Kind::Control => {
                for name in ["fillStyle", "lightFillStyle", "darkFillStyle"] {
                    child(name, Kind::Fill);
                }
                child("visualStyle", Kind::Visual);
            }
            Kind::Visual => {
                for name in ["normal", "pressed", "active", "disabled"] {
                    child(name, Kind::State);
                }
            }
            Kind::State => child("fillStyle", Kind::Fill),
            Kind::Fill => {
                if node["kind"] == "image" {
                    if let Some(v) = node["image"].get("data") {
                        pending.push((v, format!("{path}/image/data"), Kind::Data));
                    }
                }
            }
            Kind::Data => {
                if let Some(encoded) = node.as_str() {
                    let data = STANDARD
                        .decode(encoded)
                        .map_err(|_| CustomizationArtifactError::Invalid)?;
                    if STANDARD.encode(&data) != encoded {
                        return Err(CustomizationArtifactError::Invalid);
                    }
                    let digest = format!("{:x}", Sha256::digest(&data));
                    pool.insert(
                        digest.clone(),
                        serde_json::value::to_raw_value(encoded).unwrap(),
                    );
                    replacements.push((
                        path,
                        serde_json::to_vec(&serde_json::json!({"resourceSHA256": digest})).unwrap(),
                    ));
                }
            }
        }
    }
    // Keep existing version spelling and raw resource values when no conversion
    // is needed. The owning decoder validates envelope versions and hashes.
    if replacements.is_empty() && value.get("appearanceResources").is_some() {
        return Ok(bytes.to_vec());
    }
    replacements.push((
        "/appearanceResources".into(),
        serde_json::to_vec(&pool).map_err(|_| CustomizationArtifactError::Invalid)?,
    ));
    if value.get("appearanceResourceVersion").is_none() {
        replacements.push(("/appearanceResourceVersion".into(), b"1".to_vec()));
    }
    let mut result = bytes.to_vec();
    for batch in replacements.chunks(512) {
        let edits: Vec<_> = batch
            .iter()
            .map(|(pointer, value)| JsonValueEdit {
                pointer,
                value: Some(value.as_slice()),
            })
            .collect();
        result = edit_json_values(&result, &edits, MAXIMUM_CONFIGURATION_DOCUMENT_BYTES)
            .map_err(|_| CustomizationArtifactError::TooLarge)?;
    }
    Ok(result)
}

fn decimal_key(text: &str) -> Option<(String, i64)> {
    let negative = text.starts_with('-');
    let text = text.strip_prefix('-').unwrap_or(text);
    let (mantissa, exponent) = text
        .split_once(['e', 'E'])
        .map(|(a, b)| Some((a, b.parse::<i64>().ok()?)))
        .unwrap_or(Some((text, 0)))?;
    let fraction = mantissa.split_once('.').map(|(_, b)| b.len()).unwrap_or(0);
    let mut digits = mantissa.replace('.', "");
    let mut exponent = exponent.checked_sub(i64::try_from(fraction).ok()?)?;
    digits = digits.trim_start_matches('0').to_owned();
    if digits.is_empty() {
        return Some(("0".into(), 0));
    }
    let trailing = digits.len() - digits.trim_end_matches('0').len();
    digits.truncate(digits.len() - trailing);
    exponent = exponent.checked_add(i64::try_from(trailing).ok()?)?;
    if negative {
        digits.insert(0, '-');
    }
    Some((digits, exponent))
}
impl CustomizationArtifact {
    /// Legacy native DTOs carry i64/binary64, including native 17-digit float
    /// spellings. Higher-precision extensions require owned exact-byte import.
    pub fn legacy_numbers_are_supported(&self) -> bool {
        if self.is_pooled() {
            return false;
        }
        let Ok(root) = serde_json::from_str::<&RawValue>(
            std::str::from_utf8(self.original_bytes()).unwrap_or(""),
        ) else {
            return false;
        };
        let mut pending = vec![root];
        while let Some(value) = pending.pop() {
            let text = value.get();
            match text.as_bytes().first() {
                Some(b'{') => {
                    let Ok(fields) = serde_json::from_str::<BTreeMap<String, &RawValue>>(text)
                    else {
                        return false;
                    };
                    pending.extend(fields.values().copied());
                }
                Some(b'[') => {
                    let Ok(rows) = serde_json::from_str::<Vec<&RawValue>>(text) else {
                        return false;
                    };
                    pending.extend(rows);
                }
                Some(b'-' | b'0'..=b'9') => {
                    if text.parse::<i64>().is_ok() {
                        continue;
                    }
                    let Ok(number) = text.parse::<f64>() else {
                        return false;
                    };
                    if !number.is_finite() {
                        return false;
                    }
                    let Ok(encoded) = serde_json::to_string(&number) else {
                        return false;
                    };
                    let Some((digits, _)) = decimal_key(text) else {
                        return false;
                    };
                    if number == 0.0 && digits != "0" {
                        return false;
                    }
                    if decimal_key(text) != decimal_key(&encoded)
                        && (!(text.contains('.') || text.contains(['e', 'E']))
                            || digits.trim_start_matches('-').len() > 17)
                    {
                        return false;
                    }
                }
                _ => {}
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inline_bank_images_are_pooled_without_rewriting_extensions() {
        let doc: Value = serde_json::from_slice(include_bytes!(
            "../../../fixtures/owned-configuration/v1/document.json"
        ))
        .unwrap();
        let mut profile = doc["profiles"][0].clone();
        let exported = CustomizationArtifact::from_profile(
            &serde_json::to_vec(&profile).unwrap(),
            "primary",
            true,
        )
        .unwrap();
        let mut malformed: Value = serde_json::from_slice(exported.original_bytes()).unwrap();
        malformed
            .as_object_mut()
            .unwrap()
            .remove("appearanceResourceVersion");
        assert!(CustomizationArtifact::decode(&serde_json::to_vec(&malformed).unwrap()).is_err());
        let resources = profile["appearanceResources"].clone();
        fn expand(node: &mut Value, resources: &Value) {
            if let Some(id) = node.get("resourceSHA256").and_then(Value::as_str) {
                *node = resources[id].clone();
                return;
            }
            match node {
                Value::Object(o) => {
                    for v in o.values_mut() {
                        expand(v, resources);
                    }
                }
                Value::Array(a) => {
                    for v in a {
                        expand(v, resources);
                    }
                }
                _ => {}
            }
        }
        profile
            .as_object_mut()
            .unwrap()
            .remove("appearanceResources");
        profile
            .as_object_mut()
            .unwrap()
            .remove("appearanceResourceVersion");
        expand(&mut profile, &resources);
        profile["customization"]["futureOpaque"] = serde_json::json!({"kind":"image","image":{"data":"opaque, not native data"},"precise":"PRECISE"});
        let bytes = serde_json::to_vec(&profile).unwrap();
        let bytes = String::from_utf8(bytes)
            .unwrap()
            .replace("\"PRECISE\"", "0.12345678901234567890123456789");
        let file = CustomizationArtifact::from_profile(bytes.as_bytes(), "primary", true).unwrap();
        assert!(std::str::from_utf8(file.original_bytes())
            .unwrap()
            .contains("0.12345678901234567890123456789"));
        let owned = file.owned_document().unwrap();
        assert_eq!(
            owned.profiles()[0]["customization"]["futureOpaque"]["image"]["data"],
            "opaque, not native data"
        );
        assert!(owned.profiles()[0]["appearanceResources"]
            .as_object()
            .unwrap()
            .values()
            .any(|v| resources
                .as_object()
                .unwrap()
                .values()
                .any(|original| original == v)));
        // A directly received inline envelope takes the same normalization path.
        let mut envelope: Value = serde_json::from_slice(file.original_bytes()).unwrap();
        let pool = envelope
            .as_object_mut()
            .unwrap()
            .remove("appearanceResources")
            .unwrap();
        envelope
            .as_object_mut()
            .unwrap()
            .remove("appearanceResourceVersion");
        expand(&mut envelope, &pool);
        let received =
            CustomizationArtifact::decode(&serde_json::to_vec(&envelope).unwrap()).unwrap();
        assert!(
            received.owned_document().unwrap().profiles()[0]["appearanceResources"].is_object()
        );
    }
    #[test]
    fn banked_export_preserves_exact_layout_and_all_resources() {
        let root = object(include_bytes!(
            "../../../fixtures/owned-configuration/v1/document.json"
        ))
        .unwrap();
        let profiles: Vec<Box<RawValue>> = serde_json::from_str(root["profiles"].get()).unwrap();
        let artifact =
            CustomizationArtifact::from_profile(profiles[0].get().as_bytes(), "landscape", true)
                .unwrap();
        assert!(artifact.is_pooled());
        let doc = artifact.owned_document().unwrap();
        assert_eq!(
            doc.profiles()[0]["appearanceBank"]["variants"]
                .as_array()
                .unwrap()
                .len(),
            8
        );
        assert_eq!(
            doc.profiles()[0]["appearanceResources"]
                .as_object()
                .unwrap()
                .len(),
            1
        );
        let original: Value = serde_json::from_str(profiles[0].get()).unwrap();
        let bookmark = original["launchTarget"]["bookmarkData"]["resourceSHA256"]
            .as_str()
            .unwrap();
        assert!(doc.profiles()[0]["appearanceResources"]
            .get(bookmark)
            .is_none());
        assert!(CustomizationArtifact::from_profile(
            profiles[0].get().as_bytes(),
            "portrait",
            true
        )
        .is_err());
        let plain =
            CustomizationArtifact::from_profile(profiles[1].get().as_bytes(), "primary", true)
                .unwrap();
        assert!(!plain.is_pooled());
        let original = object(profiles[1].get().as_bytes()).unwrap();
        assert_eq!(
            plain.original_bytes(),
            original["customization"].get().as_bytes()
        );
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;
    #[test]
    fn exact_extensions_and_future_resource_references_survive_export() {
        let root = object(include_bytes!(
            "../../../fixtures/owned-configuration/v1/document.json"
        ))
        .unwrap();
        let profiles: Vec<Box<RawValue>> = serde_json::from_str(root["profiles"].get()).unwrap();
        let mut profile = object(profiles[0].get().as_bytes()).unwrap();
        let mut layout = object(profile["customization"].get().as_bytes()).unwrap();
        layout.insert(
            "futurePrecision".into(),
            RawValue::from_string("0.12345678901234567890123456789".into()).unwrap(),
        );
        let value: Value = serde_json::from_str(profiles[0].get()).unwrap();
        layout.insert(
            "futureResource".into(),
            serde_json::value::to_raw_value(&value["launchTarget"]["bookmarkData"]).unwrap(),
        );
        profile.insert(
            "customization".into(),
            serde_json::value::to_raw_value(&layout).unwrap(),
        );
        let file = CustomizationArtifact::from_profile(
            &serde_json::to_vec(&profile).unwrap(),
            "primary",
            true,
        )
        .unwrap();
        assert!(std::str::from_utf8(file.original_bytes())
            .unwrap()
            .contains("0.12345678901234567890123456789"));
        let doc = file.owned_document().unwrap();
        assert_eq!(
            doc.profiles()[0]["appearanceResources"]
                .as_object()
                .unwrap()
                .len(),
            2
        );
        let mut bad: Value = serde_json::from_slice(file.original_bytes()).unwrap();
        bad["version"] = serde_json::json!(2);
        assert!(CustomizationArtifact::decode(&serde_json::to_vec(&bad).unwrap()).is_err());
        bad["version"] = serde_json::json!(1);
        bad["launchTarget"] = serde_json::json!({"displayName":"must not disappear"});
        assert!(CustomizationArtifact::decode(&serde_json::to_vec(&bad).unwrap()).is_err());
        assert!(CustomizationArtifact::decode(br#"{"elements":[],"elements":[]}"#).is_err());
        assert!(CustomizationArtifact::decode(&vec![
            b' ';
            MAXIMUM_CONFIGURATION_DOCUMENT_BYTES + 1
        ])
        .is_err());
    }
    #[test]
    fn legacy_precision_gate_uses_decimal_values_instead_of_float_equality() {
        for text in [
            r#"{"value":0.1,"nested":[9223372036854775807,1e-7,-0.0]}"#,
            r#"{"value":1.000000000000000000}"#,
            r#"{"centerX":0.90000000000000002}"#,
        ] {
            assert!(CustomizationArtifact::decode(text.as_bytes())
                .unwrap()
                .legacy_numbers_are_supported());
        }
        for text in [
            r#"{"value":0.12345678901234567890123456789}"#,
            r#"{"value":18446744073709551617}"#,
        ] {
            assert!(!CustomizationArtifact::decode(text.as_bytes())
                .unwrap()
                .legacy_numbers_are_supported());
        }
    }
}
