use std::fmt::Write as FmtWrite;

use serde::Serialize;

use crate::brand;
use crate::launch_routing::{
    InterfaceMode, LaunchEnvironment, LaunchMode, LaunchRoutingReport, resolve_launch_mode,
};
use crate::module_store::{
    ForbiddenPathClass, ModuleStorePlan, STORE_SCHEMA_VERSION, module_store_plan,
};

const EXAMPLE_MODULE_ID: &str = "first-party.example";
const EXAMPLE_MODULE_VERSION: &str = "0.0.0";
const STORE_PLAN_SEED: &str = "store plan";
const SAFETY_NOTE: &str =
    "Read-only store contract inspection; no directories or state files were created.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StorePlanReport {
    pub command: &'static str,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub store: ModuleStorePlan,
    pub launch_context: LaunchRoutingReport,
    pub safety_note: &'static str,
}

pub fn store_plan_report(args: &[String]) -> StorePlanReport {
    StorePlanReport {
        command: "store plan",
        read_only: true,
        writes_attempted: false,
        store: module_store_plan(
            Some(EXAMPLE_MODULE_ID),
            Some(EXAMPLE_MODULE_VERSION),
            STORE_PLAN_SEED,
        ),
        launch_context: resolve_launch_mode(args, LaunchEnvironment::cli_subcommand()),
        safety_note: SAFETY_NOTE,
    }
}

pub fn store_plan_text(report: &StorePlanReport) -> String {
    let store = &report.store;
    let mut out = format!("{} store plan\n\n", brand::TITLE);
    let _ = writeln!(out, "mode: read-only");
    let _ = writeln!(out, "writes_attempted: no");
    let _ = writeln!(out, "store_schema_version: {}", STORE_SCHEMA_VERSION);
    let _ = writeln!(out, "plan_id: {}", store.plan_id);
    let _ = writeln!(out, "data_root: {}", store.data_root);
    let _ = writeln!(out, "state_root: {}", store.state_root);
    let _ = writeln!(out, "cache_root: {}", store.cache_root);
    let _ = writeln!(out, "log_root: {}", store.log_root);
    let _ = writeln!(out, "quarantine_root: {}", store.quarantine_root);
    let _ = writeln!(out, "modules_root: {}", store.modules_root);
    let _ = writeln!(out, "registry_path: {}", store.registry_path);
    let _ = writeln!(out, "transaction_path: {}", store.transaction_path);
    write_optional_path(&mut out, "example_module_dir", &store.module_dir);
    write_optional_path(&mut out, "example_receipt_path", &store.receipt_path);
    write_optional_path(
        &mut out,
        "example_rollback_plan_path",
        &store.rollback_plan_path,
    );
    write_optional_path(
        &mut out,
        "example_quarantine_record_path",
        &store.quarantine_record_path,
    );
    let _ = writeln!(out, "rollback_supported: {}", store.rollback_supported);
    let _ = writeln!(out, "quarantine_supported: {}", store.quarantine_supported);
    let _ = writeln!(out, "forbidden_path_classes:");
    for class in &store.forbidden_path_classes {
        let _ = writeln!(out, "  - {}", forbidden_path_label(*class));
    }
    let _ = writeln!(
        out,
        "launch_mode: {}",
        launch_mode_label(report.launch_context.launch_mode)
    );
    let _ = writeln!(
        out,
        "interface: {}",
        interface_label(report.launch_context.interface)
    );
    let _ = writeln!(
        out,
        "launch_json_requested: {}",
        report.launch_context.json_requested
    );
    let _ = writeln!(out, "launch_reason: {}", report.launch_context.reason);
    let _ = writeln!(out, "safety: {}", report.safety_note);
    out
}

fn write_optional_path(out: &mut String, label: &str, value: &Option<String>) {
    if let Some(value) = value {
        let _ = writeln!(out, "{label}: {value}");
    }
}

fn forbidden_path_label(class: ForbiddenPathClass) -> &'static str {
    match class {
        ForbiddenPathClass::Credentials => "credentials",
        ForbiddenPathClass::BrowserProfiles => "browser_profiles",
        ForbiddenPathClass::OauthSessions => "oauth_sessions",
        ForbiddenPathClass::UnknownUserData => "unknown_user_data",
        ForbiddenPathClass::Backups => "backups",
        ForbiddenPathClass::ProjectWorkspaces => "project_workspaces",
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

fn interface_label(mode: InterfaceMode) -> &'static str {
    match mode {
        InterfaceMode::Cli => "cli",
        InterfaceMode::Tui => "tui",
    }
}
