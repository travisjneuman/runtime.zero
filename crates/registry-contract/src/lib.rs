use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const INSTALLED_REGISTRY_SCHEMA_VERSION: u16 = 1;
pub const INSTALLED_MODULE_LIFECYCLE_STATE: &str = "installed_inactive";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledRegistry {
    pub schema_version: u16,
    pub modules: Vec<InstalledModuleRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledModuleRecord {
    pub id: String,
    pub version: String,
    pub manifest_path: String,
    pub receipt_path: String,
    #[serde(default = "default_lifecycle_state")]
    pub lifecycle_state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub module_dir: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RegistryViolationCode {
    UnsupportedSchema,
    TooManyRecords,
    ReservedModuleId,
    InvalidModuleId,
    InvalidVersion,
    InvalidManifestPath,
    InvalidReceiptPath,
    InvalidLifecycleState,
    InvalidModuleDirectory,
    DuplicateModuleId,
    NonCanonicalOrder,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RegistryViolation {
    pub code: RegistryViolationCode,
    pub record_index: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryValidation {
    pub valid: bool,
    pub violations: Vec<RegistryViolation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryDocumentErrorCode {
    Empty,
    LimitExceeded,
    Malformed,
    Invalid,
    Serialization,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryDocumentError {
    pub code: RegistryDocumentErrorCode,
    detail: String,
    pub violations: Vec<RegistryViolation>,
}

impl fmt::Display for RegistryDocumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for RegistryDocumentError {}

pub fn decode_registry_document(bytes: &[u8]) -> Result<InstalledRegistry, RegistryDocumentError> {
    if bytes.len() as u64 > rz0_resource_contract::MAX_REGISTRY_DOCUMENT_BYTES {
        return Err(document_error(
            RegistryDocumentErrorCode::LimitExceeded,
            "installed registry document exceeds its foundation byte ceiling",
            Vec::new(),
        ));
    }
    if bytes.is_empty() || bytes.iter().all(u8::is_ascii_whitespace) {
        return Err(document_error(
            RegistryDocumentErrorCode::Empty,
            "installed registry document is empty",
            Vec::new(),
        ));
    }
    serde_json::from_slice::<InstalledRegistry>(bytes).map_err(|error| {
        document_error(
            RegistryDocumentErrorCode::Malformed,
            format!("malformed installed registry JSON or shape: {error}"),
            Vec::new(),
        )
    })
}

pub fn parse_registry_document(bytes: &[u8]) -> Result<InstalledRegistry, RegistryDocumentError> {
    let registry = decode_registry_document(bytes)?;
    let validation = validate_registry(&registry);
    if validation.valid {
        Ok(registry)
    } else {
        Err(document_error(
            RegistryDocumentErrorCode::Invalid,
            "installed registry violates the schema-1 registry contract",
            validation.violations,
        ))
    }
}

pub fn validate_registry(registry: &InstalledRegistry) -> RegistryValidation {
    let mut violations = Vec::new();
    if registry.schema_version != INSTALLED_REGISTRY_SCHEMA_VERSION {
        push(
            &mut violations,
            RegistryViolationCode::UnsupportedSchema,
            None,
        );
    }
    if registry.modules.len() > rz0_resource_contract::MAX_INSTALLED_MODULE_RECORDS {
        push(&mut violations, RegistryViolationCode::TooManyRecords, None);
    }

    let mut counts = BTreeMap::<&str, usize>::new();
    for (index, record) in registry.modules.iter().enumerate() {
        *counts.entry(record.id.as_str()).or_default() += 1;
        if record.id.starts_with("core.") {
            push(
                &mut violations,
                RegistryViolationCode::ReservedModuleId,
                Some(index),
            );
        }
        if !rz0_validation_contract::valid_module_id(&record.id) {
            push(
                &mut violations,
                RegistryViolationCode::InvalidModuleId,
                Some(index),
            );
        }
        if !rz0_validation_contract::valid_version(&record.version) {
            push(
                &mut violations,
                RegistryViolationCode::InvalidVersion,
                Some(index),
            );
        }

        let expected_directory = format!("modules/{}/{}", record.id, record.version);
        let expected_manifest = format!("{expected_directory}/rz0-module.json");
        if !valid_relative(&record.manifest_path) || record.manifest_path != expected_manifest {
            push(
                &mut violations,
                RegistryViolationCode::InvalidManifestPath,
                Some(index),
            );
        }
        let receipt_id = record
            .receipt_path
            .strip_prefix("receipts/")
            .and_then(|path| path.strip_suffix(".json"));
        if !valid_relative(&record.receipt_path)
            || receipt_id.is_none_or(|id| {
                id.contains('/') || !rz0_validation_contract::valid_ledger_id(id, 100)
            })
        {
            push(
                &mut violations,
                RegistryViolationCode::InvalidReceiptPath,
                Some(index),
            );
        }
        if record.lifecycle_state != INSTALLED_MODULE_LIFECYCLE_STATE {
            push(
                &mut violations,
                RegistryViolationCode::InvalidLifecycleState,
                Some(index),
            );
        }
        if record
            .module_dir
            .as_deref()
            .is_some_and(|directory| !valid_relative(directory) || directory != expected_directory)
        {
            push(
                &mut violations,
                RegistryViolationCode::InvalidModuleDirectory,
                Some(index),
            );
        }
        if index > 0 && registry.modules[index - 1].id >= record.id {
            push(
                &mut violations,
                RegistryViolationCode::NonCanonicalOrder,
                Some(index),
            );
        }
    }
    for (index, record) in registry.modules.iter().enumerate() {
        if counts.get(record.id.as_str()).copied().unwrap_or_default() > 1 {
            push(
                &mut violations,
                RegistryViolationCode::DuplicateModuleId,
                Some(index),
            );
        }
    }
    violations.sort();
    violations.dedup();
    RegistryValidation {
        valid: violations.is_empty(),
        violations,
    }
}

fn default_lifecycle_state() -> String {
    INSTALLED_MODULE_LIFECYCLE_STATE.to_string()
}

pub fn canonical_registry_bytes(
    registry: &InstalledRegistry,
) -> Result<Vec<u8>, RegistryDocumentError> {
    let validation = validate_registry(registry);
    if !validation.valid {
        return Err(document_error(
            RegistryDocumentErrorCode::Invalid,
            "cannot serialize an invalid installed registry",
            validation.violations,
        ));
    }
    let mut bytes = serde_json::to_vec(registry).map_err(|error| {
        document_error(
            RegistryDocumentErrorCode::Serialization,
            format!("serialize installed registry: {error}"),
            Vec::new(),
        )
    })?;
    bytes.push(b'\n');
    if bytes.len() as u64 > rz0_resource_contract::MAX_REGISTRY_DOCUMENT_BYTES {
        return Err(document_error(
            RegistryDocumentErrorCode::LimitExceeded,
            "serialized installed registry exceeds its foundation byte ceiling",
            Vec::new(),
        ));
    }
    Ok(bytes)
}

pub fn registry_sha256(registry: &InstalledRegistry) -> Result<String, RegistryDocumentError> {
    Ok(bytes_sha256(&canonical_registry_bytes(registry)?))
}

pub fn bytes_sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn valid_relative(value: &str) -> bool {
    rz0_validation_contract::valid_contract_relative_path(value)
        && !rz0_validation_contract::is_absolute_local_path(value)
}

fn push(
    violations: &mut Vec<RegistryViolation>,
    code: RegistryViolationCode,
    record_index: Option<usize>,
) {
    violations.push(RegistryViolation { code, record_index });
}

fn document_error(
    code: RegistryDocumentErrorCode,
    detail: impl Into<String>,
    violations: Vec<RegistryViolation>,
) -> RegistryDocumentError {
    RegistryDocumentError {
        code,
        detail: detail.into(),
        violations,
    }
}
