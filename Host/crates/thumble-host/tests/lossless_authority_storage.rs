use serde_json::json;
use std::fs;
use tempfile::tempdir;
use thumble_core::{PersistentState, TrustedClient};
use thumble_host::{
    cli_profile::{execute_offline_authority, CliProfileRequest, CLI_PROFILE_SCHEMA_VERSION},
    paths::HostPaths,
    runtime::AuthorityLock,
    storage,
};
use uuid::Uuid;

#[test]
fn standalone_cli_commit_and_replay_preserve_raw_state_and_trust() {
    let root = tempdir().unwrap();
    let paths = HostPaths::new(root.path().join("state"), root.path().join("control.sock"));
    let _lease = AuthorityLock::try_acquire(&paths).unwrap().unwrap();
    let mut state = PersistentState::minimal("retained-server").unwrap();
    state.trusted_clients.insert(
        "private-token".into(),
        TrustedClient {
            name: "Phone".into(),
            created_at: 1,
            last_seen_at: 2,
        },
    );
    storage::save_atomic(&paths.state_file, &state).unwrap();
    let raw = fs::read_to_string(&paths.state_file).unwrap();
    let raw = raw.replacen('{', "{\n\"futureAuthority\":{\"number\":18446744073709551617,\"decimal\":0.12345678901234567890123456789},",1);
    let raw = raw.replacen(
        "\"name\": \"Phone\"",
        "\"futureTrust\":18446744073709551617,\"name\": \"Phone\"",
        1,
    );
    fs::write(&paths.state_file, raw).unwrap();
    let request: CliProfileRequest = serde_json::from_value(json!({
        "schemaVersion":CLI_PROFILE_SCHEMA_VERSION,"invocationID":Uuid::new_v4(),
        "expectedConfigurationRevision":state.configuration_revision,
        "command":{"type":"profile.rename","target":{"kind":"active"},"name":"Local agent edit"}
    }))
    .unwrap();
    let result = execute_offline_authority(&paths, &request);
    assert!(result.ok, "{:?}", result.error);
    assert_eq!(
        result.outcome.unwrap().configuration_revision,
        state.configuration_revision + 1
    );
    let saved = fs::read_to_string(&paths.state_file).unwrap();
    assert!(saved.contains("\"number\":18446744073709551617"));
    assert!(saved.contains("\"decimal\":0.12345678901234567890123456789"));
    assert!(saved.contains("\"futureTrust\":18446744073709551617"));
    let committed = storage::load(&paths.state_file).unwrap();
    assert_eq!(committed.server_id, state.server_id);
    assert_eq!(committed.trusted_clients, state.trusted_clients);
    assert_eq!(committed.profiles[0]["name"], "Local agent edit");
    let replay = execute_offline_authority(&paths, &request);
    assert!(replay.ok, "{:?}", replay.error);
    assert!(replay.outcome.unwrap().idempotent_replay);
    assert_eq!(fs::read_to_string(&paths.state_file).unwrap(), saved);
}
