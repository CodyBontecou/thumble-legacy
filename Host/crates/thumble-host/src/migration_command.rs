//! Offline, exact-plan activation of the backed-up authority migration.
//! A running receiver owns the lease, so this path can never migrate beneath it.
use crate::{
    bridge::ConfigurationBridge,
    legacy_import::read_file,
    owned_migration::{self, MigrationReceipt, PreparedMigrationProfile},
    paths::HostPaths,
    runtime::AuthorityLock,
    storage,
};
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};
use thumble_core::{LegacyMigrationError, CURRENT_SCHEMA_VERSION};
use uuid::Uuid;

#[derive(Debug, Args)]
pub struct MigrationArgs {
    #[command(subcommand)]
    pub action: MigrationAction,
}
#[derive(Debug, Subcommand)]
pub enum MigrationAction {
    /// Inspect a stopped receiver and prepare a private-data-free migration review.
    Plan {
        #[arg(long)]
        package: Vec<String>,
    },
    /// Restore the original legacy authority only if no later state was written.
    Rollback {
        #[arg(long)]
        plan_file: PathBuf,
        #[arg(long, required = true)]
        allow_config_write: bool,
    },
    /// Apply the exact reviewed plan, saving original state and assets first.
    Apply {
        #[arg(long)]
        plan_file: PathBuf,
        #[arg(long)]
        package: Vec<String>,
        #[arg(long, required = true)]
        allow_config_write: bool,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MigrationPlan {
    pub schema_version: u32,
    pub prepared_at: i64,
    pub receipt: MigrationReceipt,
}
fn mappings(items: &[String]) -> Result<BTreeMap<Uuid, PathBuf>, String> {
    if items.len() > 256 {
        return Err("migration_package_limit: at most 256 mappings".into());
    }
    let mut result = BTreeMap::new();
    for item in items {
        let (id, path) = item
            .split_once('=')
            .ok_or("package must be PROFILE_UUID=/absolute/archive.path")?;
        let id = id
            .parse()
            .map_err(|_| "package profile ID must be a UUID")?;
        let path = PathBuf::from(path);
        if !path.is_absolute() || result.insert(id, path).is_some() {
            return Err(
                "migration_package_mapping: paths must be absolute and profile IDs unique".into(),
            );
        }
    }
    Ok(result)
}
fn candidate(
    source: &[u8],
    packages: &BTreeMap<Uuid, PathBuf>,
    now: i64,
) -> Result<owned_migration::OwnedStateMigration, String> {
    let state: thumble_core::PersistentState =
        thumble_protocol::decode_unique_json(source).map_err(|_| "migration_source_invalid")?;
    state.validate().map_err(|_| "migration_source_invalid")?;
    // Require exact mappings before invoking the native helper; never guess an
    // archive by its filename or a currently installed library entry.
    let required: std::collections::BTreeSet<Uuid> = state
        .profiles
        .iter()
        .filter(|p| p.get("skinReference").is_some_and(|v| !v.is_null()))
        .map(|p| {
            p["id"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .ok_or("migration_profile_invalid")
        })
        .collect::<Result<_, _>>()?;
    if required.iter().ne(packages.keys()) {
        return Err("migration_packages_required: supply exactly the original archives for referenced profiles".into());
    }
    let bridge =
        ConfigurationBridge::discover().map_err(|_| "migration_native_helper_unavailable")?;
    owned_migration::prepare(source, now, |profile| {
        let value: serde_json::Value = thumble_protocol::decode_unique_json(profile)
            .map_err(|_| LegacyMigrationError::InvalidSource)?;
        let id: Uuid = value["id"]
            .as_str()
            .and_then(|v| v.parse().ok())
            .ok_or(LegacyMigrationError::InvalidSource)?;
        let original_package = packages
            .get(&id)
            .map(|path| read_file(path, 40 * 1024 * 1024))
            .transpose()
            .map_err(|_| LegacyMigrationError::PreparationFailed)?;
        // Materialize all eight appearance states. The raw local merger keeps
        // launch associations; no portable artifact/export is used as authority.
        let preparation = bridge.prepare_portable_legacy_profile(
            profile,
            original_package.as_deref(),
            Uuid::new_v4(),
        )?;
        Ok(PreparedMigrationProfile {
            preparation,
            original_package,
        })
    })
    .map_err(|error| error.to_string())
}
fn lease(paths: &HostPaths) -> Result<AuthorityLock, String> {
    // Do not initialize a fresh catalog as a side effect of migration planning.
    storage::read_state_bytes(&paths.state_file)?;
    AuthorityLock::try_acquire(paths)?.ok_or_else(|| {
        "migration_receiver_running: stop this receiver before planning or applying migration"
            .into()
    })
}
pub fn plan(
    paths: &HostPaths,
    packages: &BTreeMap<Uuid, PathBuf>,
    now: i64,
) -> Result<MigrationPlan, String> {
    let _lease = lease(paths)?;
    let source = storage::read_state_bytes(&paths.state_file)?;
    let candidate = candidate(&source, packages, now)?;
    // This additionally checks exact owned document validity and credential
    // isolation, including any pre-existing configuration extension fields.
    crate::owned_drafts::authority_document(candidate.destination_bytes())
        .map_err(|_| "migration_owned_document_invalid")?;
    Ok(MigrationPlan {
        schema_version: 1,
        prepared_at: now,
        receipt: candidate.receipt().clone(),
    })
}
pub fn apply(
    paths: &HostPaths,
    packages: &BTreeMap<Uuid, PathBuf>,
    plan: &MigrationPlan,
) -> Result<MigrationReceipt, String> {
    if plan.schema_version != 1
        || plan.prepared_at < 0
        || plan.receipt.phase != owned_migration::MigrationPhase::Prepared
    {
        return Err("migration_plan_invalid".into());
    }
    let lease = lease(paths)?;
    let source = storage::read_state_bytes(&paths.state_file)?;
    use sha2::{Digest, Sha256};
    let hash = format!("{:x}", Sha256::digest(&source));
    if hash != plan.receipt.source_sha256 {
        // Exact replays can finish an interrupted installation, but never replay
        // over a subsequent edit or a different migration destination.
        if hash == plan.receipt.destination_sha256 {
            return owned_migration::recover_reviewed(paths, &lease, &plan.receipt)
                .map_err(|e| e.to_string());
        }
        return Err(
            "migration_source_changed: prepare a new plan; original data is unchanged".into(),
        );
    }
    let state: thumble_core::PersistentState =
        thumble_protocol::decode_unique_json(&source).map_err(|_| "migration_source_invalid")?;
    if state.schema_version != CURRENT_SCHEMA_VERSION
        || state.configuration_revision != plan.receipt.base_revision
    {
        return Err("migration_source_changed".into());
    }
    let candidate = candidate(&source, packages, plan.prepared_at)?;
    crate::owned_drafts::authority_document(candidate.destination_bytes())
        .map_err(|_| "migration_owned_document_invalid")?;
    if candidate.receipt() != &plan.receipt {
        return Err(
            "migration_plan_changed: native conversion or package bytes differ; prepare a new plan"
                .into(),
        );
    }
    owned_migration::commit(paths, &lease, &candidate).map_err(|e| e.to_string())
}
/// Deliberate recovery for a phone that retained legacy work during cutover.
/// Exact state checks prohibit rolling back over any newer receiver write.
pub fn rollback(paths: &HostPaths, plan: &MigrationPlan) -> Result<MigrationReceipt, String> {
    if plan.schema_version != 1
        || plan.prepared_at < 0
        || plan.receipt.phase != owned_migration::MigrationPhase::Prepared
    {
        return Err("migration_plan_invalid".into());
    }
    let lease = lease(paths)?;
    let mut resources: Option<(crate::skins::SkinBridge, crate::skins::SkinLibrary)> = None;
    owned_migration::rollback_reviewed(paths, &lease, &plan.receipt, |expected_hash, bytes| {
        use owned_migration::MigrationError;
        if resources.is_none() {
            let inspector =
                crate::skins::SkinBridge::discover().map_err(|_| MigrationError::Preparation)?;
            let library = crate::skins::SkinLibrary::open(
                paths,
                crate::skins::SkinBridge::discover().map_err(|_| MigrationError::Preparation)?,
            )
            .map_err(|_| MigrationError::Storage)?;
            resources = Some((inspector, library));
        }
        let (inspector, library) = resources.as_ref().ok_or(MigrationError::Preparation)?;
        let package = inspector
            .inspect(bytes.to_vec())
            .map_err(|_| MigrationError::Preparation)?;
        if package.info().archive_sha256 != expected_hash {
            return Err(MigrationError::InvalidJournal);
        }
        library.restore_exact(&package).map_err(|error| {
            if error.0 == "skin_restore_conflict" {
                MigrationError::ResourceConflict
            } else {
                MigrationError::Storage
            }
        })?;
        Ok(())
    })
    .map_err(|e| e.to_string())
}
pub fn run(paths: &HostPaths, args: MigrationArgs) -> Result<(), String> {
    let value = match args.action {
        MigrationAction::Plan { package } => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| "system clock unavailable")?
                .as_millis()
                .try_into()
                .map_err(|_| "system clock exceeds bound")?;
            serde_json::to_value(plan(paths, &mappings(&package)?, now)?)
                .map_err(|_| "migration plan encoding failed")?
        }
        MigrationAction::Rollback {
            plan_file,
            allow_config_write,
        } => {
            if !allow_config_write {
                return Err("migration rollback requires --allow-config-write".into());
            }
            let bytes = read_file(&plan_file, 512 * 1024)?;
            let plan = thumble_protocol::decode_unique_json(&bytes)
                .map_err(|_| "migration plan must be valid, unambiguous JSON")?;
            serde_json::to_value(rollback(paths, &plan)?)
                .map_err(|_| "migration rollback receipt encoding failed")?
        }
        MigrationAction::Apply {
            plan_file,
            package,
            allow_config_write,
        } => {
            if !allow_config_write {
                return Err("migration requires --allow-config-write".into());
            }
            let bytes = read_file(&plan_file, 512 * 1024)?;
            let plan = thumble_protocol::decode_unique_json(&bytes)
                .map_err(|_| "migration plan must be valid, unambiguous JSON")?;
            serde_json::to_value(apply(paths, &mappings(&package)?, &plan)?)
                .map_err(|_| "migration receipt encoding failed")?
        }
    };
    println!("{value}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn package_mapping_is_bounded_explicit_and_unambiguous() {
        let id = Uuid::new_v4();
        let item = format!("{id}=/private/tmp/original.zip");
        assert!(mappings(&[item.clone()]).is_ok());
        assert!(mappings(&[item.clone(), item]).is_err());
        assert!(mappings(&[format!("{id}=relative")]).is_err());
        assert!(mappings(&["invalid=/private/tmp/archive".into()]).is_err());
        assert!(mappings(&vec![String::new(); 257]).is_err());
    }
    #[test]
    fn missing_authority_never_initializes_a_catalog() {
        let root = tempfile::tempdir().unwrap();
        let paths = HostPaths::new(root.path().join("absent"), root.path().join("control.sock"));
        assert!(plan(&paths, &BTreeMap::new(), 1).is_err());
        assert!(!paths.state_dir.exists());
    }
    #[test]
    fn cli_rollback_requires_review_and_explicit_write_flag() {
        use clap::Parser;
        assert!(crate::cli::Cli::try_parse_from([
            "thumble-host",
            "configuration",
            "migration",
            "rollback",
            "--plan-file",
            "/private/tmp/plan.json"
        ])
        .is_err());
        assert!(crate::cli::Cli::try_parse_from([
            "thumble-host",
            "configuration",
            "migration",
            "rollback",
            "--plan-file",
            "/private/tmp/plan.json",
            "--allow-config-write"
        ])
        .is_ok());
    }
    #[test]
    fn cli_apply_requires_explicit_write_flag_and_review_file() {
        use clap::Parser;
        assert!(crate::cli::Cli::try_parse_from([
            "thumble-host",
            "configuration",
            "migration",
            "apply",
            "--plan-file",
            "/private/tmp/plan.json"
        ])
        .is_err());
        assert!(crate::cli::Cli::try_parse_from([
            "thumble-host",
            "configuration",
            "migration",
            "apply",
            "--plan-file",
            "/private/tmp/plan.json",
            "--allow-config-write"
        ])
        .is_ok());
    }
}
