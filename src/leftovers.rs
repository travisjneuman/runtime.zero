use std::collections::VecDeque;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};

use rz0_finding_contract::FindingReport;
use rz0_module_leftovers::{
    INPUT_CONTRACT as LEFTOVER_INPUT_CONTRACT, LeftoverFindingInput, LeftoverRecord,
    classify_leftovers,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{ExitCode, brand, module_store::module_store_plan};

pub const LEFTOVERS_REVIEW_CONTRACT: &str = "leftovers_review";
const MAX_INPUT_BYTES: u64 = rz0_resource_contract::MAX_FINDING_REPORT_BYTES;
const MAX_SCAN_NODES: usize = 2_048;
const MAX_SCAN_BYTES: u64 = 64 * 1024 * 1024;
const MAX_WARNINGS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LeftoversReviewReport {
    pub schema_version: u16,
    pub contract: &'static str,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub raw_paths_included: bool,
    pub platform: &'static str,
    pub finding_report: FindingReport,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
struct LeftoverRoot {
    id: &'static str,
    path: PathBuf,
}

#[derive(Debug, Clone)]
struct LeftoverObservation {
    id: &'static str,
    file_count: usize,
    total_bytes: u64,
    evidence_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

pub fn leftovers_command(args: &[String]) -> (ExitCode, String, String) {
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
    let report = match options.fixture.as_deref() {
        Some(path) => fixture_report(Path::new(path)),
        None => live_report(),
    };
    let report = match report {
        Ok(report) => report,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("leftovers review failed closed: {error}\n"),
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
                format!("leftovers review JSON rendering failed: {error}\n"),
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
                return Err("leftovers --dry-run was provided more than once".to_string());
            }
            "--json" => format = OutputFormat::Json,
            "--format" => {
                let Some(value) = args.get(index + 1).map(String::as_str) else {
                    return Err("leftovers --format requires text or json".to_string());
                };
                format = match value {
                    "text" => OutputFormat::Text,
                    "json" => OutputFormat::Json,
                    _ => return Err(format!("unsupported leftovers output format '{value}'")),
                };
                index += 1;
            }
            "--fixture" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("leftovers --fixture requires a local JSON path".to_string());
                };
                if fixture.replace(value.clone()).is_some() {
                    return Err("leftovers --fixture was provided more than once".to_string());
                }
                index += 1;
            }
            value => return Err(format!("unsupported leftovers option '{value}'")),
        }
        index += 1;
    }
    if !dry_run {
        return Err("leftovers review is dry-run only; pass --dry-run".to_string());
    }
    Ok(Options { format, fixture })
}

fn fixture_report(path: &Path) -> Result<LeftoversReviewReport, String> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "fixture cannot be read".to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > MAX_INPUT_BYTES
    {
        return Err("leftovers fixture must be a bounded regular non-symlink file".to_string());
    }
    let bytes = fs::read(path).map_err(|_| "fixture cannot be read".to_string())?;
    if bytes.len() as u64 != metadata.len() {
        return Err("leftovers fixture changed while reading".to_string());
    }
    let input: LeftoverFindingInput = serde_json::from_slice(&bytes)
        .map_err(|error| format!("leftovers fixture JSON is invalid: {error}"))?;
    let finding_report = classify_leftovers(&input)?;
    Ok(wrap_report(
        finding_report,
        vec!["fixture evidence was supplied; no live leftover paths were inspected".to_string()],
    ))
}

pub fn live_report() -> Result<LeftoversReviewReport, String> {
    let plan = module_store_plan(None, None, "leftovers review");
    let roots = vec![
        LeftoverRoot {
            id: "runtime-zero-modules",
            path: PathBuf::from(plan.modules_root),
        },
        LeftoverRoot {
            id: "runtime-zero-logs",
            path: PathBuf::from(plan.log_root),
        },
    ];
    let mut warnings = Vec::new();
    let mut observations = Vec::new();
    for root in roots {
        if let Some(observation) = inspect_root(&root, &mut warnings)? {
            observations.push(observation);
        }
    }
    observations.sort_by(|left, right| left.id.cmp(right.id));
    let source_evidence_sha256 = observation_digest(&observations);
    let records = observations
        .iter()
        .map(|observation| LeftoverRecord {
            finding_id: format!("leftover.{}", &observation.evidence_sha256[..16]),
            subject_reference: format!("leftover:{}", observation.id),
            ownership: rz0_finding_contract::FindingOwnership::RuntimeOwned,
            data_class: rz0_finding_contract::FindingDataClass::OrphanedData,
            // Directory-listing evidence does not prove stale ownership or a
            // safe exact-file transaction, so this remains report-only.
            exact_evidence: None,
        })
        .collect::<Vec<_>>();
    let input = LeftoverFindingInput {
        schema_version: 1,
        contract: LEFTOVER_INPUT_CONTRACT.to_string(),
        platform: std::env::consts::OS.to_string(),
        input_evidence_sha256: source_evidence_sha256.clone(),
        source_id: "leftovers.local".to_string(),
        source_evidence_sha256,
        records,
    };
    let finding_report = classify_leftovers(&input)?;
    Ok(wrap_report(finding_report, warnings))
}

fn wrap_report(finding_report: FindingReport, warnings: Vec<String>) -> LeftoversReviewReport {
    LeftoversReviewReport {
        schema_version: 1,
        contract: LEFTOVERS_REVIEW_CONTRACT,
        read_only: true,
        writes_attempted: false,
        raw_paths_included: false,
        platform: std::env::consts::OS,
        finding_report,
        warnings,
    }
}

fn inspect_root(
    root: &LeftoverRoot,
    warnings: &mut Vec<String>,
) -> Result<Option<LeftoverObservation>, String> {
    let metadata = match fs::symlink_metadata(&root.path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => {
            add_warning(
                warnings,
                format!("{} source could not be inspected", root.id),
            );
            return Ok(None);
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        add_warning(
            warnings,
            format!("{} source is not a regular directory", root.id),
        );
        return Ok(None);
    }

    let mut queue = VecDeque::from([root.path.clone()]);
    let mut node_count = 0usize;
    let mut file_count = 0usize;
    let mut total_bytes = 0u64;
    let mut digest = Sha256::new();
    while let Some(directory) = queue.pop_front() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(_) => {
                add_warning(
                    warnings,
                    format!("{} source has an unreadable directory", root.id),
                );
                continue;
            }
        };
        for entry in entries {
            node_count = node_count.saturating_add(1);
            if node_count > MAX_SCAN_NODES {
                add_warning(
                    warnings,
                    format!("{} source reached its scan ceiling", root.id),
                );
                queue.clear();
                break;
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    add_warning(
                        warnings,
                        format!("{} source contains an unreadable entry", root.id),
                    );
                    continue;
                }
            };
            let path = entry.path();
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(_) => {
                    add_warning(
                        warnings,
                        format!("{} source changed during inspection", root.id),
                    );
                    continue;
                }
            };
            if metadata.file_type().is_symlink() {
                add_warning(
                    warnings,
                    format!("{} source contains a skipped symlink", root.id),
                );
                continue;
            }
            let relative = path.strip_prefix(&root.path).unwrap_or(path.as_path());
            digest.update(relative.to_string_lossy().as_bytes());
            digest.update([0]);
            if metadata.is_dir() {
                digest.update(b"dir\0");
                queue.push_back(path);
                continue;
            }
            if !metadata.is_file() {
                add_warning(
                    warnings,
                    format!("{} source contains a skipped special entry", root.id),
                );
                continue;
            }
            file_count = file_count.saturating_add(1);
            total_bytes = total_bytes.saturating_add(metadata.len());
            digest.update(b"file\0");
            digest.update(metadata.len().to_be_bytes());
            if total_bytes > MAX_SCAN_BYTES {
                add_warning(
                    warnings,
                    format!("{} source reached its byte ceiling", root.id),
                );
                queue.clear();
                break;
            }
        }
    }
    if file_count == 0 {
        return Ok(None);
    }
    Ok(Some(LeftoverObservation {
        id: root.id,
        file_count,
        total_bytes,
        evidence_sha256: format!("{:x}", digest.finalize()),
    }))
}

fn observation_digest(observations: &[LeftoverObservation]) -> String {
    let mut digest = Sha256::new();
    for observation in observations {
        digest.update(observation.id.as_bytes());
        digest.update([0]);
        digest.update(observation.file_count.to_be_bytes());
        digest.update(observation.total_bytes.to_be_bytes());
        digest.update(observation.evidence_sha256.as_bytes());
        digest.update([0xff]);
    }
    format!("{:x}", digest.finalize())
}

fn add_warning(warnings: &mut Vec<String>, warning: String) {
    if warnings.len() < MAX_WARNINGS && !warnings.iter().any(|existing| existing == &warning) {
        warnings.push(warning);
    }
}

fn render_text(report: &LeftoversReviewReport) -> String {
    let finding_report = &report.finding_report;
    let mut out = format!("{} leftovers review\n\n", brand::TITLE);
    let _ = writeln!(out, "mode: dry-run read-only");
    let _ = writeln!(out, "contract: {}", report.contract);
    let _ = writeln!(out, "platform: {}", report.platform);
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
                ownership_label(finding.ownership),
                disposition_label(finding.disposition)
            );
        }
    }
    if !report.warnings.is_empty() {
        out.push_str("warnings:\n");
        for warning in &report.warnings {
            let _ = writeln!(out, "  - {warning}");
        }
    }
    out.push_str(
        "\nsafety: bounded runtime-owned directory evidence only; no files were deleted, moved, uploaded, or authorized for quarantine.\n",
    );
    out
}

fn usage() -> String {
    "Usage: rz0 leftovers --dry-run [--format text|json] [--fixture <leftover-input.json>]\n\nReports bounded runtime.zero-owned module/log evidence. Live mode never scans broad user paths; fixture mode reads one local contract document. No cleanup, quarantine, restore, or deletion path exists.\n".to_string()
}

fn disposition_label(disposition: rz0_finding_contract::FindingDisposition) -> &'static str {
    match disposition {
        rz0_finding_contract::FindingDisposition::ReportOnly => "report_only",
        rz0_finding_contract::FindingDisposition::ManagerActionCandidate => {
            "manager_action_candidate"
        }
        rz0_finding_contract::FindingDisposition::QuarantineCandidate => "quarantine_candidate",
        rz0_finding_contract::FindingDisposition::Ignore => "ignore",
        rz0_finding_contract::FindingDisposition::Blocked => "blocked",
    }
}

fn ownership_label(ownership: rz0_finding_contract::FindingOwnership) -> &'static str {
    match ownership {
        rz0_finding_contract::FindingOwnership::ManagerOwned => "manager_owned",
        rz0_finding_contract::FindingOwnership::RuntimeOwned => "runtime_owned",
        rz0_finding_contract::FindingOwnership::SystemOwned => "system_owned",
        rz0_finding_contract::FindingOwnership::UserOwned => "user_owned",
        rz0_finding_contract::FindingOwnership::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rz0_finding_contract::{FindingDataClass, FindingOwnership};

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn fixture_review_is_read_only_and_path_free() {
        let input = LeftoverFindingInput {
            schema_version: 1,
            contract: LEFTOVER_INPUT_CONTRACT.to_string(),
            platform: "test".to_string(),
            input_evidence_sha256: A.to_string(),
            source_id: "leftover.fixture".to_string(),
            source_evidence_sha256: A.to_string(),
            records: vec![LeftoverRecord {
                finding_id: "leftover.fixture".to_string(),
                subject_reference: "leftover:fixture".to_string(),
                ownership: FindingOwnership::RuntimeOwned,
                data_class: FindingDataClass::OrphanedData,
                exact_evidence: None,
            }],
        };
        let report = wrap_report(
            classify_leftovers(&input).expect("leftovers report"),
            vec![],
        );
        assert!(report.read_only);
        assert!(!report.writes_attempted);
        assert!(!report.raw_paths_included);
        assert!(!serde_json::to_string(&report).unwrap().contains("/Users/"));
    }

    #[test]
    fn leftovers_command_requires_explicit_dry_run() {
        let (code, _, error) = leftovers_command(&[]);
        assert_eq!(code, ExitCode::Usage);
        assert!(error.contains("dry-run only"));
    }
}
