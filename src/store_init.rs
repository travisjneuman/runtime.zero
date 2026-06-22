use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::installed_registry::{InstalledRegistryState, installed_registry_report};
use crate::launch_routing::{LaunchEnvironment, resolve_launch_mode};
use crate::module_store::{
    ModuleStorePlan, STORE_SCHEMA_VERSION, module_store_plan, module_store_plan_for_data_root,
};
pub use crate::store_init_model::{
    StoreInitKind, StoreInitMode, StoreInitOptions, StoreInitReport, StoreInitRole,
    StoreInitStatus, StoreInitStep, StoreInitStepState,
};

const INIT_MARKER_FILE: &str = "store-init.json";
const INIT_SEED: &str = "store init";
const SAFETY_NOTE: &str = "Store init is user-local scaffolding only; it never installs modules.";

pub fn store_init_report(args: &[String], options: StoreInitOptions) -> StoreInitReport {
    let store = init_store_plan(&options);
    run_init(args, store, options.mode)
}

fn init_store_plan(options: &StoreInitOptions) -> ModuleStorePlan {
    match &options.store_root {
        Some(root) => module_store_plan_for_data_root(root.clone(), None, None, INIT_SEED),
        None => module_store_plan(None, None, INIT_SEED),
    }
}

fn run_init(args: &[String], store: ModuleStorePlan, mode: StoreInitMode) -> StoreInitReport {
    let marker = PathBuf::from(&store.state_root).join(INIT_MARKER_FILE);
    let mut steps = planned_steps(&store, &marker);
    let registry = installed_registry_report(Path::new(&store.registry_path));
    let mut registry_status = registry.status;
    apply_registry_state(&mut steps, registry_status);
    apply_marker_state(&mut steps, &marker);
    let mut status = init_status(&steps);
    let mut writes_attempted = false;
    if mode == StoreInitMode::Apply
        && !status.is_blocked()
        && status != StoreInitStatus::AlreadyInitialized
    {
        writes_attempted = true;
        apply_steps(&store, &marker, &mut steps);
        status = init_status(&steps);
        if !status.is_blocked() {
            status = StoreInitStatus::Applied;
            registry_status = installed_registry_report(Path::new(&store.registry_path)).status;
        }
    }
    StoreInitReport {
        command: "store init",
        dry_run: mode == StoreInitMode::DryRun,
        writes_attempted,
        store_schema_version: STORE_SCHEMA_VERSION,
        status,
        store,
        registry_status,
        init_marker_path: marker.display().to_string(),
        steps,
        launch_context: resolve_launch_mode(args, LaunchEnvironment::cli_subcommand()),
        safety_note: SAFETY_NOTE,
    }
}

fn planned_steps(store: &ModuleStorePlan, marker: &Path) -> Vec<StoreInitStep> {
    let transactions = parent_path(&store.transaction_path);
    let receipts = PathBuf::from(&store.state_root).join("receipts");
    vec![
        dir(StoreInitRole::DataRoot, &store.data_root),
        dir(StoreInitRole::StateRoot, &store.state_root),
        dir(StoreInitRole::CacheRoot, &store.cache_root),
        dir(StoreInitRole::LogRoot, &store.log_root),
        dir(StoreInitRole::QuarantineRoot, &store.quarantine_root),
        dir(StoreInitRole::ModulesRoot, &store.modules_root),
        dir(StoreInitRole::TransactionsDir, &transactions),
        dir(StoreInitRole::ReceiptsDir, &receipts.display().to_string()),
        file(StoreInitRole::RegistryPath, &store.registry_path),
        file(StoreInitRole::InitMarkerPath, &marker.display().to_string()),
    ]
}

fn parent_path(path: &str) -> String {
    Path::new(path)
        .parent()
        .map(|path| path.display().to_string())
        .unwrap_or_default()
}

fn dir(role: StoreInitRole, path: &str) -> StoreInitStep {
    inspect_step(role, path, StoreInitKind::Directory)
}

fn file(role: StoreInitRole, path: &str) -> StoreInitStep {
    inspect_step(role, path, StoreInitKind::File)
}

fn inspect_step(role: StoreInitRole, path: &str, kind: StoreInitKind) -> StoreInitStep {
    let (state, detail) = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => (
            StoreInitStepState::Blocked,
            "path is a symlink and is not accepted for initialization".to_string(),
        ),
        Ok(metadata) if kind == StoreInitKind::Directory && metadata.is_dir() => (
            StoreInitStepState::Exists,
            "directory already exists".to_string(),
        ),
        Ok(metadata) if kind == StoreInitKind::File && metadata.is_file() => (
            StoreInitStepState::Exists,
            "file already exists".to_string(),
        ),
        Ok(_) => (
            StoreInitStepState::Blocked,
            "path exists with the wrong filesystem type".to_string(),
        ),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => match kind {
            StoreInitKind::Directory => (
                StoreInitStepState::WouldCreate,
                "directory would be created".to_string(),
            ),
            StoreInitKind::File => (
                StoreInitStepState::WouldWrite,
                "file would be written".to_string(),
            ),
        },
        Err(err) => (
            StoreInitStepState::Blocked,
            format!("metadata error: {err}"),
        ),
    };
    StoreInitStep {
        role,
        path: path.to_string(),
        expected_kind: kind,
        state,
        detail,
    }
}

fn apply_registry_state(steps: &mut [StoreInitStep], registry_state: InstalledRegistryState) {
    let Some(step) = steps
        .iter_mut()
        .find(|step| step.role == StoreInitRole::RegistryPath)
    else {
        return;
    };
    if matches!(
        registry_state,
        InstalledRegistryState::Empty
            | InstalledRegistryState::Invalid
            | InstalledRegistryState::Unreadable
    ) {
        step.state = StoreInitStepState::Blocked;
        step.detail = "existing registry is not valid; refusing to repair or overwrite".to_string();
    }
}

fn apply_marker_state(steps: &mut [StoreInitStep], marker: &Path) {
    let Some(step) = steps
        .iter_mut()
        .find(|step| step.role == StoreInitRole::InitMarkerPath)
    else {
        return;
    };
    if step.state != StoreInitStepState::Exists {
        return;
    }
    match fs::read_to_string(marker)
        .ok()
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok())
    {
        Some(value) if valid_marker(&value) => {}
        _ => {
            step.state = StoreInitStepState::Blocked;
            step.detail = "existing store init marker is not valid; refusing to repair".to_string();
        }
    }
}

fn valid_marker(value: &serde_json::Value) -> bool {
    value.get("schema_version").and_then(|value| value.as_u64()) == Some(1)
        && value.get("kind").and_then(|value| value.as_str()) == Some("runtime_zero_store_init")
        && value
            .get("store_schema_version")
            .and_then(|value| value.as_u64())
            == Some(u64::from(STORE_SCHEMA_VERSION))
}

fn init_status(steps: &[StoreInitStep]) -> StoreInitStatus {
    if steps
        .iter()
        .any(|step| step.state == StoreInitStepState::Blocked)
    {
        StoreInitStatus::Blocked
    } else if steps
        .iter()
        .all(|step| step.state == StoreInitStepState::Exists)
    {
        StoreInitStatus::AlreadyInitialized
    } else {
        StoreInitStatus::Ready
    }
}

fn apply_steps(store: &ModuleStorePlan, marker: &Path, steps: &mut [StoreInitStep]) {
    for step in steps.iter_mut() {
        match (step.expected_kind, step.state) {
            (StoreInitKind::Directory, StoreInitStepState::WouldCreate) => create_dir_step(step),
            (StoreInitKind::File, StoreInitStepState::WouldWrite) => {
                write_file_step(store, marker, step)
            }
            _ => {}
        }
    }
}

fn create_dir_step(step: &mut StoreInitStep) {
    match fs::create_dir_all(&step.path) {
        Ok(()) => {
            step.state = StoreInitStepState::Created;
            step.detail = "directory created".to_string();
        }
        Err(err) => block_step(step, format!("create_dir_all failed: {err}")),
    }
}

fn write_file_step(store: &ModuleStorePlan, marker: &Path, step: &mut StoreInitStep) {
    let result = if step.role == StoreInitRole::RegistryPath {
        fs::write(&step.path, empty_registry_json())
    } else {
        fs::write(&step.path, init_marker_json(store, marker))
    };
    match result {
        Ok(()) => {
            step.state = StoreInitStepState::Written;
            step.detail = "file written".to_string();
        }
        Err(err) => block_step(step, format!("write failed: {err}")),
    }
}

fn block_step(step: &mut StoreInitStep, detail: String) {
    step.state = StoreInitStepState::Blocked;
    step.detail = detail;
}

fn empty_registry_json() -> String {
    "{\n  \"schema_version\": 1,\n  \"modules\": []\n}\n".to_string()
}

fn init_marker_json(store: &ModuleStorePlan, marker: &Path) -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    serde_json::json!({
        "schema_version": 1,
        "kind": "runtime_zero_store_init",
        "store_schema_version": STORE_SCHEMA_VERSION,
        "created_unix_seconds": timestamp,
        "command": "rz0 store init --yes",
        "data_root": &store.data_root,
        "state_root": &store.state_root,
        "init_marker_path": marker.display().to_string(),
        "rollback": "Review and remove only runtime.zero-owned paths recorded in this marker."
    })
    .to_string()
}
