use serde::{Deserialize, Serialize};

pub const MODULE_SCHEMA_VERSION: u16 = 1;
pub const MAX_MANIFEST_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleKind {
    CoreFoundation,
    FirstPartyModule,
    ThirdPartyModule,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleStatus {
    Active,
    Installed,
    Planned,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    None,
    ReadOnly,
    DryRunOnly,
    MutatingGated,
    DestructiveGated,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleSafety {
    pub mutates_system: bool,
    pub requires_confirmation: bool,
    pub dry_run_required: bool,
    pub quarantine_supported: bool,
    pub remote_execution_allowed: bool,
}

impl ModuleSafety {
    pub const fn core_read_only() -> Self {
        Self {
            mutates_system: false,
            requires_confirmation: false,
            dry_run_required: false,
            quarantine_supported: false,
            remote_execution_allowed: false,
        }
    }

    pub const fn module_contract_default() -> Self {
        Self {
            mutates_system: false,
            requires_confirmation: true,
            dry_run_required: true,
            quarantine_supported: false,
            remote_execution_allowed: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleManifest {
    pub manifest_version: u16,
    pub id: String,
    pub display_name: String,
    pub version: String,
    pub publisher: String,
    pub kind: ModuleKind,
    pub status: ModuleStatus,
    pub summary: String,
    pub capabilities: Vec<String>,
    pub supported_platforms: Vec<String>,
    pub risk_level: RiskLevel,
    pub safety: ModuleSafety,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub integrity: Option<PackageIntegrity>,
}

impl ModuleManifest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: &str,
        display_name: &str,
        version: &str,
        publisher: &str,
        kind: ModuleKind,
        status: ModuleStatus,
        summary: &str,
        capabilities: &[&str],
        supported_platforms: &[&str],
        risk_level: RiskLevel,
        safety: ModuleSafety,
    ) -> Self {
        Self {
            manifest_version: MODULE_SCHEMA_VERSION,
            id: id.to_string(),
            display_name: display_name.to_string(),
            version: version.to_string(),
            publisher: publisher.to_string(),
            kind,
            status,
            summary: summary.to_string(),
            capabilities: to_strings(capabilities),
            supported_platforms: to_strings(supported_platforms),
            risk_level,
            safety,
            integrity: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageIntegrity {
    pub package_format: PackageFormat,
    pub root_policy: IntegrityRootPolicy,
    pub hash_algorithm: HashAlgorithm,
    pub files: Vec<PackageFileIntegrity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<SignatureMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<ProvenanceMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageFormat {
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IntegrityRootPolicy {
    ManifestDirectory,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HashAlgorithm {
    Sha256,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackageFileIntegrity {
    pub path: String,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
    #[serde(default)]
    pub role: PackageFileRole,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageFileRole {
    Manifest,
    Payload,
    Docs,
    License,
    Config,
    Data,
    TestFixture,
}

impl Default for PackageFileRole {
    fn default() -> Self {
        Self::Payload
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignatureMetadata {
    pub scheme: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceMetadata {
    pub source: String,
    pub publisher: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

fn to_strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}
