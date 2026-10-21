//! Explicit read-only preparation of local v4 files for owned import.
use crate::bridge::ConfigurationBridge;
use serde::Serialize;
use serde_json::{value::RawValue, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::OpenOptions,
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};
use thumble_core::{
    prepare_legacy_configuration, LegacyMigrationError, LegacyMigrationReview,
    NativeLegacyPreparation, OwnedProfileArtifact, ProfileArtifact, ProfileArtifactContentHash,
    ProfileArtifactSelection, MAXIMUM_CONFIGURATION_DOCUMENT_BYTES,
};
use uuid::Uuid;
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyImportPreparation {
    pub schema_version: u32,
    #[serde(rename = "sourceSHA256")]
    pub source_sha256: String,
    #[serde(rename = "artifactJSON")]
    pub artifact_json: String,
    pub content_hash: ProfileArtifactContentHash,
    pub profiles: Vec<LegacyMigrationReview>,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn failure() -> String {
    "legacy_import_invalid: conversion could not preserve a valid owned artifact; original files are unchanged".into()
}
/// No follows, special files, foreign ownership, or unbounded reads. No file is created.
pub fn read_file(path: &Path, maximum: usize) -> Result<Vec<u8>, String> {
    if !path.is_absolute() {
        return Err("legacy_import_path: provide an absolute source/package path".into());
    }
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| "legacy_import_read: file could not be opened safely")?;
    let before = file
        .metadata()
        .map_err(|_| "legacy_import_read: file metadata unavailable")?;
    if !before.is_file()
        || before.uid() != unsafe { libc::geteuid() }
        || before.len() > maximum as u64
    {
        return Err(
            "legacy_import_read: expected a bounded regular file owned by the current user".into(),
        );
    }
    let mut bytes = Vec::new();
    (&file)
        .take(maximum as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "legacy_import_read: file could not be read")?;
    let after = file
        .metadata()
        .map_err(|_| "legacy_import_read: file metadata unavailable")?;
    if bytes.len() > maximum
        || bytes.len() as u64 != before.len()
        || before.len() != after.len()
        || before.mtime() != after.mtime()
        || before.mtime_nsec() != after.mtime_nsec()
        || before.ctime() != after.ctime()
        || before.ctime_nsec() != after.ctime_nsec()
    {
        return Err("legacy_import_changed: retry with an unchanged bounded file".into());
    }
    Ok(bytes)
}
pub fn prepare(
    source: &[u8],
    packages: &BTreeMap<Uuid, PathBuf>,
    now: i64,
) -> Result<LegacyImportPreparation, String> {
    prepare_with(source, packages, now, |profile, path| {
        let package = path
            .map(|p| read_file(p, 40 * 1024 * 1024))
            .transpose()
            .map_err(|_| LegacyMigrationError::PreparationFailed)?;
        ConfigurationBridge::discover()
            .map_err(|_| LegacyMigrationError::PreparationFailed)?
            .prepare_portable_legacy_profile(profile, package.as_deref(), Uuid::new_v4())
    })
}
fn prepare_with(
    source: &[u8],
    packages: &BTreeMap<Uuid, PathBuf>,
    now: i64,
    mut native: impl FnMut(
        &[u8],
        Option<&Path>,
    ) -> Result<NativeLegacyPreparation, LegacyMigrationError>,
) -> Result<LegacyImportPreparation, String> {
    if source.len() > MAXIMUM_CONFIGURATION_DOCUMENT_BYTES || now < 0 || packages.len() > 256 {
        return Err(failure());
    }
    let value: Value = thumble_protocol::decode_unique_json(source).map_err(|_| failure())?;
    if value["schema"] != "com.codybontecou.pocketpad.keypad-configuration"
        || value["version"].as_f64() != Some(4.0)
    {
        return Err(failure());
    }
    let mut fields: BTreeMap<String, Box<RawValue>> =
        serde_json::from_slice(source).map_err(|_| failure())?;
    if ["artifactVersion", "contentHash", "catalogRevision"]
        .iter()
        .any(|key| fields.contains_key(*key))
    {
        let artifact = ProfileArtifact::decode_import_json(source).map_err(|_| failure())?;
        for key in ["artifactVersion", "contentHash", "catalogRevision"] {
            fields.remove(key);
        }
        for (key, id) in [
            ("activeProfileID", artifact.active_profile_id.as_ref()),
            ("defaultProfileID", artifact.default_profile_id.as_ref()),
        ] {
            fields.insert(
                key.into(),
                serde_json::value::to_raw_value(&id).map_err(|_| failure())?,
            );
        }
        for key in ["keyBindings", "outputBindings"] {
            let map_key = if key == "keyBindings" {
                "profileKeyBindings"
            } else {
                "profileOutputBindings"
            };
            let maps: BTreeMap<String, Box<RawValue>> = fields
                .get(map_key)
                .map(|v| serde_json::from_str(v.get()))
                .transpose()
                .map_err(|_| failure())?
                .unwrap_or_default();
            let active = artifact.active_profile_id.as_deref().ok_or_else(failure)?;
            let alias = maps
                .into_iter()
                .find(|(id, _)| id.eq_ignore_ascii_case(active))
                .map(|(_, v)| v)
                .unwrap_or(
                    serde_json::value::to_raw_value(&serde_json::json!({}))
                        .map_err(|_| failure())?,
                );
            fields.insert(key.into(), alias);
        }
    }
    let document = serde_json::to_vec(&fields).map_err(|_| failure())?;
    let profiles = value["profiles"].as_array().ok_or_else(failure)?;
    let ids: BTreeSet<Uuid> = profiles
        .iter()
        .map(|p| {
            p["id"]
                .as_str()
                .ok_or_else(failure)?
                .parse()
                .map_err(|_| failure())
        })
        .collect::<Result<_, _>>()?;
    if packages.keys().any(|id| !ids.contains(id)) || fields.contains_key("legacyConversion") {
        return Err(failure());
    }
    for profile in profiles {
        let id: Uuid = profile["id"]
            .as_str()
            .ok_or_else(failure)?
            .parse()
            .map_err(|_| failure())?;
        let required = profile.get("skinReference").is_some_and(|v| !v.is_null());
        if required && !packages.contains_key(&id) {
            return Err(format!("legacy_package_required: provide the original package for profile {id}; no candidate was produced"));
        }
        if !required && packages.contains_key(&id) {
            return Err(
                "legacy_package_unexpected: package mapping does not match a referenced profile"
                    .into(),
            );
        }
    }
    let prepared=prepare_legacy_configuration(&document,|profile| {
        let value:Value=serde_json::from_slice(profile).map_err(|_|LegacyMigrationError::InvalidSource)?;
        let id=value["id"].as_str().and_then(|v|v.parse::<Uuid>().ok()).ok_or(LegacyMigrationError::InvalidSource)?;
        native(profile,packages.get(&id).map(PathBuf::as_path))
    }).map_err(|_|"legacy_preparation_required: native conversion requires recovery; no candidate was produced".to_owned())?;
    let source_sha256 = hash(source);
    let report = serde_json::to_vec(
        &serde_json::json!({"sourceSHA256":source_sha256,"profiles":prepared.reviews()}),
    )
    .map_err(|_| failure())?;
    let document = prepared
        .destination()
        .with_edits(&[thumble_protocol::JsonValueEdit {
            pointer: "/legacyConversion",
            value: Some(&report),
        }])
        .map_err(|_| failure())?;
    let artifact =
        OwnedProfileArtifact::from_configuration(&document, ProfileArtifactSelection::All, now)
            .map_err(|error| format!("legacy_owned_artifact_invalid: {error}"))?;
    Ok(LegacyImportPreparation {
        schema_version: 1,
        source_sha256,
        artifact_json: String::from_utf8(artifact.encode_json().map_err(|_| failure())?)
            .map_err(|_| failure())?,
        content_hash: artifact.content_hash().clone(),
        profiles: prepared.reviews().to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const ROOT: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/legacy-configuration-migration/v1/"
    );
    #[test]
    fn frozen_conversions_preserve_source_and_produce_portable_owned_results() {
        for name in [
            "plain",
            "portrait-only",
            "explicit-landscape",
            "legacy-canvas-alias",
            "missing-canvas",
        ] {
            let source = std::fs::read(format!("{ROOT}{name}-source.json")).unwrap();
            let before = source.clone();
            let request: Value = serde_json::from_slice(
                &std::fs::read(format!("{ROOT}{name}-request.json")).unwrap(),
            )
            .unwrap();
            let response = std::fs::read(format!(
                "{ROOT}../../legacy-portable-preparation/v1/{name}-response.json"
            ))
            .unwrap();
            use base64::{engine::general_purpose::STANDARD, Engine};
            let package_hash = request
                .get("packageData")
                .and_then(Value::as_str)
                .map(|v| hash(&STANDARD.decode(v).unwrap()));
            let mappings = if package_hash.is_some() {
                BTreeMap::from([(
                    serde_json::from_slice::<Value>(&source).unwrap()["profiles"][0]["id"]
                        .as_str()
                        .unwrap()
                        .parse::<Uuid>()
                        .unwrap(),
                    PathBuf::from("/private/tmp/frozen-original-package"),
                )])
            } else {
                BTreeMap::new()
            };
            let result = prepare_with(&source, &mappings, 100, |profile, package| {
                assert_eq!(package.is_some(), package_hash.is_some());
                NativeLegacyPreparation::decode_portable(
                    profile,
                    request["invocationID"].as_str().unwrap().parse().unwrap(),
                    package_hash.as_deref(),
                    &response,
                )
            })
            .unwrap();
            assert_eq!(source, before);
            assert_eq!(result.source_sha256, hash(&source));
            assert_eq!(result.profiles.len(), 1);
            let artifact =
                OwnedProfileArtifact::decode_json(result.artifact_json.as_bytes()).unwrap();
            assert!(artifact.document().profiles()[0]
                .get("skinReference")
                .is_none());
            assert_eq!(
                artifact.document().profiles()[0]["appearanceBank"]["variants"]
                    .as_array()
                    .unwrap()
                    .len(),
                8
            );
            assert!(std::str::from_utf8(artifact.document().original_bytes())
                .unwrap()
                .contains("legacyConversion"));
        }
    }
    #[test]
    fn bad_inputs_and_package_mappings_fail_before_native_work() {
        let source = std::fs::read(format!("{ROOT}plain-source.json")).unwrap();
        assert!(prepare_with(b"{}", &BTreeMap::new(), 100, |_, _| panic!(
            "invalid source invoked native work"
        ))
        .is_err());
        let mappings = BTreeMap::from([(Uuid::new_v4(), PathBuf::from("/private/tmp/unused"))]);
        assert!(prepare_with(&source, &mappings, 100, |_, _| panic!(
            "unmatched package invoked native work"
        ))
        .is_err());
        let mut value: Value = serde_json::from_slice(&source).unwrap();
        value["profiles"][0]["skinReference"] =
            serde_json::json!({"identifier":"missing","version":"1"});
        let error = prepare_with(
            &serde_json::to_vec(&value).unwrap(),
            &BTreeMap::new(),
            100,
            |_, _| panic!("missing package invoked native work"),
        )
        .unwrap_err();
        assert!(error.starts_with("legacy_package_required:"));
        assert!(prepare_with(&source, &BTreeMap::new(), 100, |_, _| Err(
            LegacyMigrationError::PreparationFailed
        ))
        .is_err());
    }
    #[test]
    fn bounded_reader_refuses_links_special_files_and_oversized_inputs() {
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("input");
        std::fs::write(&file, b"source").unwrap();
        assert_eq!(read_file(&file, 6).unwrap(), b"source");
        assert!(read_file(&file, 5).is_err());
        let link = directory.path().join("link");
        std::os::unix::fs::symlink(&file, &link).unwrap();
        assert!(read_file(&link, 100).is_err());
        assert!(read_file(directory.path(), 100).is_err());
        assert!(read_file(Path::new("relative"), 100).is_err());
    }
    #[test]
    fn portable_preparation_requires_explicit_decoder_mode() {
        let source = std::fs::read(format!("{ROOT}plain-source.json")).unwrap();
        let profile: Vec<Box<RawValue>> = serde_json::from_str(
            serde_json::from_slice::<BTreeMap<String, Box<RawValue>>>(&source).unwrap()["profiles"]
                .get(),
        )
        .unwrap();
        let request: Value =
            serde_json::from_slice(&std::fs::read(format!("{ROOT}plain-request.json")).unwrap())
                .unwrap();
        let response = std::fs::read(format!(
            "{ROOT}../../legacy-portable-preparation/v1/plain-response.json"
        ))
        .unwrap();
        let invocation = request["invocationID"].as_str().unwrap().parse().unwrap();
        assert!(NativeLegacyPreparation::decode(
            profile[0].get().as_bytes(),
            invocation,
            None,
            &response
        )
        .is_err());
        NativeLegacyPreparation::decode_portable(
            profile[0].get().as_bytes(),
            invocation,
            None,
            &response,
        )
        .unwrap();
    }
}
