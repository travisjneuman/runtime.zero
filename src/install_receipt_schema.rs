use serde::Deserialize;

use crate::installed_registry_path::validate_registry_path;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InstallReceiptFile {
    pub schema_version: u16,
    pub module: ReceiptModule,
    pub source: ReceiptSource,
    pub target: ReceiptTarget,
    pub integrity: ReceiptIntegrity,
    pub write_set: Vec<ReceiptWriteEntry>,
    pub rollback: ReceiptRollback,
    pub quarantine: ReceiptQuarantine,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReceiptModule {
    pub id: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReceiptSource {
    source_type: String,
    #[serde(default)]
    package_reference: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReceiptTarget {
    module_dir: String,
    manifest_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReceiptIntegrity {
    #[serde(default)]
    manifest_sha256: Option<String>,
    #[serde(default)]
    package_sha256: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReceiptWriteEntry {
    path: String,
    kind: ReceiptWriteKind,
    #[serde(default)]
    sha256: Option<String>,
    #[serde(default)]
    size_bytes: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReceiptWriteKind {
    Directory,
    File,
    Manifest,
    Receipt,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReceiptRollback {
    pub supported: bool,
    #[serde(default)]
    plan_path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReceiptQuarantine {
    pub supported: bool,
    #[serde(default)]
    record_path: Option<String>,
}

pub(crate) fn validate_receipt_content(receipt: &InstallReceiptFile, errors: &mut Vec<String>) {
    validate_id(&receipt.module.id, errors);
    validate_field(&receipt.module.version, "module.version", 40, errors);
    validate_field(
        &receipt.source.source_type,
        "source.source_type",
        40,
        errors,
    );
    if let Some(reference) = receipt.source.package_reference.as_deref() {
        validate_field(reference, "source.package_reference", 160, errors);
    }
    validate_registry_path(
        &receipt.target.module_dir,
        "target.module_dir",
        "modules/",
        None,
        errors,
    );
    validate_registry_path(
        &receipt.target.manifest_path,
        "target.manifest_path",
        "modules/",
        Some("rz0-module.json"),
        errors,
    );
    validate_hash(
        receipt.integrity.manifest_sha256.as_deref(),
        "manifest_sha256",
        errors,
    );
    validate_hash(
        receipt.integrity.package_sha256.as_deref(),
        "package_sha256",
        errors,
    );
    for entry in &receipt.write_set {
        validate_write_entry(entry, errors);
    }
    if let Some(path) = receipt.rollback.plan_path.as_deref() {
        validate_registry_path(
            path,
            "rollback.plan_path",
            "receipts/",
            Some(".json"),
            errors,
        );
    }
    if let Some(path) = receipt.quarantine.record_path.as_deref() {
        validate_registry_path(
            path,
            "quarantine.record_path",
            "quarantine/",
            Some(".json"),
            errors,
        );
    }
}

fn validate_write_entry(entry: &ReceiptWriteEntry, errors: &mut Vec<String>) {
    let (prefix, suffix) = match entry.kind {
        ReceiptWriteKind::Receipt => ("receipts/", Some(".json")),
        _ => ("modules/", None),
    };
    validate_registry_path(&entry.path, "write_set.path", prefix, suffix, errors);
    validate_hash(entry.sha256.as_deref(), "write_set.sha256", errors);
    if entry.size_bytes.is_some_and(|size| size > 64 * 1024 * 1024) {
        errors.push("write_set.size_bytes exceeds 64 MiB".to_string());
    }
}

fn validate_id(id: &str, errors: &mut Vec<String>) {
    if id.starts_with("core.") || !is_valid_module_id(id) {
        errors.push("module.id must be a non-core module id".to_string());
    }
}

fn validate_field(value: &str, name: &str, max_len: usize, errors: &mut Vec<String>) {
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

fn validate_hash(value: Option<&str>, name: &str, errors: &mut Vec<String>) {
    let Some(value) = value else {
        return;
    };
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        errors.push(format!("{name} must be a SHA-256 hex digest"));
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
