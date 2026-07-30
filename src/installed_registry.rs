use std::{collections::BTreeSet, fs, path::Path};

use rz0_registry_contract::{
    InstalledRegistry, RegistryViolation, RegistryViolationCode, decode_registry_document,
    validate_registry,
};
use serde::Serialize;

pub use rz0_registry_contract::INSTALLED_REGISTRY_SCHEMA_VERSION;
pub const MAX_REGISTRY_BYTES: u64 = rz0_resource_contract::MAX_REGISTRY_DOCUMENT_BYTES;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstalledRegistryReport {
    pub path: String,
    pub status: InstalledRegistryState,
    pub schema_version: Option<u16>,
    pub installed_module_count: usize,
    pub duplicate_ids: Vec<String>,
    pub malformed_record_count: usize,
    pub unsafe_path_count: usize,
    pub records: Vec<InstalledRegistryRecordStatus>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstalledRegistryState {
    Absent,
    Empty,
    Valid,
    Invalid,
    Unreadable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstalledRegistryRecordStatus {
    pub id: String,
    pub version: String,
    pub manifest_path: String,
    pub receipt_path: String,
    pub module_dir: Option<String>,
    pub valid: bool,
    pub errors: Vec<String>,
}

pub fn installed_registry_report(path: &Path) -> InstalledRegistryReport {
    let base = empty_report(path);
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return InstalledRegistryReport {
                status: InstalledRegistryState::Absent,
                warnings: vec!["installed module registry is absent".to_string()],
                ..base
            };
        }
        Err(error) => return unreadable(base, format!("metadata error: {error}")),
    };

    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return invalid(base, "registry path is not a regular file".to_string());
    }
    if metadata.len() == 0 {
        return InstalledRegistryReport {
            status: InstalledRegistryState::Empty,
            warnings: vec!["installed module registry file is empty".to_string()],
            ..base
        };
    }
    if metadata.len() > MAX_REGISTRY_BYTES {
        return invalid(base, "registry file is too large".to_string());
    }

    let source = match fs::read(path) {
        Ok(source) => source,
        Err(error) => return unreadable(base, format!("read error: {error}")),
    };
    registry_report_from_source(base, &source)
}

fn registry_report_from_source(
    base: InstalledRegistryReport,
    source: &[u8],
) -> InstalledRegistryReport {
    if source.iter().all(u8::is_ascii_whitespace) {
        return InstalledRegistryReport {
            status: InstalledRegistryState::Empty,
            warnings: vec!["installed module registry file is empty".to_string()],
            ..base
        };
    }
    match decode_registry_document(source) {
        Ok(registry) => validate_registry_file(base, registry),
        Err(error) => invalid(base, format!("malformed registry JSON or shape: {error}")),
    }
}

fn validate_registry_file(
    mut report: InstalledRegistryReport,
    registry: InstalledRegistry,
) -> InstalledRegistryReport {
    report.schema_version = Some(registry.schema_version);
    report.records = registry
        .modules
        .iter()
        .map(|record| InstalledRegistryRecordStatus {
            id: record.id.clone(),
            version: record.version.clone(),
            manifest_path: record.manifest_path.clone(),
            receipt_path: record.receipt_path.clone(),
            module_dir: record.module_dir.clone(),
            valid: true,
            errors: Vec::new(),
        })
        .collect();

    let validation = validate_registry(&registry);
    for violation in &validation.violations {
        let message = violation_message(violation);
        if let Some(index) = violation.record_index {
            if let Some(record) = report.records.get_mut(index) {
                record.valid = false;
                record.errors.push(message);
            }
        } else {
            report.errors.push(message);
        }
    }
    let duplicate_indexes = validation
        .violations
        .iter()
        .filter(|item| item.code == RegistryViolationCode::DuplicateModuleId)
        .filter_map(|item| item.record_index)
        .collect::<BTreeSet<_>>();
    report.duplicate_ids = duplicate_indexes
        .iter()
        .filter_map(|index| report.records.get(*index).map(|record| record.id.clone()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    if !report.duplicate_ids.is_empty() {
        report.errors.push(format!(
            "duplicate installed module id(s): {}",
            report.duplicate_ids.join(", ")
        ));
    }
    report.installed_module_count = report.records.len();
    report.malformed_record_count = report.records.iter().filter(|record| !record.valid).count();
    report.unsafe_path_count = report
        .records
        .iter()
        .flat_map(|record| &record.errors)
        .filter(|error| error.contains("path") || error.contains("directory"))
        .count();
    report.status = if validation.valid {
        InstalledRegistryState::Valid
    } else {
        InstalledRegistryState::Invalid
    };
    report
}

fn violation_message(violation: &RegistryViolation) -> String {
    match violation.code {
        RegistryViolationCode::UnsupportedSchema => {
            format!("schema_version must be {INSTALLED_REGISTRY_SCHEMA_VERSION}")
        }
        RegistryViolationCode::TooManyRecords => "registry has too many module records".to_string(),
        RegistryViolationCode::ReservedModuleId => {
            "installed modules must not use the reserved core.* id prefix".to_string()
        }
        RegistryViolationCode::InvalidModuleId => {
            "id must use lowercase letters, digits, dots, or hyphens".to_string()
        }
        RegistryViolationCode::InvalidVersion => "version is invalid".to_string(),
        RegistryViolationCode::InvalidManifestPath => {
            "manifest_path path is unsafe or does not match id/version".to_string()
        }
        RegistryViolationCode::InvalidReceiptPath => {
            "receipt_path path is unsafe or non-canonical".to_string()
        }
        RegistryViolationCode::InvalidModuleDirectory => {
            "module_dir directory is unsafe or does not match id/version".to_string()
        }
        RegistryViolationCode::DuplicateModuleId => "duplicate installed module id".to_string(),
        RegistryViolationCode::NonCanonicalOrder => {
            "installed module records must use unique ascending id order".to_string()
        }
    }
}

fn empty_report(path: &Path) -> InstalledRegistryReport {
    InstalledRegistryReport {
        path: path.display().to_string(),
        status: InstalledRegistryState::Absent,
        schema_version: None,
        installed_module_count: 0,
        duplicate_ids: Vec::new(),
        malformed_record_count: 0,
        unsafe_path_count: 0,
        records: Vec::new(),
        errors: Vec::new(),
        warnings: Vec::new(),
    }
}

fn invalid(mut report: InstalledRegistryReport, error: String) -> InstalledRegistryReport {
    report.status = InstalledRegistryState::Invalid;
    report.errors.push(error);
    report
}

fn unreadable(mut report: InstalledRegistryReport, error: String) -> InstalledRegistryReport {
    report.status = InstalledRegistryState::Unreadable;
    report.errors.push(error);
    report
}
