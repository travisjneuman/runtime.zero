use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use rz0_action_plan::{
    ActionDisposition, ActionKind, ActionPlan, PlanAction, WriteKind, validate_action_plan,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const MARKER: &str = ".rz0-transaction-simulation-v1";
const MARKER_CONTENT: &[u8] = b"schema_version=1\nsimulation_only=true\n";
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailurePoint {
    None,
    AfterVerifiedCopy,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuarantineRecord {
    pub schema_version: u16,
    pub simulation_only: bool,
    pub plan_id: String,
    pub original_path: String,
    pub quarantine_path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub original_removed_after_verified_copy: bool,
}

pub fn simulate_quarantine(
    root: &Path,
    plan: &ActionPlan,
    failure: FailurePoint,
) -> Result<QuarantineRecord, String> {
    validate_root_and_plan(root, plan)?;
    let action = planned_action(plan, ActionKind::Quarantine)?;
    let source = action
        .source
        .as_ref()
        .ok_or_else(|| "quarantine action has no source evidence".to_string())?;
    let payload_path = write_path(action, WriteKind::QuarantinedPayload)?;
    let record_path = write_path(action, WriteKind::QuarantineRecord)?;
    let original = root.join(&source.path);
    let quarantine = root.join(payload_path);
    let record_file = root.join(record_path);
    if quarantine.exists() || record_file.exists() {
        return Err("quarantine destination or record already exists".to_string());
    }
    ensure_safe_destination(root, &quarantine)?;
    ensure_safe_destination(root, &record_file)?;

    let bytes = read_verified(root, &original, source.size_bytes, &source.sha256)?;
    write_verified_new(&quarantine, &bytes, &source.sha256)?;
    if failure == FailurePoint::AfterVerifiedCopy {
        return Err("injected failure after verified quarantine copy".to_string());
    }

    let record = QuarantineRecord {
        schema_version: 1,
        simulation_only: true,
        plan_id: plan.plan_id.clone(),
        original_path: source.path.clone(),
        quarantine_path: payload_path.to_string(),
        sha256: source.sha256.clone(),
        size_bytes: source.size_bytes,
        original_removed_after_verified_copy: true,
    };
    fs::remove_file(&original)
        .map_err(|error| format!("remove simulated original after quarantine: {error}"))?;
    let record_json = serde_json::to_vec_pretty(&record)
        .map_err(|error| format!("serialize quarantine record: {error}"))?;
    write_new(&record_file, &record_json)?;
    Ok(record)
}

pub fn simulate_restore(root: &Path, plan: &ActionPlan) -> Result<PathBuf, String> {
    validate_root_and_plan(root, plan)?;
    let action = planned_action(plan, ActionKind::Restore)?;
    let source = action
        .source
        .as_ref()
        .ok_or_else(|| "restore action has no source evidence".to_string())?;
    let destination_path = write_path(action, WriteKind::RestoredPayload)?;
    let quarantine = root.join(&source.path);
    let destination = root.join(destination_path);
    if destination.exists() {
        return Err("restore destination already exists".to_string());
    }
    ensure_safe_destination(root, &destination)?;
    let bytes = read_verified(root, &quarantine, source.size_bytes, &source.sha256)?;
    write_verified_new(&destination, &bytes, &source.sha256)?;
    Ok(destination)
}

fn validate_root_and_plan(root: &Path, plan: &ActionPlan) -> Result<(), String> {
    let validation = validate_action_plan(plan);
    if !validation.valid {
        return Err(format!("invalid action plan: {:?}", validation.errors));
    }
    let canonical_temp = fs::canonicalize(std::env::temp_dir())
        .map_err(|error| format!("canonicalize temp root: {error}"))?;
    let canonical_root =
        fs::canonicalize(root).map_err(|error| format!("canonicalize simulation root: {error}"))?;
    if canonical_root.parent() != Some(canonical_temp.as_path())
        || !canonical_root
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("rz0-action-sim-"))
    {
        return Err("action simulation root must be a direct prefixed temp child".to_string());
    }
    let marker = canonical_root.join(MARKER);
    ensure_direct_regular_file(&canonical_root, &marker)?;
    if fs::read(marker).ok().as_deref() != Some(MARKER_CONTENT) {
        return Err("action simulation marker content is invalid".to_string());
    }
    Ok(())
}

fn planned_action(plan: &ActionPlan, kind: ActionKind) -> Result<&PlanAction, String> {
    let mut matching = plan
        .actions
        .iter()
        .filter(|action| action.kind == kind && action.disposition == ActionDisposition::Planned);
    let action = matching
        .next()
        .ok_or_else(|| "plan has no matching planned action".to_string())?;
    if matching.next().is_some() {
        return Err("plan has multiple matching actions".to_string());
    }
    Ok(action)
}

fn write_path(action: &PlanAction, kind: WriteKind) -> Result<&str, String> {
    let mut matching = action.write_set.iter().filter(|entry| entry.kind == kind);
    let path = matching
        .next()
        .map(|entry| entry.path.as_str())
        .ok_or_else(|| "action has no matching write-set entry".to_string())?;
    if matching.next().is_some() {
        return Err("action has duplicate write-set kinds".to_string());
    }
    Ok(path)
}

fn read_verified(
    root: &Path,
    path: &Path,
    expected_size: u64,
    expected_sha256: &str,
) -> Result<Vec<u8>, String> {
    ensure_direct_regular_file(root, path)?;
    let file = File::open(path).map_err(|error| format!("open simulated source: {error}"))?;
    let mut bytes = Vec::new();
    file.take(expected_size.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("read simulated source: {error}"))?;
    if bytes.len() as u64 != expected_size || sha256(&bytes) != expected_sha256 {
        return Err("simulated source hash or size mismatch".to_string());
    }
    Ok(bytes)
}

fn write_verified_new(path: &Path, bytes: &[u8], expected_sha256: &str) -> Result<(), String> {
    write_new(path, bytes)?;
    let written = fs::read(path).map_err(|error| format!("read simulated write: {error}"))?;
    if sha256(&written) != expected_sha256 {
        return Err("simulated post-write digest mismatch".to_string());
    }
    Ok(())
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create simulated parent: {error}"))?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("create simulated file: {error}"))?;
    file.write_all(bytes)
        .map_err(|error| format!("write simulated file: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("sync simulated file: {error}"))
}

fn ensure_safe_destination(root: &Path, file: &Path) -> Result<(), String> {
    let relative = file
        .strip_prefix(root)
        .map_err(|_| "simulation destination escaped root".to_string())?;
    let mut current = root.to_path_buf();
    let component_count = relative.components().count();
    for (index, component) in relative.components().enumerate() {
        if index + 1 == component_count {
            break;
        }
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                return Err("simulation destination contains an unsafe component".to_string());
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(format!("inspect simulation destination: {error}")),
        }
    }
    Ok(())
}

fn ensure_direct_regular_file(root: &Path, file: &Path) -> Result<(), String> {
    let relative = file
        .strip_prefix(root)
        .map_err(|_| "simulation file escaped root".to_string())?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| format!("inspect simulation path: {error}"))?;
        if metadata.file_type().is_symlink() {
            return Err("simulation path contains a symlink".to_string());
        }
    }
    if !fs::symlink_metadata(file)
        .map_err(|error| format!("inspect simulation file: {error}"))?
        .is_file()
    {
        return Err("simulation source is not a regular file".to_string());
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn copy_original_fixture(root: &Path) {
    let destination = root.join("workspace/stale-shim.bin");
    fs::create_dir_all(destination.parent().expect("workspace parent")).expect("workspace root");
    fs::copy(fixture_file(), destination).expect("copy transaction fixture");
}

fn fixture_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/transaction/workspace/stale-shim.bin")
}

pub struct SimulationRoot {
    path: PathBuf,
}

impl SimulationRoot {
    pub fn new() -> Self {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("rz0-action-sim-{}-{sequence}", std::process::id()));
        fs::create_dir(&path).expect("create action simulation root");
        write_new(&path.join(MARKER), MARKER_CONTENT).expect("action simulation marker");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SimulationRoot {
    fn drop(&mut self) {
        cleanup_simulation_root(&self.path);
    }
}

fn cleanup_simulation_root(path: &Path) {
    let Ok(temp) = fs::canonicalize(std::env::temp_dir()) else {
        return;
    };
    let Ok(root) = fs::canonicalize(path) else {
        return;
    };
    let marker = root.join(MARKER);
    if root.parent() == Some(temp.as_path())
        && root
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("rz0-action-sim-"))
        && fs::symlink_metadata(&marker).is_ok_and(|metadata| metadata.is_file())
        && fs::read(marker).ok().as_deref() == Some(MARKER_CONTENT)
    {
        let _ = fs::remove_dir_all(root);
    }
}
