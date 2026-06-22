use serde::Serialize;
use std::env;
use std::path::{Path, PathBuf};

use crate::module_manifest::{ModuleKind, ModuleStatus};
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
    let install_root = default_module_install_root();
    let proposed_module_dir = proposed_module_dir(&install_root, &validation);
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
        proposed_install_root: install_root.display().to_string(),
        proposed_module_dir,
        validation,
        planned_actions: actions,
        errors,
        warnings,
        safety_note: "Dry-run planner only; no files, registry entries, PATH, services, tasks, or module code were changed.",
    }
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

fn proposed_module_dir(
    install_root: &Path,
    validation: &ManifestValidationReport,
) -> Option<String> {
    let manifest = validation.manifest.as_ref()?;
    Some(
        install_root
            .join(&manifest.id)
            .join(&manifest.version)
            .display()
            .to_string(),
    )
}

fn default_module_install_root() -> PathBuf {
    match env::consts::OS {
        "windows" => env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("%LOCALAPPDATA%"))
            .join("runtime.zero")
            .join("modules"),
        "macos" => home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join("Library")
            .join("Application Support")
            .join("runtime.zero")
            .join("modules"),
        _ => env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(home_data_dir)
            .unwrap_or_else(|| PathBuf::from("~/.local/share"))
            .join("runtime.zero")
            .join("modules"),
    }
}

fn home_data_dir() -> Option<PathBuf> {
    Some(home_dir()?.join(".local").join("share"))
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
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
