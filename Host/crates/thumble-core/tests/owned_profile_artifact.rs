use serde_json::{value::RawValue, Value};
use thumble_core::{
    OwnedConfigurationDocument, OwnedProfileArtifact, OwnedProfileArtifactError,
    ProfileArtifactSelection,
};
use thumble_protocol::JsonValueEdit;

const FIXTURE: &[u8] = include_bytes!("../../../fixtures/owned-configuration/v1/document.json");
const PRECISE: &[u8] =
    br#"{"integer":9007199254740993,"fraction":0.12345678901234567890123456789}"#;
fn source() -> OwnedConfigurationDocument {
    OwnedConfigurationDocument::decode(FIXTURE)
        .unwrap()
        .with_edits(&[
            JsonValueEdit {
                pointer: "/futureRoot",
                value: Some(PRECISE),
            },
            JsonValueEdit {
                pointer: "/profiles/0/futureProfile",
                value: Some(PRECISE),
            },
            JsonValueEdit {
                pointer: "/profiles/0/launchTarget",
                value: Some(br#"{"path":"/Applications/Private.app","bookmarkData":"secret"}"#),
            },
        ])
        .unwrap()
}
fn all(
    document: &OwnedConfigurationDocument,
) -> Result<OwnedProfileArtifact, OwnedProfileArtifactError> {
    OwnedProfileArtifact::from_configuration(document, ProfileArtifactSelection::All, 123)
}
fn edit(
    document: &OwnedConfigurationDocument,
    pointer: &str,
    data: &[u8],
) -> OwnedConfigurationDocument {
    document
        .with_edits(&[JsonValueEdit {
            pointer,
            value: Some(data),
        }])
        .unwrap()
}
#[test]
fn lossless_all_profile_roundtrip_preserves_eight_states_resources_and_exact_numbers() {
    let source = source();
    let before = source.original_bytes().to_vec();
    let artifact = all(&source).unwrap();
    let bytes = artifact.encode_json().unwrap();
    let decoded = OwnedProfileArtifact::decode_json(&bytes).unwrap();
    assert_eq!(decoded.encode_json().unwrap(), bytes);
    assert_eq!(decoded.content_hash(), artifact.content_hash());
    assert_eq!(source.original_bytes(), before);
    assert_eq!(
        decoded.document().active_profile_id(),
        source.active_profile_id()
    );
    assert_eq!(
        decoded.document().default_profile_id(),
        source.default_profile_id()
    );
    let raw: std::collections::BTreeMap<String, &RawValue> =
        serde_json::from_slice(decoded.document().original_bytes()).unwrap();
    assert_eq!(raw["futureRoot"].get().as_bytes(), PRECISE);
    let text = std::str::from_utf8(decoded.document().original_bytes()).unwrap();
    assert!(text.contains(std::str::from_utf8(PRECISE).unwrap()));
    assert!(!text.contains("Private.app"));
    assert!(!text.contains("bookmarkData"));
    for (a, b) in decoded.document().profiles().iter().zip(source.profiles()) {
        for key in [
            "customization",
            "appearanceBank",
            "appearanceResources",
            "appearanceResourceVersion",
        ] {
            assert_eq!(a[key], b[key]);
        }
        assert!(a.get("launchTarget").is_none());
    }
}
#[test]
fn single_selection_filters_maps_and_preserves_selected_default_status() {
    let source = source();
    for profile in source.profiles() {
        let id = profile["id"].as_str().unwrap();
        let artifact = OwnedProfileArtifact::from_configuration(
            &source,
            ProfileArtifactSelection::ProfileId(id.to_uppercase()),
            123,
        )
        .unwrap();
        assert_eq!(artifact.document().profiles().len(), 1);
        assert_eq!(artifact.document().active_profile_id(), id);
        assert_eq!(
            artifact.document().default_profile_id().is_some(),
            source
                .default_profile_id()
                .is_some_and(|s| s.eq_ignore_ascii_case(id))
        );
        let value: Value = serde_json::from_slice(artifact.document().original_bytes()).unwrap();
        for key in ["profileKeyBindings", "profileOutputBindings"] {
            assert!(value[key]
                .as_object()
                .unwrap()
                .keys()
                .all(|s| s.eq_ignore_ascii_case(id)));
        }
        OwnedProfileArtifact::decode_json(&artifact.encode_json().unwrap()).unwrap();
    }
    assert!(matches!(
        OwnedProfileArtifact::from_configuration(
            &source,
            ProfileArtifactSelection::ProfileId("missing".into()),
            123
        ),
        Err(OwnedProfileArtifactError::InvalidSelection)
    ));
}
#[test]
fn recursive_credentials_paths_aliases_and_binary_payloads_are_rejected_without_selection_bypass() {
    let source = source();
    for (pointer, payload) in [
        ("/futureRoot/auth_token", br#""secret""#.as_slice()),
        (
            "/profiles/0/futureProfile/privateKey",
            br#""secret""#.as_slice(),
        ),
        (
            "/profiles/0/futureProfile/imageData",
            br#""secret""#.as_slice(),
        ),
        (
            "/profiles/0/futureProfile",
            br#"{"appearanceResources":{"password":"secret"}}"#.as_slice(),
        ),
        (
            "/futureRoot",
            br#"{"appearanceResources":{"authToken":"secret"}}"#.as_slice(),
        ),
        (
            "/profileKeyBindings/00000000-0000-0000-0000-000000000001",
            br#"{}"#.as_slice(),
        ),
    ] {
        let altered = source.with_edits(&[JsonValueEdit {
            pointer,
            value: Some(payload),
        }]);
        if pointer.starts_with("/profileKeyBindings/") {
            assert!(altered.is_err());
            continue;
        }
        let altered = altered.unwrap();
        assert!(all(&altered).is_err(), "{pointer}");
        let second = source.profiles()[1]["id"].as_str().unwrap();
        assert!(
            OwnedProfileArtifact::from_configuration(
                &altered,
                ProfileArtifactSelection::ProfileId(second.into()),
                123
            )
            .is_err(),
            "{pointer}"
        );
    }
    assert!(source
        .with_edits(&[JsonValueEdit {
            pointer: "/profiles/0/launch_target",
            value: Some(br#""secret""#)
        }])
        .is_err());
    let disguised = edit(
        &source,
        "/profiles~10",
        br#"{"appearanceResources":{"password":"secret"}}"#,
    );
    assert!(all(&disguised).is_err());
    for pointer in ["/futureRoot", "/profiles~10", "/profiles/0/futureProfile"] {
        let disguised = edit(
            &source,
            pointer,
            br#"{"appearanceResources":{"fake":"YmFk"}}"#,
        );
        assert!(all(&disguised).is_err());
    }
    let missing = edit(
        &source,
        "/profiles/1/futureProfile",
        br#"{"data":{"resourceSHA256":"missing"}}"#,
    );
    assert!(all(&missing).is_err());
    let altered = edit(&source, "/futureRoot/data", br#"{"resourceSHA256":"fake"}"#);
    assert!(all(&altered).is_err());
}
#[test]
fn hash_version_catalog_duplicates_and_tampering_fail_closed() {
    let bytes = all(&source()).unwrap().encode_json().unwrap();
    OwnedProfileArtifact::decode_json(&bytes).unwrap();
    for (pointer, payload, expected) in [
        (
            "/document/profiles/0/name",
            br#""tampered""#.as_slice(),
            OwnedProfileArtifactError::InvalidHash,
        ),
        (
            "/contentHash/value",
            br#""tampered""#.as_slice(),
            OwnedProfileArtifactError::InvalidHash,
        ),
        (
            "/contentHash/canonicalization",
            br#""rfc8785""#.as_slice(),
            OwnedProfileArtifactError::InvalidHash,
        ),
        (
            "/schema",
            br#""tampered""#.as_slice(),
            OwnedProfileArtifactError::InvalidEnvelope,
        ),
        (
            "/artifactVersion",
            b"99".as_slice(),
            OwnedProfileArtifactError::InvalidEnvelope,
        ),
        (
            "/catalogRevision/controllerTemplates",
            b"99".as_slice(),
            OwnedProfileArtifactError::InvalidEnvelope,
        ),
    ] {
        let changed = thumble_protocol::edit_json_values(
            &bytes,
            &[JsonValueEdit {
                pointer,
                value: Some(payload),
            }],
            8 * 1024 * 1024,
        )
        .unwrap();
        assert_eq!(
            OwnedProfileArtifact::decode_json(&changed).unwrap_err(),
            expected,
            "{pointer}"
        );
    }
    let text = String::from_utf8(bytes).unwrap();
    let duplicate = text.replacen(
        "\"artifactVersion\":1",
        "\"artifactVersion\":1,\"artifactVersion\":1",
        1,
    );
    assert!(OwnedProfileArtifact::decode_json(duplicate.as_bytes()).is_err());
    let duplicate_inner =
        text.replacen("\"futureRoot\":", "\"futureRoot\":null,\"futureRoot\":", 1);
    assert!(OwnedProfileArtifact::decode_json(duplicate_inner.as_bytes()).is_err());
    let mut unknown: Value = serde_json::from_str(&text).unwrap();
    unknown["authToken"] = serde_json::json!("secret");
    assert!(OwnedProfileArtifact::decode_json(&serde_json::to_vec(&unknown).unwrap()).is_err());
    assert!(OwnedProfileArtifact::decode_json(FIXTURE).is_err());
}

#[test]
fn valid_large_resource_tables_are_portable_but_arbitrary_large_strings_are_not() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use sha2::{Digest, Sha256};
    let source = source();
    let value = &source.profiles()[0]["appearanceResources"];
    let old = value.as_object().unwrap().keys().next().unwrap();
    let data = vec![42_u8; 300 * 1024];
    let new = format!("{:x}", Sha256::digest(&data));
    let old_text = std::str::from_utf8(source.original_bytes()).unwrap();
    let replaced = old_text
        .replace(old, &new)
        .replace(value[old].as_str().unwrap(), &STANDARD.encode(data));
    let document = OwnedConfigurationDocument::decode(replaced.as_bytes()).unwrap();
    let artifact = all(&document).unwrap();
    let decoded = OwnedProfileArtifact::decode_json(&artifact.encode_json().unwrap()).unwrap();
    assert_eq!(
        decoded.document().profiles()[0]["appearanceResources"],
        document.profiles()[0]["appearanceResources"]
    );
    let text = serde_json::to_vec(&"x".repeat(256 * 1024 + 1)).unwrap();
    assert!(matches!(
        all(&edit(&source, "/futureRoot", &text)),
        Err(OwnedProfileArtifactError::NonportableContent)
    ));
}
#[test]
fn valid_digest_does_not_bypass_portability_or_resource_validation() {
    use sha2::{Digest, Sha256};
    let artifact = all(&source()).unwrap();
    let mut wire: std::collections::BTreeMap<String, Box<RawValue>> =
        serde_json::from_slice(&artifact.encode_json().unwrap()).unwrap();
    for (pointer, payload) in [
        ("/futureRoot/password", br#""private""#.as_slice()),
        (
            "/profiles/0/appearanceResources",
            br#"{"fake":"YmFk"}"#.as_slice(),
        ),
    ] {
        let document = thumble_protocol::edit_json_values(
            artifact.document().original_bytes(),
            &[JsonValueEdit {
                pointer,
                value: Some(payload),
            }],
            8 * 1024 * 1024,
        )
        .unwrap();
        wire.insert(
            "document".into(),
            RawValue::from_string(String::from_utf8(document.clone()).unwrap()).unwrap(),
        );
        wire.insert("contentHash".into(),serde_json::value::to_raw_value(&serde_json::json!({"algorithm":"sha256","canonicalization":"exact-json-utf8","value":format!("{:x}",Sha256::digest(&document))})).unwrap());
        assert!(
            OwnedProfileArtifact::decode_json(&serde_json::to_vec(&wire).unwrap()).is_err(),
            "{pointer}"
        );
    }
}
#[test]
fn active_binding_aliases_survive_single_selection_without_an_explicit_map() {
    let document = OwnedConfigurationDocument::decode(FIXTURE).unwrap();
    let document = document
        .with_edits(&[
            JsonValueEdit {
                pointer: "/profileKeyBindings",
                value: Some(b"{}"),
            },
            JsonValueEdit {
                pointer: "/profileOutputBindings",
                value: Some(b"{}"),
            },
        ])
        .unwrap();
    let active = document.active_profile_id();
    let before: Value = serde_json::from_slice(document.original_bytes()).unwrap();
    // The fixture supplies active aliases independently of profile maps. They
    // must not disappear when exporting only the currently active profile.
    let artifact = OwnedProfileArtifact::from_configuration(
        &document,
        ProfileArtifactSelection::ProfileId(active.into()),
        123,
    )
    .unwrap();
    let after: Value = serde_json::from_slice(artifact.document().original_bytes()).unwrap();
    for (map, key) in [
        ("profileKeyBindings", "keyBindings"),
        ("profileOutputBindings", "outputBindings"),
    ] {
        let expected = before[map]
            .as_object()
            .and_then(|m| {
                m.iter()
                    .find(|(id, _)| id.eq_ignore_ascii_case(active))
                    .map(|(_, v)| v)
            })
            .unwrap_or(&before[key]);
        assert_eq!(&after[key], expected);
    }
}
#[test]
fn container_size_includes_metadata_and_refuses_unbounded_input() {
    use thumble_core::MAXIMUM_OWNED_PROFILE_ARTIFACT_BYTES as MAX;
    assert!(matches!(
        OwnedProfileArtifact::decode_json(&vec![b' '; MAX + 1]),
        Err(OwnedProfileArtifactError::TooLarge)
    ));
    let source = all(&source()).unwrap().document().clone();
    let base = edit(&source, "/padding", b"[]");
    let target = MAX - base.original_bytes().len() - 10;
    let mut chunks = Vec::new();
    let mut remaining = target;
    while remaining > 2 {
        let len = (256 * 1024).min(remaining.saturating_sub(3));
        if len == 0 {
            break;
        }
        chunks.push("x".repeat(len));
        remaining = remaining.saturating_sub(len + 3);
    }
    let value = serde_json::to_vec(&chunks).unwrap();
    let large = edit(&source, "/padding", &value);
    assert!(large.original_bytes().len() < MAX);
    assert!(matches!(
        all(&large),
        Err(OwnedProfileArtifactError::TooLarge)
    ));
}

#[test]
fn frozen_fixture_matches_export_and_reencodes_exactly() {
    const ARTIFACT: &[u8] =
        include_bytes!("../../../fixtures/owned-profile-artifact/v1/artifact.json");
    let source = OwnedConfigurationDocument::decode(FIXTURE).unwrap();
    let artifact =
        OwnedProfileArtifact::from_configuration(&source, ProfileArtifactSelection::All, 123)
            .unwrap();
    assert_eq!(artifact.encode_json().unwrap(), ARTIFACT);
    assert_eq!(
        OwnedProfileArtifact::decode_json(ARTIFACT)
            .unwrap()
            .encode_json()
            .unwrap(),
        ARTIFACT
    );
    assert!(matches!(
        OwnedProfileArtifact::from_configuration(&source, ProfileArtifactSelection::All, -1),
        Err(OwnedProfileArtifactError::InvalidEnvelope)
    ));
}

#[test]
fn digest_covers_inner_whitespace_but_not_outer_whitespace() {
    let artifact = all(&source()).unwrap();
    let bytes = artifact.encode_json().unwrap();
    let outside = [b" \n".as_slice(), bytes.as_slice(), b" \n".as_slice()].concat();
    assert_eq!(
        OwnedProfileArtifact::decode_json(&outside)
            .unwrap()
            .content_hash(),
        artifact.content_hash()
    );
    let mut map: std::collections::BTreeMap<String, Box<RawValue>> =
        serde_json::from_slice(&bytes).unwrap();
    let inner = map["document"].get();
    let altered = format!("{{ \n{}", &inner[1..]);
    map.insert("document".into(), RawValue::from_string(altered).unwrap());
    assert_eq!(
        OwnedProfileArtifact::decode_json(&serde_json::to_vec(&map).unwrap()).unwrap_err(),
        OwnedProfileArtifactError::InvalidHash
    );
}

#[test]
fn launch_target_paint_owner_is_portable_only_inside_a_validated_bank() {
    let source = source();
    let painted = edit(
        &source,
        "/profiles/0/appearanceBank/variants/0/appearance/controlBar/launch_target",
        br#"{"shadowStrength":1}"#,
    );
    let portable = all(&painted).unwrap();
    OwnedProfileArtifact::decode_json(&portable.encode_json().unwrap()).unwrap();
    for pointer in ["/futureRoot", "/profiles/0/futureProfile"] {
        let disguised=edit(&source,pointer,br#"{"appearanceBank":{"variants":[{"appearance":{"controlBar":{"launch_target":{"shadowStrength":1}}}}]}}"#);
        assert!(all(&disguised).is_err());
    }
    let local = edit(
        &source,
        "/profiles/0/futureProfile",
        br#"{"launch_target":{"path":"/private/secret"}}"#,
    );
    assert!(all(&local).is_err());
}
