//! Lossless portable v5 configuration container. This pure codec does not change
//! authority storage, activate migrations, or advertise a peer transfer format.
use crate::{
    OwnedConfigurationDocument, ProfileArtifactCatalogRevision, ProfileArtifactContentHash,
    ProfileArtifactSelection, ProfileResourceTable, MAXIMUM_CONFIGURATION_DOCUMENT_BYTES,
};
use serde::{Deserialize, Serialize};
use serde_json::{value::RawValue, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fmt};

pub const OWNED_PROFILE_ARTIFACT_SCHEMA: &str = "com.codybontecou.thumble.owned-profile-artifact";
pub const OWNED_PROFILE_ARTIFACT_VERSION: u32 = 1;
pub const OWNED_PROFILE_ARTIFACT_HASH_ENCODING: &str = "exact-json-utf8";
/// The container and configuration share the existing portable 8 MiB ceiling.
pub const MAXIMUM_OWNED_PROFILE_ARTIFACT_BYTES: usize = MAXIMUM_CONFIGURATION_DOCUMENT_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedProfileArtifactError {
    TooLarge,
    InvalidEnvelope,
    InvalidConfiguration,
    InvalidHash,
    InvalidSelection,
    NonportableContent,
}
impl fmt::Display for OwnedProfileArtifactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::TooLarge => "owned profile artifact exceeds 8 MiB",
            Self::InvalidEnvelope => "invalid owned profile artifact envelope",
            Self::InvalidConfiguration => "invalid owned profile artifact configuration",
            Self::InvalidHash => "owned profile artifact content hash does not match",
            Self::InvalidSelection => "owned profile selection is missing",
            Self::NonportableContent => "owned profile artifact contains nonportable content",
        })
    }
}
impl std::error::Error for OwnedProfileArtifactError {}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Wire {
    schema: String,
    artifact_version: u32,
    catalog_revision: ProfileArtifactCatalogRevision,
    content_hash: ProfileArtifactContentHash,
    document: Box<RawValue>,
}
/// Keep exact inner JSON for future metadata and integers beyond the I-JSON range.
/// The digest covers these bytes, rather than a float-based normalized projection.
#[derive(Clone, Debug)]
pub struct OwnedProfileArtifact {
    document: OwnedConfigurationDocument,
    hash: ProfileArtifactContentHash,
    catalog_revision: ProfileArtifactCatalogRevision,
}
type RawObject = BTreeMap<String, Box<RawValue>>;
fn raw<T: Serialize>(value: &T) -> Result<Box<RawValue>, OwnedProfileArtifactError> {
    serde_json::value::to_raw_value(value).map_err(|_| OwnedProfileArtifactError::InvalidEnvelope)
}
fn object(bytes: &[u8]) -> Result<RawObject, OwnedProfileArtifactError> {
    serde_json::from_slice(bytes).map_err(|_| OwnedProfileArtifactError::InvalidEnvelope)
}
fn digest(bytes: &[u8]) -> ProfileArtifactContentHash {
    ProfileArtifactContentHash {
        algorithm: "sha256".into(),
        canonicalization: OWNED_PROFILE_ARTIFACT_HASH_ENCODING.into(),
        value: format!("{:x}", Sha256::digest(bytes)),
    }
}
impl OwnedProfileArtifact {
    pub fn from_configuration(
        document: &OwnedConfigurationDocument,
        selection: ProfileArtifactSelection,
        exported_at: i64,
    ) -> Result<Self, OwnedProfileArtifactError> {
        if exported_at < 0 {
            return Err(OwnedProfileArtifactError::InvalidEnvelope);
        }
        let mut root = object(document.original_bytes())?;
        let profiles: Vec<Box<RawValue>> = serde_json::from_str(root["profiles"].get())
            .map_err(|_| OwnedProfileArtifactError::InvalidConfiguration)?;
        let mut selected = Vec::new();
        let mut all_portable = Vec::new();
        let mut ids = Vec::new();
        for (value, projection) in profiles.into_iter().zip(document.profiles()) {
            let id = projection["id"]
                .as_str()
                .ok_or(OwnedProfileArtifactError::InvalidConfiguration)?;
            // Validate every source profile before selection. Invalid data cannot
            // disappear by selecting another profile; only known local launch
            // metadata is deliberately excluded from the portable copy.
            let mut profile = object(value.get().as_bytes())?;
            profile.remove("launchTarget");
            let portable = raw(&profile)?;
            validate_portable(
                &serde_json::from_str(portable.get())
                    .map_err(|_| OwnedProfileArtifactError::InvalidConfiguration)?,
                0,
                "/profiles/0",
                false,
            )?;
            let keep = match &selection {
                ProfileArtifactSelection::All => true,
                ProfileArtifactSelection::ProfileId(wanted) => wanted.eq_ignore_ascii_case(id),
            };
            all_portable.push(portable.clone());
            if keep {
                ids.push(id.to_owned());
                selected.push(portable);
            }
        }
        if selected.is_empty() {
            return Err(OwnedProfileArtifactError::InvalidSelection);
        }
        root.insert("profiles".into(), raw(&all_portable)?);
        validate_portable(
            &serde_json::from_slice(
                &serde_json::to_vec(&root)
                    .map_err(|_| OwnedProfileArtifactError::InvalidEnvelope)?,
            )
            .map_err(|_| OwnedProfileArtifactError::InvalidEnvelope)?,
            0,
            "",
            false,
        )?;
        root.insert("profiles".into(), raw(&selected)?);
        root.insert("exportedAt".into(), raw(&exported_at)?);
        if matches!(selection, ProfileArtifactSelection::ProfileId(_)) {
            let id = &ids[0];
            root.insert("activeProfileID".into(), raw(id)?);
            let default = document
                .default_profile_id()
                .filter(|v| v.eq_ignore_ascii_case(id));
            root.insert("defaultProfileID".into(), raw(&default)?);
            for (map_key, active_key) in [
                ("profileKeyBindings", "keyBindings"),
                ("profileOutputBindings", "outputBindings"),
            ] {
                let all = root
                    .get(map_key)
                    .map(|v| object(v.get().as_bytes()))
                    .transpose()?
                    .unwrap_or_default();
                let filtered: RawObject = all
                    .into_iter()
                    .filter(|(key, _)| key.eq_ignore_ascii_case(id))
                    .collect();
                let active = if let Some(value) = filtered.values().next() {
                    raw(value)?
                } else if document.active_profile_id().eq_ignore_ascii_case(id) {
                    root.get(active_key)
                        .map(raw)
                        .transpose()?
                        .unwrap_or(raw(&serde_json::json!({}))?)
                } else {
                    raw(&serde_json::json!({}))?
                };
                root.insert(map_key.into(), raw(&filtered)?);
                root.insert(active_key.into(), active);
            }
        }
        let bytes =
            serde_json::to_vec(&root).map_err(|_| OwnedProfileArtifactError::InvalidEnvelope)?;
        Self::validated(&bytes, ProfileArtifactCatalogRevision::default())
    }
    fn validated(
        bytes: &[u8],
        catalog_revision: ProfileArtifactCatalogRevision,
    ) -> Result<Self, OwnedProfileArtifactError> {
        let document = OwnedConfigurationDocument::decode(bytes)
            .map_err(|_| OwnedProfileArtifactError::InvalidConfiguration)?;
        let value = thumble_protocol::decode_unique_json(bytes)
            .map_err(|_| OwnedProfileArtifactError::InvalidConfiguration)?;
        validate_portable(&value, 0, "", false)?;
        let artifact = Self {
            hash: digest(bytes),
            document,
            catalog_revision,
        };
        // Check the actual encoded container size, including hash and metadata.
        artifact.encode_json()?;
        Ok(artifact)
    }
    pub fn decode_json(bytes: &[u8]) -> Result<Self, OwnedProfileArtifactError> {
        if bytes.len() > MAXIMUM_OWNED_PROFILE_ARTIFACT_BYTES {
            return Err(OwnedProfileArtifactError::TooLarge);
        }
        thumble_protocol::validate_unique_json(bytes)
            .map_err(|_| OwnedProfileArtifactError::InvalidEnvelope)?;
        let wire: Wire = serde_json::from_slice(bytes)
            .map_err(|_| OwnedProfileArtifactError::InvalidEnvelope)?;
        if wire.schema != OWNED_PROFILE_ARTIFACT_SCHEMA
            || wire.artifact_version != OWNED_PROFILE_ARTIFACT_VERSION
            || wire.catalog_revision != ProfileArtifactCatalogRevision::default()
        {
            return Err(OwnedProfileArtifactError::InvalidEnvelope);
        }
        if wire.content_hash != digest(wire.document.get().as_bytes()) {
            return Err(OwnedProfileArtifactError::InvalidHash);
        }
        Self::validated(wire.document.get().as_bytes(), wire.catalog_revision)
    }
    pub fn encode_json(&self) -> Result<Vec<u8>, OwnedProfileArtifactError> {
        let wire = Wire {
            schema: OWNED_PROFILE_ARTIFACT_SCHEMA.into(),
            artifact_version: OWNED_PROFILE_ARTIFACT_VERSION,
            catalog_revision: self.catalog_revision,
            content_hash: self.hash.clone(),
            document: RawValue::from_string(
                String::from_utf8(self.document.original_bytes().to_vec())
                    .map_err(|_| OwnedProfileArtifactError::InvalidEnvelope)?,
            )
            .map_err(|_| OwnedProfileArtifactError::InvalidEnvelope)?,
        };
        let bytes =
            serde_json::to_vec(&wire).map_err(|_| OwnedProfileArtifactError::InvalidEnvelope)?;
        if bytes.len() > MAXIMUM_OWNED_PROFILE_ARTIFACT_BYTES {
            return Err(OwnedProfileArtifactError::TooLarge);
        }
        Ok(bytes)
    }
    pub fn document(&self) -> &OwnedConfigurationDocument {
        &self.document
    }
    pub fn content_hash(&self) -> &ProfileArtifactContentHash {
        &self.hash
    }
}

fn validate_portable(
    value: &Value,
    depth: usize,
    path: &str,
    has_resources: bool,
) -> Result<(), OwnedProfileArtifactError> {
    let invalid = OwnedProfileArtifactError::NonportableContent;
    if depth > 64 {
        return Err(invalid);
    }
    let profile_root = path
        .strip_prefix("/profiles/")
        .is_some_and(|s| s.parse::<usize>().is_ok());
    let has_resources = if profile_root {
        ProfileResourceTable::from_profile(value)
            .map_err(|_| OwnedProfileArtifactError::NonportableContent)?
            .is_some()
    } else {
        has_resources
    };
    match value {
        Value::Object(map) => {
            if map.len() > 4096 {
                return Err(invalid);
            }
            for (key, child) in map {
                if key.len() > 256 {
                    return Err(invalid);
                }
                // Embedded data is portable only as a validated profile resource
                // reference. Resource table integrity is checked by the document
                // codec; no arbitrary Base64 payload is accepted through `data`.
                if has_resources
                    && path.starts_with("/profiles/")
                    && key == "data"
                    && child.as_object().is_some_and(|v| {
                        v.len() == 1 && v.get("resourceSHA256").is_some_and(Value::is_string)
                    })
                {
                    continue;
                }
                let normalized: String = key
                    .chars()
                    .filter(char::is_ascii_alphanumeric)
                    .map(|c| c.to_ascii_lowercase())
                    .collect();
                if ["appearanceresources", "appearanceresourceversion"]
                    .contains(&normalized.as_str())
                {
                    if !profile_root
                        || !["appearanceResources", "appearanceResourceVersion"]
                            .contains(&key.as_str())
                    {
                        return Err(invalid);
                    }
                    if key == "appearanceResources" {
                        // Only this profile's validated table receives the binary-data exception.
                        continue;
                    }
                }
                if crate::profile_artifact::is_forbidden_field_name(key) && !child.is_null() {
                    return Err(invalid);
                }
                validate_portable(
                    child,
                    depth + 1,
                    &format!("{path}/{}", key.replace('~', "~0").replace('/', "~1")),
                    has_resources,
                )?;
            }
        }
        Value::Array(values) => {
            if values.len() > 4096 {
                return Err(invalid);
            }
            for (index, child) in values.iter().enumerate() {
                validate_portable(child, depth + 1, &format!("{path}/{index}"), has_resources)?;
            }
        }
        Value::String(value) if value.len() > 256 * 1024 => return Err(invalid),
        _ => {}
    }
    Ok(())
}
