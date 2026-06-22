use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::brand;
use crate::module_manifest::{
    MODULE_SCHEMA_VERSION, ModuleKind, ModuleManifest, ModuleSafety, ModuleStatus, RiskLevel,
};
use crate::module_validation::{ManifestValidationReport, load_manifest_file};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModuleRegistryReport {
    pub schema_version: u16,
    pub runtime: RuntimeSummary,
    pub core: Vec<ModuleManifest>,
    pub installed_modules: Vec<ModuleManifest>,
    pub planned_module_families: Vec<ModuleManifest>,
    pub validation_reports: Vec<ManifestValidationReport>,
    pub summary: RegistrySummary,
    pub safety_note: &'static str,
}

impl ModuleRegistryReport {
    pub fn empty_installed() -> Self {
        Self::from_reports(Vec::new())
    }

    pub fn from_directory(path: &Path) -> Self {
        Self::from_reports(load_manifest_directory(path))
    }

    fn from_reports(mut validation_reports: Vec<ManifestValidationReport>) -> Self {
        flag_duplicate_installed_ids(&mut validation_reports);
        let core = core_foundation_manifests();
        let installed_modules = valid_installed_modules(&validation_reports);
        let planned_module_families = planned_module_family_manifests();

        Self {
            schema_version: MODULE_SCHEMA_VERSION,
            runtime: RuntimeSummary::current(),
            summary: RegistrySummary {
                core_count: core.len(),
                installed_module_count: installed_modules.len(),
                planned_family_count: planned_module_families.len(),
                validation_error_count: count_validation_errors(&validation_reports),
            },
            core,
            installed_modules,
            planned_module_families,
            validation_reports,
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
    pub validation_error_count: usize,
}

pub fn core_foundation_manifests() -> Vec<ModuleManifest> {
    vec![
        core_manifest(
            "core.cli",
            "CLI core",
            "Command parsing, output routing, and safe bootstrap commands.",
            &["command-routing", "text-output", "json-output"],
            RiskLevel::None,
        ),
        core_manifest(
            "core.policy",
            "Safety policy",
            "Shared safety metadata and future mutation gates.",
            &["risk-metadata", "dry-run-contracts", "confirmation-gates"],
            RiskLevel::None,
        ),
        core_manifest(
            "core.registry",
            "Module registry",
            "Lists core primitives and explicitly installed modules.",
            &["manifest-schema", "installed-module-listing"],
            RiskLevel::ReadOnly,
        ),
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
    ModuleManifest::new(
        id,
        display_name,
        "planned",
        "runtime.zero",
        ModuleKind::FirstPartyModule,
        ModuleStatus::Planned,
        summary,
        &[],
        &["windows", "macos", "linux"],
        risk_level,
        ModuleSafety::module_contract_default(),
    )
}

fn core_manifest(
    id: &str,
    display_name: &str,
    summary: &str,
    capabilities: &[&str],
    risk_level: RiskLevel,
) -> ModuleManifest {
    ModuleManifest::new(
        id,
        display_name,
        env!("CARGO_PKG_VERSION"),
        "runtime.zero",
        ModuleKind::CoreFoundation,
        ModuleStatus::Active,
        summary,
        capabilities,
        &["windows", "macos", "linux"],
        risk_level,
        ModuleSafety::core_read_only(),
    )
}

fn valid_installed_modules(reports: &[ManifestValidationReport]) -> Vec<ModuleManifest> {
    reports
        .iter()
        .filter(|report| report.valid)
        .filter_map(|report| report.manifest.clone())
        .filter(|manifest| manifest.kind == ModuleKind::FirstPartyModule)
        .filter(|manifest| manifest.status == ModuleStatus::Installed)
        .collect()
}

fn flag_duplicate_installed_ids(reports: &mut [ManifestValidationReport]) {
    let mut counts = BTreeMap::new();
    for id in reports.iter().filter_map(installed_id) {
        *counts.entry(id).or_insert(0) += 1;
    }
    for report in reports {
        let Some(id) = installed_id(report) else {
            continue;
        };
        if counts.get(&id).copied().unwrap_or_default() > 1 {
            report.valid = false;
            report
                .errors
                .push(format!("duplicate installed module id '{id}'"));
        }
    }
}

fn installed_id(report: &ManifestValidationReport) -> Option<String> {
    let manifest = report.manifest.as_ref()?;
    if !report.valid || manifest.status != ModuleStatus::Installed {
        return None;
    }
    Some(manifest.id.clone())
}

fn count_validation_errors(reports: &[ManifestValidationReport]) -> usize {
    reports.iter().map(|report| report.errors.len()).sum()
}

fn load_manifest_directory(path: &Path) -> Vec<ManifestValidationReport> {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return vec![load_manifest_file(path)],
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths.iter().map(|path| load_manifest_file(path)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_has_no_installed_modules() {
        let report = ModuleRegistryReport::empty_installed();
        assert_eq!(report.installed_modules.len(), 0);
        assert_eq!(report.summary.validation_error_count, 0);
    }
}
