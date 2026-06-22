use serde::Serialize;

use crate::brand;

pub const MODULE_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleKind {
    CoreFoundation,
    FirstPartyModule,
    ThirdPartyModule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleStatus {
    Active,
    Installed,
    Planned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    None,
    ReadOnly,
    DryRunOnly,
    MutatingGated,
    DestructiveGated,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModuleManifest {
    pub manifest_version: u16,
    pub id: &'static str,
    pub display_name: &'static str,
    pub version: &'static str,
    pub publisher: &'static str,
    pub kind: ModuleKind,
    pub status: ModuleStatus,
    pub summary: &'static str,
    pub capabilities: Vec<&'static str>,
    pub supported_platforms: Vec<&'static str>,
    pub risk_level: RiskLevel,
    pub safety: ModuleSafety,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModuleRegistryReport {
    pub schema_version: u16,
    pub runtime: RuntimeSummary,
    pub core: Vec<ModuleManifest>,
    pub installed_modules: Vec<ModuleManifest>,
    pub planned_module_families: Vec<ModuleManifest>,
    pub summary: RegistrySummary,
    pub safety_note: &'static str,
}

impl ModuleRegistryReport {
    pub fn empty_installed() -> Self {
        let core = core_foundation_manifests();
        let installed_modules = Vec::new();
        let planned_module_families = planned_module_family_manifests();

        Self {
            schema_version: MODULE_SCHEMA_VERSION,
            runtime: RuntimeSummary::current(),
            summary: RegistrySummary {
                core_count: core.len(),
                installed_module_count: installed_modules.len(),
                planned_family_count: planned_module_families.len(),
            },
            core,
            installed_modules,
            planned_module_families,
            safety_note: "No optional feature modules are bundled or executed by default.",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RuntimeSummary {
    pub title: &'static str,
    pub command: &'static str,
    pub version: &'static str,
    pub safety_posture: &'static str,
    pub module_schema_version: u16,
}

impl RuntimeSummary {
    pub const fn current() -> Self {
        Self {
            title: brand::TITLE,
            command: brand::COMMAND,
            version: env!("CARGO_PKG_VERSION"),
            safety_posture: brand::SAFETY_POSTURE,
            module_schema_version: MODULE_SCHEMA_VERSION,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegistrySummary {
    pub core_count: usize,
    pub installed_module_count: usize,
    pub planned_family_count: usize,
}

pub fn core_foundation_manifests() -> Vec<ModuleManifest> {
    vec![
        ModuleManifest {
            manifest_version: MODULE_SCHEMA_VERSION,
            id: "core.cli",
            display_name: "CLI core",
            version: env!("CARGO_PKG_VERSION"),
            publisher: "runtime.zero",
            kind: ModuleKind::CoreFoundation,
            status: ModuleStatus::Active,
            summary: "Command parsing, output routing, and safe bootstrap commands.",
            capabilities: vec!["command-routing", "text-output", "json-output"],
            supported_platforms: vec!["windows", "macos", "linux"],
            risk_level: RiskLevel::None,
            safety: ModuleSafety::core_read_only(),
        },
        ModuleManifest {
            manifest_version: MODULE_SCHEMA_VERSION,
            id: "core.policy",
            display_name: "Safety policy",
            version: env!("CARGO_PKG_VERSION"),
            publisher: "runtime.zero",
            kind: ModuleKind::CoreFoundation,
            status: ModuleStatus::Active,
            summary: "Shared safety metadata and future mutation gates.",
            capabilities: vec!["risk-metadata", "dry-run-contracts", "confirmation-gates"],
            supported_platforms: vec!["windows", "macos", "linux"],
            risk_level: RiskLevel::None,
            safety: ModuleSafety::core_read_only(),
        },
        ModuleManifest {
            manifest_version: MODULE_SCHEMA_VERSION,
            id: "core.registry",
            display_name: "Module registry",
            version: env!("CARGO_PKG_VERSION"),
            publisher: "runtime.zero",
            kind: ModuleKind::CoreFoundation,
            status: ModuleStatus::Active,
            summary: "Lists core primitives and explicitly installed modules.",
            capabilities: vec!["manifest-schema", "installed-module-listing"],
            supported_platforms: vec!["windows", "macos", "linux"],
            risk_level: RiskLevel::ReadOnly,
            safety: ModuleSafety::core_read_only(),
        },
    ]
}

pub fn planned_module_family_manifests() -> Vec<ModuleManifest> {
    vec![
        planned_family(
            "first-party.inventory",
            "Inventory modules",
            "Read-only environment, tool, app, and runtime discovery modules.",
            RiskLevel::ReadOnly,
        ),
        planned_family(
            "first-party.updater",
            "Updater modules",
            "Installed-only update planning modules with no surprise installs.",
            RiskLevel::DryRunOnly,
        ),
        planned_family(
            "first-party.uninstall",
            "Uninstall modules",
            "Manager-native uninstall planning modules behind explicit review.",
            RiskLevel::MutatingGated,
        ),
        planned_family(
            "first-party.leftovers",
            "Leftover scanner modules",
            "Report-first leftover classification before quarantine design.",
            RiskLevel::DryRunOnly,
        ),
    ]
}

fn planned_family(
    id: &'static str,
    display_name: &'static str,
    summary: &'static str,
    risk_level: RiskLevel,
) -> ModuleManifest {
    ModuleManifest {
        manifest_version: MODULE_SCHEMA_VERSION,
        id,
        display_name,
        version: "planned",
        publisher: "runtime.zero",
        kind: ModuleKind::FirstPartyModule,
        status: ModuleStatus::Planned,
        summary,
        capabilities: vec![],
        supported_platforms: vec!["windows", "macos", "linux"],
        risk_level,
        safety: ModuleSafety::module_contract_default(),
    }
}
