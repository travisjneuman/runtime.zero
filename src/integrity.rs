use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::Path;

use rz0_artifact_identity::open_observed_artifact;
use rz0_finding_contract::{
    ExactFindingEvidence, FindingDataClass, FindingOwnership, FindingReport,
};
use rz0_module_security_integrity::{
    INPUT_CONTRACT as INTEGRITY_INPUT_CONTRACT, IntegrityFindingInput, IntegrityRecord,
    classify_integrity,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

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
    let report = match (options.fixture.as_deref(), options.path.as_deref(), options.sha256.as_deref()) {
        (Some(fixture), None, None) => fixture_report(fixture),
        (None, Some(path), Some(expected_sha256)) => live_exact_file_report(path, expected_sha256),
        _ => Err(
            "integrity review requires either --fixture <path> or --path <absolute-file> --sha256 <digest>"
                .to_string(),
        ),
    };
    let report = match report {
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
    path: Option<String>,
    sha256: Option<String>,
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut dry_run = false;
    let mut format = OutputFormat::Text;
    let mut fixture = None;
    let mut path = None;
    let mut sha256 = None;
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
            "--path" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("integrity --path requires an absolute local file path".to_string());
                };
                if path.replace(value.clone()).is_some() {
                    return Err("integrity --path was provided more than once".to_string());
                }
                index += 1;
            }
            "--sha256" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(
                        "integrity --sha256 requires a 64-character lowercase digest".to_string(),
                    );
                };
                if sha256.replace(value.clone()).is_some() {
                    return Err("integrity --sha256 was provided more than once".to_string());
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
    if fixture.is_some() && (path.is_some() || sha256.is_some()) {
        return Err("integrity --fixture cannot be combined with --path or --sha256".to_string());
    }
    match (&path, &sha256) {
        (Some(path), Some(sha256)) => {
            if !Path::new(path).is_absolute() {
                return Err("integrity --path must be absolute".to_string());
            }
            if !rz0_validation_contract::valid_sha256(sha256) {
                return Err(
                    "integrity --sha256 must be a lowercase 64-character digest".to_string()
                );
            }
        }
        (None, None) if fixture.is_none() => {
            return Err(
                "integrity review requires --fixture or --path with --sha256; no trusted baseline is configured"
                    .to_string(),
            );
        }
        (None, Some(_)) | (Some(_), None) => {
            return Err("integrity --path and --sha256 must be provided together".to_string());
        }
        _ => {}
    }
    Ok(Options {
        format,
        fixture,
        path,
        sha256,
    })
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

fn live_exact_file_report(
    path: &str,
    expected_sha256: &str,
) -> Result<IntegrityReviewReport, String> {
    let path = Path::new(path);
    let parent = path
        .parent()
        .filter(|parent| parent.is_absolute())
        .ok_or_else(|| "integrity exact-file path has no absolute parent".to_string())?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "integrity exact-file path has no valid final name".to_string())?;
    let observed = open_observed_artifact(parent, file_name)
        .map_err(|error| format!("observe exact integrity file: {error}"))?;
    let expected_size = observed.size_bytes;
    let expected_digest_matches = observed.sha256 == expected_sha256;
    let evidence_sha256 = digest_parts(&[
        "integrity.exact-file",
        expected_sha256,
        &observed.sha256,
        &expected_size.to_string(),
    ]);
    let input = IntegrityFindingInput {
        schema_version: 1,
        contract: INTEGRITY_INPUT_CONTRACT.to_string(),
        platform: std::env::consts::OS.to_string(),
        input_evidence_sha256: evidence_sha256.clone(),
        source_id: "integrity.exact-file".to_string(),
        source_evidence_sha256: evidence_sha256,
        records: vec![IntegrityRecord {
            finding_id: format!("integrity.exact-file.{}", &observed.sha256[..16]),
            subject_reference: format!("artifact:sha256:{}", &observed.sha256[..16]),
            ownership: FindingOwnership::Unknown,
            data_class: FindingDataClass::Unknown,
            expected_digest_matches,
            exact_evidence: ExactFindingEvidence {
                sha256: observed.sha256,
                size_bytes: observed.size_bytes,
            },
        }],
    };
    let finding_report = classify_integrity(&input)?;
    Ok(IntegrityReviewReport {
        schema_version: 1,
        contract: INTEGRITY_REVIEW_CONTRACT,
        read_only: true,
        writes_attempted: false,
        raw_paths_included: false,
        platform: std::env::consts::OS,
        baseline_status: "caller-supplied exact digest; not a runtime trust baseline",
        finding_report,
        warnings: vec![
            "the exact file was observed through a path-safe opened artifact; the path is omitted from the report"
                .to_string(),
            "no trusted runtime baseline is configured; the supplied digest is evidence only"
                .to_string(),
            "integrity review does not detect malware or authorize remediation".to_string(),
        ],
    })
}

fn digest_parts(parts: &[&str]) -> String {
    let mut digest = Sha256::new();
    digest.update(b"runtime.zero.integrity-observation.v1\0");
    for part in parts {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part.as_bytes());
    }
    format!("{:x}", digest.finalize())
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
    "Usage: rz0 integrity --dry-run --fixture <integrity-input.json> [--format text|json]\n       rz0 integrity --dry-run --path <absolute-file> --sha256 <digest> [--format text|json]\n\nReviews caller-supplied exact digest observations through the shared report contract. The live exact-file form is bounded, path-safe, read-only, and still not a trusted runtime baseline. No remediation, malware-detection, quarantine, restore, or deletion path exists.\n".to_string()
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

    #[test]
    fn exact_file_review_is_read_only_path_free_and_unknown_owned() {
        let root = std::env::temp_dir().join(format!(
            "rz0-integrity-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir(&root).expect("create integrity test root");
        let file = root.join("sample.bin");
        let bytes = b"runtime.zero integrity sample";
        fs::write(&file, bytes).expect("write integrity sample");
        let expected = format!("{:x}", Sha256::digest(bytes));
        let (code, output, error) = integrity_command(&[
            "--dry-run".to_string(),
            "--path".to_string(),
            file.display().to_string(),
            "--sha256".to_string(),
            expected,
            "--format".to_string(),
            "json".to_string(),
        ]);
        assert_eq!(code, ExitCode::Ok);
        assert!(error.is_empty());
        let json: serde_json::Value = serde_json::from_str(&output).expect("integrity JSON");
        assert_eq!(json["read_only"], true);
        assert_eq!(json["writes_attempted"], false);
        assert_eq!(json["raw_paths_included"], false);
        assert_eq!(
            json["baseline_status"],
            "caller-supplied exact digest; not a runtime trust baseline"
        );
        assert_eq!(
            json["finding_report"]["findings"][0]["ownership"],
            "unknown"
        );
        assert!(!output.contains("sample.bin"));
        fs::remove_file(file).expect("remove integrity sample");
        fs::remove_dir(root).expect("remove integrity test root");
    }
}
