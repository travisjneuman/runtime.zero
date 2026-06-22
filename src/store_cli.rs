use std::path::PathBuf;
use std::{env, fs};

use crate::store_init::{StoreInitMode, StoreInitOptions, store_init_report};
use crate::store_init_text::store_init_text;
use crate::store_plan::{store_plan_report, store_plan_text};
use crate::store_status::{StoreStatusReport, store_status_report, store_status_report_for_root};
use crate::store_status_text::store_status_text;
use crate::{ExitCode, brand};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

enum StoreAction {
    Help,
    Plan(OutputFormat),
    Status(StoreStatusOptions),
    Init(StoreInitCliOptions),
}

struct StoreStatusOptions {
    format: OutputFormat,
    store_root: Option<PathBuf>,
}

struct StoreInitCliOptions {
    format: OutputFormat,
    mode: StoreInitMode,
}

pub fn store_command(args: &[String]) -> (ExitCode, String, String) {
    match parse_store_args(args) {
        Ok(StoreAction::Help) => (ExitCode::Ok, store_usage(), String::new()),
        Ok(StoreAction::Plan(format)) => render_store_plan(format, args),
        Ok(StoreAction::Status(options)) => render_store_status(options, args),
        Ok(StoreAction::Init(options)) => render_store_init(options, args),
        Err(err) => (ExitCode::Usage, String::new(), err),
    }
}

fn parse_store_args(args: &[String]) -> Result<StoreAction, String> {
    match args {
        [flag] if matches!(flag.as_str(), "--help" | "-h" | "help") => Ok(StoreAction::Help),
        [cmd] if cmd == "plan" => Ok(StoreAction::Plan(OutputFormat::Text)),
        [cmd, flag] if cmd == "plan" && flag == "--json" => {
            Ok(StoreAction::Plan(OutputFormat::Json))
        }
        [cmd, fmt, value] if cmd == "plan" && fmt == "--format" && value == "json" => {
            Ok(StoreAction::Plan(OutputFormat::Json))
        }
        [cmd, rest @ ..] if cmd == "status" => parse_store_status_args(rest),
        [cmd, rest @ ..] if cmd == "init" => parse_store_init_args(rest),
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

fn parse_store_init_args(args: &[String]) -> Result<StoreAction, String> {
    let mut format = OutputFormat::Text;
    let mut mode = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--json" => format = OutputFormat::Json,
            "--format" if args.get(index + 1).map(String::as_str) == Some("json") => {
                format = OutputFormat::Json;
                index += 1;
            }
            "--format" => return Err(usage_error(args)),
            "--dry-run" if mode.is_none() => mode = Some(StoreInitMode::DryRun),
            "--yes" if mode.is_none() => mode = Some(StoreInitMode::Apply),
            "--dry-run" | "--yes" => {
                return Err("store init accepts only one of --dry-run or --yes".to_string());
            }
            _ => return Err(usage_error(args)),
        }
        index += 1;
    }
    let mode = mode.ok_or_else(|| {
        format!(
            "store init requires --dry-run or --yes; start with '{} store init --dry-run'.\n",
            brand::COMMAND
        )
    })?;
    Ok(StoreAction::Init(StoreInitCliOptions { format, mode }))
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

fn render_store_init(options: StoreInitCliOptions, args: &[String]) -> (ExitCode, String, String) {
    let report = store_init_report(args, StoreInitOptions::new(options.mode));
    let code = if report.status.is_blocked() {
        ExitCode::Usage
    } else {
        ExitCode::Ok
    };
    match options.format {
        OutputFormat::Text => (code, store_init_text(&report), String::new()),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => (code, format!("{json}\n"), String::new()),
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

fn usage_error(args: &[String]) -> String {
    format!(
        "unsupported store option(s): {}\n\n{}",
        args.join(", "),
        store_usage()
    )
}

fn store_usage() -> String {
    format!(
        "Usage: {} store plan [--format json]\n       {} store status [--store-root <path>] [--format json]\n       {} store init --dry-run [--format json]\n       {} store init --yes [--format json]\n\nSafety: store status and plan are read-only; store init writes only with explicit --yes.\n",
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND,
        brand::COMMAND
    )
}
