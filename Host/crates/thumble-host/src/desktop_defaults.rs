//! Explicit, backed-up bootstrap. Receiver startup never probes desktop preferences.
use crate::{legacy_import, paths::HostPaths, runtime::AuthorityLock, storage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{Cursor, Write},
    os::unix::fs::OpenOptionsExt,
    path::Path,
};
const MAXIMUM: usize = 16 * 1024 * 1024;
const BACKUP: &str = "desktop-defaults-source.plist";
const RECEIPT: &str = "desktop-defaults-import.json";
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Receipt {
    #[serde(rename = "sourceSHA256")]
    pub source_sha256: String,
    #[serde(rename = "stateSHA256")]
    pub state_sha256: String,
    pub profile_count: usize,
    pub trusted_client_count: usize,
    pub configuration_revision: u64,
    pub replayed: bool,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| "Defaults backup/receipt already exists or cannot be created safely")?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| "Defaults backup/receipt could not be saved")?;
    Ok(())
}
pub fn import(paths: &HostPaths, source: &Path, expected: &str) -> Result<Receipt, String> {
    if expected.len() != 64
        || !expected
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return Err(
            "Defaults source SHA256 must be exactly 64 lowercase hexadecimal characters".into(),
        );
    }
    let bytes = legacy_import::read_file(source, MAXIMUM)?;
    if hash(&bytes) != expected {
        return Err(
            "Desktop defaults source changed; review its exact SHA256 before importing".into(),
        );
    }
    let value = plist::Value::from_reader(Cursor::new(&bytes))
        .map_err(|_| "Desktop defaults source is not a valid plist")?;
    let fields = value
        .as_dictionary()
        .ok_or("Desktop defaults source must be a plist dictionary")?;
    if !fields.contains_key("PocketPad.gamepadConfigurationProfiles.v1")
        && !fields.contains_key("PocketPad.gamepadCustomization.v1")
    {
        return Err(
            "Desktop defaults source has no saved profile/customization declaration".into(),
        );
    }
    for (key, value) in fields {
        if matches!(
            key.as_str(),
            "PocketPad.gamepadConfigurationProfiles.v1"
                | "PocketPad.gamepadCustomization.v1"
                | "PocketPadMac.trustedClients.v1"
                | "PocketPadMac.keyBindings.v2"
                | "PocketPadMac.keyBindings.v1"
                | "PocketPadMac.profileKeyBindings.v1"
                | "PocketPadMac.outputBindings.v1"
                | "PocketPadMac.profileOutputBindings.v1"
        ) && value
            .as_data()
            .is_some_and(|data| !thumble_core::legacy_json_numbers_are_supported(data))
        {
            return Err("Desktop defaults JSON contains unsupported numeric precision; use exact legacy JSON preparation instead; no authority was written".into());
        }
    }
    let state = storage::migrate_plist_bytes(&bytes, &uuid::Uuid::new_v4().to_string())?;
    thumble_core::ConfigurationDocument::from_state(&state)
        .map_err(|_| "Desktop defaults configuration exceeds supported bounds")?;
    let _lease = AuthorityLock::try_acquire(paths)?
        .ok_or("Stop the receiver/legacy owner before importing desktop defaults")?;
    let backup = paths.state_dir.join(BACKUP);
    let receipt_path = paths.state_dir.join(RECEIPT);
    if fs::symlink_metadata(&paths.state_file).is_ok() {
        let data = legacy_import::read_file(&receipt_path, 4096)
            .map_err(|_| "An authority already exists; desktop defaults never overwrite it")?;
        let mut receipt: Receipt = thumble_protocol::decode_unique_json(&data)
            .map_err(|_| "Desktop defaults receipt is invalid; authority was not changed")?;
        if receipt.source_sha256 != expected
            || receipt.state_sha256 != hash(&storage::read_state_bytes(&paths.state_file)?)
            || hash(&legacy_import::read_file(&backup, MAXIMUM)?) != expected
        {
            return Err(
                "Existing authority/backup differs from the defaults receipt; no data was changed"
                    .into(),
            );
        }
        receipt.replayed = true;
        return Ok(receipt);
    }
    if fs::symlink_metadata(&receipt_path).is_ok() {
        return Err("Defaults receipt exists without its authority; recover the private backup before retrying".into());
    }
    if fs::symlink_metadata(&backup).is_ok() {
        if legacy_import::read_file(&backup, MAXIMUM)? != bytes {
            return Err(
                "Existing defaults backup belongs to another source; no data was changed".into(),
            );
        }
    } else {
        write_new(&backup, &bytes)?;
    }
    fs::File::open(&paths.state_dir)
        .and_then(|f| f.sync_all())
        .map_err(|_| "Defaults backup directory could not be synced")?;
    storage::save_atomic(&paths.state_file, &state)?;
    let receipt = Receipt {
        source_sha256: expected.into(),
        state_sha256: hash(&storage::read_state_bytes(&paths.state_file)?),
        profile_count: state.profiles.len(),
        trusted_client_count: state.trusted_clients.len(),
        configuration_revision: state.configuration_revision,
        replayed: false,
    };
    write_new(
        &receipt_path,
        &serde_json::to_vec(&receipt).map_err(|_| "Defaults receipt could not be encoded")?,
    )?;
    fs::File::open(&paths.state_dir)
        .and_then(|f| f.sync_all())
        .map_err(|_| "Defaults receipt directory could not be synced")?;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    fn fixture(root: &Path) -> (std::path::PathBuf, Vec<u8>) {
        let state = thumble_core::PersistentState::minimal("fixture-server").unwrap();
        let profiles = serde_json::json!({"profiles":state.profiles,"activeProfileID":state.active_profile_id,"defaultProfileID":state.default_profile_id});
        let mut fields = plist::Dictionary::new();
        fields.insert(
            "PocketPadMac.serverIdentity.v1".into(),
            plist::Value::String("fixture-server".into()),
        );
        fields.insert(
            "PocketPad.gamepadConfigurationProfiles.v1".into(),
            plist::Value::Data(serde_json::to_vec(&profiles).unwrap()),
        );
        fields.insert("PocketPadMac.trustedClients.v1".into(),plist::Value::Data(br#"[{"token":"fixture-token","clientName":"Fixture phone","createdAt":1,"lastSeenAt":2}]"#.to_vec()));
        let mut bytes = vec![];
        plist::to_writer_xml(&mut bytes, &plist::Value::Dictionary(fields)).unwrap();
        let source = root.join("source.plist");
        fs::write(&source, &bytes).unwrap();
        (source, bytes)
    }
    #[test]
    fn reviewed_defaults_preserve_pairing_back_up_exactly_and_replay_without_overwrite() {
        let root = tempfile::tempdir().unwrap();
        let (source, bytes) = fixture(root.path());
        let dir = root.path().join("authority");
        let paths = HostPaths::new(dir.clone(), dir.join("control.sock"));
        assert!(import(&paths, &source, &"0".repeat(64)).is_err());
        assert!(!dir.exists());
        let receipt = import(&paths, &source, &hash(&bytes)).unwrap();
        assert_eq!(receipt.profile_count, 1);
        assert_eq!(receipt.trusted_client_count, 1);
        assert!(!receipt.replayed);
        let state = storage::load(&paths.state_file).unwrap();
        assert_eq!(state.server_id, "fixture-server");
        assert!(state.trusted_clients.contains_key("fixture-token"));
        assert_eq!(fs::read(dir.join(BACKUP)).unwrap(), bytes);
        assert_eq!(
            fs::metadata(dir.join(BACKUP)).unwrap().permissions().mode() & 0o777,
            0o600
        );
        let before = fs::read(&paths.state_file).unwrap();
        assert!(import(&paths, &source, &hash(&bytes)).unwrap().replayed);
        assert_eq!(fs::read(&paths.state_file).unwrap(), before);
        let mut state = state;
        state.profiles[0]["name"] = serde_json::json!("Later edit");
        storage::save_atomic(&paths.state_file, &state).unwrap();
        let later = fs::read(&paths.state_file).unwrap();
        assert!(import(&paths, &source, &hash(&bytes)).is_err());
        assert_eq!(fs::read(&paths.state_file).unwrap(), later);
    }
    #[test]
    fn defaults_refuse_running_authority_symlinks_and_precision_loss() {
        let root = tempfile::tempdir().unwrap();
        let (source, bytes) = fixture(root.path());
        let dir = root.path().join("authority");
        let paths = HostPaths::new(dir.clone(), dir.join("control.sock"));
        let lease = AuthorityLock::try_acquire(&paths).unwrap().unwrap();
        assert!(import(&paths, &source, &hash(&bytes)).is_err());
        assert!(!paths.state_file.exists());
        drop(lease);
        let alias = root.path().join("alias.plist");
        std::os::unix::fs::symlink(&source, &alias).unwrap();
        assert!(import(&paths, &alias, &hash(&bytes)).is_err());
        let mut parsed = plist::Value::from_reader(Cursor::new(bytes)).unwrap();
        let value = parsed
            .as_dictionary_mut()
            .unwrap()
            .get_mut("PocketPad.gamepadConfigurationProfiles.v1")
            .unwrap();
        let raw = value.as_data().unwrap();
        let mut profile: serde_json::Value = serde_json::from_slice(raw).unwrap();
        profile["profiles"][0]["futureExact"] = serde_json::json!("PRECISE");
        *value = plist::Value::Data(
            serde_json::to_string(&profile)
                .unwrap()
                .replace("\"PRECISE\"", "0.123456789012345678901234567890123456789")
                .into_bytes(),
        );
        let mut invalid = vec![];
        plist::to_writer_xml(&mut invalid, &parsed).unwrap();
        fs::write(&source, &invalid).unwrap();
        assert!(import(&paths, &source, &hash(&invalid))
            .unwrap_err()
            .contains("precision"));
        assert!(!paths.state_file.exists());
    }
}
