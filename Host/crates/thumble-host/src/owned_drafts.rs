//! Durable, credential-free raw owned drafts. Legacy drafts keep their own format.
use crate::{
    bridge::ConfigurationBridge,
    control::{ConfigurationDraftSummary, ConfigurationSaveSummary},
    draft_operation::{ConfigurationOperation, ConfigurationOperationOutcome},
    drafts::{
        DraftError, DraftOperationRecord, DraftStore, CONFIGURATION_DRAFT_LIFETIME_MILLIS,
        MAXIMUM_CONFIGURATION_DRAFT_OPERATIONS, MAXIMUM_LIVE_CONFIGURATION_DRAFTS,
    },
    paths::HostPaths,
    storage,
};
use fs2::FileExt;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{value::RawValue, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::Read,
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
use thumble_core::{
    ConfigurationCommitRecord, OwnedConfigurationDocument, PersistentState, OWNED_SCHEMA_VERSION,
};
use thumble_protocol::{edit_json_values, JsonValueEdit};
use uuid::Uuid;
const MAXIMUM_DRAFT: u64 = 18 * 1024 * 1024;
const FIELDS: &[&str] = &[
    "profiles",
    "activeProfileID",
    "defaultProfileID",
    "keyBindings",
    "outputBindings",
    "profileKeyBindings",
    "profileOutputBindings",
];
#[derive(Clone)]
struct Document(OwnedConfigurationDocument);
impl Serialize for Document {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let raw: &RawValue =
            serde_json::from_slice(self.0.original_bytes()).map_err(serde::ser::Error::custom)?;
        raw.serialize(s)
    }
}
impl<'de> Deserialize<'de> for Document {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = Box::<RawValue>::deserialize(d)?;
        OwnedConfigurationDocument::decode(raw.get().as_bytes())
            .map(Self)
            .map_err(|_| serde::de::Error::custom("invalid owned draft document"))
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OwnedDraft {
    version: u32,
    #[serde(rename = "draftID")]
    pub draft_id: String,
    authority_hash: String,
    pub base_configuration_revision: u64,
    pub draft_revision: u64,
    base_document: Document,
    working_document: Document,
    operation_log: Vec<DraftOperationRecord>,
    created_at: i64,
    updated_at: i64,
    expires_at: i64,
}
impl OwnedDraft {
    pub(crate) fn operation_records(&self) -> &[DraftOperationRecord] {
        &self.operation_log
    }
    pub fn document(&self) -> &OwnedConfigurationDocument {
        &self.working_document.0
    }
    pub fn summary(&self) -> Result<ConfigurationDraftSummary, DraftError> {
        Ok(ConfigurationDraftSummary {
            draft_id: self.draft_id.clone(),
            base_configuration_revision: self.base_configuration_revision,
            draft_revision: self.draft_revision,
            profile_count: self.document().profiles().len(),
            active_profile_id: self.document().active_profile_id().into(),
            default_profile_id: self
                .document()
                .default_profile_id()
                .ok_or(DraftError::Malformed)?
                .into(),
            operation_count: self.operation_log.len(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            expires_at: self.expires_at,
        })
    }
}
pub struct OwnedDraftEdit {
    pub draft: OwnedDraft,
    pub outcome: ConfigurationOperationOutcome,
    pub idempotent_replay: bool,
}
#[derive(Clone)]
pub struct OwnedDraftStore {
    paths: HostPaths,
}
impl OwnedDraftStore {
    pub fn new(paths: &HostPaths) -> Self {
        Self {
            paths: paths.clone(),
        }
    }
    fn root(&self) -> PathBuf {
        self.paths.drafts_dir.join("owned-v1")
    }
    fn lock(&self) -> Result<File, DraftError> {
        private_directory(&self.paths.drafts_dir)?;
        private_directory(&self.root())?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(self.root().join("store.lock"))
            .map_err(DraftError::Io)?;
        validate_file(&file)?;
        file.lock_exclusive().map_err(DraftError::Io)?;
        Ok(file)
    }
    fn path(&self, id: &str) -> PathBuf {
        self.root().join(format!("{id}.json"))
    }
    pub fn begin(
        &self,
        source: &[u8],
        expected: u64,
        id: &str,
        now: i64,
    ) -> Result<OwnedDraft, DraftError> {
        let state = validated_state(source)?;
        if state.configuration_revision != expected {
            return Err(DraftError::ConfigurationRevisionConflict {
                expected,
                actual: state.configuration_revision,
            });
        }
        if now < 0 {
            return Err(DraftError::Malformed);
        }
        let document = authority_document(source)?;
        let id = canonical(id)?;
        let _lock = self.lock()?;
        match self.read(&id) {
            Ok(draft) => {
                validate(&draft, &id, now)?;
                if draft.base_configuration_revision != expected
                    || draft.authority_hash != hash(state.server_id.as_bytes())
                    || draft.base_document.0.original_bytes() != document.original_bytes()
                {
                    return Err(DraftError::DraftIdConflict);
                }
                return Ok(draft);
            }
            Err(DraftError::NotFound) => (),
            Err(error) => return Err(error),
        }
        self.prune(now)?;
        if fs::read_dir(self.root())
            .map_err(DraftError::Io)?
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|s| s == "json"))
            .count()
            >= MAXIMUM_LIVE_CONFIGURATION_DRAFTS
        {
            return Err(DraftError::TooManyLiveDrafts);
        }
        let draft = OwnedDraft {
            version: 1,
            draft_id: id,
            authority_hash: hash(state.server_id.as_bytes()),
            base_configuration_revision: expected,
            draft_revision: 1,
            base_document: Document(document.clone()),
            working_document: Document(document),
            operation_log: vec![],
            created_at: now,
            updated_at: now,
            expires_at: now
                .checked_add(CONFIGURATION_DRAFT_LIFETIME_MILLIS)
                .ok_or(DraftError::Malformed)?,
        };
        self.save(&draft)?;
        Ok(draft)
    }
    pub fn get(&self, id: &str, revision: Option<u64>, now: i64) -> Result<OwnedDraft, DraftError> {
        let id = canonical(id)?;
        let _lock = self.lock()?;
        let draft = self.read(&id)?;
        validate(&draft, &id, now)?;
        check_revision(&draft, revision)?;
        Ok(draft)
    }
    pub fn edit(
        &self,
        id: &str,
        revision: u64,
        operation_id: &str,
        operation: &ConfigurationOperation,
        now: i64,
    ) -> Result<OwnedDraftEdit, DraftError> {
        self.edit_with(id, revision, operation_id, operation, now, |document| {
            if let ConfigurationOperation::ProfileRename { profile_id, name } = operation {
                return rename(document, profile_id, name, now);
            }
            ConfigurationBridge::discover()
                .map_err(DraftError::Bridge)?
                .apply_owned(document, operation, now, Uuid::new_v4())
                .map_err(DraftError::Bridge)
        })
    }
    pub fn edit_with<F>(
        &self,
        id: &str,
        revision: u64,
        operation_id: &str,
        operation: &ConfigurationOperation,
        now: i64,
        apply: F,
    ) -> Result<OwnedDraftEdit, DraftError>
    where
        F: FnOnce(
            &OwnedConfigurationDocument,
        ) -> Result<
            (OwnedConfigurationDocument, ConfigurationOperationOutcome),
            DraftError,
        >,
    {
        let descriptor = DraftStore::operation_digest(operation)?;
        self.edit_with_descriptor(id, revision, operation_id, &descriptor, now, apply)
    }

    /// Internal adapters may supply a digest of their typed operation descriptor.
    /// Replay protection is identical to ordinary configuration operations.
    pub(crate) fn edit_with_descriptor<F>(
        &self,
        id: &str,
        revision: u64,
        operation_id: &str,
        descriptor: &str,
        now: i64,
        apply: F,
    ) -> Result<OwnedDraftEdit, DraftError>
    where
        F: FnOnce(
            &OwnedConfigurationDocument,
        ) -> Result<
            (OwnedConfigurationDocument, ConfigurationOperationOutcome),
            DraftError,
        >,
    {
        if descriptor.len() != 64
            || !descriptor
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(DraftError::InvalidOperationOutcome);
        }
        let descriptor = descriptor.to_owned();
        let id = canonical(id)?;
        let operation_id = canonical(operation_id)?;
        let _lock = self.lock()?;
        let mut draft = self.read(&id)?;
        validate(&draft, &id, now)?;
        if let Some(record) = draft
            .operation_log
            .iter()
            .find(|record| record.operation_id == operation_id)
        {
            if record.operation_digest != descriptor {
                return Err(DraftError::OperationIdConflict);
            }
            if record.base_draft_revision != revision {
                return Err(DraftError::DraftRevisionConflict {
                    expected: revision,
                    actual: record.base_draft_revision,
                });
            }
            let outcome = ConfigurationOperationOutcome {
                changed: record.changed,
                changed_paths: record.changed_paths.clone(),
            };
            return Ok(OwnedDraftEdit {
                draft,
                outcome,
                idempotent_replay: true,
            });
        }
        check_revision(&draft, Some(revision))?;
        if draft.operation_log.len() >= MAXIMUM_CONFIGURATION_DRAFT_OPERATIONS {
            return Err(DraftError::TooManyOperations);
        }
        let (document, outcome) = apply(draft.document())?;
        let raw: &RawValue = serde_json::from_slice(document.original_bytes())
            .map_err(|_| DraftError::InvalidOperationOutcome)?;
        if raw.get().as_bytes() != document.original_bytes()
            || outcome.changed != (document.original_bytes() != draft.document().original_bytes())
            || outcome.changed == outcome.changed_paths.is_empty()
            || document.default_profile_id().is_none()
            || outcome.changed_paths.len() > 128
            || outcome
                .changed_paths
                .iter()
                .any(|p| p.len() > 512 || !p.starts_with('/') || p.chars().any(char::is_control))
        {
            return Err(DraftError::InvalidOperationOutcome);
        }
        let next = if outcome.changed {
            revision
                .checked_add(1)
                .ok_or(DraftError::DraftRevisionExhausted)?
        } else {
            revision
        };
        if outcome.changed {
            draft.working_document = Document(document);
            draft.draft_revision = next;
            draft.updated_at = now.max(draft.updated_at);
        }
        draft.operation_log.push(DraftOperationRecord {
            operation_id,
            operation_digest: descriptor,
            base_draft_revision: revision,
            result_draft_revision: next,
            changed: outcome.changed,
            changed_paths: outcome.changed_paths.clone(),
            applied_at: now,
        });
        self.save(&draft)?;
        Ok(OwnedDraftEdit {
            draft,
            outcome,
            idempotent_replay: false,
        })
    }
    pub fn discard(&self, id: &str, revision: u64, now: i64) -> Result<(), DraftError> {
        let id = canonical(id)?;
        let _lock = self.lock()?;
        let draft = self.read(&id)?;
        validate(&draft, &id, now)?;
        check_revision(&draft, Some(revision))?;
        fs::remove_file(self.path(&id)).map_err(DraftError::Io)?;
        File::open(self.root())
            .and_then(|f| f.sync_all())
            .map_err(DraftError::Io)
    }
    fn read(&self, id: &str) -> Result<OwnedDraft, DraftError> {
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(self.path(id))
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    DraftError::NotFound
                } else {
                    DraftError::InsecureFile
                }
            })?;
        validate_file(&file)?;
        let size = file.metadata().map_err(DraftError::Io)?.len();
        if size > MAXIMUM_DRAFT {
            return Err(DraftError::TooLarge(size));
        }
        let mut bytes = Vec::with_capacity(size as usize);
        file.take(MAXIMUM_DRAFT + 1)
            .read_to_end(&mut bytes)
            .map_err(DraftError::Io)?;
        if bytes.len() as u64 > MAXIMUM_DRAFT {
            return Err(DraftError::TooLarge(bytes.len() as u64));
        }
        thumble_protocol::decode_unique_json(&bytes).map_err(|_| DraftError::Malformed)
    }
    fn save(&self, draft: &OwnedDraft) -> Result<(), DraftError> {
        validate(draft, &draft.draft_id, draft.updated_at)?;
        let bytes = serde_json::to_vec(draft).map_err(|_| DraftError::EncodingFailed)?;
        if bytes.len() as u64 > MAXIMUM_DRAFT {
            return Err(DraftError::TooLarge(bytes.len() as u64));
        }
        let path = self.path(&draft.draft_id);
        let expected = match fs::symlink_metadata(&path) {
            Ok(_) => Some(read_raw(&path)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => return Err(DraftError::InsecureFile),
        };
        // Same atomic writer and source check as authority state; draft's bound
        // is 18 MiB, so use its independent byte installer below.
        atomic_draft(&path, &bytes, expected.as_deref())
    }
    fn prune(&self, now: i64) -> Result<(), DraftError> {
        for entry in fs::read_dir(self.root()).map_err(DraftError::Io)? {
            let path = entry.map_err(DraftError::Io)?.path();
            let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if path.extension().is_none_or(|s| s != "json") || canonical(id).is_err() {
                continue;
            }
            if let Ok(draft) = self.read(id) {
                if validate(&draft, id, draft.created_at).is_ok() && now >= draft.expires_at {
                    fs::remove_file(path).map_err(DraftError::Io)?;
                }
            }
        }
        File::open(self.root())
            .and_then(|f| f.sync_all())
            .map_err(DraftError::Io)
    }
}
fn canonical(id: &str) -> Result<String, DraftError> {
    Uuid::parse_str(id)
        .map(|id| id.to_string())
        .map_err(|_| DraftError::InvalidDraftId)
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn valid_hash(text: &str) -> bool {
    text.len() == 64
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn check_revision(draft: &OwnedDraft, expected: Option<u64>) -> Result<(), DraftError> {
    if let Some(expected) = expected {
        if draft.draft_revision != expected {
            return Err(DraftError::DraftRevisionConflict {
                expected,
                actual: draft.draft_revision,
            });
        }
    }
    Ok(())
}
fn validate(draft: &OwnedDraft, id: &str, now: i64) -> Result<(), DraftError> {
    if draft.version != 1 {
        return Err(DraftError::UnsupportedVersion);
    }
    if draft.draft_id != id
        || canonical(id)? != id
        || draft.base_configuration_revision == 0
        || draft.draft_revision == 0
        || !valid_hash(&draft.authority_hash)
        || draft.created_at < 0
        || draft.updated_at < draft.created_at
        || draft.updated_at >= draft.expires_at
        || draft.expires_at.checked_sub(draft.created_at)
            != Some(CONFIGURATION_DRAFT_LIFETIME_MILLIS)
        || draft.base_document.0.default_profile_id().is_none()
        || draft.document().default_profile_id().is_none()
    {
        return Err(DraftError::Malformed);
    }
    if draft.operation_log.len() > MAXIMUM_CONFIGURATION_DRAFT_OPERATIONS {
        return Err(DraftError::TooManyOperations);
    }
    let mut ids = BTreeSet::new();
    let mut revision = 1u64;
    for record in &draft.operation_log {
        if canonical(&record.operation_id)? != record.operation_id
            || !ids.insert(&record.operation_id)
            || !valid_hash(&record.operation_digest)
            || record.base_draft_revision != revision
            || record.result_draft_revision
                != revision
                    .checked_add(u64::from(record.changed))
                    .ok_or(DraftError::Malformed)?
            || record.changed == record.changed_paths.is_empty()
            || record.changed_paths.len() > 128
            || record
                .changed_paths
                .iter()
                .any(|p| p.len() > 512 || !p.starts_with('/') || p.chars().any(char::is_control))
            || record.applied_at < draft.created_at
            || record.applied_at >= draft.expires_at
        {
            return Err(DraftError::Malformed);
        }
        revision = record.result_draft_revision;
    }
    if revision != draft.draft_revision {
        return Err(DraftError::Malformed);
    }
    if now >= draft.expires_at {
        return Err(DraftError::Expired);
    }
    if now < draft.created_at {
        return Err(DraftError::Malformed);
    }
    Ok(())
}
fn private_directory(path: &Path) -> Result<(), DraftError> {
    match fs::DirBuilder::new().mode(0o700).create(path) {
        Ok(()) => (),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => (),
        Err(e) => return Err(DraftError::Io(e)),
    }
    let m = fs::symlink_metadata(path).map_err(DraftError::Io)?;
    if !m.is_dir()
        || m.file_type().is_symlink()
        || m.uid() != unsafe { libc::geteuid() }
        || m.permissions().mode() & 0o077 != 0
    {
        return Err(DraftError::InsecureDirectory);
    }
    File::open(path.parent().ok_or(DraftError::InsecureDirectory)?)
        .and_then(|f| f.sync_all())
        .map_err(DraftError::Io)
}
fn validate_file(file: &File) -> Result<(), DraftError> {
    let m = file.metadata().map_err(DraftError::Io)?;
    if !m.is_file() || m.uid() != unsafe { libc::geteuid() } || m.permissions().mode() & 0o077 != 0
    {
        return Err(DraftError::InsecureFile);
    }
    Ok(())
}
fn read_raw(path: &Path) -> Result<Vec<u8>, DraftError> {
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| DraftError::InsecureFile)?;
    validate_file(&file)?;
    if file.metadata().map_err(DraftError::Io)?.len() > MAXIMUM_DRAFT {
        return Err(DraftError::TooLarge(MAXIMUM_DRAFT + 1));
    }
    let mut bytes = Vec::new();
    file.take(MAXIMUM_DRAFT + 1)
        .read_to_end(&mut bytes)
        .map_err(DraftError::Io)?;
    if bytes.len() as u64 > MAXIMUM_DRAFT {
        return Err(DraftError::TooLarge(bytes.len() as u64));
    }
    Ok(bytes)
}
fn atomic_draft(path: &Path, bytes: &[u8], expected: Option<&[u8]>) -> Result<(), DraftError> {
    let temp = path.with_extension(format!("{}.tmp", Uuid::new_v4()));
    let result = (|| {
        use std::io::Write;
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)
            .map_err(DraftError::Io)?;
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(DraftError::Io)?;
        let unchanged = match (expected, fs::symlink_metadata(path)) {
            (Some(expected), Ok(_)) => read_raw(path).is_ok_and(|bytes| bytes == expected),
            (None, Err(e)) if e.kind() == std::io::ErrorKind::NotFound => true,
            _ => false,
        };
        if !unchanged {
            return Err(DraftError::DraftIdConflict);
        }
        fs::rename(&temp, path).map_err(DraftError::Io)?;
        File::open(path.parent().ok_or(DraftError::InsecureDirectory)?)
            .and_then(|f| f.sync_all())
            .map_err(DraftError::Io)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
fn validated_state(source: &[u8]) -> Result<PersistentState, DraftError> {
    if source.len() > 16 * 1024 * 1024 {
        return Err(DraftError::TooLarge(source.len() as u64));
    }
    let state: PersistentState =
        thumble_protocol::decode_unique_json(source).map_err(|_| DraftError::Malformed)?;
    state.validate().map_err(|_| DraftError::Malformed)?;
    if state.schema_version != OWNED_SCHEMA_VERSION {
        return Err(DraftError::UnsupportedVersion);
    }
    Ok(state)
}
fn source_key<'a>(
    source: &BTreeMap<String, &'a RawValue>,
    name: &str,
) -> Option<(String, &'a RawValue)> {
    let aliases: &[&str] = match name {
        "activeProfileID" => &[
            "activeProfileID",
            "activeProfileId",
            "activeGamepadProfileID",
        ],
        "defaultProfileID" => &[
            "defaultProfileID",
            "defaultProfileId",
            "defaultGamepadProfileID",
        ],
        _ => &[],
    };
    if aliases.is_empty() {
        source.get(name).map(|v| (name.to_owned(), *v))
    } else {
        aliases
            .iter()
            .find_map(|key| source.get(*key).map(|v| ((*key).to_owned(), *v)))
    }
}
pub fn authority_document(source: &[u8]) -> Result<OwnedConfigurationDocument, DraftError> {
    let state = validated_state(source)?;
    let raw: BTreeMap<String, &RawValue> =
        serde_json::from_slice(source).map_err(|_| DraftError::Malformed)?;
    let mut output = String::from(
        "{\"schema\":\"com.codybontecou.pocketpad.keypad-configuration\",\"version\":5",
    );
    for name in FIELDS {
        if let Some((_, value)) = source_key(&raw, name) {
            output.push_str(&format!(",\"{name}\":{}", value.get()));
        }
    }
    if let Some(metadata) = raw.get("ownedConfigurationExtensions") {
        let extensions: BTreeMap<String, &RawValue> =
            serde_json::from_str(metadata.get()).map_err(|_| DraftError::Malformed)?;
        for (key, value) in extensions {
            if FIELDS.contains(&key.as_str())
                || ["schema", "version", "exportedAt"].contains(&key.as_str())
            {
                return Err(DraftError::Malformed);
            }
            output.push(',');
            output.push_str(&serde_json::to_string(&key).map_err(|_| DraftError::EncodingFailed)?);
            output.push(':');
            output.push_str(value.get());
        }
    }
    output.push('}');
    let document =
        OwnedConfigurationDocument::decode(output.as_bytes()).map_err(|_| DraftError::Malformed)?;
    check_credentials(&document, &state)?;
    Ok(document)
}
fn check_credentials(
    document: &OwnedConfigurationDocument,
    state: &PersistentState,
) -> Result<(), DraftError> {
    let value: Value = thumble_protocol::decode_unique_json(document.original_bytes())
        .map_err(|_| DraftError::Malformed)?;
    let encoded = serde_json::to_string(&value).map_err(|_| DraftError::EncodingFailed)?;
    if state
        .trusted_clients
        .keys()
        .any(|token| !token.is_empty() && encoded.contains(token))
    {
        return Err(DraftError::Malformed);
    }
    Ok(())
}
fn rename(
    source: &OwnedConfigurationDocument,
    id: &str,
    name: &str,
    now: i64,
) -> Result<(OwnedConfigurationDocument, ConfigurationOperationOutcome), DraftError> {
    let id = Uuid::parse_str(id).map_err(|_| DraftError::Malformed)?;
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 256 || now < 0 {
        return Err(DraftError::Malformed);
    }
    let index = source
        .profiles()
        .iter()
        .position(|p| p["id"].as_str().and_then(|id| Uuid::parse_str(id).ok()) == Some(id))
        .ok_or(DraftError::NotFound)?;
    if source.profiles()[index]["name"].as_str() == Some(name) {
        return Ok((
            source.clone(),
            ConfigurationOperationOutcome {
                changed: false,
                changed_paths: vec![],
            },
        ));
    }
    let encoded = serde_json::to_vec(name).map_err(|_| DraftError::EncodingFailed)?;
    let updated = now.to_string();
    let paths = [
        format!("/profiles/{index}/name"),
        format!("/profiles/{index}/updatedAt"),
    ];
    let document = source
        .with_edits(&[
            JsonValueEdit {
                pointer: &paths[0],
                value: Some(&encoded),
            },
            JsonValueEdit {
                pointer: &paths[1],
                value: Some(updated.as_bytes()),
            },
        ])
        .map_err(|_| DraftError::Malformed)?;
    Ok((
        document,
        ConfigurationOperationOutcome {
            changed: true,
            changed_paths: vec![format!("/profiles/{id}/name")],
        },
    ))
}
/// Raw application for preview/commit. No revision/history change here.
pub fn apply_document(
    source: &[u8],
    document: &OwnedConfigurationDocument,
) -> Result<Vec<u8>, DraftError> {
    let state = validated_state(source)?;
    check_credentials(document, &state)?;
    if document.default_profile_id().is_none() {
        return Err(DraftError::Malformed);
    }
    let raw: BTreeMap<String, &RawValue> =
        serde_json::from_slice(source).map_err(|_| DraftError::Malformed)?;
    let desired: BTreeMap<String, &RawValue> =
        serde_json::from_slice(document.original_bytes()).map_err(|_| DraftError::Malformed)?;
    let mut changes = Vec::new();
    for key in FIELDS {
        let original = source_key(&raw, key);
        let target = desired.get(*key).copied();
        if original.as_ref().map(|(_, v)| v.get()) == target.map(RawValue::get) {
            continue;
        }
        changes.push((
            format!("/{}", original.as_ref().map_or(*key, |(key, _)| key)),
            target.map(|v| v.get().as_bytes()),
        ));
    }
    let extensions: BTreeMap<_, _> = desired
        .iter()
        .filter(|(key, _)| {
            !FIELDS.contains(&key.as_str())
                && !["schema", "version", "exportedAt"].contains(&key.as_str())
        })
        .collect();
    let metadata = serde_json::to_vec(&extensions).map_err(|_| DraftError::EncodingFailed)?;
    if extensions.is_empty() {
        if raw.contains_key("ownedConfigurationExtensions") {
            changes.push(("/ownedConfigurationExtensions".into(), None));
        }
    } else if raw
        .get("ownedConfigurationExtensions")
        .map(|v| v.get().as_bytes())
        != Some(metadata.as_slice())
    {
        changes.push((
            "/ownedConfigurationExtensions".into(),
            Some(metadata.as_slice()),
        ));
    }
    let edits: Vec<_> = changes
        .iter()
        .map(|(pointer, value)| JsonValueEdit {
            pointer,
            value: *value,
        })
        .collect();
    let result =
        edit_json_values(source, &edits, 16 * 1024 * 1024).map_err(|_| DraftError::Malformed)?;
    validated_state(&result)?;
    Ok(result)
}
pub struct OwnedCommitRequest<'a> {
    pub draft_id: &'a str,
    pub draft_revision: u64,
    pub expected: u64,
    pub commit_id: &'a str,
    pub request_digest: Option<&'a str>,
}
pub struct PreparedOwnedCommit {
    pub candidate: Option<Vec<u8>>,
    pub summary: ConfigurationSaveSummary,
}
impl OwnedDraftStore {
    pub fn prepare_commit(
        &self,
        source: &[u8],
        request: OwnedCommitRequest<'_>,
        now: i64,
    ) -> Result<PreparedOwnedCommit, DraftError> {
        let OwnedCommitRequest {
            draft_id,
            draft_revision,
            expected,
            commit_id,
            request_digest,
        } = request;
        let state = validated_state(source)?;
        let draft_id = canonical(draft_id)?;
        let commit_id = canonical(commit_id)?;
        if now < 0 || request_digest.is_some_and(|digest| !valid_hash(digest)) {
            return Err(DraftError::Malformed);
        }
        if let Some(record) = state.recent_configuration_commit(&commit_id) {
            if record.draft_id != draft_id
                || record.base_configuration_revision != expected
                || record.draft_revision != draft_revision
                || record.client_request_digest.as_deref() != request_digest
            {
                return Err(DraftError::OperationIdConflict);
            }
            return Ok(PreparedOwnedCommit {
                candidate: None,
                summary: ConfigurationSaveSummary {
                    draft_id,
                    commit_id,
                    base_configuration_revision: expected,
                    configuration_revision: record.result_configuration_revision,
                    draft_revision,
                    changed: record.result_configuration_revision != expected,
                    idempotent_replay: true,
                    phone_sync_queued: false,
                },
            });
        }
        if state.configuration_revision != expected {
            return Err(DraftError::ConfigurationRevisionConflict {
                expected,
                actual: state.configuration_revision,
            });
        }
        let draft = self.get(&draft_id, Some(draft_revision), now)?;
        if draft.authority_hash != hash(state.server_id.as_bytes())
            || draft.base_configuration_revision != expected
            || authority_document(source)?.original_bytes()
                != draft.base_document.0.original_bytes()
        {
            return Err(DraftError::ConfigurationRevisionConflict {
                expected,
                actual: state.configuration_revision,
            });
        }
        let changed = draft.document().original_bytes() != draft.base_document.0.original_bytes();
        let revision = if changed {
            expected
                .checked_add(1)
                .ok_or(DraftError::DraftRevisionExhausted)?
        } else {
            expected
        };
        let mut result = if changed {
            apply_document(source, draft.document())?
        } else {
            source.to_vec()
        };
        let raw: BTreeMap<String, &RawValue> =
            serde_json::from_slice(source).map_err(|_| DraftError::Malformed)?;
        let prior: Vec<&RawValue> = raw
            .get("recentConfigurationCommits")
            .map(|value| serde_json::from_str(value.get()))
            .transpose()
            .map_err(|_| DraftError::Malformed)?
            .unwrap_or_default();
        let record = ConfigurationCommitRecord {
            commit_id: commit_id.clone(),
            draft_id: draft_id.clone(),
            base_configuration_revision: expected,
            result_configuration_revision: revision,
            draft_revision,
            draft_digest: hash(draft.document().original_bytes()),
            client_request_digest: request_digest.map(str::to_owned),
            committed_at: now,
        };
        let mut records = String::from("[");
        for row in prior.iter().skip(prior.len().saturating_sub(31)) {
            if records.len() > 1 {
                records.push(',');
            }
            records.push_str(row.get());
        }
        if records.len() > 1 {
            records.push(',');
        }
        records.push_str(&serde_json::to_string(&record).map_err(|_| DraftError::EncodingFailed)?);
        records.push(']');
        let revision_text = revision.to_string();
        let updated = now.max(state.configuration_updated_at).to_string();
        let mut edits = vec![JsonValueEdit {
            pointer: "/recentConfigurationCommits",
            value: Some(records.as_bytes()),
        }];
        if changed {
            edits.push(JsonValueEdit {
                pointer: "/configurationRevision",
                value: Some(revision_text.as_bytes()),
            });
            edits.push(JsonValueEdit {
                pointer: "/configurationUpdatedAt",
                value: Some(updated.as_bytes()),
            });
        }
        result = edit_json_values(&result, &edits, 16 * 1024 * 1024)
            .map_err(|_| DraftError::Malformed)?;
        validated_state(&result)?;
        Ok(PreparedOwnedCommit {
            candidate: Some(result),
            summary: ConfigurationSaveSummary {
                draft_id,
                commit_id,
                base_configuration_revision: expected,
                configuration_revision: revision,
                draft_revision,
                changed,
                idempotent_replay: false,
                phone_sync_queued: false,
            },
        })
    }
}

impl OwnedDraftStore {
    pub fn rebase(
        &self,
        source: &[u8],
        id: &str,
        revision: u64,
        expected: u64,
        rebase_id: &str,
        now: i64,
    ) -> Result<OwnedDraftEdit, DraftError> {
        let state = validated_state(source)?;
        if state.configuration_revision != expected {
            return Err(DraftError::ConfigurationRevisionConflict {
                expected,
                actual: state.configuration_revision,
            });
        }
        let current = authority_document(source)?;
        let id = canonical(id)?;
        let rebase_id = canonical(rebase_id)?;
        let descriptor = DraftStore::operation_digest(
            &serde_json::json!({"type":"owned.rebase","configurationRevision":expected,"sourceSHA256":hash(current.original_bytes())}),
        )?;
        let _lock = self.lock()?;
        let mut draft = self.read(&id)?;
        validate(&draft, &id, now)?;
        if draft.authority_hash != hash(state.server_id.as_bytes()) {
            return Err(DraftError::DraftIdConflict);
        }
        if let Some(record) = draft
            .operation_log
            .iter()
            .find(|record| record.operation_id == rebase_id)
        {
            if record.operation_digest != descriptor {
                return Err(DraftError::OperationIdConflict);
            }
            if record.base_draft_revision != revision {
                return Err(DraftError::DraftRevisionConflict {
                    expected: revision,
                    actual: record.base_draft_revision,
                });
            }
            let outcome = ConfigurationOperationOutcome {
                changed: record.changed,
                changed_paths: record.changed_paths.clone(),
            };
            return Ok(OwnedDraftEdit {
                draft,
                outcome,
                idempotent_replay: true,
            });
        }
        check_revision(&draft, Some(revision))?;
        if draft.operation_log.len() >= MAXIMUM_CONFIGURATION_DRAFT_OPERATIONS {
            return Err(DraftError::TooManyOperations);
        }
        let merged =
            crate::owned_rebase::merge(&draft.base_document.0, draft.document(), &current)?;
        let changed = merged.original_bytes() != draft.document().original_bytes()
            || draft.base_configuration_revision != expected;
        let next = if changed {
            revision
                .checked_add(1)
                .ok_or(DraftError::DraftRevisionExhausted)?
        } else {
            revision
        };
        let outcome = ConfigurationOperationOutcome {
            changed,
            changed_paths: if changed {
                vec!["/configuration".into()]
            } else {
                vec![]
            },
        };
        draft.base_document = Document(current);
        draft.base_configuration_revision = expected;
        draft.working_document = Document(merged);
        draft.draft_revision = next;
        draft.updated_at = now.max(draft.updated_at);
        draft.operation_log.push(DraftOperationRecord {
            operation_id: rebase_id,
            operation_digest: descriptor,
            base_draft_revision: revision,
            result_draft_revision: next,
            changed,
            changed_paths: outcome.changed_paths.clone(),
            applied_at: now,
        });
        self.save(&draft)?;
        Ok(OwnedDraftEdit {
            draft,
            outcome,
            idempotent_replay: false,
        })
    }
    pub fn commit(
        &self,
        lease: &crate::runtime::AuthorityLock,
        draft_id: &str,
        draft_revision: u64,
        expected: u64,
        commit_id: &str,
        now: i64,
    ) -> Result<ConfigurationSaveSummary, DraftError> {
        if !lease.protects(&self.paths) {
            return Err(DraftError::InsecureDirectory);
        }
        let source = storage::read_state_bytes(&self.paths.state_file)
            .map_err(|_| DraftError::InsecureFile)?;
        let prepared = self.prepare_commit(
            &source,
            OwnedCommitRequest {
                draft_id,
                draft_revision,
                expected,
                commit_id,
                request_digest: None,
            },
            now,
        )?;
        if let Some(candidate) = prepared.candidate {
            storage::install_private_state_bytes(
                &self.paths.state_file,
                &candidate,
                Some(&source),
                || Ok(()),
            )
            .map_err(|_| DraftError::DraftIdConflict)?;
            let _ = self.discard(draft_id, draft_revision, now);
        }
        Ok(prepared.summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const PRECISE: &str =
        "{\"integer\":9007199254740993,\"decimal\":0.12345678901234567890123456789}";
    fn fixture() -> (tempfile::TempDir, HostPaths, Vec<u8>) {
        let temp = tempfile::tempdir().unwrap();
        let paths = HostPaths::new(temp.path().join("authority"), temp.path().join("control"));
        paths.ensure_state_dir().unwrap();
        let state = PersistentState::minimal("owned-draft-test-authority").unwrap();
        let encoded = serde_json::to_vec(&state).unwrap();
        let document: BTreeMap<String, &RawValue> = serde_json::from_slice(include_bytes!(
            "../../../fixtures/owned-configuration/v1/document.json"
        ))
        .unwrap();
        let mut owned = edit_json_values(
            &encoded,
            &[
                JsonValueEdit {
                    pointer: "/schemaVersion",
                    value: Some(b"4"),
                },
                JsonValueEdit {
                    pointer: "/configurationRevision",
                    value: Some(b"41"),
                },
                JsonValueEdit {
                    pointer: "/futureState",
                    value: Some(PRECISE.as_bytes()),
                },
            ],
            16 * 1024 * 1024,
        )
        .unwrap();
        for field in FIELDS {
            if let Some(raw) = document.get(*field) {
                owned = edit_json_values(
                    &owned,
                    &[JsonValueEdit {
                        pointer: &format!("/{field}"),
                        value: Some(raw.get().as_bytes()),
                    }],
                    16 * 1024 * 1024,
                )
                .unwrap();
            }
        }
        validated_state(&owned).unwrap();
        (temp, paths, owned)
    }
    fn id() -> String {
        Uuid::new_v4().to_string()
    }
    fn operation(document: &OwnedConfigurationDocument, name: &str) -> ConfigurationOperation {
        ConfigurationOperation::ProfileRename {
            profile_id: document.profiles()[0]["id"].as_str().unwrap().into(),
            name: name.into(),
        }
    }
    #[test]
    fn restart_preserves_raw_numbers_and_replays_without_reapplying() {
        let (_temp, paths, source) = fixture();
        let store = OwnedDraftStore::new(&paths);
        let draft_id = id();
        let mut draft = store.begin(&source, 41, &draft_id, 100).unwrap();
        let with_opaque = draft
            .document()
            .with_edits(&[JsonValueEdit {
                pointer: "/profiles/0/futureDesign",
                value: Some(PRECISE.as_bytes()),
            }])
            .unwrap();
        let op = operation(draft.document(), "Renamed");
        let operation_id = id();
        draft = store
            .edit_with(&draft_id, 1, &operation_id, &op, 101, |_| {
                Ok((
                    with_opaque,
                    ConfigurationOperationOutcome {
                        changed: true,
                        changed_paths: vec!["/profiles/futureDesign".into()],
                    },
                ))
            })
            .unwrap()
            .draft;
        let bytes = draft.document().original_bytes().to_vec();
        let restarted = OwnedDraftStore::new(&paths);
        assert_eq!(
            restarted
                .get(&draft_id, Some(2), 102)
                .unwrap()
                .document()
                .original_bytes(),
            bytes
        );
        assert!(std::str::from_utf8(&bytes).unwrap().contains(PRECISE));
        let replay = restarted
            .edit_with(&draft_id, 1, &operation_id, &op, 103, |_| {
                panic!("replay called provider")
            })
            .unwrap();
        assert!(replay.idempotent_replay);
        assert!(matches!(
            restarted.edit(
                &draft_id,
                2,
                &operation_id,
                &operation(draft.document(), "Other"),
                104
            ),
            Err(DraftError::OperationIdConflict)
        ));
        assert_eq!(
            fs::metadata(store.path(&draft_id))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    #[test]
    fn commit_preserves_authority_metadata_and_replays_after_draft_cleanup() {
        let (_temp, paths, source) = fixture();
        let lease = crate::runtime::AuthorityLock::try_acquire(&paths)
            .unwrap()
            .unwrap();
        let store = OwnedDraftStore::new(&paths);
        let draft_id = id();
        let draft = store.begin(&source, 41, &draft_id, 100).unwrap();
        store
            .edit(
                &draft_id,
                1,
                &id(),
                &operation(draft.document(), "Updated"),
                101,
            )
            .unwrap();
        // Non-configuration metadata can change while the draft remains valid.
        let current = edit_json_values(
            &source,
            &[JsonValueEdit {
                pointer: "/futureAuthority",
                value: Some(PRECISE.as_bytes()),
            }, JsonValueEdit {
                pointer: "/trustedClients",
                value: Some(br#"{"private-pairing-token":{"name":"Phone","createdAt":100,"lastSeenAt":101,"futureTrust":9007199254740993}}"#),
            }],
            16 * 1024 * 1024,
        )
        .unwrap();
        let exported = authority_document(&current).unwrap();
        assert!(!std::str::from_utf8(exported.original_bytes())
            .unwrap()
            .contains("private-pairing-token"));
        storage::install_private_state_bytes(&paths.state_file, &current, None, || Ok(())).unwrap();
        let commit_id = id();
        let summary = store
            .commit(&lease, &draft_id, 2, 41, &commit_id, 102)
            .unwrap();
        assert_eq!(summary.configuration_revision, 42);
        let committed = storage::read_state_bytes(&paths.state_file).unwrap();
        let raw: BTreeMap<String, &RawValue> = serde_json::from_slice(&committed).unwrap();
        assert_eq!(raw["futureAuthority"].get(), PRECISE);
        assert_eq!(raw["futureState"].get(), PRECISE);
        assert!(raw["trustedClients"].get().contains("9007199254740993"));
        assert!(raw["trustedClients"]
            .get()
            .contains("private-pairing-token"));
        assert!(!std::str::from_utf8(draft.document().original_bytes())
            .unwrap()
            .contains("private-pairing-token"));
        assert_eq!(
            validated_state(&committed).unwrap().profiles[0]["name"],
            "Updated"
        );
        assert!(!store.path(&draft_id).exists());
        assert!(
            store
                .commit(&lease, &draft_id, 2, 41, &commit_id, 103)
                .unwrap()
                .idempotent_replay
        );
        assert_eq!(
            storage::read_state_bytes(&paths.state_file).unwrap(),
            committed
        );
    }
    #[test]
    fn conflicting_rebase_keeps_draft_and_authority_unchanged() {
        let (_temp, paths, source) = fixture();
        let store = OwnedDraftStore::new(&paths);
        let draft_id = id();
        let draft = store.begin(&source, 41, &draft_id, 100).unwrap();
        store
            .edit(
                &draft_id,
                1,
                &id(),
                &operation(draft.document(), "Local"),
                101,
            )
            .unwrap();
        let before = fs::read(store.path(&draft_id)).unwrap();
        let current = edit_json_values(
            &source,
            &[
                JsonValueEdit {
                    pointer: "/profiles/0/name",
                    value: Some(b"\"Remote\""),
                },
                JsonValueEdit {
                    pointer: "/configurationRevision",
                    value: Some(b"42"),
                },
            ],
            16 * 1024 * 1024,
        )
        .unwrap();
        assert!(matches!(
            store.rebase(&current, &draft_id, 2, 42, &id(), 102),
            Err(DraftError::MergeConflict(_))
        ));
        assert_eq!(fs::read(store.path(&draft_id)).unwrap(), before);
        assert!(matches!(
            store.prepare_commit(
                &current,
                OwnedCommitRequest {
                    draft_id: &draft_id,
                    draft_revision: 2,
                    expected: 41,
                    commit_id: &id(),
                    request_digest: None
                },
                103
            ),
            Err(DraftError::ConfigurationRevisionConflict { .. })
        ));
    }
    #[test]
    fn disjoint_rebase_preserves_raw_remote_metadata_and_local_name() {
        let (_temp, paths, source) = fixture();
        let store = OwnedDraftStore::new(&paths);
        let draft_id = id();
        let draft = store.begin(&source, 41, &draft_id, 100).unwrap();
        store
            .edit(
                &draft_id,
                1,
                &id(),
                &operation(draft.document(), "Local"),
                101,
            )
            .unwrap();
        let current = edit_json_values(
            &source,
            &[
                JsonValueEdit {
                    pointer: "/profiles/0/futureDesign",
                    value: Some(PRECISE.as_bytes()),
                },
                JsonValueEdit {
                    pointer: "/configurationRevision",
                    value: Some(b"42"),
                },
            ],
            16 * 1024 * 1024,
        )
        .unwrap();
        let rebase_id = id();
        let rebased = store
            .rebase(&current, &draft_id, 2, 42, &rebase_id, 102)
            .unwrap();
        assert_eq!(rebased.draft.document().profiles()[0]["name"], "Local");
        assert!(
            std::str::from_utf8(rebased.draft.document().original_bytes())
                .unwrap()
                .contains(PRECISE)
        );
        assert_eq!(rebased.draft.draft_revision, 3);
        assert!(
            store
                .rebase(&current, &draft_id, 2, 42, &rebase_id, 103)
                .unwrap()
                .idempotent_replay
        );
        assert!(store
            .prepare_commit(
                &current,
                OwnedCommitRequest {
                    draft_id: &draft_id,
                    draft_revision: 3,
                    expected: 42,
                    commit_id: &id(),
                    request_digest: None
                },
                104
            )
            .unwrap()
            .candidate
            .is_some());
    }
    #[test]
    fn simultaneous_revisions_allow_only_one_edit() {
        let (_temp, paths, source) = fixture();
        let store = OwnedDraftStore::new(&paths);
        let draft_id = id();
        let draft = store.begin(&source, 41, &draft_id, 100).unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let handles: Vec<_> = ["One", "Two"]
            .into_iter()
            .map(|name| {
                let paths = paths.clone();
                let draft_id = draft_id.clone();
                let operation = operation(draft.document(), name);
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    OwnedDraftStore::new(&paths).edit(&draft_id, 1, &id(), &operation, 101)
                })
            })
            .collect();
        let mut successes = 0;
        let mut conflicts = 0;
        for handle in handles {
            match handle.join().unwrap() {
                Ok(_) => successes += 1,
                Err(DraftError::DraftRevisionConflict { .. }) => conflicts += 1,
                Err(error) => panic!("unexpected edit error: {error}"),
            }
        }
        assert_eq!((successes, conflicts), (1, 1));
        assert_eq!(
            store
                .get(&draft_id, Some(2), 102)
                .unwrap()
                .operation_log
                .len(),
            1
        );
    }
    #[test]
    fn no_op_replay_and_expiration_are_durable() {
        let (_temp, paths, source) = fixture();
        let store = OwnedDraftStore::new(&paths);
        let draft_id = id();
        let draft = store.begin(&source, 41, &draft_id, 100).unwrap();
        let name = draft.document().profiles()[0]["name"].as_str().unwrap();
        let operation = operation(draft.document(), name);
        let operation_id = id();
        let edited = store
            .edit(&draft_id, 1, &operation_id, &operation, 101)
            .unwrap();
        assert!(!edited.outcome.changed);
        assert_eq!(edited.draft.draft_revision, 1);
        let restarted = OwnedDraftStore::new(&paths);
        assert!(
            restarted
                .edit(&draft_id, 1, &operation_id, &operation, 102)
                .unwrap()
                .idempotent_replay
        );
        assert!(matches!(
            restarted.get(&draft_id, None, 100 + CONFIGURATION_DRAFT_LIFETIME_MILLIS),
            Err(DraftError::Expired)
        ));
        assert!(store.path(&draft_id).exists());
    }
    #[test]
    fn unsafe_files_and_invalid_outcomes_cannot_replace_a_draft() {
        let (_temp, paths, source) = fixture();
        let store = OwnedDraftStore::new(&paths);
        let draft_id = id();
        let draft = store.begin(&source, 41, &draft_id, 100).unwrap();
        let before = fs::read(store.path(&draft_id)).unwrap();
        let operation = operation(draft.document(), "Changed");
        let result = store.edit_with(&draft_id, 1, &id(), &operation, 101, |document| {
            Ok((
                document.clone(),
                ConfigurationOperationOutcome {
                    changed: true,
                    changed_paths: vec!["/profiles".into()],
                },
            ))
        });
        assert!(matches!(result, Err(DraftError::InvalidOperationOutcome)));
        assert_eq!(fs::read(store.path(&draft_id)).unwrap(), before);
        fs::set_permissions(store.path(&draft_id), fs::Permissions::from_mode(0o644)).unwrap();
        assert!(matches!(
            store.get(&draft_id, None, 102),
            Err(DraftError::InsecureFile)
        ));
        fs::remove_file(store.path(&draft_id)).unwrap();
        let outside = paths.state_dir.join("outside");
        fs::write(&outside, &before).unwrap();
        std::os::unix::fs::symlink(&outside, store.path(&draft_id)).unwrap();
        assert!(store.get(&draft_id, None, 103).is_err());
        assert_eq!(fs::read(outside).unwrap(), before);
    }
}
