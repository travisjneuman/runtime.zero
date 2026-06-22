use std::fmt::Write as FmtWrite;

use crate::brand;
use crate::launch_routing::LaunchMode;
use crate::store_status::{
    StoreOverallState, StorePathKind, StorePathRole, StorePathState, StoreStatusReport,
};

pub fn store_status_text(report: &StoreStatusReport) -> String {
    let mut out = format!("{} store status\n\n", brand::TITLE);
    let _ = writeln!(out, "mode: read-only");
    let _ = writeln!(out, "writes_attempted: no");
    let _ = writeln!(out, "store_schema_version: {}", report.store_schema_version);
    let _ = writeln!(
        out,
        "overall_state: {}",
        overall_state_label(report.overall_state)
    );
    let _ = writeln!(out, "paths:");
    for path in &report.paths {
        let _ = writeln!(
            out,
            "  - {}: {} {} ({})",
            path_role_label(path.role),
            path_state_label(path.state),
            path_kind_label(path.expected_kind),
            path.path
        );
        let _ = writeln!(out, "    detail: {}", path.detail);
    }
    let _ = writeln!(
        out,
        "launch_mode: {}",
        launch_mode_label(report.launch_context.launch_mode)
    );
    let _ = writeln!(
        out,
        "launch_json_requested: {}",
        report.launch_context.json_requested
    );
    let _ = writeln!(out, "safety: {}", report.safety_note);
    out
}

fn overall_state_label(state: StoreOverallState) -> &'static str {
    match state {
        StoreOverallState::NotInitialized => "not_initialized",
        StoreOverallState::Empty => "empty",
        StoreOverallState::Present => "present",
        StoreOverallState::Invalid => "invalid",
    }
}

fn path_role_label(role: StorePathRole) -> &'static str {
    match role {
        StorePathRole::DataRoot => "data_root",
        StorePathRole::StateRoot => "state_root",
        StorePathRole::CacheRoot => "cache_root",
        StorePathRole::LogRoot => "log_root",
        StorePathRole::QuarantineRoot => "quarantine_root",
        StorePathRole::ModulesRoot => "modules_root",
        StorePathRole::RegistryPath => "registry_path",
        StorePathRole::TransactionsDir => "transactions_dir",
        StorePathRole::ReceiptsDir => "receipts_dir",
    }
}

fn path_kind_label(kind: StorePathKind) -> &'static str {
    match kind {
        StorePathKind::Directory => "directory",
        StorePathKind::File => "file",
    }
}

fn path_state_label(state: StorePathState) -> &'static str {
    match state {
        StorePathState::Absent => "absent",
        StorePathState::Empty => "empty",
        StorePathState::Present => "present",
        StorePathState::Invalid => "invalid",
    }
}

fn launch_mode_label(mode: LaunchMode) -> &'static str {
    match mode {
        LaunchMode::TuiDashboard => "tui_dashboard",
        LaunchMode::CliDashboardText => "cli_dashboard_text",
        LaunchMode::CliDashboardJson => "cli_dashboard_json",
        LaunchMode::CliSubcommand => "cli_subcommand",
        LaunchMode::TuiUnavailable => "tui_unavailable",
    }
}
