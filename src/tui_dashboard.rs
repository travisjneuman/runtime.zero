use rz0_action_plan::{ActionDisposition, ActionPlan, PlanAction};
use serde::Serialize;

use crate::apps::{
    AppCatalog, InstalledSoftware, SoftwareKind, SoftwareUpdate, SoftwareView, UninstallOption,
    collect_app_catalog,
};
use crate::brand;
use crate::cache::{self, CacheReviewReport};
use crate::install_receipt::ReceiptInventoryState;
use crate::installed_registry::InstalledRegistryState;
use crate::leftovers::{self, LeftoversReviewReport};
use crate::module_registry::ModuleRegistryReport;
use crate::store_init::{StoreInitMode, StoreInitOptions, StoreInitStatus, store_init_report};
use crate::store_status::{StoreOverallState, StoreStatusReport, store_status_report};
use crate::system_monitor::{self, SystemSnapshot};
use crate::toolchain::{is_toolchain_software, is_toolchain_text, toolchain_provider_id};
use crate::tui_dashboard_labels::{
    init_label, init_status_label, init_tone, receipt_label, receipt_state_label, receipt_tone,
    registry_label, registry_state_label, registry_tone, row, row_count, store_state_label,
};
use crate::tui_theme;
use crate::update_cli::TuiUpdateChallenge;
use crate::update_execution::UpdateExecutionReport;
use crate::updates::{LiveUpdateCatalog, LiveUpdateReview, collect_live_update_review};

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
    pub store_state: StoreOverallState,
    pub registry_state: InstalledRegistryState,
    pub receipt_state: ReceiptInventoryState,
    pub store_init_status: StoreInitStatus,
    pub installed_module_count: usize,
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

pub fn dashboard() -> TuiDashboard {
    dashboard_with_inventory(true)
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
    let inventory = include_software_names.then(collect_app_catalog);
    let monitor = include_software_names.then(|| system_monitor::collect_snapshot(None));
    let cache = include_software_names.then(cache::live_report);
    let leftovers = include_software_names.then(leftovers::live_report);
    build_dashboard(
        &store,
        init.status,
        &modules,
        inventory,
        monitor,
        cache,
        leftovers,
    )
}

fn build_dashboard(
    store: &StoreStatusReport,
    init_status: StoreInitStatus,
    modules: &ModuleRegistryReport,
    inventory: Option<Result<AppCatalog, String>>,
    monitor: Option<SystemSnapshot>,
    cache: Option<Result<CacheReviewReport, String>>,
    leftovers: Option<Result<LeftoversReviewReport, String>>,
) -> TuiDashboard {
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
        Some(Ok(report)) => format!(
            "live · {} bounded observations",
            report.finding_report.summary.finding_count
        ),
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
        store_state: store.overall_state,
        registry_state: store.registry.status,
        receipt_state: store.receipts.overall_state,
        store_init_status: init_status,
        installed_module_count: store.registry.installed_module_count,
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
        update_check_status: "not checked".to_string(),
        update_action_status: "idle · u scans providers · review action requires confirmation"
            .to_string(),
        update_source_count: 0,
        update_candidate_count: 0,
        sections: sections(SectionContext {
            store,
            init_status,
            modules,
            catalog: catalog.as_ref(),
            inventory_error: inventory_error.as_deref(),
            cache: cache_value.as_ref(),
            leftovers: leftovers_value.as_ref(),
            view: &default_view,
            updates: None,
            update_plan: None,
            pending_update: None,
            update_status: "not checked",
            update_action_status: "idle · u scans providers · review action requires confirmation",
            monitor: monitor.as_ref(),
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
    catalog: Option<&'a AppCatalog>,
    inventory_error: Option<&'a str>,
    cache: Option<&'a CacheReviewReport>,
    leftovers: Option<&'a LeftoversReviewReport>,
    view: &'a SoftwareView,
    updates: Option<&'a LiveUpdateCatalog>,
    update_plan: Option<&'a ActionPlan>,
    pending_update: Option<&'a TuiUpdateChallenge>,
    update_status: &'a str,
    update_action_status: &'a str,
    monitor: Option<&'a SystemSnapshot>,
}

fn sections(context: SectionContext<'_>) -> Vec<TuiSection> {
    let SectionContext {
        store,
        init_status,
        modules,
        catalog,
        inventory_error,
        cache,
        leftovers,
        view,
        updates,
        update_plan,
        pending_update,
        update_status,
        update_action_status,
        monitor,
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
            inventory_error,
            update_action_status,
            cache,
            leftovers,
        ),
    ];
    sections.insert(3, system_monitor_section(monitor));
    sections
}

fn diagnostics_section(
    store: &StoreStatusReport,
    init_status: StoreInitStatus,
    modules: &ModuleRegistryReport,
    inventory_error: Option<&str>,
    update_action_status: &str,
    cache: Option<&CacheReviewReport>,
    leftovers: Option<&LeftoversReviewReport>,
) -> TuiSection {
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
            modules.summary.installed_module_count,
            "installed modules",
            "info",
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
            "module package checks and store status are available from the CLI",
            "info",
        ),
        match (cache, leftovers) {
            (Some(cache), Some(leftovers)) => row(
                if cache.warnings.is_empty() && leftovers.warnings.is_empty() {
                    tui_theme::LABEL_INFO
                } else {
                    tui_theme::LABEL_WARN
                },
                &format!(
                    "bounded evidence · cache {} · leftovers {}",
                    cache.finding_report.summary.finding_count,
                    leftovers.finding_report.summary.finding_count
                ),
                if cache.warnings.is_empty() && leftovers.warnings.is_empty() {
                    "info"
                } else {
                    "warn"
                },
            ),
            _ => row(
                tui_theme::LABEL_WARN,
                "bounded cache/leftovers evidence unavailable",
                "warn",
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
    let mut rows = vec![
        row(
            tui_theme::LABEL_OK,
            &format!(
                "CPU {} · load {} · {} logical CPUs",
                snapshot
                    .cpu
                    .usage_percent
                    .map(|value| format!("{value}%"))
                    .unwrap_or_else(|| "sampling".to_string()),
                snapshot
                    .cpu
                    .load_average_milli
                    .iter()
                    .map(|value| {
                        value
                            .map(|value| format!("{}.{:03}", value / 1000, value % 1000))
                            .unwrap_or_else(|| "?".to_string())
                    })
                    .collect::<Vec<_>>()
                    .join(" "),
                snapshot.cpu.logical_cpus.unwrap_or(0)
            ),
            "safe",
        ),
        row(
            tui_theme::LABEL_INFO,
            &format!(
                "memory {} used · {} available of {}",
                system_monitor::format_bytes(snapshot.memory.used_bytes),
                system_monitor::format_bytes(snapshot.memory.available_bytes),
                system_monitor::format_bytes(snapshot.memory.total_bytes)
            ),
            "info",
        ),
        row(
            tui_theme::LABEL_INFO,
            &format!(
                "network {} interfaces · received {} · sent {}",
                snapshot.network.interface_count,
                system_monitor::format_bytes(snapshot.network.received_bytes),
                system_monitor::format_bytes(snapshot.network.transmitted_bytes)
            ),
            "info",
        ),
        row(
            tui_theme::LABEL_INFO,
            &format!(
                "processes {} total · {} running",
                snapshot.processes.total, snapshot.processes.running
            ),
            "info",
        ),
    ];
    for disk in &snapshot.disks {
        rows.push(row(
            tui_theme::LABEL_INFO,
            &format!(
                "disk {} · {} used · {} available",
                disk.mount,
                system_monitor::format_bytes(disk.used_bytes),
                system_monitor::format_bytes(disk.available_bytes)
            ),
            "info",
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
    let mut rows = vec![row(
        tui_theme::LABEL_OK,
        "local control surface loaded",
        "safe",
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
            match updates {
                Some(updates) => rows.push(TuiRow {
                    label: tui_theme::LABEL_PLAN,
                    value: format!("{update_status} · {update_action_status}"),
                    tone: if update_action_status.contains("failed") || !updates.warnings.is_empty()
                    {
                        "warn"
                    } else {
                        "accent"
                    },
                    preview: None,
                }),
                None => rows.push(TuiRow {
                    label: tui_theme::LABEL_PLAN,
                    value: if update_status == "not checked" {
                        format!("updates not checked · {update_action_status}")
                    } else {
                        format!("{update_status} · {update_action_status}")
                    },
                    tone: if update_status == "checking provider availability"
                        || update_action_status.contains("preparing")
                        || update_action_status.contains("executing")
                        || update_action_status.contains("confirm")
                    {
                        "accent"
                    } else {
                        "info"
                    },
                    preview: None,
                }),
            }
            rows.push(row_count(
                tui_theme::LABEL_OK,
                visible_count,
                "software records shown",
                "safe",
            ));
            if visible_count != catalog.app_count || !view.query().is_empty() {
                rows.push(TuiRow {
                    label: tui_theme::LABEL_INFO,
                    value: view_description(view),
                    tone: "info",
                    preview: None,
                });
            }
            rows.push(row_count(
                tui_theme::LABEL_INFO,
                catalog.identity_group_count,
                "identity groups",
                "info",
            ));
            rows.push(row_count(
                tui_theme::LABEL_INFO,
                catalog.service_count,
                "service/persistence records",
                "info",
            ));
            if let Some(updates) = updates {
                rows.push(TuiRow {
                    label: tui_theme::LABEL_INFO,
                    value: format!(
                        "update sources: {}/{} ready · warnings: {}",
                        updates.source_ok_count,
                        updates.source_count,
                        updates.warnings.len()
                    ),
                    tone: if updates.warnings.is_empty() {
                        "info"
                    } else {
                        "warn"
                    },
                    preview: None,
                });
            }
            rows.push(row_count(
                tui_theme::LABEL_PLAN,
                uninstall_reviews,
                "uninstall reviews available",
                "accent",
            ));
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
    rows.push(row(
        tui_theme::LABEL_INFO,
        "Tab focus · Enter details · u scan providers · review action · r refresh",
        "info",
    ));
    TuiSection {
        code: "01",
        title: "overview",
        summary: "live local inventory and provider-backed update actions",
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
            let visible_dynamic = visible_dynamic_updates(catalog, updates, view)
                .into_iter()
                .filter(|update| is_toolchain_update(update) == toolchain_only)
                .collect::<Vec<_>>();
            let visible_count = visible.len() + visible_dynamic.len();
            rows.push(row_count(
                tui_theme::LABEL_OK,
                visible_count,
                "software records shown",
                "safe",
            ));
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
                    options = format!("provider {provider} · {options}");
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
                let row = row_index.checked_sub(2)?;
                apps.get(row).map(|app| app.id.clone())
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
        dynamic.get(row.saturating_sub(apps.len())).copied()
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
