use std::fs;
use std::path::Path;

use serde::Serialize;

use crate::apps::{
    AppCatalog, InstalledSoftware, SoftwareKind, SoftwareView, UninstallOption, collect_app_catalog,
};
use crate::brand;
use crate::install_receipt::ReceiptInventoryState;
use crate::installed_registry::InstalledRegistryState;
use crate::module_registry::ModuleRegistryReport;
use crate::store_init::{StoreInitMode, StoreInitOptions, StoreInitStatus, store_init_report};
use crate::store_status::{StoreOverallState, StoreStatusReport, store_status_report};
use crate::tui_dashboard_labels::{
    init_label, init_status_label, init_tone, receipt_label, receipt_state_label, receipt_tone,
    registry_label, registry_state_label, registry_tone, row, row_count, store_state_label,
};
use crate::tui_theme;
use crate::updates::{LiveUpdateCatalog, collect_live_update_catalog};

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
    pub inventory_status: String,
    pub update_check_status: String,
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

fn dashboard_with_inventory(include_software_names: bool) -> TuiDashboard {
    let store = store_status_report(&["tui".to_string()]);
    let init = store_init_report(
        &["tui".to_string()],
        StoreInitOptions::new(StoreInitMode::DryRun),
    );
    let modules = ModuleRegistryReport::empty_installed();
    let inventory = include_software_names.then(collect_app_catalog);
    build_dashboard(&store, init.status, &modules, inventory)
}

fn build_dashboard(
    store: &StoreStatusReport,
    init_status: StoreInitStatus,
    modules: &ModuleRegistryReport,
    inventory: Option<Result<AppCatalog, String>>,
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
        inventory_status,
        update_check_status: "not checked".to_string(),
        update_source_count: 0,
        update_candidate_count: 0,
        sections: sections(
            store,
            init_status,
            modules,
            catalog.as_ref(),
            inventory_error.as_deref(),
            &default_view,
            None,
        ),
        palette: palette(),
        software_catalog: catalog,
        inventory_error,
        update_catalog: None,
    }
}

fn sections(
    store: &StoreStatusReport,
    init_status: StoreInitStatus,
    modules: &ModuleRegistryReport,
    catalog: Option<&AppCatalog>,
    inventory_error: Option<&str>,
    view: &SoftwareView,
    updates: Option<&LiveUpdateCatalog>,
) -> Vec<TuiSection> {
    vec![
        overview_section(catalog, inventory_error, view, updates),
        TuiSection {
            code: "02",
            title: "local store",
            summary: "user-local store and registry health",
            rows: vec![
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
            ],
        },
        installed_software_section(catalog, inventory_error, view, updates),
        TuiSection {
            code: "04",
            title: "modules",
            summary: "first-party module source and lifecycle state",
            rows: vec![
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
                    "module package checks are available from the CLI",
                    "info",
                ),
            ],
        },
        TuiSection {
            code: "05",
            title: "actions",
            summary: "available actions and permissions",
            rows: vec![
                row(
                    tui_theme::LABEL_OK,
                    "software list and update checks are available",
                    "safe",
                ),
                row(
                    tui_theme::LABEL_DRY_RUN,
                    "initialize local state with `rz0 store init --yes`",
                    "info",
                ),
                row(
                    tui_theme::LABEL_INFO,
                    "module package checks are available from the CLI",
                    "info",
                ),
                row(
                    tui_theme::LABEL_SKIP,
                    "updates use the selected manager and confirmation",
                    "info",
                ),
            ],
        },
    ]
}

fn overview_section(
    catalog: Option<&AppCatalog>,
    inventory_error: Option<&str>,
    view: &SoftwareView,
    updates: Option<&LiveUpdateCatalog>,
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
            match updates {
                Some(updates) => {
                    rows.push(row_count(
                        tui_theme::LABEL_PLAN,
                        updates.candidate_count,
                        "update candidates found",
                        "accent",
                    ));
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
                None => rows.push(row(
                    tui_theme::LABEL_INFO,
                    "updates not checked · press u to check",
                    "info",
                )),
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
        "Tab details · Enter details · u checks updates · r refresh",
        "info",
    ));
    TuiSection {
        code: "01",
        title: "overview",
        summary: "live local inventory and available safe actions",
        rows,
    }
}

fn installed_software_section(
    catalog: Option<&AppCatalog>,
    inventory_error: Option<&str>,
    view: &SoftwareView,
    updates: Option<&LiveUpdateCatalog>,
) -> TuiSection {
    let mut rows = Vec::new();
    match catalog {
        Some(catalog) => {
            let mut visible = catalog
                .apps
                .iter()
                .filter(|app| view.matches(app))
                .collect::<Vec<&InstalledSoftware>>();
            visible.sort_by(|left, right| view.compare(left, right));
            rows.push(row_count(
                tui_theme::LABEL_OK,
                visible.len(),
                "software records shown",
                "safe",
            ));
            rows.push(TuiRow {
                label: tui_theme::LABEL_INFO,
                value: format!(
                    "{} · select an item and press Enter for details; u checks updates",
                    view_description(view)
                ),
                tone: "info",
                preview: None,
            });
            if visible.is_empty() {
                rows.push(row(
                    tui_theme::LABEL_WARN,
                    "no software records match the current search/filter",
                    "warn",
                ));
            }
            for app in visible {
                let label = match app.kind {
                    SoftwareKind::HomebrewFormula | SoftwareKind::HomebrewCask => "[PKG]",
                    SoftwareKind::ApplicationBundle | SoftwareKind::PlatformPackage => "[APP]",
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
                if let Some(update) = updates.and_then(|updates| {
                    updates
                        .candidates
                        .iter()
                        .find(|update| update.software_id == app.id)
                }) {
                    options = format!(
                        "update available: {} · {}",
                        update.available_version, options
                    );
                    let apply_command = update_apply_command(app.kind);
                    preview = match apply_command {
                        Some(command) => format!(
                            "update: {} {} -> {}; command: {}; {}",
                            update.manager,
                            update.installed_version.as_deref().unwrap_or("unknown"),
                            update.available_version,
                            command,
                            preview
                        ),
                        None => format!(
                            "update: {} {} -> {}; use the matching `rz0 updates --apply` command; {}",
                            update.manager,
                            update.installed_version.as_deref().unwrap_or("unknown"),
                            update.available_version,
                            preview
                        ),
                    };
                }
                preview = format!(
                    "{preview}; source: {} · identity: {} ({})",
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
        code: "03",
        title: "installed software",
        summary: "live bounded applications and package-manager records",
        rows,
    }
}

fn update_apply_command(kind: SoftwareKind) -> Option<String> {
    let manager_id = match kind {
        SoftwareKind::HomebrewFormula => "homebrew-formula",
        SoftwareKind::HomebrewCask => "homebrew-cask",
        SoftwareKind::ApplicationBundle | SoftwareKind::PlatformPackage => return None,
    };
    let executable = ["/opt/homebrew/bin/brew", "/usr/local/bin/brew"]
        .iter()
        .map(Path::new)
        .find(|path| {
            fs::symlink_metadata(path)
                .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
        })?;
    Some(format!(
        "rz0 updates --apply --probe --manager {manager_id} --executable {} --allow-network-read --allow-network-write --all --accept-no-rollback",
        executable.display()
    ))
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
        );
        self.sections[2] = installed_software_section(
            self.software_catalog.as_ref(),
            self.inventory_error.as_deref(),
            view,
            self.update_catalog.as_ref(),
        );
    }

    pub fn check_updates(&mut self) {
        let Some(catalog) = self.software_catalog.as_ref() else {
            self.update_check_status = "unavailable · private summary".to_string();
            return;
        };
        let updates = collect_live_update_catalog(catalog);
        self.update_source_count = updates.source_count;
        self.update_candidate_count = updates.candidate_count;
        self.update_check_status = format!(
            "checked · {} candidates · {}/{} sources",
            updates.candidate_count, updates.source_ok_count, updates.source_count
        );
        self.update_catalog = Some(updates);
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
