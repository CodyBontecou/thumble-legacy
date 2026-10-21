//! Versioned canonical phone layout intent. Transport/format activation is separate.
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;
pub const MAXIMUM_OWNED_PHONE_EDIT_BYTES: usize = 64 * 1024;
pub const MAXIMUM_OWNED_PHONE_CONTROL_EDITS: usize = 128;
fn checked_uuid<E: serde::de::Error>(value: String) -> Result<Uuid, E> {
    let id = Uuid::parse_str(&value).map_err(|_| E::custom("expected canonical UUID"))?;
    if !id.to_string().eq_ignore_ascii_case(&value) {
        return Err(E::custom("expected canonical UUID"));
    }
    Ok(id)
}
fn canonical_uuid<'de, D: Deserializer<'de>>(decoder: D) -> Result<Uuid, D::Error> {
    checked_uuid(String::deserialize(decoder)?)
}
fn canonical_uuids<'de, D: Deserializer<'de>>(decoder: D) -> Result<Vec<Uuid>, D::Error> {
    Vec::<String>::deserialize(decoder)?
        .into_iter()
        .map(checked_uuid)
        .collect()
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnedPhoneEdit {
    pub schema_version: u32,
    #[serde(rename = "editID", deserialize_with = "canonical_uuid")]
    pub edit_id: Uuid,
    #[serde(rename = "profileID", deserialize_with = "canonical_uuid")]
    pub profile_id: Uuid,
    pub base_configuration_revision: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub control_edits: Vec<OwnedPhoneControlEdit>,
    #[serde(
        rename = "removedControlIDs",
        default,
        skip_serializing_if = "Vec::is_empty",
        deserialize_with = "canonical_uuids"
    )]
    pub removed_control_ids: Vec<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_scheme: Option<OwnedPhoneColorScheme>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnedPhoneControlEdit {
    #[serde(rename = "controlID", deserialize_with = "canonical_uuid")]
    pub control_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center_x: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center_y: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width_scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height_scale: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rotation_degrees: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_hidden: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_location_locked: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub z_index: Option<i32>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OwnedPhoneColorScheme {
    System,
    Light,
    Dark,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnedPhoneEditError {
    TooLarge,
    InvalidJson,
    InvalidIntent,
}
impl OwnedPhoneEdit {
    pub fn decode_json(bytes: &[u8]) -> Result<Self, OwnedPhoneEditError> {
        if bytes.len() > MAXIMUM_OWNED_PHONE_EDIT_BYTES {
            return Err(OwnedPhoneEditError::TooLarge);
        }
        let intent: Self =
            crate::decode_unique_json(bytes).map_err(|_| OwnedPhoneEditError::InvalidJson)?;
        intent.validate()?;
        Ok(intent)
    }
    pub fn validate(&self) -> Result<(), OwnedPhoneEditError> {
        let invalid = OwnedPhoneEditError::InvalidIntent;
        let count = self.control_edits.len() + self.removed_control_ids.len();
        if self.schema_version != 1
            || self.base_configuration_revision == 0
            || self.edit_id.is_nil()
            || self.profile_id.is_nil()
            || count > MAXIMUM_OWNED_PHONE_CONTROL_EDITS
            || (count == 0 && self.color_scheme.is_none())
        {
            return Err(invalid);
        }
        let mut ids = BTreeSet::new();
        for edit in &self.control_edits {
            if edit.control_id.is_nil() || !ids.insert(edit.control_id) {
                return Err(invalid);
            }
            if edit.center_x.is_none()
                && edit.center_y.is_none()
                && edit.width_scale.is_none()
                && edit.height_scale.is_none()
                && edit.rotation_degrees.is_none()
                && edit.is_hidden.is_none()
                && edit.is_location_locked.is_none()
                && edit.z_index.is_none()
            {
                return Err(invalid);
            }
            for (value, min, max) in [
                (edit.center_x, 0.0, 1.0),
                (edit.center_y, 0.0, 1.0),
                (edit.width_scale, 0.001, 12.0),
                (edit.height_scale, 0.001, 12.0),
                (edit.rotation_degrees, -36000.0, 36000.0),
            ] {
                if value.is_some_and(|v| !v.is_finite() || v < min || v > max) {
                    return Err(invalid);
                }
            }
            if edit.z_index.is_some_and(|v| !(-100..=100).contains(&v)) {
                return Err(invalid);
            }
        }
        for id in &self.removed_control_ids {
            if id.is_nil() || !ids.insert(*id) {
                return Err(invalid);
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OwnedPhoneEditStatus {
    Applied,
    Replayed,
    Conflict,
    Rejected,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnedPhoneEditResult {
    pub schema_version: u32,
    #[serde(rename = "editID", deserialize_with = "canonical_uuid")]
    pub edit_id: Uuid,
    #[serde(rename = "profileID", deserialize_with = "canonical_uuid")]
    pub profile_id: Uuid,
    pub base_configuration_revision: u64,
    pub configuration_revision: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub committed_revision: Option<u64>,
    pub changed: bool,
    pub status: OwnedPhoneEditStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn valid() -> Vec<u8> {
        br#"{"schemaVersion":1,"editID":"aa000000-0000-0000-0000-000000000001","profileID":"ea000000-0000-0000-0000-000000000001","baseConfigurationRevision":41,"controlEdits":[{"controlID":"00000000-0000-0000-0000-000000000109","centerX":0.6,"rotationDegrees":12,"isHidden":false}],"colorScheme":"dark"}"#.to_vec()
    }
    #[test]
    fn roundtrip_retains_scalar_intent_without_full_layout_or_appearance() {
        let intent = OwnedPhoneEdit::decode_json(&valid()).unwrap();
        assert_eq!(
            OwnedPhoneEdit::decode_json(&serde_json::to_vec(&intent).unwrap()).unwrap(),
            intent
        );
        assert_eq!(intent.control_edits[0].is_hidden, Some(false));
        assert_eq!(intent.color_scheme, Some(OwnedPhoneColorScheme::Dark));
    }
    #[test]
    fn unsafe_fields_duplicates_alias_ids_and_unbounded_numbers_are_rejected() {
        let original: serde_json::Value = serde_json::from_slice(&valid()).unwrap();
        for (pointer, value) in [
            ("/skinReference", serde_json::json!({})),
            ("/gamepadCustomization", serde_json::json!({})),
            ("/controlEdits/0/output", serde_json::json!({})),
            ("/controlEdits/0/centerX", serde_json::json!(2)),
            ("/controlEdits/0/widthScale", serde_json::json!(0)),
            ("/baseConfigurationRevision", serde_json::json!(0)),
            ("/schemaVersion", serde_json::json!(2)),
        ] {
            let mut value_json = original.clone();
            let (parent, key) = pointer.rsplit_once('/').unwrap();
            if parent.is_empty() {
                value_json[key] = value;
            } else {
                value_json.pointer_mut(parent).unwrap()[key] = value;
            }
            assert!(
                OwnedPhoneEdit::decode_json(&serde_json::to_vec(&value_json).unwrap()).is_err(),
                "{pointer}"
            );
        }
        let text = String::from_utf8(valid()).unwrap();
        assert!(OwnedPhoneEdit::decode_json(
            text.replacen(
                "\"schemaVersion\":1",
                "\"schemaVersion\":1,\"schemaVersion\":1",
                1
            )
            .as_bytes()
        )
        .is_err());
        let mut intent = OwnedPhoneEdit::decode_json(&valid()).unwrap();
        intent
            .removed_control_ids
            .push(intent.control_edits[0].control_id);
        assert!(intent.validate().is_err());
        intent.removed_control_ids.clear();
        intent.control_edits[0].center_x = Some(f64::NAN);
        assert!(intent.validate().is_err());
        assert_eq!(
            OwnedPhoneEdit::decode_json(&vec![b' '; MAXIMUM_OWNED_PHONE_EDIT_BYTES + 1]),
            Err(OwnedPhoneEditError::TooLarge)
        );
    }
    #[test]
    fn shared_intent_matrix_matches_strict_codec_and_keeps_activation_gated() {
        let cases: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../fixtures/owned-phone-edits/v1/cases.json"
        ))
        .unwrap();
        for row in cases["cases"].as_array().unwrap() {
            let bytes = if let Some(raw) = row["raw"].as_str() {
                raw.as_bytes().to_vec()
            } else {
                serde_json::to_vec(&row["request"]).unwrap()
            };
            assert_eq!(
                OwnedPhoneEdit::decode_json(&bytes).is_ok(),
                row["allowed"].as_bool().unwrap(),
                "{}",
                row["name"]
            );
        }
        assert_eq!(crate::SUPPORTED_PROFILE_FORMATS, &[4]);
    }
    #[test]
    fn edit_and_receipt_wire_roundtrip_and_never_hide_in_compact_input() {
        use crate::{
            ButtonPressState, ControllerMessage, ControllerMessageType, ControllerWireCodec,
            KeypadElementID,
        };
        let intent = OwnedPhoneEdit::decode_json(&valid()).unwrap();
        let mut message = ControllerMessage::new(ControllerMessageType::OwnedPhoneEdit, 0);
        message.profile_format_version = Some(5);
        message.owned_phone_edit = Some(Box::new(intent.clone()));
        assert_eq!(
            ControllerWireCodec::decode(&ControllerWireCodec::encode(&message).unwrap()).unwrap(),
            message
        );
        let result = OwnedPhoneEditResult {
            schema_version: 1,
            edit_id: intent.edit_id,
            profile_id: intent.profile_id,
            base_configuration_revision: 41,
            configuration_revision: 43,
            committed_revision: Some(42),
            changed: true,
            status: OwnedPhoneEditStatus::Replayed,
            error_code: None,
        };
        let mut receipt = ControllerMessage::new(ControllerMessageType::OwnedPhoneEditResult, 0);
        receipt.configuration_revision = Some(43);
        receipt.owned_phone_edit_result = Some(Box::new(result));
        assert_eq!(
            ControllerWireCodec::decode(&ControllerWireCodec::encode(&receipt).unwrap()).unwrap(),
            receipt
        );
        let mut input = ControllerMessage::new(ControllerMessageType::Button, 0);
        input.button = Some(KeypadElementID::preset(1));
        input.state = Some(ButtonPressState::Down);
        assert!(ControllerWireCodec::encode(&input)
            .unwrap()
            .starts_with(b"PP"));
        input.owned_phone_edit = Some(Box::new(intent));
        assert!(!ControllerWireCodec::encode(&input)
            .unwrap()
            .starts_with(b"PP"));
        input.owned_phone_edit = None;
        input.configuration_revision = Some(41);
        assert!(!ControllerWireCodec::encode(&input)
            .unwrap()
            .starts_with(b"PP"));
    }
    #[test]
    fn frozen_wire_frames_keep_current_revision_separate_from_commit_receipts() {
        use crate::{ControllerMessage, ControllerMessageType, ControllerWireCodec};
        let frames: Vec<ControllerMessage> = serde_json::from_slice(include_bytes!(
            "../../../fixtures/owned-phone-wire/v1/frames.json"
        ))
        .unwrap();
        assert_eq!(frames.len(), 5);
        let intent = frames[0].owned_phone_edit.as_ref().unwrap();
        intent.validate().unwrap();
        for frame in &frames {
            assert_eq!(frame.profile_format_version, Some(5));
            let wire = ControllerWireCodec::encode(frame).unwrap();
            assert!(!wire.starts_with(b"PP"));
            assert_eq!(ControllerWireCodec::decode(&wire).unwrap(), *frame);
            if frame.message_type == ControllerMessageType::OwnedPhoneEditResult {
                let result = frame.owned_phone_edit_result.as_ref().unwrap();
                assert_eq!(result.edit_id, intent.edit_id);
                assert_eq!(result.profile_id, intent.profile_id);
                assert_eq!(
                    result.base_configuration_revision,
                    intent.base_configuration_revision
                );
                assert_eq!(
                    frame.configuration_revision,
                    Some(result.configuration_revision)
                );
                if result.status == OwnedPhoneEditStatus::Replayed {
                    assert_eq!(result.committed_revision, Some(42));
                    assert_eq!(result.configuration_revision, 43);
                }
                assert_eq!(
                    result.committed_revision.is_some(),
                    result.error_code.is_none()
                );
            }
        }
        assert_eq!(crate::SUPPORTED_PROFILE_FORMATS, &[4]);
    }
}
