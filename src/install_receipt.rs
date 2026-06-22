use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

use crate::install_receipt_schema::{InstallReceiptFile, validate_receipt_content};
use crate::installed_registry::InstalledRegistryRecordStatus;
use crate::installed_registry_path::validate_store_relative_path;

pub const INSTALL_RECEIPT_SCHEMA_VERSION: u16 = 1;
const MAX_RECEIPT_BYTES: u64 = 128 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReceiptInventoryReport {
    pub overall_state: ReceiptInventoryState,
    pub checked_count: usize,
    pub absent_count: usize,
    pub valid_count: usize,
    pub invalid_count: usize,
    pub unreadable_count: usize,
    pub unsupported_schema_count: usize,
    pub module_mismatch_count: usize,
    pub unsafe_path_count: usize,
    pub receipts: Vec<InstallReceiptReport>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptInventoryState {
    NotReferenced,
    Absent,
    Valid,
    Invalid,
    Unreadable,
    UnsupportedSchema,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallReceiptReport {
    pub path: String,
    pub reference_path: String,
    pub status: InstallReceiptState,
    pub schema_version: Option<u16>,
    pub module_id: Option<String>,
    pub module_version: Option<String>,
    pub module_matches_registry: bool,
    pub write_entry_count: usize,
    pub rollback_supported: Option<bool>,
    pub quarantine_supported: Option<bool>,
    pub unsafe_path_count: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallReceiptState {
    Absent,
    Valid,
    Invalid,
    Unreadable,
    UnsupportedSchema,
}

pub fn receipt_inventory_report(
    state_root: &Path,
    records: &[InstalledRegistryRecordStatus],
) -> ReceiptInventoryReport {
    let receipts = records
        .iter()
        .filter(|record| record.valid)
        .map(|record| {
            let path = receipt_full_path(state_root, &record.receipt_path);
            install_receipt_report(&path, &record.receipt_path, &record.id, &record.version)
        })
        .collect::<Vec<_>>();
    summarize_receipts(receipts)
}

pub fn install_receipt_report(
    path: &Path,
    reference_path: &str,
    expected_id: &str,
    expected_version: &str,
) -> InstallReceiptReport {
    let base = empty_report(path, reference_path);
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return InstallReceiptReport {
                status: InstallReceiptState::Absent,
                warnings: vec!["install receipt is absent".to_string()],
                ..base
            };
        }
        Err(err) => return unreadable(base, format!("metadata error: {err}")),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return invalid(base, "receipt path is not a regular file".to_string());
    }
    if metadata.len() > MAX_RECEIPT_BYTES {
        return invalid(base, "receipt file is too large".to_string());
    }
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => return unreadable(base, format!("read error: {err}")),
    };
    receipt_report_from_source(base, &source, expected_id, expected_version)
}

fn receipt_full_path(state_root: &Path, receipt_path: &str) -> PathBuf {
    if validate_store_relative_path(receipt_path).is_ok() {
        state_root.join(receipt_path)
    } else {
        state_root.join("__invalid_receipt_reference__")
    }
}

fn receipt_report_from_source(
    base: InstallReceiptReport,
    source: &str,
    expected_id: &str,
    expected_version: &str,
) -> InstallReceiptReport {
    if source.trim().is_empty() {
        return invalid(base, "receipt file is empty".to_string());
    }
    let receipt = match serde_json::from_str::<InstallReceiptFile>(source) {
        Ok(receipt) => receipt,
        Err(err) => return invalid(base, format!("malformed receipt JSON or shape: {err}")),
    };
    validate_receipt_file(base, receipt, expected_id, expected_version)
}

fn validate_receipt_file(
    mut report: InstallReceiptReport,
    receipt: InstallReceiptFile,
    expected_id: &str,
    expected_version: &str,
) -> InstallReceiptReport {
    report.schema_version = Some(receipt.schema_version);
    report.module_id = Some(receipt.module.id.clone());
    report.module_version = Some(receipt.module.version.clone());
    report.write_entry_count = receipt.write_set.len();
    report.rollback_supported = Some(receipt.rollback.supported);
    report.quarantine_supported = Some(receipt.quarantine.supported);
    if receipt.schema_version != INSTALL_RECEIPT_SCHEMA_VERSION {
        report.status = InstallReceiptState::UnsupportedSchema;
        report.errors.push(format!(
            "schema_version must be {INSTALL_RECEIPT_SCHEMA_VERSION}"
        ));
    }
    validate_receipt_content(&receipt, &mut report.errors);
    report.module_matches_registry =
        receipt.module.id == expected_id && receipt.module.version == expected_version;
    if !report.module_matches_registry {
        report.errors.push(format!(
            "receipt module {}@{} does not match registry {expected_id}@{expected_version}",
            receipt.module.id, receipt.module.version
        ));
    }
    report.unsafe_path_count = report
        .errors
        .iter()
        .filter(|error| error.contains("path"))
        .count();
    if report.status != InstallReceiptState::UnsupportedSchema {
        report.status = if report.errors.is_empty() {
            InstallReceiptState::Valid
        } else {
            InstallReceiptState::Invalid
        };
    }
    report
}

fn summarize_receipts(receipts: Vec<InstallReceiptReport>) -> ReceiptInventoryReport {
    let checked_count = receipts.len();
    let absent_count = count_status(&receipts, InstallReceiptState::Absent);
    let valid_count = count_status(&receipts, InstallReceiptState::Valid);
    let invalid_count = count_status(&receipts, InstallReceiptState::Invalid);
    let unreadable_count = count_status(&receipts, InstallReceiptState::Unreadable);
    let unsupported_schema_count = count_status(&receipts, InstallReceiptState::UnsupportedSchema);
    let module_mismatch_count = receipts
        .iter()
        .filter(|receipt| {
            !receipt.module_matches_registry && receipt.status != InstallReceiptState::Absent
        })
        .count();
    let unsafe_path_count = receipts
        .iter()
        .map(|receipt| receipt.unsafe_path_count)
        .sum();
    ReceiptInventoryReport {
        overall_state: receipt_overall_state(
            checked_count,
            absent_count,
            invalid_count,
            unreadable_count,
            unsupported_schema_count,
        ),
        checked_count,
        absent_count,
        valid_count,
        invalid_count,
        unreadable_count,
        unsupported_schema_count,
        module_mismatch_count,
        unsafe_path_count,
        receipts,
    }
}

fn receipt_overall_state(
    checked_count: usize,
    absent_count: usize,
    invalid_count: usize,
    unreadable_count: usize,
    unsupported_schema_count: usize,
) -> ReceiptInventoryState {
    if checked_count == 0 {
        ReceiptInventoryState::NotReferenced
    } else if unreadable_count > 0 {
        ReceiptInventoryState::Unreadable
    } else if unsupported_schema_count > 0 {
        ReceiptInventoryState::UnsupportedSchema
    } else if invalid_count > 0 {
        ReceiptInventoryState::Invalid
    } else if absent_count > 0 {
        ReceiptInventoryState::Absent
    } else {
        ReceiptInventoryState::Valid
    }
}

fn count_status(receipts: &[InstallReceiptReport], status: InstallReceiptState) -> usize {
    receipts
        .iter()
        .filter(|receipt| receipt.status == status)
        .count()
}

fn empty_report(path: &Path, reference_path: &str) -> InstallReceiptReport {
    InstallReceiptReport {
        path: path.display().to_string(),
        reference_path: reference_path.to_string(),
        status: InstallReceiptState::Absent,
        schema_version: None,
        module_id: None,
        module_version: None,
        module_matches_registry: false,
        write_entry_count: 0,
        rollback_supported: None,
        quarantine_supported: None,
        unsafe_path_count: 0,
        errors: Vec::new(),
        warnings: Vec::new(),
    }
}

fn invalid(mut report: InstallReceiptReport, error: String) -> InstallReceiptReport {
    report.status = InstallReceiptState::Invalid;
    report.errors.push(error);
    report
}

fn unreadable(mut report: InstallReceiptReport, error: String) -> InstallReceiptReport {
    report.status = InstallReceiptState::Unreadable;
    report.errors.push(error);
    report
}
