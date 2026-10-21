//! Bounded, configuration-only WoW optical transport. Reading a matching game
//! window is opt-in; these messages never carry executable Lua or host commands.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thumble_core::VirtualGamepadButton;
use uuid::Uuid;

pub const MAX_SNAPSHOT_BYTES: usize = 32768;
pub const FRAME_BYTES: usize = 512;
const HEADER_BYTES: usize = 25;
const PAYLOAD_BYTES: usize = FRAME_BYTES - HEADER_BYTES;
const MAX_PAGES: usize = MAX_SNAPSHOT_BYTES.div_ceil(PAYLOAD_BYTES);
pub const LIVE_TIMEOUT_MS: u64 = 2000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LiveSnapshot {
    pub schema: String,
    pub version: u32,
    pub client: Client,
    pub character: Character,
    pub actions: Vec<Action>,
    pub icons: Vec<u32>,
    pub devices: Vec<Device>,
    pub console_port_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Client {
    pub flavor: String,
    pub interface: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Character {
    pub id: String,
    pub name: String,
    pub realm: String,
    pub class_tag: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Bank {
    Base,
    LeftTrigger,
    RightTrigger,
    BothTriggers,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Action {
    pub bank: Bank,
    pub button: VirtualGamepadButton,
    pub command: String,
    pub label: String,
    pub kind: String,
    pub slot: Option<u32>,
    #[serde(rename = "actionID")]
    pub action_id: Option<u32>,
    #[serde(rename = "iconFileID")]
    pub icon_file_id: Option<u32>,
    pub icon_index: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Device {
    pub name: String,
    #[serde(rename = "vendorID")]
    pub vendor_id: Option<u32>,
    #[serde(rename = "productID")]
    pub product_id: Option<u32>,
}

fn valid_text(value: &str, max_bytes: usize) -> bool {
    !value.is_empty() && value.len() <= max_bytes && !value.chars().any(char::is_control)
}

impl LiveSnapshot {
    pub fn decode(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_SNAPSHOT_BYTES {
            return Err("WoW live snapshot exceeds bound".into());
        }
        let snapshot: Self = thumble_protocol::decode_unique_json(bytes)
            .map_err(|_| "Invalid or ambiguous WoW live snapshot")?;
        snapshot.validate()?;
        Ok(snapshot)
    }

    pub fn validate(&self) -> Result<(), String> {
        let compatible = matches!(
            (self.client.flavor.as_str(), self.client.interface),
            ("classic-era", 11509) | ("forever", 16001)
        );
        if self.schema != "thumble.wow-live" || self.version != 1 || !compatible {
            return Err("Unsupported WoW live schema or client".into());
        }
        if !self.character.id.starts_with("Player-")
            || !self
                .character
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            || !valid_text(&self.character.id, 96)
            || !valid_text(&self.character.name, 96)
            || !valid_text(&self.character.realm, 96)
            || !valid_text(&self.character.class_tag, 32)
            || !valid_text(&self.console_port_version, 32)
            || self.actions.len() > 64
            || self.icons.len() > 64
            || self.devices.len() > 16
        {
            return Err("Invalid WoW character or collection bounds".into());
        }
        let mut icons = BTreeSet::new();
        if self
            .icons
            .iter()
            .any(|id| *id == 0 || *id > i32::MAX as u32 || !icons.insert(*id))
        {
            return Err("Invalid or duplicate WoW icon identity".into());
        }
        let mut actions = BTreeSet::new();
        for action in &self.actions {
            if !actions.insert((action.bank, action.button))
                || !valid_text(&action.command, 128)
                || !valid_text(&action.label, 128)
                || !valid_text(&action.kind, 32)
                || action.slot.is_some_and(|slot| slot == 0 || slot > 180)
                || action
                    .action_id
                    .is_some_and(|id| id == 0 || id > i32::MAX as u32)
                || action
                    .icon_file_id
                    .is_some_and(|id| id == 0 || id > i32::MAX as u32)
            {
                return Err("Invalid or duplicate WoW controller action".into());
            }
            match (action.icon_index, action.icon_file_id) {
                (Some(index), Some(id))
                    if index > 0 && self.icons.get(index as usize - 1) == Some(&id) => {}
                (None, None) => {}
                _ => return Err("WoW action icon does not match captured atlas".into()),
            }
        }
        if self
            .devices
            .iter()
            .any(|device| !valid_text(&device.name, 128))
        {
            return Err("Invalid WoW controller device name".into());
        }
        Ok(())
    }

    /// Renames and class/talent changes retain identity; client flavors do not collide.
    pub fn profile_id(&self) -> Uuid {
        Uuid::new_v5(
            &Uuid::NAMESPACE_URL,
            format!(
                "thumble:wow-live:{}:{}",
                self.client.flavor, self.character.id
            )
            .as_bytes(),
        )
    }
}

pub fn checksum(bytes: &[u8]) -> u32 {
    let (mut a, mut b) = (1_u32, 0_u32);
    for byte in bytes {
        a = (a + u32::from(*byte)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

#[derive(Debug, Clone)]
pub struct Frame<'a> {
    pub tick: u32,
    pub digest: u32,
    pub page: usize,
    pub page_count: usize,
    pub total_bytes: usize,
    pub payload: &'a [u8],
}

impl<'a> Frame<'a> {
    pub fn decode(bytes: &'a [u8]) -> Result<Self, String> {
        if bytes.len() != FRAME_BYTES || &bytes[..5] != b"THWL\x01" {
            return Err("Invalid WoW optical frame header".into());
        }
        let u16_at = |offset| u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as usize;
        let u32_at = |offset| u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
        let (page, page_count, length, total_bytes) =
            (u16_at(13), u16_at(15), u16_at(17), u16_at(19));
        if total_bytes == 0
            || total_bytes > MAX_SNAPSHOT_BYTES
            || page_count == 0
            || page_count > MAX_PAGES
            || page_count != total_bytes.div_ceil(PAYLOAD_BYTES)
            || page >= page_count
            || length != (total_bytes - page * PAYLOAD_BYTES).min(PAYLOAD_BYTES)
            || bytes[HEADER_BYTES + length..].iter().any(|byte| *byte != 0)
        {
            return Err("Invalid WoW optical frame bounds".into());
        }
        let payload = &bytes[HEADER_BYTES..HEADER_BYTES + length];
        if checksum(&[&bytes[..21], payload].concat()) != u32_at(21) {
            return Err("Corrupt WoW optical page".into());
        }
        Ok(Self {
            tick: u32_at(5),
            digest: u32_at(9),
            page,
            page_count,
            total_bytes,
            payload,
        })
    }
}

#[derive(Debug, Default)]
pub struct Reassembler {
    last_tick: Option<u32>,
    last_received_ms: Option<u64>,
    last_advanced_ms: Option<u64>,
    digest: Option<u32>,
    total_bytes: usize,
    page_count: usize,
    pages: BTreeMap<usize, Vec<u8>>,
    delivered: Option<u32>,
}

impl Reassembler {
    pub fn is_live(&self, now_ms: u64) -> bool {
        self.last_advanced_ms
            .is_some_and(|last| now_ms.saturating_sub(last) < LIVE_TIMEOUT_MS)
    }

    /// Only a changing frame tick refreshes liveness; a frozen screenshot cannot.
    pub fn observe(&mut self, bytes: &[u8], now_ms: u64) -> Result<Option<LiveSnapshot>, String> {
        let frame = Frame::decode(bytes)?;
        if self
            .last_received_ms
            .is_some_and(|last| now_ms.saturating_sub(last) >= LIVE_TIMEOUT_MS)
        {
            *self = Self::default();
        }
        if self.last_tick.is_some_and(|last| {
            let delta = frame.tick.wrapping_sub(last);
            delta == 0 || delta >= 1 << 31
        }) {
            return Ok(None);
        }
        // A single frame can be a frozen screenshot. Require two distinct
        // advancing ticks before it becomes a live source, including reloads.
        if self.last_tick.is_some() {
            self.last_advanced_ms = Some(now_ms);
        }
        self.last_tick = Some(frame.tick);
        self.last_received_ms = Some(now_ms);
        if self.digest != Some(frame.digest)
            || self.total_bytes != frame.total_bytes
            || self.page_count != frame.page_count
        {
            self.pages.clear();
            self.digest = Some(frame.digest);
            self.total_bytes = frame.total_bytes;
            self.page_count = frame.page_count;
        }
        if self.delivered == Some(frame.digest) {
            return Ok(None);
        }
        if self
            .pages
            .get(&frame.page)
            .is_some_and(|previous| previous != frame.payload)
        {
            self.pages.clear();
            return Err("Inconsistent WoW optical page".into());
        }
        self.pages.insert(frame.page, frame.payload.to_vec());
        if self.pages.len() != self.page_count || !self.is_live(now_ms) {
            return Ok(None);
        }
        let message: Vec<u8> = self.pages.values().flatten().copied().collect();
        self.pages.clear();
        if message.len() != frame.total_bytes || checksum(&message) != frame.digest {
            return Err("Corrupt WoW optical snapshot".into());
        }
        let snapshot = LiveSnapshot::decode(&message)?;
        self.delivered = Some(frame.digest);
        Ok(Some(snapshot))
    }
}

/// A validated update is an intent, not a disk write. The runtime must commit it
/// through its normal revision-checked authority, then acknowledge the receipt.
#[derive(Debug, Clone)]
pub struct ProfileUpdate {
    pub receipt: u64,
    pub profile_id: Uuid,
    pub select: bool,
    pub snapshot: LiveSnapshot,
    pub atlas_png: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct Observer {
    window_id: Option<u32>,
    decoder: Reassembler,
    acknowledged_character: Option<Uuid>,
    pending: Option<ProfileUpdate>,
    next_receipt: u64,
}

impl Observer {
    /// A failed capture or hidden panel must expire normally; it cannot refresh
    /// liveness by replaying the last good frame.
    pub fn is_live(&self, now_ms: u64) -> bool {
        self.decoder.is_live(now_ms)
    }

    pub fn pending(&self, now_ms: u64) -> Option<&ProfileUpdate> {
        self.decoder
            .is_live(now_ms)
            .then_some(self.pending.as_ref())
            .flatten()
    }

    pub fn observe(
        &mut self,
        window_id: u32,
        frame: &[u8],
        atlas_png: &[u8],
        now_ms: u64,
    ) -> Result<(), String> {
        // Image decoding/cropping happens in the native helper. Enforce the wire
        // size and format here before retaining even a partial observation.
        if window_id == 0
            || atlas_png.len() > 2 * 1024 * 1024
            || !atlas_png.starts_with(b"\x89PNG\r\n\x1a\n")
        {
            return Err("Invalid WoW capture source or atlas".into());
        }
        Frame::decode(frame)?;
        if self.window_id != Some(window_id) {
            self.window_id = Some(window_id);
            self.decoder = Reassembler::default();
            self.pending = None;
        }
        let old_digest = self.decoder.digest;
        let was_live = self.decoder.is_live(now_ms);
        let snapshot = self.decoder.observe(frame, now_ms)?;
        if !was_live || old_digest != self.decoder.digest {
            // A newer incomplete character/action snapshot supersedes older work.
            self.pending = None;
        }
        if let Some(snapshot) = snapshot {
            self.next_receipt = self.next_receipt.wrapping_add(1);
            let profile_id = snapshot.profile_id();
            self.pending = Some(ProfileUpdate {
                receipt: self.next_receipt,
                profile_id,
                select: self.acknowledged_character != Some(profile_id),
                snapshot,
                atlas_png: atlas_png.to_vec(),
            });
        }
        Ok(())
    }

    /// Call only after a successful authority commit. Revision conflicts retain
    /// the pending update so it can be replanned against the new configuration.
    pub fn acknowledge(&mut self, receipt: u64, now_ms: u64) -> bool {
        if self
            .pending(now_ms)
            .is_none_or(|update| update.receipt != receipt)
        {
            return false;
        }
        self.acknowledged_character = self.pending.take().map(|update| update.profile_id);
        true
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum SyncAction {
    Status,
    Start,
    Stop,
    RequestPermission,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub enabled: bool,
    pub phase: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
    #[serde(skip)]
    pub generation: u64,
}
impl Default for SyncStatus {
    fn default() -> Self {
        Self {
            enabled: false,
            phase: "disabled".into(),
            profile_id: None,
            character_name: None,
            last_error: None,
            generation: 0,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CaptureReply {
    pub status: String,
    #[serde(rename = "windowID")]
    pub window_id: Option<u32>,
    pub frame_base64: Option<String>,
    #[serde(rename = "atlasPNGBase64")]
    pub atlas_png_base64: Option<String>,
}

pub(crate) fn capture(
    bridge: &crate::bridge::ConfigurationBridge,
    command: &str,
) -> Result<CaptureReply, String> {
    let mut request = serde_json::to_vec(&serde_json::json!({"wowLiveCommand":command}))
        .map_err(|e| e.to_string())?;
    request.push(b'\n');
    let bytes = bridge
        .execute_with_limits(request, 1024, 3 * 1024 * 1024)
        .map_err(|e| e.to_string())?;
    let reply: CaptureReply =
        thumble_protocol::decode_unique_json(&bytes).map_err(|_| "Invalid WoW capture reply")?;
    if ![
        "authorized",
        "permission-required",
        "game-not-visible",
        "multiple-game-windows",
        "panel-not-visible",
        "frame",
    ]
    .contains(&reply.status.as_str())
    {
        return Err("Invalid WoW capture status".into());
    }
    Ok(reply)
}

/// Correlated native projections. The raw owned document remains authoritative.
pub(crate) struct PreparedProfile {
    id: String,
    before_profile: Option<serde_json::Value>,
    document: thumble_core::ConfigurationDocument,
}

pub(crate) fn prepare_profile(
    bridge: &crate::bridge::ConfigurationBridge,
    update: &ProfileUpdate,
    current: &thumble_core::OwnedConfigurationDocument,
) -> Result<PreparedProfile, String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde_json::{value::RawValue, Value};
    use sha2::{Digest, Sha256};
    let id = update.profile_id.to_string();
    let root: BTreeMap<String, &RawValue> =
        serde_json::from_slice(current.original_bytes()).map_err(|e| e.to_string())?;
    let profiles: Vec<&RawValue> =
        serde_json::from_str(root["profiles"].get()).map_err(|e| e.to_string())?;
    let index = current.profiles().iter().position(|profile| {
        profile["id"]
            .as_str()
            .is_some_and(|v| v.eq_ignore_ascii_case(&id))
    });
    let existing = index.map(|i| profiles[i].get());
    let mut bindings = serde_json::Map::new();
    if existing.is_some() {
        for (field, output) in [
            ("profileKeyBindings", "keyBindings"),
            ("profileOutputBindings", "outputBindings"),
        ] {
            if let Some(raw) = root.get(field) {
                let maps: BTreeMap<String, &RawValue> =
                    serde_json::from_str(raw.get()).map_err(|e| e.to_string())?;
                if let Some((_, raw)) = maps.iter().find(|(key, _)| key.eq_ignore_ascii_case(&id)) {
                    bindings.insert(
                        output.into(),
                        serde_json::from_str(raw.get()).map_err(|e| e.to_string())?,
                    );
                }
            }
        }
    }
    let mut request = serde_json::to_vec(&serde_json::json!({
        "schemaVersion":2, "wowProfileCommand":"prepare-owned", "profileID":update.profile_id,
        "snapshotJSON":serde_json::to_string(&update.snapshot).map_err(|e| e.to_string())?,
        "atlasPNGBase64":STANDARD.encode(&update.atlas_png), "existingProfileJSON":existing,
        "existingBindingsJSON":existing.map(|_| Value::Object(bindings).to_string()),
    }))
    .map_err(|e| e.to_string())?;
    request.push(b'\n');
    let response = bridge
        .execute_with_limits(request, 12 * 1024 * 1024, 16 * 1024 * 1024)
        .map_err(|e| e.to_string())?;
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    struct Reply {
        schema_version: u32,
        #[serde(rename = "sourceSHA256")]
        source_sha256: Option<String>,
        before_profile: Option<Value>,
        document: thumble_core::ConfigurationDocument,
    }
    let reply: Reply = thumble_protocol::decode_unique_json(&response)
        .map_err(|_| "Invalid WoW profile preparation reply")?;
    let expected = existing.map(|raw| format!("{:x}", Sha256::digest(raw.as_bytes())));
    if reply.schema_version != 2
        || reply.source_sha256 != expected
        || reply.before_profile.is_some() != existing.is_some()
        || reply.document.profiles.len() != 1
        || !reply.document.active_profile_id.eq_ignore_ascii_case(&id)
        || !reply.document.default_profile_id.eq_ignore_ascii_case(&id)
        || reply.document.profiles[0]["id"]
            .as_str()
            .is_none_or(|v| !v.eq_ignore_ascii_case(&id))
        || reply.document.profiles[0]["outputMode"] != "controller"
        || reply.document.profiles[0]["wowLive"]["characterID"] != update.snapshot.character.id
        || reply.document.profiles[0]["wowLive"]["flavor"] != update.snapshot.client.flavor
    {
        return Err("WoW profile preparation returned an uncorrelated profile".into());
    }
    let mut envelope = serde_json::to_value(&reply.document).map_err(|e| e.to_string())?;
    envelope["schema"] = Value::String("com.codybontecou.pocketpad.keypad-configuration".into());
    envelope["version"] = Value::from(5);
    thumble_core::OwnedConfigurationDocument::decode(
        &serde_json::to_vec(&envelope).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(PreparedProfile {
        id,
        before_profile: reply.before_profile,
        document: reply.document,
    })
}

impl PreparedProfile {
    pub(crate) fn merge(
        &self,
        current: &thumble_core::OwnedConfigurationDocument,
        select: bool,
    ) -> Result<thumble_core::OwnedConfigurationDocument, String> {
        use serde_json::Value;
        use thumble_core::{ConfigurationDocument, OwnedConfigurationDocument};
        let mut before: Value =
            serde_json::from_slice(current.original_bytes()).map_err(|e| e.to_string())?;
        let mut after = before.clone();
        let index = current.profiles().iter().position(|p| {
            p["id"]
                .as_str()
                .is_some_and(|v| v.eq_ignore_ascii_case(&self.id))
        });
        match (index, &self.before_profile) {
            (Some(index), Some(profile)) => {
                before["profiles"][index] = profile.clone();
                after["profiles"][index] = self.document.profiles[0].clone();
            }
            (None, None) => {
                after["profiles"]
                    .as_array_mut()
                    .ok_or("Missing profiles")?
                    .push(self.document.profiles[0].clone());
                for (field, map) in [
                    (
                        "profileKeyBindings",
                        serde_json::to_value(&self.document.profile_key_bindings)
                            .map_err(|e| e.to_string())?,
                    ),
                    (
                        "profileOutputBindings",
                        serde_json::to_value(&self.document.profile_output_bindings)
                            .map_err(|e| e.to_string())?,
                    ),
                ] {
                    let value = map
                        .as_object()
                        .ok_or("Invalid new WoW bindings")?
                        .iter()
                        .find(|(id, _)| id.eq_ignore_ascii_case(&self.id))
                        .map(|(_, v)| serde_json::to_value(v))
                        .transpose()
                        .map_err(|e| e.to_string())?
                        .ok_or("Missing new WoW bindings")?;
                    let root = after.as_object_mut().ok_or("Missing document")?;
                    root.entry(field)
                        .or_insert_with(|| serde_json::json!({}))
                        .as_object_mut()
                        .ok_or("Invalid binding map")?
                        .insert(self.id.clone(), value);
                }
            }
            _ => return Err("WoW source changed during preparation".into()),
        }
        if select {
            after["activeProfileID"] = index
                .map(|i| current.profiles()[i]["id"].clone())
                .unwrap_or_else(|| Value::String(self.id.clone()));
            let mut mirrors: ConfigurationDocument = serde_json::from_value(Value::Object(
                after
                    .as_object()
                    .ok_or("Missing document")?
                    .iter()
                    .filter(|(key, _)| {
                        [
                            "profiles",
                            "activeProfileID",
                            "defaultProfileID",
                            "keyBindings",
                            "outputBindings",
                            "profileKeyBindings",
                            "profileOutputBindings",
                        ]
                        .contains(&key.as_str())
                    })
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            ))
            .map_err(|e| e.to_string())?;
            mirrors
                .refresh_global_binding_mirrors()
                .map_err(|e| e.to_string())?;
            after["keyBindings"] =
                serde_json::to_value(mirrors.key_bindings).map_err(|e| e.to_string())?;
            after["outputBindings"] =
                serde_json::to_value(mirrors.output_bindings).map_err(|e| e.to_string())?;
        }
        let before = OwnedConfigurationDocument::decode(
            &serde_json::to_vec(&before).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        let after = OwnedConfigurationDocument::decode(
            &serde_json::to_vec(&after).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        current
            .with_projected_changes(&before, &after)
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn seal(frame: &mut [u8]) {
        let length = u16::from_le_bytes([frame[17], frame[18]]) as usize;
        let digest = checksum(&[&frame[..21], &frame[25..25 + length]].concat());
        frame[21..25].copy_from_slice(&digest.to_le_bytes());
    }

    fn fixture() -> (LiveSnapshot, Vec<Vec<u8>>) {
        let v: Value =
            serde_json::from_str(include_str!("../../../fixtures/wow-live/v1.json")).unwrap();
        let snapshot = LiveSnapshot::decode(&serde_json::to_vec(&v["snapshot"]).unwrap()).unwrap();
        let frames = v["frames"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .unwrap()
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect()
            })
            .collect();
        (snapshot, frames)
    }

    #[test]
    fn actual_lua_pages_reassemble_to_valid_character_and_bank_icons() {
        let (snapshot, mut frames) = fixture();
        let mut decoder = Reassembler::default();
        let mut result = None;
        for (index, frame) in frames.iter_mut().enumerate() {
            frame[5..9].copy_from_slice(&(index as u32 + 1).to_le_bytes());
            seal(frame);
            result = decoder
                .observe(frame, index as u64 * 200)
                .unwrap()
                .or(result);
        }
        assert_eq!(result, Some(snapshot));
        assert!(decoder.is_live(1000));
        assert!(!decoder.is_live(10000));
    }

    #[test]
    fn corruption_oversize_and_mixed_pages_never_become_profiles() {
        let (_, frames) = fixture();
        for position in [0, 4, 17, 19, 21, 100] {
            let mut corrupt = frames[0].clone();
            corrupt[position] ^= 0xff;
            assert!(Frame::decode(&corrupt).is_err(), "position {position}");
        }
        assert!(LiveSnapshot::decode(&vec![b' '; MAX_SNAPSHOT_BYTES + 1]).is_err());
        let mut decoder = Reassembler::default();
        let mut altered = frames[1].clone();
        altered[5..9].copy_from_slice(&124_u32.to_le_bytes());
        altered[9] ^= 0x80;
        seal(&mut altered);
        assert!(decoder.observe(&frames[0], 0).unwrap().is_none());
        assert!(decoder.observe(&altered, 200).unwrap().is_none());
    }

    #[test]
    fn frozen_or_backward_frames_do_not_refresh_liveness() {
        let (_, frames) = fixture();
        let mut decoder = Reassembler::default();
        decoder.observe(&frames[0], 0).unwrap();
        decoder.observe(&frames[0], 1000).unwrap();
        assert!(!decoder.is_live(2000));
        // Addon reload restarts tick; once stale a new session is admissible.
        let mut restarted = frames[0].clone();
        restarted[5..9].copy_from_slice(&1_u32.to_le_bytes());
        seal(&mut restarted);
        decoder.observe(&restarted, 2001).unwrap();
        assert!(!decoder.is_live(2001));
        restarted[5..9].copy_from_slice(&2_u32.to_le_bytes());
        seal(&mut restarted);
        decoder.observe(&restarted, 2002).unwrap();
        assert!(decoder.is_live(2002));
    }

    #[test]
    fn identities_are_character_and_flavor_scoped_and_atlas_is_checked() {
        let (mut snapshot, _) = fixture();
        let id = snapshot.profile_id();
        snapshot.character.name = "Renamed".into();
        assert_eq!(snapshot.profile_id(), id);
        snapshot.character.id = "Player-1234-00000002".into();
        assert_ne!(snapshot.profile_id(), id);
        snapshot.actions[1].icon_index = Some(64);
        assert!(snapshot.validate().is_err());
    }

    #[test]
    fn duplicate_json_keys_and_duplicate_controls_are_rejected() {
        assert!(LiveSnapshot::decode(br#"{"schema":"a","schema":"b"}"#).is_err());
        let (mut snapshot, _) = fixture();
        snapshot.actions.push(snapshot.actions[0].clone());
        assert!(snapshot.validate().is_err());
    }

    fn send_snapshot(
        observer: &mut Observer,
        snapshot: &LiveSnapshot,
        tick: &mut u32,
        now: &mut u64,
    ) {
        let message = serde_json::to_vec(snapshot).unwrap();
        let digest = checksum(&message);
        for (page, payload) in message.chunks(PAYLOAD_BYTES).enumerate() {
            *tick += 1;
            *now += 200;
            let mut bytes = vec![0; FRAME_BYTES];
            bytes[..5].copy_from_slice(b"THWL\x01");
            bytes[5..9].copy_from_slice(&tick.to_le_bytes());
            bytes[9..13].copy_from_slice(&digest.to_le_bytes());
            bytes[13..15].copy_from_slice(&(page as u16).to_le_bytes());
            bytes[15..17]
                .copy_from_slice(&(message.len().div_ceil(PAYLOAD_BYTES) as u16).to_le_bytes());
            bytes[17..19].copy_from_slice(&(payload.len() as u16).to_le_bytes());
            bytes[19..21].copy_from_slice(&(message.len() as u16).to_le_bytes());
            bytes[25..25 + payload.len()].copy_from_slice(payload);
            seal(&mut bytes);
            observer
                .observe(42, &bytes, b"\x89PNG\r\n\x1a\nfixture", *now)
                .unwrap();
        }
    }

    #[test]
    fn swaps_follow_character_guid_and_same_character_refresh_does_not_steal_manual_selection() {
        let (mut snapshot, _) = fixture();
        let mut observer = Observer::default();
        let (mut tick, mut now) = (0, 0);
        send_snapshot(&mut observer, &snapshot, &mut tick, &mut now);
        let first = observer.pending(now).unwrap().clone();
        assert!(first.select);
        assert!(observer.acknowledge(first.receipt, now));
        snapshot.actions[0].label = "Changed spell".into();
        send_snapshot(&mut observer, &snapshot, &mut tick, &mut now);
        let refresh = observer.pending(now).unwrap().clone();
        assert!(!refresh.select);
        assert_eq!(refresh.profile_id, first.profile_id);
        assert!(observer.acknowledge(refresh.receipt, now));
        snapshot.character.id = "Player-1234-00000002".into();
        send_snapshot(&mut observer, &snapshot, &mut tick, &mut now);
        let second = observer.pending(now).unwrap().clone();
        assert!(second.select);
        assert_ne!(second.profile_id, first.profile_id);
        assert!(!observer.acknowledge(first.receipt, now));
        assert!(observer.acknowledge(second.receipt, now));
        snapshot.character.id = first.snapshot.character.id;
        send_snapshot(&mut observer, &snapshot, &mut tick, &mut now);
        assert_eq!(observer.pending(now).unwrap().profile_id, first.profile_id);
        assert!(observer.pending(now).unwrap().select);
    }

    #[test]
    fn uncommitted_updates_retry_but_expire_when_capture_freezes() {
        let (snapshot, _) = fixture();
        let mut observer = Observer::default();
        let (mut tick, mut now) = (0, 0);
        send_snapshot(&mut observer, &snapshot, &mut tick, &mut now);
        let update = observer.pending(now).unwrap().clone();
        assert_eq!(observer.pending(now + 100).unwrap().receipt, update.receipt);
        assert!(observer.pending(now + LIVE_TIMEOUT_MS).is_none());
        assert!(!observer.acknowledge(update.receipt, now + LIVE_TIMEOUT_MS));
    }
    #[test]
    fn a_frozen_single_page_never_publishes_even_after_repeated_expiry() {
        let (mut snapshot, _) = fixture();
        snapshot.actions.clear();
        snapshot.icons.clear();
        snapshot.devices.clear();
        let message = serde_json::to_vec(&snapshot).unwrap();
        assert!(message.len() <= PAYLOAD_BYTES);
        let mut frame = vec![0; FRAME_BYTES];
        frame[..5].copy_from_slice(b"THWL\x01");
        frame[5..9].copy_from_slice(&1_u32.to_le_bytes());
        frame[9..13].copy_from_slice(&checksum(&message).to_le_bytes());
        frame[15..17].copy_from_slice(&1_u16.to_le_bytes());
        frame[17..19].copy_from_slice(&(message.len() as u16).to_le_bytes());
        frame[19..21].copy_from_slice(&(message.len() as u16).to_le_bytes());
        frame[25..25 + message.len()].copy_from_slice(&message);
        seal(&mut frame);
        let mut decoder = Reassembler::default();
        for now in [0, 100, 1000, 2000, 2100, 4000, 10000] {
            assert!(decoder.observe(&frame, now).unwrap().is_none());
            assert!(!decoder.is_live(now));
        }
        frame[5..9].copy_from_slice(&2_u32.to_le_bytes());
        seal(&mut frame);
        assert_eq!(decoder.observe(&frame, 10001).unwrap(), Some(snapshot));
    }
}
