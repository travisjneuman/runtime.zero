use std::fmt::Write as FmtWrite;

use serde::Serialize;

use crate::{ExitCode, brand};

pub const CONFIGURATION_REPORT_SCHEMA_VERSION: u16 = 1;
pub const CONFIGURATION_REPORT_CONTRACT: &str = "configuration_review";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Serialize)]
pub struct ConfigurationReport {
    pub schema_version: u16,
    pub contract: &'static str,
    pub valid: bool,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub configuration_authorizes_execution: bool,
    pub configuration_sha256: String,
    pub configuration: rz0_configuration_contract::FoundationConfiguration,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn configuration_command(args: &[String]) -> (ExitCode, String, String) {
    let format = match parse_format(args) {
        Ok(format) => format,
        Err(error) => return (ExitCode::Usage, String::new(), error),
    };
    let report = configuration_report();
    match format {
        OutputFormat::Text => (ExitCode::Ok, configuration_text(&report), String::new()),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => (ExitCode::Ok, format!("{json}\n"), String::new()),
            Err(error) => (
                ExitCode::Usage,
                String::new(),
                format!("failed to serialize configuration review: {error}\n"),
            ),
        },
    }
}

pub fn configuration_report() -> ConfigurationReport {
    let configuration = rz0_configuration_contract::default_configuration();
    let validation = rz0_configuration_contract::validate_configuration(&configuration);
    let digest = rz0_configuration_contract::configuration_sha256(&configuration)
        .unwrap_or_else(|_| "0".repeat(64));
    let mut warnings = vec![
        "configuration is built-in schema-one policy; no user configuration is loaded".to_string(),
        "configuration describes safeguards but never authorizes module execution or mutation"
            .to_string(),
    ];
    if !validation.valid {
        warnings.push("built-in configuration failed its own validation".to_string());
    }
    ConfigurationReport {
        schema_version: CONFIGURATION_REPORT_SCHEMA_VERSION,
        contract: CONFIGURATION_REPORT_CONTRACT,
        valid: validation.valid,
        read_only: true,
        writes_attempted: false,
        configuration_authorizes_execution: configuration.configuration_authorizes_execution,
        configuration_sha256: digest,
        configuration,
        errors: validation.errors,
        warnings,
    }
}

fn parse_format(args: &[String]) -> Result<OutputFormat, String> {
    let mut format = None;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--json" if format.is_none() => format = Some(OutputFormat::Json),
            "--json" => return Err(configuration_usage()),
            "--format" => {
                let Some(value) = args.get(index + 1).map(String::as_str) else {
                    return Err(configuration_usage());
                };
                if format.is_some() {
                    return Err(configuration_usage());
                }
                format = match value {
                    "text" => Some(OutputFormat::Text),
                    "json" => Some(OutputFormat::Json),
                    _ => return Err(configuration_usage()),
                };
                index += 1;
            }
            "--help" | "-h" | "help" => return Err(configuration_usage()),
            _ => return Err(configuration_usage()),
        }
        index += 1;
    }
    Ok(format.unwrap_or(OutputFormat::Text))
}

fn configuration_text(report: &ConfigurationReport) -> String {
    let mut out = format!("{} effective configuration\n\n", brand::TITLE);
    let _ = writeln!(out, "contract: {}", report.contract);
    let _ = writeln!(out, "schema_version: {}", report.schema_version);
    let _ = writeln!(out, "valid: {}", report.valid);
    let _ = writeln!(out, "configuration_sha256: {}", report.configuration_sha256);
    let _ = writeln!(out, "source: built_in_defaults");
    let _ = writeln!(out, "read_only: {}", report.read_only);
    let _ = writeln!(out, "writes_attempted: {}", report.writes_attempted);
    let _ = writeln!(
        out,
        "configuration_authorizes_execution: {}",
        report.configuration_authorizes_execution
    );
    let _ = writeln!(out, "privacy:");
    let _ = writeln!(
        out,
        "  redact_local_paths_by_default: {}",
        report.configuration.privacy.redact_local_paths_by_default
    );
    let _ = writeln!(
        out,
        "  collect_hostname: {}",
        report.configuration.privacy.collect_hostname
    );
    let _ = writeln!(
        out,
        "  collect_current_user: {}",
        report.configuration.privacy.collect_current_user
    );
    let _ = writeln!(
        out,
        "  collect_environment_values: {}",
        report.configuration.privacy.collect_environment_values
    );
    let _ = writeln!(
        out,
        "  telemetry_enabled: {}",
        report.configuration.privacy.telemetry_enabled
    );
    let _ = writeln!(out, "execution:");
    let _ = writeln!(out, "  production_modules: disabled");
    let _ = writeln!(out, "  remote_execution: disabled");
    let _ = writeln!(out, "  shell_execution: disabled");
    let _ = writeln!(out, "  network_default: deny");
    let _ = writeln!(
        out,
        "  maximum_parallel_module_processes: {}",
        report
            .configuration
            .execution
            .maximum_parallel_module_processes
    );
    let _ = writeln!(out, "mutation:");
    let _ = writeln!(
        out,
        "  dry_run_required: {}",
        report.configuration.mutation.dry_run_required
    );
    let _ = writeln!(
        out,
        "  exact_confirmation_required: {}",
        report.configuration.mutation.exact_confirmation_required
    );
    let _ = writeln!(
        out,
        "  quarantine_before_removal: {}",
        report.configuration.mutation.quarantine_before_removal
    );
    let _ = writeln!(
        out,
        "  automatic_retry: {}",
        report.configuration.mutation.automatic_retry
    );
    let _ = writeln!(out, "lifecycle:");
    let _ = writeln!(out, "  automatic_update: false");
    let _ = writeln!(out, "  background_service: false");
    let _ = writeln!(out, "  implicit_migration: false");
    let _ = writeln!(out, "  startup_repair: false");
    if !report.errors.is_empty() {
        let _ = writeln!(out, "errors:");
        for error in &report.errors {
            let _ = writeln!(out, "  - {error}");
        }
    }
    let _ = writeln!(out, "warnings:");
    for warning in &report.warnings {
        let _ = writeln!(out, "  - {warning}");
    }
    out
}

fn configuration_usage() -> String {
    format!(
        "configuration review is read-only and built-in-defaults-only\n\nUsage: {} config [--format text|json|--json]\n\nIt reports the effective schema-one privacy, execution, mutation, and lifecycle safeguards. It never loads user configuration, writes state, enables modules, or authorizes execution.\n",
        brand::COMMAND
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuration_report_is_private_and_non_authorizing() {
        let report = configuration_report();
        assert!(report.valid);
        assert!(report.read_only);
        assert!(!report.writes_attempted);
        assert!(!report.configuration_authorizes_execution);
        assert!(report.configuration.privacy.redact_local_paths_by_default);
        assert_eq!(
            report.configuration.execution.network_default,
            rz0_configuration_contract::NetworkDefault::Deny
        );
    }

    #[test]
    fn configuration_format_rejects_unknown_or_duplicate_options() {
        assert!(parse_format(&["--unknown".to_string()]).is_err());
        assert!(parse_format(&["--json".to_string(), "--json".to_string()]).is_err());
        assert!(
            parse_format(&[
                "--format".to_string(),
                "text".to_string(),
                "--format".to_string(),
                "json".to_string()
            ])
            .is_err()
        );
        assert!(parse_format(&["--format".to_string(), "yaml".to_string()]).is_err());
    }
}
