use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use crate::module_manifest::{
    MAX_MANIFEST_BYTES, MODULE_SCHEMA_VERSION, ModuleKind, ModuleManifest, ModuleStatus, RiskLevel,
};
use crate::package_integrity::{PackageIntegrityReport, verify_package_integrity};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManifestValidationReport {
    pub path: String,
    pub valid: bool,
    pub manifest: Option<ModuleManifest>,
    pub integrity: Option<PackageIntegrityReport>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ManifestValidationReport {
    fn invalid(path: &Path, error: String) -> Self {
        Self {
            path: path.display().to_string(),
            valid: false,
            manifest: None,
            integrity: None,
            errors: vec![error],
            warnings: Vec::new(),
        }
    }
}

pub fn load_manifest_file(path: &Path) -> ManifestValidationReport {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) => return ManifestValidationReport::invalid(path, err.to_string()),
    };
    if !metadata.is_file() {
        return ManifestValidationReport::invalid(path, "manifest path is not a file".to_string());
    }
    if metadata.len() > MAX_MANIFEST_BYTES {
        return ManifestValidationReport::invalid(path, "manifest file is too large".to_string());
    }
    let source = match fs::read_to_string(path) {
        Ok(source) => source,
        Err(err) => return ManifestValidationReport::invalid(path, err.to_string()),
    };
    match serde_json::from_str::<ModuleManifest>(&source) {
        Ok(manifest) => validate_manifest(path, manifest),
        Err(err) => ManifestValidationReport::invalid(path, format!("invalid JSON: {err}")),
    }
}

pub fn validate_manifest(path: &Path, manifest: ModuleManifest) -> ManifestValidationReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    validate_identity(&manifest, &mut errors);
    validate_text(&manifest, &mut errors);
    validate_lists(&manifest, &mut errors, &mut warnings);
    validate_trust(&manifest, &mut errors, &mut warnings);
    validate_safety(&manifest, &mut errors);
    let integrity = verify_package_integrity(path, &manifest);
    errors.extend(
        integrity
            .errors
            .iter()
            .map(|error| format!("package_integrity: {error}")),
    );
    warnings.extend(
        integrity
            .warnings
            .iter()
            .map(|warning| format!("package_integrity: {warning}")),
    );
    ManifestValidationReport {
        path: path.display().to_string(),
        valid: errors.is_empty(),
        manifest: Some(manifest),
        integrity: Some(integrity),
        errors,
        warnings,
    }
}

fn validate_identity(manifest: &ModuleManifest, errors: &mut Vec<String>) {
    if manifest.manifest_version != MODULE_SCHEMA_VERSION {
        errors.push(format!("manifest_version must be {MODULE_SCHEMA_VERSION}"));
    }
    if !is_valid_id(&manifest.id) {
        errors.push("id must use lowercase letters, digits, dots, or hyphens".to_string());
    }
    if manifest.id.starts_with("core.") && manifest.kind != ModuleKind::CoreFoundation {
        errors.push("non-core modules must not use the reserved core.* id prefix".to_string());
    }
}

fn validate_text(manifest: &ModuleManifest, errors: &mut Vec<String>) {
    validate_field(&manifest.display_name, "display_name", 80, errors);
    validate_field(&manifest.version, "version", 40, errors);
    validate_field(&manifest.publisher, "publisher", 80, errors);
    validate_field(&manifest.summary, "summary", 240, errors);
}

fn validate_lists(manifest: &ModuleManifest, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    check_unique(&manifest.capabilities, "capabilities", warnings);
    check_unique(
        &manifest.supported_platforms,
        "supported_platforms",
        warnings,
    );
    if manifest.supported_platforms.is_empty() {
        errors.push("supported_platforms must not be empty".to_string());
    }
    for platform in &manifest.supported_platforms {
        if !matches!(platform.as_str(), "windows" | "macos" | "linux") {
            errors.push(format!("unsupported platform '{platform}'"));
        }
    }
}

fn validate_trust(manifest: &ModuleManifest, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    if manifest.kind == ModuleKind::ThirdPartyModule {
        errors
            .push("third-party modules are not supported until trust hardening exists".to_string());
    }
    if manifest.kind == ModuleKind::CoreFoundation && manifest.publisher != "runtime.zero" {
        errors.push("core foundation manifests must be published by runtime.zero".to_string());
    }
    if manifest.kind == ModuleKind::FirstPartyModule && manifest.publisher != "runtime.zero" {
        errors.push("first-party modules must be published by runtime.zero".to_string());
    }
    if manifest.kind != ModuleKind::CoreFoundation && manifest.status != ModuleStatus::Installed {
        warnings.push("loaded optional module manifests should use installed status".to_string());
    }
}

fn validate_safety(manifest: &ModuleManifest, errors: &mut Vec<String>) {
    if manifest.safety.remote_execution_allowed {
        errors.push("remote_execution_allowed must be false in this foundation".to_string());
    }
    if manifest.risk_level == RiskLevel::None && manifest.safety.mutates_system {
        errors.push("risk_level none cannot mutate the system".to_string());
    }
    if manifest.safety.mutates_system && !manifest.safety.requires_confirmation {
        errors.push("mutating modules must require confirmation".to_string());
    }
    if manifest.safety.mutates_system && !manifest.safety.dry_run_required {
        errors.push("mutating modules must require dry-run support".to_string());
    }
    if manifest.risk_level == RiskLevel::DestructiveGated && !manifest.safety.quarantine_supported {
        errors.push("destructive-gated modules must support quarantine or rollback".to_string());
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

fn check_unique(values: &[String], name: &str, warnings: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            warnings.push(format!("{name} contains duplicate value '{value}'"));
        }
    }
}

fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && !id.starts_with(['.', '-'])
        && !id.ends_with(['.', '-'])
        && !id.contains("..")
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module_manifest::{ModuleSafety, RiskLevel};

    fn valid_manifest() -> ModuleManifest {
        ModuleManifest::new(
            "first-party.inventory",
            "Inventory",
            "0.1.0",
            "runtime.zero",
            ModuleKind::FirstPartyModule,
            ModuleStatus::Planned,
            "Read-only inventory.",
            &["inventory"],
            &["windows"],
            RiskLevel::ReadOnly,
            ModuleSafety::module_contract_default(),
        )
    }

    #[test]
    fn validates_safe_first_party_manifest() {
        let report = validate_manifest(Path::new("module.json"), valid_manifest());
        assert!(report.valid, "{:?}", report.errors);
    }

    #[test]
    fn rejects_remote_execution() {
        let mut manifest = valid_manifest();
        manifest.safety.remote_execution_allowed = true;
        let report = validate_manifest(Path::new("module.json"), manifest);
        assert!(!report.valid);
        assert!(
            report
                .errors
                .iter()
                .any(|error| error.contains("remote_execution"))
        );
    }

    #[test]
    fn rejects_third_party_until_trust_model_exists() {
        let mut manifest = valid_manifest();
        manifest.kind = ModuleKind::ThirdPartyModule;
        manifest.publisher = "someone.else".to_string();
        let report = validate_manifest(Path::new("module.json"), manifest);
        assert!(!report.valid);
        assert!(
            report
                .errors
                .iter()
                .any(|error| error.contains("third-party"))
        );
    }
}
