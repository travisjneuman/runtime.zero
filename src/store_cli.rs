use std::fmt::Write as FmtWrite;
use std::path::PathBuf;
use std::{env, fs};

use serde::Serialize;

use crate::launch_routing::{
    InterfaceMode, LaunchEnvironment, LaunchMode, LaunchRoutingReport, resolve_launch_mode,
};
use crate::module_store::{
    ForbiddenPathClass, ModuleStorePlan, STORE_SCHEMA_VERSION, module_store_plan,
};
use crate::store_status::{StoreStatusReport, store_status_report, store_status_report_for_root};
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
    Status(StoreStatusOptions),
}

struct StoreStatusOptions {
    format: OutputFormat,
    store_root: Option<PathBuf>,
}

pub fn store_command(args: &[String]) -> (ExitCode, String, String) {
    match parse_store_args(args) {
        Ok(StoreAction::Plan(format)) => render_store_plan(format, args),
        Ok(StoreAction::Status(options)) => render_store_status(options, args),
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
        [cmd, rest @ ..] if cmd == "status" => parse_store_status_args(rest),
        _ => Err(usage_error(args)),
    }
}

fn parse_store_status_args(args: &[String]) -> Result<StoreAction, String> {
    let mut format = OutputFormat::Text;
    let mut store_root = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => format = OutputFormat::Json,
            "--format" if args.get(index + 1).map(String::as_str) == Some("json") => {
                format = OutputFormat::Json;
                index += 1;
            }
            "--format" => return Err(usage_error(args)),
            "--store-root" => {
                let value = args.get(index + 1).ok_or_else(|| usage_error(args))?;
                if store_root.is_some() {
                    return Err("store root override was provided more than once".to_string());
                }
                store_root = Some(resolve_store_root_override(value)?);
                index += 1;
            }
            _ => return Err(usage_error(args)),
        }
        index += 1;
    }
    Ok(StoreAction::Status(StoreStatusOptions {
        format,
        store_root,
    }))
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

fn render_store_status(options: StoreStatusOptions, args: &[String]) -> (ExitCode, String, String) {
    let report = status_report(args, options.store_root);
    match options.format {
        OutputFormat::Text => (ExitCode::Ok, store_status_text(&report), String::new()),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => (ExitCode::Ok, format!("{json}\n"), String::new()),
            Err(err) => (ExitCode::Usage, String::new(), err.to_string()),
        },
    }
}

fn status_report(args: &[String], store_root: Option<PathBuf>) -> StoreStatusReport {
    match store_root {
        Some(root) => store_status_report_for_root(args, Some(root)),
        None => store_status_report(args),
    }
}

fn resolve_store_root_override(value: &str) -> Result<PathBuf, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || looks_url_like(trimmed) {
        return Err("store root override must be a local filesystem path".to_string());
    }
    let path = PathBuf::from(trimmed);
    let absolute = if path.is_absolute() {
        path
    } else {
        env::current_dir()
            .map_err(|err| format!("failed to resolve current directory: {err}"))?
            .join(path)
    };
    if absolute.exists() {
        fs::canonicalize(&absolute)
            .map_err(|err| format!("failed to canonicalize store root override: {err}"))
    } else {
        Ok(absolute)
    }
}

fn looks_url_like(value: &str) -> bool {
    value.contains("://")
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
        "unsupported store option(s): {}\n\nUsage: {} store plan [--format json]\n       {} store status [--store-root <path>] [--format json]\n",
        args.join(", "),
        brand::COMMAND,
        brand::COMMAND
    )
}
