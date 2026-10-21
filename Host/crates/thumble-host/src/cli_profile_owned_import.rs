//! Raw v5 import planning; no legacy/native roundtrip or implicit migration.
use super::*;
use crate::draft_operation::ConfigurationOperationOutcome;
use thumble_core::OwnedProfileArtifact;
use thumble_protocol::JsonValueEdit;
type RawObject = BTreeMap<String, Box<RawValue>>;
fn invalid() -> TransactionFailure {
    TransactionFailure::new(
        "invalid_owned_profile_artifact",
        "owned profile artifact or import candidate is invalid",
    )
}
fn object(bytes: &[u8]) -> Result<RawObject, TransactionFailure> {
    serde_json::from_slice(bytes).map_err(|_| invalid())
}
fn raw<T: Serialize>(value: &T) -> Result<Box<RawValue>, TransactionFailure> {
    serde_json::value::to_raw_value(value).map_err(|_| invalid())
}
fn id(profile: &RawObject) -> Result<String, TransactionFailure> {
    serde_json::from_str(profile.get("id").ok_or_else(invalid)?.get()).map_err(|_| invalid())
}
fn replace(map: &mut RawObject, id: &str, value: Box<RawValue>) {
    map.retain(|key, _| !key.eq_ignore_ascii_case(id));
    map.insert(id.into(), value);
}
pub(super) struct Import {
    artifact: OwnedProfileArtifact,
    descriptor: ProfileImportDescriptor,
}
pub(super) struct Plan {
    pub document: OwnedConfigurationDocument,
    pub names: Vec<String>,
    pub outcome: ConfigurationOperationOutcome,
}
pub(super) fn is_owned(bytes: &[u8]) -> bool {
    if bytes.len() > thumble_core::MAXIMUM_OWNED_PROFILE_ARTIFACT_BYTES {
        return false;
    }
    object(bytes)
        .ok()
        .and_then(|m| {
            m.get("schema")
                .and_then(|v| serde_json::from_str::<String>(v.get()).ok())
        })
        .is_some_and(|s| s == thumble_core::OWNED_PROFILE_ARTIFACT_SCHEMA)
}
pub(super) fn decode(
    command: &CliProfileCommand,
    invocation: Uuid,
) -> Result<Option<Import>, TransactionFailure> {
    let CliProfileCommand::Import {
        artifact_json,
        append_as_copies,
        select,
        make_default,
    } = command
    else {
        return Ok(None);
    };
    let artifact =
        OwnedProfileArtifact::decode_json(artifact_json.as_bytes()).map_err(|_| invalid())?;
    let generated_profile_ids = if *append_as_copies {
        artifact
            .document()
            .profiles()
            .iter()
            .enumerate()
            .map(|(ordinal, p)| {
                deterministic_uuid(
                    invocation,
                    &format!(
                        "profile.import:append:{}:{ordinal}",
                        p["id"].as_str().unwrap().to_ascii_lowercase()
                    ),
                )
                .to_string()
            })
            .collect()
    } else {
        vec![]
    };
    let descriptor = ProfileImportDescriptor {
        operation: "profile.import",
        content_hash: artifact.content_hash().clone(),
        append_as_copies: *append_as_copies,
        select: *select,
        make_default: *make_default,
        generated_profile_ids,
    };
    Ok(Some(Import {
        artifact,
        descriptor,
    }))
}
impl Import {
    pub fn digest(&self) -> Result<String, TransactionFailure> {
        descriptor_digest(&self.descriptor)
    }
    pub fn replay_names(&self, catalog: &CliProfileCatalog) -> Vec<String> {
        self.artifact
            .document()
            .profiles()
            .iter()
            .enumerate()
            .map(|(index, p)| {
                let target = if self.descriptor.append_as_copies {
                    self.descriptor.generated_profile_ids[index].as_str()
                } else {
                    p["id"].as_str().unwrap()
                };
                catalog
                    .profiles
                    .iter()
                    .find(|v| v.profile_id.to_string().eq_ignore_ascii_case(target))
                    .map(|v| v.name.clone())
                    .unwrap_or_else(|| p["name"].as_str().unwrap().into())
            })
            .collect()
    }
    pub fn plan(
        &self,
        destination: &OwnedConfigurationDocument,
        now: i64,
    ) -> Result<Plan, TransactionFailure> {
        let mut root = object(destination.original_bytes())?;
        let imported = object(self.artifact.document().original_bytes())?;
        let mut profiles: Vec<RawObject> =
            serde_json::from_str(root["profiles"].get()).map_err(|_| invalid())?;
        let incoming: Vec<RawObject> =
            serde_json::from_str(imported["profiles"].get()).map_err(|_| invalid())?;
        let mut keys = root
            .get("profileKeyBindings")
            .map(|v| object(v.get().as_bytes()))
            .transpose()?
            .unwrap_or_default();
        let mut outputs = root
            .get("profileOutputBindings")
            .map(|v| object(v.get().as_bytes()))
            .transpose()?
            .unwrap_or_default();
        let incoming_keys = imported
            .get("profileKeyBindings")
            .map(|v| object(v.get().as_bytes()))
            .transpose()?
            .unwrap_or_default();
        let incoming_outputs = imported
            .get("profileOutputBindings")
            .map(|v| object(v.get().as_bytes()))
            .transpose()?
            .unwrap_or_default();
        let mut mapping = BTreeMap::new();
        let mut claimed = HashSet::new();
        let mut names = Vec::new();
        for (ordinal, mut profile) in incoming.into_iter().enumerate() {
            let source_id = id(&profile)?;
            let name = self.artifact.document().profiles()[ordinal]["name"]
                .as_str()
                .ok_or_else(invalid)?;
            let index = if self.descriptor.append_as_copies {
                None
            } else {
                profiles
                    .iter()
                    .position(|p| {
                        id(p).is_ok_and(|v| {
                            v.eq_ignore_ascii_case(&source_id)
                                && !claimed.contains(&v.to_ascii_lowercase())
                        })
                    })
                    .or_else(|| {
                        profiles.iter().position(|p| {
                            id(p).is_ok_and(|v| !claimed.contains(&v.to_ascii_lowercase()))
                                && p.get("name").is_some_and(|v| {
                                    serde_json::from_str::<String>(v.get())
                                        .is_ok_and(|s| s.to_lowercase() == name.to_lowercase())
                                })
                        })
                    })
            };
            let target = if self.descriptor.append_as_copies {
                self.descriptor.generated_profile_ids[ordinal].clone()
            } else if let Some(index) = index {
                id(&profiles[index])?
            } else {
                source_id.clone()
            };
            if index.is_none()
                && profiles
                    .iter()
                    .any(|p| id(p).is_ok_and(|v| v.eq_ignore_ascii_case(&target)))
            {
                return Err(invalid());
            }
            let projected: Vec<Value> = profiles
                .iter()
                .map(serde_json::to_value)
                .collect::<Result<_, _>>()
                .map_err(|_| invalid())?;
            let selected_name = if index.is_some() {
                name.to_owned()
            } else {
                unique_import_name(&projected, name, None)
            };
            profile.insert("id".into(), raw(&target)?);
            profile.insert("name".into(), raw(&selected_name)?);
            profile.insert("updatedAt".into(), raw(&now)?);
            if let Some(index) = index {
                if let Some(local) = profiles[index].get("launchTarget") {
                    profile.insert("launchTarget".into(), local.clone());
                }
                profiles[index] = profile;
            } else {
                profiles.push(profile);
            }
            for (incoming_map, global, destination_map) in [
                (&incoming_keys, "keyBindings", &mut keys),
                (&incoming_outputs, "outputBindings", &mut outputs),
            ] {
                let value = incoming_map
                    .iter()
                    .find(|(id, _)| id.eq_ignore_ascii_case(&source_id))
                    .map(|(_, v)| v.clone())
                    .or_else(|| {
                        source_id
                            .eq_ignore_ascii_case(self.artifact.document().active_profile_id())
                            .then(|| imported.get(global).cloned())
                            .flatten()
                    });
                let value = match value {
                    Some(v) => v,
                    None => {
                        let owned = thumble_core::profile_owned_outputs(
                            &self.artifact.document().profiles()[ordinal],
                        );
                        if global == "outputBindings" {
                            raw(&owned)?
                        } else {
                            let mut bindings = ButtonBindings::default();
                            for (id, output) in owned.iter_ids() {
                                if let Some(key) = &output.keyboard {
                                    bindings.insert(id, key.clone());
                                }
                            }
                            raw(&bindings)?
                        }
                    }
                };
                replace(destination_map, &target, value);
            }
            mapping.insert(source_id.to_ascii_lowercase(), target.clone());
            claimed.insert(target.to_ascii_lowercase());
            names.push(selected_name);
        }
        let active = mapping
            .get(
                &self
                    .artifact
                    .document()
                    .active_profile_id()
                    .to_ascii_lowercase(),
            )
            .ok_or_else(invalid)?;
        if self.descriptor.select {
            root.insert("activeProfileID".into(), raw(active)?);
        }
        if self.descriptor.make_default {
            let selected = self
                .artifact
                .document()
                .default_profile_id()
                .and_then(|id| mapping.get(&id.to_ascii_lowercase()))
                .unwrap_or(active);
            root.insert("defaultProfileID".into(), raw(selected)?);
        }
        let active_id: String =
            serde_json::from_str(root["activeProfileID"].get()).map_err(|_| invalid())?;
        for (map, key) in [(&keys, "keyBindings"), (&outputs, "outputBindings")] {
            if let Some((_, value)) = map
                .iter()
                .find(|(id, _)| id.eq_ignore_ascii_case(&active_id))
            {
                root.insert(key.into(), value.clone());
            }
        }
        root.insert("profiles".into(), raw(&profiles)?);
        root.insert("profileKeyBindings".into(), raw(&keys)?);
        root.insert("profileOutputBindings".into(), raw(&outputs)?);
        // Incoming portable root extensions merge by exact field name; preserve
        // destination extensions absent from this artifact. Known envelope data
        // and local authority/trust fields never enter this map.
        for (key, value) in imported {
            if ![
                "schema",
                "version",
                "exportedAt",
                "profiles",
                "activeProfileID",
                "defaultProfileID",
                "keyBindings",
                "outputBindings",
                "profileKeyBindings",
                "profileOutputBindings",
            ]
            .contains(&key.as_str())
            {
                root.insert(key, value);
            }
        }
        let original = object(destination.original_bytes())?;
        let changes: Vec<_> = root
            .iter()
            .filter(|(key, value)| original.get(*key).map(|v| v.get()) != Some(value.get()))
            .map(|(key, value)| {
                (
                    format!("/{}", key.replace('~', "~0").replace('/', "~1")),
                    value.get().as_bytes(),
                )
            })
            .collect();
        let edits: Vec<_> = changes
            .iter()
            .map(|(pointer, bytes)| JsonValueEdit {
                pointer,
                value: Some(bytes),
            })
            .collect();
        let document = destination.with_edits(&edits).map_err(|_| invalid())?;
        let outcome = ConfigurationOperationOutcome {
            changed: !changes.is_empty(),
            changed_paths: changes.iter().map(|(p, _)| p.clone()).collect(),
        };
        if outcome.changed_paths.len() > 128 {
            return Err(invalid());
        }
        Ok(Plan {
            document,
            names,
            outcome,
        })
    }
}
