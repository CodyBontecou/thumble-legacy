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
