use std::collections::VecDeque;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};

use rz0_finding_contract::FindingReport;
use rz0_module_cache::{
    CacheFindingInput, CacheRecord, INPUT_CONTRACT as CACHE_INPUT_CONTRACT, classify_caches,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{ExitCode, brand, module_store::module_store_plan};

pub const CACHE_REVIEW_CONTRACT: &str = "cache_review";
const MAX_INPUT_BYTES: u64 = rz0_resource_contract::MAX_FINDING_REPORT_BYTES;
const MAX_SCAN_NODES: usize = 2_048;
const MAX_SCAN_BYTES: u64 = 64 * 1024 * 1024;
const MAX_WARNINGS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CacheReviewReport {
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
struct CacheRoot {
    id: &'static str,
    path: PathBuf,
    ownership: rz0_finding_contract::FindingOwnership,
}

#[derive(Debug, Clone)]
struct CacheObservation {
    id: &'static str,
    ownership: rz0_finding_contract::FindingOwnership,
    file_count: usize,
    total_bytes: u64,
    evidence_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

pub fn cache_command(args: &[String]) -> (ExitCode, String, String) {
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
                format!("cache review failed closed: {error}\n"),
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
                format!("cache review JSON rendering failed: {error}\n"),
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
            "--dry-run" => return Err("cache --dry-run was provided more than once".to_string()),
            "--json" => format = OutputFormat::Json,
            "--format" => {
                let Some(value) = args.get(index + 1).map(String::as_str) else {
                    return Err("cache --format requires text or json".to_string());
                };
                format = match value {
                    "text" => OutputFormat::Text,
                    "json" => OutputFormat::Json,
                    _ => return Err(format!("unsupported cache output format '{value}'")),
                };
                index += 1;
            }
            "--fixture" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("cache --fixture requires a local JSON path".to_string());
                };
                if fixture.replace(value.clone()).is_some() {
                    return Err("cache --fixture was provided more than once".to_string());
                }
                index += 1;
            }
            value => return Err(format!("unsupported cache option '{value}'")),
        }
        index += 1;
    }
    if !dry_run {
        return Err("cache review is dry-run only; pass --dry-run".to_string());
    }
    Ok(Options { format, fixture })
}

fn fixture_report(path: &Path) -> Result<CacheReviewReport, String> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "fixture cannot be read".to_string())?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > MAX_INPUT_BYTES
    {
        return Err("cache fixture must be a bounded regular non-symlink file".to_string());
    }
    let bytes = fs::read(path).map_err(|_| "cache fixture cannot be read".to_string())?;
    if bytes.len() as u64 != metadata.len() {
        return Err("cache fixture changed while reading".to_string());
    }
    let input: CacheFindingInput = serde_json::from_slice(&bytes)
        .map_err(|error| format!("cache fixture JSON is invalid: {error}"))?;
    let finding_report = classify_caches(&input)?;
    Ok(wrap_report(
        finding_report,
        vec!["fixture evidence was supplied; no live cache paths were inspected".to_string()],
    ))
}

pub fn live_report() -> Result<CacheReviewReport, String> {
    let mut warnings = Vec::new();
    let mut observations = Vec::new();
    for root in cache_roots() {
        if let Some(observation) = inspect_root(&root, &mut warnings)? {
            observations.push(observation);
        }
    }
    observations.sort_by(|left, right| left.id.cmp(right.id));
    let source_evidence_sha256 = observation_digest(&observations);
    let records = observations
        .iter()
        .map(|observation| CacheRecord {
            finding_id: format!("cache.{}", &observation.evidence_sha256[..16]),
            subject_reference: format!("cache:{}", observation.id),
            ownership: observation.ownership,
            // Directory listing evidence is intentionally not an action-ready
            // file identity. Future quarantine must bind an exact manifest.
            exact_evidence: None,
        })
        .collect::<Vec<_>>();
    let input = CacheFindingInput {
        schema_version: 1,
        contract: CACHE_INPUT_CONTRACT.to_string(),
        platform: std::env::consts::OS.to_string(),
        input_evidence_sha256: source_evidence_sha256.clone(),
        source_id: "cache.local".to_string(),
        source_evidence_sha256,
        records,
    };
    let finding_report = classify_caches(&input)?;
    Ok(wrap_report(finding_report, warnings))
}

fn wrap_report(finding_report: FindingReport, warnings: Vec<String>) -> CacheReviewReport {
    CacheReviewReport {
        schema_version: 1,
        contract: CACHE_REVIEW_CONTRACT,
        read_only: true,
        writes_attempted: false,
        raw_paths_included: false,
        platform: std::env::consts::OS,
        finding_report,
        warnings,
    }
}

fn cache_roots() -> Vec<CacheRoot> {
    let mut roots = Vec::new();
    let store_cache = PathBuf::from(module_store_plan(None, None, "cache review").cache_root);
    roots.push(CacheRoot {
        id: "runtime-zero",
        path: store_cache,
        ownership: rz0_finding_contract::FindingOwnership::RuntimeOwned,
    });

    let Some(home) = home_dir() else {
        return roots;
    };
    match std::env::consts::OS {
        "macos" => {
            roots.push(CacheRoot {
                id: "homebrew",
                path: home.join("Library/Caches/Homebrew"),
                ownership: rz0_finding_contract::FindingOwnership::ManagerOwned,
            });
            roots.push(CacheRoot {
                id: "npm",
                path: home.join(".npm"),
                ownership: rz0_finding_contract::FindingOwnership::ManagerOwned,
            });
            roots.push(CacheRoot {
                id: "pip",
                path: home.join("Library/Caches/pip"),
                ownership: rz0_finding_contract::FindingOwnership::ManagerOwned,
            });
            roots.push(CacheRoot {
                id: "cargo",
                path: home.join(".cargo/registry/cache"),
                ownership: rz0_finding_contract::FindingOwnership::ManagerOwned,
            });
        }
        "windows" => {
            if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
                roots.push(CacheRoot {
                    id: "npm",
                    path: local.join("npm-cache"),
                    ownership: rz0_finding_contract::FindingOwnership::ManagerOwned,
                });
                roots.push(CacheRoot {
                    id: "pip",
                    path: local.join("pip/Cache"),
                    ownership: rz0_finding_contract::FindingOwnership::ManagerOwned,
                });
            }
            roots.push(CacheRoot {
                id: "cargo",
                path: home.join(".cargo/registry/cache"),
                ownership: rz0_finding_contract::FindingOwnership::ManagerOwned,
            });
        }
        _ => {
            let cache_home = std::env::var_os("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".cache"));
            roots.push(CacheRoot {
                id: "npm",
                path: home.join(".npm"),
                ownership: rz0_finding_contract::FindingOwnership::ManagerOwned,
            });
            roots.push(CacheRoot {
                id: "pip",
                path: cache_home.join("pip"),
                ownership: rz0_finding_contract::FindingOwnership::ManagerOwned,
            });
            roots.push(CacheRoot {
                id: "cargo",
                path: home.join(".cargo/registry/cache"),
                ownership: rz0_finding_contract::FindingOwnership::ManagerOwned,
            });
        }
    }
    roots
}

fn inspect_root(
    root: &CacheRoot,
    warnings: &mut Vec<String>,
) -> Result<Option<CacheObservation>, String> {
    let metadata = match fs::symlink_metadata(&root.path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => {
            add_warning(
                warnings,
                format!("{} cache source could not be inspected", root.id),
            );
            return Ok(None);
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        add_warning(
            warnings,
            format!("{} cache source is not a regular directory", root.id),
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
                    format!("{} cache source has an unreadable directory", root.id),
                );
                continue;
            }
        };
        for entry in entries {
            node_count = node_count.saturating_add(1);
            if node_count > MAX_SCAN_NODES {
                add_warning(
                    warnings,
                    format!("{} cache source reached its scan ceiling", root.id),
                );
                queue.clear();
                break;
            }
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => {
                    add_warning(
                        warnings,
                        format!("{} cache source contains an unreadable entry", root.id),
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
                        format!("{} cache source changed during inspection", root.id),
                    );
                    continue;
                }
            };
            if metadata.file_type().is_symlink() {
                add_warning(
                    warnings,
                    format!("{} cache source contains a skipped symlink", root.id),
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
                    format!("{} cache source contains a skipped special entry", root.id),
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
                    format!("{} cache source reached its byte ceiling", root.id),
                );
                queue.clear();
                break;
            }
        }
    }
    if file_count == 0 {
        return Ok(None);
    }
    Ok(Some(CacheObservation {
        id: root.id,
        ownership: root.ownership,
        file_count,
        total_bytes,
        evidence_sha256: format!("{:x}", digest.finalize()),
    }))
}

fn observation_digest(observations: &[CacheObservation]) -> String {
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

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

fn render_text(report: &CacheReviewReport) -> String {
    let finding_report = &report.finding_report;
    let mut out = format!("{} cache review\n\n", brand::TITLE);
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
        "\nsafety: bounded cache metadata only; no cache contents were deleted, moved, uploaded, or authorized for cleanup.\n",
    );
    out
}

fn usage() -> String {
    "Usage: rz0 cache --dry-run [--format text|json] [--fixture <cache-input.json>]\n\nReports bounded ownership-aware cache evidence. Live mode inspects only known manager/runtime cache roots; fixture mode reads one local contract document. No cleanup, quarantine, restore, or deletion path exists.\n".to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    use rz0_finding_contract::FindingOwnership;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn fixture_input() -> CacheFindingInput {
        CacheFindingInput {
            schema_version: 1,
            contract: CACHE_INPUT_CONTRACT.to_string(),
            platform: "test".to_string(),
            input_evidence_sha256: A.to_string(),
            source_id: "cache.fixture".to_string(),
            source_evidence_sha256: A.to_string(),
            records: vec![CacheRecord {
                finding_id: "cache.fixture".to_string(),
                subject_reference: "cache:fixture".to_string(),
                ownership: FindingOwnership::ManagerOwned,
                exact_evidence: None,
            }],
        }
    }

    #[test]
    fn fixture_review_is_read_only_and_path_free() {
        let report = wrap_report(
            classify_caches(&fixture_input()).expect("cache report"),
            vec![],
        );
        assert!(report.read_only);
        assert!(!report.writes_attempted);
        assert!(!report.raw_paths_included);
        assert!(!serde_json::to_string(&report).unwrap().contains("/Users/"));
    }

    #[test]
    fn cache_command_requires_explicit_dry_run() {
        let (code, _, error) = cache_command(&[]);
        assert_eq!(code, ExitCode::Usage);
        assert!(error.contains("dry-run only"));
    }
}
