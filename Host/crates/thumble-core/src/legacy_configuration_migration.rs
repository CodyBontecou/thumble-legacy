//! Pure, streaming v4 -> v5 staging. Live authority/peer activation is separate.
use crate::{
    ConfigurationDocument, OwnedConfigurationDocument,
    MAXIMUM_CONFIGURATION_DOCUMENT_BYTES as MAXIMUM,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use serde_json::{value::RawValue, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    sync::Arc,
};
use thumble_protocol::{edit_json_values, merge_json_projection, JsonValueEdit};
use uuid::Uuid;

const SCHEMA: &str = "com.codybontecou.pocketpad.keypad-configuration";
const RETIRED: &[&str] = &[
    "skinReference",
    "skinBaselineCustomization",
    "landscapeSkinBaselineCustomization",
    "portraitSkinBaselineCustomization",
    "landscapeCustomization",
    "portraitCustomization",
    "orientationPreference",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyMigrationError {
    TooLarge,
    InvalidSource,
    InvalidPreparation,
    PreparationFailed,
    OpaqueConflict,
    InvalidDestination,
}
impl fmt::Display for LegacyMigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::TooLarge => "legacy migration exceeds its byte budget",
            Self::InvalidSource => "legacy migration source failed validation",
            Self::InvalidPreparation => "native preparation does not match its source",
            Self::PreparationFailed => "native profile preparation requires recovery",
            Self::OpaqueConflict => "legacy migration would overwrite opaque data",
            Self::InvalidDestination => "legacy migration destination failed validation",
        })
    }
}
impl std::error::Error for LegacyMigrationError {}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum LegacySourceLayout {
    #[serde(rename = "customization")]
    Primary,
    #[serde(rename = "landscapeCustomization")]
    Landscape,
    #[serde(rename = "portraitCustomization")]
    Portrait,
}
impl LegacySourceLayout {
    fn field(&self) -> &'static str {
        match self {
            Self::Primary => "customization",
            Self::Landscape => "landscapeCustomization",
            Self::Portrait => "portraitCustomization",
        }
    }
}

/// Only the correlated decoder can construct a preparation. Native projections
/// are not authoritative profile replacements; original raw fields stay below.
pub struct NativeLegacyPreparation {
    profile_id: Uuid,
    source_sha256: String,
    package_sha256: Option<String>,
    source_layout: LegacySourceLayout,
    source_orientation: String,
    reviews: Vec<String>,
    before: Vec<u8>,
    after: Vec<u8>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NativeResponse {
    schema_version: u32,
    #[serde(rename = "invocationID")]
    invocation_id: Uuid,
    #[serde(rename = "profileID")]
    profile_id: Uuid,
    #[serde(rename = "sourceSHA256")]
    source_sha256: String,
    #[serde(rename = "packageSHA256")]
    package_sha256: Option<String>,
    source_layout: LegacySourceLayout,
    source_orientation: String,
    review_reasons: Vec<String>,
    before_customization_data: String,
    after_profile_data: String,
}
impl NativeLegacyPreparation {
    pub fn decode(
        source: &[u8],
        invocation: Uuid,
        package_sha256: Option<&str>,
        response: &[u8],
    ) -> Result<Self, LegacyMigrationError> {
        if source.len() > MAXIMUM || response.len() > 24 * 1024 * 1024 {
            return Err(LegacyMigrationError::TooLarge);
        }
        let packet: NativeResponse = thumble_protocol::decode_unique_json(response)
            .map_err(|_| LegacyMigrationError::InvalidPreparation)?;
        let profile: Value = thumble_protocol::decode_unique_json(source)
            .map_err(|_| LegacyMigrationError::InvalidSource)?;
        let id = profile
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| Uuid::parse_str(id).ok())
            .ok_or(LegacyMigrationError::InvalidSource)?;
        let has_skin = profile
            .get("skinReference")
            .is_some_and(|value| !value.is_null());
        if packet.schema_version != 1
            || packet.invocation_id != invocation
            || packet.profile_id != id
            || packet.source_sha256 != format!("{:x}", Sha256::digest(source))
            || packet.package_sha256.as_deref() != package_sha256
            || package_sha256.is_some_and(|hash| {
                hash.len() != 64
                    || !hash
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
            || has_skin != package_sha256.is_some()
            || !["landscape", "portrait"].contains(&packet.source_orientation.as_str())
            || packet.review_reasons.len() > 3
            || packet.review_reasons.iter().collect::<BTreeSet<_>>().len()
                != packet.review_reasons.len()
            || packet.review_reasons.iter().any(|reason| {
                ![
                    "portraitArranged",
                    "alternateLayoutArchived",
                    "canvasOrientationCorrected",
                ]
                .contains(&reason.as_str())
            })
        {
            return Err(LegacyMigrationError::InvalidPreparation);
        }
        let before = decode_projection(&packet.before_customization_data)?;
        let after = decode_projection(&packet.after_profile_data)?;
        let staged: Value = thumble_protocol::decode_unique_json(&after)
            .map_err(|_| LegacyMigrationError::InvalidPreparation)?;
        if staged
            .get("id")
            .and_then(Value::as_str)
            .and_then(|id| Uuid::parse_str(id).ok())
            != Some(id)
            || staged.get("appearanceBank").is_some_and(Value::is_object) != has_skin
        {
            return Err(LegacyMigrationError::InvalidPreparation);
        }
        Ok(Self {
            profile_id: id,
            source_sha256: packet.source_sha256,
            package_sha256: packet.package_sha256,
            source_layout: packet.source_layout,
            source_orientation: packet.source_orientation,
            reviews: packet.review_reasons,
            before,
            after,
        })
    }
}
fn decode_projection(encoded: &str) -> Result<Vec<u8>, LegacyMigrationError> {
    if encoded.len() > MAXIMUM.div_ceil(3) * 4 {
        return Err(LegacyMigrationError::TooLarge);
    }
    let data = STANDARD
        .decode(encoded)
        .map_err(|_| LegacyMigrationError::InvalidPreparation)?;
    if data.len() > MAXIMUM {
        return Err(LegacyMigrationError::TooLarge);
    }
    if STANDARD.encode(&data) != encoded {
        return Err(LegacyMigrationError::InvalidPreparation);
    }
    thumble_protocol::validate_unique_json(&data)
        .map_err(|_| LegacyMigrationError::InvalidPreparation)?;
    Ok(data)
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LegacyMigrationReview {
    pub profile_id: Uuid,
    pub source_sha256: String,
    pub package_sha256: Option<String>,
    pub source_layout: LegacySourceLayout,
    pub source_orientation: String,
    pub reasons: Vec<String>,
}
/// Immutable staging, with the exact source retained for the later transaction.
/// Constructing this never writes files, changes format offers, or installs data.
pub struct LegacyMigrationCandidate {
    original: Arc<[u8]>,
    destination: OwnedConfigurationDocument,
    reviews: Vec<LegacyMigrationReview>,
}
impl LegacyMigrationCandidate {
    pub fn original_bytes(&self) -> &[u8] {
        &self.original
    }
    pub fn destination(&self) -> &OwnedConfigurationDocument {
        &self.destination
    }
    pub fn reviews(&self) -> &[LegacyMigrationReview] {
        &self.reviews
    }
}

/// Prepare one profile at a time so intermediate native responses cannot grow
/// with the whole catalog. Callback failures return no partial destination.
pub fn prepare_legacy_configuration(
    source: &[u8],
    mut prepare: impl FnMut(&[u8]) -> Result<NativeLegacyPreparation, LegacyMigrationError>,
) -> Result<LegacyMigrationCandidate, LegacyMigrationError> {
    if source.len() > MAXIMUM {
        return Err(LegacyMigrationError::TooLarge);
    }
    let root: Value = thumble_protocol::decode_unique_json(source)
        .map_err(|_| LegacyMigrationError::InvalidSource)?;
    if root.get("schema").and_then(Value::as_str) != Some(SCHEMA)
        || root.get("version").and_then(Value::as_f64) != Some(4.0)
    {
        return Err(LegacyMigrationError::InvalidSource);
    }
    let mut known = serde_json::Map::new();
    for field in [
        "profiles",
        "activeProfileID",
        "defaultProfileID",
        "keyBindings",
        "outputBindings",
        "profileKeyBindings",
        "profileOutputBindings",
    ] {
        if let Some(value) = root.get(field) {
            known.insert(field.into(), value.clone());
        }
    }
    // Export envelopes may declare no default. Validate using active only as a
    // temporary owner reference, preserving the original null in staged bytes.
    if known.get("defaultProfileID").is_some_and(Value::is_null) {
        known.insert("defaultProfileID".into(), root["activeProfileID"].clone());
    }
    let validation: ConfigurationDocument = serde_json::from_value(Value::Object(known))
        .map_err(|_| LegacyMigrationError::InvalidSource)?;
    validation
        .validate()
        .map_err(|_| LegacyMigrationError::InvalidSource)?;
    let raw_root: BTreeMap<String, &RawValue> =
        serde_json::from_slice(source).map_err(|_| LegacyMigrationError::InvalidSource)?;
    let profiles: Vec<&RawValue> = serde_json::from_str(
        raw_root
            .get("profiles")
            .ok_or(LegacyMigrationError::InvalidSource)?
            .get(),
    )
    .map_err(|_| LegacyMigrationError::InvalidSource)?;
    let mut output = edit_json_values(
        source,
        &[JsonValueEdit {
            pointer: "/version",
            value: Some(b"5"),
        }],
        MAXIMUM,
    )
    .map_err(|_| LegacyMigrationError::TooLarge)?;
    let mut reviews = Vec::new();
    for (index, profile) in profiles.iter().enumerate() {
        let original = profile.get().as_bytes();
        let preparation = prepare(original)?;
        let migrated = compose_profile(original, &preparation)?;
        let path = format!("/profiles/{index}");
        output = edit_json_values(
            &output,
            &[JsonValueEdit {
                pointer: &path,
                value: Some(&migrated),
            }],
            MAXIMUM,
        )
        .map_err(|_| LegacyMigrationError::TooLarge)?;
        reviews.push(LegacyMigrationReview {
            profile_id: preparation.profile_id,
            source_sha256: preparation.source_sha256,
            package_sha256: preparation.package_sha256,
            source_layout: preparation.source_layout,
            source_orientation: preparation.source_orientation,
            reasons: preparation.reviews,
        });
    }
    let destination = OwnedConfigurationDocument::decode(&output)
        .map_err(|_| LegacyMigrationError::InvalidDestination)?;
    Ok(LegacyMigrationCandidate {
        original: Arc::from(source),
        destination,
        reviews,
    })
}

fn compose_profile(
    source: &[u8],
    preparation: &NativeLegacyPreparation,
) -> Result<Vec<u8>, LegacyMigrationError> {
    if preparation.source_sha256 != format!("{:x}", Sha256::digest(source)) {
        return Err(LegacyMigrationError::InvalidPreparation);
    }
    let original: BTreeMap<String, &RawValue> =
        serde_json::from_slice(source).map_err(|_| LegacyMigrationError::InvalidSource)?;
    let selected = original
        .get(preparation.source_layout.field())
        .ok_or(LegacyMigrationError::InvalidPreparation)?;
    // Repeat the no-control-loss guard at the receiving boundary; a native
    // implementation mistake cannot silently omit another executable variant.
    let profile: Value =
        serde_json::from_slice(source).map_err(|_| LegacyMigrationError::InvalidSource)?;
    let selected_ids = crate::configuration::declared_element_ids(
        &serde_json::json!({"customization": profile[preparation.source_layout.field()]}),
    );
    if !crate::configuration::declared_element_ids(&profile).is_subset(&selected_ids) {
        return Err(LegacyMigrationError::InvalidPreparation);
    }
    let staged: BTreeMap<String, &RawValue> = serde_json::from_slice(&preparation.after)
        .map_err(|_| LegacyMigrationError::InvalidPreparation)?;
    let canonical = staged
        .get("customization")
        .ok_or(LegacyMigrationError::InvalidPreparation)?;
    preserve_declarations(
        selected.get().as_bytes(),
        &preparation.before,
        canonical.get().as_bytes(),
    )?;
    let layout = merge_json_projection(
        selected.get().as_bytes(),
        &preparation.before,
        canonical.get().as_bytes(),
        MAXIMUM,
    )
    .map_err(|_| LegacyMigrationError::OpaqueConflict)?;
    let layout = canonicalize_canvas(layout, canonical)?;
    let mut edits = vec![
        JsonValueEdit {
            pointer: "/customization",
            value: Some(&layout),
        },
        JsonValueEdit {
            pointer: "/profileFormatVersion",
            value: Some(b"5"),
        },
    ];
    let mut paths = Vec::new();
    for key in RETIRED {
        if original.contains_key(*key) {
            paths.push(format!("/{key}"));
        }
    }
    for path in &paths {
        edits.push(JsonValueEdit {
            pointer: path,
            value: None,
        });
    }
    for (key, path) in [
        ("appearanceBank", "/appearanceBank"),
        ("appearanceResources", "/appearanceResources"),
        ("appearanceResourceVersion", "/appearanceResourceVersion"),
    ] {
        if let Some(value) = staged.get(key) {
            edits.push(JsonValueEdit {
                pointer: path,
                value: Some(value.get().as_bytes()),
            });
        }
    }
    edit_json_values(source, &edits, MAXIMUM).map_err(|_| LegacyMigrationError::TooLarge)
}
fn canonicalize_canvas(
    layout: Vec<u8>,
    staged: &RawValue,
) -> Result<Vec<u8>, LegacyMigrationError> {
    // The legacy decoder canonicalizes known frame aliases/defaults before
    // projection. Their typed before/after values can therefore be identical;
    // apply the required canvas leaf explicitly without erasing opaque children.
    let known: BTreeMap<String, &RawValue> =
        serde_json::from_str(staged.get()).map_err(|_| LegacyMigrationError::InvalidPreparation)?;
    let target = known
        .get("deviceCanvas")
        .ok_or(LegacyMigrationError::InvalidPreparation)?;
    let fields: BTreeMap<String, &RawValue> =
        serde_json::from_str(target.get()).map_err(|_| LegacyMigrationError::InvalidPreparation)?;
    let frame = fields
        .get("frameID")
        .ok_or(LegacyMigrationError::InvalidPreparation)?;
    let frame_id: String =
        serde_json::from_str(frame.get()).map_err(|_| LegacyMigrationError::InvalidPreparation)?;
    let raw: BTreeMap<String, &RawValue> =
        serde_json::from_slice(&layout).map_err(|_| LegacyMigrationError::InvalidPreparation)?;
    let edit = if let Some(canvas) = raw
        .get("deviceCanvas")
        .filter(|canvas| canvas.get() != "null")
    {
        let fields: BTreeMap<String, &RawValue> =
            serde_json::from_str(canvas.get()).map_err(|_| LegacyMigrationError::OpaqueConflict)?;
        if fields
            .get("frameID")
            .and_then(|value| serde_json::from_str::<String>(value.get()).ok())
            .as_deref()
            == Some(&frame_id)
        {
            return Ok(layout);
        }
        JsonValueEdit {
            pointer: "/deviceCanvas/frameID",
            value: Some(frame.get().as_bytes()),
        }
    } else {
        JsonValueEdit {
            pointer: "/deviceCanvas",
            value: Some(target.get().as_bytes()),
        }
    };
    edit_json_values(&layout, &[edit], MAXIMUM)
        .map_err(|_| LegacyMigrationError::InvalidDestination)
}
fn preserve_declarations(
    source: &[u8],
    before: &[u8],
    after: &[u8],
) -> Result<(), LegacyMigrationError> {
    let parse = |data| {
        thumble_protocol::decode_unique_json::<Value>(data)
            .map_err(|_| LegacyMigrationError::InvalidPreparation)
    };
    let source = parse(source)?;
    let before = parse(before)?;
    let after = parse(after)?;
    let controls =
        |layout: &Value| -> Result<(Vec<Uuid>, BTreeMap<Uuid, Value>), LegacyMigrationError> {
            let items = layout
                .get("elements")
                .and_then(Value::as_array)
                .ok_or(LegacyMigrationError::InvalidPreparation)?;
            let mut result = BTreeMap::new();
            let mut order = Vec::new();
            for item in items {
                let mut fields = item
                    .as_object()
                    .ok_or(LegacyMigrationError::InvalidPreparation)?
                    .clone();
                let id = fields
                    .get("id")
                    .and_then(Value::as_str)
                    .and_then(|id| Uuid::parse_str(id).ok())
                    .ok_or(LegacyMigrationError::InvalidPreparation)?;
                fields.remove("layout");
                order.push(id);
                if result.insert(id, Value::Object(fields)).is_some() {
                    return Err(LegacyMigrationError::InvalidPreparation);
                }
            }
            Ok((order, result))
        };
    let old = controls(&before)?;
    let new = controls(&after)?;
    if old != new || controls(&source)?.1.keys().ne(old.1.keys()) {
        return Err(LegacyMigrationError::InvalidPreparation);
    }
    Ok(())
}
