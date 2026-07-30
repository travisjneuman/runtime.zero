use std::fmt::Write as FmtWrite;

use rz0_inventory_contract::AppRecord;
use rz0_module_inventory::{InventoryOptions, collect_inventory};
use serde::Serialize;

use crate::ExitCode;

pub const APP_CATALOG_CONTRACT: &str = "installed_software_catalog";
pub const UNINSTALL_REVIEW_CONTRACT: &str = "uninstall_review";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppCatalog {
    pub schema_version: u16,
    pub contract: &'static str,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub platform: &'static str,
    pub source_count: usize,
    pub app_count: usize,
    pub apps: Vec<InstalledSoftware>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstalledSoftware {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub kind: SoftwareKind,
    pub scope: InstallScope,
    pub uninstall_option: UninstallOption,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SoftwareKind {
    ApplicationBundle,
    HomebrewFormula,
    HomebrewCask,
    PlatformPackage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstallScope {
    System,
    Local,
    User,
    Manager,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UninstallOption {
    Protected,
    ManagerReview,
    QuarantineReview,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UninstallReview {
    pub schema_version: u16,
    pub contract: &'static str,
    pub dry_run: bool,
    pub writes_attempted: bool,
    pub app_id: String,
    pub app_name: String,
    pub scope: InstallScope,
    pub option: UninstallOption,
    pub status: &'static str,
    pub confirmation_required: bool,
    pub rollback_required: bool,
    pub product_execution_authorized: bool,
    pub next_step: String,
}

pub fn collect_app_catalog() -> Result<AppCatalog, String> {
    let report = collect_inventory(&InventoryOptions {
        fixture: None,
        redact_paths: false,
        probe_versions: false,
        include_apps: true,
    })?;
    let mut apps = report.apps.iter().map(classify_app).collect::<Vec<_>>();
    apps.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    let warnings = report
        .sources
        .iter()
        .flat_map(|source| source.warnings.iter().cloned())
        .take(rz0_resource_contract::MAX_INVENTORY_WARNINGS)
        .collect::<Vec<_>>();
    Ok(AppCatalog {
        schema_version: 1,
        contract: APP_CATALOG_CONTRACT,
        read_only: true,
        writes_attempted: false,
        platform: std::env::consts::OS,
        source_count: report.summary.source_count,
        app_count: apps.len(),
        apps,
        warnings,
    })
}

fn classify_app(app: &AppRecord) -> InstalledSoftware {
    let (kind, scope, uninstall_option) = match app.source_id.as_str() {
        "macos.homebrew.formulae" => (
            SoftwareKind::HomebrewFormula,
            InstallScope::Manager,
            UninstallOption::ManagerReview,
        ),
        "macos.homebrew.casks" => (
            SoftwareKind::HomebrewCask,
            InstallScope::Manager,
            UninstallOption::ManagerReview,
        ),
        _ => classify_bundle(app.install_location.as_deref()),
    };
    InstalledSoftware {
        id: app.id.clone(),
        name: app.name.clone(),
        version: app.version.clone(),
        kind,
        scope,
        uninstall_option,
    }
}

fn classify_bundle(location: Option<&str>) -> (SoftwareKind, InstallScope, UninstallOption) {
    let Some(location) = location else {
        return (
            SoftwareKind::PlatformPackage,
            InstallScope::Unknown,
            UninstallOption::Unsupported,
        );
    };
    if location.starts_with("/System/Applications/") {
        return (
            SoftwareKind::ApplicationBundle,
            InstallScope::System,
            UninstallOption::Protected,
        );
    }
    if location.starts_with("/Applications/") {
        return (
            SoftwareKind::ApplicationBundle,
            InstallScope::Local,
            UninstallOption::QuarantineReview,
        );
    }
    if location.contains("/Applications/") {
        return (
            SoftwareKind::ApplicationBundle,
            InstallScope::User,
            UninstallOption::QuarantineReview,
        );
    }
    (
        SoftwareKind::ApplicationBundle,
        InstallScope::Unknown,
        UninstallOption::Unsupported,
    )
}

pub fn apps_command(args: &[String]) -> (ExitCode, String, String) {
    let format = match args {
        [] => AppOutputFormat::Text,
        [flag, value] if flag == "--format" && value == "json" => AppOutputFormat::Json,
        [help] if matches!(help.as_str(), "--help" | "-h" | "help") => {
            return (ExitCode::Ok, apps_usage(), String::new());
        }
        _ => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("unsupported apps option\n\n{}", apps_usage()),
            );
        }
    };
    let catalog = match collect_app_catalog() {
        Ok(catalog) => catalog,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("installed software inventory failed closed: {error}\n"),
            );
        }
    };
    match format {
        AppOutputFormat::Text => (ExitCode::Ok, render_catalog_text(&catalog), String::new()),
        AppOutputFormat::Json => match serde_json::to_string_pretty(&catalog) {
            Ok(json) => (ExitCode::Ok, format!("{json}\n"), String::new()),
            Err(error) => (ExitCode::Usage, String::new(), format!("{error}\n")),
        },
    }
}

pub fn uninstall_command(args: &[String]) -> (ExitCode, String, String) {
    let (app_id, format) = match args {
        [plan, app_id] if plan == "plan" => (app_id, AppOutputFormat::Text),
        [plan, app_id, flag, value] if plan == "plan" && flag == "--format" && value == "json" => {
            (app_id, AppOutputFormat::Json)
        }
        [help] if matches!(help.as_str(), "--help" | "-h" | "help") => {
            return (ExitCode::Ok, uninstall_usage(), String::new());
        }
        _ => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("uninstall requires a review plan\n\n{}", uninstall_usage()),
            );
        }
    };
    if !rz0_validation_contract::valid_dotted_id(app_id, 100) {
        return (
            ExitCode::Usage,
            String::new(),
            "installed software id is invalid\n".to_string(),
        );
    }
    let catalog = match collect_app_catalog() {
        Ok(catalog) => catalog,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("installed software inventory failed closed: {error}\n"),
            );
        }
    };
    let Some(app) = catalog.apps.iter().find(|app| app.id == *app_id) else {
        return (
            ExitCode::Usage,
            String::new(),
            format!("installed software id '{app_id}' was not found\n"),
        );
    };
    let review = build_uninstall_review(app);
    match format {
        AppOutputFormat::Text => (ExitCode::Ok, render_uninstall_text(&review), String::new()),
        AppOutputFormat::Json => match serde_json::to_string_pretty(&review) {
            Ok(json) => (ExitCode::Ok, format!("{json}\n"), String::new()),
            Err(error) => (ExitCode::Usage, String::new(), format!("{error}\n")),
        },
    }
}

fn build_uninstall_review(app: &InstalledSoftware) -> UninstallReview {
    let (status, confirmation_required, rollback_required, next_step) =
        match app.uninstall_option {
            UninstallOption::Protected => (
                "blocked",
                false,
                false,
                "Protected system software cannot be uninstalled by runtime.zero.".to_string(),
            ),
            UninstallOption::ManagerReview => (
                "review_available",
                true,
                true,
                "Review the manager-owned uninstall; execution remains disabled until the manager transaction path is authorized.".to_string(),
            ),
            UninstallOption::QuarantineReview => (
                "review_available",
                true,
                true,
                "Review quarantine-first removal; execution remains disabled until the exact bundle transaction path is authorized.".to_string(),
            ),
            UninstallOption::Unsupported => (
                "unsupported",
                false,
                false,
                "No safe ownership-specific uninstall method is available.".to_string(),
            ),
        };
    UninstallReview {
        schema_version: 1,
        contract: UNINSTALL_REVIEW_CONTRACT,
        dry_run: true,
        writes_attempted: false,
        app_id: app.id.clone(),
        app_name: app.name.clone(),
        scope: app.scope,
        option: app.uninstall_option,
        status,
        confirmation_required,
        rollback_required,
        product_execution_authorized: false,
        next_step,
    }
}

fn render_catalog_text(catalog: &AppCatalog) -> String {
    let mut output = format!(
        "runtime.zero installed software\n\nmode: read-only\nplatform: {}\nitems: {}\n\n",
        catalog.platform, catalog.app_count
    );
    for app in &catalog.apps {
        let _ = writeln!(
            output,
            "{}\t{}\tversion={}\tscope={:?}\tuninstall={:?}",
            app.id,
            app.name,
            app.version.as_deref().unwrap_or("unknown"),
            app.scope,
            app.uninstall_option
        );
    }
    output.push_str("\nUse `rz0 uninstall plan <id>` to review an available uninstall option.\n");
    output.push_str("No software was changed.\n");
    output
}

fn render_uninstall_text(review: &UninstallReview) -> String {
    format!(
        "runtime.zero uninstall review\n\napp: {}\nid: {}\nstatus: {}\noption: {:?}\nconfirmation_required: {}\nrollback_required: {}\nwrites_attempted: no\nexecution_authorized: no\n\n{}\n",
        review.app_name,
        review.app_id,
        review.status,
        review.option,
        review.confirmation_required,
        review.rollback_required,
        review.next_step
    )
}

fn apps_usage() -> String {
    "Usage: rz0 apps [--format json]\n\nLists bounded local application and package-manager evidence without paths or writes.\n".to_string()
}

fn uninstall_usage() -> String {
    "Usage: rz0 uninstall plan <installed-software-id> [--format json]\n\nBuilds a read-only uninstall review. It does not remove software.\n".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppOutputFormat {
    Text,
    Json,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_apps_are_protected_and_local_apps_offer_quarantine_review() {
        let system = AppRecord {
            id: "macos.app.system".to_string(),
            name: "System App".to_string(),
            source_id: "macos.application_bundles".to_string(),
            version: None,
            publisher: None,
            install_location: Some("/System/Applications/System App.app".to_string()),
            warnings: Vec::new(),
        };
        let local = AppRecord {
            install_location: Some("/Applications/Local App.app".to_string()),
            id: "macos.app.local".to_string(),
            name: "Local App".to_string(),
            ..system.clone()
        };
        assert_eq!(
            classify_app(&system).uninstall_option,
            UninstallOption::Protected
        );
        assert_eq!(
            classify_app(&local).uninstall_option,
            UninstallOption::QuarantineReview
        );
    }

    #[test]
    fn uninstall_ids_reject_terminal_control_input_before_inventory() {
        let (code, output, error) =
            uninstall_command(&["plan".to_string(), "bad\u{1b}[31m".to_string()]);
        assert_eq!(code, ExitCode::Usage);
        assert!(output.is_empty());
        assert_eq!(error, "installed software id is invalid\n");
        assert!(!error.contains('\u{1b}'));
    }

    #[test]
    fn reviews_never_authorize_execution() {
        let app = InstalledSoftware {
            id: "macos.app.local".to_string(),
            name: "Local App".to_string(),
            version: None,
            kind: SoftwareKind::ApplicationBundle,
            scope: InstallScope::Local,
            uninstall_option: UninstallOption::QuarantineReview,
        };
        let review = build_uninstall_review(&app);
        assert_eq!(review.status, "review_available");
        assert!(!review.product_execution_authorized);
        assert!(!review.writes_attempted);
    }
}
