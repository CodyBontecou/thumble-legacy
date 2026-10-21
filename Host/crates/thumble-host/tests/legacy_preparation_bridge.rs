use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;
use std::{fs, os::unix::fs::PermissionsExt, path::PathBuf};
use thumble_core::{prepare_legacy_configuration, LegacyMigrationError};
use thumble_host::bridge::ConfigurationBridge;
use uuid::Uuid;

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/legacy-configuration-migration/v1")
            .join(name),
    )
    .unwrap()
}
fn bridge(corrupt: bool) -> (tempfile::TempDir, ConfigurationBridge) {
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("thumble-bridge");
    let request = String::from_utf8(fixture("portrait-only-request.json")).unwrap();
    let response = String::from_utf8(fixture("portrait-only-response.json")).unwrap();
    let script = format!(
        r#"#!/usr/bin/python3
import json,sys
from pathlib import Path
Path(__file__).with_suffix('.ran').touch()
request=json.load(sys.stdin)
expected=json.loads({request})
assert request['invocationID'].lower()==expected['invocationID'].lower()
request['invocationID']=expected['invocationID']
assert request==expected
response=json.loads({response})
if {corrupt}: response['invocationID']='00000000-0000-0000-0000-000000000000'
print(json.dumps(response,separators=(',',':')))
"#,
        request = serde_json::to_string(&request).unwrap(),
        response = serde_json::to_string(&response).unwrap(),
        corrupt = if corrupt { "True" } else { "False" }
    );
    fs::write(&executable, script).unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
    let helper = ConfigurationBridge::at_path(executable).unwrap();
    (directory, helper)
}
#[test]
fn bounded_host_adapter_preserves_exact_source_and_package_bytes() {
    let (_directory, bridge) = bridge(false);
    let request: Value = serde_json::from_slice(&fixture("portrait-only-request.json")).unwrap();
    let package = STANDARD
        .decode(request["packageData"].as_str().unwrap())
        .unwrap();
    let invocation = Uuid::parse_str(request["invocationID"].as_str().unwrap()).unwrap();
    let source = fixture("portrait-only-source.json");
    let result = prepare_legacy_configuration(&source, |profile| {
        bridge.prepare_legacy_profile(profile, Some(&package), invocation)
    })
    .unwrap();
    assert_eq!(result.original_bytes(), source);
    assert_eq!(
        result.destination().original_bytes(),
        fixture("portrait-only-destination.json")
    );
    assert!(result.reviews()[0].package_sha256.is_some());
}
#[test]
fn uncorrelated_responses_and_oversized_inputs_fail_before_any_candidate() {
    let (directory, helper) = bridge(true);
    let request: Value = serde_json::from_slice(&fixture("portrait-only-request.json")).unwrap();
    let package = STANDARD
        .decode(request["packageData"].as_str().unwrap())
        .unwrap();
    let invocation = Uuid::parse_str(request["invocationID"].as_str().unwrap()).unwrap();
    assert!(matches!(
        helper.prepare_legacy_profile(&vec![0; 8 * 1024 * 1024 + 1], None, invocation),
        Err(LegacyMigrationError::TooLarge)
    ));
    assert!(!directory.path().join("thumble-bridge.ran").exists());
    assert!(matches!(
        helper.prepare_legacy_profile(
            &fixture("portrait-only-profile.json"),
            Some(&package),
            invocation
        ),
        Err(LegacyMigrationError::InvalidPreparation)
    ));
}
