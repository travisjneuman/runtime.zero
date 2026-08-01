use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as FmtWrite;

use rz0_inventory_contract::AppRecord;
use rz0_module_inventory::{InventoryOptions, collect_inventory};
use serde::Serialize;
use sha2::{Digest, Sha256};

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
    pub identity_group_count: usize,
    pub identity_groups: Vec<SoftwareIdentityGroup>,
    pub apps: Vec<InstalledSoftware>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstalledSoftware {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub source_id: String,
    pub identity_group_id: String,
    pub identity_confidence: IdentityConfidence,
    pub kind: SoftwareKind,
    pub scope: InstallScope,
    pub uninstall_option: UninstallOption,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SoftwareUpdate {
    pub software_id: String,
    pub manager: String,
    pub installed_version: Option<String>,
    pub available_version: String,
    pub network_required: bool,
    pub requires_elevation: bool,
    pub rollback_supported: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityConfidence {
    ExactEvidence,
    Corroborated,
    Heuristic,
    Disputed,
}

impl IdentityConfidence {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ExactEvidence => "exact evidence",
            Self::Corroborated => "corroborated",
            Self::Heuristic => "heuristic",
            Self::Disputed => "disputed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SoftwareIdentityGroup {
    pub group_id: String,
    pub normalized_name: String,
    pub evidence_ids: Vec<String>,
    pub source_ids: Vec<String>,
    pub versions: Vec<String>,
    pub confidence: IdentityConfidence,
    pub version_disagreement: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoftwareFilter {
    All,
    Applications,
    PackageManagers,
    Reviewable,
}

impl SoftwareFilter {
    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Applications => "applications",
            Self::PackageManagers => "package managers",
            Self::Reviewable => "reviewable",
        }
    }

    fn matches(self, app: &InstalledSoftware) -> bool {
        match self {
            Self::All => true,
            Self::Applications => matches!(app.kind, SoftwareKind::ApplicationBundle),
            Self::PackageManagers => matches!(
                app.kind,
                SoftwareKind::HomebrewFormula
                    | SoftwareKind::HomebrewCask
                    | SoftwareKind::PlatformPackage
            ),
            Self::Reviewable => matches!(
                app.uninstall_option,
                UninstallOption::ManagerReview | UninstallOption::QuarantineReview
            ),
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::All => Self::Applications,
            Self::Applications => Self::PackageManagers,
            Self::PackageManagers => Self::Reviewable,
            Self::Reviewable => Self::All,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoftwareSort {
    Name,
    Version,
    Kind,
}

impl SoftwareSort {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Version => "version",
            Self::Kind => "kind",
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Name => Self::Version,
            Self::Version => Self::Kind,
            Self::Kind => Self::Name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SoftwareView {
    query: String,
    pub filter: SoftwareFilter,
    pub sort: SoftwareSort,
}

impl Default for SoftwareView {
    fn default() -> Self {
        Self {
            query: String::new(),
            filter: SoftwareFilter::All,
            sort: SoftwareSort::Name,
        }
    }
}

impl SoftwareView {
    pub const MAX_QUERY_BYTES: usize = 80;

    pub fn query(&self) -> &str {
        &self.query
    }

    pub fn push_query(&mut self, value: char) {
        if value.is_control() || self.query.len() >= Self::MAX_QUERY_BYTES {
            return;
        }
        let mut encoded = [0u8; 4];
        let width = value.encode_utf8(&mut encoded).len();
        if self.query.len().saturating_add(width) <= Self::MAX_QUERY_BYTES {
            self.query.push(value);
        }
    }

    pub fn pop_query(&mut self) {
        self.query.pop();
    }

    pub fn clear_query(&mut self) {
        self.query.clear();
    }

    pub fn matches(&self, app: &InstalledSoftware) -> bool {
        self.filter.matches(app)
            && (self.query.is_empty()
                || app
                    .name
                    .to_ascii_lowercase()
                    .contains(&self.query.to_ascii_lowercase())
                || app
                    .id
                    .to_ascii_lowercase()
                    .contains(&self.query.to_ascii_lowercase())
                || app
                    .source_id
                    .to_ascii_lowercase()
                    .contains(&self.query.to_ascii_lowercase()))
    }

    pub fn compare(
        &self,
        left: &InstalledSoftware,
        right: &InstalledSoftware,
    ) -> std::cmp::Ordering {
        let ordering = match self.sort {
            SoftwareSort::Name => left
                .name
                .to_ascii_lowercase()
                .cmp(&right.name.to_ascii_lowercase()),
            SoftwareSort::Version => left
                .version
                .as_deref()
                .unwrap_or("")
                .cmp(right.version.as_deref().unwrap_or("")),
            SoftwareSort::Kind => {
                software_kind_label(left.kind).cmp(software_kind_label(right.kind))
            }
        };
        ordering
            .then_with(|| {
                left.name
                    .to_ascii_lowercase()
                    .cmp(&right.name.to_ascii_lowercase())
            })
            .then_with(|| left.id.cmp(&right.id))
    }
}

fn software_kind_label(kind: SoftwareKind) -> &'static str {
    match kind {
        SoftwareKind::ApplicationBundle => "application_bundle",
        SoftwareKind::HomebrewFormula => "homebrew_formula",
        SoftwareKind::HomebrewCask => "homebrew_cask",
        SoftwareKind::PlatformPackage => "platform_package",
    }
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
    let identity_groups = assign_identity_groups(&mut apps);
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
        identity_group_count: identity_groups.len(),
        identity_groups,
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
        source_id: app.source_id.clone(),
        identity_group_id: String::new(),
        identity_confidence: IdentityConfidence::ExactEvidence,
        kind,
        scope,
        uninstall_option,
    }
}

fn assign_identity_groups(apps: &mut [InstalledSoftware]) -> Vec<SoftwareIdentityGroup> {
    let mut by_key = BTreeMap::<String, Vec<usize>>::new();
    for (index, app) in apps.iter().enumerate() {
        by_key.entry(identity_key(app)).or_default().push(index);
    }
    let mut groups = Vec::with_capacity(by_key.len());
    for (key, indexes) in by_key {
        let group_id = identity_group_id(&key);
        let evidence_ids = indexes
            .iter()
            .map(|index| apps[*index].id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let source_ids = indexes
            .iter()
            .map(|index| apps[*index].source_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let versions = indexes
            .iter()
            .filter_map(|index| apps[*index].version.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let version_disagreement = versions.len() > 1;
        let confidence = if version_disagreement {
            IdentityConfidence::Disputed
        } else if source_ids.len() > 1 {
            IdentityConfidence::Heuristic
        } else if evidence_ids.len() > 1 {
            IdentityConfidence::Corroborated
        } else {
            IdentityConfidence::ExactEvidence
        };
        for index in indexes {
            apps[index].identity_group_id = group_id.clone();
            apps[index].identity_confidence = confidence;
        }
        groups.push(SoftwareIdentityGroup {
            group_id,
            normalized_name: key,
            evidence_ids,
            source_ids,
            versions,
            confidence,
            version_disagreement,
        });
    }
    groups.sort_by(|left, right| left.group_id.cmp(&right.group_id));
    groups
}

pub fn software_name_key(value: &str) -> String {
    value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect::<String>()
}

fn identity_key(app: &InstalledSoftware) -> String {
    let mut normalized = software_name_key(&app.name);
    if normalized.len() < 3 {
        normalized = app.id.clone();
    }
    normalized
}

fn identity_group_id(key: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(b"runtime.zero.software-identity.v1\0");
    digest.update(key.as_bytes());
    let digest = format!("{:x}", digest.finalize());
    format!("software.{}", &digest[..24])
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
    if matches!(args, [help] if matches!(help.as_str(), "--help" | "-h" | "help")) {
        return (ExitCode::Ok, apps_usage(), String::new());
    }
    let format = match parse_output_format(args) {
        Ok(format) => format,
        Err(_) => {
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
    if matches!(args, [help] if matches!(help.as_str(), "--help" | "-h" | "help")) {
        return (ExitCode::Ok, uninstall_usage(), String::new());
    }
    if args.first().map(String::as_str) != Some("plan") {
        return (
            ExitCode::Usage,
            String::new(),
            format!("uninstall requires a review plan\n\n{}", uninstall_usage()),
        );
    }
    let mut app_id = None;
    let mut format = AppOutputFormat::Text;
    let mut index = 1usize;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => format = AppOutputFormat::Json,
            "--format" => {
                let Some(value) = args.get(index + 1).map(String::as_str) else {
                    return (
                        ExitCode::Usage,
                        String::new(),
                        format!("uninstall requires a review plan\n\n{}", uninstall_usage()),
                    );
                };
                format = match value {
                    "text" => AppOutputFormat::Text,
                    "json" => AppOutputFormat::Json,
                    _ => {
                        return (
                            ExitCode::Usage,
                            String::new(),
                            format!("uninstall requires a review plan\n\n{}", uninstall_usage()),
                        );
                    }
                };
                index += 1;
            }
            value if app_id.is_none() => app_id = Some(value),
            _ => {
                return (
                    ExitCode::Usage,
                    String::new(),
                    format!("uninstall requires a review plan\n\n{}", uninstall_usage()),
                );
            }
        }
        index += 1;
    }
    let Some(app_id) = app_id else {
        return (
            ExitCode::Usage,
            String::new(),
            format!("uninstall requires a review plan\n\n{}", uninstall_usage()),
        );
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
    let Some(app) = catalog.apps.iter().find(|app| app.id == app_id) else {
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

fn parse_output_format(args: &[String]) -> Result<AppOutputFormat, ()> {
    let mut format = AppOutputFormat::Text;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => format = AppOutputFormat::Json,
            "--format" => {
                let Some(value) = args.get(index + 1).map(String::as_str) else {
                    return Err(());
                };
                format = match value {
                    "text" => AppOutputFormat::Text,
                    "json" => AppOutputFormat::Json,
                    _ => return Err(()),
                };
                index += 1;
            }
            _ => return Err(()),
        }
        index += 1;
    }
    Ok(format)
}

fn apps_usage() -> String {
    "Usage: rz0 apps [--format text|json]\n\nLists bounded local application and package-manager evidence without paths or writes.\n"
        .to_string()
}

fn uninstall_usage() -> String {
    "Usage: rz0 uninstall plan <installed-software-id> [--format text|json]\n\nBuilds a read-only uninstall review. It does not remove software.\n".to_string()
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
            source_id: "macos.application_bundles".to_string(),
            identity_group_id: "software.fixture".to_string(),
            identity_confidence: IdentityConfidence::ExactEvidence,
            kind: SoftwareKind::ApplicationBundle,
            scope: InstallScope::Local,
            uninstall_option: UninstallOption::QuarantineReview,
        };
        let review = build_uninstall_review(&app);
        assert_eq!(review.status, "review_available");
        assert!(!review.product_execution_authorized);
        assert!(!review.writes_attempted);
    }

    #[test]
    fn identity_groups_preserve_disagreement_without_merging_evidence() {
        let mut apps = vec![
            classify_app(&AppRecord {
                id: "macos.app.alpha".to_string(),
                name: "Alpha Tool".to_string(),
                source_id: "macos.application_bundles".to_string(),
                version: Some("1.0".to_string()),
                publisher: None,
                install_location: Some("/Applications/Alpha Tool.app".to_string()),
                warnings: Vec::new(),
            }),
            classify_app(&AppRecord {
                id: "macos.package.alpha".to_string(),
                name: "Alpha Tool".to_string(),
                source_id: "macos.homebrew.casks".to_string(),
                version: Some("2.0".to_string()),
                publisher: Some("Homebrew".to_string()),
                install_location: Some("/opt/homebrew/Caskroom/alpha".to_string()),
                warnings: Vec::new(),
            }),
        ];
        let groups = assign_identity_groups(&mut apps);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].confidence, IdentityConfidence::Disputed);
        assert!(groups[0].version_disagreement);
        assert_eq!(groups[0].evidence_ids.len(), 2);
        assert_ne!(apps[0].id, apps[1].id);
        assert_eq!(apps[0].identity_group_id, apps[1].identity_group_id);
    }

    #[test]
    fn software_view_matches_source_and_sorts_deterministically() {
        let mut view = SoftwareView::default();
        view.push_query('b');
        view.push_query('r');
        let brew = InstalledSoftware {
            id: "macos.package.brew".to_string(),
            name: "Brew Tool".to_string(),
            version: Some("2.0".to_string()),
            source_id: "macos.homebrew.formulae".to_string(),
            identity_group_id: "software.brew".to_string(),
            identity_confidence: IdentityConfidence::ExactEvidence,
            kind: SoftwareKind::HomebrewFormula,
            scope: InstallScope::Manager,
            uninstall_option: UninstallOption::ManagerReview,
        };
        let app = InstalledSoftware {
            id: "macos.app.other".to_string(),
            name: "Other App".to_string(),
            version: Some("1.0".to_string()),
            source_id: "macos.application_bundles".to_string(),
            identity_group_id: "software.other".to_string(),
            identity_confidence: IdentityConfidence::ExactEvidence,
            kind: SoftwareKind::ApplicationBundle,
            scope: InstallScope::Local,
            uninstall_option: UninstallOption::QuarantineReview,
        };
        assert!(view.matches(&brew));
        assert!(!view.matches(&app));
        assert_eq!(view.compare(&brew, &app), std::cmp::Ordering::Less);
        assert_eq!(SoftwareFilter::Reviewable.next(), SoftwareFilter::All);
        assert_eq!(SoftwareSort::Kind.next(), SoftwareSort::Name);
    }
}
