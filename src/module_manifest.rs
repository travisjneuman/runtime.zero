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
        }
    }
}

fn to_strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}
