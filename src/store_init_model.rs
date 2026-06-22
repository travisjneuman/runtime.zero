use std::path::PathBuf;

use serde::Serialize;

use crate::installed_registry::InstalledRegistryState;
use crate::launch_routing::LaunchRoutingReport;
use crate::module_store::ModuleStorePlan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreInitMode {
    DryRun,
    Apply,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreInitOptions {
    pub(crate) mode: StoreInitMode,
    pub(crate) store_root: Option<PathBuf>,
}

impl StoreInitOptions {
    pub const fn new(mode: StoreInitMode) -> Self {
        Self {
            mode,
            store_root: None,
        }
    }

    pub fn with_store_root(mode: StoreInitMode, store_root: PathBuf) -> Self {
        Self {
            mode,
            store_root: Some(store_root),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoreInitReport {
    pub command: &'static str,
    pub dry_run: bool,
    pub writes_attempted: bool,
    pub store_schema_version: u16,
    pub status: StoreInitStatus,
    pub store: ModuleStorePlan,
    pub registry_status: InstalledRegistryState,
    pub init_marker_path: String,
    pub steps: Vec<StoreInitStep>,
    pub launch_context: LaunchRoutingReport,
    pub safety_note: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StoreInitStatus {
    Ready,
    AlreadyInitialized,
    Applied,
    Blocked,
}

impl StoreInitStatus {
    pub const fn is_blocked(self) -> bool {
        matches!(self, Self::Blocked)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoreInitStep {
    pub role: StoreInitRole,
    pub path: String,
    pub expected_kind: StoreInitKind,
    pub state: StoreInitStepState,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StoreInitRole {
    DataRoot,
    StateRoot,
    CacheRoot,
    LogRoot,
    QuarantineRoot,
    ModulesRoot,
    TransactionsDir,
    ReceiptsDir,
    RegistryPath,
    InitMarkerPath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StoreInitKind {
    Directory,
    File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StoreInitStepState {
    WouldCreate,
    WouldWrite,
    Exists,
    Created,
    Written,
    Blocked,
}
