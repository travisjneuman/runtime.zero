use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use rz0_module_trust::{StagingPlan, validate_relative_path, validate_staging_plan};
use sha2::{Digest, Sha256};

const MARKER: &str = ".rz0-transaction-simulation-v1";
const MARKER_CONTENT: &[u8] = b"schema_version=1\nsimulation_only=true\n";
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
pub struct StageReceipt {
    pub simulation_only: bool,
    pub production_writes_attempted: bool,
    pub published_root: String,
    pub files: Vec<String>,
}

pub fn simulate_staging(root: &Path, plan: &StagingPlan) -> Result<StageReceipt, String> {
    let validation = validate_staging_plan(plan);
    if !validation.valid {
        return Err(format!("invalid staging plan: {:?}", validation.errors));
    }
    validate_simulation_root(root)?;
    let publication = root.join(&plan.publication_root);
    if publication.exists() {
        return Err("publication destination already exists".to_string());
    }
    let staging = root.join(&plan.staging_root);
    if staging.exists() {
        return Err("staging destination already exists".to_string());
    }
    ensure_safe_destination(root, &staging)?;
    ensure_safe_destination(root, &publication)?;
    fs::create_dir_all(&staging).map_err(|error| format!("create staging root: {error}"))?;

    let mut staged_files = Vec::new();
    for planned_file in &plan.files {
        validate_relative_path(&planned_file.path)?;
        let source = root.join(&plan.source_root).join(&planned_file.path);
        ensure_direct_regular_file(root, &source)?;
        let bytes = read_bounded(&source, planned_file.size_bytes)?;
        if bytes.len() as u64 != planned_file.size_bytes || sha256(&bytes) != planned_file.sha256 {
            return Err(format!(
                "hash or size mismatch for staged file {}",
                planned_file.path
            ));
        }

        let destination = staging.join(&planned_file.path);
        ensure_safe_destination(root, &destination)?;
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("create staging parent: {error}"))?;
        }
        write_new(&destination, &bytes)?;
        let staged_bytes = read_bounded(&destination, planned_file.size_bytes)?;
        if sha256(&staged_bytes) != planned_file.sha256 {
            return Err(format!(
                "post-write verification failed for {}",
                planned_file.path
            ));
        }
        staged_files.push(planned_file.path.clone());
    }

    let publication_parent = publication
        .parent()
        .ok_or_else(|| "publication root has no parent".to_string())?;
    fs::create_dir_all(publication_parent)
        .map_err(|error| format!("create publication parent: {error}"))?;
    fs::rename(&staging, &publication)
        .map_err(|error| format!("atomic fixture publication failed: {error}"))?;

    Ok(StageReceipt {
        simulation_only: true,
        production_writes_attempted: false,
        published_root: plan.publication_root.clone(),
        files: staged_files,
    })
}

fn validate_simulation_root(root: &Path) -> Result<(), String> {
    let canonical_temp = fs::canonicalize(std::env::temp_dir())
        .map_err(|error| format!("canonicalize temp root: {error}"))?;
    let canonical_root =
        fs::canonicalize(root).map_err(|error| format!("canonicalize simulation root: {error}"))?;
    if canonical_root.parent() != Some(canonical_temp.as_path())
        || !canonical_root
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("rz0-transaction-sim-"))
    {
        return Err(
            "simulation root must be a direct prefixed child of the OS temp root".to_string(),
        );
    }
    let marker = canonical_root.join(MARKER);
    ensure_direct_regular_file(&canonical_root, &marker)?;
    if fs::read(marker).ok().as_deref() != Some(MARKER_CONTENT) {
        return Err("simulation marker content is invalid".to_string());
    }
    Ok(())
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
        .map_err(|_| "file escaped simulation root".to_string())?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component.as_os_str());
        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| format!("inspect simulation path: {error}"))?;
        if metadata.file_type().is_symlink() {
            return Err("simulation path contains a symlink".to_string());
        }
    }
    let metadata =
        fs::symlink_metadata(file).map_err(|error| format!("inspect simulation file: {error}"))?;
    if !metadata.is_file() {
        return Err("simulation source is not a regular file".to_string());
    }
    Ok(())
}

fn read_bounded(path: &Path, expected_size: u64) -> Result<Vec<u8>, String> {
    let file = File::open(path).map_err(|error| format!("open fixture file: {error}"))?;
    let mut bytes = Vec::new();
    file.take(expected_size.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("read fixture file: {error}"))?;
    if bytes.len() as u64 > expected_size {
        return Err("fixture file exceeded its declared size".to_string());
    }
    Ok(bytes)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("create staged fixture file: {error}"))?;
    file.write_all(bytes)
        .map_err(|error| format!("write staged fixture file: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("sync staged fixture file: {error}"))
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn copy_fixture_input(root: &Path) {
    let destination = root.join("input/package");
    fs::create_dir_all(&destination).expect("fixture input root");
    for name in ["rz0-module.json", "payload.txt"] {
        let source = fixture_root().join("input/package").join(name);
        fs::copy(source, destination.join(name)).expect("copy fixture input");
    }
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("staging")
}

pub struct SimulationRoot {
    path: PathBuf,
}

impl SimulationRoot {
    pub fn new() -> Self {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "rz0-transaction-sim-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create simulation root");
        write_new(&path.join(MARKER), MARKER_CONTENT).expect("simulation marker");
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
            .is_some_and(|name| name.starts_with("rz0-transaction-sim-"))
        && fs::symlink_metadata(&marker).is_ok_and(|metadata| metadata.is_file())
        && fs::read(marker).ok().as_deref() == Some(MARKER_CONTENT)
    {
        let _ = fs::remove_dir_all(root);
    }
}
