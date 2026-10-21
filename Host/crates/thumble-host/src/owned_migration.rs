//! Backed-up owned-state transactions. No automatic migration or CLI activation.
use crate::{paths::HostPaths, runtime::AuthorityLock, storage};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fmt,
    fs::{self, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::Arc,
};
use thumble_core::{
    prepare_legacy_configuration, ConfigurationDocument, LegacyMigrationError,
    LegacyMigrationReview, NativeLegacyPreparation, PersistentState, CURRENT_SCHEMA_VERSION,
    OWNED_SCHEMA_VERSION,
};
use thumble_protocol::{edit_json_values, JsonValueEdit};
const MAXIMUM: usize = 16 * 1024 * 1024;
const MAXIMUM_PACKAGE: usize = 40 * 1024 * 1024;
const MAXIMUM_PACKAGES: usize = 512 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationError {
    InvalidSource,
    Preparation,
    InvalidJournal,
    Conflict,
    Security,
    Storage,
}
impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidSource => "migration source failed validation",
            Self::Preparation => "profile migration requires recovery",
            Self::InvalidJournal => "migration backup or journal requires recovery",
            Self::Conflict => "migration source changed; preserved data requires review",
            Self::Security => "migration storage or authority lease failed validation",
            Self::Storage => "migration data could not be persisted",
        })
    }
}
impl std::error::Error for MigrationError {}

/// Opaque source and destination are private; Debug never prints credentials.
pub struct OwnedStateMigration {
    source: Arc<[u8]>,
    destination: Arc<[u8]>,
    receipt: MigrationReceipt,
    packages: BTreeMap<String, Arc<[u8]>>,
}
impl OwnedStateMigration {
    pub fn source_bytes(&self) -> &[u8] {
        &self.source
    }
    pub fn destination_bytes(&self) -> &[u8] {
        &self.destination
    }
    pub fn receipt(&self) -> &MigrationReceipt {
        &self.receipt
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MigrationPhase {
    Prepared,
    Committed,
    RollbackPrepared,
    RolledBack,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MigrationReceipt {
    pub migration_version: u32,
    pub source_sha256: String,
    pub destination_sha256: String,
    pub base_revision: u64,
    pub result_revision: u64,
    pub phase: MigrationPhase,
    pub profiles: Vec<LegacyMigrationReview>,
    pub package_sha256: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollback_sha256: Option<String>,
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn state(bytes: &[u8]) -> Result<PersistentState, MigrationError> {
    if bytes.len() > MAXIMUM {
        return Err(MigrationError::InvalidSource);
    }
    let state: PersistentState =
        thumble_protocol::decode_unique_json(bytes).map_err(|_| MigrationError::InvalidSource)?;
    state
        .validate()
        .map_err(|_| MigrationError::InvalidSource)?;
    Ok(state)
}

/// Original archive bytes allow rollback after the package library is retired.
pub struct PreparedMigrationProfile {
    pub preparation: NativeLegacyPreparation,
    pub original_package: Option<Vec<u8>>,
}

/// Exact-source preparation, with only credential-free profiles sent to native
/// preparation. Whole receiver state, credentials and opaque metadata stay here.
pub fn prepare(
    source: &[u8],
    now_millis: i64,
    mut prepare_profile: impl FnMut(&[u8]) -> Result<PreparedMigrationProfile, LegacyMigrationError>,
) -> Result<OwnedStateMigration, MigrationError> {
    let original = state(source)?;
    if original.schema_version != CURRENT_SCHEMA_VERSION || now_millis < 0 {
        return Err(MigrationError::InvalidSource);
    }
    ConfigurationDocument::from_state(&original).map_err(|_| MigrationError::InvalidSource)?;
    let revision = original
        .configuration_revision
        .checked_add(1)
        .ok_or(MigrationError::InvalidSource)?;
    let raw: BTreeMap<String, &RawValue> =
        serde_json::from_slice(source).map_err(|_| MigrationError::InvalidSource)?;
    let mut document = String::from(
        "{\"schema\":\"com.codybontecou.pocketpad.keypad-configuration\",\"version\":4",
    );
    for (name, aliases) in [
        ("profiles", &[][..]),
        (
            "activeProfileID",
            &["activeProfileId", "activeGamepadProfileID"][..],
        ),
        (
            "defaultProfileID",
            &["defaultProfileId", "defaultGamepadProfileID"][..],
        ),
        ("keyBindings", &[][..]),
        ("outputBindings", &[][..]),
        ("profileKeyBindings", &[][..]),
        ("profileOutputBindings", &[][..]),
    ] {
        if let Some(value) = raw
            .get(name)
            .or_else(|| aliases.iter().find_map(|alias| raw.get(*alias)))
        {
            document.push(',');
            document.push_str(&format!("\"{name}\":"));
            document.push_str(value.get());
        }
    }
    document.push('}');
    let mut packages: BTreeMap<String, Arc<[u8]>> = BTreeMap::new();
    let mut package_bytes = 0usize;
    let converted = prepare_legacy_configuration(document.as_bytes(), |profile| {
        let prepared = prepare_profile(profile)?;
        if let Some(package) = prepared.original_package {
            if package.len() > MAXIMUM_PACKAGE {
                return Err(LegacyMigrationError::TooLarge);
            }
            let hash = digest(&package);
            if let std::collections::btree_map::Entry::Vacant(entry) = packages.entry(hash) {
                package_bytes = package_bytes
                    .checked_add(package.len())
                    .filter(|size| *size <= MAXIMUM_PACKAGES)
                    .ok_or(LegacyMigrationError::TooLarge)?;
                entry.insert(Arc::from(package));
            }
        }
        Ok(prepared.preparation)
    })
    .map_err(|_| MigrationError::Preparation)?;
    let required: std::collections::BTreeSet<_> = converted
        .reviews()
        .iter()
        .filter_map(|review| review.package_sha256.clone())
        .collect();
    if required.iter().ne(packages.keys()) {
        return Err(MigrationError::Preparation);
    }

    let fields: BTreeMap<String, &RawValue> =
        serde_json::from_slice(converted.destination().original_bytes())
            .map_err(|_| MigrationError::Preparation)?;
    let version = OWNED_SCHEMA_VERSION.to_string();
    let revision_json = revision.to_string();
    let updated = now_millis
        .max(original.configuration_updated_at)
        .to_string();
    let destination = edit_json_values(
        source,
        &[
            JsonValueEdit {
                pointer: "/profiles",
                value: Some(fields["profiles"].get().as_bytes()),
            },
            JsonValueEdit {
                pointer: "/schemaVersion",
                value: Some(version.as_bytes()),
            },
            JsonValueEdit {
                pointer: "/configurationRevision",
                value: Some(revision_json.as_bytes()),
            },
            JsonValueEdit {
                pointer: "/configurationUpdatedAt",
                value: Some(updated.as_bytes()),
            },
        ],
        MAXIMUM,
    )
    .map_err(|_| MigrationError::Preparation)?;
    let final_state = state(&destination)?;
    retained_authority(&original, &final_state)?;
    Ok(OwnedStateMigration {
        source: Arc::from(source),
        destination: Arc::from(destination.as_slice()),
        receipt: MigrationReceipt {
            migration_version: 1,
            source_sha256: digest(source),
            destination_sha256: digest(&destination),
            base_revision: original.configuration_revision,
            result_revision: revision,
            phase: MigrationPhase::Prepared,
            profiles: converted.reviews().to_vec(),
            package_sha256: packages.keys().cloned().collect(),
            rollback_sha256: None,
        },
        packages,
    })
}
fn retained_authority(
    before: &PersistentState,
    after: &PersistentState,
) -> Result<(), MigrationError> {
    if before.server_id != after.server_id
        || before.trusted_clients != after.trusted_clients
        || before.active_profile_id != after.active_profile_id
        || before.default_profile_id != after.default_profile_id
        || before.key_bindings != after.key_bindings
        || before.output_bindings != after.output_bindings
        || before.profile_key_bindings != after.profile_key_bindings
        || before.profile_output_bindings != after.profile_output_bindings
        || before.recent_configuration_commits != after.recent_configuration_commits
        || before.profile_artifact_adoptions != after.profile_artifact_adoptions
    {
        return Err(MigrationError::InvalidJournal);
    }
    Ok(())
}
fn directory(path: &Path) -> Result<(), MigrationError> {
    match fs::DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(_) => return Err(MigrationError::Storage),
    }
    let m = fs::symlink_metadata(path).map_err(|_| MigrationError::Security)?;
    if !m.is_dir()
        || m.file_type().is_symlink()
        || m.uid() != unsafe { libc::geteuid() }
        || m.permissions().mode() & 0o077 != 0
    {
        return Err(MigrationError::Security);
    }
    fs::File::open(path.parent().ok_or(MigrationError::Security)?)
        .and_then(|p| p.sync_all())
        .map_err(|_| MigrationError::Storage)
}
fn journal(paths: &HostPaths, lease: &AuthorityLock, id: &str) -> Result<PathBuf, MigrationError> {
    if !lease.protects(paths)
        || id.len() != 64
        || !id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(MigrationError::Security);
    }
    let root = paths.state_dir.join("owned-migrations-v1");
    directory(&root)?;
    let path = root.join(id);
    directory(&path)?;
    Ok(path)
}
fn read(path: &Path) -> Result<Vec<u8>, MigrationError> {
    read_bounded(path, MAXIMUM)
}
fn read_bounded(path: &Path, maximum: usize) -> Result<Vec<u8>, MigrationError> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| MigrationError::InvalidJournal)?;
    let metadata = file
        .metadata()
        .map_err(|_| MigrationError::InvalidJournal)?;
    if !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.permissions().mode() & 0o077 != 0
        || metadata.len() > maximum as u64
    {
        return Err(MigrationError::InvalidJournal);
    }
    let mut data = Vec::with_capacity(metadata.len() as usize);
    file.take(maximum as u64 + 1)
        .read_to_end(&mut data)
        .map_err(|_| MigrationError::InvalidJournal)?;
    if data.len() > maximum {
        return Err(MigrationError::InvalidJournal);
    }
    Ok(data)
}
fn once(path: &Path, bytes: &[u8]) -> Result<(), MigrationError> {
    if bytes.len() > MAXIMUM_PACKAGE {
        return Err(MigrationError::InvalidJournal);
    }
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path);
    match file {
        Ok(mut file) => {
            file.write_all(bytes)
                .and_then(|()| file.sync_all())
                .map_err(|_| MigrationError::Storage)?;
            fs::File::open(path.parent().ok_or(MigrationError::Security)?)
                .and_then(|p| p.sync_all())
                .map_err(|_| MigrationError::Storage)
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            if read_bounded(path, MAXIMUM_PACKAGE)? == bytes {
                // A previous attempt may have stopped after writing all bytes
                // but before syncing them. Matching content alone is insufficient.
                let file = OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW)
                    .open(path)
                    .map_err(|_| MigrationError::Storage)?;
                file.sync_all().map_err(|_| MigrationError::Storage)?;
                fs::File::open(path.parent().ok_or(MigrationError::Security)?)
                    .and_then(|parent| parent.sync_all())
                    .map_err(|_| MigrationError::Storage)
            } else {
                Err(MigrationError::InvalidJournal)
            }
        }
        Err(_) => Err(MigrationError::Storage),
    }
}
fn write_receipt(path: &Path, receipt: &MigrationReceipt) -> Result<(), MigrationError> {
    let file = path.join("receipt.json");
    let expected = match fs::symlink_metadata(&file) {
        Ok(_) => Some(read(&file)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err(MigrationError::InvalidJournal),
    };
    let bytes = serde_json::to_vec(receipt).map_err(|_| MigrationError::InvalidJournal)?;
    storage::install_private_state_bytes(&file, &bytes, expected.as_deref(), || Ok(()))
        .map_err(|_| MigrationError::Storage)
}
fn load_journal(
    path: &Path,
    id: &str,
) -> Result<(MigrationReceipt, Vec<u8>, Vec<u8>), MigrationError> {
    let receipt_bytes = read(&path.join("receipt.json"))?;
    if receipt_bytes.len() > 512 * 1024 {
        return Err(MigrationError::InvalidJournal);
    }
    let receipt: MigrationReceipt = thumble_protocol::decode_unique_json(&receipt_bytes)
        .map_err(|_| MigrationError::InvalidJournal)?;
    let source = read(&path.join("source-state.json"))?;
    let destination = read(&path.join("destination-state.json"))?;
    let before = state(&source).map_err(|_| MigrationError::InvalidJournal)?;
    let after = state(&destination).map_err(|_| MigrationError::InvalidJournal)?;
    if receipt.migration_version != 1
        || receipt.source_sha256 != id
        || digest(&source) != id
        || digest(&destination) != receipt.destination_sha256
        || before.schema_version != CURRENT_SCHEMA_VERSION
        || after.schema_version != OWNED_SCHEMA_VERSION
        || before.configuration_revision != receipt.base_revision
        || after.configuration_revision != receipt.result_revision
        || receipt.base_revision.checked_add(1) != Some(receipt.result_revision)
    {
        return Err(MigrationError::InvalidJournal);
    }
    retained_authority(&before, &after)?;
    let required: std::collections::BTreeSet<_> = receipt
        .profiles
        .iter()
        .filter_map(|review| review.package_sha256.clone())
        .collect();
    if receipt.profiles.len() != before.profiles.len()
        || required.iter().ne(receipt.package_sha256.iter())
    {
        return Err(MigrationError::InvalidJournal);
    }
    let raw_source: BTreeMap<String, &RawValue> =
        serde_json::from_slice(&source).map_err(|_| MigrationError::InvalidJournal)?;
    let raw_profiles: Vec<&RawValue> = serde_json::from_str(raw_source["profiles"].get())
        .map_err(|_| MigrationError::InvalidJournal)?;
    for ((raw, profile), (review, converted)) in raw_profiles
        .iter()
        .zip(&before.profiles)
        .zip(receipt.profiles.iter().zip(&after.profiles))
    {
        let id = profile
            .get("id")
            .and_then(serde_json::Value::as_str)
            .and_then(|id| uuid::Uuid::parse_str(id).ok());
        if id != Some(review.profile_id)
            || profile.get("id") != converted.get("id")
            || digest(raw.get().as_bytes()) != review.source_sha256
            || !["landscape", "portrait"].contains(&review.source_orientation.as_str())
            || review.reasons.len() > 3
            || review.reasons.iter().any(|reason| {
                ![
                    "portraitArranged",
                    "alternateLayoutArchived",
                    "canvasOrientationCorrected",
                ]
                .contains(&reason.as_str())
            })
            || profile
                .get("skinReference")
                .is_some_and(|value| !value.is_null())
                != review.package_sha256.is_some()
        {
            return Err(MigrationError::InvalidJournal);
        }
    }
    if matches!(
        receipt.phase,
        MigrationPhase::RollbackPrepared | MigrationPhase::RolledBack
    ) != receipt.rollback_sha256.is_some()
    {
        return Err(MigrationError::InvalidJournal);
    }
    if !receipt.package_sha256.is_empty() {
        let metadata = fs::symlink_metadata(path.join("packages"))
            .map_err(|_| MigrationError::InvalidJournal)?;
        if !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || metadata.uid() != unsafe { libc::geteuid() }
            || metadata.permissions().mode() & 0o077 != 0
        {
            return Err(MigrationError::InvalidJournal);
        }
    }
    let mut total = 0usize;
    for hash in &receipt.package_sha256 {
        if hash.len() != 64
            || !hash
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(MigrationError::InvalidJournal);
        }
        let bytes = read_bounded(&path.join("packages").join(hash), MAXIMUM_PACKAGE)?;
        total = total
            .checked_add(bytes.len())
            .filter(|size| *size <= MAXIMUM_PACKAGES)
            .ok_or(MigrationError::InvalidJournal)?;
        if digest(&bytes) != *hash {
            return Err(MigrationError::InvalidJournal);
        }
    }
    Ok((receipt, source, destination))
}
#[derive(Clone, Copy)]
enum Checkpoint {
    Backup,
    Destination,
    Prepared,
    Installed,
    RollbackInstalled,
}
/// Commit a validated candidate under the receiver's exact authority lease.
/// Original backup and staged assets are durable before the prepared receipt.
pub fn commit(
    paths: &HostPaths,
    lease: &AuthorityLock,
    candidate: &OwnedStateMigration,
) -> Result<MigrationReceipt, MigrationError> {
    commit_with_hook(paths, lease, candidate, |_| Ok(()))
}
fn commit_with_hook(
    paths: &HostPaths,
    lease: &AuthorityLock,
    candidate: &OwnedStateMigration,
    mut hook: impl FnMut(Checkpoint) -> Result<(), MigrationError>,
) -> Result<MigrationReceipt, MigrationError> {
    let path = journal(paths, lease, &candidate.receipt.source_sha256)?;
    if fs::symlink_metadata(path.join("receipt.json")).is_ok() {
        let (receipt, _, _) = load_journal(&path, &candidate.receipt.source_sha256)?;
        if receipt.destination_sha256 != candidate.receipt.destination_sha256
            || matches!(
                receipt.phase,
                MigrationPhase::RollbackPrepared | MigrationPhase::RolledBack
            )
        {
            return Err(MigrationError::Conflict);
        }
        return recover(paths, lease, &receipt.source_sha256, |_, _| {
            Err(MigrationError::Preparation)
        });
    }
    if read(&paths.state_file)? != candidate.source.as_ref() {
        return Err(MigrationError::Conflict);
    }
    once(&path.join("source-state.json"), &candidate.source)?;
    if !candidate.packages.is_empty() {
        directory(&path.join("packages"))?;
        for (hash, bytes) in &candidate.packages {
            once(&path.join("packages").join(hash), bytes)?;
        }
    }
    hook(Checkpoint::Backup)?;
    once(&path.join("destination-state.json"), &candidate.destination)?;
    hook(Checkpoint::Destination)?;
    write_receipt(&path, &candidate.receipt)?;
    hook(Checkpoint::Prepared)?;
    storage::install_private_state_bytes(
        &paths.state_file,
        &candidate.destination,
        Some(&candidate.source),
        || Ok(()),
    )
    .map_err(|_| MigrationError::Conflict)?;
    hook(Checkpoint::Installed)?;
    let mut receipt = candidate.receipt.clone();
    receipt.phase = MigrationPhase::Committed;
    write_receipt(&path, &receipt)?;
    Ok(receipt)
}
/// Explicit recovery of recorded intent; startup wiring remains gated. Never
/// treat a staging file without a valid receipt as a committed migration.
pub fn recover(
    paths: &HostPaths,
    lease: &AuthorityLock,
    id: &str,
    mut restore_package: impl FnMut(&str, &[u8]) -> Result<(), MigrationError>,
) -> Result<MigrationReceipt, MigrationError> {
    let path = journal(paths, lease, id)?;
    let (mut receipt, source, destination) = load_journal(&path, id)?;
    let current = read(&paths.state_file)?;
    match receipt.phase {
        MigrationPhase::Prepared => {
            if current == source {
                storage::install_private_state_bytes(
                    &paths.state_file,
                    &destination,
                    Some(&source),
                    || Ok(()),
                )
                .map_err(|_| MigrationError::Conflict)?;
            } else if current != destination {
                return Err(MigrationError::Conflict);
            }
            // Recovery can observe a rename interrupted before its directory
            // sync. Make that installation durable before acknowledging it.
            fs::File::open(&paths.state_dir)
                .and_then(|directory| directory.sync_all())
                .map_err(|_| MigrationError::Storage)?;
            receipt.phase = MigrationPhase::Committed;
            write_receipt(&path, &receipt)?;
        }
        MigrationPhase::RollbackPrepared => {
            let rollback = read(&path.join("rollback-state.json"))?;
            let restored = state(&rollback).map_err(|_| MigrationError::InvalidJournal)?;
            let mut expected_state = state(&source).map_err(|_| MigrationError::InvalidJournal)?;
            expected_state.configuration_revision = receipt
                .result_revision
                .checked_add(1)
                .ok_or(MigrationError::InvalidJournal)?;
            expected_state.configuration_updated_at = state(&destination)
                .map_err(|_| MigrationError::InvalidJournal)?
                .configuration_updated_at;
            if receipt.rollback_sha256.as_deref() != Some(digest(&rollback).as_str())
                || restored != expected_state
            {
                return Err(MigrationError::InvalidJournal);
            }
            if current == destination {
                for hash in &receipt.package_sha256 {
                    let bytes = read_bounded(&path.join("packages").join(hash), MAXIMUM_PACKAGE)?;
                    restore_package(hash, &bytes)?;
                }
                storage::install_private_state_bytes(
                    &paths.state_file,
                    &rollback,
                    Some(&destination),
                    || Ok(()),
                )
                .map_err(|_| MigrationError::Conflict)?;
            } else if current != rollback {
                return Err(MigrationError::Conflict);
            }
            fs::File::open(&paths.state_dir)
                .and_then(|directory| directory.sync_all())
                .map_err(|_| MigrationError::Storage)?;
            receipt.phase = MigrationPhase::RolledBack;
            write_receipt(&path, &receipt)?;
        }
        MigrationPhase::Committed | MigrationPhase::RolledBack => (),
    }
    Ok(receipt)
}
/// Restore callbacks must durably and idempotently install/verify each archive
/// in the legacy resource store before the original profile state is published.
pub fn rollback(
    paths: &HostPaths,
    lease: &AuthorityLock,
    id: &str,
    restore_package: impl FnMut(&str, &[u8]) -> Result<(), MigrationError>,
) -> Result<MigrationReceipt, MigrationError> {
    rollback_with_hook(paths, lease, id, restore_package, |_| Ok(()))
}
fn rollback_with_hook(
    paths: &HostPaths,
    lease: &AuthorityLock,
    id: &str,
    mut restore_package: impl FnMut(&str, &[u8]) -> Result<(), MigrationError>,
    mut hook: impl FnMut(Checkpoint) -> Result<(), MigrationError>,
) -> Result<MigrationReceipt, MigrationError> {
    let path = journal(paths, lease, id)?;
    let (mut receipt, source, destination) = load_journal(&path, id)?;
    if matches!(
        receipt.phase,
        MigrationPhase::RollbackPrepared | MigrationPhase::RolledBack
    ) {
        return recover(paths, lease, id, restore_package);
    }
    if receipt.phase != MigrationPhase::Committed || read(&paths.state_file)? != destination {
        return Err(MigrationError::Conflict);
    }
    let revision = receipt
        .result_revision
        .checked_add(1)
        .ok_or(MigrationError::InvalidJournal)?
        .to_string();
    let updated = state(&destination)?.configuration_updated_at.to_string();
    let rollback = edit_json_values(
        &source,
        &[
            JsonValueEdit {
                pointer: "/configurationRevision",
                value: Some(revision.as_bytes()),
            },
            JsonValueEdit {
                pointer: "/configurationUpdatedAt",
                value: Some(updated.as_bytes()),
            },
        ],
        MAXIMUM,
    )
    .map_err(|_| MigrationError::InvalidJournal)?;
    state(&rollback)?;
    once(&path.join("rollback-state.json"), &rollback)?;
    receipt.rollback_sha256 = Some(digest(&rollback));
    receipt.phase = MigrationPhase::RollbackPrepared;
    write_receipt(&path, &receipt)?;
    for hash in &receipt.package_sha256 {
        let bytes = read_bounded(&path.join("packages").join(hash), MAXIMUM_PACKAGE)?;
        restore_package(hash, &bytes)?;
    }
    storage::install_private_state_bytes(&paths.state_file, &rollback, Some(&destination), || {
        Ok(())
    })
    .map_err(|_| MigrationError::Conflict)?;
    hook(Checkpoint::RollbackInstalled)?;
    receipt.phase = MigrationPhase::RolledBack;
    write_receipt(&path, &receipt)?;
    Ok(receipt)
}

#[cfg(test)]
#[path = "owned_migration_tests.rs"]
mod tests;
