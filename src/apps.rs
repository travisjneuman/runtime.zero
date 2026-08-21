use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rz0_inventory_contract::{AppRecord, SoftwareIdentifier, ToolRecord};
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
    pub service_count: usize,
    pub identity_group_count: usize,
    pub identity_groups: Vec<SoftwareIdentityGroup>,
    pub apps: Vec<InstalledSoftware>,
    #[serde(skip)]
    pub(crate) known_tools: Vec<ToolRecord>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstalledSoftware {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub source_id: String,
    pub identifiers: Vec<SoftwareIdentifier>,
    pub identity_group_id: String,
    pub identity_confidence: IdentityConfidence,
    pub kind: SoftwareKind,
    pub scope: InstallScope,
    pub uninstall_option: UninstallOption,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SoftwareUpdate {
    pub finding_id: String,
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
        if !self.filter.matches(app) {
            return false;
        }
        if self.query.is_empty() {
            return true;
        }

        let query = self.query.to_ascii_lowercase();
        app.name.to_ascii_lowercase().contains(&query)
            || app.id.to_ascii_lowercase().contains(&query)
            || app.source_id.to_ascii_lowercase().contains(&query)
            || app.identifiers.iter().any(|identifier| {
                identifier.kind.to_ascii_lowercase().contains(&query)
                    || identifier.value.to_ascii_lowercase().contains(&query)
            })
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
    pub finding_report: rz0_finding_contract::FindingReport,
    pub action_plan: Option<rz0_action_plan::ActionPlan>,
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
        .chain(report.warnings.iter().cloned())
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
        service_count: report.summary.service_count,
        identity_group_count: identity_groups.len(),
        identity_groups,
        apps,
        known_tools: report.tools,
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
        "macos.macports.packages" => (
            SoftwareKind::PlatformPackage,
            InstallScope::Manager,
            UninstallOption::ManagerReview,
        ),
        "macos.package_receipts" => (
            SoftwareKind::PlatformPackage,
            InstallScope::Unknown,
            UninstallOption::Unsupported,
        ),
        "linux.dpkg.packages" | "linux.pacman.packages" => (
            SoftwareKind::PlatformPackage,
            InstallScope::Manager,
            UninstallOption::ManagerReview,
        ),
        "linux.flatpak.packages" => (
            SoftwareKind::PlatformPackage,
            InstallScope::Unknown,
            UninstallOption::Unsupported,
        ),
        "windows.installed_apps" | "linux.desktop_entries" => (
            SoftwareKind::PlatformPackage,
            InstallScope::Unknown,
            UninstallOption::Unsupported,
        ),
        "macos.application_bundles" => classify_bundle(app.install_location.as_deref()),
        _ => (
            SoftwareKind::PlatformPackage,
            InstallScope::Unknown,
            UninstallOption::Unsupported,
        ),
    };
    InstalledSoftware {
        id: app.id.clone(),
        name: app.name.clone(),
        version: app.version.clone(),
        source_id: app.source_id.clone(),
        identifiers: app.identifiers.clone(),
        identity_group_id: String::new(),
        identity_confidence: IdentityConfidence::ExactEvidence,
        kind,
        scope,
        uninstall_option,
    }
}

fn assign_identity_groups(apps: &mut [InstalledSoftware]) -> Vec<SoftwareIdentityGroup> {
    let mut parents = (0..apps.len()).collect::<Vec<_>>();
    let mut identifier_counts = BTreeMap::<String, usize>::new();
    let mut identifier_owner = BTreeMap::<String, usize>::new();
    let mut identifier_edges = Vec::new();
    for (index, app) in apps.iter().enumerate() {
        for identifier in &app.identifiers {
            let key = software_identifier_key(identifier);
            *identifier_counts.entry(key.clone()).or_default() += 1;
            if let Some(owner) = identifier_owner.insert(key, index) {
                union_groups(&mut parents, owner, index);
                identifier_edges.push((owner, index));
            }
        }
    }

    // Exact identifiers are reconciled first. Name-only joins are retained as
    // explicit heuristic evidence only when exact identity did not already
    // connect the records.
    let mut name_owner = BTreeMap::<String, usize>::new();
    let mut heuristic_edges = Vec::new();
    for (index, app) in apps.iter().enumerate() {
        let key = normalized_name_key(app);
        if let Some(owner) = name_owner.insert(key, index)
            && find_group(&mut parents, owner) != find_group(&mut parents, index)
        {
            union_groups(&mut parents, owner, index);
            heuristic_edges.push((owner, index));
        }
    }

    let mut by_root = BTreeMap::<usize, Vec<usize>>::new();
    for index in 0..apps.len() {
        let root = find_group(&mut parents, index);
        by_root.entry(root).or_default().push(index);
    }

    let mut groups = Vec::with_capacity(by_root.len());
    for indexes in by_root.into_values() {
        let index_set = indexes.iter().copied().collect::<BTreeSet<_>>();
        let shared_identifiers = indexes
            .iter()
            .flat_map(|index| apps[*index].identifiers.iter())
            .map(software_identifier_key)
            .filter(|key| identifier_counts.get(key).copied().unwrap_or_default() > 1)
            .collect::<BTreeSet<_>>();
        let all_identifiers = indexes
            .iter()
            .flat_map(|index| apps[*index].identifiers.iter())
            .map(software_identifier_key)
            .collect::<BTreeSet<_>>();
        let key = shared_identifiers
            .first()
            .or_else(|| {
                (indexes.len() == 1)
                    .then(|| all_identifiers.first())
                    .flatten()
            })
            .map_or_else(
                || format!("name:{}", normalized_name_key(&apps[indexes[0]])),
                |identifier| format!("identifier:{identifier}"),
            );
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
        let exact_link = identifier_edges
            .iter()
            .any(|(left, right)| index_set.contains(left) && index_set.contains(right));
        let heuristic_link = heuristic_edges
            .iter()
            .any(|(left, right)| index_set.contains(left) && index_set.contains(right));
        let confidence = if version_disagreement {
            IdentityConfidence::Disputed
        } else if heuristic_link {
            IdentityConfidence::Heuristic
        } else if exact_link {
            IdentityConfidence::ExactEvidence
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

fn find_group(parents: &mut [usize], index: usize) -> usize {
    if parents[index] != index {
        parents[index] = find_group(parents, parents[index]);
    }
    parents[index]
}

fn union_groups(parents: &mut [usize], left: usize, right: usize) {
    let left = find_group(parents, left);
    let right = find_group(parents, right);
    if left != right {
        let (first, second) = if left < right {
            (left, right)
        } else {
            (right, left)
        };
        parents[second] = first;
    }
}

pub fn software_name_key(value: &str) -> String {
    value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect::<String>()
}

fn normalized_name_key(app: &InstalledSoftware) -> String {
    let mut normalized = software_name_key(&app.name);
    if normalized.len() < 3 {
        normalized = app.id.clone();
    }
    normalized
}

fn software_identifier_key(identifier: &SoftwareIdentifier) -> String {
    format!("{}:{}", identifier.kind, identifier.value.to_lowercase())
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
    if args.len() == 2
        && matches!(args[0].as_str(), "plan" | "apply")
        && matches!(args[1].as_str(), "--help" | "-h" | "help")
    {
        return (ExitCode::Ok, uninstall_usage(), String::new());
    }
    let Some(mode) = args.first().map(String::as_str) else {
        return (
            ExitCode::Usage,
            String::new(),
            format!(
                "uninstall requires a review plan or apply action\n\n{}",
                uninstall_usage()
            ),
        );
    };
    let apply = match mode {
        "plan" => false,
        "apply" => true,
        _ => {
            return (
                ExitCode::Usage,
                String::new(),
                format!(
                    "uninstall requires a review plan or apply action\n\n{}",
                    uninstall_usage()
                ),
            );
        }
    };
    let mut app_id = None;
    let mut executable = None;
    let mut format = AppOutputFormat::Text;
    let mut accept_no_rollback = false;
    let mut challenge_issued_unix_seconds = None;
    let mut confirmation = None;
    let mut index = 1usize;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => format = AppOutputFormat::Json,
            "--accept-no-rollback" if apply && !accept_no_rollback => {
                accept_no_rollback = true;
            }
            "--accept-no-rollback" => {
                return (
                    ExitCode::Usage,
                    String::new(),
                    format!(
                        "uninstall option was provided more than once\n\n{}",
                        uninstall_usage()
                    ),
                );
            }
            "--challenge-issued-unix-seconds" if apply => {
                let Some(value) = args.get(index + 1) else {
                    return (
                        ExitCode::Usage,
                        String::new(),
                        format!(
                            "uninstall apply requires a challenge timestamp\n\n{}",
                            uninstall_usage()
                        ),
                    );
                };
                if challenge_issued_unix_seconds.is_some() {
                    return (
                        ExitCode::Usage,
                        String::new(),
                        format!(
                            "uninstall challenge timestamp was provided more than once\n\n{}",
                            uninstall_usage()
                        ),
                    );
                }
                challenge_issued_unix_seconds = value.parse::<u64>().ok();
                if challenge_issued_unix_seconds.is_none() {
                    return (
                        ExitCode::Usage,
                        String::new(),
                        "uninstall challenge timestamp must be an integer\n".to_string(),
                    );
                }
                index += 1;
            }
            "--confirm" if apply => {
                let Some(value) = args.get(index + 1) else {
                    return (
                        ExitCode::Usage,
                        String::new(),
                        format!(
                            "uninstall apply requires an exact confirmation phrase\n\n{}",
                            uninstall_usage()
                        ),
                    );
                };
                if confirmation.replace(value.clone()).is_some() {
                    return (
                        ExitCode::Usage,
                        String::new(),
                        format!(
                            "uninstall confirmation was provided more than once\n\n{}",
                            uninstall_usage()
                        ),
                    );
                }
                index += 1;
            }
            "--executable" => {
                let Some(value) = args.get(index + 1) else {
                    return (
                        ExitCode::Usage,
                        String::new(),
                        format!(
                            "uninstall requires an exact executable path\n\n{}",
                            uninstall_usage()
                        ),
                    );
                };
                if executable.replace(value.clone()).is_some() {
                    return (
                        ExitCode::Usage,
                        String::new(),
                        format!(
                            "uninstall accepts --executable only once\n\n{}",
                            uninstall_usage()
                        ),
                    );
                }
                index += 1;
            }
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
                    format!("uninstall arguments are invalid\n\n{}", uninstall_usage()),
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
    if !apply
        && (accept_no_rollback || challenge_issued_unix_seconds.is_some() || confirmation.is_some())
    {
        return (
            ExitCode::Usage,
            String::new(),
            format!(
                "confirmation options require `uninstall apply`\n\n{}",
                uninstall_usage()
            ),
        );
    }
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
    if apply {
        return uninstall_apply_command(
            app,
            executable.as_deref(),
            format,
            accept_no_rollback,
            challenge_issued_unix_seconds,
            confirmation.as_deref(),
        );
    }
    let (finding_report, action_plan) = match shared_uninstall_evidence(app, executable.as_deref())
    {
        Ok(evidence) => evidence,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("shared uninstall planning failed closed: {error}\n"),
            );
        }
    };
    let review = build_uninstall_review(app, finding_report, action_plan);
    match format {
        AppOutputFormat::Text => (ExitCode::Ok, render_uninstall_text(&review), String::new()),
        AppOutputFormat::Json => match serde_json::to_string_pretty(&review) {
            Ok(json) => (ExitCode::Ok, format!("{json}\n"), String::new()),
            Err(error) => (ExitCode::Usage, String::new(), format!("{error}\n")),
        },
    }
}

fn shared_uninstall_evidence(
    app: &InstalledSoftware,
    executable: Option<&str>,
) -> Result<
    (
        rz0_finding_contract::FindingReport,
        Option<rz0_action_plan::ActionPlan>,
    ),
    String,
> {
    let manager = match app.source_id.as_str() {
        "macos.macports.packages" => Some((
            "macports",
            vec!["uninstall".to_string(), app.name.clone()],
            true,
        )),
        "linux.dpkg.packages" => Some(("apt", vec!["remove".to_string(), app.name.clone()], true)),
        "linux.pacman.packages" => Some(("pacman", vec!["-R".to_string(), app.name.clone()], true)),
        _ => match app.kind {
            SoftwareKind::HomebrewFormula => Some((
                "homebrew",
                vec!["uninstall".to_string(), app.name.clone()],
                false,
            )),
            SoftwareKind::HomebrewCask => Some((
                "homebrew",
                vec![
                    "uninstall".to_string(),
                    "--cask".to_string(),
                    app.name.clone(),
                ],
                false,
            )),
            SoftwareKind::ApplicationBundle | SoftwareKind::PlatformPackage => None,
        },
    };
    if executable.is_some() && manager.is_none() {
        return Err(
            "--executable is accepted only for an ownership-matched manager uninstall review"
                .to_string(),
        );
    }
    let executable_identity = match (manager.as_ref(), executable) {
        (Some((manager, _, _)), Some(executable)) => {
            if !Path::new(executable).is_absolute()
                || !rz0_module_uninstall::manager_executable_allowed(
                    manager,
                    std::env::consts::OS,
                    executable,
                )
            {
                return Err(
                    "uninstall executable is not the exact allowlisted manager path".to_string(),
                );
            }
            Some(crate::update_execution::observe_manager_executable(
                Path::new(executable),
            )?)
        }
        _ => None,
    };
    let manager_record_present = manager.is_some();
    let requires_elevation = manager
        .as_ref()
        .is_some_and(|(_, _, requires_elevation)| *requires_elevation);
    let record = rz0_module_uninstall::UninstallRecord {
        finding_id: format!("uninstall.{}", app.id),
        subject_reference: format!("software:{}", app.id),
        installed: true,
        manager_record_present,
        ownership: match app.uninstall_option {
            UninstallOption::ManagerReview => rz0_module_uninstall::UninstallOwnership::Manager,
            UninstallOption::Protected => rz0_module_uninstall::UninstallOwnership::System,
            UninstallOption::QuarantineReview => rz0_module_uninstall::UninstallOwnership::User,
            UninstallOption::Unsupported => rz0_module_uninstall::UninstallOwnership::Unknown,
        },
        manager: manager
            .as_ref()
            .map(|(manager, _, _)| (*manager).to_string()),
        executable: executable.map(str::to_string),
        executable_sha256: executable_identity
            .as_ref()
            .map(|identity| identity.sha256.clone()),
        executable_size_bytes: executable_identity.map(|identity| identity.size_bytes),
        arguments: manager.map_or_else(Vec::new, |(_, arguments, _)| arguments),
        requires_elevation,
        rollback_supported: false,
    };
    let evidence_bytes = serde_json::to_vec(&record)
        .map_err(|error| format!("serialize uninstall catalog evidence: {error}"))?;
    let evidence_sha256 = format!("{:x}", Sha256::digest(&evidence_bytes));
    let input = rz0_module_uninstall::UninstallFindingInput {
        schema_version: 1,
        contract: rz0_module_uninstall::INPUT_CONTRACT.to_string(),
        platform: std::env::consts::OS.to_string(),
        input_evidence_sha256: evidence_sha256.clone(),
        source_id: app.source_id.clone(),
        source_evidence_sha256: evidence_sha256,
        records: vec![record],
    };
    let report = rz0_module_uninstall::classify_uninstalls(&input)?;
    let plan = if manager_record_present {
        Some(rz0_module_uninstall::build_uninstall_action_plan(
            &input, &report,
        )?)
    } else {
        None
    };
    Ok((report, plan))
}

fn uninstall_apply_command(
    app: &InstalledSoftware,
    executable: Option<&str>,
    format: AppOutputFormat,
    accept_no_rollback: bool,
    challenge_issued_unix_seconds: Option<u64>,
    confirmation: Option<&str>,
) -> (ExitCode, String, String) {
    if app.uninstall_option != UninstallOption::ManagerReview {
        return (
            ExitCode::Usage,
            String::new(),
            "uninstall apply is available only for manager-owned records; protected, user-owned, and unknown software remain report-only\n".to_string(),
        );
    }
    let Some(executable) = executable else {
        return (
            ExitCode::Usage,
            String::new(),
            format!(
                "uninstall apply requires --executable <absolute-manager-path>\n\n{}",
                uninstall_usage()
            ),
        );
    };
    let (_finding_report, action_plan) = match shared_uninstall_evidence(app, Some(executable)) {
        Ok(evidence) => evidence,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("shared uninstall planning failed closed: {error}\n"),
            );
        }
    };
    let Some(plan) = action_plan else {
        return (
            ExitCode::Usage,
            String::new(),
            "manager-owned uninstall did not produce an action plan\n".to_string(),
        );
    };
    let Some(action) = plan.actions.first() else {
        return (
            ExitCode::Usage,
            String::new(),
            "manager-owned uninstall action plan is empty\n".to_string(),
        );
    };
    if action.disposition != rz0_action_plan::ActionDisposition::Planned {
        return (
            ExitCode::Usage,
            String::new(),
            "manager-owned uninstall action is blocked by missing exact command evidence\n"
                .to_string(),
        );
    }
    if !action.rollback.supported && !accept_no_rollback {
        return (
            ExitCode::Usage,
            String::new(),
            "this uninstall has no proven rollback path; pass --accept-no-rollback to acknowledge manual recovery risk\n".to_string(),
        );
    }
    let single_plan = match crate::update_execution::make_single_action_plan(&plan, action) {
        Ok(plan) => plan,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("single-action uninstall plan failed closed: {error}\n"),
            );
        }
    };
    let single_action = &single_plan.actions[0];
    let issued = challenge_issued_unix_seconds.unwrap_or_else(unix_seconds);
    let (challenge, view) = match crate::update_execution::build_update_challenge(
        &single_plan,
        single_action,
        accept_no_rollback,
        issued,
    ) {
        Ok(challenge) => challenge,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("uninstall confirmation challenge failed closed: {error}\n"),
            );
        }
    };
    let Some(confirmation) = confirmation else {
        return (
            ExitCode::Ok,
            render_uninstall_challenge(&view, format),
            String::new(),
        );
    };
    if challenge_issued_unix_seconds.is_none() {
        return (
            ExitCode::Usage,
            String::new(),
            "uninstall apply requires --challenge-issued-unix-seconds from the exact dry-run challenge\n".to_string(),
        );
    }
    let response = match crate::update_execution::validate_update_confirmation(
        &challenge,
        confirmation,
        unix_seconds(),
    ) {
        Ok(response) => response,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("uninstall confirmation rejected: {error}\n"),
            );
        }
    };
    let state_root = PathBuf::from(
        crate::module_store::module_store_plan(None, None, "uninstall execution").state_root,
    );
    if !state_root.is_dir() {
        return (
            ExitCode::Usage,
            String::new(),
            "runtime.zero state store is not initialized; run `rz0 store init --dry-run` before explicit initialization\n".to_string(),
        );
    }
    let (controller, cancellation) = rz0_cancellation_contract::cancellation_pair();
    let _interrupt = match crate::update_cli::InterruptBridge::install(controller) {
        Ok(bridge) => bridge,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("install uninstall cancellation bridge: {error}\n"),
            );
        }
    };
    let app_id = app.id.clone();
    let result = crate::update_execution::execute_uninstall_action(
        crate::update_execution::UpdateExecutionRequest {
            state_root: &state_root,
            plan: &single_plan,
            action: single_action,
            challenge: &challenge,
            response: &response,
            now_unix_seconds: unix_seconds(),
            environment: crate::update_cli::probe_environment(),
            cancellation: &cancellation,
            verify_after: |cancellation| {
                if let Some(reason) = cancellation.reason() {
                    return Err(format!(
                        "uninstall verification cancelled before fresh inventory: {reason:?}"
                    ));
                }
                let fresh = collect_app_catalog()?;
                if let Some(reason) = cancellation.reason() {
                    return Err(format!(
                        "uninstall verification cancelled after fresh inventory: {reason:?}"
                    ));
                }
                if fresh.apps.iter().any(|candidate| candidate.id == app_id) {
                    Err(
                        "fresh installed-software inventory still reports the exact target"
                            .to_string(),
                    )
                } else {
                    Ok(
                        "fresh installed-software inventory no longer reports the exact target"
                            .to_string(),
                    )
                }
            },
        },
    );
    match result {
        Ok(report) => (
            ExitCode::Ok,
            render_uninstall_execution(&report, format),
            String::new(),
        ),
        Err(error) => (ExitCode::Usage, String::new(), format!("{error}\n")),
    }
}

fn render_uninstall_challenge(
    view: &crate::update_execution::UpdateChallengeView,
    format: AppOutputFormat,
) -> String {
    match format {
        AppOutputFormat::Text => format!(
            "runtime.zero uninstall confirmation\n\noperation: {:?}\nmanager: {}\ntarget: {}\ncommand_arguments: {:?}\nrisk: {:?}\nrequires_elevation: {}\nnetwork_required: {}\nexecutable_sha256: {}\nexecutable_size_bytes: {}\ncapabilities: {:?}\nplan_id: {}\naction_id: {}\nplan_sha256: {}\nissued_unix_seconds: {}\nexpires_unix_seconds: {}\nrollback_available: {}\nmanual_recovery_acknowledged: {}\n\nType this exact phrase in a new command invocation and pass --challenge-issued-unix-seconds {}:\n{}\n\nNo manager command was executed.\n",
            view.operation,
            view.manager.as_deref().unwrap_or("unknown"),
            view.target,
            view.arguments,
            view.risk,
            view.requires_elevation,
            view.network_required,
            view.executable_sha256.as_deref().unwrap_or("unavailable"),
            view.executable_size_bytes
                .map_or_else(|| "unavailable".to_string(), |size| size.to_string()),
            view.capabilities,
            view.plan_id,
            view.action_id,
            view.plan_sha256,
            view.issued_unix_seconds,
            view.expires_unix_seconds,
            view.rollback_available,
            view.manual_recovery_acknowledged,
            view.issued_unix_seconds,
            view.expected_phrase,
        ),
        AppOutputFormat::Json => serde_json::to_string_pretty(view).map_or_else(
            |error| format!("challenge serialization failed: {error}\n"),
            |json| format!("{json}\n"),
        ),
    }
}

fn render_uninstall_execution(
    report: &crate::update_execution::UpdateExecutionReport,
    format: AppOutputFormat,
) -> String {
    match format {
        AppOutputFormat::Text => format!(
            "runtime.zero uninstall execution\n\noperation: uninstall\ntransaction_id: {}\naction_id: {}\nmanager: {}\ntarget: {}\nstatus: {:?}\nexit_code: {:?}\nverification: {}\nreceipt_reference: {}\nwrites_attempted: {}\nproduct_execution_authorized: {}\n",
            report.transaction_id,
            report.action_id,
            report.manager,
            report.target,
            report.status,
            report.exit_code,
            report.verification,
            report.receipt_reference,
            report.writes_attempted,
            report.product_execution_authorized,
        ),
        AppOutputFormat::Json => serde_json::to_string_pretty(report).map_or_else(
            |error| format!("execution serialization failed: {error}\n"),
            |json| format!("{json}\n"),
        ),
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn build_uninstall_review(
    app: &InstalledSoftware,
    finding_report: rz0_finding_contract::FindingReport,
    action_plan: Option<rz0_action_plan::ActionPlan>,
) -> UninstallReview {
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
                "Review the exact manager action; apply requires an allowlisted executable, destructive confirmation, and explicit no-rollback acknowledgement.".to_string(),
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
        finding_report,
        action_plan,
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
    let action = review
        .action_plan
        .as_ref()
        .and_then(|plan| plan.actions.first());
    format!(
        "runtime.zero uninstall review\n\napp: {}\nid: {}\nstatus: {}\noption: {:?}\nfinding_report_id: {}\naction_plan_id: {}\naction_disposition: {}\nconfirmation_required: {}\nrollback_required: {}\nwrites_attempted: no\nexecution_authorized: no\n\n{}\n",
        review.app_name,
        review.app_id,
        review.status,
        review.option,
        review.finding_report.report_id,
        review
            .action_plan
            .as_ref()
            .map_or("none", |plan| plan.plan_id.as_str()),
        action.map_or("none", |action| match action.disposition {
            rz0_action_plan::ActionDisposition::Planned => "planned",
            rz0_action_plan::ActionDisposition::Blocked => "blocked",
            rz0_action_plan::ActionDisposition::Unsupported => "unsupported",
        }),
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
    "Usage: rz0 uninstall plan <installed-software-id> [--executable <absolute-manager-path>] [--format text|json]\n       rz0 uninstall apply <installed-software-id> --executable <absolute-manager-path> --accept-no-rollback [--challenge-issued-unix-seconds <seconds>] [--confirm <exact-phrase>] [--format text|json]\n\n`plan` builds a live finding-bound, read-only uninstall review. `apply` is limited to manager-owned records, revalidates the exact manager executable, requires a short-lived destructive confirmation and explicit no-rollback acknowledgement, records the external effect through the shared transaction/receipt path, and verifies fresh installed-software inventory. It never recursively deletes files, handles user bundles, collects credentials, invokes interactive elevation, or authorizes automatic mutation.\n".to_string()
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
            identifiers: Vec::new(),
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
            identifiers: Vec::new(),
            identity_group_id: "software.fixture".to_string(),
            identity_confidence: IdentityConfidence::ExactEvidence,
            kind: SoftwareKind::ApplicationBundle,
            scope: InstallScope::Local,
            uninstall_option: UninstallOption::QuarantineReview,
        };
        let (report, plan) = shared_uninstall_evidence(&app, None).expect("shared evidence");
        let review = build_uninstall_review(&app, report, plan);
        assert_eq!(review.status, "review_available");
        assert!(!review.product_execution_authorized);
        assert!(!review.writes_attempted);
    }

    #[test]
    fn uninstall_apply_keeps_non_manager_software_report_only() {
        let app = InstalledSoftware {
            id: "macos.app.local".to_string(),
            name: "Local App".to_string(),
            version: None,
            source_id: "macos.application_bundles".to_string(),
            identifiers: Vec::new(),
            identity_group_id: "software.fixture".to_string(),
            identity_confidence: IdentityConfidence::ExactEvidence,
            kind: SoftwareKind::ApplicationBundle,
            scope: InstallScope::Local,
            uninstall_option: UninstallOption::QuarantineReview,
        };
        let (code, stdout, stderr) = uninstall_apply_command(
            &app,
            Some("/opt/homebrew/bin/brew"),
            AppOutputFormat::Text,
            true,
            None,
            None,
        );
        assert_eq!(code, ExitCode::Usage);
        assert!(stdout.is_empty());
        assert!(stderr.contains("manager-owned records"));
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
                identifiers: Vec::new(),
                install_location: Some("/Applications/Alpha Tool.app".to_string()),
                warnings: Vec::new(),
            }),
            classify_app(&AppRecord {
                id: "macos.package.alpha".to_string(),
                name: "Alpha Tool".to_string(),
                source_id: "macos.homebrew.casks".to_string(),
                version: Some("2.0".to_string()),
                publisher: Some("Homebrew".to_string()),
                identifiers: Vec::new(),
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
    fn shared_source_identifier_overrides_display_name_aliases() {
        let identifier = SoftwareIdentifier {
            kind: "bundle_id".to_string(),
            value: "dev.example.alpha".to_string(),
        };
        let mut apps = vec![
            InstalledSoftware {
                id: "macos.app.alpha".to_string(),
                name: "Alpha".to_string(),
                version: Some("1.0".to_string()),
                source_id: "macos.application_bundles".to_string(),
                identifiers: vec![identifier.clone()],
                identity_group_id: String::new(),
                identity_confidence: IdentityConfidence::Heuristic,
                kind: SoftwareKind::ApplicationBundle,
                scope: InstallScope::Local,
                uninstall_option: UninstallOption::QuarantineReview,
            },
            InstalledSoftware {
                id: "macos.receipt.alpha".to_string(),
                name: "Vendor Alpha Suite".to_string(),
                version: Some("1.0".to_string()),
                source_id: "macos.package_receipts".to_string(),
                identifiers: vec![identifier],
                identity_group_id: String::new(),
                identity_confidence: IdentityConfidence::Heuristic,
                kind: SoftwareKind::PlatformPackage,
                scope: InstallScope::Unknown,
                uninstall_option: UninstallOption::Unsupported,
            },
        ];
        let groups = assign_identity_groups(&mut apps);
        assert_eq!(groups.len(), 1);
        assert!(
            groups[0]
                .normalized_name
                .starts_with("identifier:bundle_id:")
        );
        assert_eq!(groups[0].confidence, IdentityConfidence::ExactEvidence);
        assert_eq!(groups[0].source_ids.len(), 2);
        assert_eq!(apps[0].identity_group_id, apps[1].identity_group_id);
    }

    #[test]
    fn identity_reconciliation_is_transitive_and_stabilizes_identified_singletons() {
        let bundle = SoftwareIdentifier {
            kind: "bundle_id".to_string(),
            value: "dev.example.alpha".to_string(),
        };
        let package = SoftwareIdentifier {
            kind: "manager_package".to_string(),
            value: "homebrew:alpha".to_string(),
        };
        let template = InstalledSoftware {
            id: "macos.app.alpha".to_string(),
            name: "Alpha".to_string(),
            version: Some("1.0".to_string()),
            source_id: "macos.application_bundles".to_string(),
            identifiers: vec![bundle.clone(), package.clone()],
            identity_group_id: String::new(),
            identity_confidence: IdentityConfidence::Heuristic,
            kind: SoftwareKind::ApplicationBundle,
            scope: InstallScope::Local,
            uninstall_option: UninstallOption::QuarantineReview,
        };
        let mut apps = vec![
            template.clone(),
            InstalledSoftware {
                id: "macos.package.alpha".to_string(),
                name: "Renamed Alpha".to_string(),
                source_id: "macos.homebrew.casks".to_string(),
                identifiers: vec![package],
                ..template.clone()
            },
            InstalledSoftware {
                id: "macos.receipt.alpha".to_string(),
                name: "Vendor Suite".to_string(),
                source_id: "macos.package_receipts".to_string(),
                identifiers: vec![bundle],
                ..template.clone()
            },
        ];
        let groups = assign_identity_groups(&mut apps);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].evidence_ids.len(), 3);
        assert_eq!(groups[0].confidence, IdentityConfidence::ExactEvidence);

        let mut singleton = vec![InstalledSoftware {
            id: "macos.app.changed-location".to_string(),
            ..template
        }];
        let first = assign_identity_groups(&mut singleton)[0].group_id.clone();
        singleton[0].id = "macos.app.other-location".to_string();
        singleton[0].name = "Alpha Renamed".to_string();
        let second = assign_identity_groups(&mut singleton)[0].group_id.clone();
        assert_eq!(first, second);
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
            identifiers: vec![SoftwareIdentifier {
                kind: "homebrew_formula".to_string(),
                value: "fixture-tool".to_string(),
            }],
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
            identifiers: Vec::new(),
            identity_group_id: "software.other".to_string(),
            identity_confidence: IdentityConfidence::ExactEvidence,
            kind: SoftwareKind::ApplicationBundle,
            scope: InstallScope::Local,
            uninstall_option: UninstallOption::QuarantineReview,
        };
        assert!(view.matches(&brew));
        assert!(!view.matches(&app));
        view.clear_query();
        for value in "fixture-tool".chars() {
            view.push_query(value);
        }
        assert!(view.matches(&brew));
        assert!(!view.matches(&app));
        assert_eq!(view.compare(&brew, &app), std::cmp::Ordering::Less);
        assert_eq!(SoftwareFilter::Reviewable.next(), SoftwareFilter::All);
        assert_eq!(SoftwareSort::Kind.next(), SoftwareSort::Name);
    }
}
