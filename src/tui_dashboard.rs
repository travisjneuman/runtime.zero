use rz0_action_plan::{ActionDisposition, ActionPlan, PlanAction};
use rz0_cancellation_contract::CancellationToken;
use serde::Serialize;

use crate::aiup::report_from_catalog as aiup_report_from_catalog;
use crate::apps::{
    AppCatalog, InstalledSoftware, SoftwareFilter, SoftwareKind, SoftwareUpdate, SoftwareView,
    UninstallOption, collect_app_catalog, collect_app_catalog_cancellable,
};
use crate::brand;
use crate::cache::{self, CacheReviewReport};
use crate::install_receipt::ReceiptInventoryState;
use crate::installed_registry::InstalledRegistryState;
use crate::leftovers::{self, LeftoversReviewReport};
use crate::module_registry::ModuleRegistryReport;
use crate::module_status::{ModuleStatusReport, module_status_report_from_store};
use crate::recovery_cli::{RecoverySummary, recovery_summary};
use crate::store_init::{StoreInitMode, StoreInitOptions, StoreInitStatus, store_init_report};
use crate::store_status::{StoreOverallState, StoreStatusReport, store_status_report};
use crate::system_monitor::{self, SystemSnapshot};
use crate::toolchain::{
    is_toolchain_record, is_toolchain_software, is_toolchain_text, toolchain_provider_id,
    toolchain_tools_from_catalog,
};
use crate::tui_dashboard_labels::{
    init_label, init_status_label, init_tone, receipt_label, receipt_state_label, receipt_tone,
    registry_label, registry_state_label, registry_tone, row, row_count, row_with_preview,
    store_state_label,
};
use crate::tui_theme;
use crate::update_cli::TuiUpdateChallenge;
use crate::update_execution::UpdateExecutionReport;
use crate::updates::{LiveUpdateCatalog, LiveUpdateReview, collect_live_update_review};
use rz0_inventory_contract::ToolRecord;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TuiDashboard {
    pub schema_version: u8,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub contract: &'static str,
    pub title: &'static str,
    pub command: &'static str,
    pub version: &'static str,
    pub mode: &'static str,
    pub safety_posture: &'static str,
    pub configuration_sha256: String,
    pub store_state: StoreOverallState,
    pub registry_state: InstalledRegistryState,
    pub receipt_state: ReceiptInventoryState,
    pub store_init_status: StoreInitStatus,
    pub installed_module_count: usize,
    pub inactive_module_count: usize,
    pub degraded_module_count: usize,
    pub staged_module_count: usize,
    pub invalid_staged_module_count: usize,
    pub module_lifecycle_execution_available: bool,
    pub planned_module_family_count: usize,
    pub installed_software_count: usize,
    pub service_and_persistence_count: usize,
    pub inventory_status: String,
    pub cache_status: String,
    pub cache_finding_count: usize,
    pub cache_warning_count: usize,
    pub leftovers_status: String,
    pub leftover_finding_count: usize,
    pub leftover_warning_count: usize,
    pub recovery_status: String,
    pub recovery_record_count: usize,
    pub recovery_restore_available_count: usize,
    pub recovery_transaction_count: usize,
    pub recovery_transaction_invalid_count: usize,
    pub recovery_transaction_action_required_count: usize,
    pub recovery_transaction_warning_count: usize,
    pub integrity_status: String,
    pub update_check_status: String,
    pub update_action_status: String,
    pub update_source_count: usize,
    pub update_candidate_count: usize,
    pub sections: Vec<TuiSection>,
    pub palette: TuiPalette,
    #[serde(skip)]
    software_catalog: Option<AppCatalog>,
    #[serde(skip)]
    inventory_error: Option<String>,
    #[serde(skip)]
    update_catalog: Option<LiveUpdateCatalog>,
    #[serde(skip)]
    update_plan: Option<ActionPlan>,
    #[serde(skip)]
    pending_update: Option<TuiUpdateChallenge>,
    #[serde(skip)]
    monitor_snapshot: Option<SystemSnapshot>,
    #[serde(skip)]
    cache_report: Option<CacheReviewReport>,
    #[serde(skip)]
    leftovers_report: Option<LeftoversReviewReport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TuiSection {
    pub code: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub rows: Vec<TuiRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TuiRow {
    pub label: &'static str,
    pub value: String,
    pub tone: &'static str,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TuiPalette {
    pub surface_bg: &'static str,
    pub panel_bg: &'static str,
    pub brand_accent: &'static str,
    pub text_primary: &'static str,
    pub text_muted: &'static str,
}

/// Shared navigation copy for the Ratatui and scriptable renderers.
pub const WORKSPACE_LABELS: [&str; 5] = ["Home", "Toolchain", "Software", "System", "Diagnostics"];

pub fn workspace_label(title: &str) -> String {
    match title {
        "overview" => WORKSPACE_LABELS[0].to_string(),
        "toolchain" => WORKSPACE_LABELS[1].to_string(),
        "software" => WORKSPACE_LABELS[2].to_string(),
        "system" => WORKSPACE_LABELS[3].to_string(),
        "diagnostics" => WORKSPACE_LABELS[4].to_string(),
        other => other.to_string(),
    }
}

pub fn workspace_heading(title: &str) -> String {
    if title == "overview" {
        "Home / next step".to_string()
    } else {
        workspace_label(title)
    }
}

pub fn dashboard() -> TuiDashboard {
    dashboard_with_inventory(true)
}

pub(crate) fn dashboard_cancellable(
    cancellation: &CancellationToken,
) -> Result<TuiDashboard, String> {
    check_dashboard_cancellation(cancellation)?;
    let store = store_status_report(&["tui".to_string()]);
    check_dashboard_cancellation(cancellation)?;
    let init = store_init_report(
        &["tui".to_string()],
        StoreInitOptions::new(StoreInitMode::DryRun),
    );
    check_dashboard_cancellation(cancellation)?;
    let modules = ModuleRegistryReport::empty_installed();
    let module_status = module_status_report_from_store(&store);
    check_dashboard_cancellation(cancellation)?;
    let inventory = Some(collect_app_catalog_cancellable(Some(cancellation)));
    check_dashboard_cancellation(cancellation)?;
    let monitor = Some(system_monitor::collect_snapshot(None));
    check_dashboard_cancellation(cancellation)?;
    let cache = Some(cache::live_report());
    check_dashboard_cancellation(cancellation)?;
    let leftovers = Some(leftovers::live_report());
    check_dashboard_cancellation(cancellation)?;
    let recovery = Some(recovery_summary(&store.store));
    check_dashboard_cancellation(cancellation)?;
    Ok(build_dashboard(
        &store,
        init.status,
        &modules,
        &module_status,
        DashboardEvidence {
            inventory,
            monitor,
            cache,
            leftovers,
            recovery,
        },
    ))
}

pub fn private_dashboard() -> TuiDashboard {
    dashboard_with_inventory(false)
}

pub fn loading_dashboard() -> TuiDashboard {
    let mut dashboard = private_dashboard();
    dashboard.inventory_status = "loading".to_string();
    dashboard.update_check_status = "not started".to_string();
    dashboard.update_action_status =
        "loading local snapshot · no provider action is running".to_string();
    if let Some(home) = dashboard.sections.first_mut() {
        home.rows.insert(
            1,
            row(
                tui_theme::LABEL_PLAN,
                "loading local inventory and system evidence",
                "accent",
            ),
        );
    }
    dashboard
}

pub fn mark_startup_load_failed(dashboard: &mut TuiDashboard, detail: &str) {
    dashboard.inventory_status = format!("unavailable · {detail}");
    dashboard.update_action_status =
        "startup load failed · press r to retry explicitly".to_string();
    if let Some(home) = dashboard.sections.first_mut() {
        home.rows.insert(
            1,
            row(
                tui_theme::LABEL_WARN,
                "local inventory did not load; no automatic retry was attempted",
                "warn",
            ),
        );
    }
}

fn dashboard_with_inventory(include_software_names: bool) -> TuiDashboard {
    let store = store_status_report(&["tui".to_string()]);
    let init = store_init_report(
        &["tui".to_string()],
        StoreInitOptions::new(StoreInitMode::DryRun),
    );
    let modules = ModuleRegistryReport::empty_installed();
    let module_status = module_status_report_from_store(&store);
    let inventory = include_software_names.then(collect_app_catalog);
    let monitor = include_software_names.then(|| system_monitor::collect_snapshot(None));
    let cache = include_software_names.then(cache::live_report);
    let leftovers = include_software_names.then(leftovers::live_report);
    let recovery = include_software_names.then(|| recovery_summary(&store.store));
    build_dashboard(
        &store,
        init.status,
        &modules,
        &module_status,
        DashboardEvidence {
            inventory,
            monitor,
            cache,
            leftovers,
            recovery,
        },
    )
}

fn check_dashboard_cancellation(cancellation: &CancellationToken) -> Result<(), String> {
    if let Some(reason) = cancellation.reason() {
        return Err(format!("dashboard load cancelled: {reason:?}"));
    }
    Ok(())
}

fn build_dashboard(
    store: &StoreStatusReport,
    init_status: StoreInitStatus,
    modules: &ModuleRegistryReport,
    module_status: &ModuleStatusReport,
    evidence: DashboardEvidence,
) -> TuiDashboard {
    let DashboardEvidence {
        inventory,
        monitor,
        cache,
        leftovers,
        recovery,
    } = evidence;
    let catalog = inventory
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned();
    let inventory_status = match &inventory {
        Some(Ok(catalog)) => format!("live · {} items", catalog.app_count),
        Some(Err(_)) => "unavailable".to_string(),
        None => "private summary".to_string(),
    };
    let inventory_error = inventory
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let cache_value = cache
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned();
    let cache_status = match &cache {
        Some(Ok(report)) => {
            let aged_file_count = report
                .observations
                .iter()
                .map(|observation| observation.files_older_than_review_threshold)
                .sum::<usize>();
            let active_use_state = if report
                .observations
                .iter()
                .any(|observation| observation.active_use_state != "unknown")
            {
                "possible lock marker"
            } else {
                "active use unknown"
            };
            format!(
                "live · {} bounded observations · {} over age threshold · {}",
                report.finding_report.summary.finding_count, aged_file_count, active_use_state
            )
        }
        Some(Err(_)) => "unavailable".to_string(),
        None => "private summary".to_string(),
    };
    let cache_finding_count = cache_value
        .as_ref()
        .map_or(0, |report| report.finding_report.summary.finding_count);
    let cache_warning_count = cache_value
        .as_ref()
        .map_or(0, |report| report.warnings.len());
    let leftovers_value = leftovers
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned();
    let leftovers_status = match &leftovers {
        Some(Ok(report)) => format!(
            "live · {} bounded observations",
            report.finding_report.summary.finding_count
        ),
        Some(Err(_)) => "unavailable".to_string(),
        None => "private summary".to_string(),
    };
    let leftover_finding_count = leftovers_value
        .as_ref()
        .map_or(0, |report| report.finding_report.summary.finding_count);
    let leftover_warning_count = leftovers_value
        .as_ref()
        .map_or(0, |report| report.warnings.len());
    let recovery_status = recovery.as_ref().map_or_else(
        || "private summary".to_string(),
        |summary| {
            format!(
                "{} · {} valid · {} restore-capable · journals {} · {} invalid · {} action-required · {} review warnings",
                summary.quarantine_root_state,
                summary.valid_count,
                summary.restore_available_count,
                summary.checked_transaction_count,
                summary.invalid_transaction_count,
                summary.transaction_action_required_count,
                summary.transaction_warning_count,
            )
        },
    );
    let recovery_record_count = recovery.as_ref().map_or(0, |summary| summary.checked_count);
    let recovery_restore_available_count = recovery
        .as_ref()
        .map_or(0, |summary| summary.restore_available_count);
    let recovery_transaction_count = recovery
        .as_ref()
        .map_or(0, |summary| summary.checked_transaction_count);
    let recovery_transaction_invalid_count = recovery
        .as_ref()
        .map_or(0, |summary| summary.invalid_transaction_count);
    let recovery_transaction_action_required_count = recovery
        .as_ref()
        .map_or(0, |summary| summary.transaction_action_required_count);
    let recovery_transaction_warning_count = recovery
        .as_ref()
        .map_or(0, |summary| summary.transaction_warning_count);
    let integrity_status = "baseline unavailable · fixture or exact-file evidence".to_string();
    let configuration = rz0_configuration_contract::default_configuration();
    let configuration_sha256 = rz0_configuration_contract::configuration_sha256(&configuration)
        .expect("built-in foundation configuration is canonical");
    let default_view = SoftwareView::default();
    TuiDashboard {
        schema_version: 1,
        read_only: true,
        writes_attempted: false,
        contract: "foundation_dashboard",
        title: brand::TITLE,
        command: brand::COMMAND,
        version: env!("CARGO_PKG_VERSION"),
        mode: "interactive dashboard",
        safety_posture: brand::SAFETY_POSTURE,
        configuration_sha256: configuration_sha256.clone(),
        store_state: store.overall_state,
        registry_state: store.registry.status,
        receipt_state: store.receipts.overall_state,
        store_init_status: init_status,
        installed_module_count: store.registry.installed_module_count,
        inactive_module_count: module_status.inactive_module_count,
        degraded_module_count: module_status.degraded_module_count,
        staged_module_count: module_status.staged_module_count,
        invalid_staged_module_count: module_status.invalid_staged_module_count,
        module_lifecycle_execution_available: module_status.lifecycle_execution_available,
        planned_module_family_count: modules.summary.planned_family_count,
        installed_software_count: catalog.as_ref().map_or(0, |catalog| catalog.app_count),
        service_and_persistence_count: catalog.as_ref().map_or(0, |catalog| catalog.service_count),
        inventory_status,
        cache_status,
        cache_finding_count,
        cache_warning_count,
        leftovers_status,
        leftover_finding_count,
        leftover_warning_count,
        recovery_status,
        recovery_record_count,
        recovery_restore_available_count,
        recovery_transaction_count,
        recovery_transaction_invalid_count,
        recovery_transaction_action_required_count,
        recovery_transaction_warning_count,
        integrity_status: integrity_status.clone(),
        update_check_status: "not checked".to_string(),
        update_action_status: "idle · u scans providers · review action requires confirmation"
            .to_string(),
        update_source_count: 0,
        update_candidate_count: 0,
        sections: sections(SectionContext {
            store,
            init_status,
            modules,
            module_status,
            catalog: catalog.as_ref(),
            inventory_error: inventory_error.as_deref(),
            cache: cache_value.as_ref(),
            leftovers: leftovers_value.as_ref(),
            recovery: recovery.as_ref(),
            integrity_status: &integrity_status,
            view: &default_view,
            updates: None,
            update_plan: None,
            pending_update: None,
            update_status: "not checked",
            update_action_status: "idle · u scans providers · review action requires confirmation",
            monitor: monitor.as_ref(),
            configuration_sha256: &configuration_sha256,
        }),
        palette: palette(),
        software_catalog: catalog,
        inventory_error,
        update_catalog: None,
        update_plan: None,
        pending_update: None,
        monitor_snapshot: monitor,
        cache_report: cache_value,
        leftovers_report: leftovers_value,
    }
}

struct SectionContext<'a> {
    store: &'a StoreStatusReport,
    init_status: StoreInitStatus,
    modules: &'a ModuleRegistryReport,
    module_status: &'a ModuleStatusReport,
    catalog: Option<&'a AppCatalog>,
    inventory_error: Option<&'a str>,
    cache: Option<&'a CacheReviewReport>,
    leftovers: Option<&'a LeftoversReviewReport>,
    recovery: Option<&'a RecoverySummary>,
    integrity_status: &'a str,
    view: &'a SoftwareView,
    updates: Option<&'a LiveUpdateCatalog>,
    update_plan: Option<&'a ActionPlan>,
    pending_update: Option<&'a TuiUpdateChallenge>,
    update_status: &'a str,
    update_action_status: &'a str,
    monitor: Option<&'a SystemSnapshot>,
    configuration_sha256: &'a str,
}

struct DashboardEvidence {
    inventory: Option<Result<AppCatalog, String>>,
    monitor: Option<SystemSnapshot>,
    cache: Option<Result<CacheReviewReport, String>>,
    leftovers: Option<Result<LeftoversReviewReport, String>>,
    recovery: Option<RecoverySummary>,
}

struct EvidenceContext<'a> {
    cache: Option<&'a CacheReviewReport>,
    leftovers: Option<&'a LeftoversReviewReport>,
    recovery: Option<&'a RecoverySummary>,
    integrity_status: &'a str,
    configuration_sha256: &'a str,
}

fn sections(context: SectionContext<'_>) -> Vec<TuiSection> {
    let SectionContext {
        store,
        init_status,
        modules,
        module_status,
        catalog,
        inventory_error,
        cache,
        leftovers,
        recovery,
        integrity_status,
        view,
        updates,
        update_plan,
        pending_update,
        update_status,
        update_action_status,
        monitor,
        configuration_sha256,
    } = context;
    let mut sections = vec![
        overview_section(
            catalog,
            inventory_error,
            view,
            updates,
            update_status,
            update_action_status,
        ),
        installed_software_section(
            catalog,
            inventory_error,
            view,
            updates,
            update_plan,
            pending_update,
            true,
        ),
        installed_software_section(
            catalog,
            inventory_error,
            view,
            updates,
            update_plan,
            pending_update,
            false,
        ),
        diagnostics_section(
            store,
            init_status,
            modules,
            module_status,
            inventory_error,
            update_action_status,
            EvidenceContext {
                cache,
                leftovers,
                recovery,
                integrity_status,
                configuration_sha256,
            },
        ),
    ];
    sections.insert(3, system_monitor_section(monitor));
    sections
}

fn diagnostics_section(
    store: &StoreStatusReport,
    init_status: StoreInitStatus,
    modules: &ModuleRegistryReport,
    module_status: &ModuleStatusReport,
    inventory_error: Option<&str>,
    update_action_status: &str,
    evidence: EvidenceContext<'_>,
) -> TuiSection {
    let EvidenceContext {
        cache,
        leftovers,
        recovery,
        integrity_status,
        configuration_sha256,
    } = evidence;
    let mut rows = vec![
        row(
            tui_theme::LABEL_INFO,
            store_state_label(store.overall_state),
            "info",
        ),
        row(
            init_label(init_status),
            init_status_label(init_status),
            init_tone(init_status),
        ),
        row(
            registry_label(store.registry.status),
            registry_state_label(store.registry.status),
            registry_tone(store.registry.status),
        ),
        row(
            receipt_label(store.receipts.overall_state),
            receipt_state_label(store.receipts.overall_state),
            receipt_tone(store.receipts.overall_state),
        ),
        row_count(
            tui_theme::LABEL_INFO,
            module_status.inactive_module_count,
            "installed inactive modules",
            "info",
        ),
        row_count(
            if module_status.degraded_module_count == 0 {
                tui_theme::LABEL_INFO
            } else {
                tui_theme::LABEL_WARN
            },
            module_status.degraded_module_count,
            "degraded modules",
            if module_status.degraded_module_count == 0 {
                "info"
            } else {
                "warn"
            },
        ),
        row_count(
            if module_status.invalid_staged_module_count == 0 {
                tui_theme::LABEL_INFO
            } else {
                tui_theme::LABEL_WARN
            },
            module_status.staged_module_count,
            "developer-staged modules",
            if module_status.invalid_staged_module_count == 0 {
                "info"
            } else {
                "warn"
            },
        ),
        row_count(
            if module_status.invalid_staged_module_count == 0 {
                tui_theme::LABEL_INFO
            } else {
                tui_theme::LABEL_WARN
            },
            module_status.invalid_staged_module_count,
            "staged modules requiring review",
            if module_status.invalid_staged_module_count == 0 {
                "info"
            } else {
                "warn"
            },
        ),
        row_count(
            tui_theme::LABEL_PLAN,
            modules.summary.planned_family_count,
            "planned first-party families",
            "accent",
        ),
        row(
            tui_theme::LABEL_PLAN,
            "inventory adapter is built in; domain modules remain isolated",
            "accent",
        ),
        row(
            tui_theme::LABEL_INFO,
            "module lifecycle execution unavailable · use rz0 modules status for redacted detail",
            "info",
        ),
        row_with_preview(
            tui_theme::LABEL_INFO,
            "effective policy: built-in defaults · network deny",
            "info",
            &format!(
                "configuration {configuration_sha256}; user configuration is not loaded; production modules, remote execution, shell execution, telemetry, automatic updates, background services, and startup repair remain disabled"
            ),
        ),
        match (cache, leftovers) {
            (Some(cache), Some(leftovers)) => {
                let cache_count = cache.finding_report.summary.finding_count;
                let aged_count = cache
                    .observations
                    .iter()
                    .map(|observation| observation.files_older_than_review_threshold)
                    .sum::<usize>();
                let leftovers_count = leftovers.finding_report.summary.finding_count;
                let active_use = if cache
                    .observations
                    .iter()
                    .any(|observation| observation.active_use_state != "unknown")
                {
                    "possible lock marker"
                } else {
                    "active use unknown"
                };
                let tone = if cache.warnings.is_empty() && leftovers.warnings.is_empty() {
                    "info"
                } else {
                    "warn"
                };
                row_with_preview(
                    if tone == "info" {
                        tui_theme::LABEL_INFO
                    } else {
                        tui_theme::LABEL_WARN
                    },
                    &format!(
                        "cache {cache_count} · age {aged_count} · leftovers {leftovers_count}"
                    ),
                    tone,
                    &format!(
                        "bounded evidence: cache {cache_count} findings, {aged_count} over the review age threshold, {active_use}; leftovers {leftovers_count}; integrity: {integrity_status}. Warnings: {}.",
                        cache.warnings.len() + leftovers.warnings.len()
                    ),
                )
            }
            _ => row(
                tui_theme::LABEL_WARN,
                "bounded cache/leftovers evidence unavailable",
                "warn",
            ),
        },
        match recovery {
            Some(summary) => {
                let tone = if summary.invalid_count == 0
                    && summary.invalid_transaction_count == 0
                    && summary.transaction_warning_count == 0
                    && summary.quarantine_root_state != "invalid"
                {
                    "info"
                } else {
                    "warn"
                };
                row_with_preview(
                    if tone == "info" {
                        tui_theme::LABEL_INFO
                    } else {
                        tui_theme::LABEL_WARN
                    },
                    &format!(
                        "recovery {} valid · {} restore · {} journals",
                        summary.valid_count,
                        summary.restore_available_count,
                        summary.checked_transaction_count
                    ),
                    tone,
                    &format!(
                        "recovery review: {} quarantine records, {} valid, {} restore-capable; journals {} checked, {} invalid, {} action-required, {} review warnings.",
                        summary.checked_count,
                        summary.valid_count,
                        summary.restore_available_count,
                        summary.checked_transaction_count,
                        summary.invalid_transaction_count,
                        summary.transaction_action_required_count,
                        summary.transaction_warning_count
                    ),
                )
            }
            None => row(
                tui_theme::LABEL_INFO,
                "quarantine recovery review available from the CLI",
                "info",
            ),
        },
        row(tui_theme::LABEL_INFO, update_action_status, "info"),
    ];
    if let Some(error) = inventory_error {
        rows.push(row(
            tui_theme::LABEL_WARN,
            &format!("inventory error: {error}"),
            "warn",
        ));
    }
    TuiSection {
        code: "05",
        title: "diagnostics",
        summary: "store, receipts, registry, modules, and recovery evidence",
        rows,
    }
}

fn system_monitor_section(snapshot: Option<&SystemSnapshot>) -> TuiSection {
    let Some(snapshot) = snapshot else {
        return TuiSection {
            code: "04",
            title: "system",
            summary: "native system monitor and bounded runtime evidence",
            rows: vec![row(
                tui_theme::LABEL_WARN,
                "system monitor unavailable on this platform",
                "warn",
            )],
        };
    };
    let cpu_usage = snapshot
        .cpu
        .usage_percent
        .map(|value| format!("{value}%"))
        .unwrap_or_else(|| "sampling".to_string());
    let load = snapshot
        .cpu
        .load_average_milli
        .iter()
        .map(|value| {
            value
                .map(|value| format!("{}.{:03}", value / 1000, value % 1000))
                .unwrap_or_else(|| "?".to_string())
        })
        .collect::<Vec<_>>()
        .join(" ");
    let memory_used = system_monitor::format_bytes(snapshot.memory.used_bytes);
    let memory_available = system_monitor::format_bytes(snapshot.memory.available_bytes);
    let memory_total = system_monitor::format_bytes(snapshot.memory.total_bytes);
    let received = system_monitor::format_bytes(snapshot.network.received_bytes);
    let transmitted = system_monitor::format_bytes(snapshot.network.transmitted_bytes);
    let mut rows = vec![
        row_with_preview(
            tui_theme::LABEL_OK,
            &format!(
                "CPU {cpu_usage} · {} cores",
                snapshot.cpu.logical_cpus.unwrap_or(0)
            ),
            "safe",
            &format!(
                "CPU usage: {cpu_usage}; load averages: {load}; logical CPUs: {}.",
                snapshot.cpu.logical_cpus.unwrap_or(0)
            ),
        ),
        row_with_preview(
            tui_theme::LABEL_INFO,
            &format!("memory {memory_used} used · {memory_available} free"),
            "info",
            &format!(
                "memory: {memory_used} used, {memory_available} available of {memory_total} total."
            ),
        ),
        row_with_preview(
            tui_theme::LABEL_INFO,
            &format!("network {} interfaces", snapshot.network.interface_count),
            "info",
            &format!(
                "network: {} interfaces; received {received}; sent {transmitted}.",
                snapshot.network.interface_count
            ),
        ),
        row_with_preview(
            tui_theme::LABEL_INFO,
            &format!(
                "processes {} total · {} running",
                snapshot.processes.total, snapshot.processes.running
            ),
            "info",
            &format!(
                "processes: {} total, {} running. Top process details are available in this workspace.",
                snapshot.processes.total, snapshot.processes.running
            ),
        ),
    ];
    for disk in &snapshot.disks {
        rows.push(row_with_preview(
            tui_theme::LABEL_INFO,
            &format!(
                "disk {} · {} used",
                disk.mount,
                system_monitor::format_bytes(disk.used_bytes)
            ),
            "info",
            &format!(
                "disk {}: {} used, {} available.",
                disk.mount,
                system_monitor::format_bytes(disk.used_bytes),
                system_monitor::format_bytes(disk.available_bytes)
            ),
        ));
    }
    for process in &snapshot.processes.top {
        rows.push(row(
            tui_theme::LABEL_INFO,
            &format!(
                "{} (pid {}) · cpu {} · memory {}",
                process.name,
                process.pid,
                process
                    .cpu_percent
                    .map(|value| format!("{value}%"))
                    .unwrap_or_else(|| "sampling".to_string()),
                system_monitor::format_bytes(process.memory_bytes)
            ),
            "muted",
        ));
    }
    for warning in &snapshot.warnings {
        rows.push(row(tui_theme::LABEL_WARN, warning, "warn"));
    }
    TuiSection {
        code: "04",
        title: "system",
        summary: "live CPU, memory, disk, network, and process activity",
        rows,
    }
}

fn overview_section(
    catalog: Option<&AppCatalog>,
    inventory_error: Option<&str>,
    view: &SoftwareView,
    updates: Option<&LiveUpdateCatalog>,
    update_status: &str,
    update_action_status: &str,
) -> TuiSection {
    let mut rows = vec![row_with_preview(
        tui_theme::LABEL_OK,
        "local snapshot ready",
        "safe",
        "The local snapshot is read-only. Nothing has been installed, changed, or started.",
    )];
    match catalog {
        Some(catalog) => {
            let uninstall_reviews = catalog
                .apps
                .iter()
                .filter(|app| {
                    matches!(
                        app.uninstall_option,
                        UninstallOption::ManagerReview | UninstallOption::QuarantineReview
                    )
                })
                .count();
            let visible_count = catalog.apps.iter().filter(|app| view.matches(app)).count();
            let tool_count = toolchain_tools_from_catalog(catalog).len();
            let aiup = aiup_report_from_catalog(catalog);
            let (update_value, update_tone, update_preview) = match updates {
                Some(updates) => (
                    format!(
                        "{} candidates · {}/{} sources ready",
                        updates.candidate_count,
                        updates.source_ok_count,
                        updates.source_count
                    ),
                    if update_action_status.contains("failed") || !updates.warnings.is_empty() {
                        "warn"
                    } else {
                        "accent"
                    },
                    format!(
                        "Provider review is complete: {} candidates from {} sources. {} warnings. {}",
                        updates.candidate_count,
                        updates.source_count,
                        updates.warnings.len(),
                        update_action_status
                    ),
                ),
                None => (
                    if update_status == "not checked" {
                        "updates not reviewed · press u to review".to_string()
                    } else {
                        format!("{update_status} · {update_action_status}")
                    },
                    if update_status == "checking provider availability"
                        || update_action_status.contains("preparing")
                        || update_action_status.contains("executing")
                        || update_action_status.contains("confirm")
                    {
                        "accent"
                    } else {
                        "info"
                    },
                    "Press u to perform a read-only provider review. Review action [U] never skips confirmation or fresh verification.".to_string(),
                ),
            };
            rows.push(row_with_preview(
                tui_theme::LABEL_PLAN,
                &update_value,
                update_tone,
                &update_preview,
            ));
            rows.push(row_with_preview(
                tui_theme::LABEL_INFO,
                &format!(
                    "{visible_count} software · {tool_count} toolchain records · AIUP {}",
                    aiup.orchestrator.state
                ),
                "info",
                &format!(
                    "{} software records match the current view; {} identity groups and {} service/persistence records are available in Software and Diagnostics. Rust-owned AIUP review sees {} AI tools and {} provider-review boundaries; run `rz0 aiup` for the scriptable report.",
                    visible_count,
                    catalog.identity_group_count,
                    catalog.service_count,
                    aiup.tools.len(),
                    aiup
                        .providers
                        .iter()
                        .filter(|provider| provider.state == "provider-review")
                        .count()
                ),
            ));
            if visible_count != catalog.app_count || !view.query().is_empty() {
                rows.push(row_with_preview(
                    tui_theme::LABEL_INFO,
                    &view_description(view),
                    "info",
                    "The list is filtered locally. No provider query or write is implied.",
                ));
            }
            if uninstall_reviews > 0 {
                rows.push(row_with_preview(
                    tui_theme::LABEL_PLAN,
                    &format!("{uninstall_reviews} uninstall reviews available"),
                    "accent",
                    "Software details may offer a manager-owned uninstall review. It is separate from update review and requires its own confirmation.",
                ));
            }
        }
        None if inventory_error.is_some() => rows.push(row(
            tui_theme::LABEL_WARN,
            "local inventory unavailable",
            "warn",
        )),
        None => rows.push(row(
            tui_theme::LABEL_INFO,
            "software names omitted from private output",
            "info",
        )),
    }
    TuiSection {
        code: "01",
        title: "overview",
        summary: "the next safe step from one local snapshot",
        rows,
    }
}

fn installed_software_section(
    catalog: Option<&AppCatalog>,
    inventory_error: Option<&str>,
    view: &SoftwareView,
    updates: Option<&LiveUpdateCatalog>,
    update_plan: Option<&ActionPlan>,
    pending_update: Option<&TuiUpdateChallenge>,
    toolchain_only: bool,
) -> TuiSection {
    let mut rows = Vec::new();
    match catalog {
        Some(catalog) => {
            let mut visible = visible_apps(catalog, view)
                .into_iter()
                .filter(|app| is_toolchain_app(app) == toolchain_only)
                .collect::<Vec<_>>();
            visible.sort_by(|left, right| view.compare(left, right));
            let visible_known_tools = if toolchain_only {
                visible_tool_records(catalog, view)
            } else {
                Vec::new()
            };
            let visible_dynamic = visible_dynamic_updates(catalog, updates, view)
                .into_iter()
                .filter(|update| is_toolchain_update(update) == toolchain_only)
                .collect::<Vec<_>>();
            let visible_count = visible.len() + visible_known_tools.len() + visible_dynamic.len();
            if toolchain_only {
                let aiup = aiup_report_from_catalog(catalog);
                let provider_review_count = aiup
                    .providers
                    .iter()
                    .filter(|provider| provider.state == "provider-review")
                    .count();
                rows.push(row_with_preview(
                    tui_theme::LABEL_OK,
                    &format!(
                        "{visible_count} toolchain records shown · AIUP {}",
                        aiup.orchestrator.state
                    ),
                    "safe",
                    &format!(
                        "Rust-owned AIUP review: {} AI tools, {} provider-review boundaries, {} AIUP-managed records. The review is read-only; use `rz0 aiup` or `rz0 updates` for the next explicit boundary.",
                        aiup.tools.len(),
                        provider_review_count,
                        aiup
                            .tools
                            .iter()
                            .filter(|tool| tool.provider == "aiup")
                            .count()
                    ),
                ));
            } else {
                rows.push(row_count(
                    tui_theme::LABEL_OK,
                    visible_count,
                    "software records shown",
                    "safe",
                ));
            }
            rows.push(TuiRow {
                label: tui_theme::LABEL_INFO,
                value: format!(
                    "{} · Enter details · review the selected action before confirmation",
                    view_description(view)
                ),
                tone: "info",
                preview: None,
            });
            if visible_count == 0 {
                rows.push(row(
                    tui_theme::LABEL_WARN,
                    "no software records match the current search/filter",
                    "warn",
                ));
            }
            for app in visible {
                let label = match app.kind {
                    SoftwareKind::HomebrewFormula
                    | SoftwareKind::HomebrewCask
                    | SoftwareKind::PlatformPackage => "[PKG]",
                    SoftwareKind::ApplicationBundle => "[APP]",
                };
                let (uninstall_options, tone, mut preview) = match app.uninstall_option {
                    UninstallOption::Protected => (
                        "options: details · system protected",
                        "muted",
                        "details only; protected system software has no uninstall option"
                            .to_string(),
                    ),
                    UninstallOption::ManagerReview => (
                        "options: details · manager uninstall review",
                        "accent",
                        format!("run: rz0 uninstall plan {}", app.id),
                    ),
                    UninstallOption::QuarantineReview => (
                        "options: details · quarantine uninstall review",
                        "accent",
                        format!("run: rz0 uninstall plan {}", app.id),
                    ),
                    UninstallOption::Unsupported => (
                        "options: details · uninstall unavailable",
                        "warn",
                        "details only; no safe ownership-specific uninstall option is available"
                            .to_string(),
                    ),
                };
                let mut options = uninstall_options.to_string();
                let provider = toolchain_provider_id(&format!(
                    "{} {} {} {:?}",
                    app.id, app.name, app.source_id, app.identifiers
                ));
                if toolchain_only {
                    let provider_state = if provider == "aiup" {
                        "observed-only"
                    } else {
                        "ready"
                    };
                    options = format!("provider {provider} · {provider_state} · {options}");
                }
                if let Some(update) = updates.and_then(|updates| {
                    updates
                        .candidates
                        .iter()
                        .find(|update| update.software_id == app.id)
                }) {
                    let action = update_action(update_plan, update);
                    let (update_label, update_tone) = update_label_and_tone(action);
                    options = format!(
                        "{} {} · {}",
                        update_label, update.available_version, options
                    );
                    if let Some(action) = action {
                        preview = format!(
                            "{}; manager: {} · target: {} · command: {} · {}",
                            preview,
                            update.manager,
                            action.target,
                            display_action_command(action),
                            display_action_requirements(action)
                        );
                        if let Some(pending) = pending_update
                            .filter(|pending| pending.action.action_id == action.action_id)
                        {
                            preview = format!(
                                "{preview}; confirmation required: type `{}` then Enter · Esc cancels",
                                pending.view.expected_phrase
                            );
                        }
                    } else {
                        preview = format!(
                            "{preview}; manager: {} · target: {}@{} · action plan unavailable",
                            update.manager, update.software_id, update.available_version
                        );
                    }
                    if update_tone == "warn" {
                        // Preserve the existing uninstall tone for the row while making a
                        // blocked update visible in the row text and details.
                        preview = format!("update action is blocked; {preview}");
                    }
                }
                preview = format!(
                    "{preview}; provider: {provider} · source: {} · identity: {} ({})",
                    app.source_id,
                    app.identity_group_id,
                    app.identity_confidence.label()
                );
                rows.push(TuiRow {
                    label,
                    value: format!(
                        "{} · version {} · {}",
                        app.name,
                        app.version.as_deref().unwrap_or("unknown"),
                        options
                    ),
                    tone,
                    preview: Some(preview),
                });
            }
            for tool in visible_known_tools {
                let source_id = tool
                    .source_ids
                    .first()
                    .map(String::as_str)
                    .unwrap_or("known.executables");
                let provider = toolchain_provider_id(&format!(
                    "{} {} {} {:?}",
                    tool.id, tool.display_name, source_id, tool.source_ids
                ));
                rows.push(TuiRow {
                    label: "[OBS]",
                    value: format!(
                        "{} · version {} · provider {} · observed-only",
                        tool.display_name,
                        tool.version.as_deref().unwrap_or("unknown"),
                        provider
                    ),
                    tone: "info",
                    preview: Some(format!(
                        "exact executable evidence · source: {source_id} · confidence: {}; provider action is not established",
                        tool.confidence
                    )),
                });
            }
            for update in visible_dynamic {
                let action = update_action(update_plan, update);
                let (update_label, tone) = update_label_and_tone(action);
                let target_name = update
                    .software_id
                    .rsplit(':')
                    .next()
                    .unwrap_or(update.software_id.as_str());
                let provider =
                    toolchain_provider_id(&format!("{} {}", update.software_id, update.manager));
                let mut preview = format!(
                    "provider: {provider} · manager: {} · target: {} · command: {} · {}",
                    update.manager,
                    action
                        .map(|action| action.target.as_str())
                        .unwrap_or(update.software_id.as_str()),
                    action
                        .map(display_action_command)
                        .unwrap_or_else(|| "exact action unavailable".to_string()),
                    action
                        .map(display_action_requirements)
                        .unwrap_or_else(|| "requirements unavailable".to_string())
                );
                if let Some(action) = action
                    && let Some(pending) = pending_update
                        .filter(|pending| pending.action.action_id == action.action_id)
                {
                    preview = format!(
                        "{preview}; confirmation required: type `{}` then Enter · Esc cancels",
                        pending.view.expected_phrase
                    );
                }
                rows.push(TuiRow {
                    label: "[TOOL]",
                    value: format!(
                        "{target_name} · provider {provider} · version {} -> {} · {}",
                        update.installed_version.as_deref().unwrap_or("unknown"),
                        update.available_version,
                        update_label
                    ),
                    tone,
                    preview: Some(preview),
                });
            }
        }
        None if inventory_error.is_some() => rows.push(row(
            tui_theme::LABEL_WARN,
            "local software inventory failed closed; run `rz0 apps` for details",
            "warn",
        )),
        None => rows.push(row(
            tui_theme::LABEL_INFO,
            "software names omitted from private dashboard output",
            "info",
        )),
    }
    TuiSection {
        code: if toolchain_only { "02" } else { "03" },
        title: if toolchain_only {
            "toolchain"
        } else {
            "software"
        },
        summary: if toolchain_only {
            "Rust-first AI and developer toolchain records by provider"
        } else {
            "installed application and package records outside the toolchain"
        },
        rows,
    }
}

fn is_toolchain_app(app: &InstalledSoftware) -> bool {
    is_toolchain_software(app)
}

fn visible_tool_records<'a>(catalog: &'a AppCatalog, view: &SoftwareView) -> Vec<&'a ToolRecord> {
    let query = view.query().to_ascii_lowercase();
    let mut tools = catalog
        .known_tools
        .iter()
        .filter(|tool| is_toolchain_record(tool))
        .filter(|tool| match view.filter {
            SoftwareFilter::All => true,
            SoftwareFilter::Applications | SoftwareFilter::Reviewable => false,
            SoftwareFilter::PackageManagers => tool.category == "package_manager",
        })
        .filter(|tool| {
            query.is_empty()
                || tool.display_name.to_ascii_lowercase().contains(&query)
                || tool.id.to_ascii_lowercase().contains(&query)
                || tool
                    .source_ids
                    .iter()
                    .any(|source| source.to_ascii_lowercase().contains(&query))
        })
        .collect::<Vec<_>>();
    tools.sort_by(|left, right| {
        left.display_name
            .to_ascii_lowercase()
            .cmp(&right.display_name.to_ascii_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    tools
}

fn is_toolchain_update(update: &SoftwareUpdate) -> bool {
    is_toolchain_text(&format!("{} {}", update.software_id, update.manager))
}

fn visible_apps<'a>(catalog: &'a AppCatalog, view: &SoftwareView) -> Vec<&'a InstalledSoftware> {
    let mut apps = catalog
        .apps
        .iter()
        .filter(|app| view.matches(app))
        .collect::<Vec<_>>();
    apps.sort_by(|left, right| view.compare(left, right));
    apps
}

fn visible_dynamic_updates<'a>(
    catalog: &'a AppCatalog,
    updates: Option<&'a LiveUpdateCatalog>,
    view: &SoftwareView,
) -> Vec<&'a SoftwareUpdate> {
    let Some(updates) = updates else {
        return Vec::new();
    };
    let mut candidates = updates
        .candidates
        .iter()
        .filter(|update| !catalog.apps.iter().any(|app| app.id == update.software_id))
        .filter(|update| update_matches_view(update, view))
        .collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        left.software_id
            .to_ascii_lowercase()
            .cmp(&right.software_id.to_ascii_lowercase())
            .then_with(|| left.available_version.cmp(&right.available_version))
            .then_with(|| left.finding_id.cmp(&right.finding_id))
    });
    candidates
}

fn update_matches_view(update: &SoftwareUpdate, view: &SoftwareView) -> bool {
    if matches!(view.filter, crate::apps::SoftwareFilter::Applications) {
        return false;
    }
    if matches!(view.filter, crate::apps::SoftwareFilter::Reviewable) && update.manager.is_empty() {
        return false;
    }
    let query = view.query().to_ascii_lowercase();
    query.is_empty()
        || update.software_id.to_ascii_lowercase().contains(&query)
        || update.manager.to_ascii_lowercase().contains(&query)
        || update
            .available_version
            .to_ascii_lowercase()
            .contains(&query)
}

fn update_action<'a>(
    update_plan: Option<&'a ActionPlan>,
    update: &SoftwareUpdate,
) -> Option<&'a PlanAction> {
    update_plan?.actions.iter().find(|action| {
        action.finding_id == update.finding_id && action.kind == rz0_action_plan::ActionKind::Update
    })
}

fn update_label_and_tone(action: Option<&PlanAction>) -> (&'static str, &'static str) {
    match action.map(|action| action.disposition) {
        Some(ActionDisposition::Planned) => ("update available · review action", "accent"),
        Some(ActionDisposition::Blocked) => ("update blocked", "warn"),
        Some(ActionDisposition::Unsupported) | None => ("update action unavailable", "warn"),
    }
}

fn display_action_command(action: &PlanAction) -> String {
    let executable = action
        .executable
        .as_deref()
        .unwrap_or("<unresolved-executable>");
    let arguments = action
        .arguments
        .iter()
        .map(|argument| {
            if argument
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-_/.:=@%+".contains(&byte))
            {
                argument.clone()
            } else {
                format!("'{}'", argument.replace('\'', "'\\''"))
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    if arguments.is_empty() {
        executable.to_string()
    } else {
        format!("{executable} {arguments}")
    }
}

fn display_action_requirements(action: &PlanAction) -> String {
    format!(
        "risk {:?} · network {} · elevation {} · rollback {}",
        action.risk,
        if action.network_required { "yes" } else { "no" },
        if action.requires_elevation {
            "yes"
        } else {
            "no"
        },
        if action.rollback.supported {
            "proven"
        } else {
            "manual recovery"
        }
    )
}

fn view_description(view: &SoftwareView) -> String {
    let search = if view.query().is_empty() {
        "search=none".to_string()
    } else {
        format!("search={}", view.query())
    };
    format!(
        "filter={} · sort={} · {search}",
        view.filter.label(),
        view.sort.label()
    )
}

impl TuiDashboard {
    pub fn apply_software_view(&mut self, view: &SoftwareView) {
        self.sections[0] = overview_section(
            self.software_catalog.as_ref(),
            self.inventory_error.as_deref(),
            view,
            self.update_catalog.as_ref(),
            &self.update_check_status,
            &self.update_action_status,
        );
        self.sections[1] = installed_software_section(
            self.software_catalog.as_ref(),
            self.inventory_error.as_deref(),
            view,
            self.update_catalog.as_ref(),
            self.update_plan.as_ref(),
            self.pending_update.as_ref(),
            true,
        );
        self.sections[2] = installed_software_section(
            self.software_catalog.as_ref(),
            self.inventory_error.as_deref(),
            view,
            self.update_catalog.as_ref(),
            self.update_plan.as_ref(),
            self.pending_update.as_ref(),
            false,
        );
    }

    pub fn refresh_monitor(&mut self) {
        let Some(previous) = self.monitor_snapshot.as_ref() else {
            return;
        };
        let snapshot = system_monitor::collect_snapshot(Some(previous));
        self.monitor_snapshot = Some(snapshot.clone());
        if let Some(index) = self
            .sections
            .iter()
            .position(|section| section.code == "04")
        {
            self.sections[index] = system_monitor_section(Some(&snapshot));
        }
    }

    pub fn start_update_check(&mut self) -> Option<AppCatalog> {
        let Some(catalog) = self.software_catalog.as_ref() else {
            self.update_check_status = "unavailable · private summary".to_string();
            return None;
        };
        self.update_catalog = None;
        self.update_plan = None;
        self.pending_update = None;
        self.update_source_count = 0;
        self.update_candidate_count = 0;
        self.update_check_status = "checking provider availability".to_string();
        self.update_action_status =
            "checking provider availability · waiting for results".to_string();
        Some(catalog.clone())
    }

    pub fn complete_update_check(&mut self, updates: LiveUpdateCatalog) {
        if !updates.checked {
            self.fail_update_check();
            return;
        }
        self.update_plan = None;
        self.pending_update = None;
        self.update_source_count = updates.source_count;
        self.update_candidate_count = updates.candidate_count;
        self.update_check_status = format!(
            "checked · {} candidates · {}/{} sources",
            updates.candidate_count, updates.source_ok_count, updates.source_count
        );
        self.update_catalog = Some(updates);
        self.update_action_status =
            "review ready · choose Review action for the selected item".to_string();
    }

    pub(crate) fn complete_update_review(&mut self, review: LiveUpdateReview) {
        self.complete_update_check(review.catalog);
        self.update_plan = review.plan;
    }

    pub fn fail_update_check(&mut self) {
        self.update_catalog = None;
        self.update_plan = None;
        self.pending_update = None;
        self.update_source_count = 0;
        self.update_candidate_count = 0;
        self.update_check_status = "update check failed · press u to retry".to_string();
        self.update_action_status = "update check failed · press u to retry".to_string();
    }

    pub(crate) fn fail_update_check_with_error(&mut self, error: &str) {
        self.update_catalog = None;
        self.update_plan = None;
        self.pending_update = None;
        self.update_source_count = 0;
        self.update_candidate_count = 0;
        let detail = error.lines().next().unwrap_or("provider scan failed");
        self.update_check_status = format!("update check failed · {detail}");
        self.update_action_status = format!("update check failed · {detail}");
    }

    pub(crate) fn selected_software_id(
        &self,
        section_index: usize,
        row_index: usize,
        view: &SoftwareView,
    ) -> Option<String> {
        let toolchain_only = self
            .sections
            .get(section_index)
            .is_some_and(|section| section.code == "02");
        if !self
            .sections
            .get(section_index)
            .is_some_and(|section| matches!(section.code, "02" | "03"))
        {
            return None;
        }
        self.selected_update_candidate(section_index, row_index, view)
            .map(|candidate| candidate.software_id.clone())
            .or_else(|| {
                let catalog = self.software_catalog.as_ref()?;
                let apps = visible_apps(catalog, view)
                    .into_iter()
                    .filter(|app| is_toolchain_app(app) == toolchain_only)
                    .collect::<Vec<_>>();
                let known_tools = if toolchain_only {
                    visible_tool_records(catalog, view)
                } else {
                    Vec::new()
                };
                let row = row_index.checked_sub(2)?;
                if let Some(app) = apps.get(row) {
                    return Some(app.id.clone());
                }
                if row.saturating_sub(apps.len()) < known_tools.len() {
                    return None;
                }
                None
            })
    }

    pub(crate) fn selected_update_action(
        &self,
        section_index: usize,
        row_index: usize,
        view: &SoftwareView,
    ) -> Option<PlanAction> {
        let candidate = self.selected_update_candidate(section_index, row_index, view)?;
        update_action(self.update_plan.as_ref(), candidate).cloned()
    }

    pub(crate) fn has_update_review(&self) -> bool {
        self.update_catalog.is_some()
    }

    pub(crate) fn update_action_for_software_id(&self, software_id: &str) -> Option<PlanAction> {
        let candidate = self
            .update_catalog
            .as_ref()?
            .candidates
            .iter()
            .find(|candidate| candidate.software_id == software_id)?;
        update_action(self.update_plan.as_ref(), candidate).cloned()
    }

    fn selected_update_candidate<'a>(
        &'a self,
        section_index: usize,
        row_index: usize,
        view: &SoftwareView,
    ) -> Option<&'a SoftwareUpdate> {
        let toolchain_only = match self.sections.get(section_index)?.code {
            "02" => true,
            "03" => false,
            _ => return None,
        };
        let catalog = self.software_catalog.as_ref()?;
        let apps = visible_apps(catalog, view)
            .into_iter()
            .filter(|app| is_toolchain_app(app) == toolchain_only)
            .collect::<Vec<_>>();
        let known_tools = if toolchain_only {
            visible_tool_records(catalog, view)
        } else {
            Vec::new()
        };
        let dynamic = visible_dynamic_updates(catalog, self.update_catalog.as_ref(), view)
            .into_iter()
            .filter(|update| is_toolchain_update(update) == toolchain_only)
            .collect::<Vec<_>>();
        let row = row_index.checked_sub(2)?;
        if row < apps.len() {
            let app = apps[row];
            return self
                .update_catalog
                .as_ref()?
                .candidates
                .iter()
                .find(|candidate| candidate.software_id == app.id);
        }
        let after_apps = row.saturating_sub(apps.len());
        if after_apps < known_tools.len() {
            return None;
        }
        dynamic.get(after_apps - known_tools.len()).copied()
    }

    pub(crate) fn start_update_prepare(&mut self, action: &PlanAction) {
        self.pending_update = None;
        self.update_action_status = format!(
            "preparing selected update · {} · {}",
            action.manager.as_deref().unwrap_or("unknown manager"),
            action.target
        );
    }

    pub(crate) fn update_action_unavailable(&mut self, detail: &str) {
        self.pending_update = None;
        self.update_action_status = format!("update unavailable · {detail}");
    }

    pub(crate) fn complete_update_challenge(&mut self, challenge: TuiUpdateChallenge) {
        self.update_action_status = format!(
            "confirm selected update · type the exact phrase and press Enter · Esc cancels · {}",
            challenge.action.target
        );
        self.pending_update = Some(challenge);
    }

    pub(crate) fn pending_update_challenge(&self) -> Option<&TuiUpdateChallenge> {
        self.pending_update.as_ref()
    }

    pub(crate) fn begin_update_execution(&mut self) {
        let target = self
            .pending_update
            .as_ref()
            .map(|pending| pending.action.target.clone())
            .unwrap_or_else(|| "selected item".to_string());
        self.update_action_status = format!("executing update · {target}");
    }

    pub(crate) fn complete_update_execution(&mut self, report: &UpdateExecutionReport) {
        self.read_only = false;
        self.writes_attempted |= report.writes_attempted;
        self.pending_update = None;
        self.update_plan = None;
        self.update_catalog = None;
        self.update_candidate_count = 0;
        self.update_check_status = "stale · press u to rescan providers".to_string();
        self.update_action_status = format!(
            "updated · {} · receipt {}",
            report.target, report.receipt_reference
        );
    }

    pub(crate) fn fail_update_action(&mut self, error: &str) {
        let detail = error.lines().next().unwrap_or("update execution failed");
        self.pending_update = None;
        self.update_action_status = format!("update failed · {detail}");
    }

    pub(crate) fn cancel_update_action(&mut self) {
        self.update_action_status = "cancelling update · waiting for manager boundary".to_string();
    }

    pub fn check_updates(&mut self) {
        let Some(catalog) = self.start_update_check() else {
            return;
        };
        match collect_live_update_review(&catalog) {
            Ok(review) => self.complete_update_review(review),
            Err(error) => self.fail_update_check_with_error(&error),
        }
    }
}

fn palette() -> TuiPalette {
    TuiPalette {
        surface_bg: tui_theme::SURFACE_BG,
        panel_bg: tui_theme::PANEL_BG,
        brand_accent: tui_theme::BRAND_ACCENT,
        text_primary: tui_theme::TEXT_PRIMARY,
        text_muted: tui_theme::TEXT_MUTED,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_cancelled_dashboard_load_never_publishes_a_snapshot() {
        let (controller, cancellation) = rz0_cancellation_contract::cancellation_pair();
        controller.cancel(rz0_cancellation_contract::CancellationReason::UserRequested);

        let error = dashboard_cancellable(&cancellation)
            .expect_err("pre-cancelled dashboard load should stop before publishing");
        assert!(error.contains("dashboard load cancelled"));
    }
}
