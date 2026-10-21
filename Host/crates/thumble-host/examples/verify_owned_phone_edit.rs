//! Developer verification harness. Writes only inside a caller-supplied private fixture directory.
use serde_json::{json, value::RawValue};
use std::{collections::BTreeMap, path::PathBuf};
use thumble_core::PersistentState;
use thumble_host::{
    bridge::ConfigurationBridge,
    owned_drafts::{authority_document, OwnedCommitRequest, OwnedDraftStore},
    owned_phone_edit::{stage, OwnedPhoneStage},
    paths::HostPaths,
    runtime::AuthorityLock,
};
use thumble_protocol::{edit_json_values, JsonValueEdit, OwnedPhoneEdit};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(
        args.len(),
        3,
        "provide helper and a private fixture directory"
    );
    let bridge = ConfigurationBridge::at_path(PathBuf::from(&args[1])).unwrap();
    let root = PathBuf::from(&args[2]);
    let paths = HostPaths::new(root.join("authority"), root.join("control.sock"));
    let _lease = AuthorityLock::try_acquire(&paths).unwrap().unwrap();
    assert!(!paths.state_file.exists());
    let mut source =
        serde_json::to_vec(&PersistentState::minimal("owned-phone-native-fixture").unwrap())
            .unwrap();
    let doc: BTreeMap<String, &RawValue> = serde_json::from_slice(include_bytes!(
        "../../../fixtures/owned-element-sequences/v1/after-reset-and-hide.json"
    ))
    .unwrap();
    for key in [
        "profiles",
        "activeProfileID",
        "defaultProfileID",
        "keyBindings",
        "outputBindings",
        "profileKeyBindings",
        "profileOutputBindings",
    ] {
        if let Some(v) = doc.get(key) {
            source = edit_json_values(
                &source,
                &[JsonValueEdit {
                    pointer: &format!("/{key}"),
                    value: Some(v.get().as_bytes()),
                }],
                16 * 1024 * 1024,
            )
            .unwrap();
        }
    }
    source=edit_json_values(&source,&[JsonValueEdit{pointer:"/schemaVersion",value:Some(b"4")},JsonValueEdit{pointer:"/configurationRevision",value:Some(b"41")},JsonValueEdit{pointer:"/ownedConfigurationExtensions",value:Some(br#"{"phoneFuture":{"integer":9007199254740993,"fraction":0.12345678901234567890123456789}}"#)}],16*1024*1024).unwrap();
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut initial = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&paths.state_file)
        .unwrap();
    initial.write_all(&source).unwrap();
    initial.sync_all().unwrap();
    drop(initial);
    let before = authority_document(&source).unwrap();
    let profile = &before.profiles()[0];
    let mut intent = OwnedPhoneEdit::decode_json(include_bytes!(
        "../../../fixtures/owned-phone-edits/v1/intent.json"
    ))
    .unwrap();
    assert_eq!(
        intent.profile_id.to_string(),
        profile["id"].as_str().unwrap().to_ascii_lowercase()
    );
    let encoded = serde_json::to_vec(&intent).unwrap();
    intent = OwnedPhoneEdit::decode_json(&encoded).unwrap();
    let prepared = match stage(&paths, &source, &intent, &bridge, 100).unwrap() {
        OwnedPhoneStage::Prepared(p) => p,
        _ => panic!(),
    };
    assert_eq!(std::fs::read(&paths.state_file).unwrap(), source);
    let retry = match stage(&paths, &source, &intent, &bridge, 101).unwrap() {
        OwnedPhoneStage::Prepared(p) => p,
        _ => panic!(),
    };
    assert_eq!(
        retry.draft.document().original_bytes(),
        prepared.draft.document().original_bytes()
    );
    let after = &prepared.draft.document().profiles()[0];
    for (a, b) in before.profiles()[0]["appearanceBank"]["variants"]
        .as_array()
        .unwrap()
        .iter()
        .zip(after["appearanceBank"]["variants"].as_array().unwrap())
    {
        let mut expected = a.clone();
        expected["appearance"]["controls"]
            .as_object_mut()
            .unwrap()
            .remove("ac000000-0000-0000-0000-000000000002");
        assert_eq!(&expected, b);
    }
    assert_eq!(after["appearanceResources"], profile["appearanceResources"]);
    assert_eq!(after["launchTarget"], profile["launchTarget"]);
    assert!(!after["customization"]["elements"]
        .as_array()
        .unwrap()
        .iter()
        .any(|v| v["id"]
            .as_str()
            .is_some_and(|id| id.eq_ignore_ascii_case("AC000000-0000-0000-0000-000000000002"))));
    let original_elements = profile["customization"]["elements"].as_array().unwrap();
    let final_elements = after["customization"]["elements"].as_array().unwrap();
    for original in original_elements {
        let control = original["id"].as_str().unwrap();
        if control.eq_ignore_ascii_case("AC000000-0000-0000-0000-000000000002") {
            continue;
        }
        let actual = final_elements
            .iter()
            .find(|v| {
                v["id"]
                    .as_str()
                    .is_some_and(|id| id.eq_ignore_ascii_case(control))
            })
            .unwrap();
        let mut expected = original.clone();
        let mut actual = actual.clone();
        expected.as_object_mut().unwrap().remove("layout");
        actual.as_object_mut().unwrap().remove("layout");
        assert_eq!(actual, expected);
    }
    fn remove_owner(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                map.retain(|key, _| {
                    !key.eq_ignore_ascii_case("AC000000-0000-0000-0000-000000000002")
                });
                for child in map.values_mut() {
                    remove_owner(child);
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    remove_owner(item);
                }
            }
            _ => {}
        }
    }
    let before_json: serde_json::Value = serde_json::from_slice(before.original_bytes()).unwrap();
    let after_json: serde_json::Value =
        serde_json::from_slice(prepared.draft.document().original_bytes()).unwrap();
    for field in [
        "keyBindings",
        "outputBindings",
        "profileKeyBindings",
        "profileOutputBindings",
    ] {
        let mut expected = before_json[field].clone();
        remove_owner(&mut expected);
        assert_eq!(after_json[field], expected, "{field}");
    }
    let commit = OwnedDraftStore::new(&paths)
        .prepare_commit(
            &source,
            OwnedCommitRequest {
                draft_id: &prepared.draft.draft_id,
                draft_revision: prepared.draft.draft_revision,
                expected: 41,
                commit_id: &prepared.commit_id.to_string(),
                request_digest: Some(&prepared.request_digest),
            },
            102,
        )
        .unwrap();
    let candidate = commit.candidate.unwrap();
    assert_eq!(std::fs::read(&paths.state_file).unwrap(), source);
    let next = paths.state_file.with_extension("fixture-next");
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&next)
        .unwrap();
    output.write_all(&candidate).unwrap();
    output.sync_all().unwrap();
    drop(output);
    std::fs::rename(next, &paths.state_file).unwrap();
    assert!(matches!(
        stage(&paths, &candidate, &intent, &bridge, 103).unwrap(),
        OwnedPhoneStage::AlreadyCommitted(_)
    ));
    assert_eq!(std::fs::read(&paths.state_file).unwrap(), candidate);
    assert!(std::str::from_utf8(&candidate)
        .unwrap()
        .contains("0.12345678901234567890123456789"));
    println!(
        "{}",
        json!({"nativePhoneIntent":true,"stagingReplay":true,"commitReplay":true,"appearanceStates":8,"canonicalBindingsAndResources":true,"exactMetadata":true,"configurationRevision":42,"wireActivation":false})
    );
}
