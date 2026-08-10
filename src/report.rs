use rz0_support_contract::{build_support_report, support_json, support_text};

use crate::{ExitCode, doctor_report, inventory};

pub fn report_command(args: &[String]) -> (ExitCode, String, String) {
    if matches!(args, [help] if matches!(help.as_str(), "--help" | "-h" | "help")) {
        return (ExitCode::Ok, usage(), String::new());
    }
    let format = match parse_format(args) {
        Ok(format) => format,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("{error}\n\n{}", usage()),
            );
        }
    };
    let inventory = match inventory::live_report(true) {
        Ok(report) => report,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("privacy-reviewed inventory failed closed: {error}\n"),
            );
        }
    };
    let diagnostics = doctor_report();
    let report = match build_support_report(&inventory, &diagnostics) {
        Ok(report) => report,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("support report failed closed: {error}\n"),
            );
        }
    };
    let rendered = match format {
        ReportFormat::Text => support_text(&report),
        ReportFormat::Json => support_json(&report),
    };
    match rendered {
        Ok(output) => (ExitCode::Ok, output, String::new()),
        Err(error) => (
            ExitCode::Usage,
            String::new(),
            format!("support report rendering failed closed: {error}\n"),
        ),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReportFormat {
    Text,
    Json,
}

fn parse_format(args: &[String]) -> Result<ReportFormat, String> {
    match args {
        [] => Ok(ReportFormat::Text),
        [json] if json == "--json" => Ok(ReportFormat::Json),
        [flag, value] if flag == "--format" && value == "text" => Ok(ReportFormat::Text),
        [flag, value] if flag == "--format" && value == "json" => Ok(ReportFormat::Json),
        _ => Err("unsupported report option".to_string()),
    }
}

fn usage() -> String {
    "Usage: rz0 report [--format text|json]\n\nBuilds a deterministic local summary from path-redacted inventory and private diagnostics. Raw reports, paths, host/user identities, application/service names, process output, credentials, and free-form warnings are omitted. The result is not automatically authorized for external sharing.\n".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_format_is_small_explicit_and_scriptable() {
        assert_eq!(parse_format(&[]).unwrap(), ReportFormat::Text);
        assert_eq!(
            parse_format(&["--json".to_string()]).unwrap(),
            ReportFormat::Json
        );
        assert_eq!(
            parse_format(&["--format".to_string(), "json".to_string()]).unwrap(),
            ReportFormat::Json
        );
        assert!(parse_format(&["--output".to_string()]).is_err());
    }
}
