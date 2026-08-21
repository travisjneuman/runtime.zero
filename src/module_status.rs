use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};

use crate::brand;
use crate::install_receipt::{InstallReceiptState, ReceiptInventoryState};
use crate::installed_registry::{InstalledRegistryRecordStatus, InstalledRegistryState};
use crate::module_registry;
use crate::module_validation::load_manifest_file;
use crate::store_status::{StoreOverallState, store_status_report, store_status_report_for_root};
use rz0_module_lifecycle::ModuleLifecycleState;
use serde::Serialize;

pub const MODULE_STATUS_SCHEMA_VERSION: u16 = 1;
pub const MODULE_STATUS_CONTRACT: &str = "module_lifecycle_status";

const SAFETY_NOTE: &str = "Read-only module lifecycle status; no module files, registry records, receipts, or runtime processes were changed.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModuleStatusReport {
    pub schema_version: u16,
    pub contract: &'static str,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub product_execution_authorized: bool,
    pub lifecycle_execution_available: bool,
    pub store_state: StoreOverallState,
    pub registry_state: InstalledRegistryState,
    pub receipt_state: ReceiptInventoryState,
    pub installed_module_count: usize,
    pub inactive_module_count: usize,
    pub degraded_module_count: usize,
    pub planned_module_family_count: usize,
    pub modules: Vec<ModuleStatusEntry>,
    pub warnings: Vec<&'static str>,
    pub guidance: Vec<&'static str>,
    pub safety_note: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModuleStatusEntry {
    pub id: String,
    pub version: String,
    pub state: ModuleLifecycleState,
    pub receipt_state: Option<InstallReceiptState>,
    pub activation_supported: bool,
    pub invocation_supported: bool,
    pub reason: &'static str,
    pub errors: Vec<&'static str>,
}

pub fn module_status_report(args: &[String], store_root: Option<PathBuf>) -> ModuleStatusReport {
    let store = match store_root {
        Some(root) => store_status_report_for_root(args, Some(root)),
        None => store_status_report(args),
    };
    module_status_report_from_store(&store)
}

pub fn module_status_report_from_store(
    store: &crate::store_status::StoreStatusReport,
) -> ModuleStatusReport {
    let planned_module_family_count = module_registry::ModuleRegistryReport::empty_installed()
        .summary
        .planned_family_count;
    let mut modules = Vec::new();
    let mut receipt_index = 0;

    for (index, record) in store.registry.records.iter().enumerate() {
        if !record.valid {
            modules.push(invalid_record_status(index));
            continue;
        }

        let receipt = store.receipts.receipts.get(receipt_index);
        receipt_index += 1;
        let module_files = module_file_assessment(store, record);
        modules.push(valid_record_status(record, receipt, module_files));
    }

    let inactive_module_count = modules
        .iter()
        .filter(|module| module.state == ModuleLifecycleState::InstalledInactive)
        .count();
    let degraded_module_count = modules
        .iter()
        .filter(|module| module.state == ModuleLifecycleState::Degraded)
        .count();
    let warnings = status_warnings(store, &modules);

    ModuleStatusReport {
        schema_version: MODULE_STATUS_SCHEMA_VERSION,
        contract: MODULE_STATUS_CONTRACT,
        read_only: true,
        writes_attempted: false,
        product_execution_authorized: false,
        lifecycle_execution_available: false,
        store_state: store.overall_state,
        registry_state: store.registry.status,
        receipt_state: store.receipts.overall_state,
        installed_module_count: modules.len(),
        inactive_module_count,
        degraded_module_count,
        planned_module_family_count,
        modules,
        warnings,
        guidance: vec![
            "module status does not install, activate, invoke, repair, migrate, upgrade, or uninstall modules",
            "lifecycle execution remains unavailable until production trust, capability, isolation, rollback, transaction, and platform gates are implemented",
            "use registry, receipt, and module-byte findings as review inputs; do not infer active runtime execution from an installed record",
        ],
        safety_note: SAFETY_NOTE,
    }
}

pub fn module_status_text(report: &ModuleStatusReport) -> String {
    let mut out = format!("{} module status\n\n", brand::TITLE);
    let _ = writeln!(out, "contract: {}", report.contract);
    let _ = writeln!(out, "schema_version: {}", report.schema_version);
    let _ = writeln!(out, "mode: read-only");
    let _ = writeln!(out, "writes_attempted: no");
    let _ = writeln!(
        out,
        "store_state: {}",
        store_state_label(report.store_state)
    );
    let _ = writeln!(
        out,
        "registry_state: {}",
        registry_state_label(report.registry_state)
    );
    let _ = writeln!(
        out,
        "receipt_state: {}",
        receipt_state_label(report.receipt_state)
    );
    let _ = writeln!(
        out,
        "installed_module_count: {}",
        report.installed_module_count
    );
    let _ = writeln!(
        out,
        "inactive_module_count: {}",
        report.inactive_module_count
    );
    let _ = writeln!(
        out,
        "degraded_module_count: {}",
        report.degraded_module_count
    );
    let _ = writeln!(
        out,
        "planned_module_family_count: {}",
        report.planned_module_family_count
    );
    let _ = writeln!(out, "lifecycle_execution_available: no");
    let _ = writeln!(out, "product_execution_authorized: no");
    let _ = writeln!(out, "modules:");
    if report.modules.is_empty() {
        let _ = writeln!(out, "  none");
    } else {
        for module in &report.modules {
            let _ = writeln!(
                out,
                "  - {}@{}: {} ({})",
                module.id,
                module.version,
                state_label(module.state),
                module.reason
            );
            if let Some(receipt_state) = module.receipt_state {
                let _ = writeln!(
                    out,
                    "    receipt_state: {}",
                    install_receipt_state_label(receipt_state)
                );
            }
            for error in &module.errors {
                let _ = writeln!(out, "    finding: {error}");
            }
        }
    }
    let _ = writeln!(out, "warnings:");
    if report.warnings.is_empty() {
        let _ = writeln!(out, "  none");
    } else {
        for warning in &report.warnings {
            let _ = writeln!(out, "  - {warning}");
        }
    }
    let _ = writeln!(out, "guidance:");
    for guidance in &report.guidance {
        let _ = writeln!(out, "  - {guidance}");
    }
    let _ = writeln!(out, "safety: {}", report.safety_note);
    out
}

fn valid_record_status(
    record: &InstalledRegistryRecordStatus,
    receipt: Option<&crate::install_receipt::InstallReceiptReport>,
    module_files: ModuleFileAssessment,
) -> ModuleStatusEntry {
    match receipt.map(|report| report.status) {
        Some(InstallReceiptState::Valid) if module_files.valid => ModuleStatusEntry {
            id: record.id.clone(),
            version: record.version.clone(),
            state: ModuleLifecycleState::InstalledInactive,
            receipt_state: Some(InstallReceiptState::Valid),
            activation_supported: false,
            invocation_supported: false,
            reason: "installed record, receipt, and module bytes are valid; execution is disabled",
            errors: Vec::new(),
        },
        Some(InstallReceiptState::Valid) => ModuleStatusEntry {
            id: record.id.clone(),
            version: record.version.clone(),
            state: ModuleLifecycleState::Degraded,
            receipt_state: Some(InstallReceiptState::Valid),
            activation_supported: false,
            invocation_supported: false,
            reason: "receipt is valid but installed module bytes require review",
            errors: vec![module_files.error],
        },
        Some(status) => ModuleStatusEntry {
            id: record.id.clone(),
            version: record.version.clone(),
            state: ModuleLifecycleState::Degraded,
            receipt_state: Some(status),
            activation_supported: false,
            invocation_supported: false,
            reason: "installed record requires receipt review",
            errors: vec![receipt_error_label(status)],
        },
        None => ModuleStatusEntry {
            id: record.id.clone(),
            version: record.version.clone(),
            state: ModuleLifecycleState::Degraded,
            receipt_state: None,
            activation_supported: false,
            invocation_supported: false,
            reason: "installed record has no receipt assessment",
            errors: vec!["receipt_assessment_missing"],
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ModuleFileAssessment {
    valid: bool,
    error: &'static str,
}

fn module_file_assessment(
    store: &crate::store_status::StoreStatusReport,
    record: &InstalledRegistryRecordStatus,
) -> ModuleFileAssessment {
    let manifest_path = Path::new(&store.store.data_root).join(&record.manifest_path);
    match fs::symlink_metadata(&manifest_path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return ModuleFileAssessment {
                valid: false,
                error: "module_manifest_not_a_regular_file",
            };
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return ModuleFileAssessment {
                valid: false,
                error: "module_manifest_missing",
            };
        }
        Err(_) => {
            return ModuleFileAssessment {
                valid: false,
                error: "module_manifest_unreadable",
            };
        }
    }

    let validation = load_manifest_file(&manifest_path);
    let Some(manifest) = validation.manifest else {
        return ModuleFileAssessment {
            valid: false,
            error: "module_manifest_invalid",
        };
    };
    if !validation.valid || manifest.id != record.id || manifest.version != record.version {
        return ModuleFileAssessment {
            valid: false,
            error: "module_package_integrity_invalid",
        };
    }
    ModuleFileAssessment {
        valid: true,
        error: "none",
    }
}

fn invalid_record_status(index: usize) -> ModuleStatusEntry {
    ModuleStatusEntry {
        id: format!("invalid-module-{}", index + 1),
        version: "unknown".to_string(),
        state: ModuleLifecycleState::Degraded,
        receipt_state: None,
        activation_supported: false,
        invocation_supported: false,
        reason: "registry record is invalid; identifying fields are redacted",
        errors: vec!["registry_record_invalid"],
    }
}

fn status_warnings(
    store: &crate::store_status::StoreStatusReport,
    modules: &[ModuleStatusEntry],
) -> Vec<&'static str> {
    let mut warnings = Vec::new();
    match store.registry.status {
        InstalledRegistryState::Absent | InstalledRegistryState::Empty => {
            warnings.push("no_installed_module_records")
        }
        InstalledRegistryState::Invalid | InstalledRegistryState::Unreadable => {
            warnings.push("installed_module_registry_requires_review")
        }
        InstalledRegistryState::Valid => {}
    }
    match store.receipts.overall_state {
        ReceiptInventoryState::Absent => warnings.push("an_installed_module_receipt_is_missing"),
        ReceiptInventoryState::Invalid
        | ReceiptInventoryState::Unreadable
        | ReceiptInventoryState::UnsupportedSchema => {
            warnings.push("an_installed_module_receipt_requires_review")
        }
        ReceiptInventoryState::NotReferenced | ReceiptInventoryState::Valid => {}
    }
    if modules
        .iter()
        .any(|module| module.reason.starts_with("registry record"))
    {
        warnings.push("one_or_more_registry_records_are_invalid")
    }
    if !modules.is_empty() {
        warnings.push("installed_modules_are_inactive_until_lifecycle_execution_is_authorized")
    }
    warnings.push("module_lifecycle_execution_is_unavailable");
    warnings
}

fn state_label(state: ModuleLifecycleState) -> &'static str {
    match state {
        ModuleLifecycleState::Absent => "absent",
        ModuleLifecycleState::Staged => "staged",
        ModuleLifecycleState::InstalledInactive => "installed_inactive",
        ModuleLifecycleState::Active => "active",
        ModuleLifecycleState::Degraded => "degraded",
        ModuleLifecycleState::Quarantined => "quarantined",
    }
}

fn store_state_label(state: StoreOverallState) -> &'static str {
    match state {
        StoreOverallState::NotInitialized => "not_initialized",
        StoreOverallState::Empty => "empty",
        StoreOverallState::Present => "present",
        StoreOverallState::Invalid => "invalid",
    }
}

fn registry_state_label(state: InstalledRegistryState) -> &'static str {
    match state {
        InstalledRegistryState::Absent => "absent",
        InstalledRegistryState::Empty => "empty",
        InstalledRegistryState::Valid => "valid",
        InstalledRegistryState::Invalid => "invalid",
        InstalledRegistryState::Unreadable => "unreadable",
    }
}

fn receipt_state_label(state: ReceiptInventoryState) -> &'static str {
    match state {
        ReceiptInventoryState::NotReferenced => "not_referenced",
        ReceiptInventoryState::Absent => "absent",
        ReceiptInventoryState::Valid => "valid",
        ReceiptInventoryState::Invalid => "invalid",
        ReceiptInventoryState::Unreadable => "unreadable",
        ReceiptInventoryState::UnsupportedSchema => "unsupported_schema",
    }
}

fn receipt_error_label(state: InstallReceiptState) -> &'static str {
    match state {
        InstallReceiptState::Absent => "receipt_missing",
        InstallReceiptState::Valid => "none",
        InstallReceiptState::Invalid => "receipt_invalid",
        InstallReceiptState::Unreadable => "receipt_unreadable",
        InstallReceiptState::UnsupportedSchema => "receipt_schema_unsupported",
    }
}

fn install_receipt_state_label(state: InstallReceiptState) -> &'static str {
    match state {
        InstallReceiptState::Absent => "absent",
        InstallReceiptState::Valid => "valid",
        InstallReceiptState::Invalid => "invalid",
        InstallReceiptState::Unreadable => "unreadable",
        InstallReceiptState::UnsupportedSchema => "unsupported_schema",
    }
}
