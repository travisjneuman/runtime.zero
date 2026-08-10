use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rz0_module_updater::{
    ManagerKind, ManagerParseContext, UpdateRecord, manager_probe_specs_for_platform,
    parse_manager_output,
};
use serde::Serialize;

use crate::apps::{AppCatalog, SoftwareUpdate, software_name_key};

pub const UPDATE_CATALOG_CONTRACT: &str = "live_update_catalog";
const MAX_WARNINGS: usize = rz0_resource_contract::MAX_INVENTORY_WARNINGS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LiveUpdateCatalog {
    pub schema_version: u16,
    pub contract: &'static str,
    pub checked: bool,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub network_read_requested: bool,
    pub source_count: usize,
    pub source_ok_count: usize,
    pub candidate_count: usize,
    pub candidates: Vec<SoftwareUpdate>,
    pub warnings: Vec<String>,
}

pub fn collect_live_update_catalog(catalog: &AppCatalog) -> LiveUpdateCatalog {
    let specs = manager_probe_specs_for_platform(std::env::consts::OS);
    let mut warnings = Vec::new();
    let mut source_ok_count = 0usize;
    let mut candidates = Vec::new();
    for spec in &specs {
        let Some(executable) = resolve_executable(spec.executable_candidates) else {
            warnings.push(format!(
                "{} availability source executable was not found",
                spec.manager.id()
            ));
            continue;
        };
        let executable_identity =
            match crate::update_execution::observe_manager_executable(&executable) {
                Ok(identity) => identity,
                Err(error) => {
                    warnings.push(format!(
                        "{} executable identity unavailable before probe: {error}",
                        spec.manager.id()
                    ));
                    continue;
                }
            };
        let output = match rz0_process_host::run_read_only_process(
            &rz0_process_host::ReadOnlyProcessRequest {
                executable: executable.clone(),
                arguments: spec
                    .query_arguments
                    .iter()
                    .map(|argument| (*argument).to_string())
                    .collect(),
                working_directory: PathBuf::from("/"),
                environment: probe_environment(),
                timeout: Duration::from_secs(10),
                output_limit: rz0_resource_contract::MAX_FINDING_REPORT_BYTES,
            },
        ) {
            Ok(output) => output,
            Err(error) => {
                warnings.push(format!(
                    "{} availability probe failed closed: {error}",
                    spec.manager.id()
                ));
                continue;
            }
        };
        let accepted_nonzero =
            spec.manager == ManagerKind::Dnf && output.status.code() == Some(100);
        if !output.status.success() && !accepted_nonzero {
            warnings.push(format!(
                "{} availability probe returned an unaccepted status: {}",
                spec.manager.id(),
                output.status
            ));
            continue;
        }
        let bytes = if output.stdout.bytes.is_empty() {
            &output.stderr.bytes
        } else {
            &output.stdout.bytes
        };
        if bytes.is_empty() {
            warnings.push(format!(
                "{} availability probe returned no parseable output",
                spec.manager.id()
            ));
            continue;
        }
        let identity_after = crate::update_execution::observe_manager_executable(&executable);
        if identity_after.as_ref() != Ok(&executable_identity) {
            warnings.push(format!(
                "{} executable identity changed during availability probe",
                spec.manager.id()
            ));
            continue;
        }
        let context = ManagerParseContext {
            manager: spec.manager,
            executable: Some(executable.display().to_string()),
            executable_sha256: Some(executable_identity.sha256.clone()),
            executable_size_bytes: Some(executable_identity.size_bytes),
            network_required: spec.network_required,
            requires_elevation: spec.requires_elevation,
            rollback_supported: false,
        };
        let records = match parse_manager_output(&context, bytes) {
            Ok(records) => records,
            Err(error) => {
                warnings.push(format!(
                    "{} availability output unavailable: {error}",
                    spec.manager.id()
                ));
                continue;
            }
        };
        source_ok_count = source_ok_count.saturating_add(1);
        for record in records {
            if let Some(update) = match_record(catalog, spec.manager, &record) {
                candidates.push(update);
            } else {
                warnings.push(format!(
                    "{} update candidate '{}' did not match an installed catalog record",
                    spec.manager.id(),
                    record.subject_reference
                ));
            }
        }
    }
    warnings.truncate(MAX_WARNINGS);
    candidates.sort_by(|left, right| {
        left.software_id
            .cmp(&right.software_id)
            .then_with(|| left.available_version.cmp(&right.available_version))
    });
    candidates.dedup_by(|left, right| {
        left.software_id == right.software_id && left.available_version == right.available_version
    });
    LiveUpdateCatalog {
        schema_version: 1,
        contract: UPDATE_CATALOG_CONTRACT,
        checked: true,
        read_only: true,
        writes_attempted: false,
        network_read_requested: true,
        source_count: specs.len(),
        source_ok_count,
        candidate_count: candidates.len(),
        candidates,
        warnings,
    }
}

fn resolve_executable(candidates: &[&str]) -> Option<PathBuf> {
    candidates.iter().map(Path::new).find_map(|path| {
        let metadata = fs::symlink_metadata(path).ok()?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return None;
        }
        Some(path.to_path_buf())
    })
}

fn match_record(
    catalog: &AppCatalog,
    manager: ManagerKind,
    record: &UpdateRecord,
) -> Option<SoftwareUpdate> {
    let package_key = record
        .subject_reference
        .rsplit(':')
        .next()
        .map(software_name_key)?;
    let app = catalog.apps.iter().find(|app| {
        if software_name_key(&app.name) != package_key {
            return false;
        }
        match manager {
            ManagerKind::HomebrewFormula => app.source_id == "macos.homebrew.formulae",
            ManagerKind::HomebrewCask => app.source_id == "macos.homebrew.casks",
            ManagerKind::MacPorts => app.source_id == "macos.macports.packages",
            ManagerKind::Apt => app.source_id == "linux.dpkg.packages",
            ManagerKind::Pacman => app.source_id == "linux.pacman.packages",
            ManagerKind::Winget
            | ManagerKind::Dnf
            | ManagerKind::Zypper
            | ManagerKind::Snap
            | ManagerKind::Flatpak => false,
        }
    })?;
    Some(SoftwareUpdate {
        software_id: app.id.clone(),
        manager: record
            .manager
            .clone()
            .unwrap_or_else(|| manager.manager_name().to_string()),
        installed_version: record
            .installed_version
            .clone()
            .or_else(|| app.version.clone()),
        available_version: record.available_version.clone()?,
        network_required: record.network_required,
        requires_elevation: record.requires_elevation,
        rollback_supported: record.rollback_supported,
    })
}

fn probe_environment() -> Vec<(String, String)> {
    let mut environment = Vec::new();
    if std::env::consts::OS == "macos" {
        if let Some(home) = std::env::var_os("HOME").and_then(|value| value.into_string().ok()) {
            environment.push(("HOME".to_string(), home));
        }
        environment.push(("HOMEBREW_NO_AUTO_UPDATE".to_string(), "1".to_string()));
        environment.push(("HOMEBREW_NO_ENV_HINTS".to_string(), "1".to_string()));
        environment.push((
            "PATH".to_string(),
            "/usr/bin:/bin:/opt/homebrew/bin:/usr/local/bin".to_string(),
        ));
    }
    environment
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::{InstallScope, SoftwareKind, UninstallOption};

    #[test]
    fn unavailable_manager_sources_remain_read_only_and_bounded() {
        let mut catalog = AppCatalog {
            schema_version: 1,
            contract: crate::apps::APP_CATALOG_CONTRACT,
            read_only: true,
            writes_attempted: false,
            platform: "test",
            source_count: 0,
            app_count: 1,
            service_count: 0,
            identity_group_count: 1,
            identity_groups: Vec::new(),
            apps: vec![crate::apps::InstalledSoftware {
                id: "macos.package.alpha".to_string(),
                name: "alpha".to_string(),
                version: Some("1.0".to_string()),
                source_id: "macos.homebrew.formulae".to_string(),
                identifiers: Vec::new(),
                identity_group_id: "software.alpha".to_string(),
                identity_confidence: crate::apps::IdentityConfidence::ExactEvidence,
                kind: SoftwareKind::HomebrewFormula,
                scope: InstallScope::Manager,
                uninstall_option: UninstallOption::ManagerReview,
            }],
            warnings: Vec::new(),
        };
        let result = LiveUpdateCatalog {
            schema_version: 1,
            contract: UPDATE_CATALOG_CONTRACT,
            checked: true,
            read_only: true,
            writes_attempted: false,
            network_read_requested: true,
            source_count: 0,
            source_ok_count: 0,
            candidate_count: 0,
            candidates: Vec::new(),
            warnings: vec!["fixture source unavailable".to_string()],
        };
        assert!(result.read_only);
        assert!(!result.writes_attempted);
        assert_eq!(catalog.apps[0].scope, InstallScope::Manager);

        let record = UpdateRecord {
            finding_id: "update.apt.alpha".to_string(),
            subject_reference: "package:apt:alpha".to_string(),
            installed: true,
            manager_record_present: true,
            update_available: true,
            installed_version: Some("1.0".to_string()),
            available_version: Some("2.0".to_string()),
            manager: Some("apt".to_string()),
            executable: Some("/usr/bin/apt".to_string()),
            executable_sha256: Some("a".repeat(64)),
            executable_size_bytes: Some(4096),
            arguments: vec![
                "install".to_string(),
                "--only-upgrade".to_string(),
                "alpha".to_string(),
            ],
            network_required: true,
            requires_elevation: true,
            rollback_supported: false,
        };
        assert!(match_record(&catalog, ManagerKind::Apt, &record).is_none());
        catalog.apps[0].source_id = "linux.dpkg.packages".to_string();
        assert!(match_record(&catalog, ManagerKind::Apt, &record).is_some());
    }
}
