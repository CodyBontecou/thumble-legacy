use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use tempfile::{tempdir, TempDir};
use thumble_core::{Effect, HostCore, TokenSource};
use thumble_protocol::{ControllerMessage, ControllerMessageType as Kind};
use uuid::Uuid;

fn restore(paths: &HostPaths, hash: &str, bytes: &[u8]) -> Result<(), MigrationError> {
    let path = paths.state_dir.join("restored-packages");
    directory(&path)?;
    once(&path.join(hash), bytes)
}
fn recover(
    paths: &HostPaths,
    lease: &AuthorityLock,
    id: &str,
) -> Result<MigrationReceipt, MigrationError> {
    super::recover(paths, lease, id, |hash, bytes| restore(paths, hash, bytes))
}
fn rollback(
    paths: &HostPaths,
    lease: &AuthorityLock,
    id: &str,
) -> Result<MigrationReceipt, MigrationError> {
    super::rollback(paths, lease, id, |hash, bytes| restore(paths, hash, bytes))
}
fn fixture(name: &str, suffix: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/legacy-configuration-migration/v1")
            .join(format!("{name}-{suffix}.json")),
    )
    .unwrap()
}
fn source(name: &str) -> Vec<u8> {
    let profile = fixture(name, "profile");
    let value: Value = serde_json::from_slice(&profile).unwrap();
    format!(r#"{{"schemaVersion":3,"serverID":"stable-server","trustedClients":{{"private-token":{{"name":"Phone","createdAt":1,"lastSeenAt":2,"futureTrust":18446744073709551617}}}},"configurationRevision":41,"configurationUpdatedAt":5,"recentConfigurationCommits":[],"profiles":[{}],"activeProfileID":"{}","defaultProfileID":"{}","keyBindings":{{}},"outputBindings":{{}},"profileKeyBindings":{{}},"profileOutputBindings":{{}},"futureState":{{"integer":18446744073709551617,"decimal":0.12345678901234567890123456789}}}}
"#,std::str::from_utf8(&profile).unwrap(),value["id"].as_str().unwrap(),value["id"].as_str().unwrap()).into_bytes()
}
fn candidate(name: &str) -> OwnedStateMigration {
    prepare(&source(name), 1000, |profile| {
        let request: Value = serde_json::from_slice(&fixture(name, "request")).unwrap();
        let original_package = request
            .get("packageData")
            .map(|value| STANDARD.decode(value.as_str().unwrap()).unwrap());
        let hash = original_package.as_deref().map(digest);
        let preparation = NativeLegacyPreparation::decode(
            profile,
            Uuid::parse_str(request["invocationID"].as_str().unwrap()).unwrap(),
            hash.as_deref(),
            &fixture(name, "response"),
        )?;
        Ok(PreparedMigrationProfile {
            preparation,
            original_package,
        })
    })
    .unwrap()
}
fn setup(name: &str) -> (TempDir, HostPaths, AuthorityLock, OwnedStateMigration) {
    let root = tempdir().unwrap();
    let paths = HostPaths::new(root.path().join("state"), root.path().join("control.sock"));
    let lease = AuthorityLock::try_acquire(&paths).unwrap().unwrap();
    let candidate = candidate(name);
    fs::write(&paths.state_file, candidate.source_bytes()).unwrap();
    fs::set_permissions(&paths.state_file, fs::Permissions::from_mode(0o600)).unwrap();
    (root, paths, lease, candidate)
}
fn receipt_path(paths: &HostPaths, candidate: &OwnedStateMigration) -> PathBuf {
    paths
        .state_dir
        .join("owned-migrations-v1")
        .join(&candidate.receipt.source_sha256)
}

#[test]
fn commit_backup_restart_repeat_and_monotonic_rollback_preserve_authority() {
    for name in [
        "plain",
        "portrait-only",
        "explicit-landscape",
        "legacy-canvas-alias",
        "missing-canvas",
    ] {
        let (_root, paths, lease, candidate) = setup(name);
        let receipt = commit(&paths, &lease, &candidate).unwrap();
        assert_eq!(receipt.phase, MigrationPhase::Committed);
        assert_eq!(
            read(&paths.state_file).unwrap(),
            candidate.destination_bytes()
        );
        let journal = receipt_path(&paths, &candidate);
        assert_eq!(
            read(&journal.join("source-state.json")).unwrap(),
            candidate.source_bytes()
        );
        for (hash, bytes) in &candidate.packages {
            assert_eq!(
                read_bounded(&journal.join("packages").join(hash), MAXIMUM_PACKAGE).unwrap(),
                bytes.as_ref()
            );
        }
        let loaded = storage::load_or_migrate(&paths).unwrap();
        assert_eq!(loaded.schema_version, OWNED_SCHEMA_VERSION);
        assert_eq!(loaded.configuration_revision, 42);
        assert_eq!(commit(&paths, &lease, &candidate).unwrap(), receipt);
        let raw = String::from_utf8(read(&paths.state_file).unwrap()).unwrap();
        assert!(raw.contains("18446744073709551617"));
        assert!(raw.contains("0.12345678901234567890123456789"));
        assert!(raw.contains("\"futureTrust\":18446744073709551617"));
        let legacy =
            ConfigurationDocument::from_state(&state(candidate.source_bytes()).unwrap()).unwrap();
        let mut owned = loaded.clone();
        assert!(legacy.install_into(&mut owned).is_err());
        assert_eq!(owned, loaded);
        let rolled = rollback(&paths, &lease, &receipt.source_sha256).unwrap();
        assert_eq!(rolled.phase, MigrationPhase::RolledBack);
        let mut original = state(candidate.source_bytes()).unwrap();
        original.configuration_revision = 43;
        original.configuration_updated_at = 1000;
        assert_eq!(storage::load_or_migrate(&paths).unwrap(), original);
        for (hash, bytes) in &candidate.packages {
            assert_eq!(
                read_bounded(
                    &paths.state_dir.join("restored-packages").join(hash),
                    MAXIMUM_PACKAGE
                )
                .unwrap(),
                bytes.as_ref()
            );
        }
        let bytes = read(&paths.state_file).unwrap();
        assert_eq!(
            rollback(&paths, &lease, &receipt.source_sha256).unwrap(),
            rolled
        );
        assert_eq!(read(&paths.state_file).unwrap(), bytes);
        assert_eq!(
            commit(&paths, &lease, &candidate).unwrap_err(),
            MigrationError::Conflict
        );
        assert_eq!(
            read(&journal.join("source-state.json")).unwrap(),
            candidate.source_bytes()
        );
        let receipt_json = read(&journal.join("receipt.json")).unwrap();
        assert!(!String::from_utf8(receipt_json)
            .unwrap()
            .contains("private-token"));
    }
}
#[test]
fn interrupted_checkpoints_recover_only_recorded_validated_intent() {
    for fail in [
        Checkpoint::Backup,
        Checkpoint::Destination,
        Checkpoint::Prepared,
        Checkpoint::Installed,
    ] {
        let (_root, paths, lease, candidate) = setup("portrait-only");
        assert!(commit_with_hook(&paths, &lease, &candidate, |stage| {
            if std::mem::discriminant(&stage) == std::mem::discriminant(&fail) {
                Err(MigrationError::Storage)
            } else {
                Ok(())
            }
        })
        .is_err());
        let before = read(&paths.state_file).unwrap();
        if matches!(fail, Checkpoint::Backup | Checkpoint::Destination) {
            assert_eq!(before, candidate.source_bytes());
            assert!(recover(&paths, &lease, &candidate.receipt.source_sha256).is_err());
            assert_eq!(read(&paths.state_file).unwrap(), before);
            commit(&paths, &lease, &candidate).unwrap();
        } else {
            assert_eq!(
                recover(&paths, &lease, &candidate.receipt.source_sha256)
                    .unwrap()
                    .phase,
                MigrationPhase::Committed
            );
        }
        assert_eq!(
            read(&paths.state_file).unwrap(),
            candidate.destination_bytes()
        );
        assert_eq!(
            read(&receipt_path(&paths, &candidate).join("source-state.json")).unwrap(),
            candidate.source_bytes()
        );
    }
}
#[test]
fn rollback_interruption_finishes_without_reapplying_or_lowering_revision() {
    let (_root, paths, lease, candidate) = setup("portrait-only");
    commit(&paths, &lease, &candidate).unwrap();
    assert!(rollback_with_hook(
        &paths,
        &lease,
        &candidate.receipt.source_sha256,
        |hash, bytes| restore(&paths, hash, bytes),
        |_| Err(MigrationError::Storage)
    )
    .is_err());
    let before = read(&paths.state_file).unwrap();
    assert_eq!(state(&before).unwrap().configuration_revision, 43);
    assert_eq!(
        recover(&paths, &lease, &candidate.receipt.source_sha256)
            .unwrap()
            .phase,
        MigrationPhase::RolledBack
    );
    assert_eq!(read(&paths.state_file).unwrap(), before);
}
#[test]
fn source_changes_conflict_at_preparation_commit_recovery_and_rollback() {
    for checkpoint in [
        None,
        Some(Checkpoint::Prepared),
        Some(Checkpoint::Installed),
    ] {
        let (_root, paths, lease, candidate) = setup("plain");
        if let Some(fail) = checkpoint {
            assert!(commit_with_hook(&paths, &lease, &candidate, |stage| {
                if std::mem::discriminant(&stage) == std::mem::discriminant(&fail) {
                    Err(MigrationError::Storage)
                } else {
                    Ok(())
                }
            })
            .is_err());
        }
        let mut changed = read(&paths.state_file).unwrap();
        changed.push(b' ');
        fs::write(&paths.state_file, &changed).unwrap();
        if checkpoint.is_none() {
            assert_eq!(
                commit(&paths, &lease, &candidate).unwrap_err(),
                MigrationError::Conflict
            );
        } else {
            assert_eq!(
                recover(&paths, &lease, &candidate.receipt.source_sha256).unwrap_err(),
                MigrationError::Conflict
            );
        }
        assert_eq!(read(&paths.state_file).unwrap(), changed);
    }
    let (_root, paths, lease, candidate) = setup("plain");
    commit(&paths, &lease, &candidate).unwrap();
    let mut changed = storage::load(&paths.state_file).unwrap();
    changed
        .trusted_clients
        .get_mut("private-token")
        .unwrap()
        .last_seen_at += 1;
    storage::save_atomic(&paths.state_file, &changed).unwrap();
    let bytes = read(&paths.state_file).unwrap();
    assert_eq!(
        rollback(&paths, &lease, &candidate.receipt.source_sha256).unwrap_err(),
        MigrationError::Conflict
    );
    assert_eq!(read(&paths.state_file).unwrap(), bytes);
}
#[test]
fn corrupt_missing_or_partial_backups_do_not_replace_live_state() {
    for file in [
        "source-state.json",
        "destination-state.json",
        "receipt.json",
        "package",
    ] {
        let (_root, paths, lease, candidate) = setup("portrait-only");
        assert!(commit_with_hook(&paths, &lease, &candidate, |stage| {
            if matches!(stage, Checkpoint::Prepared) {
                Err(MigrationError::Storage)
            } else {
                Ok(())
            }
        })
        .is_err());
        let journal = receipt_path(&paths, &candidate);
        let path = if file == "package" {
            journal
                .join("packages")
                .join(candidate.packages.keys().next().unwrap())
        } else {
            journal.join(file)
        };
        fs::write(path, b"partial-or-corrupt").unwrap();
        assert!(recover(&paths, &lease, &candidate.receipt.source_sha256).is_err());
        assert_eq!(read(&paths.state_file).unwrap(), candidate.source_bytes());
    }
    let (_root, paths, lease, candidate) = setup("plain");
    let journal = journal(&paths, &lease, &candidate.receipt.source_sha256).unwrap();
    once(&journal.join("source-state.json"), b"partial").unwrap();
    assert!(commit(&paths, &lease, &candidate).is_err());
    assert_eq!(read(&paths.state_file).unwrap(), candidate.source_bytes());
    assert_eq!(
        read(&journal.join("source-state.json")).unwrap(),
        b"partial"
    );
}
#[test]
fn lease_directory_and_symlink_boundaries_fail_closed() {
    let (_root, paths, lease, candidate) = setup("plain");
    let (_other, other, other_lease, _) = setup("plain");
    assert_eq!(
        commit(&paths, &other_lease, &candidate).unwrap_err(),
        MigrationError::Security
    );
    assert_eq!(
        recover(&paths, &lease, "../escape").unwrap_err(),
        MigrationError::Security
    );
    std::os::unix::fs::symlink(
        &other.state_dir,
        paths.state_dir.join("owned-migrations-v1"),
    )
    .unwrap();
    assert_eq!(
        commit(&paths, &lease, &candidate).unwrap_err(),
        MigrationError::Security
    );
    assert_eq!(read(&paths.state_file).unwrap(), candidate.source_bytes());
}
#[test]
fn missing_archive_and_invalid_source_fail_before_any_persistence() {
    let source = source("portrait-only");
    let result = prepare(&source, 1000, |profile| {
        let request: Value = serde_json::from_slice(&fixture("portrait-only", "request")).unwrap();
        let package = STANDARD
            .decode(request["packageData"].as_str().unwrap())
            .unwrap();
        let hash = digest(&package);
        let preparation = NativeLegacyPreparation::decode(
            profile,
            Uuid::parse_str(request["invocationID"].as_str().unwrap()).unwrap(),
            Some(&hash),
            &fixture("portrait-only", "response"),
        )?;
        Ok(PreparedMigrationProfile {
            preparation,
            original_package: None,
        })
    });
    assert!(matches!(result, Err(MigrationError::Preparation)));
    for raw in [
        b"{}".as_slice(),
        b"{\"schemaVersion\":3,\"schemaVersion\":3}".as_slice(),
    ] {
        assert!(prepare(raw, 1000, |_| panic!(
            "invalid source reached native helper"
        ))
        .is_err());
    }
    let mut valid: Value = serde_json::from_slice(&source).unwrap();
    valid["configurationRevision"] = json!(u64::MAX);
    assert!(
        prepare(&serde_json::to_vec(&valid).unwrap(), 1000, |_| panic!(
            "exhausted revision reached helper"
        ))
        .is_err()
    );
}
struct Tokens;
impl TokenSource for Tokens {
    fn next_pairing_code(&mut self) -> String {
        "222222".into()
    }
    fn next_auth_token(&mut self) -> String {
        "new-token".into()
    }
}
#[test]
fn actual_owned_authority_startup_rejects_legacy_peers_before_trust_or_profiles() {
    let candidate = candidate("portrait-only");
    let loaded = state(candidate.destination_bytes()).unwrap();
    for offered in [None, Some(vec![4]), Some(vec![4, 5])] {
        let mut core = HostCore::new(loaded.clone(), "111111").unwrap();
        let mut hello = ControllerMessage::new(Kind::Hello, 0);
        hello.pairing_code = Some("111111".into());
        hello.supported_profile_formats = offered.clone();
        let effects = core.handle_message(1, hello, 1000, &mut Tokens).unwrap();
        if offered == Some(vec![4, 5]) {
            assert!(effects.iter().any(|effect|matches!(effect,Effect::SendMessage{message,..} if message.message_type==Kind::PairingAccepted && message.profile_format_version==Some(5))));
        } else {
            assert!(effects
                .iter()
                .all(|effect| !matches!(effect, Effect::PersistState)));
            assert!(effects.iter().all(|effect| match effect {
                Effect::SendMessage { message, .. } =>
                    message.gamepad_profiles.is_none() && message.auth_token.is_none(),
                _ => true,
            }));
            assert_eq!(core.persistent_state(), &loaded);
        }
    }
    let mut wrong = loaded;
    wrong.schema_version = CURRENT_SCHEMA_VERSION;
    assert!(wrong.validate().is_err());
    let mut wrong = state(candidate.source_bytes()).unwrap();
    wrong.schema_version = OWNED_SCHEMA_VERSION;
    assert!(wrong.validate().is_err());
}

#[test]
fn failed_package_restore_keeps_owned_state_and_can_resume_durably() {
    let (_root, paths, lease, candidate) = setup("portrait-only");
    commit(&paths, &lease, &candidate).unwrap();
    assert_eq!(
        super::rollback(
            &paths,
            &lease,
            &candidate.receipt.source_sha256,
            |_, _| Err(MigrationError::Storage)
        )
        .unwrap_err(),
        MigrationError::Storage
    );
    assert_eq!(
        read(&paths.state_file).unwrap(),
        candidate.destination_bytes()
    );
    assert_eq!(
        recover(&paths, &lease, &candidate.receipt.source_sha256)
            .unwrap()
            .phase,
        MigrationPhase::RolledBack
    );
    assert_eq!(
        storage::load(&paths.state_file)
            .unwrap()
            .configuration_revision,
        43
    );
    for (hash, bytes) in &candidate.packages {
        assert_eq!(
            read_bounded(
                &paths.state_dir.join("restored-packages").join(hash),
                MAXIMUM_PACKAGE
            )
            .unwrap(),
            bytes.as_ref()
        );
    }
}

#[test]
fn legacy_wire_mutations_cannot_corrupt_an_owned_authority() {
    let candidate = candidate("portrait-only");
    let mut core = HostCore::new(state(candidate.destination_bytes()).unwrap(), "111111").unwrap();
    let mut hello = ControllerMessage::new(Kind::Hello, 0);
    hello.pairing_code = Some("111111".into());
    hello.supported_profile_formats = Some(vec![5]);
    let effects = core.handle_message(1, hello, 1000, &mut Tokens).unwrap();
    for effect in effects {
        if let Effect::SendMessage { message, .. } = effect {
            assert!(!message.capabilities.unwrap_or_default().contains(&thumble_protocol::ControllerCapability::GamepadProfileOrientationPreferenceMutation));
        }
    }
    let before = core.persistent_state().clone();
    for kind in [
        Kind::GamepadCustomization,
        Kind::GamepadProfileOrientationPreferenceMutation,
    ] {
        let mut message = ControllerMessage::new(kind, 0);
        message.gamepad_customization = Some(json!({"elements":[]}));
        message.gamepad_profile_id = Some(before.active_profile_id.clone());
        message.gamepad_profile_orientation_preference_mutation =
            Some(thumble_protocol::GamepadProfileOrientationPreference::Portrait);
        let effects = core.handle_message(1, message, 1001, &mut Tokens).unwrap();
        assert!(effects
            .iter()
            .all(|effect| !matches!(effect, Effect::PersistState)));
        assert!(effects.iter().any(|effect|matches!(effect,Effect::SendMessage{message,..} if message.message_type==Kind::Error)));
        assert_eq!(core.persistent_state(), &before);
        core.persistent_state().validate().unwrap();
    }
}

#[test]
fn reviewed_recovery_requires_exact_candidate_and_current_destination() {
    let (_root, paths, lease, candidate) = setup("plain");
    // An unapplied plan must not install anything through the replay path.
    assert!(super::recover_reviewed(&paths, &lease, candidate.receipt()).is_err());
    super::commit(&paths, &lease, &candidate).unwrap();
    let original = read(&paths.state_file).unwrap();
    assert_eq!(
        super::recover_reviewed(&paths, &lease, candidate.receipt())
            .unwrap()
            .phase,
        MigrationPhase::Committed
    );
    let mut changed = candidate.receipt().clone();
    changed.result_revision += 1;
    assert!(super::recover_reviewed(&paths, &lease, &changed).is_err());
    assert_eq!(read(&paths.state_file).unwrap(), original);
    let edited = edit_json_values(
        &original,
        &[JsonValueEdit {
            pointer: "/configurationRevision",
            value: Some(b"43"),
        }],
        MAXIMUM,
    )
    .unwrap();
    storage::install_private_state_bytes(&paths.state_file, &edited, Some(&original), || Ok(()))
        .unwrap();
    assert!(super::recover_reviewed(&paths, &lease, candidate.receipt()).is_err());
    assert_eq!(read(&paths.state_file).unwrap(), edited);
}

#[test]
fn reviewed_rollback_binds_plan_and_refuses_later_forward_or_legacy_writes() {
    let (_root, paths, lease, candidate) = setup("plain");
    assert!(
        super::rollback_reviewed(&paths, &lease, candidate.receipt(), |h, b| restore(
            &paths, h, b
        ))
        .is_err()
    );
    commit(&paths, &lease, &candidate).unwrap();
    let mut altered = candidate.receipt().clone();
    altered.result_revision += 1;
    assert!(
        super::rollback_reviewed(&paths, &lease, &altered, |h, b| restore(&paths, h, b)).is_err()
    );
    let receipt = super::rollback_reviewed(&paths, &lease, candidate.receipt(), |h, b| {
        restore(&paths, h, b)
    })
    .unwrap();
    assert_eq!(receipt.phase, MigrationPhase::RolledBack);
    assert_eq!(
        super::rollback_reviewed(&paths, &lease, candidate.receipt(), |h, b| restore(
            &paths, h, b
        ))
        .unwrap(),
        receipt
    );
    let original = read(&paths.state_file).unwrap();
    let edited = edit_json_values(
        &original,
        &[JsonValueEdit {
            pointer: "/configurationRevision",
            value: Some(b"44"),
        }],
        MAXIMUM,
    )
    .unwrap();
    storage::install_private_state_bytes(&paths.state_file, &edited, Some(&original), || Ok(()))
        .unwrap();
    assert!(
        super::rollback_reviewed(&paths, &lease, candidate.receipt(), |h, b| restore(
            &paths, h, b
        ))
        .is_err()
    );
    assert_eq!(read(&paths.state_file).unwrap(), edited);
}

#[test]
fn reviewed_rollback_rejects_corrupt_opaque_bytes_even_with_matching_receipt_hash() {
    let (_root, paths, lease, candidate) = setup("plain");
    commit(&paths, &lease, &candidate).unwrap();
    super::rollback_reviewed(&paths, &lease, candidate.receipt(), |h, b| {
        restore(&paths, h, b)
    })
    .unwrap();
    let journal = receipt_path(&paths, &candidate);
    let original = read(&paths.state_file).unwrap();
    let changed = edit_json_values(
        &original,
        &[JsonValueEdit {
            pointer: "/futureState/decimal",
            value: Some(b"0.5"),
        }],
        MAXIMUM,
    )
    .unwrap();
    fs::write(journal.join("rollback-state.json"), &changed).unwrap();
    fs::write(&paths.state_file, &changed).unwrap();
    let (mut receipt, _, _) = load_journal(&journal, &candidate.receipt.source_sha256).unwrap();
    receipt.rollback_sha256 = Some(digest(&changed));
    write_receipt(&journal, &receipt).unwrap();
    assert!(
        super::rollback_reviewed(&paths, &lease, candidate.receipt(), |_, _| panic!(
            "corrupt rollback must not restore resources"
        ))
        .is_err()
    );
    assert_eq!(read(&paths.state_file).unwrap(), changed);
}
