use std::fmt::Write as FmtWrite;

use serde::Serialize;

use crate::launch_routing::{
    InterfaceMode, LaunchEnvironment, LaunchMode, LaunchRoutingReport, resolve_launch_mode,
};
use crate::module_store::{
    ForbiddenPathClass, ModuleStorePlan, STORE_SCHEMA_VERSION, module_store_plan,
};
use crate::store_status::store_status_report;
use crate::store_status_text::store_status_text;
use crate::{ExitCode, brand};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

enum StoreAction {
    Plan(OutputFormat),
    Status(OutputFormat),
}

pub fn store_command(args: &[String]) -> (ExitCode, String, String) {
    match parse_store_args(args) {
        Ok(StoreAction::Plan(format)) => render_store_plan(format, args),
        Ok(StoreAction::Status(format)) => render_store_status(format, args),
        Err(err) => (ExitCode::Usage, String::new(), err),
    }
}

fn parse_store_args(args: &[String]) -> Result<StoreAction, String> {
    match args {
        [cmd] if cmd == "plan" => Ok(StoreAction::Plan(OutputFormat::Text)),
        [cmd, flag] if cmd == "plan" && flag == "--json" => {
            Ok(StoreAction::Plan(OutputFormat::Json))
        }
        [cmd, fmt, value] if cmd == "plan" && fmt == "--format" && value == "json" => {
            Ok(StoreAction::Plan(OutputFormat::Json))
        }
        [cmd] if cmd == "status" => Ok(StoreAction::Status(OutputFormat::Text)),
        [cmd, flag] if cmd == "status" && flag == "--json" => {
            Ok(StoreAction::Status(OutputFormat::Json))
        }
        [cmd, fmt, value] if cmd == "status" && fmt == "--format" && value == "json" => {
            Ok(StoreAction::Status(OutputFormat::Json))
        }
        _ => Err(usage_error(args)),
    }
}

fn render_store_plan(format: OutputFormat, args: &[String]) -> (ExitCode, String, String) {
    let report = store_plan_report(args);
    match format {
        OutputFormat::Text => (ExitCode::Ok, store_plan_text(&report), String::new()),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => (ExitCode::Ok, format!("{json}\n"), String::new()),
            Err(err) => (ExitCode::Usage, String::new(), err.to_string()),
        },
    }
}

fn render_store_status(format: OutputFormat, args: &[String]) -> (ExitCode, String, String) {
    let report = store_status_report(args);
    match format {
        OutputFormat::Text => (ExitCode::Ok, store_status_text(&report), String::new()),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => (ExitCode::Ok, format!("{json}\n"), String::new()),
            Err(err) => (ExitCode::Usage, String::new(), err.to_string()),
        },
    }
}

fn store_plan_report(args: &[String]) -> StorePlanReport {
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

fn store_plan_text(report: &StorePlanReport) -> String {
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

fn usage_error(args: &[String]) -> String {
    format!(
        "unsupported store option(s): {}\n\nUsage: {} store plan [--format json]\n       {} store status [--format json]\n",
        args.join(", "),
        brand::COMMAND,
        brand::COMMAND
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_plan_text_reports_read_only_contract() {
        let args = vec!["plan".to_string()];
        let (code, out, err) = store_command(&args);
        assert_eq!(code, ExitCode::Ok);
        assert!(err.is_empty());
        assert!(out.contains("mode: read-only"));
        assert!(out.contains("writes_attempted: no"));
        assert!(out.contains("registry_path:"));
        assert!(out.contains("forbidden_path_classes:"));
        assert!(out.contains("launch_mode: cli_subcommand"));
    }

    #[test]
    fn store_plan_json_reports_stable_contract_shape() {
        let args = vec![
            "plan".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ];
        let (code, out, err) = store_command(&args);
        assert_eq!(code, ExitCode::Ok);
        assert!(err.is_empty());
        assert!(out.contains("\"store_schema_version\": 1"));
        assert!(out.contains("\"writes_attempted\": false"));
        assert!(out.contains("\"registry_path\""));
        assert!(out.contains("\"receipt_path\""));
        assert!(out.contains("\"forbidden_path_classes\""));
        assert!(out.contains("\"launch_mode\": \"cli_subcommand\""));
        assert!(out.contains("\"json_requested\": true"));
    }

    #[test]
    fn store_status_text_reports_read_only_inventory() {
        let args = vec!["status".to_string()];
        let (code, out, err) = store_command(&args);
        assert_eq!(code, ExitCode::Ok);
        assert!(err.is_empty());
        assert!(out.contains("mode: read-only"));
        assert!(out.contains("writes_attempted: no"));
        assert!(out.contains("overall_state:"));
        assert!(out.contains("registry_path:"));
        assert!(out.contains("registry:"));
        assert!(out.contains("installed_module_count:"));
        assert!(out.contains("launch_mode: cli_subcommand"));
    }

    #[test]
    fn store_status_json_reports_stable_inventory_shape() {
        let args = vec![
            "status".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ];
        let (code, out, err) = store_command(&args);
        assert_eq!(code, ExitCode::Ok);
        assert!(err.is_empty());
        assert!(out.contains("\"command\": \"store status\""));
        assert!(out.contains("\"writes_attempted\": false"));
        assert!(out.contains("\"overall_state\""));
        assert!(out.contains("\"registry_path\""));
        assert!(out.contains("\"registry\""));
        assert!(out.contains("\"installed_module_count\""));
        assert!(out.contains("\"transactions_dir\""));
        assert!(out.contains("\"receipts_dir\""));
    }
}
