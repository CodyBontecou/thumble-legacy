//! Direct local CLI access to the same configuration transactions as stdio MCP.
use crate::{
    control::{send_request, ControlRequest},
    draft_operation::ConfigurationOperation,
    paths::HostPaths,
};
use clap::{Args, Subcommand};
use std::{io::Read, os::unix::fs::OpenOptionsExt, path::PathBuf};
const MAXIMUM_OPERATION_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Args)]
pub struct ConfigurationArgs {
    #[command(subcommand)]
    pub action: ConfigurationCommand,
}
#[derive(Debug, Subcommand)]
pub enum ConfigurationCommand {
    Status,
    /// Prepare an owned artifact from a legacy file without writing authority state.
    PrepareImport {
        #[arg(long)]
        source_file: PathBuf,
        /// Original package mapping: PROFILE_UUID=/absolute/archive.path.
        #[arg(long)]
        package: Vec<String>,
    },
    Begin {
        #[arg(long)]
        expected_configuration_revision: u64,
    },
    Get {
        draft_id: String,
    },
    Edit {
        draft_id: String,
        #[arg(long)]
        expected_draft_revision: u64,
        #[arg(long)]
        operation_id: String,
        #[arg(
            long,
            required_unless_present = "operation_json",
            conflicts_with = "operation_json"
        )]
        operation_file: Option<PathBuf>,
        #[arg(long, required_unless_present = "operation_file")]
        operation_json: Option<String>,
    },
    Rebase {
        draft_id: String,
        #[arg(long)]
        expected_draft_revision: u64,
        #[arg(long)]
        expected_configuration_revision: u64,
        #[arg(long)]
        rebase_id: String,
    },
    Validate {
        draft_id: String,
        #[arg(long)]
        expected_draft_revision: u64,
    },
    Preview {
        draft_id: String,
        #[arg(long)]
        expected_draft_revision: u64,
    },
    Save {
        draft_id: String,
        #[arg(long)]
        expected_draft_revision: u64,
        #[arg(long)]
        expected_configuration_revision: u64,
        #[arg(long)]
        commit_id: String,
    },
    Discard {
        draft_id: String,
        #[arg(long)]
        expected_draft_revision: u64,
    },
}
impl ConfigurationCommand {
    pub(crate) fn request(self) -> Result<ControlRequest, String> {
        Ok(match self {
            Self::PrepareImport { .. } => {
                return Err(
                    "legacy preparation is handled locally without a control request".into(),
                )
            }
            Self::Status => ControlRequest::ConfigurationStatus,
            Self::Begin {
                expected_configuration_revision,
            } => ControlRequest::BeginConfigurationDraft {
                expected_configuration_revision,
            },
            Self::Get { draft_id } => ControlRequest::GetConfigurationDraft { draft_id },
            Self::Edit {
                draft_id,
                expected_draft_revision,
                operation_id,
                operation_file,
                operation_json,
            } => {
                let bytes = match (operation_file, operation_json) {
                    (Some(path), None) => {
                        let file = std::fs::OpenOptions::new()
                            .read(true)
                            .custom_flags(libc::O_NONBLOCK)
                            .open(path)
                            .map_err(|_| "configuration operation file could not be opened")?;
                        if !file
                            .metadata()
                            .map_err(|_| "configuration operation file could not be inspected")?
                            .is_file()
                        {
                            return Err(
                                "configuration operation input must be a regular file".into()
                            );
                        }
                        let mut bytes = Vec::new();
                        file.take(MAXIMUM_OPERATION_BYTES as u64 + 1)
                            .read_to_end(&mut bytes)
                            .map_err(|_| "configuration operation file could not be read")?;
                        bytes
                    }
                    (None, Some(json)) => json.into_bytes(),
                    _ => {
                        return Err(
                            "provide exactly one of --operation-file and --operation-json".into(),
                        )
                    }
                };
                if bytes.len() > MAXIMUM_OPERATION_BYTES {
                    return Err("configuration operation exceeds 8 MiB".into());
                }
                let operation: ConfigurationOperation =
                    thumble_protocol::decode_unique_json(&bytes).map_err(|_| {
                        "configuration operation must be valid, unambiguous operation JSON"
                    })?;
                ControlRequest::EditConfigurationDraft {
                    draft_id,
                    expected_draft_revision,
                    operation_id,
                    operation,
                }
            }
            Self::Rebase {
                draft_id,
                expected_draft_revision,
                expected_configuration_revision,
                rebase_id,
            } => ControlRequest::RebaseConfigurationDraft {
                draft_id,
                expected_draft_revision,
                expected_configuration_revision,
                rebase_id,
            },
            Self::Validate {
                draft_id,
                expected_draft_revision,
            } => ControlRequest::ValidateConfigurationDraft {
                draft_id,
                expected_draft_revision,
            },
            Self::Preview {
                draft_id,
                expected_draft_revision,
            } => ControlRequest::PreviewConfigurationDraft {
                draft_id,
                expected_draft_revision,
            },
            Self::Save {
                draft_id,
                expected_draft_revision,
                expected_configuration_revision,
                commit_id,
            } => ControlRequest::SaveConfigurationDraft {
                draft_id,
                expected_draft_revision,
                expected_configuration_revision,
                commit_id,
            },
            Self::Discard {
                draft_id,
                expected_draft_revision,
            } => ControlRequest::DiscardConfigurationDraft {
                draft_id,
                expected_draft_revision,
            },
        })
    }
}
pub async fn run(paths: &HostPaths, args: ConfigurationArgs) -> Result<(), String> {
    if let ConfigurationCommand::PrepareImport {
        source_file,
        package,
    } = &args.action
    {
        let mut mappings = std::collections::BTreeMap::new();
        for item in package {
            let (id, path) = item
                .split_once('=')
                .ok_or("package must be PROFILE_UUID=/absolute/archive.path")?;
            let id: uuid::Uuid = id
                .parse()
                .map_err(|_| "package profile ID must be a UUID")?;
            if mappings.insert(id, PathBuf::from(path)).is_some() {
                return Err("duplicate package profile ID".into());
            }
        }
        let source = crate::legacy_import::read_file(
            source_file,
            thumble_core::MAXIMUM_CONFIGURATION_DOCUMENT_BYTES,
        )?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| "system clock is unavailable")?
            .as_millis()
            .try_into()
            .map_err(|_| "system clock exceeds its bound")?;
        let result = crate::legacy_import::prepare(&source, &mappings, now)?;
        println!(
            "{}",
            serde_json::to_string(&result)
                .map_err(|_| "legacy preparation result could not be encoded")?
        );
        return Ok(());
    }
    let request = args.action.request()?;
    let response = send_request(&paths.control_socket, &request).await?;
    println!(
        "{}",
        serde_json::to_string(&response)
            .map_err(|_| "configuration response could not be encoded")?
    );
    if response.ok {
        Ok(())
    } else {
        Err("configuration transaction failed; see JSON error code".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    #[test]
    fn cli_transactions_require_revisions_and_one_operation_source() {
        let parse = |args: &[&str]| crate::cli::Cli::try_parse_from(args);
        assert!(parse(&["thumble-host", "configuration", "save", "draft"]).is_err());
        assert!(parse(&[
            "thumble-host",
            "configuration",
            "edit",
            "draft",
            "--expected-draft-revision",
            "1",
            "--operation-id",
            "op"
        ])
        .is_err());
        let parsed = parse(&[
            "thumble-host",
            "configuration",
            "save",
            "draft",
            "--expected-draft-revision",
            "2",
            "--expected-configuration-revision",
            "41",
            "--commit-id",
            "commit",
        ])
        .unwrap();
        let Some(crate::cli::HostCommand::Configuration(args)) = parsed.command else {
            panic!("wrong command");
        };
        assert!(matches!(
            args.action.request().unwrap(),
            ControlRequest::SaveConfigurationDraft {
                expected_draft_revision: 2,
                expected_configuration_revision: 41,
                ..
            }
        ));
    }
    #[test]
    fn edit_rejects_duplicate_keys_and_oversized_files_before_transport() {
        let action = |json: &str| ConfigurationCommand::Edit {
            draft_id: "draft".into(),
            expected_draft_revision: 1,
            operation_id: "op".into(),
            operation_file: None,
            operation_json: Some(json.into()),
        };
        assert!(
            action(r#"{"type":"profile.rename","type":"profile.delete"}"#)
                .request()
                .is_err()
        );
        assert!(action(&" ".repeat(MAXIMUM_OPERATION_BYTES + 1))
            .request()
            .is_err());
        let mut file = tempfile::NamedTempFile::new().unwrap();
        use std::io::Write;
        file.write_all(&vec![b' '; MAXIMUM_OPERATION_BYTES + 1])
            .unwrap();
        let command = ConfigurationCommand::Edit {
            draft_id: "draft".into(),
            expected_draft_revision: 1,
            operation_id: "op".into(),
            operation_file: Some(file.path().into()),
            operation_json: None,
        };
        assert!(command.request().unwrap_err().contains("exceeds"));
    }
}
