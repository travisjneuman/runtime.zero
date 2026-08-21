use std::collections::VecDeque;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};

use rz0_action_plan::ActionPlan;
use rz0_finding_contract::FindingReport;
use rz0_module_cache::{
    CacheFindingInput, CacheRecord, INPUT_CONTRACT as CACHE_INPUT_CONTRACT, classify_caches,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    ExitCode, brand,
    exact_quarantine::{
        ExactQuarantineActionSpec, build_exact_quarantine_action_plan,
        build_exact_quarantine_challenge, execute_exact_quarantine,
        render_exact_quarantine_challenge, unix_seconds, validate_exact_quarantine_confirmation,
    },
    module_store::{ModuleStorePlan, module_store_plan},
};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_plan: Option<ActionPlan>,
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
    if options.apply {
        let Some(path) = options.path.as_deref() else {
            return (
                ExitCode::Usage,
                String::new(),
                format!("cache --apply requires --path\n\n{}", usage()),
            );
        };
        let (_, action_plan) = match exact_cache_plan(Path::new(path)) {
            Ok(value) => value,
            Err(error) => {
                return (
                    ExitCode::Usage,
                    String::new(),
                    format!("cache action plan failed closed: {error}\n"),
                );
            }
        };
        let issued = options
            .challenge_issued_unix_seconds
            .unwrap_or_else(unix_seconds);
        let challenge = match build_exact_quarantine_challenge(&action_plan, issued) {
            Ok(challenge) => challenge,
            Err(error) => {
                return (
                    ExitCode::Usage,
                    String::new(),
                    format!("cache confirmation challenge failed closed: {error}\n"),
                );
            }
        };
        let Some(phrase) = options.confirm.as_deref() else {
            return (
                ExitCode::Ok,
                render_exact_quarantine_challenge(
                    &challenge,
                    &action_plan.actions[0].action_id,
                    options.format == OutputFormat::Json,
                ),
                String::new(),
            );
        };
        let response =
            match validate_exact_quarantine_confirmation(&challenge, phrase, unix_seconds()) {
                Ok(response) => response,
                Err(error) => {
                    return (
                        ExitCode::Usage,
                        String::new(),
                        format!("cache confirmation rejected: {error}\n"),
                    );
                }
            };
        let store = module_store_plan(None, None, "cache exact plan");
        let effect = match execute_exact_quarantine(
            &store,
            &action_plan,
            &challenge,
            &response,
            unix_seconds(),
        ) {
            Ok(effect) => effect,
            Err(error) => {
                return (
                    ExitCode::Usage,
                    String::new(),
                    format!(
                        "cache filesystem effect failed closed [{:?}]: {error}\n",
                        error.code
                    ),
                );
            }
        };
        return render_effect(&effect, options.format);
    }
    let report = if options.plan {
        let Some(path) = options.path.as_deref() else {
            return (
                ExitCode::Usage,
                String::new(),
                format!("cache --plan requires --path\n\n{}", usage()),
            );
        };
        match exact_cache_plan(Path::new(path)) {
            Ok((finding_report, action_plan)) => {
                let mut report = wrap_report(
                    finding_report,
                    vec![
                        "exact path evidence was supplied explicitly; no broad live scan was used"
                            .to_string(),
                    ],
                );
                report.action_plan = Some(action_plan);
                Ok(report)
            }
            Err(error) => Err(error),
        }
    } else {
        match options.fixture.as_deref() {
            Some(path) => fixture_report(Path::new(path)),
            None => live_report(),
        }
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
    plan: bool,
    apply: bool,
    path: Option<String>,
    confirm: Option<String>,
    challenge_issued_unix_seconds: Option<u64>,
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut dry_run = false;
    let mut format = OutputFormat::Text;
    let mut fixture = None;
    let mut plan = false;
    let mut apply = false;
    let mut path = None;
    let mut confirm = None;
    let mut challenge_issued_unix_seconds = None;
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
            "--plan" if !plan => plan = true,
            "--plan" => return Err("cache --plan was provided more than once".to_string()),
            "--apply" if !apply => apply = true,
            "--apply" => return Err("cache --apply was provided more than once".to_string()),
            "--path" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("cache --path requires one absolute cache file path".to_string());
                };
                if path.replace(value.clone()).is_some() {
                    return Err("cache --path was provided more than once".to_string());
                }
                index += 1;
            }
            "--confirm" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("cache --confirm requires the exact challenge phrase".to_string());
                };
                if confirm.replace(value.clone()).is_some() {
                    return Err("cache --confirm was provided more than once".to_string());
                }
                index += 1;
            }
            "--challenge-issued-unix-seconds" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(
                        "cache --challenge-issued-unix-seconds requires an integer".to_string()
                    );
                };
                if challenge_issued_unix_seconds.is_some() {
                    return Err(
                        "cache --challenge-issued-unix-seconds was provided more than once"
                            .to_string(),
                    );
                }
                challenge_issued_unix_seconds = Some(value.parse().map_err(|_| {
                    "cache --challenge-issued-unix-seconds must be an integer".to_string()
                })?);
                index += 1;
            }
            value => return Err(format!("unsupported cache option '{value}'")),
        }
        index += 1;
    }
    if !dry_run && !apply {
        return Err("cache review is dry-run only; pass --dry-run".to_string());
    }
    if dry_run && apply {
        return Err("cache --dry-run and --apply are mutually exclusive".to_string());
    }
    if apply && path.is_none() {
        return Err("cache --apply requires --path".to_string());
    }
    if apply && plan {
        return Err("cache --apply and --plan are mutually exclusive".to_string());
    }
    if plan && path.is_none() {
        return Err("cache --plan requires --path".to_string());
    }
    if path.is_some() && fixture.is_some() {
        return Err("cache --plan cannot be combined with --fixture".to_string());
    }
    if !apply && (confirm.is_some() || challenge_issued_unix_seconds.is_some()) {
        return Err("cache confirmation options require --apply".to_string());
    }
    Ok(Options {
        format,
        fixture,
        plan,
        apply,
        path,
        confirm,
        challenge_issued_unix_seconds,
    })
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
        action_plan: None,
        warnings,
    }
}

fn exact_cache_plan(path: &Path) -> Result<(FindingReport, ActionPlan), String> {
    let store = module_store_plan(None, None, "cache exact plan");
    exact_cache_plan_for_store(&store, path)
}

fn exact_cache_plan_for_store(
    store: &ModuleStorePlan,
    path: &Path,
) -> Result<(FindingReport, ActionPlan), String> {
    let cache_root = PathBuf::from(&store.cache_root);
    if !path.is_absolute() {
        return Err("exact cache plan paths must be absolute".to_string());
    }
    let relative = path
        .strip_prefix(&cache_root)
        .map_err(|_| "exact cache plan path must remain inside the runtime.zero cache root")?;
    let logical_relative = logical_relative_path(relative)?;
    validate_no_symlinked_file(&cache_root, relative)?;
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("inspect exact cache path: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("exact cache plan path must be a regular non-symlink file".to_string());
    }
    if metadata.len() > rz0_action_plan::MAX_ACTION_SOURCE_BYTES {
        return Err("exact cache file exceeds the action source byte ceiling".to_string());
    }
    let bytes = fs::read(path).map_err(|error| format!("read exact cache path: {error}"))?;
    if bytes.len() as u64 != metadata.len() {
        return Err("exact cache file changed while reading".to_string());
    }
    let source_sha256 = format!("{:x}", Sha256::digest(&bytes));
    let source_path = format!("workspace/cache/{logical_relative}");
    let finding_id = format!("cache.exact.{}", &source_sha256[..24]);
    let source_evidence_sha256 =
        exact_evidence_digest(&source_path, &source_sha256, metadata.len());
    let input = CacheFindingInput {
        schema_version: 1,
        contract: CACHE_INPUT_CONTRACT.to_string(),
        platform: std::env::consts::OS.to_string(),
        input_evidence_sha256: source_evidence_sha256.clone(),
        source_id: "cache.exact-plan".to_string(),
        source_evidence_sha256,
        records: vec![CacheRecord {
            finding_id,
            subject_reference: "cache:runtime-zero-cache-artifact".to_string(),
            ownership: rz0_finding_contract::FindingOwnership::RuntimeOwned,
            exact_evidence: Some(rz0_finding_contract::ExactFindingEvidence {
                sha256: source_sha256.clone(),
                size_bytes: metadata.len(),
            }),
        }],
    };
    let finding_report = classify_caches(&input)?;
    let action_plan = build_exact_quarantine_action_plan(ExactQuarantineActionSpec {
        module_id: rz0_module_cache::MODULE_ID,
        target: "runtime.zero-owned cache artifact",
        source_path,
        source_sha256,
        source_size_bytes: metadata.len(),
        finding_report: &finding_report,
    })?;
    Ok((finding_report, action_plan))
}

fn logical_relative_path(path: &Path) -> Result<String, String> {
    let components = path
        .components()
        .map(|component| match component {
            std::path::Component::Normal(value) => value
                .to_str()
                .map(str::to_string)
                .ok_or_else(|| "exact cache path must use valid UTF-8 components".to_string()),
            _ => Err("exact cache path contains an unsafe component".to_string()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if components.is_empty() || components.len() > 32 {
        return Err("exact cache path is empty or too deep".to_string());
    }
    Ok(components.join("/"))
}

fn validate_no_symlinked_file(root: &Path, relative: &Path) -> Result<(), String> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let std::path::Component::Normal(name) = component else {
            return Err("exact cache path contains an unsafe component".to_string());
        };
        current.push(name);
        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| format!("inspect exact cache component: {error}"))?;
        if metadata.file_type().is_symlink() {
            return Err("exact cache plan refuses symlinked components".to_string());
        }
    }
    Ok(())
}

fn exact_evidence_digest(path: &str, sha256: &str, size_bytes: u64) -> String {
    let mut digest = Sha256::new();
    digest.update(b"runtime.zero.cache-exact-plan.v1\0");
    digest.update((path.len() as u64).to_be_bytes());
    digest.update(path.as_bytes());
    digest.update((sha256.len() as u64).to_be_bytes());
    digest.update(sha256.as_bytes());
    digest.update(size_bytes.to_be_bytes());
    format!("{:x}", digest.finalize())
}

fn render_effect(
    effect: &crate::quarantine::FilesystemEffectReport,
    format: OutputFormat,
) -> (ExitCode, String, String) {
    match format {
        OutputFormat::Text => (
            ExitCode::Ok,
            format!(
                "runtime.zero cache execution\n\ntransaction_id: {}\naction_id: {}\nstatus: {:?}\nsource_sha256: {}\nsource_size_bytes: {}\nsource_removed: {}\ndestination_verified: {}\nreceipt_reference: {}\nwrites_attempted: {}\nproduct_execution_authorized: {}\n",
                effect.transaction_id,
                effect.action_id,
                effect.status,
                effect.source_sha256,
                effect.source_size_bytes,
                effect.source_removed,
                effect.destination_verified,
                effect.receipt_reference,
                effect.writes_attempted,
                effect.product_execution_authorized,
            ),
            String::new(),
        ),
        OutputFormat::Json => match serde_json::to_string_pretty(effect) {
            Ok(json) => (ExitCode::Ok, format!("{json}\n"), String::new()),
            Err(error) => (
                ExitCode::Usage,
                String::new(),
                format!("cache execution JSON rendering failed: {error}\n"),
            ),
        },
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
    if let Some(action_plan) = report.action_plan.as_ref() {
        let _ = writeln!(out, "action_plan: {}", action_plan.plan_id);
        let _ = writeln!(out, "action_plan_actions: {}", action_plan.actions.len());
        out.push_str("action_plan_safety: dry-run only; no file was moved or authorized\n");
    }
    out.push_str(
        "\nsafety: bounded cache metadata only; no files were deleted, moved, uploaded, or authorized for cleanup in review mode.\n",
    );
    out
}

fn usage() -> String {
    "Usage: rz0 cache --dry-run [--format text|json] [--fixture <cache-input.json>]\n       rz0 cache --dry-run --plan --path <absolute-cache-file> [--format text|json]\n       rz0 cache --apply --path <absolute-cache-file> [--challenge-issued-unix-seconds <seconds>] [--confirm <exact-phrase>] [--format text|json]\n\nReports bounded ownership-aware cache evidence. The explicit plan/apply form is limited to one runtime.zero-owned cache file; it never deletes, recurses, elevates, or uses network access.\n".to_string()
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
    use std::sync::atomic::{AtomicU64, Ordering};

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use super::*;
    use rz0_action_plan::validate_action_plan;
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

    #[test]
    fn exact_runtime_cache_plan_is_sealed_and_path_redacted() {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "runtime-zero-cache-plan-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let cache_file = root.join("cache/first-party-cache/old-entry");
        fs::create_dir_all(cache_file.parent().expect("cache parent")).expect("cache root");
        fs::write(&cache_file, b"old cache\n").expect("cache entry");
        let store = crate::module_store::module_store_plan_for_data_root(
            root.clone(),
            None,
            None,
            "test exact cache plan",
        );
        let (finding_report, action_plan) =
            exact_cache_plan_for_store(&store, &cache_file).expect("exact cache plan");
        assert_eq!(finding_report.summary.quarantine_candidate_count, 1);
        assert!(validate_action_plan(&action_plan).valid);
        assert_eq!(
            action_plan.actions[0].source.as_ref().expect("source").path,
            "workspace/cache/first-party-cache/old-entry"
        );
        assert!(
            !serde_json::to_string(&action_plan)
                .expect("plan JSON")
                .contains(root.to_str().expect("root UTF-8"))
        );
        assert!(!action_plan.writes_attempted);
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn exact_runtime_cache_file_moves_only_to_foundation_quarantine() {
        static SEQUENCE: AtomicU64 = AtomicU64::new(1000);
        let root = std::env::temp_dir().join(format!(
            "runtime-zero-cache-apply-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let cache_file = root.join("cache/first-party-cache/old-entry");
        fs::create_dir_all(cache_file.parent().expect("cache parent")).expect("cache root");
        for directory in [
            root.clone(),
            root.join("cache"),
            root.join("cache/first-party-cache"),
            root.join("state"),
            root.join("state/transactions"),
            root.join("state/receipts"),
            root.join("quarantine"),
        ] {
            fs::create_dir_all(&directory).expect("effect directory");
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
                .expect("private effect directory");
        }
        fs::write(&cache_file, b"old cache\n").expect("cache entry");
        fs::set_permissions(&cache_file, fs::Permissions::from_mode(0o600))
            .expect("private cache entry");
        let store = crate::module_store::module_store_plan_for_data_root(
            root.clone(),
            None,
            None,
            "test exact cache apply",
        );
        let (_, action_plan) =
            exact_cache_plan_for_store(&store, &cache_file).expect("exact cache plan");
        let challenge = build_exact_quarantine_challenge(&action_plan, 1_000).expect("challenge");
        let response =
            validate_exact_quarantine_confirmation(&challenge, &challenge.expected_phrase, 1_100)
                .expect("confirmation");
        let effect = execute_exact_quarantine(&store, &action_plan, &challenge, &response, 1_100)
            .expect("exact cache effect");
        assert!(effect.source_removed);
        assert!(!cache_file.exists());
        assert!(
            root.join(format!("quarantine/{}/payload.bin", action_plan.plan_id))
                .is_file()
        );
        let _ = fs::remove_dir_all(root);
    }
}
