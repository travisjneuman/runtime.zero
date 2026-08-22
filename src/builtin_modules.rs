//! The first-party capabilities shipped inside the `rz0` binary.
//!
//! Built-ins do not pretend to be downloaded packages. Their executable bytes
//! are part of the signed/source-built runtime; the user lifecycle therefore
//! controls availability and local preference, while every system action still
//! goes through its capability-specific plan and confirmation contract.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::module_manifest::{ModuleKind, ModuleManifest, ModuleSafety, ModuleStatus, RiskLevel};
use crate::module_store::{ModuleStorePlan, module_store_plan, module_store_plan_for_data_root};
use crate::store_init::{StoreInitMode, StoreInitOptions, store_init_report};

pub const BUILTIN_STATE_FILE: &str = "builtin-modules.json";
pub const BUILTIN_STATE_SCHEMA_VERSION: u16 = 1;
const MAX_MODULE_STATE_BYTES: u64 = rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinOperation {
    Install,
    Enable,
    Disable,
    Update,
    Uninstall,
}

impl BuiltinOperation {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Enable => "enable",
            Self::Disable => "disable",
            Self::Update => "update",
            Self::Uninstall => "uninstall",
        }
    }

    const fn desired_enabled(self) -> Option<bool> {
        match self {
            Self::Install | Self::Enable => Some(true),
            Self::Disable | Self::Uninstall => Some(false),
            Self::Update => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuiltinLifecycleMode {
    DryRun,
    Apply {
        challenge_issued_unix_seconds: u64,
        confirmation: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinLifecycleRequest {
    pub operation: BuiltinOperation,
    pub module_id: String,
    pub store_root: Option<PathBuf>,
    pub mode: BuiltinLifecycleMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuiltinLifecycleChallenge {
    pub plan_id: String,
    pub plan_sha256: String,
    pub issued_unix_seconds: u64,
    pub expires_unix_seconds: u64,
    pub expected_phrase: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuiltinLifecycleReport {
    pub schema_version: u16,
    pub contract: &'static str,
    pub valid: bool,
    pub operation: &'static str,
    pub module_id: String,
    pub display_name: Option<String>,
    pub module_version: Option<String>,
    pub state_before: Option<&'static str>,
    pub state_after: Option<&'static str>,
    pub compiled_in: bool,
    pub dry_run: bool,
    pub writes_attempted: bool,
    pub product_execution_authorized: bool,
    pub rollback_available: bool,
    pub quarantine_available: bool,
    pub plan_id: Option<String>,
    pub plan_sha256: Option<String>,
    pub state_path: String,
    pub challenge: Option<BuiltinLifecycleChallenge>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BuiltinStateDocument {
    schema_version: u16,
    contract: String,
    enabled: BTreeMap<String, bool>,
}

pub fn builtin_manifests() -> Vec<ModuleManifest> {
    vec![
        builtin(
            "first-party.inventory",
            "Inventory",
            "Path-redacted local applications, tools, services, and runtime evidence.",
            &["inventory.read", "inventory.apps", "inventory.tools"],
            RiskLevel::ReadOnly,
            ModuleSafety::core_read_only(),
        ),
        builtin(
            "first-party.updater",
            "Updater",
            "Installed-only Homebrew formula and cask update review with exact confirmation.",
            &["updates.review", "updates.homebrew", "updates.execute"],
            RiskLevel::MutatingGated,
            ModuleSafety {
                mutates_system: true,
                requires_confirmation: true,
                dry_run_required: true,
                quarantine_supported: false,
                remote_execution_allowed: false,
            },
        ),
        builtin(
            "first-party.uninstall",
            "Uninstall",
            "Manager-owned uninstall review; protected and uncertain ownership stays blocked.",
            &["uninstall.review", "uninstall.manager"],
            RiskLevel::MutatingGated,
            ModuleSafety {
                mutates_system: true,
                requires_confirmation: true,
                dry_run_required: true,
                quarantine_supported: false,
                remote_execution_allowed: false,
            },
        ),
        builtin(
            "first-party.leftovers",
            "Leftovers",
            "Exact runtime-owned leftover review with quarantine and restore evidence.",
            &[
                "leftovers.review",
                "filesystem.quarantine",
                "filesystem.restore",
            ],
            RiskLevel::MutatingGated,
            ModuleSafety {
                mutates_system: true,
                requires_confirmation: true,
                dry_run_required: true,
                quarantine_supported: true,
                remote_execution_allowed: false,
            },
        ),
        builtin(
            "first-party.cache",
            "Cache",
            "Ownership-aware cache review with exact local quarantine and recovery.",
            &[
                "cache.review",
                "filesystem.quarantine",
                "filesystem.restore",
            ],
            RiskLevel::MutatingGated,
            ModuleSafety {
                mutates_system: true,
                requires_confirmation: true,
                dry_run_required: true,
                quarantine_supported: true,
                remote_execution_allowed: false,
            },
        ),
        builtin(
            "first-party.security-integrity",
            "Security and integrity",
            "Evidence-only integrity checks; no malware-removal or unsupported assurance claims.",
            &["integrity.review", "security.review"],
            RiskLevel::ReadOnly,
            ModuleSafety::core_read_only(),
        ),
        builtin(
            "first-party.report-export",
            "Report export",
            "Deterministic privacy-reviewed local support report generation.",
            &["report.generate", "report.export"],
            RiskLevel::ReadOnly,
            ModuleSafety::core_read_only(),
        ),
    ]
}

pub fn builtin_ids() -> Vec<String> {
    builtin_manifests()
        .into_iter()
        .map(|manifest| manifest.id)
        .collect()
}

pub fn state_path(store_root: Option<&Path>) -> PathBuf {
    let store = store_for_root(store_root);
    PathBuf::from(store.state_root).join(BUILTIN_STATE_FILE)
}

pub fn load_enabled_state(
    store_root: Option<&Path>,
) -> Result<(BTreeMap<String, bool>, String), String> {
    let path = state_path(store_root);
    if !path.exists() {
        return Ok((default_enabled_state(), "compiled_default".to_string()));
    }
    let bytes = fs::read(&path).map_err(|error| format!("read built-in module state: {error}"))?;
    if bytes.len() as u64 > MAX_MODULE_STATE_BYTES {
        return Err("built-in module state exceeds its foundation byte ceiling".to_string());
    }
    let document: BuiltinStateDocument = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse built-in module state: {error}"))?;
    validate_state(&document)?;
    Ok((document.enabled, "local_state".to_string()))
}

pub fn require_enabled(module_id: &str, store_root: Option<&Path>) -> Result<(), String> {
    let (enabled, _) = load_enabled_state(store_root)?;
    if enabled.get(module_id).copied().unwrap_or(true) {
        Ok(())
    } else {
        Err(format!(
            "built-in capability {module_id} is disabled; run `rz0 modules builtin enable --module-id {module_id} --dry-run` before applying an enable plan"
        ))
    }
}

pub fn lifecycle_report(request: &BuiltinLifecycleRequest) -> BuiltinLifecycleReport {
    let dry_run = matches!(request.mode, BuiltinLifecycleMode::DryRun);
    let store = store_for_root(request.store_root.as_deref());
    let path = PathBuf::from(&store.state_root).join(BUILTIN_STATE_FILE);
    let mut report = empty_report(request, &path, dry_run);
    let Some(manifest) = builtin_manifests()
        .into_iter()
        .find(|manifest| manifest.id == request.module_id)
    else {
        report
            .errors
            .push("module is not a shipped first-party built-in".to_string());
        return report;
    };
    report.display_name = Some(manifest.display_name.clone());
    report.module_version = Some(manifest.version.clone());
    let (enabled, source) = match load_enabled_state(request.store_root.as_deref()) {
        Ok(value) => value,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };
    let before = enabled.get(&request.module_id).copied().unwrap_or(true);
    report.state_before = Some(state_label(before));
    if request.operation == BuiltinOperation::Update {
        report.warnings.push(
            "built-in bytes are updated with the runtime.zero executable; use the package manager lane for supported software updates".to_string(),
        );
        report
            .errors
            .push("built-in module update is not an independent operation".to_string());
        return report;
    }
    let desired = request
        .operation
        .desired_enabled()
        .expect("non-update operation");
    let plan_id = format!(
        "builtin.{}.{}",
        request.operation.label(),
        short_digest(format!("{}:{}:{}", request.module_id, before, desired).as_bytes())
    );
    let plan_sha256 = sha256(format!("{plan_id}\0{}\0{source}", request.module_id).as_bytes());
    let issued = match request.mode {
        BuiltinLifecycleMode::DryRun => now_unix_seconds(),
        BuiltinLifecycleMode::Apply {
            challenge_issued_unix_seconds,
            ..
        } => challenge_issued_unix_seconds,
    };
    let challenge = BuiltinLifecycleChallenge {
        plan_id: plan_id.clone(),
        plan_sha256: plan_sha256.clone(),
        issued_unix_seconds: issued,
        expires_unix_seconds: issued.saturating_add(300),
        expected_phrase: format!(
            "confirm {} {}",
            request.operation.label(),
            short_digest(plan_id.as_bytes())
        ),
    };
    report.valid = true;
    report.plan_id = Some(plan_id);
    report.plan_sha256 = Some(plan_sha256);
    report.state_after = Some(state_label(desired));
    report.challenge = Some(challenge.clone());
    report.warnings.push(
        "built-in lifecycle changes local availability only; module bytes remain compiled into the runtime".to_string(),
    );
    if dry_run {
        return report;
    }
    let BuiltinLifecycleMode::Apply {
        challenge_issued_unix_seconds,
        confirmation,
    } = &request.mode
    else {
        unreachable!("dry-run returned above");
    };
    let now = now_unix_seconds();
    if *challenge_issued_unix_seconds > now
        || now > challenge.expires_unix_seconds
        || confirmation != &challenge.expected_phrase
    {
        report.valid = false;
        report.errors.push(
            "built-in lifecycle confirmation is expired or does not match the exact plan"
                .to_string(),
        );
        return report;
    }
    let init = store_init_report(
        &[],
        match request.store_root.clone() {
            Some(root) => StoreInitOptions::with_store_root(StoreInitMode::Apply, root),
            None => StoreInitOptions::new(StoreInitMode::Apply),
        },
    );
    if init.status.is_blocked() {
        report.valid = false;
        report.errors.push(format!(
            "local state initialization blocked: {:?}",
            init.status
        ));
        return report;
    }
    let mut next = enabled;
    next.insert(request.module_id.clone(), desired);
    match write_enabled_state(&store, &next) {
        Ok(()) => {
            report.writes_attempted = true;
            report.state_after = Some(state_label(desired));
            report
                .warnings
                .push("local built-in availability state committed and re-read".to_string());
        }
        Err(error) => {
            report.valid = false;
            report.writes_attempted = true;
            report.errors.push(error);
        }
    }
    report
}

fn builtin(
    id: &str,
    display_name: &str,
    summary: &str,
    capabilities: &[&str],
    risk_level: RiskLevel,
    safety: ModuleSafety,
) -> ModuleManifest {
    let mut manifest = ModuleManifest::new(
        id,
        display_name,
        env!("CARGO_PKG_VERSION"),
        "runtime.zero",
        ModuleKind::FirstPartyModule,
        ModuleStatus::Active,
        summary,
        capabilities,
        &["macos"],
        risk_level,
        safety,
    );
    manifest.availability = crate::module_manifest::ModuleAvailability::BuiltInCapability;
    manifest
}

fn empty_report(
    request: &BuiltinLifecycleRequest,
    _path: &Path,
    dry_run: bool,
) -> BuiltinLifecycleReport {
    BuiltinLifecycleReport {
        schema_version: BUILTIN_STATE_SCHEMA_VERSION,
        contract: "builtin_module_lifecycle",
        valid: false,
        operation: request.operation.label(),
        module_id: request.module_id.clone(),
        display_name: None,
        module_version: None,
        state_before: None,
        state_after: None,
        compiled_in: true,
        dry_run,
        writes_attempted: false,
        product_execution_authorized: false,
        rollback_available: true,
        quarantine_available: false,
        plan_id: None,
        plan_sha256: None,
        state_path: "state/builtin-modules.json".to_string(),
        challenge: None,
        errors: Vec::new(),
        warnings: Vec::new(),
    }
}

fn store_for_root(store_root: Option<&Path>) -> ModuleStorePlan {
    match store_root {
        Some(root) => {
            module_store_plan_for_data_root(root.to_path_buf(), None, None, "built-in module state")
        }
        None => module_store_plan(None, None, "built-in module state"),
    }
}

fn default_enabled_state() -> BTreeMap<String, bool> {
    builtin_manifests()
        .into_iter()
        .map(|manifest| (manifest.id, true))
        .collect()
}

fn validate_state(document: &BuiltinStateDocument) -> Result<(), String> {
    if document.schema_version != BUILTIN_STATE_SCHEMA_VERSION
        || document.contract != "builtin_module_state"
    {
        return Err("built-in module state schema or contract is unsupported".to_string());
    }
    let known = builtin_manifests()
        .into_iter()
        .map(|manifest| manifest.id)
        .collect::<std::collections::BTreeSet<_>>();
    if document.enabled.keys().any(|id| !known.contains(id)) {
        return Err("built-in module state contains an unknown module".to_string());
    }
    Ok(())
}

fn write_enabled_state(
    store: &ModuleStorePlan,
    enabled: &BTreeMap<String, bool>,
) -> Result<(), String> {
    let document = BuiltinStateDocument {
        schema_version: BUILTIN_STATE_SCHEMA_VERSION,
        contract: "builtin_module_state".to_string(),
        enabled: enabled.clone(),
    };
    let bytes = serde_json::to_vec_pretty(&document)
        .map_err(|error| format!("serialize built-in module state: {error}"))?;
    if bytes.len() as u64 > MAX_MODULE_STATE_BYTES {
        return Err("built-in module state exceeds its foundation byte ceiling".to_string());
    }
    let state = rz0_secure_fs::SecureDirectory::open(Path::new(&store.state_root))
        .map_err(|error| format!("open private module state root: {error}"))?;
    let pending_name = format!("{BUILTIN_STATE_FILE}.pending-{}", std::process::id());
    state
        .write_new_child(OsStr::new(&pending_name), &bytes, MAX_MODULE_STATE_BYTES)
        .map_err(|error| format!("write pending built-in module state: {error}"))?;
    if Path::new(&store.state_root)
        .join(BUILTIN_STATE_FILE)
        .exists()
    {
        state
            .replace_child_atomic(
                OsStr::new(&pending_name),
                &state,
                OsStr::new(BUILTIN_STATE_FILE),
            )
            .map_err(|error| format!("publish built-in module state: {error}"))?;
    } else {
        state
            .publish_child_noreplace(
                OsStr::new(&pending_name),
                &state,
                OsStr::new(BUILTIN_STATE_FILE),
            )
            .map_err(|error| format!("publish initial built-in module state: {error}"))?;
    }
    Ok(())
}

fn state_label(enabled: bool) -> &'static str {
    if enabled { "active" } else { "disabled" }
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn short_digest(bytes: &[u8]) -> String {
    sha256(bytes)[..12].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ships_all_seven_first_party_capabilities_as_builtins() {
        let manifests = builtin_manifests();
        assert_eq!(manifests.len(), 7);
        assert!(
            manifests
                .iter()
                .all(|manifest| manifest.status == ModuleStatus::Active)
        );
        assert!(
            manifests
                .iter()
                .all(|manifest| manifest.supported_platforms == ["macos"])
        );
    }

    #[test]
    fn dry_run_enable_is_exact_and_non_mutating() {
        let report = lifecycle_report(&BuiltinLifecycleRequest {
            operation: BuiltinOperation::Enable,
            module_id: "first-party.inventory".to_string(),
            store_root: Some(PathBuf::from("/definitely/not-created")),
            mode: BuiltinLifecycleMode::DryRun,
        });
        assert!(report.valid);
        assert!(report.dry_run);
        assert!(!report.writes_attempted);
        assert_eq!(report.state_after, Some("active"));
        assert!(report.challenge.is_some());
    }
}
