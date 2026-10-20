use thumble_core::{
    ConfigurationDocumentError, OwnedConfigurationDocument, OwnedConfigurationError,
};
use thumble_protocol::JsonValueEdit;

#[test]
fn valid_base64_with_a_wrong_digest_cannot_replace_an_owned_resource() {
    let source = include_bytes!("../../../fixtures/owned-configuration/v1/document.json");
    let original = OwnedConfigurationDocument::decode(source).unwrap();
    let digest = original.profiles()[0]["appearanceResources"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap();
    let pointer = format!("/profiles/0/appearanceResources/{digest}");
    // Canonical standard Base64 for "wrong-bytes": valid syntax, wrong SHA-256.
    let result = original.with_edits(&[JsonValueEdit {
        pointer: &pointer,
        value: Some(br#""d3JvbmctYnl0ZXM=""#),
    }]);
    assert!(matches!(
        result,
        Err(OwnedConfigurationError::Configuration(
            ConfigurationDocumentError::InvalidOwnedProfile
        ))
    ));
    assert_eq!(original.original_bytes(), source);
}

#[test]
fn native_projected_changes_preserve_original_metadata_and_validate_the_final_document() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/owned-projection-merge/v1");
    let source = include_bytes!("../../../fixtures/owned-configuration-edits/v1/input.json");
    let original = OwnedConfigurationDocument::decode(source).unwrap();
    let before =
        OwnedConfigurationDocument::decode(&std::fs::read(directory.join("before.json")).unwrap())
            .unwrap();
    let after =
        OwnedConfigurationDocument::decode(&std::fs::read(directory.join("after.json")).unwrap())
            .unwrap();
    let result = original.with_projected_changes(&before, &after).unwrap();
    assert_eq!(
        original
            .with_projected_changes(&before, &before)
            .unwrap()
            .original_bytes(),
        source
    );
    let expected: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../fixtures/owned-configuration-edits/v1/destination.json"
    ))
    .unwrap();
    let actual: serde_json::Value = serde_json::from_slice(result.original_bytes()).unwrap();
    for (actual, expected) in actual["profiles"]
        .as_array()
        .unwrap()
        .iter()
        .zip(expected["profiles"].as_array().unwrap())
    {
        assert_eq!(actual["id"], expected["id"]);
        assert_eq!(actual["name"], expected["name"]);
        assert_eq!(
            actual["customization"]["elements"][0]["layout"]["centerX"],
            expected["customization"]["elements"][0]["layout"]["centerX"]
        );
        let bank = thumble_core::ProfileAppearanceBank::from_profile(actual).unwrap();
        let expected_bank = thumble_core::ProfileAppearanceBank::from_profile(expected).unwrap();
        let Some(bank) = bank else {
            assert!(expected_bank.is_none());
            assert_eq!(actual["customization"], expected["customization"]);
            continue;
        };
        let expected_bank = expected_bank.unwrap();
        for scheme in ["light", "dark"] {
            for action in ["base", "left-trigger", "right-trigger", "both-triggers"] {
                assert_eq!(
                    bank.resolve_layout(&actual["customization"], scheme, action)
                        .unwrap(),
                    expected_bank
                        .resolve_layout(&expected["customization"], scheme, action)
                        .unwrap()
                );
            }
        }
    }
    let text = std::str::from_utf8(result.original_bytes()).unwrap();
    assert!(text.contains("18446744073709551617"));
    assert!(text.contains("0.12345678901234567890123456789"));
    assert!(actual.get("futureRootMetadata").is_some());
    assert!(actual["profiles"][0].get("futureProfileMetadata").is_some());
    assert_eq!(original.original_bytes(), source);
    assert_eq!(
        result
            .with_projected_changes(&after, &after)
            .unwrap()
            .original_bytes(),
        result.original_bytes()
    );
    let output =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/OwnedProjectionMerge");
    std::fs::create_dir_all(&output).unwrap();
    std::fs::write(output.join("destination.json"), result.original_bytes()).unwrap();
    assert_eq!(
        result.original_bytes(),
        std::fs::read(directory.join("destination.json"))
            .expect("Freeze target/OwnedProjectionMerge/destination.json")
            .as_slice()
    );
}
