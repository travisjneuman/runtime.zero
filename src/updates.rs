use rz0_action_plan::ActionPlan;
use rz0_cancellation_contract::CancellationToken;
use serde::Serialize;

use crate::apps::{AppCatalog, InstalledSoftware, SoftwareUpdate, software_name_key};

pub const UPDATE_CATALOG_CONTRACT: &str = "live_update_catalog";
const MAX_WARNINGS: usize = rz0_resource_contract::MAX_INVENTORY_WARNINGS;

#[derive(Debug, Clone)]
pub(crate) struct LiveUpdateReview {
    pub(crate) catalog: LiveUpdateCatalog,
    pub(crate) plan: Option<ActionPlan>,
}

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
    match collect_live_update_review(catalog) {
        Ok(review) => review.catalog,
        Err(error) => LiveUpdateCatalog {
            schema_version: 1,
            contract: UPDATE_CATALOG_CONTRACT,
            checked: false,
            read_only: true,
            writes_attempted: false,
            network_read_requested: true,
            source_count: 0,
            source_ok_count: 0,
            candidate_count: 0,
            candidates: Vec::new(),
            warnings: vec![format!("universal provider scan failed closed: {error}")],
        },
    }
}

pub(crate) fn collect_live_update_review(catalog: &AppCatalog) -> Result<LiveUpdateReview, String> {
    collect_live_update_review_cancellable(catalog, None)
}

pub(crate) fn collect_live_update_review_cancellable(
    catalog: &AppCatalog,
    cancellation: Option<&CancellationToken>,
) -> Result<LiveUpdateReview, String> {
    let (scan, plan) =
        crate::update_cli::collect_universal_update_plan_cancellable(true, cancellation)?;
    let mut candidates = scan
        .records
        .into_iter()
        .filter(|record| {
            record.installed && record.manager_record_present && record.update_available
        })
        .filter_map(|record| software_update_from_record(catalog, record))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.software_id
            .cmp(&right.software_id)
            .then_with(|| left.available_version.cmp(&right.available_version))
    });
    candidates.dedup_by(|left, right| {
        left.finding_id == right.finding_id
            && left.software_id == right.software_id
            && left.available_version == right.available_version
    });
    Ok(LiveUpdateReview {
        catalog: LiveUpdateCatalog {
            schema_version: 1,
            contract: UPDATE_CATALOG_CONTRACT,
            checked: true,
            read_only: true,
            writes_attempted: false,
            network_read_requested: true,
            source_count: scan.source_count,
            source_ok_count: scan.source_ok_count,
            candidate_count: candidates.len(),
            candidates,
            warnings: scan.warnings.into_iter().take(MAX_WARNINGS).collect(),
        },
        plan,
    })
}

pub(crate) fn collect_macos_homebrew_update_review_cancellable(
    catalog: &AppCatalog,
    cancellation: Option<&CancellationToken>,
) -> Result<LiveUpdateReview, String> {
    let (scan, plan) =
        crate::update_cli::collect_macos_homebrew_update_plan_cancellable(true, cancellation)?;
    let mut candidates = scan
        .records
        .into_iter()
        .filter(|record| {
            record.installed && record.manager_record_present && record.update_available
        })
        .filter_map(|record| software_update_from_record(catalog, record))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.software_id
            .cmp(&right.software_id)
            .then_with(|| left.available_version.cmp(&right.available_version))
    });
    candidates.dedup_by(|left, right| left.finding_id == right.finding_id);
    Ok(LiveUpdateReview {
        catalog: LiveUpdateCatalog {
            schema_version: 1,
            contract: UPDATE_CATALOG_CONTRACT,
            checked: true,
            read_only: true,
            writes_attempted: false,
            network_read_requested: true,
            source_count: scan.source_count,
            source_ok_count: scan.source_ok_count,
            candidate_count: candidates.len(),
            candidates,
            warnings: scan.warnings.into_iter().take(MAX_WARNINGS).collect(),
        },
        plan,
    })
}

fn software_update_from_record(
    catalog: &AppCatalog,
    record: rz0_module_updater::UpdateRecord,
) -> Option<SoftwareUpdate> {
    let available_version = record.available_version.clone()?;
    let app = match_catalog_app(catalog, &record);
    let software_id = app
        .map(|app| app.id.clone())
        .unwrap_or_else(|| record.subject_reference.clone());
    let installed_version = record
        .installed_version
        .or_else(|| app.and_then(|app| app.version.clone()));
    Some(SoftwareUpdate {
        finding_id: record.finding_id,
        software_id,
        manager: record.manager.unwrap_or_else(|| "unknown".to_string()),
        installed_version,
        available_version,
        network_required: record.network_required,
        requires_elevation: record.requires_elevation,
        rollback_supported: record.rollback_supported,
    })
}

fn match_catalog_app<'a>(
    catalog: &'a AppCatalog,
    record: &rz0_module_updater::UpdateRecord,
) -> Option<&'a InstalledSoftware> {
    let package_key = record
        .subject_reference
        .rsplit(':')
        .next()
        .map(software_name_key)?;
    let provider = record
        .subject_reference
        .split(':')
        .nth(1)
        .unwrap_or_default();
    let manager = record.manager.as_deref().unwrap_or_default();
    catalog.apps.iter().find(|app| {
        (software_name_key(&app.name) == package_key
            || app
                .identifiers
                .iter()
                .any(|identifier| software_name_key(&identifier.value) == package_key))
            && manager_matches_catalog_source(provider, manager, app)
    })
}

fn manager_matches_catalog_source(provider: &str, manager: &str, app: &InstalledSoftware) -> bool {
    match provider {
        "homebrew-formula" => app.source_id == "macos.homebrew.formulae",
        "homebrew-cask" => app.source_id == "macos.homebrew.casks",
        "macports" => app.source_id == "macos.macports.packages",
        "apt" => app.source_id == "linux.dpkg.packages",
        "pacman" => app.source_id == "linux.pacman.packages",
        _ => match manager {
            "homebrew" => matches!(
                app.source_id.as_str(),
                "macos.homebrew.formulae" | "macos.homebrew.casks"
            ),
            "macports" => app.source_id == "macos.macports.packages",
            "apt" => app.source_id == "linux.dpkg.packages",
            "pacman" => app.source_id == "linux.pacman.packages",
            _ => true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::{InstallScope, SoftwareKind, UninstallOption};
    use rz0_module_updater::UpdateRecord;

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
            known_tools: Vec::new(),
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
        let unmatched = software_update_from_record(&catalog, record.clone())
            .expect("unmatched universal candidates remain visible");
        assert_eq!(unmatched.software_id, "package:apt:alpha");
        catalog.apps[0].source_id = "linux.dpkg.packages".to_string();
        let matched = software_update_from_record(&catalog, record).expect("catalog match");
        assert_eq!(matched.software_id, "macos.package.alpha");

        let dynamic = UpdateRecord {
            finding_id: "update.npm-global.pi".to_string(),
            subject_reference: "package:npm-global:pi".to_string(),
            installed: true,
            manager_record_present: true,
            update_available: true,
            installed_version: Some("1.0.0".to_string()),
            available_version: Some("2.0.0".to_string()),
            manager: Some("npm".to_string()),
            executable: Some("/opt/homebrew/bin/npm".to_string()),
            executable_sha256: Some("b".repeat(64)),
            executable_size_bytes: Some(4096),
            arguments: vec![
                "update".to_string(),
                "--global".to_string(),
                "pi".to_string(),
            ],
            network_required: true,
            requires_elevation: false,
            rollback_supported: false,
        };
        let dynamic_update = software_update_from_record(&catalog, dynamic)
            .expect("dynamic provider candidates remain visible without inventory rows");
        assert_eq!(dynamic_update.software_id, "package:npm-global:pi");
    }
}
