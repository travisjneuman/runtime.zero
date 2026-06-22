use serde::Serialize;

use crate::brand;
use crate::install_receipt::ReceiptInventoryState;
use crate::installed_registry::InstalledRegistryState;
use crate::module_registry::ModuleRegistryReport;
use crate::store_init::{StoreInitMode, StoreInitOptions, StoreInitStatus, store_init_report};
use crate::store_status::{StoreOverallState, StoreStatusReport, store_status_report};
use crate::tui_theme;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TuiDashboard {
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
    pub sections: Vec<TuiSection>,
    pub palette: TuiPalette,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TuiSection {
    pub title: &'static str,
    pub rows: Vec<TuiRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TuiRow {
    pub label: &'static str,
    pub value: String,
    pub tone: &'static str,
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
    let store = store_status_report(&["tui".to_string()]);
    let init = store_init_report(
        &["tui".to_string()],
        StoreInitOptions::new(StoreInitMode::DryRun),
    );
    let modules = ModuleRegistryReport::empty_installed();
    build_dashboard(&store, init.status, &modules)
}

fn build_dashboard(
    store: &StoreStatusReport,
    init_status: StoreInitStatus,
    modules: &ModuleRegistryReport,
) -> TuiDashboard {
    TuiDashboard {
        title: brand::TITLE,
        command: brand::COMMAND,
        version: env!("CARGO_PKG_VERSION"),
        mode: "safe review dashboard",
        safety_posture: brand::SAFETY_POSTURE,
        store_state: store.overall_state,
        registry_state: store.registry.status,
        receipt_state: store.receipts.overall_state,
        store_init_status: init_status,
        installed_module_count: store.registry.installed_module_count,
        planned_module_family_count: modules.summary.planned_family_count,
        sections: sections(store, init_status, modules),
        palette: palette(),
    }
}

fn sections(
    store: &StoreStatusReport,
    init_status: StoreInitStatus,
    modules: &ModuleRegistryReport,
) -> Vec<TuiSection> {
    vec![
        TuiSection {
            title: "foundation",
            rows: vec![
                row(tui_theme::LABEL_OK, "core CLI loaded", "safe"),
                row(tui_theme::LABEL_INFO, brand::SAFETY_POSTURE, "info"),
                row(tui_theme::LABEL_SKIP, "module mutation disabled", "muted"),
            ],
        },
        TuiSection {
            title: "local store",
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
        TuiSection {
            title: "modules",
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
                    tui_theme::LABEL_DRY_RUN,
                    "install planner remains dry-run only",
                    "dry_run",
                ),
            ],
        },
        TuiSection {
            title: "safety gates",
            rows: vec![
                row(
                    tui_theme::LABEL_OK,
                    "TUI is read-only review surface",
                    "safe",
                ),
                row(
                    tui_theme::LABEL_DRY_RUN,
                    "store init stays explicit",
                    "dry_run",
                ),
                row(tui_theme::LABEL_SKIP, "module execution blocked", "muted"),
                row(
                    tui_theme::LABEL_SKIP,
                    "remote fetch and trust blocked",
                    "muted",
                ),
            ],
        },
    ]
}

fn row(label: &'static str, value: &str, tone: &'static str) -> TuiRow {
    TuiRow {
        label,
        value: value.to_string(),
        tone,
    }
}

fn row_count(label: &'static str, count: usize, suffix: &str, tone: &'static str) -> TuiRow {
    row(label, &format!("{count} {suffix}"), tone)
}

fn store_state_label(state: StoreOverallState) -> &'static str {
    match state {
        StoreOverallState::NotInitialized => "store not initialized",
        StoreOverallState::Empty => "store paths exist but are empty",
        StoreOverallState::Present => "store paths present",
        StoreOverallState::Invalid => "store path mismatch detected",
    }
}

fn init_status_label(status: StoreInitStatus) -> &'static str {
    match status {
        StoreInitStatus::Ready => "store init dry-run ready",
        StoreInitStatus::AlreadyInitialized => "store scaffolding initialized",
        StoreInitStatus::Applied => "store init applied",
        StoreInitStatus::Blocked => "store init blocked",
    }
}

fn init_label(status: StoreInitStatus) -> &'static str {
    match status {
        StoreInitStatus::Blocked => tui_theme::LABEL_WARN,
        StoreInitStatus::AlreadyInitialized | StoreInitStatus::Applied => tui_theme::LABEL_OK,
        StoreInitStatus::Ready => tui_theme::LABEL_DRY_RUN,
    }
}

fn init_tone(status: StoreInitStatus) -> &'static str {
    match status {
        StoreInitStatus::Blocked => "warn",
        StoreInitStatus::AlreadyInitialized | StoreInitStatus::Applied => "safe",
        StoreInitStatus::Ready => "dry_run",
    }
}

fn registry_state_label(state: InstalledRegistryState) -> &'static str {
    match state {
        InstalledRegistryState::Absent => "registry absent",
        InstalledRegistryState::Empty => "registry file empty",
        InstalledRegistryState::Valid => "registry valid",
        InstalledRegistryState::Invalid => "registry invalid",
        InstalledRegistryState::Unreadable => "registry unreadable",
    }
}

fn registry_label(state: InstalledRegistryState) -> &'static str {
    match state {
        InstalledRegistryState::Invalid | InstalledRegistryState::Unreadable => {
            tui_theme::LABEL_WARN
        }
        InstalledRegistryState::Valid => tui_theme::LABEL_OK,
        InstalledRegistryState::Absent | InstalledRegistryState::Empty => tui_theme::LABEL_INFO,
    }
}

fn registry_tone(state: InstalledRegistryState) -> &'static str {
    match state {
        InstalledRegistryState::Invalid | InstalledRegistryState::Unreadable => "warn",
        InstalledRegistryState::Valid => "safe",
        InstalledRegistryState::Absent | InstalledRegistryState::Empty => "info",
    }
}

fn receipt_state_label(state: ReceiptInventoryState) -> &'static str {
    match state {
        ReceiptInventoryState::NotReferenced => "receipts not referenced",
        ReceiptInventoryState::Absent => "receipt missing",
        ReceiptInventoryState::Valid => "receipts valid",
        ReceiptInventoryState::Invalid => "receipt invalid",
        ReceiptInventoryState::Unreadable => "receipt unreadable",
        ReceiptInventoryState::UnsupportedSchema => "receipt unsupported schema",
    }
}

fn receipt_label(state: ReceiptInventoryState) -> &'static str {
    match state {
        ReceiptInventoryState::Invalid
        | ReceiptInventoryState::Unreadable
        | ReceiptInventoryState::UnsupportedSchema
        | ReceiptInventoryState::Absent => tui_theme::LABEL_WARN,
        ReceiptInventoryState::Valid => tui_theme::LABEL_OK,
        ReceiptInventoryState::NotReferenced => tui_theme::LABEL_INFO,
    }
}

fn receipt_tone(state: ReceiptInventoryState) -> &'static str {
    match state {
        ReceiptInventoryState::Invalid
        | ReceiptInventoryState::Unreadable
        | ReceiptInventoryState::UnsupportedSchema
        | ReceiptInventoryState::Absent => "warn",
        ReceiptInventoryState::Valid => "safe",
        ReceiptInventoryState::NotReferenced => "info",
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
