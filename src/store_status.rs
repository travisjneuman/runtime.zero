use crate::install_receipt::{
    ReceiptInventoryReport, ReceiptInventoryState, receipt_inventory_report,
};
use crate::installed_registry::{
    InstalledRegistryReport, InstalledRegistryState, installed_registry_report,
};
use crate::launch_routing::{LaunchEnvironment, LaunchRoutingReport, resolve_launch_mode};
use crate::module_store::{ModuleStorePlan, STORE_SCHEMA_VERSION, module_store_plan};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
const STATUS_SEED: &str = "store status";
const EXAMPLE_MODULE_ID: &str = "first-party.example";
const EXAMPLE_MODULE_VERSION: &str = "0.0.0";
const SAFETY_NOTE: &str =
    "Read-only store inventory; no directories or state files were created or repaired.";
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoreStatusReport {
    pub command: &'static str,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub store_schema_version: u16,
    pub overall_state: StoreOverallState,
    pub store: ModuleStorePlan,
    pub paths: Vec<StorePathStatus>,
    pub registry: InstalledRegistryReport,
    pub receipts: ReceiptInventoryReport,
    pub launch_context: LaunchRoutingReport,
    pub safety_note: &'static str,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StoreOverallState {
    NotInitialized,
    Empty,
    Present,
    Invalid,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StorePathStatus {
    pub role: StorePathRole,
    pub path: String,
    pub expected_kind: StorePathKind,
    pub state: StorePathState,
    pub detail: String,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StorePathRole {
    DataRoot,
    StateRoot,
    CacheRoot,
    LogRoot,
    QuarantineRoot,
    ModulesRoot,
    RegistryPath,
    TransactionsDir,
    ReceiptsDir,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StorePathKind {
    Directory,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StorePathState {
    Absent,
    Empty,
    Present,
    Invalid,
}

pub fn store_status_report(args: &[String]) -> StoreStatusReport {
    let store = module_store_plan(
        Some(EXAMPLE_MODULE_ID),
        Some(EXAMPLE_MODULE_VERSION),
        STATUS_SEED,
    );
    let paths = inspect_store_paths(&store);
    let registry = installed_registry_report(Path::new(&store.registry_path));
    let receipts = receipt_inventory_report(Path::new(&store.state_root), &registry.records);
    let overall_state = overall_state(&paths, registry.status, receipts.overall_state);
    StoreStatusReport {
        command: "store status",
        read_only: true,
        writes_attempted: false,
        store_schema_version: STORE_SCHEMA_VERSION,
        overall_state,
        store,
        paths,
        registry,
        receipts,
        launch_context: resolve_launch_mode(args, LaunchEnvironment::cli_subcommand()),
        safety_note: SAFETY_NOTE,
    }
}

fn inspect_store_paths(store: &ModuleStorePlan) -> Vec<StorePathStatus> {
    let mut paths = vec![
        dir_status(StorePathRole::DataRoot, &store.data_root),
        dir_status(StorePathRole::StateRoot, &store.state_root),
        dir_status(StorePathRole::CacheRoot, &store.cache_root),
        dir_status(StorePathRole::LogRoot, &store.log_root),
        dir_status(StorePathRole::QuarantineRoot, &store.quarantine_root),
        dir_status(StorePathRole::ModulesRoot, &store.modules_root),
        file_status(StorePathRole::RegistryPath, &store.registry_path),
    ];
    if let Some(path) = parent_dir(&store.transaction_path) {
        paths.push(dir_status(StorePathRole::TransactionsDir, &path));
    }
    if let Some(path) = store.receipt_path.as_deref().and_then(parent_dir) {
        paths.push(dir_status(StorePathRole::ReceiptsDir, &path));
    }
    paths
}

fn dir_status(role: StorePathRole, path: &str) -> StorePathStatus {
    inspect_path(role, path, StorePathKind::Directory)
}

fn file_status(role: StorePathRole, path: &str) -> StorePathStatus {
    inspect_path(role, path, StorePathKind::File)
}

fn parent_dir(path: &str) -> Option<String> {
    Path::new(path)
        .parent()
        .map(|path| path.display().to_string())
}

fn inspect_path(role: StorePathRole, path: &str, expected_kind: StorePathKind) -> StorePathStatus {
    let path_buf = PathBuf::from(path);
    let (state, detail) = classify_path(&path_buf, expected_kind);
    StorePathStatus {
        role,
        path: path.to_string(),
        expected_kind,
        state,
        detail,
    }
}

fn classify_path(path: &Path, expected_kind: StorePathKind) -> (StorePathState, String) {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => (
            StorePathState::Invalid,
            "path is a symlink and is not accepted as store state".to_string(),
        ),
        Ok(metadata) => classify_existing_path(path, expected_kind, &metadata),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            (StorePathState::Absent, "path does not exist".to_string())
        }
        Err(err) => (StorePathState::Invalid, format!("metadata error: {err}")),
    }
}

fn classify_existing_path(
    path: &Path,
    expected_kind: StorePathKind,
    metadata: &fs::Metadata,
) -> (StorePathState, String) {
    match (expected_kind, metadata.is_dir(), metadata.is_file()) {
        (StorePathKind::Directory, true, _) => classify_directory(path),
        (StorePathKind::File, _, true) if metadata.len() == 0 => (
            StorePathState::Empty,
            "file exists but is empty".to_string(),
        ),
        (StorePathKind::File, _, true) => (StorePathState::Present, "file exists".to_string()),
        _ => (
            StorePathState::Invalid,
            "path exists with the wrong filesystem type".to_string(),
        ),
    }
}

fn classify_directory(path: &Path) -> (StorePathState, String) {
    match fs::read_dir(path) {
        Ok(entries) => classify_directory_entries(entries),
        Err(err) => (StorePathState::Invalid, format!("read_dir error: {err}")),
    }
}

fn classify_directory_entries(mut entries: fs::ReadDir) -> (StorePathState, String) {
    if entries.next().is_none() {
        (
            StorePathState::Empty,
            "directory exists and is empty".to_string(),
        )
    } else {
        (StorePathState::Present, "directory exists".to_string())
    }
}

fn overall_state(
    paths: &[StorePathStatus],
    registry_state: InstalledRegistryState,
    receipt_state: ReceiptInventoryState,
) -> StoreOverallState {
    if matches!(
        registry_state,
        InstalledRegistryState::Invalid | InstalledRegistryState::Unreadable
    ) || matches!(
        receipt_state,
        ReceiptInventoryState::Invalid
            | ReceiptInventoryState::Unreadable
            | ReceiptInventoryState::UnsupportedSchema
            | ReceiptInventoryState::Absent
    ) {
        return StoreOverallState::Invalid;
    }
    if paths
        .iter()
        .any(|path| path.state == StorePathState::Invalid)
    {
        StoreOverallState::Invalid
    } else if paths
        .iter()
        .all(|path| path.state == StorePathState::Absent)
        && registry_state == InstalledRegistryState::Absent
    {
        StoreOverallState::NotInitialized
    } else if paths
        .iter()
        .all(|path| matches!(path.state, StorePathState::Absent | StorePathState::Empty))
        && matches!(
            registry_state,
            InstalledRegistryState::Absent | InstalledRegistryState::Empty
        )
    {
        StoreOverallState::Empty
    } else {
        StoreOverallState::Present
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn absent_paths_report_not_initialized() {
        let missing = unique_missing_path();
        let paths = vec![
            dir_status(StorePathRole::DataRoot, &missing),
            file_status(StorePathRole::RegistryPath, &missing),
        ];
        assert!(
            paths
                .iter()
                .all(|path| path.state == StorePathState::Absent)
        );
        assert_eq!(
            overall_state(
                &paths,
                InstalledRegistryState::Absent,
                ReceiptInventoryState::NotReferenced
            ),
            StoreOverallState::NotInitialized
        );
    }

    #[test]
    fn status_report_has_stable_json_shape() {
        let report = store_status_report(&[
            "status".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ]);
        let json = serde_json::to_string(&report).expect("status JSON should serialize");
        assert!(json.contains("\"store_schema_version\":1"));
        assert!(json.contains("\"writes_attempted\":false"));
        assert!(json.contains("\"registry_path\""));
        assert!(json.contains("\"registry\""));
        assert!(json.contains("\"installed_module_count\""));
        assert!(json.contains("\"receipts\""));
        assert!(json.contains("\"checked_count\""));
        assert!(json.contains("\"transactions_dir\""));
        assert!(json.contains("\"receipts_dir\""));
        assert!(json.contains("\"launch_mode\":\"cli_subcommand\""));
    }

    fn unique_missing_path() -> String {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        std::env::temp_dir()
            .join(format!("rz0-store-status-missing-{nanos}"))
            .display()
            .to_string()
    }
}
