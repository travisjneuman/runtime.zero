use serde::Serialize;

use crate::brand;
use crate::module_registry::ModuleRegistryReport;
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
    let modules = ModuleRegistryReport::empty_installed();
    build_dashboard(&store, &modules)
}

fn build_dashboard(store: &StoreStatusReport, modules: &ModuleRegistryReport) -> TuiDashboard {
    TuiDashboard {
        title: brand::TITLE,
        command: brand::COMMAND,
        version: env!("CARGO_PKG_VERSION"),
        mode: "safe review dashboard",
        safety_posture: brand::SAFETY_POSTURE,
        store_state: store.overall_state,
        installed_module_count: modules.summary.installed_module_count,
        planned_module_family_count: modules.summary.planned_family_count,
        sections: sections(store, modules),
        palette: palette(),
    }
}

fn sections(store: &StoreStatusReport, modules: &ModuleRegistryReport) -> Vec<TuiSection> {
    vec![
        TuiSection {
            title: "foundation",
            rows: vec![
                row(tui_theme::LABEL_OK, "core CLI loaded", "safe"),
                row(tui_theme::LABEL_INFO, brand::SAFETY_POSTURE, "info"),
                row(
                    tui_theme::LABEL_SKIP,
                    "mutation capability disabled",
                    "muted",
                ),
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
                    tui_theme::LABEL_SKIP,
                    "no store writes or initialization",
                    "muted",
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
    fn dashboard_does_not_claim_active_feature_modules() {
        let dashboard = dashboard();
        assert_eq!(dashboard.installed_module_count, 0);
        assert!(dashboard.planned_module_family_count > 0);
    }
}
