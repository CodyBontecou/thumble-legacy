use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use thumble_core::OwnedConfigurationDocument;
use thumble_host::{bridge::ConfigurationBridge, draft_operation::ConfigurationOperation};
use uuid::Uuid;
fn prepare(
    corrupt: Option<&str>,
) -> (
    tempfile::TempDir,
    ConfigurationBridge,
    OwnedConfigurationDocument,
    ConfigurationOperation,
    i64,
    Uuid,
) {
    let fixtures =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/rich-background/v1");
    let mut request: Value =
        serde_json::from_slice(&fs::read(fixtures.join("request.json")).unwrap()).unwrap();
    let mut response: Value =
        serde_json::from_slice(&fs::read(fixtures.join("response.json")).unwrap()).unwrap();
    if let Some(kind) = corrupt {
        let before: Value = serde_json::from_slice(
            &STANDARD
                .decode(response["beforeDocumentData"].as_str().unwrap())
                .unwrap(),
        )
        .unwrap();
        let mut after: Value = serde_json::from_slice(
            &STANDARD
                .decode(response["afterDocumentData"].as_str().unwrap())
                .unwrap(),
        )
        .unwrap();
        match kind {
            "bank" => {
                after["profiles"][0]["appearanceBank"] =
                    before["profiles"][0]["appearanceBank"].clone()
            }
            "image" => {
                let hash = before["profiles"][0]["appearanceResources"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .next()
                    .unwrap();
                after["profiles"][0]["customization"]["backgroundDarkFillStyle"]["image"]["data"]
                    ["resourceSHA256"] = hash.clone().into();
            }
            _ => unreachable!(),
        }
        response["afterDocumentData"] = STANDARD.encode(serde_json::to_vec(&after).unwrap()).into();
    }
    let source = OwnedConfigurationDocument::decode(
        &STANDARD
            .decode(request["documentData"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    let light = source.profiles()[0]["appearanceBank"]["variants"]
        .as_array()
        .unwrap()
        .iter()
        .position(|row| row["colorScheme"] == "light" && row["actionSet"] == "base")
        .unwrap();
    let pointer = format!("/profiles/0/appearanceBank/variants/{light}/colorScheme");
    let source = source
        .with_edits(&[thumble_protocol::JsonValueEdit {
            pointer: &pointer,
            value: Some(br#""li\u0067ht""#),
        }])
        .unwrap();
    request["documentData"] = STANDARD.encode(source.original_bytes()).into();
    use sha2::{Digest, Sha256};
    response["sourceSHA256"] = format!("{:x}", Sha256::digest(source.original_bytes())).into();
    let operation: ConfigurationOperation =
        serde_json::from_value(request["operation"].clone()).unwrap();
    let now = request["nowMillis"].as_i64().unwrap();
    let invocation = Uuid::parse_str(request["invocationID"].as_str().unwrap()).unwrap();
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    fs::write(
        root.path().join("response.json"),
        serde_json::to_vec(&response).unwrap(),
    )
    .unwrap();
    let executable = root.path().join("thumble-bridge");
    fs::write(&executable, "#!/usr/bin/python3\nimport json,sys\nfrom pathlib import Path\nr=Path(__file__).parent\nassert json.load(sys.stdin)==json.loads((r/'request.json').read_bytes())\nprint((r/'response.json').read_text())\n").unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let bridge = ConfigurationBridge::at_path(executable).unwrap();
    (root, bridge, source, operation, now, invocation)
}
#[test]
fn scoped_background_merge_preserves_exact_metadata_and_opaque_resources() {
    let (_root, bridge, source, operation, now, invocation) = prepare(None);
    let (after, outcome) = bridge
        .apply_owned(&source, &operation, now, invocation)
        .unwrap();
    assert!(outcome.changed);
    let text = std::str::from_utf8(after.original_bytes()).unwrap();
    assert!(text.contains("0.12345678901234567890123456789"));
    assert!(text.contains(r#""li\u0067ht""#));
    assert!(text.contains("0.98765432109876543210987654321"));
    assert_eq!(source.profiles()[1], after.profiles()[1]);
    let id = source.profiles()[0]["futureResource"]["resourceSHA256"]
        .as_str()
        .unwrap();
    assert_eq!(
        source.profiles()[0]["appearanceResources"][id],
        after.profiles()[0]["appearanceResources"][id]
    );
    assert_eq!(
        source.profiles()[0]["launchTarget"],
        after.profiles()[0]["launchTarget"]
    );
    assert_eq!(
        source.profiles()[0]["customization"]["elements"],
        after.profiles()[0]["customization"]["elements"]
    );
}
#[test]
fn rich_background_rejects_residual_saved_bank_paint() {
    let (_root, bridge, source, operation, now, invocation) = prepare(Some("bank"));
    assert!(bridge
        .apply_owned(&source, &operation, now, invocation)
        .is_err());
}
#[test]
fn rich_background_rejects_a_valid_blob_with_the_wrong_image_bytes() {
    let (_root, bridge, source, operation, now, invocation) = prepare(Some("image"));
    assert!(bridge
        .apply_owned(&source, &operation, now, invocation)
        .is_err());
}
