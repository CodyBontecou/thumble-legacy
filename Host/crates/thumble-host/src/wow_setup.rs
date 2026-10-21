//! Agent-facing addon installation. It does not edit executable WoW saved Lua
//! or the receiver's profile catalog; in-game setup uses a declarative opt-in.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};
use uuid::Uuid;
// ConsolePort 3.3.11 expands to about 47 MiB including its icon/texture assets.
const MAX_BYTES: usize = 64 * 1024 * 1024;
const MAX_FILES: usize = 4096;
const ADDONS: &[&str] = &[
    "ConsolePort",
    "ConsolePort_Bar",
    "ConsolePort_Config",
    "ConsolePort_Cursor",
    "ConsolePort_Keyboard",
    "ConsolePort_Menu",
    "ConsolePort_Rings",
    "ConsolePort_Target",
    "ConsolePort_World",
];
pub const CONSOLEPORT_VERSION: &str = "3.3.11";
type Tree = BTreeMap<String, Vec<u8>>;

#[derive(Debug, Clone, Serialize, Deserialize, clap::Args)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetupRequest {
    #[arg(long)]
    pub client_root: PathBuf,
    #[arg(long)]
    #[serde(rename = "consolePortSource")]
    pub consoleport_source: PathBuf,
    #[serde(default)]
    #[arg(long)]
    pub replace_bindings: bool,
    #[serde(default)]
    #[arg(long)]
    pub replace_addons: bool,
    #[serde(default = "enabled")]
    #[arg(long = "no-live", action = clap::ArgAction::SetFalse, default_value_t = true)]
    pub enable_live: bool,
}
fn enabled() -> bool {
    true
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddonChange {
    pub name: String,
    #[serde(rename = "beforeSHA256")]
    pub before_sha256: Option<String>,
    #[serde(rename = "afterSHA256")]
    pub after_sha256: String,
    pub changed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupPlan {
    pub client_root: PathBuf,
    pub flavor: String,
    #[serde(rename = "consolePortVersion")]
    pub consoleport_version: String,
    pub source_root: PathBuf,
    pub replace_bindings: bool,
    pub replace_addons: bool,
    pub enable_live: bool,
    pub game_running: bool,
    pub addons: Vec<AddonChange>,
    #[serde(rename = "settingsBackupSHA256")]
    pub settings_backup_sha256: String,
    pub settings_backup_files: usize,
    pub conflicts: Vec<String>,
    #[serde(rename = "planSHA256")]
    pub plan_sha256: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupResult {
    pub plan: SetupPlan,
    pub applied: bool,
    pub changed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_directory: Option<PathBuf>,
}
#[derive(Debug, clap::Args)]
pub struct SetupArgs {
    #[command(flatten)]
    pub request: SetupRequest,
    #[arg(long, requires = "expected_plan_sha256")]
    pub apply: bool,
    #[arg(long, requires = "apply")]
    pub expected_plan_sha256: Option<String>,
    #[arg(long)]
    pub json: bool,
}
impl SetupArgs {
    pub fn run(&self) -> Result<(), String> {
        if self.apply != self.expected_plan_sha256.is_some() {
            return Err(
                "Apply requires an exact expected plan SHA-256; omit both flags for planning"
                    .into(),
            );
        }
        let result = execute(&self.request, self.expected_plan_sha256.as_deref())?;
        let text = if self.json {
            serde_json::to_string(&result)
        } else {
            serde_json::to_string_pretty(&result)
        }
        .map_err(|e| e.to_string())?;
        println!("{text}");
        Ok(())
    }
}

struct Prepared {
    plan: SetupPlan,
    desired: BTreeMap<String, Tree>,
    before: BTreeMap<String, Tree>,
    settings: Tree,
}

pub fn execute(request: &SetupRequest, expected_plan: Option<&str>) -> Result<SetupResult, String> {
    execute_with(request, expected_plan, |_| Ok(()))
}
fn execute_with<F: FnMut(usize) -> Result<(), String>>(
    request: &SetupRequest,
    expected: Option<&str>,
    mut hook: F,
) -> Result<SetupResult, String> {
    let prepared = prepare(request)?;
    let changed = prepared.plan.addons.iter().any(|addon| addon.changed);
    let Some(expected) = expected else {
        return Ok(SetupResult {
            plan: prepared.plan,
            applied: false,
            changed,
            backup_directory: None,
        });
    };
    if expected != prepared.plan.plan_sha256 {
        return Err("WoW setup plan changed; inspect a fresh plan before applying".into());
    }
    if !prepared.plan.conflicts.is_empty() {
        return Err("Existing ConsolePort files differ; inspect the plan and explicitly enable replaceAddons to replace them with backups".into());
    }
    if !changed {
        return Ok(SetupResult {
            plan: prepared.plan,
            applied: true,
            changed: false,
            backup_directory: None,
        });
    }
    if prepared.plan.game_running {
        return Err("Close this WoW client before applying addon changes".into());
    }
    let interface = prepared.plan.client_root.join("Interface");
    let addons = interface.join("AddOns");
    ensure_real_directory(&interface)?;
    ensure_real_directory(&addons)?;
    let lease = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(interface.join(".thumble-setup.lock"))
        .map_err(io_error)?;
    fs2::FileExt::try_lock_exclusive(&lease)
        .map_err(|_| "Another Thumble setup is modifying this client")?;
    let _lease = lease;
    let id = Uuid::new_v4().to_string();
    let stage = interface.join(format!(".thumble-stage-{id}"));
    private_directory(&stage)?;
    let backups = interface.join("ThumbleBackups");
    ensure_real_directory(&backups)?;
    use std::os::unix::fs::MetadataExt;
    let metadata = fs::symlink_metadata(&backups).map_err(io_error)?;
    if metadata.uid() != unsafe { libc::geteuid() } || metadata.permissions().mode() & 0o077 != 0 {
        return Err("ThumbleBackups must be a private directory owned by the current user".into());
    }
    let backup = backups.join(&id);
    private_directory(&backup)?;
    let mut installed: Vec<(String, bool, bool)> = vec![];
    let operation = (|| -> Result<(), String> {
        for change in prepared.plan.addons.iter().filter(|a| a.changed) {
            write_tree(&stage.join(&change.name), &prepared.desired[&change.name])?;
            make_addon_readable(&stage.join(&change.name))?;
            if let Some(tree) = prepared.before.get(&change.name) {
                write_tree(&backup.join("AddOns").join(&change.name), tree)?;
            }
        }
        write_tree(&backup.join("WTF"), &prepared.settings)?;
        write_private(
            &backup.join("plan.json"),
            &serde_json::to_vec_pretty(&prepared.plan).map_err(|e| e.to_string())?,
        )?;
        write_private(&backup.join("phase"), b"prepared\n")?;
        sync_directory(&backup)?;
        sync_directory(&backups)?;
        sync_directory(&interface)?;
        let fresh = prepare(request)?;
        if fresh.plan.plan_sha256 != expected {
            return Err("WoW files changed while preparing backups; no addon was replaced".into());
        }
        for (index, change) in prepared
            .plan
            .addons
            .iter()
            .filter(|a| a.changed)
            .enumerate()
        {
            hook(index)?;
            if game_running(&prepared.plan.client_root)? {
                return Err("WoW started during installation; restoring previous addons".into());
            }
            let target = addons.join(&change.name);
            if tree_hash_if_exists(&target)? != change.before_sha256 {
                return Err(
                    "An addon changed during installation; restoring previous addons".into(),
                );
            }
            let had_before = change.before_sha256.is_some();
            if had_before {
                fs::rename(&target, stage.join(format!("old-{}", change.name)))
                    .map_err(io_error)?;
            }
            installed.push((change.name.clone(), had_before, false));
            fs::rename(stage.join(&change.name), &target).map_err(io_error)?;
            installed.last_mut().expect("installation entry exists").2 = true;
            sync_directory(&addons)?;
            sync_directory(&stage)?;
        }
        // Commit receipt follows all durable directory replacements.
        fs::write(backup.join("phase"), b"committed\n").map_err(io_error)?;
        File::open(backup.join("phase"))
            .map_err(io_error)?
            .sync_all()
            .map_err(io_error)?;
        sync_directory(&backup)?;
        Ok(())
    })();
    if let Err(error) = operation {
        for (name, had_before, new_installed) in installed.iter().rev() {
            let target = addons.join(name);
            let removed = stage.join(format!("failed-{name}"));
            if target.exists() {
                if !new_installed
                    || tree_hash_if_exists(&target)? != Some(hash_tree(&prepared.desired[name]))
                {
                    return Err(format!(
                        "{error}; an addon changed after installation; recovery required using {}",
                        backup.display()
                    ));
                }
                fs::rename(&target, &removed).map_err(|_| {
                    format!("{error}; recovery required using {}", backup.display())
                })?;
            }
            if *had_before {
                fs::rename(stage.join(format!("old-{name}")), &target).map_err(|_| {
                    format!("{error}; recovery required using {}", backup.display())
                })?;
            }
        }
        sync_directory(&addons)?;
        let _ = fs::write(backup.join("phase"), b"rolled-back\n");
        let _ = fs::remove_dir_all(&stage);
        return Err(format!("{error}; backup retained at {}", backup.display()));
    }
    fs::remove_dir_all(&stage).map_err(io_error)?;
    Ok(SetupResult {
        plan: prepared.plan,
        applied: true,
        changed: true,
        backup_directory: Some(backup),
    })
}
fn prepare(request: &SetupRequest) -> Result<Prepared, String> {
    let client = fs::canonicalize(&request.client_root).map_err(io_error)?;
    let flavor = match client.file_name().and_then(|s| s.to_str()) {
        Some("_classic_era_") => "classic-era",
        Some("_forever_") => "forever",
        _ => return Err("Select the _classic_era_ or _forever_ client directory".into()),
    };
    let interface = client.join("Interface");
    let destination = interface.join("AddOns");
    check_directory_if_exists(&interface)?;
    check_directory_if_exists(&destination)?;
    let source = fs::canonicalize(&request.consoleport_source).map_err(io_error)?;
    let mut desired = BTreeMap::new();
    let mut before = BTreeMap::new();
    let mut changes = vec![];
    let mut conflicts = vec![];
    let mut total = 0;
    let mut old_bytes = 0;
    let mut old_files = 0;
    let mut file_count = 0;
    for name in ADDONS.iter().copied().chain(std::iter::once("ThumbleLink")) {
        let tree = if name == "ThumbleLink" {
            link_tree(request)
        } else {
            let tree = read_tree(&source.join(name), false)?;
            let toc = tree.get(&format!("{name}.toc")).ok_or_else(|| {
                format!("Missing {name}/{name}.toc in extracted ConsolePort source")
            })?;
            let toc = std::str::from_utf8(toc).map_err(|_| "ConsolePort TOC is not UTF-8")?;
            let interface = if flavor == "classic-era" {
                "11509"
            } else {
                "16001"
            };
            let version_ok = if name == "ConsolePort" {
                toc_value(toc, "Version") == Some(CONSOLEPORT_VERSION)
            } else {
                toc_value(toc, "RequiredDeps")
                    .is_some_and(|deps| deps.split(',').any(|dep| dep.trim() == "ConsolePort"))
            };
            if !version_ok
                || !toc_value(toc, "Interface")
                    .is_some_and(|versions| versions.split(',').any(|v| v.trim() == interface))
            {
                return Err(format!(
                    "{name} must be ConsolePort {CONSOLEPORT_VERSION} with interface {interface}"
                ));
            }
            tree
        };
        total += tree.values().map(Vec::len).sum::<usize>();
        file_count += tree.len();
        if total > MAX_BYTES || file_count > MAX_FILES {
            return Err("Addon source exceeds installation bounds".into());
        }
        let old = if destination.join(name).exists()
            || fs::symlink_metadata(destination.join(name)).is_ok()
        {
            Some(read_tree(&destination.join(name), false)?)
        } else {
            None
        };
        let old_hash = old.as_ref().map(hash_tree);
        let after_sha256 = hash_tree(&tree);
        let changed = old_hash.as_ref() != Some(&after_sha256);
        if changed && old.is_some() && name != "ThumbleLink" && !request.replace_addons {
            conflicts.push(name.to_owned());
        }
        if let Some(old) = old {
            old_bytes += old.values().map(Vec::len).sum::<usize>();
            old_files += old.len();
            if old_bytes > MAX_BYTES || old_files > MAX_FILES {
                return Err("Existing addon backups exceed installation bounds".into());
            }
            before.insert(name.into(), old);
        }
        desired.insert(name.into(), tree);
        changes.push(AddonChange {
            name: name.into(),
            before_sha256: old_hash,
            after_sha256,
            changed,
        });
    }
    let settings = if changes.iter().any(|change| change.changed) {
        read_settings(&client.join("WTF"))?
    } else {
        Tree::new()
    };
    let mut plan = SetupPlan {
        client_root: client.clone(),
        flavor: flavor.into(),
        consoleport_version: CONSOLEPORT_VERSION.into(),
        source_root: source,
        replace_bindings: request.replace_bindings,
        replace_addons: request.replace_addons,
        enable_live: request.enable_live,
        game_running: game_running(&client)?,
        addons: changes,
        settings_backup_sha256: hash_tree(&settings),
        settings_backup_files: settings.len(),
        conflicts,
        plan_sha256: String::new(),
    };
    plan.plan_sha256 = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&plan).map_err(|e| e.to_string())?)
    );
    Ok(Prepared {
        plan,
        desired,
        before,
        settings,
    })
}
fn link_tree(request: &SetupRequest) -> Tree {
    let files: &[(&str, &[u8])] = &[
        (
            "ThumbleLink.toc",
            include_bytes!("../../../../Addons/ThumbleLink/ThumbleLink.toc"),
        ),
        (
            "ThumbleLink.lua",
            include_bytes!("../../../../Addons/ThumbleLink/ThumbleLink.lua"),
        ),
        (
            "ThumbleLinkLive.lua",
            include_bytes!("../../../../Addons/ThumbleLink/ThumbleLinkLive.lua"),
        ),
        (
            "ThumbleLinkOnboarding.lua",
            include_bytes!("../../../../Addons/ThumbleLink/ThumbleLinkOnboarding.lua"),
        ),
    ];
    let mut tree: Tree = files
        .iter()
        .map(|(name, data)| (name.to_string(), data.to_vec()))
        .collect();
    tree.insert("ThumbleLinkSetup.lua".into(), format!("-- Generated declarative ConsolePort onboarding; no receiver credentials.\nThumbleLinkSetup = {{ version = 1, enabled = true, replaceBindings = {}, enableLive = {} }}\n", request.replace_bindings, request.enable_live).into_bytes());
    tree
}
fn toc_value<'a>(text: &'a str, field: &str) -> Option<&'a str> {
    text.lines().find_map(|line| {
        line.trim()
            .strip_prefix("## ")?
            .split_once(':')
            .filter(|(key, _)| *key == field)
            .map(|(_, value)| value.trim())
    })
}
fn hash_tree(tree: &Tree) -> String {
    let mut hash = Sha256::new();
    for (name, bytes) in tree {
        hash.update((name.len() as u64).to_le_bytes());
        hash.update(name.as_bytes());
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    format!("{hash:x}", hash = hash.finalize())
}
fn tree_hash_if_exists(path: &Path) -> Result<Option<String>, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(Some(hash_tree(&read_tree(path, false)?))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(io_error(e)),
    }
}
fn read_tree(root: &Path, settings_only: bool) -> Result<Tree, String> {
    check_directory_if_exists(root)?;
    if !root.is_dir() {
        return Err(format!("Missing directory {}", root.display()));
    }
    let mut tree = Tree::new();
    let mut pending = vec![root.to_path_buf()];
    let mut nodes = 0;
    let mut bytes = 0;
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(directory).map_err(io_error)? {
            let entry = entry.map_err(io_error)?;
            let path = entry.path();
            nodes += 1;
            if nodes > 20000 {
                return Err("Addon/settings tree exceeds directory bounds".into());
            }
            let metadata = fs::symlink_metadata(&path).map_err(io_error)?;
            if metadata.file_type().is_symlink() {
                return Err(format!(
                    "Symlink requires explicit manual handling: {}",
                    path.display()
                ));
            }
            if metadata.is_dir() {
                pending.push(path);
                continue;
            }
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .ok_or("Non-UTF-8 addon filename")?;
            if settings_only
                && !(name == "bindings-cache.wtf"
                    || ((name.starts_with("ConsolePort") || name.starts_with("ThumbleLink"))
                        && (name.ends_with(".lua") || name.ends_with(".lua.bak"))))
            {
                continue;
            }
            if !metadata.is_file() || metadata.len() > 8 * 1024 * 1024 {
                return Err("Invalid or oversized addon/settings file".into());
            }
            let mut data = vec![];
            OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW)
                .open(&path)
                .map_err(io_error)?
                .take(8 * 1024 * 1024 + 1)
                .read_to_end(&mut data)
                .map_err(io_error)?;
            bytes += data.len();
            if data.len() > 8 * 1024 * 1024 || bytes > MAX_BYTES || tree.len() >= MAX_FILES {
                return Err("Addon/settings content exceeds bounds".into());
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| "Invalid addon path")?
                .to_str()
                .ok_or("Non-UTF-8 addon path")?
                .to_owned();
            tree.insert(relative, data);
        }
    }
    Ok(tree)
}
fn read_settings(root: &Path) -> Result<Tree, String> {
    match fs::symlink_metadata(root) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Tree::new()),
        Err(error) => Err(io_error(error)),
        Ok(_) => read_tree(root, true),
    }
}

fn check_directory_if_exists(path: &Path) -> Result<(), String> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() && !meta.file_type().is_symlink() => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err(format!("Expected a real directory: {}", path.display())),
    }
}
fn ensure_real_directory(path: &Path) -> Result<(), String> {
    check_directory_if_exists(path)?;
    if !path.exists() {
        fs::DirBuilder::new()
            .mode(0o700)
            .create(path)
            .map_err(io_error)?;
    }
    Ok(())
}
fn private_directory(path: &Path) -> Result<(), String> {
    fs::DirBuilder::new()
        .mode(0o700)
        .create(path)
        .map_err(io_error)
}
fn write_private(path: &Path, data: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(io_error)?;
    file.write_all(data).map_err(io_error)?;
    file.sync_all().map_err(io_error)
}
fn write_tree(root: &Path, tree: &Tree) -> Result<(), String> {
    fs::create_dir_all(root).map_err(io_error)?;
    fs::set_permissions(root, fs::Permissions::from_mode(0o700)).map_err(io_error)?;
    let mut directories = std::collections::BTreeSet::from([root.to_path_buf()]);
    for (name, bytes) in tree {
        let path = root.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(io_error)?;
            let mut current = parent;
            loop {
                directories.insert(current.to_path_buf());
                if current == root {
                    break;
                }
                current = current.parent().ok_or("Invalid backup directory")?;
            }
        }
        write_private(&path, bytes)?;
    }
    let mut directories = directories.into_iter().collect::<Vec<_>>();
    directories.sort_by_key(|path| std::cmp::Reverse(path.components().count()));
    for directory in directories {
        sync_directory(&directory)?;
    }
    Ok(())
}

fn make_addon_readable(root: &Path) -> Result<(), String> {
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o755)).map_err(io_error)?;
        for entry in fs::read_dir(&directory).map_err(io_error)? {
            let path = entry.map_err(io_error)?.path();
            if path.is_dir() {
                pending.push(path);
            } else {
                fs::set_permissions(path, fs::Permissions::from_mode(0o644)).map_err(io_error)?;
            }
        }
        sync_directory(&directory)?;
    }
    Ok(())
}
fn sync_directory(path: &Path) -> Result<(), String> {
    File::open(path)
        .map_err(io_error)?
        .sync_all()
        .map_err(io_error)
}
fn io_error(error: std::io::Error) -> String {
    format!("WoW setup filesystem operation failed: {error}")
}
#[cfg(target_os = "macos")]
fn game_running(root: &Path) -> Result<bool, String> {
    #[link(name = "proc")]
    extern "C" {
        fn proc_listallpids(buffer: *mut libc::c_void, size: i32) -> i32;
        fn proc_pidpath(pid: i32, buffer: *mut libc::c_void, size: u32) -> i32;
    }
    let count = unsafe { proc_listallpids(std::ptr::null_mut(), 0) };
    if count <= 0 || count > 65536 {
        return Err("Could not inspect running WoW processes".into());
    }
    let mut pids = vec![0_i32; count as usize + 512];
    let read = unsafe { proc_listallpids(pids.as_mut_ptr().cast(), (pids.len() * 4) as i32) };
    if read < 0 {
        return Err("Could not inspect running WoW processes".into());
    }
    for pid in pids.into_iter().take(read as usize) {
        let mut bytes = [0_u8; 4096];
        let length = unsafe { proc_pidpath(pid, bytes.as_mut_ptr().cast(), bytes.len() as u32) };
        if length > 0 {
            if let Ok(path) = std::str::from_utf8(&bytes[..length as usize]) {
                if Path::new(path.trim_end_matches('\0')).starts_with(root) {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}
#[cfg(not(target_os = "macos"))]
fn game_running(_root: &Path) -> Result<bool, String> {
    Err("WoW addon setup currently supports macOS clients".into())
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    fn fixture() -> (tempfile::TempDir, SetupRequest) {
        let temp = tempfile::tempdir().unwrap();
        let client_root = temp.path().join("_classic_era_");
        let source = temp.path().join("source");
        fs::create_dir_all(client_root.join("Interface/AddOns")).unwrap();
        for name in ADDONS {
            fs::create_dir_all(source.join(name)).unwrap();
            fs::write(
                source.join(name).join(format!("{name}.toc")),
                if *name == "ConsolePort" {
                    "## Interface: 16001, 11509\n## Version: 3.3.11\n"
                } else {
                    "## Interface: 16001, 11509\n## RequiredDeps: ConsolePort\n"
                },
            )
            .unwrap();
            fs::write(
                source.join(name).join("Main.lua"),
                "-- trusted local fixture\n",
            )
            .unwrap();
        }
        (
            temp,
            SetupRequest {
                client_root,
                consoleport_source: source,
                replace_bindings: false,
                replace_addons: false,
                enable_live: true,
            },
        )
    }
    #[test]
    fn dry_run_does_not_write_and_install_is_repeatable_with_embedded_complete_addon() {
        let (_temp, request) = fixture();
        let result = execute(&request, None).unwrap();
        assert!(!result.applied && result.changed);
        assert_eq!(result.plan.addons.len(), 10);
        assert!(!request
            .client_root
            .join("Interface/ThumbleBackups")
            .exists());
        let applied = execute(&request, Some(&result.plan.plan_sha256)).unwrap();
        assert!(applied.applied && applied.changed);
        let link = request.client_root.join("Interface/AddOns/ThumbleLink");
        for file in [
            "ThumbleLink.lua",
            "ThumbleLinkLive.lua",
            "ThumbleLinkOnboarding.lua",
            "ThumbleLinkSetup.lua",
            "ThumbleLink.toc",
        ] {
            assert!(link.join(file).is_file());
        }
        let setup = fs::read_to_string(link.join("ThumbleLinkSetup.lua")).unwrap();
        assert!(setup.contains("enabled = true") && setup.contains("replaceBindings = false"));
        let backup = applied.backup_directory.unwrap();
        assert_eq!(
            fs::metadata(&backup).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::read_to_string(backup.join("phase")).unwrap(),
            "committed\n"
        );
        let again = execute(&request, None).unwrap();
        assert!(!again.changed);
        assert!(
            !execute(&request, Some(&again.plan.plan_sha256))
                .unwrap()
                .changed
        );
    }
    #[test]
    fn overwrites_require_explicit_intent_and_back_up_exact_addons_and_binding_files() {
        let (_temp, mut request) = fixture();
        let old = request.client_root.join("Interface/AddOns/ConsolePort");
        fs::create_dir_all(&old).unwrap();
        fs::write(old.join("old.lua"), b"old addon").unwrap();
        let settings = request.client_root.join("WTF/Account/Test/Realm/Hunter");
        fs::create_dir_all(&settings).unwrap();
        fs::write(settings.join("bindings-cache.wtf"), b"original bindings").unwrap();
        fs::write(settings.join("unrelated.txt"), b"not collected").unwrap();
        let plan = execute(&request, None).unwrap().plan;
        assert_eq!(plan.conflicts, ["ConsolePort"]);
        assert!(execute(&request, Some(&plan.plan_sha256)).is_err());
        assert!(old.join("old.lua").exists());
        request.replace_addons = true;
        let plan = execute(&request, None).unwrap().plan;
        assert_eq!(plan.settings_backup_files, 1);
        let result = execute(&request, Some(&plan.plan_sha256)).unwrap();
        let backup = result.backup_directory.unwrap();
        assert_eq!(
            fs::read(backup.join("AddOns/ConsolePort/old.lua")).unwrap(),
            b"old addon"
        );
        assert_eq!(
            fs::read(backup.join("WTF/Account/Test/Realm/Hunter/bindings-cache.wtf")).unwrap(),
            b"original bindings"
        );
        assert_eq!(
            fs::read(settings.join("bindings-cache.wtf")).unwrap(),
            b"original bindings"
        );
        assert!(!backup
            .join("WTF/Account/Test/Realm/Hunter/unrelated.txt")
            .exists());
    }
    #[test]
    fn stale_plans_unsupported_versions_and_symlink_sources_fail_without_replacement() {
        let (temp, request) = fixture();
        let plan = execute(&request, None).unwrap().plan;
        fs::write(
            request.consoleport_source.join("ConsolePort/Main.lua"),
            "changed",
        )
        .unwrap();
        assert!(execute(&request, Some(&plan.plan_sha256)).is_err());
        assert!(!request
            .client_root
            .join("Interface/ThumbleBackups")
            .exists());
        fs::write(
            request
                .consoleport_source
                .join("ConsolePort/ConsolePort.toc"),
            "## Interface: 11509\n## Version: 99\n",
        )
        .unwrap();
        assert!(execute(&request, None).is_err());
        fs::write(
            request
                .consoleport_source
                .join("ConsolePort/ConsolePort.toc"),
            "## Interface: 11509\n## Version: 3.3.11\n",
        )
        .unwrap();
        symlink(
            temp.path(),
            request.consoleport_source.join("ConsolePort/link"),
        )
        .unwrap();
        assert!(execute(&request, None).is_err());
        fs::remove_file(request.consoleport_source.join("ConsolePort/link")).unwrap();
        symlink(
            &request.consoleport_source,
            request.client_root.join("Interface/AddOns/ConsolePort"),
        )
        .unwrap();
        assert!(execute(&request, None).is_err());
    }
    #[test]
    fn mid_install_failure_restores_prior_folders_and_preserves_external_edit() {
        let (_temp, request) = fixture();
        let plan = execute(&request, None).unwrap().plan;
        let second = request.client_root.join("Interface/AddOns/ConsolePort_Bar");
        let error = execute_with(&request, Some(&plan.plan_sha256), |index| {
            if index == 1 {
                fs::create_dir_all(&second).unwrap();
                fs::write(second.join("external.lua"), "external edit").unwrap();
            }
            Ok(())
        })
        .unwrap_err();
        assert!(error.contains("changed during installation"));
        assert!(!request
            .client_root
            .join("Interface/AddOns/ConsolePort")
            .exists());
        assert_eq!(
            fs::read_to_string(second.join("external.lua")).unwrap(),
            "external edit"
        );
    }
    #[test]
    fn failed_install_cannot_delete_changes_to_an_already_installed_folder() {
        let (_temp, request) = fixture();
        let plan = execute(&request, None).unwrap().plan;
        let first = request.client_root.join("Interface/AddOns/ConsolePort");
        let error = execute_with(&request, Some(&plan.plan_sha256), |index| {
            if index == 1 {
                fs::write(first.join("external.lua"), "keep me").unwrap();
                return Err("injected failure".into());
            }
            Ok(())
        })
        .unwrap_err();
        assert!(error.contains("recovery required"));
        assert_eq!(
            fs::read_to_string(first.join("external.lua")).unwrap(),
            "keep me"
        );
    }
}
