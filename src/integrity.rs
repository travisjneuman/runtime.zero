use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::Path;

use rz0_finding_contract::FindingReport;
use rz0_module_security_integrity::{
    INPUT_CONTRACT as INTEGRITY_INPUT_CONTRACT, IntegrityFindingInput, classify_integrity,
};
use serde::Serialize;

use crate::{ExitCode, brand};

pub const INTEGRITY_REVIEW_CONTRACT: &str = "integrity_review";
const MAX_INPUT_BYTES: u64 = rz0_resource_contract::MAX_FINDING_REPORT_BYTES;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IntegrityReviewReport {
    pub schema_version: u16,
    pub contract: &'static str,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub raw_paths_included: bool,
    pub platform: &'static str,
    pub baseline_status: &'static str,
    pub finding_report: FindingReport,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

pub fn integrity_command(args: &[String]) -> (ExitCode, String, String) {
    if matches!(args, [help] if matches!(help.as_str(), "--help" | "-h" | "help")) {
        return (ExitCode::Ok, usage(), String::new());
    }
    let options = match parse_args(args) {
        Ok(options) => options,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("{error}\n\n{}", usage()),
            );
        }
    };
    let report = match fixture_report(options.fixture.as_deref().expect("fixture is required")) {
        Ok(report) => report,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("integrity review failed closed: {error}\n"),
            );
        }
    };
    match options.format {
        OutputFormat::Text => (ExitCode::Ok, render_text(&report), String::new()),
        OutputFormat::Json => match serde_json::to_string_pretty(&report) {
            Ok(json) => (ExitCode::Ok, format!("{json}\n"), String::new()),
            Err(error) => (
                ExitCode::Usage,
                String::new(),
                format!("integrity review JSON rendering failed: {error}\n"),
            ),
        },
    }
}

struct Options {
    format: OutputFormat,
    fixture: Option<String>,
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut dry_run = false;
    let mut format = OutputFormat::Text;
    let mut fixture = None;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--dry-run" if !dry_run => dry_run = true,
            "--dry-run" => {
                return Err("integrity --dry-run was provided more than once".to_string());
            }
            "--json" => format = OutputFormat::Json,
            "--format" => {
                let Some(value) = args.get(index + 1).map(String::as_str) else {
                    return Err("integrity --format requires text or json".to_string());
                };
                format = match value {
                    "text" => OutputFormat::Text,
                    "json" => OutputFormat::Json,
                    _ => return Err(format!("unsupported integrity output format '{value}'")),
                };
                index += 1;
            }
            "--fixture" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("integrity --fixture requires a local JSON path".to_string());
                };
                if fixture.replace(value.clone()).is_some() {
                    return Err("integrity --fixture was provided more than once".to_string());
                }
                index += 1;
            }
            value => return Err(format!("unsupported integrity option '{value}'")),
        }
        index += 1;
    }
    if !dry_run {
        return Err("integrity review is dry-run only; pass --dry-run".to_string());
    }
    if fixture.is_none() {
        return Err(
            "integrity review requires --fixture because no trusted baseline is configured"
                .to_string(),
        );
    }
    Ok(Options { format, fixture })
}

fn fixture_report(path: &str) -> Result<IntegrityReviewReport, String> {
    let path = Path::new(path);
    let metadata = fs::symlink_metadata(path).map_err(|_| "fixture cannot be read".to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > MAX_INPUT_BYTES
    {
        return Err("integrity fixture must be a bounded regular non-symlink file".to_string());
    }
    let bytes = fs::read(path).map_err(|_| "fixture cannot be read".to_string())?;
    if bytes.len() as u64 != metadata.len() {
        return Err("integrity fixture changed while reading".to_string());
    }
    let input: IntegrityFindingInput = serde_json::from_slice(&bytes)
        .map_err(|error| format!("integrity fixture JSON is invalid: {error}"))?;
    if input.contract != INTEGRITY_INPUT_CONTRACT {
        return Err("integrity fixture contract is invalid".to_string());
    }
    let finding_report = classify_integrity(&input)?;
    Ok(IntegrityReviewReport {
        schema_version: 1,
        contract: INTEGRITY_REVIEW_CONTRACT,
        read_only: true,
        writes_attempted: false,
        raw_paths_included: false,
        platform: std::env::consts::OS,
        baseline_status: "caller-supplied fixture; not a runtime trust baseline",
        finding_report,
        warnings: vec![
            "no trusted runtime baseline is configured; fixture results are evidence only"
                .to_string(),
            "integrity review does not detect malware or authorize remediation".to_string(),
        ],
    })
}

fn render_text(report: &IntegrityReviewReport) -> String {
    let finding_report = &report.finding_report;
    let mut out = format!("{} integrity review\n\n", brand::TITLE);
    let _ = writeln!(out, "mode: dry-run read-only");
    let _ = writeln!(out, "contract: {}", report.contract);
    let _ = writeln!(out, "platform: {}", report.platform);
    let _ = writeln!(out, "baseline: {}", report.baseline_status);
    let _ = writeln!(out, "sources: {}", finding_report.summary.source_count);
    let _ = writeln!(out, "findings: {}", finding_report.summary.finding_count);
    let _ = writeln!(
        out,
        "report_only: {}",
        finding_report.summary.report_only_count
    );
    let _ = writeln!(out, "blocked: {}", finding_report.summary.blocked_count);
    let _ = writeln!(out, "writes_attempted: no");
    let _ = writeln!(out, "raw_paths_included: no");
    if !finding_report.findings.is_empty() {
        out.push_str("observations:\n");
        for finding in &finding_report.findings {
            let _ = writeln!(
                out,
                "  - {} [{}] {}",
                finding.subject_reference,
                integrity_risk_label(finding.risk),
                integrity_disposition_label(finding.disposition)
            );
        }
    }
    out.push_str("warnings:\n");
    for warning in &report.warnings {
        let _ = writeln!(out, "  - {warning}");
    }
    out.push_str(
        "\nsafety: evidence-only digest observations; no remediation, quarantine, restore, deletion, or malware-detection claim is available.\n",
    );
    out
}

fn usage() -> String {
    "Usage: rz0 integrity --dry-run --fixture <integrity-input.json> [--format text|json]\n\nReviews caller-supplied exact digest observations through the shared report contract. A trusted runtime baseline is not configured, so fixture input is mandatory. No remediation, malware-detection, quarantine, restore, or deletion path exists.\n".to_string()
}

fn integrity_risk_label(risk: rz0_finding_contract::FindingRisk) -> &'static str {
    match risk {
        rz0_finding_contract::FindingRisk::Low => "low",
        rz0_finding_contract::FindingRisk::Medium => "medium",
        rz0_finding_contract::FindingRisk::High => "high",
        rz0_finding_contract::FindingRisk::Blocked => "blocked",
    }
}

fn integrity_disposition_label(
    disposition: rz0_finding_contract::FindingDisposition,
) -> &'static str {
    match disposition {
        rz0_finding_contract::FindingDisposition::ReportOnly => "report_only",
        rz0_finding_contract::FindingDisposition::Blocked => "blocked",
        rz0_finding_contract::FindingDisposition::Ignore => "ignore",
        rz0_finding_contract::FindingDisposition::ManagerActionCandidate => {
            "manager_action_candidate"
        }
        rz0_finding_contract::FindingDisposition::QuarantineCandidate => "quarantine_candidate",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integrity_requires_an_explicit_fixture() {
        let (code, _, error) = integrity_command(&["--dry-run".to_string()]);
        assert_eq!(code, ExitCode::Usage);
        assert!(error.contains("trusted baseline"));
    }
}
