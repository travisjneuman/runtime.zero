use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::launch_routing::{LaunchRoutingReport, cli_subcommand_report};
use crate::module_manifest::{ModuleKind, ModuleStatus};
use crate::module_store::{ModuleStorePlan, module_store_plan};
use crate::module_validation::{ManifestValidationReport, load_manifest_file};

const MODULE_MANIFEST_FILE: &str = "rz0-module.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModuleInstallPlanReport {
    pub input_path: String,
    pub valid: bool,
    pub dry_run: bool,
    pub manifest_path: String,
    pub package_root: String,
    pub proposed_install_root: String,
    pub proposed_module_dir: Option<String>,
    pub store: ModuleStorePlan,
    pub launch_context: LaunchRoutingReport,
    pub validation: ManifestValidationReport,
    pub planned_actions: Vec<PlannedInstallAction>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub safety_note: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PlannedInstallAction {
    pub action: PlannedInstallActionKind,
    pub target: String,
    pub description: String,
    pub would_write: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannedInstallActionKind {
    CreateModuleDirectory,
    CopyPackageFile,
    RecordInstalledManifest,
}

pub fn plan_module_install_dry_run(input_path: &Path) -> ModuleInstallPlanReport {
    let resolved = resolve_package_input(input_path);
    let manifest_path = resolved.manifest_path;
    let package_root = resolved.package_root;
    let validation = load_manifest_file(&manifest_path);
    build_report(
        input_path,
        manifest_path,
        package_root,
        validation,
        resolved.errors,
    )
}

fn build_report(
    input_path: &Path,
    manifest_path: PathBuf,
    package_root: PathBuf,
    validation: ManifestValidationReport,
    mut errors: Vec<String>,
) -> ModuleInstallPlanReport {
    errors.extend(validation.errors.iter().cloned());
    let store = store_plan(&manifest_path, &validation);
    let proposed_module_dir = store.module_dir.clone();
    validate_manifest_for_install_plan(&validation, &mut errors);
    let valid = errors.is_empty();
    let actions = if valid {
        planned_actions(&validation, proposed_module_dir.as_deref())
    } else {
        Vec::new()
    };
    let warnings = validation.warnings.clone();

    ModuleInstallPlanReport {
        input_path: input_path.display().to_string(),
        valid,
        dry_run: true,
        manifest_path: manifest_path.display().to_string(),
        package_root: package_root.display().to_string(),
        proposed_install_root: store.modules_root.clone(),
        proposed_module_dir,
        store,
        launch_context: cli_subcommand_report("modules install --dry-run"),
        validation,
        planned_actions: actions,
        errors,
        warnings,
        safety_note: "Dry-run planner only; no files, registry entries, PATH, services, tasks, or module code were changed.",
    }
}

fn store_plan(manifest_path: &Path, validation: &ManifestValidationReport) -> ModuleStorePlan {
    let seed = validation
        .manifest
        .as_ref()
        .map(|manifest| {
            format!(
                "{}|{}|{}",
                manifest.id,
                manifest.version,
                manifest_path.display()
            )
        })
        .unwrap_or_else(|| manifest_path.display().to_string());
    let module_id = validation
        .manifest
        .as_ref()
        .map(|manifest| manifest.id.as_str());
    let version = validation
        .manifest
        .as_ref()
        .map(|manifest| manifest.version.as_str());
    module_store_plan(module_id, version, &seed)
}

fn resolve_package_input(input_path: &Path) -> ResolvedPackageInput {
    let mut errors = Vec::new();
    if looks_url_like(&input_path.display().to_string()) {
        errors.push("remote URLs are not supported for module install planning".to_string());
    }
    if input_path.is_dir() {
        return ResolvedPackageInput {
            manifest_path: input_path.join(MODULE_MANIFEST_FILE),
            package_root: input_path.to_path_buf(),
            errors,
        };
    }
    let package_root = input_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    ResolvedPackageInput {
        manifest_path: input_path.to_path_buf(),
        package_root,
        errors,
    }
}

fn validate_manifest_for_install_plan(
    validation: &ManifestValidationReport,
    errors: &mut Vec<String>,
) {
    let Some(manifest) = &validation.manifest else {
        return;
    };
    if manifest.kind != ModuleKind::FirstPartyModule {
        errors.push("only first-party module packages can be planned for install".to_string());
    }
    if manifest.status != ModuleStatus::Installed {
        errors.push("install plans require a manifest with status installed".to_string());
    }
}

fn planned_actions(
    validation: &ManifestValidationReport,
    proposed_module_dir: Option<&str>,
) -> Vec<PlannedInstallAction> {
    let Some(manifest) = &validation.manifest else {
        return Vec::new();
    };
    let Some(target_dir) = proposed_module_dir else {
        return Vec::new();
    };
    let mut actions = vec![PlannedInstallAction {
        action: PlannedInstallActionKind::CreateModuleDirectory,
        target: target_dir.to_string(),
        description: format!("would create module directory for {}", manifest.id),
        would_write: false,
    }];
    if let Some(integrity) = &manifest.integrity {
        actions.extend(integrity.files.iter().map(|file| PlannedInstallAction {
            action: PlannedInstallActionKind::CopyPackageFile,
            target: Path::new(target_dir).join(&file.path).display().to_string(),
            description: format!("would copy verified package file {}", file.path),
            would_write: false,
        }));
    }
    actions.push(PlannedInstallAction {
        action: PlannedInstallActionKind::RecordInstalledManifest,
        target: Path::new(target_dir)
            .join(MODULE_MANIFEST_FILE)
            .display()
            .to_string(),
        description: "would record validated module manifest metadata".to_string(),
        would_write: false,
    });
    actions
}

fn looks_url_like(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("://")
        || lower.starts_with("file:")
        || lower.starts_with("http:")
        || lower.starts_with("https:")
}

struct ResolvedPackageInput {
    manifest_path: PathBuf,
    package_root: PathBuf,
    errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_valid_fixture_without_write_actions() {
        let report = plan_module_install_dry_run(Path::new(
            "tests/fixtures/module-packages/valid-inventory",
        ));
        assert!(report.valid, "{:?}", report.errors);
        assert!(report.dry_run);
        assert_eq!(report.store.store_schema_version, 1);
        assert!(
            report
                .store
                .registry_path
                .contains("installed-modules.json")
        );
        assert!(report.store.receipt_path.is_some());
        assert_eq!(
            report.launch_context.launch_mode,
            crate::launch_routing::LaunchMode::CliSubcommand
        );
        assert!(
            report
                .planned_actions
                .iter()
                .all(|action| !action.would_write)
        );
        assert!(
            report
                .planned_actions
                .iter()
                .any(|action| action.action == PlannedInstallActionKind::CopyPackageFile)
        );
    }

    #[test]
    fn rejects_hash_mismatch_fixture() {
        let report =
            plan_module_install_dry_run(Path::new("tests/fixtures/module-packages/hash-mismatch"));
        assert!(!report.valid);
        assert!(report.planned_actions.is_empty());
        assert!(
            report
                .errors
                .iter()
                .any(|error| error.contains("hash mismatch"))
        );
    }
}
