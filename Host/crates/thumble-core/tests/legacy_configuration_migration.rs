use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use thumble_core::{
    prepare_legacy_configuration, ConfigurationDocument, LegacyMigrationError,
    NativeLegacyPreparation, PersistentState, ProfileAppearanceBank,
};
use uuid::Uuid;

fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/legacy-configuration-migration/v1")
}
fn file(name: &str, suffix: &str) -> Vec<u8> {
    std::fs::read(directory().join(format!("{name}-{suffix}.json"))).unwrap()
}
fn packet(name: &str) -> (Uuid, Option<String>, Vec<u8>) {
    let request: Value = serde_json::from_slice(&file(name, "request")).unwrap();
    let package = request
        .get("packageData")
        .and_then(Value::as_str)
        .map(|data| format!("{:x}", Sha256::digest(STANDARD.decode(data).unwrap())));
    (
        Uuid::parse_str(request["invocationID"].as_str().unwrap()).unwrap(),
        package,
        file(name, "response"),
    )
}
fn document(profile: &[u8]) -> Vec<u8> {
    let data: Value = serde_json::from_slice(profile).unwrap();
    format!(
        r#"{{
"schema":"com.codybontecou.pocketpad.keypad-configuration","version":4,"exportedAt":100,
"profiles":[{}],"activeProfileID":"{}","defaultProfileID":null,
"keyBindings":{{}},"outputBindings":{{}},"profileKeyBindings":{{}},"profileOutputBindings":{{}},
"futureRootMetadata":{{"integer":18446744073709551617,"decimal":0.12345678901234567890123456789}}
}}
"#,
        std::str::from_utf8(profile).unwrap(),
        data["id"].as_str().unwrap()
    )
    .into_bytes()
}
#[test]
fn native_prepared_legacy_profiles_become_real_validated_owned_documents() {
    let output =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/LegacyConfigurationMigration");
    std::fs::create_dir_all(&output).unwrap();
    let mut missing = Vec::new();
    for name in [
        "plain",
        "portrait-only",
        "explicit-landscape",
        "legacy-canvas-alias",
        "missing-canvas",
    ] {
        let profile = file(name, "profile");
        let source = document(&profile);
        let (invocation, package, response) = packet(name);
        let mut calls = 0;
        let result = prepare_legacy_configuration(&source, |raw| {
            calls += 1;
            assert_eq!(raw, profile);
            NativeLegacyPreparation::decode(raw, invocation, package.as_deref(), &response)
        })
        .unwrap();
        assert_eq!(calls, 1);
        assert_eq!(result.original_bytes(), source);
        assert_eq!(result.destination().default_profile_id(), None);
        let candidate = &result.destination().profiles()[0];
        let original: Value = serde_json::from_slice(&profile).unwrap();
        assert_eq!(candidate["id"], original["id"]);
        assert_eq!(candidate["name"], original["name"]);
        assert_eq!(candidate["outputMode"], original["outputMode"]);
        assert_eq!(candidate["updatedAt"], original["updatedAt"]);
        assert_eq!(candidate["profileFormatVersion"], 5);
        assert!(candidate["customization"]["deviceCanvas"]["frameID"]
            .as_str()
            .unwrap()
            .ends_with("-landscape"));
        assert_eq!(
            candidate["customization"]["futureLayoutMetadata"]["scope"],
            if name == "explicit-landscape" {
                "landscape"
            } else {
                "selected"
            }
        );
        if name == "legacy-canvas-alias" {
            assert_eq!(
                candidate["customization"]["deviceCanvas"]["futureCanvasMetadata"],
                "keep"
            );
        }
        if name == "explicit-landscape" {
            assert_eq!(candidate["customization"]["colorSchemePreference"], "light");
        }
        if name == "portrait-only" {
            assert!(result.reviews()[0]
                .reasons
                .contains(&"portraitArranged".into()));
            let native: Value = serde_json::from_slice(&response).unwrap();
            let staged: Value = serde_json::from_slice(
                &STANDARD
                    .decode(native["afterProfileData"].as_str().unwrap())
                    .unwrap(),
            )
            .unwrap();
            let bank = ProfileAppearanceBank::from_profile(candidate)
                .unwrap()
                .unwrap();
            let expected = ProfileAppearanceBank::from_profile(&staged)
                .unwrap()
                .unwrap();
            for scheme in ["light", "dark"] {
                for action in ["base", "left-trigger", "right-trigger", "both-triggers"] {
                    let mut actual = bank
                        .resolve_layout(&candidate["customization"], scheme, action)
                        .unwrap();
                    actual
                        .as_object_mut()
                        .unwrap()
                        .remove("futureLayoutMetadata");
                    assert_eq!(
                        actual,
                        expected
                            .resolve_layout(&staged["customization"], scheme, action)
                            .unwrap()
                    );
                }
            }
        }
        let bytes = result.destination().original_bytes();
        assert!(std::str::from_utf8(bytes)
            .unwrap()
            .contains("18446744073709551617"));
        assert!(std::str::from_utf8(bytes)
            .unwrap()
            .contains("0.12345678901234567890123456789"));
        let again = prepare_legacy_configuration(&source, |raw| {
            NativeLegacyPreparation::decode(raw, invocation, package.as_deref(), &response)
        })
        .unwrap();
        assert_eq!(again.destination().original_bytes(), bytes);
        let integral_float_source = String::from_utf8(source.clone())
            .unwrap()
            .replace("\"version\":4", "\"version\":4.0");
        let float_version = prepare_legacy_configuration(integral_float_source.as_bytes(), |raw| {
            NativeLegacyPreparation::decode(raw, invocation, package.as_deref(), &response)
        })
        .unwrap();
        assert_eq!(
            float_version.original_bytes(),
            integral_float_source.as_bytes()
        );
        assert_eq!(float_version.destination().original_bytes(), bytes);
        let mut state = PersistentState::minimal("migration-gate").unwrap();
        state.profiles = result.destination().profiles().to_vec();
        state.active_profile_id = result.destination().active_profile_id().into();
        state.default_profile_id = state.active_profile_id.clone();
        assert!(ConfigurationDocument::from_state(&state).is_err());
        assert_eq!(thumble_protocol::SUPPORTED_PROFILE_FORMATS, &[4]);
        std::fs::write(output.join(format!("{name}-source.json")), &source).unwrap();
        std::fs::write(output.join(format!("{name}-destination.json")), bytes).unwrap();
        for (suffix, data) in [("source", source.as_slice()), ("destination", bytes)] {
            let frozen = directory().join(format!("{name}-{suffix}.json"));
            match std::fs::read(&frozen) {
                Ok(expected) => assert_eq!(expected, data, "{name}-{suffix}"),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => missing.push(frozen),
                Err(error) => panic!("{error}"),
            }
        }
    }
    assert!(
        missing.is_empty(),
        "Freeze target/LegacyConfigurationMigration outputs: {missing:?}"
    );
}
#[test]
fn preparation_correlation_source_and_package_digests_are_mandatory() {
    let source = file("portrait-only", "profile");
    let (invocation, package, response) = packet("portrait-only");
    assert!(
        NativeLegacyPreparation::decode(&source, invocation, package.as_deref(), &response).is_ok()
    );
    assert!(
        NativeLegacyPreparation::decode(&source, Uuid::nil(), package.as_deref(), &response)
            .is_err()
    );
    assert!(
        NativeLegacyPreparation::decode(&source, invocation, Some(&"0".repeat(64)), &response)
            .is_err()
    );
    assert!(NativeLegacyPreparation::decode(&source, invocation, None, &response).is_err());
    let mut modified = source.clone();
    modified.push(b' ');
    assert!(
        NativeLegacyPreparation::decode(&modified, invocation, package.as_deref(), &response)
            .is_err()
    );
}
#[test]
fn source_and_preparation_failures_return_no_partial_destination() {
    let profile = file("missing-package", "profile");
    let source = document(&profile);
    assert!(matches!(
        prepare_legacy_configuration(&source, |_| Err(LegacyMigrationError::PreparationFailed)),
        Err(LegacyMigrationError::PreparationFailed)
    ));
    assert_eq!(source, document(&profile));
    let invalid = String::from_utf8(source)
        .unwrap()
        .replace("\"version\":4", "\"version\":5");
    assert!(matches!(
        prepare_legacy_configuration(invalid.as_bytes(), |_| panic!(
            "Invalid source must not invoke native preparation"
        )),
        Err(LegacyMigrationError::InvalidSource)
    ));
}
#[test]
fn changed_control_declarations_and_missing_resources_cannot_escape_final_validation() {
    let profile = file("portrait-only", "profile");
    let source = document(&profile);
    let (invocation, package, response) = packet("portrait-only");
    for mutation in ["declaration", "resource", "source-layout", "control-order"] {
        let mut packet: Value = serde_json::from_slice(&response).unwrap();
        if mutation == "source-layout" {
            packet["sourceLayout"] = "portraitCustomization".into();
        } else {
            let mut after: Value = serde_json::from_slice(
                &STANDARD
                    .decode(packet["afterProfileData"].as_str().unwrap())
                    .unwrap(),
            )
            .unwrap();
            if mutation == "resource" {
                after.as_object_mut().unwrap().remove("appearanceResources");
            } else if mutation == "control-order" {
                after["customization"]["elements"]
                    .as_array_mut()
                    .unwrap()
                    .reverse();
            } else {
                after["customization"]["elements"][0]["label"] = "Unexpected change".into();
            }
            packet["afterProfileData"] =
                STANDARD.encode(serde_json::to_vec(&after).unwrap()).into();
        }
        let data = serde_json::to_vec(&packet).unwrap();
        assert!(
            prepare_legacy_configuration(&source, |raw| NativeLegacyPreparation::decode(
                raw,
                invocation,
                package.as_deref(),
                &data
            ))
            .is_err(),
            "{mutation}"
        );
    }
}

#[test]
fn failure_on_a_later_profile_does_not_publish_a_partial_candidate() {
    let first = file("plain", "profile");
    let source = document(&first);
    let mut second: Value = serde_json::from_slice(&first).unwrap();
    second["id"] = "10101010-1111-4222-8333-000000000002".into();
    let original = String::from_utf8(source).unwrap();
    let one = format!("\"profiles\":[{}]", std::str::from_utf8(&first).unwrap());
    let two = format!(
        "\"profiles\":[{},{}]",
        std::str::from_utf8(&first).unwrap(),
        serde_json::to_string(&second).unwrap()
    );
    let source = original.replace(&one, &two).into_bytes();
    let (invocation, package, response) = packet("plain");
    let mut calls = 0;
    let result = prepare_legacy_configuration(&source, |raw| {
        calls += 1;
        if calls == 2 {
            return Err(LegacyMigrationError::PreparationFailed);
        }
        NativeLegacyPreparation::decode(raw, invocation, package.as_deref(), &response)
    });
    assert!(matches!(
        result,
        Err(LegacyMigrationError::PreparationFailed)
    ));
    assert_eq!(calls, 2);
    assert_eq!(source, original.replace(&one, &two).into_bytes());
}

#[test]
fn another_variant_cannot_lose_controls_even_if_a_preparation_omits_the_guard() {
    let mut profile: Value = serde_json::from_slice(&file("plain", "profile")).unwrap();
    let mut variant = profile["customization"].clone();
    let mut extra = variant["elements"][0].clone();
    extra["id"] = "cc000000-0000-4000-8000-000000000001".into();
    variant["elements"].as_array_mut().unwrap().push(extra);
    profile["landscapeCustomization"] = variant;
    let profile = serde_json::to_vec(&profile).unwrap();
    let source = document(&profile);
    let (invocation, package, response) = packet("plain");
    let mut response: Value = serde_json::from_slice(&response).unwrap();
    response["sourceSHA256"] = format!("{:x}", Sha256::digest(&profile)).into();
    let response = serde_json::to_vec(&response).unwrap();
    let result = prepare_legacy_configuration(&source, |raw| {
        NativeLegacyPreparation::decode(raw, invocation, package.as_deref(), &response)
    });
    assert!(matches!(
        result,
        Err(LegacyMigrationError::InvalidPreparation)
    ));
}
