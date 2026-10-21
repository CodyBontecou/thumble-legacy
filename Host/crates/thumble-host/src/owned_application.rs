//! App attachment owns only the launch field and its timestamp/resource reachability.
use super::*;
use serde_json::value::RawValue;
use std::collections::BTreeMap;
pub(super) fn merge(
    source: &OwnedConfigurationDocument,
    before: &OwnedConfigurationDocument,
    after: &OwnedConfigurationDocument,
    id: &str,
) -> Result<OwnedConfigurationDocument, ConfigurationBridgeError> {
    let (index, original) = profile_clone::raw_profile(source, id)?;
    let old = profile_clone::raw_profile(before, id)?.1;
    let new = profile_clone::raw_profile(after, id)?.1;
    let parse =
        |profile: &RawValue| -> Result<BTreeMap<String, Box<RawValue>>, ConfigurationBridgeError> {
            serde_json::from_str(profile.get())
                .map_err(|_| ConfigurationBridgeError::InvalidResponse)
        };
    let original = parse(original)?;
    let old = parse(old)?;
    let new = parse(new)?;
    let previous = old.get("launchTarget").filter(|v| v.get() != "null");
    let next = new.get("launchTarget").filter(|v| v.get() != "null");
    let mut launch = next.map(|v| v.get().as_bytes().to_vec());
    if let (Some(raw), Some(previous), Some(next)) = (original.get("launchTarget"), previous, next)
    {
        let a: Value = serde_json::from_str(previous.get())
            .map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
        let b: Value = serde_json::from_str(next.get())
            .map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
        if a.get("filePath") == b.get("filePath")
            && a.get("bundleIdentifier") == b.get("bundleIdentifier")
        {
            launch = Some(
                thumble_protocol::merge_json_projection(
                    raw.get().as_bytes(),
                    previous.get().as_bytes(),
                    next.get().as_bytes(),
                    MAXIMUM,
                )
                .map_err(|_| ConfigurationBridgeError::InvalidResponse)?,
            );
        }
    }
    let timestamp = new
        .get("updatedAt")
        .ok_or(ConfigurationBridgeError::InvalidResponse)?;
    let mut profile_edits = vec![thumble_protocol::JsonValueEdit {
        pointer: "/updatedAt",
        value: Some(timestamp.get().as_bytes()),
    }];
    if launch.is_some() || original.contains_key("launchTarget") {
        profile_edits.push(thumble_protocol::JsonValueEdit {
            pointer: "/launchTarget",
            value: launch.as_deref(),
        });
    }
    let original_bytes = profile_clone::raw_profile(source, id)?.1.get().as_bytes();
    let mut staged =
        thumble_protocol::edit_json_values(original_bytes, &profile_edits, MAXIMUM * 2)
            .map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
    if let Some(incoming) = new.get("appearanceResources") {
        let mut pool: BTreeMap<String, Box<RawValue>> = original
            .get("appearanceResources")
            .map(|v| serde_json::from_str(v.get()))
            .transpose()
            .map_err(|_| ConfigurationBridgeError::InvalidResponse)?
            .unwrap_or_default();
        let incoming: BTreeMap<String, Box<RawValue>> = serde_json::from_str(incoming.get())
            .map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
        for (digest, entry) in incoming {
            if let Some(old) = pool.get(&digest) {
                let a: String = serde_json::from_str(old.get())
                    .map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
                let b: String = serde_json::from_str(entry.get())
                    .map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
                if a != b {
                    return Err(ConfigurationBridgeError::InvalidResponse);
                }
            } else {
                pool.insert(digest, entry);
            }
        }
        let bytes =
            serde_json::to_vec(&pool).map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
        staged = thumble_protocol::edit_json_values(
            &staged,
            &[
                thumble_protocol::JsonValueEdit {
                    pointer: "/appearanceResources",
                    value: Some(&bytes),
                },
                thumble_protocol::JsonValueEdit {
                    pointer: "/appearanceResourceVersion",
                    value: Some(b"1"),
                },
            ],
            MAXIMUM * 2,
        )
        .map_err(|_| ConfigurationBridgeError::InvalidResponse)?;
    }
    let bytes = profile_clone::prune_resources(staged)?;
    source
        .with_edits(&[thumble_protocol::JsonValueEdit {
            pointer: &format!("/profiles/{index}"),
            value: Some(&bytes),
        }])
        .map_err(|_| ConfigurationBridgeError::InvalidResponse)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn app_changes_scope_opaque_metadata_and_prune_only_unreachable_launch_resources() {
        let mut value: Value = serde_json::from_slice(include_bytes!(
            "../../../fixtures/owned-configuration/v1/document.json"
        ))
        .unwrap();
        let data = b"old-bookmark";
        let digest = format!("{:x}", Sha256::digest(data));
        value["profiles"][0]["appearanceResources"][&digest] = json!(STANDARD.encode(data));
        value["profiles"][0]["launchTarget"] = json!({"kind":"application","displayName":"WoW","filePath":"/Applications/WoW.app","attachedAt":100,"bookmarkData":{"resourceSHA256":digest}});
        let before =
            OwnedConfigurationDocument::decode(&serde_json::to_vec(&value).unwrap()).unwrap();
        let precise = b"0.123456789012345678901234567890123456789";
        let source = before
            .with_edits(&[
                thumble_protocol::JsonValueEdit {
                    pointer: "/profiles/0/launchTarget/futureLaunchNumber",
                    value: Some(precise),
                },
                thumble_protocol::JsonValueEdit {
                    pointer: "/profiles/0/futureProfileNumber",
                    value: Some(precise),
                },
            ])
            .unwrap();
        let id = before.profiles()[0]["id"].as_str().unwrap();
        for clear in [false, true] {
            let mut after = value.clone();
            after["profiles"][0]["updatedAt"] = json!(2000);
            if clear {
                after["profiles"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("launchTarget");
                after["profiles"][0]["appearanceResources"]
                    .as_object_mut()
                    .unwrap()
                    .remove(&digest);
            } else {
                after["profiles"][0]["launchTarget"]["displayName"] = json!("Renamed");
                after["profiles"][0]["launchTarget"]["attachedAt"] = json!(2000);
            }
            let after =
                OwnedConfigurationDocument::decode(&serde_json::to_vec(&after).unwrap()).unwrap();
            let actual = merge(&source, &before, &after, id).unwrap();
            let text = std::str::from_utf8(actual.original_bytes()).unwrap();
            assert_eq!(
                text.matches(std::str::from_utf8(precise).unwrap()).count(),
                if clear { 1 } else { 2 }
            );
            assert_eq!(actual.profiles()[1], source.profiles()[1]);
            assert_eq!(
                actual.profiles()[0]["appearanceBank"],
                source.profiles()[0]["appearanceBank"]
            );
            assert_eq!(
                actual.profiles()[0]["appearanceResources"]
                    .get(&digest)
                    .is_none(),
                clear
            );
        }
    }
}
