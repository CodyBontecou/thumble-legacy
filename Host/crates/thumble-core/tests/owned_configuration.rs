use serde_json::Value;
use thumble_core::{
    ConfigurationDocument, OwnedConfigurationDocument, PersistentState, ProfileAppearanceBank,
};

const DOCUMENT: &[u8] = include_bytes!("../../../fixtures/owned-configuration/v1/document.json");
#[test]
fn shared_version_five_contract_and_rejected_mutations() {
    let original: Value = serde_json::from_slice(DOCUMENT).unwrap();
    let matrix: Value = serde_json::from_str(include_str!(
        "../../../fixtures/owned-configuration/v1/cases.json"
    ))
    .unwrap();
    for row in matrix["cases"].as_array().unwrap() {
        let mut candidate = original.clone();
        let path = row["path"].as_array().unwrap();
        if let Some((last, parents)) = path.split_last() {
            let mut target = &mut candidate;
            for key in parents {
                target = if let Some(index) = key.as_u64() {
                    &mut target[index as usize]
                } else {
                    &mut target[key.as_str().unwrap()]
                };
            }
            let key = last.as_str().unwrap();
            if row["action"] == "remove" {
                target.as_object_mut().unwrap().remove(key);
            } else {
                target[key] = row["value"].clone();
            }
        }
        let bytes = serde_json::to_vec(&candidate).unwrap();
        assert_eq!(
            OwnedConfigurationDocument::decode(&bytes).is_ok(),
            row["allowed"].as_bool().unwrap(),
            "{}",
            row["name"]
        );
    }
}
#[test]
fn exact_source_metadata_resources_and_all_states_survive_without_packages() {
    let document = OwnedConfigurationDocument::decode(DOCUMENT).unwrap();
    assert_eq!(document.original_bytes(), DOCUMENT);
    assert_eq!(document.profiles().len(), 2);
    assert_eq!(
        document.active_profile_id(),
        document.profiles()[0]["id"].as_str().unwrap()
    );
    assert_eq!(
        document.default_profile_id(),
        document.profiles()[1]["id"].as_str()
    );
    let raw = &document.profiles()[0];
    assert_eq!(
        raw["futureProfileMetadata"]["opaque"][2].as_u64(),
        Some(u64::MAX)
    );
    let bank = ProfileAppearanceBank::from_profile(raw).unwrap().unwrap();
    for scheme in ["light", "dark"] {
        for action in ["base", "left-trigger", "right-trigger", "both-triggers"] {
            let layout = bank
                .resolve_layout(&raw["customization"], scheme, action)
                .unwrap();
            for (actual, canonical) in layout["elements"]
                .as_array()
                .unwrap()
                .iter()
                .zip(raw["customization"]["elements"].as_array().unwrap())
            {
                for field in ["id", "kind", "output", "defaultOutput", "partOutputs"] {
                    assert_eq!(actual[field], canonical[field]);
                }
                for field in [
                    "centerX",
                    "centerY",
                    "widthScale",
                    "heightScale",
                    "rotationDegrees",
                    "isHidden",
                ] {
                    assert_eq!(actual["layout"][field], canonical["layout"][field]);
                }
            }
            assert_eq!(
                layout["elements"][0]["layout"]["icon"]["value"],
                format!("{scheme}/{action}")
            );
            assert_eq!(layout["deviceCanvas"], raw["customization"]["deviceCanvas"]);
        }
    }
    // The codec is explicit. Reading a destination cannot authorize an install
    // through the still-v4 persistent-state boundary or change the live format offer.
    let mut state = PersistentState::minimal("owned-codec-test").unwrap();
    state.profiles = document.profiles().to_vec();
    state.active_profile_id = document.active_profile_id().into();
    state.default_profile_id = document.default_profile_id().unwrap().into();
    assert!(ConfigurationDocument::from_state(&state).is_err());
    assert_eq!(thumble_protocol::SUPPORTED_PROFILE_FORMATS, &[4]);
}
#[test]
fn duplicate_keys_invalid_utf8_and_complete_document_overflow_are_rejected() {
    let source = std::str::from_utf8(DOCUMENT).unwrap();
    let duplicate = source.replacen("\"version\": 5", "\"version\": 5, \"version\": 4", 1);
    assert!(OwnedConfigurationDocument::decode(duplicate.as_bytes()).is_err());
    assert!(OwnedConfigurationDocument::decode(&[0xff]).is_err());
    let mut maximum = DOCUMENT.to_vec();
    maximum.resize(thumble_core::MAXIMUM_CONFIGURATION_DOCUMENT_BYTES, b' ');
    assert!(OwnedConfigurationDocument::decode(&maximum).is_ok());
    maximum.push(b' ');
    assert!(OwnedConfigurationDocument::decode(&maximum).is_err());
}

#[test]
fn lossless_edits_revalidate_complete_destinations_and_keep_original_on_failure() {
    use thumble_protocol::JsonValueEdit;
    let source = std::str::from_utf8(DOCUMENT)
        .unwrap()
        .replace("18446744073709551615", "18446744073709551616");
    let original = OwnedConfigurationDocument::decode(source.as_bytes()).unwrap();
    let updated = original
        .with_edits(&[
            JsonValueEdit {
                pointer: "/profiles/0/name",
                value: Some(br#""Edited locally""#),
            },
            JsonValueEdit {
                pointer: "/profiles/0/customization/futureDesignSetting",
                value: Some(br#"{"enabled":true}"#),
            },
        ])
        .unwrap();
    assert_eq!(original.original_bytes(), source.as_bytes());
    let expected = source.replacen(
        "\"name\": \"Resource fixture\"",
        "\"name\": \"Edited locally\"",
        1,
    );
    // The added setting changes only its insertion site. Removing it must leave
    // the exact original body except for the deliberate name replacement.
    let reverted_setting = updated
        .with_edits(&[JsonValueEdit {
            pointer: "/profiles/0/customization/futureDesignSetting",
            value: None,
        }])
        .unwrap();
    assert_eq!(reverted_setting.original_bytes(), expected.as_bytes());
    assert!(updated
        .with_edits(&[JsonValueEdit {
            pointer: "/profiles/0/profileFormatVersion",
            value: Some(b"4")
        }])
        .is_err());
    assert!(updated
        .with_edits(&[JsonValueEdit {
            pointer: "/profiles/0/portraitCustomization",
            value: Some(b"null")
        }])
        .is_err());
    assert_eq!(original.original_bytes(), source.as_bytes());
    assert_eq!(thumble_protocol::SUPPORTED_PROFILE_FORMATS, &[4]);
}

#[test]
fn lossless_geometry_icon_and_haptic_edits_match_shared_destination() {
    use thumble_protocol::JsonValueEdit;
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/owned-configuration-edits/v1");
    let input = std::fs::read(root.join("input.json")).unwrap();
    let request: Value =
        serde_json::from_slice(&std::fs::read(root.join("edits.json")).unwrap()).unwrap();
    let edits: Vec<_> = request["edits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| JsonValueEdit {
            pointer: row["pointer"].as_str().unwrap(),
            value: row["json"].as_str().map(str::as_bytes),
        })
        .collect();
    let original = OwnedConfigurationDocument::decode(&input).unwrap();
    let edited = original.with_edits(&edits).unwrap();
    let profile = &edited.profiles()[0];
    assert_eq!(profile["name"], "Edited keypad");
    let bank = ProfileAppearanceBank::from_profile(profile)
        .unwrap()
        .unwrap();
    for scheme in ["light", "dark"] {
        for action in ["base", "left-trigger", "right-trigger", "both-triggers"] {
            let rendered = bank
                .resolve_layout(&profile["customization"], scheme, action)
                .unwrap();
            let style = &rendered["elements"][0]["layout"];
            assert_eq!(style["centerX"], 0.78);
            assert_eq!(style["icon"]["value"], "Mine");
            assert_eq!(style["hapticFeedback"]["pattern"], "double");
            assert_eq!(style["hapticFeedback"]["intensity"], 0.25);
            assert_eq!(style["hapticFeedback"]["sharpness"], 0.75);
            assert_eq!(style["hapticFeedback"]["duration"], 0.12);
            assert_eq!(
                rendered["elements"][0]["output"],
                original.profiles()[0]["customization"]["elements"][0]["output"]
            );
        }
    }
    let output = std::str::from_utf8(edited.original_bytes()).unwrap();
    assert!(output.contains("18446744073709551617"));
    assert!(output.contains("0.123456789012345678901234567890"));
    assert_eq!(original.original_bytes(), input);
    if std::env::var_os("THUMBLE_REGENERATE_OWNED_EDIT_FIXTURE").is_some() {
        std::fs::write(root.join("destination.json"), edited.original_bytes()).unwrap();
    }
    assert_eq!(
        edited.original_bytes(),
        std::fs::read(root.join("destination.json")).unwrap()
    );
}

#[test]
fn resource_edits_require_matching_bytes_and_retain_original_resources() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use sha2::{Digest, Sha256};
    use thumble_core::ProfileResourceTable;
    use thumble_protocol::JsonValueEdit;
    let original = OwnedConfigurationDocument::decode(DOCUMENT).unwrap();
    let bytes = b"replacement-image";
    let digest = format!("{:x}", Sha256::digest(bytes));
    let pointer = format!("/profiles/0/appearanceResources/{digest}");
    let encoded = serde_json::to_vec(&STANDARD.encode(bytes)).unwrap();
    let reference = serde_json::to_vec(&serde_json::json!({"resourceSHA256":digest})).unwrap();
    let updated = original
        .with_edits(&[
            JsonValueEdit {
                pointer: &pointer,
                value: Some(&encoded),
            },
            JsonValueEdit {
                pointer: "/profiles/0/customization/assetLibrary/assets/0/data",
                value: Some(&reference),
            },
        ])
        .unwrap();
    let table = ProfileResourceTable::from_profile(&updated.profiles()[0])
        .unwrap()
        .unwrap();
    assert_eq!(
        table
            .data(&serde_json::from_slice(&reference).unwrap())
            .unwrap()
            .as_ref(),
        bytes
    );
    assert_eq!(original.original_bytes(), DOCUMENT);
    assert_eq!(
        updated.profiles()[0]["appearanceResources"]
            .as_object()
            .unwrap()
            .len(),
        original.profiles()[0]["appearanceResources"]
            .as_object()
            .unwrap()
            .len()
            + 1
    );
    assert!(original
        .with_edits(&[JsonValueEdit {
            pointer: &pointer,
            value: Some(br#""incorrect-hash""#)
        }])
        .is_err());
}
