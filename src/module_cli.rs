use std::fmt::Write as FmtWrite;
use std::path::Path;

use crate::{ExitCode, brand, module_manifest, module_registry, module_validation};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

enum ModulesAction {
    List {
        format: OutputFormat,
        from: Option<String>,
    },
    Validate {
        path: String,
        format: OutputFormat,
    },
}

pub fn modules_command(args: &[String]) -> (ExitCode, String, String) {
    match parse_modules_args(args) {
        Ok(ModulesAction::List { format, from }) => render_modules(format, from.as_deref()),
        Ok(ModulesAction::Validate { path, format }) => render_validation(format, &path),
        Err(err) => (ExitCode::Usage, String::new(), err),
    }
}

pub fn modules_text() -> String {
    modules_text_from(None)
}

pub fn modules_json() -> Result<String, String> {
    modules_json_from(None)
}

fn parse_modules_args(args: &[String]) -> Result<ModulesAction, String> {
    match args {
        [] => Ok(list_action(OutputFormat::Text, None)),
        [flag] if flag == "--json" => Ok(list_action(OutputFormat::Json, None)),
        [flag, value] if flag == "--format" && value == "json" => {
            Ok(list_action(OutputFormat::Json, None))
        }
        [flag, path] if flag == "--from" => Ok(list_action(OutputFormat::Text, Some(path))),
        [flag, path, json] if flag == "--from" && json == "--json" => {
            Ok(list_action(OutputFormat::Json, Some(path)))
        }
        [flag, path, fmt, value] if flag == "--from" && fmt == "--format" && value == "json" => {
            Ok(list_action(OutputFormat::Json, Some(path)))
        }
        [cmd, path] if cmd == "validate" => Ok(validate_action(path, OutputFormat::Text)),
        [cmd, path, json] if cmd == "validate" && json == "--json" => {
            Ok(validate_action(path, OutputFormat::Json))
        }
        [cmd, path, fmt, value] if cmd == "validate" && fmt == "--format" && value == "json" => {
            Ok(validate_action(path, OutputFormat::Json))
        }
        _ => Err(usage_error(args)),
    }
}

fn list_action(format: OutputFormat, from: Option<&String>) -> ModulesAction {
    ModulesAction::List {
        format,
        from: from.cloned(),
    }
}

fn validate_action(path: &str, format: OutputFormat) -> ModulesAction {
    ModulesAction::Validate {
        path: path.to_string(),
        format,
    }
}

fn render_modules(format: OutputFormat, from: Option<&str>) -> (ExitCode, String, String) {
    match format {
        OutputFormat::Text => (ExitCode::Ok, modules_text_from(from), String::new()),
        OutputFormat::Json => match modules_json_from(from) {
            Ok(json) => (ExitCode::Ok, json, String::new()),
            Err(err) => (ExitCode::Usage, String::new(), err),
        },
    }
}

fn render_validation(format: OutputFormat, path: &str) -> (ExitCode, String, String) {
    let report = module_validation::load_manifest_file(Path::new(path));
    let code = if report.valid {
        ExitCode::Ok
    } else {
        ExitCode::Usage
    };
    match format {
        OutputFormat::Text => (code, validation_text(&report), String::new()),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => (code, format!("{json}\n"), String::new()),
            Err(err) => (ExitCode::Usage, String::new(), err.to_string()),
        },
    }
}

fn modules_text_from(from: Option<&str>) -> String {
    let report = registry_report(from);
    let mut out = format!("{} modules\n\n", brand::TITLE);
    write_core(&mut out, &report.core);
    write_installed(&mut out, &report);
    write_planned(&mut out, &report.planned_module_families);
    let _ = writeln!(
        out,
        "\nsafety: optional modules are not bundled, installed, or executed by default"
    );
    out
}

fn write_core(out: &mut String, modules: &[module_manifest::ModuleManifest]) {
    let _ = writeln!(out, "core foundation:");
    for module in modules {
        let _ = writeln!(out, "  {:<16} active   {}", module.id, module.summary);
    }
}

fn write_installed(out: &mut String, report: &module_registry::ModuleRegistryReport) {
    let _ = writeln!(out, "\ninstalled modules:");
    if report.installed_modules.is_empty() {
        let _ = writeln!(out, "  none");
    } else {
        for module in &report.installed_modules {
            let _ = writeln!(out, "  {:<22} installed {}", module.id, module.summary);
        }
    }
    if !report.validation_reports.is_empty() {
        write_validation_summary(out, &report.validation_reports);
    }
}

fn write_validation_summary(
    out: &mut String,
    reports: &[module_validation::ManifestValidationReport],
) {
    let _ = writeln!(out, "\nvalidation:");
    for report in reports {
        let status = if report.valid { "valid" } else { "invalid" };
        let _ = writeln!(out, "  {:<8} {}", status, report.path);
    }
}

fn write_planned(out: &mut String, modules: &[module_manifest::ModuleManifest]) {
    let _ = writeln!(out, "\nplanned first-party module families:");
    for module in modules {
        let _ = writeln!(out, "  {:<22} planned  {}", module.id, module.summary);
    }
}

fn modules_json_from(from: Option<&str>) -> Result<String, String> {
    serde_json::to_string_pretty(&registry_report(from))
        .map(|json| format!("{json}\n"))
        .map_err(|err| format!("failed to render module registry JSON: {err}\n"))
}

fn registry_report(from: Option<&str>) -> module_registry::ModuleRegistryReport {
    match from {
        Some(path) => module_registry::ModuleRegistryReport::from_directory(Path::new(path)),
        None => module_registry::ModuleRegistryReport::empty_installed(),
    }
}

fn validation_text(report: &module_validation::ManifestValidationReport) -> String {
    let status = if report.valid { "valid" } else { "invalid" };
    let mut out = format!("{} module manifest validation\n\n", brand::TITLE);
    let _ = writeln!(out, "path: {}", report.path);
    let _ = writeln!(out, "status: {status}");
    for error in &report.errors {
        let _ = writeln!(out, "error: {error}");
    }
    for warning in &report.warnings {
        let _ = writeln!(out, "warning: {warning}");
    }
    out
}

fn usage_error(args: &[String]) -> String {
    format!(
        "unsupported modules option(s): {}\n\nUsage: {} modules [--from <dir>] [--format json]\n       {} modules validate <manifest.json> [--format json]\n",
        args.join(", "),
        brand::COMMAND,
        brand::COMMAND
    )
}
