use std::collections::{BTreeSet, VecDeque};
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};

use rz0_action_plan::ActionPlan;
use rz0_finding_contract::FindingReport;
use rz0_module_leftovers::{
    INPUT_CONTRACT as LEFTOVER_INPUT_CONTRACT, LeftoverFindingInput, LeftoverRecord,
    classify_leftovers,
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
    installed_registry::{InstalledRegistryState, installed_registry_report},
    module_store::{ModuleStorePlan, REGISTRY_FILE, module_store_plan},
    quarantine::FilesystemEffectReport,
};

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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_plan: Option<ActionPlan>,
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
    let (mut report, action_plan) = if options.apply {
        let Some(path) = options.path.as_deref() else {
            return (
                ExitCode::Usage,
                String::new(),
                format!("leftovers --apply requires --path\n\n{}", usage()),
            );
        };
        let (_finding_report, action_plan) = match exact_quarantine_plan(Path::new(path)) {
            Ok(value) => value,
            Err(error) => {
                return (
                    ExitCode::Usage,
                    String::new(),
                    format!("leftovers action plan failed closed: {error}\n"),
                );
            }
        };
        let now = options
            .challenge_issued_unix_seconds
            .unwrap_or_else(unix_seconds);
        let challenge = match build_exact_quarantine_challenge(&action_plan, now) {
            Ok(challenge) => challenge,
            Err(error) => {
                return (
                    ExitCode::Usage,
                    String::new(),
                    format!("leftovers confirmation challenge failed closed: {error}\n"),
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
                        format!("leftovers confirmation rejected: {error}\n"),
                    );
                }
            };
        let store = module_store_plan(None, None, "leftovers exact plan");
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
                        "leftovers filesystem effect failed closed [{:?}]: {error}\n",
                        error.code
                    ),
                );
            }
        };
        return render_leftovers_effect(&effect, options.format);
    } else if options.plan {
        let Some(path) = options.path.as_deref() else {
            return (
                ExitCode::Usage,
                String::new(),
                format!("leftovers --plan requires --path\n\n{}", usage()),
            );
        };
        match exact_quarantine_plan(Path::new(path)) {
            Ok((finding_report, action_plan)) => (
                wrap_report(
                    finding_report,
                    vec![
                        "exact path evidence was supplied explicitly; no broad live scan was used"
                            .to_string(),
                    ],
                ),
                Some(action_plan),
            ),
            Err(error) => {
                return (
                    ExitCode::Usage,
                    String::new(),
                    format!("leftovers action plan failed closed: {error}\n"),
                );
            }
        }
    } else {
        if options.path.is_some() {
            return (
                ExitCode::Usage,
                String::new(),
                format!("leftovers --path requires --plan\n\n{}", usage()),
            );
        }
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
        (report, None)
    };
    report.action_plan = action_plan;
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
            "--plan" if !plan => plan = true,
            "--plan" => return Err("leftovers --plan was provided more than once".to_string()),
            "--apply" if !apply => apply = true,
            "--apply" => return Err("leftovers --apply was provided more than once".to_string()),
            "--path" => {
                let Some(value) = args.get(index + 1) else {
                    return Err("leftovers --path requires one absolute file path".to_string());
                };
                if path.replace(value.clone()).is_some() {
                    return Err("leftovers --path was provided more than once".to_string());
                }
                index += 1;
            }
            "--confirm" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(
                        "leftovers --confirm requires the exact challenge phrase".to_string()
                    );
                };
                if confirm.replace(value.clone()).is_some() {
                    return Err("leftovers --confirm was provided more than once".to_string());
                }
                index += 1;
            }
            "--challenge-issued-unix-seconds" => {
                let Some(value) = args.get(index + 1) else {
                    return Err(
                        "leftovers --challenge-issued-unix-seconds requires an integer".to_string(),
                    );
                };
                if challenge_issued_unix_seconds.is_some() {
                    return Err(
                        "leftovers --challenge-issued-unix-seconds was provided more than once"
                            .to_string(),
                    );
                }
                challenge_issued_unix_seconds = Some(value.parse().map_err(|_| {
                    "leftovers --challenge-issued-unix-seconds must be an integer".to_string()
                })?);
                index += 1;
            }
            value => return Err(format!("unsupported leftovers option '{value}'")),
        }
        index += 1;
    }
    if !dry_run && !apply {
        return Err("leftovers review is dry-run only; pass --dry-run".to_string());
    }
    if dry_run && apply {
        return Err("leftovers --dry-run and --apply are mutually exclusive".to_string());
    }
    if apply && path.is_none() {
        return Err("leftovers --apply requires --path".to_string());
    }
    if apply && plan {
        return Err("leftovers --apply and --plan are mutually exclusive".to_string());
    }
    if !apply && confirm.is_some() {
        return Err("leftovers --confirm requires --apply".to_string());
    }
    if !apply && challenge_issued_unix_seconds.is_some() {
        return Err("leftovers --challenge-issued-unix-seconds requires --apply".to_string());
    }
    if plan && path.is_none() {
        return Err("leftovers --plan requires --path".to_string());
    }
    if path.is_some() && fixture.is_some() {
        return Err("leftovers --plan cannot be combined with --fixture".to_string());
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
    if let Some(observation) =
        inspect_unreferenced_receipts(Path::new(&plan.state_root), &mut warnings)?
    {
        observations.push(observation);
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
        action_plan: None,
        warnings,
    }
}

fn exact_quarantine_plan(path: &Path) -> Result<(FindingReport, ActionPlan), String> {
    let store = module_store_plan(None, None, "leftovers exact plan");
    exact_quarantine_plan_for_store(&store, path)
}

fn exact_quarantine_plan_for_store(
    store: &ModuleStorePlan,
    path: &Path,
) -> Result<(FindingReport, ActionPlan), String> {
    let data_root = PathBuf::from(&store.data_root);
    let modules_root = PathBuf::from(&store.modules_root);
    if !path.is_absolute() {
        return Err("exact leftovers plan paths must be absolute".to_string());
    }
    let _data_relative = path
        .strip_prefix(&data_root)
        .map_err(|_| "exact leftovers plan path must remain inside the runtime.zero data root")?;
    let module_relative = path.strip_prefix(&modules_root).map_err(
        |_| "exact leftovers plan path must remain inside the runtime.zero module store",
    )?;
    let logical_relative = logical_relative_path(module_relative)?;
    validate_no_symlinked_file(&modules_root, module_relative)?;
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("inspect exact leftover path: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("exact leftovers plan path must be a regular non-symlink file".to_string());
    }
    if metadata.len() > rz0_action_plan::MAX_ACTION_SOURCE_BYTES {
        return Err("exact leftover file exceeds the action source byte ceiling".to_string());
    }
    let bytes = fs::read(path).map_err(|error| format!("read exact leftover path: {error}"))?;
    if bytes.len() as u64 != metadata.len() {
        return Err("exact leftover file changed while reading".to_string());
    }
    let source_sha256 = format!("{:x}", Sha256::digest(&bytes));
    let source_path = format!("workspace/modules/{logical_relative}");
    let finding_id = format!("leftover.exact.{}", &source_sha256[..24]);
    let source_evidence_sha256 =
        exact_evidence_digest(&source_path, &source_sha256, metadata.len());
    let input = LeftoverFindingInput {
        schema_version: 1,
        contract: LEFTOVER_INPUT_CONTRACT.to_string(),
        platform: std::env::consts::OS.to_string(),
        input_evidence_sha256: source_evidence_sha256.clone(),
        source_id: "leftovers.exact-plan".to_string(),
        source_evidence_sha256: source_evidence_sha256.clone(),
        records: vec![LeftoverRecord {
            finding_id: finding_id.clone(),
            subject_reference: "leftover:runtime-zero-module-artifact".to_string(),
            ownership: rz0_finding_contract::FindingOwnership::RuntimeOwned,
            data_class: rz0_finding_contract::FindingDataClass::OrphanedData,
            exact_evidence: Some(rz0_finding_contract::ExactFindingEvidence {
                sha256: source_sha256.clone(),
                size_bytes: metadata.len(),
            }),
        }],
    };
    let finding_report = classify_leftovers(&input)?;
    let action_plan = build_exact_quarantine_action_plan(ExactQuarantineActionSpec {
        module_id: rz0_module_leftovers::MODULE_ID,
        target: "runtime.zero-owned module artifact",
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
            std::path::Component::Normal(value) => {
                value.to_str().map(str::to_string).ok_or_else(|| {
                    "exact leftovers plan path must use valid UTF-8 components".to_string()
                })
            }
            _ => Err("exact leftovers plan path contains an unsafe component".to_string()),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if components.is_empty() || components.len() > 32 {
        return Err("exact leftovers plan path is empty or too deep".to_string());
    }
    Ok(components.join("/"))
}

fn validate_no_symlinked_file(root: &Path, relative: &Path) -> Result<(), String> {
    let mut current = root.to_path_buf();
    for component in relative.components() {
        let std::path::Component::Normal(name) = component else {
            return Err("exact leftovers plan path contains an unsafe component".to_string());
        };
        current.push(name);
        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| format!("inspect exact leftover component: {error}"))?;
        if metadata.file_type().is_symlink() {
            return Err("exact leftovers plan refuses symlinked components".to_string());
        }
    }
    Ok(())
}

fn exact_evidence_digest(path: &str, sha256: &str, size_bytes: u64) -> String {
    let mut digest = Sha256::new();
    digest.update(b"runtime.zero.leftovers-exact-plan.v1\0");
    digest.update((path.len() as u64).to_be_bytes());
    digest.update(path.as_bytes());
    digest.update((sha256.len() as u64).to_be_bytes());
    digest.update(sha256.as_bytes());
    digest.update(size_bytes.to_be_bytes());
    format!("{:x}", digest.finalize())
}

fn render_leftovers_effect(
    effect: &FilesystemEffectReport,
    format: OutputFormat,
) -> (ExitCode, String, String) {
    match format {
        OutputFormat::Text => (
            ExitCode::Ok,
            format!(
                "runtime.zero leftovers execution\n\ntransaction_id: {}\naction_id: {}\nstatus: {:?}\nsource_sha256: {}\nsource_size_bytes: {}\nsource_removed: {}\ndestination_verified: {}\nreceipt_reference: {}\nwrites_attempted: {}\nproduct_execution_authorized: {}\n",
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
                format!("leftovers execution JSON rendering failed: {error}\n"),
            ),
        },
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

fn inspect_unreferenced_receipts(
    state_root: &Path,
    warnings: &mut Vec<String>,
) -> Result<Option<LeftoverObservation>, String> {
    let registry = installed_registry_report(&state_root.join(REGISTRY_FILE));
    let referenced = match registry.status {
        InstalledRegistryState::Absent | InstalledRegistryState::Empty => BTreeSet::new(),
        InstalledRegistryState::Valid => registry
            .records
            .iter()
            .filter(|record| record.valid)
            .map(|record| record.receipt_path.clone())
            .collect::<BTreeSet<_>>(),
        InstalledRegistryState::Invalid | InstalledRegistryState::Unreadable => {
            add_warning(
                warnings,
                "runtime-zero receipt ownership could not be checked because the installed-module registry is not valid".to_string(),
            );
            return Ok(None);
        }
    };
    let receipts_root = state_root.join("receipts");
    let metadata = match fs::symlink_metadata(&receipts_root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => {
            add_warning(
                warnings,
                "runtime-zero receipt source could not be inspected".to_string(),
            );
            return Ok(None);
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        add_warning(
            warnings,
            "runtime-zero receipt source is not a regular directory".to_string(),
        );
        return Ok(None);
    }

    let mut node_count = 0usize;
    let mut file_count = 0usize;
    let mut total_bytes = 0u64;
    let mut digest = Sha256::new();
    for entry in fs::read_dir(&receipts_root)
        .map_err(|_| "runtime-zero receipt source could not be read".to_string())?
    {
        node_count = node_count.saturating_add(1);
        if node_count > MAX_SCAN_NODES {
            add_warning(
                warnings,
                "runtime-zero receipt source reached its scan ceiling".to_string(),
            );
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                add_warning(
                    warnings,
                    "runtime-zero receipt source contains an unreadable entry".to_string(),
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
                    "runtime-zero receipt source changed during inspection".to_string(),
                );
                continue;
            }
        };
        if metadata.file_type().is_symlink() {
            add_warning(
                warnings,
                "runtime-zero receipt source contains a skipped symlink".to_string(),
            );
            continue;
        }
        if !metadata.is_file()
            || path.extension().and_then(|extension| extension.to_str()) != Some("json")
        {
            continue;
        }
        let relative = format!("receipts/{}", entry.file_name().to_string_lossy());
        if referenced.contains(&relative) {
            continue;
        }
        file_count = file_count.saturating_add(1);
        total_bytes = total_bytes.saturating_add(metadata.len());
        digest.update(relative.as_bytes());
        digest.update([0]);
        digest.update(metadata.len().to_be_bytes());
        if total_bytes > MAX_SCAN_BYTES {
            add_warning(
                warnings,
                "runtime-zero receipt source reached its byte ceiling".to_string(),
            );
            break;
        }
    }
    if file_count == 0 {
        return Ok(None);
    }
    Ok(Some(LeftoverObservation {
        id: "runtime-zero-unreferenced-receipts",
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
    if let Some(action_plan) = report.action_plan.as_ref() {
        let _ = writeln!(out, "action_plan: {}", action_plan.plan_id);
        let _ = writeln!(out, "action_plan_actions: {}", action_plan.actions.len());
        out.push_str("action_plan_safety: dry-run only; no file was moved or authorized\n");
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
    "Usage: rz0 leftovers --dry-run [--format text|json] [--fixture <leftover-input.json>]\n       rz0 leftovers --dry-run --plan --path <absolute-module-file> [--format text|json]\n       rz0 leftovers --apply --path <absolute-module-file> [--challenge-issued-unix-seconds <seconds>] [--confirm <exact-phrase>] [--format text|json]\n\nReports bounded runtime.zero-owned module/log and unreferenced-receipt evidence. The explicit plan form seals one exact runtime-owned module-file quarantine plan. Apply is a separate confirmation-bound quarantine lane; it never deletes, recurses, elevates, or uses network access.\n".to_string()
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
    use std::sync::atomic::{AtomicU64, Ordering};

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use super::*;
    use rz0_action_plan::validate_action_plan;
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

    #[test]
    fn exact_module_file_plan_is_sealed_path_free_and_non_authorizing() {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "runtime-zero-leftovers-plan-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let module_file = root.join("modules/first-party.leftovers/0.1.0/old-shim");
        fs::create_dir_all(module_file.parent().expect("module parent")).expect("module root");
        fs::write(&module_file, b"old shim\n").expect("module artifact");
        let store = crate::module_store::module_store_plan_for_data_root(
            root.clone(),
            None,
            None,
            "test exact plan",
        );

        let (finding_report, action_plan) =
            exact_quarantine_plan_for_store(&store, &module_file).expect("exact plan");
        let validation = validate_action_plan(&action_plan);
        assert!(validation.valid, "{:?}", validation.errors);
        assert_eq!(finding_report.summary.quarantine_candidate_count, 1);
        assert_eq!(action_plan.evidence_report_id, finding_report.report_id);
        assert_eq!(action_plan.actions.len(), 1);
        assert_eq!(
            action_plan.actions[0].source.as_ref().expect("source").path,
            "workspace/modules/first-party.leftovers/0.1.0/old-shim"
        );
        assert!(
            !serde_json::to_string(&action_plan)
                .expect("plan JSON")
                .contains(root.to_str().expect("root UTF-8"))
        );
        assert!(!action_plan.writes_attempted);
        assert!(action_plan.actions[0].requires_confirmation);
        let challenge = build_exact_quarantine_challenge(&action_plan, 1_000).expect("challenge");
        assert_eq!(challenge.action_count, 1);
        assert!(!challenge.dry_run_writes_attempted);
        assert!(!challenge.manual_recovery_acknowledged);
        let response =
            validate_exact_quarantine_confirmation(&challenge, &challenge.expected_phrase, 1_100)
                .expect("confirmation");
        assert_eq!(response.challenge_sha256, challenge.challenge_sha256);
        assert!(!response.execution_authorized);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn exact_plan_refuses_log_and_outside_paths() {
        static SEQUENCE: AtomicU64 = AtomicU64::new(1000);
        let root = std::env::temp_dir().join(format!(
            "runtime-zero-leftovers-plan-reject-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let outside = root.join("logs/not-a-module.log");
        fs::create_dir_all(outside.parent().expect("log parent")).expect("log root");
        fs::write(&outside, b"log\n").expect("log");
        let store = crate::module_store::module_store_plan_for_data_root(
            root.clone(),
            None,
            None,
            "test exact reject",
        );
        let error = exact_quarantine_plan_for_store(&store, &outside).expect_err("log rejection");
        assert!(error.contains("module store"));
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn exact_plan_binds_module_store_path_to_foundation_quarantine() {
        static SEQUENCE: AtomicU64 = AtomicU64::new(2000);
        let root = std::env::temp_dir().join(format!(
            "runtime-zero-leftovers-apply-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let module_file = root.join("modules/first-party.leftovers/0.1.0/old-shim");
        fs::create_dir_all(module_file.parent().expect("module parent")).expect("module root");
        for directory in [
            root.clone(),
            root.join("modules"),
            root.join("modules/first-party.leftovers"),
            root.join("modules/first-party.leftovers/0.1.0"),
            root.join("state"),
            root.join("state/transactions"),
            root.join("state/receipts"),
            root.join("quarantine"),
        ] {
            fs::create_dir_all(&directory).expect("effect directory");
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
                .expect("private effect directory");
        }
        fs::write(&module_file, b"old shim\n").expect("module artifact");
        fs::set_permissions(&module_file, fs::Permissions::from_mode(0o600))
            .expect("private module artifact");
        let store = crate::module_store::module_store_plan_for_data_root(
            root.clone(),
            None,
            None,
            "test exact apply",
        );
        let (_, action_plan) =
            exact_quarantine_plan_for_store(&store, &module_file).expect("exact plan");
        let challenge = build_exact_quarantine_challenge(&action_plan, 1_000).expect("challenge");
        let response =
            validate_exact_quarantine_confirmation(&challenge, &challenge.expected_phrase, 1_100)
                .expect("confirmation");
        let effect = execute_exact_quarantine(&store, &action_plan, &challenge, &response, 1_100)
            .expect("exact quarantine effect");
        assert!(effect.source_removed);
        assert!(!module_file.exists());
        assert!(
            root.join(format!("quarantine/{}/payload.bin", action_plan.plan_id))
                .is_file()
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn unreferenced_receipt_review_is_bounded_and_path_free() {
        let root = std::env::temp_dir().join(format!(
            "runtime-zero-leftovers-receipts-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("receipts")).expect("receipt root");
        fs::write(
            root.join(REGISTRY_FILE),
            r#"{"schema_version":1,"modules":[{"id":"first-party.inventory","version":"0.1.0","manifest_path":"modules/first-party.inventory/0.1.0/rz0-module.json","receipt_path":"receipts/referenced.json","module_dir":"modules/first-party.inventory/0.1.0"}]}"#,
        )
        .expect("registry");
        fs::write(root.join("receipts/referenced.json"), b"referenced").expect("referenced");
        fs::write(root.join("receipts/orphan.json"), b"orphan").expect("orphan");
        let mut warnings = Vec::new();
        let observation = inspect_unreferenced_receipts(&root, &mut warnings)
            .expect("receipt review")
            .expect("unreferenced receipt observation");
        assert_eq!(observation.id, "runtime-zero-unreferenced-receipts");
        assert_eq!(observation.file_count, 1);
        assert_eq!(observation.total_bytes, 6);
        assert!(warnings.is_empty());
        assert!(!observation.evidence_sha256.contains("orphan"));
        let _ = fs::remove_dir_all(root);
    }
}
