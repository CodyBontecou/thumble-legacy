use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use thumble_core::OwnedConfigurationDocument;
use thumble_host::{bridge::ConfigurationBridge, draft_operation::ConfigurationOperation};
use uuid::Uuid;
fn fixture(name: &str, suffix: &str) -> Value {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/owned-native-operations/v1");
    serde_json::from_slice(&fs::read(root.join(format!("{name}-{suffix}.json"))).unwrap()).unwrap()
}
fn helper(
    name: &str,
    corrupt: Option<&str>,
) -> (
    tempfile::TempDir,
    ConfigurationBridge,
    OwnedConfigurationDocument,
    ConfigurationOperation,
    Uuid,
) {
    let mut request = fixture(name, "request");
    let mut response = fixture(name, "response");
    let operation: ConfigurationOperation =
        serde_json::from_value(request["operation"].clone()).unwrap();
    let mut canonical = serde_json::to_value(&operation).unwrap();
    if canonical.get("variant").is_some() {
        canonical["variant"] = "primary".into();
    }
    request["operation"] = canonical;
    request["invocationID"] = request["invocationID"]
        .as_str()
        .unwrap()
        .to_lowercase()
        .into();
    if let Some(field) = corrupt {
        match field {
            "nonce" => response["invocationID"] = "00000000-0000-0000-0000-000000000000".into(),
            "source" => response["sourceSHA256"] = "0".repeat(64).into(),
            "paths" => response["changedPaths"] = serde_json::json!(["/trustedClients"]),
            "changed" => response["changed"] = false.into(),
            "before" => response["beforeDocumentData"] = "e30=".into(),
            "output" => {
                let mut after: Value = serde_json::from_slice(
                    &STANDARD
                        .decode(response["afterDocumentData"].as_str().unwrap())
                        .unwrap(),
                )
                .unwrap();
                after["profiles"][0]["customization"]["elements"][0]["output"]["keyboard"]
                    ["keyCode"] = serde_json::json!(23);
                response["afterDocumentData"] =
                    STANDARD.encode(serde_json::to_vec(&after).unwrap()).into();
            }
            _ => panic!("unknown corruption"),
        }
    }
    let source = OwnedConfigurationDocument::decode(
        &STANDARD
            .decode(request["documentData"].as_str().unwrap())
            .unwrap(),
    )
    .unwrap();
    let invocation = Uuid::parse_str(request["invocationID"].as_str().unwrap()).unwrap();
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("thumble-bridge");
    let script = format!(
        r#"#!/usr/bin/python3
import json,sys
from pathlib import Path
Path(__file__).with_suffix('.ran').touch()
request=json.load(sys.stdin)
expected=json.loads({expected})
assert request==expected
print({response})
"#,
        expected = serde_json::to_string(&request.to_string()).unwrap(),
        response = serde_json::to_string(&response.to_string()).unwrap()
    );
    fs::write(&executable, script).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let bridge = ConfigurationBridge::at_path(executable).unwrap();
    (root, bridge, source, operation, invocation)
}
#[test]
fn native_operations_merge_into_opaque_source_without_packages() {
    let output =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/OwnedNativeOperations");
    fs::create_dir_all(&output).unwrap();
    for name in [
        "nullable-select",
        "geometry",
        "haptic",
        "add",
        "delete",
        "scheme",
    ] {
        let (_root, bridge, source, operation, invocation) = helper(name, None);
        let original = source.original_bytes().to_vec();
        let (after, outcome) = bridge
            .apply_owned(&source, &operation, 2000, invocation)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        assert!(outcome.changed);
        assert_eq!(source.original_bytes(), original);
        assert_eq!(source.profiles().len(), after.profiles().len());
        let raw = String::from_utf8(original).unwrap();
        let merged = std::str::from_utf8(after.original_bytes()).unwrap();
        for literal in ["18446744073709551617", "0.12345678901234567890123456789"] {
            if raw.contains(literal) {
                assert!(merged.contains(literal), "{name}");
            }
        }
        assert_eq!(after.profiles()[0]["profileFormatVersion"], 5);
        assert!(!after.profiles()[0]
            .as_object()
            .unwrap()
            .contains_key("landscapeCustomization"));
        fs::write(
            output.join(format!("{name}-destination.json")),
            after.original_bytes(),
        )
        .unwrap();
        let frozen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/owned-native-operations/v1")
            .join(format!("{name}-destination.json"));
        assert_eq!(after.original_bytes(), fs::read(frozen).unwrap(), "{name}");
    }
}
#[test]
fn mismatched_or_malformed_native_responses_cannot_change_source() {
    for corruption in ["nonce", "source", "paths", "changed", "before", "output"] {
        let (_root, bridge, source, operation, invocation) = helper("geometry", Some(corruption));
        let original = source.original_bytes().to_vec();
        assert!(
            bridge
                .apply_owned(&source, &operation, 2000, invocation)
                .is_err(),
            "{corruption}"
        );
        assert_eq!(source.original_bytes(), original);
    }
}
#[test]
fn portrait_and_orientation_mutations_reject_before_native_work() {
    let (root, bridge, source, _, invocation) = helper("geometry", None);
    for operation in [
        serde_json::json!({"type":"orientation.set","profileID":source.active_profile_id(),"preference":"portrait"}),
        serde_json::json!({"type":"customization.reset","profileID":source.active_profile_id(),"variant":"portrait"}),
    ] {
        let operation: ConfigurationOperation = serde_json::from_value(operation).unwrap();
        assert!(bridge
            .apply_owned(&source, &operation, 2000, invocation)
            .is_err());
    }
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}
