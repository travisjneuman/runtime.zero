use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::installed_registry_path::validate_registry_path;

pub const INSTALLED_REGISTRY_SCHEMA_VERSION: u16 = 1;
pub const MAX_REGISTRY_BYTES: u64 = 128 * 1024;

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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InstalledRegistryFile {
    schema_version: u16,
    modules: Vec<InstalledRegistryRecord>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InstalledRegistryRecord {
    id: String,
    version: String,
    manifest_path: String,
    receipt_path: String,
    #[serde(default)]
    module_dir: Option<String>,
}

pub fn installed_registry_report(path: &Path) -> InstalledRegistryReport {
    let base = empty_report(path);
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return InstalledRegistryReport {
                status: InstalledRegistryState::Absent,
                warnings: vec!["installed module registry is absent".to_string()],
                ..base
            };
        }
        Err(err) => return unreadable(base, format!("metadata error: {err}")),
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

    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => return unreadable(base, format!("read error: {err}")),
    };
    registry_report_from_source(path, &source)
}

fn registry_report_from_source(path: &Path, source: &str) -> InstalledRegistryReport {
    let base = empty_report(path);
    if source.trim().is_empty() {
        return InstalledRegistryReport {
            status: InstalledRegistryState::Empty,
            warnings: vec!["installed module registry file is empty".to_string()],
            ..base
        };
    }
    let registry = match serde_json::from_str::<InstalledRegistryFile>(source) {
        Ok(registry) => registry,
        Err(err) => return invalid(base, format!("malformed registry JSON or shape: {err}")),
    };
    validate_registry_file(base, registry)
}

fn validate_registry_file(
    mut report: InstalledRegistryReport,
    registry: InstalledRegistryFile,
) -> InstalledRegistryReport {
    report.schema_version = Some(registry.schema_version);
    if registry.schema_version != INSTALLED_REGISTRY_SCHEMA_VERSION {
        report.errors.push(format!(
            "schema_version must be {INSTALLED_REGISTRY_SCHEMA_VERSION}"
        ));
    }
    report.records = registry
        .modules
        .into_iter()
        .map(validate_registry_record)
        .collect();
    flag_duplicate_ids(&mut report);
    report.installed_module_count = report.records.len();
    report.malformed_record_count = report.records.iter().filter(|record| !record.valid).count();
    report.unsafe_path_count = report
        .records
        .iter()
        .map(|record| {
            record
                .errors
                .iter()
                .filter(|error| error.contains("path"))
                .count()
        })
        .sum();
    if report.errors.is_empty() && report.malformed_record_count == 0 {
        report.status = InstalledRegistryState::Valid;
    } else {
        report.status = InstalledRegistryState::Invalid;
    }
    report
}

fn validate_registry_record(record: InstalledRegistryRecord) -> InstalledRegistryRecordStatus {
    let mut errors = Vec::new();
    validate_id(&record.id, &mut errors);
    validate_text_field(&record.version, "version", 40, &mut errors);
    validate_registry_path(
        &record.manifest_path,
        "manifest_path",
        "modules/",
        Some("rz0-module.json"),
        &mut errors,
    );
    validate_registry_path(
        &record.receipt_path,
        "receipt_path",
        "receipts/",
        Some(".json"),
        &mut errors,
    );
    if let Some(module_dir) = record.module_dir.as_deref() {
        validate_registry_path(module_dir, "module_dir", "modules/", None, &mut errors);
    }
    InstalledRegistryRecordStatus {
        id: record.id,
        version: record.version,
        manifest_path: record.manifest_path,
        receipt_path: record.receipt_path,
        module_dir: record.module_dir,
        valid: errors.is_empty(),
        errors,
    }
}

fn flag_duplicate_ids(report: &mut InstalledRegistryReport) {
    let mut counts = BTreeMap::new();
    for record in &report.records {
        *counts.entry(record.id.clone()).or_insert(0_usize) += 1;
    }
    report.duplicate_ids = counts
        .into_iter()
        .filter_map(|(id, count)| (count > 1).then_some(id))
        .collect();
    if report.duplicate_ids.is_empty() {
        return;
    }
    for record in &mut report.records {
        if report.duplicate_ids.contains(&record.id) {
            record.valid = false;
            record
                .errors
                .push(format!("duplicate installed module id '{}'", record.id));
        }
    }
    report.errors.push(format!(
        "duplicate installed module id(s): {}",
        report.duplicate_ids.join(", ")
    ));
}

fn validate_id(id: &str, errors: &mut Vec<String>) {
    if id.starts_with("core.") {
        errors.push("installed modules must not use the reserved core.* id prefix".to_string());
    }
    if !is_valid_module_id(id) {
        errors.push("id must use lowercase letters, digits, dots, or hyphens".to_string());
    }
}

fn validate_text_field(value: &str, name: &str, max_len: usize, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        errors.push(format!("{name} must not be empty"));
    }
    if value.len() > max_len {
        errors.push(format!("{name} must be at most {max_len} characters"));
    }
    if value.chars().any(char::is_control) {
        errors.push(format!("{name} must not contain control characters"));
    }
}

fn is_valid_module_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && !id.starts_with(['.', '-'])
        && !id.ends_with(['.', '-'])
        && !id.contains("..")
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-')
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
