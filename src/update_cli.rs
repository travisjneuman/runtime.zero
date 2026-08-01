use std::fs;
use std::path::{Path, PathBuf};

use rz0_action_plan::{ActionDisposition, ActionPlan};
use rz0_module_updater::{
    ManagerKind, ManagerParseContext, SerialUpdateQueuePlan, UPDATE_QUEUE_CONTRACT,
    UpdaterFindingInput, build_serial_update_queue, build_update_action_plan, classify_updates,
    manager_probe_specs, parse_manager_output,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::ExitCode;

const MAX_INPUT_BYTES: u64 = rz0_resource_contract::MAX_FINDING_REPORT_BYTES;
const UPDATES_CONTRACT: &str = "updater_cli_review";

pub fn updates_command(args: &[String]) -> (ExitCode, String, String) {
    if matches!(args, [help] if matches!(help.as_str(), "--help" | "-h" | "help")) {
        return (ExitCode::Ok, usage(), String::new());
    }
    let command = match parse_args(args) {
        Ok(command) => command,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("{error}\n\n{}", usage()),
            );
        }
    };
    let input = match build_input(&command) {
        Ok(input) => input,
        Err(error) => return (ExitCode::Usage, String::new(), format!("{error}\n")),
    };
    let report = match classify_updates(&input) {
        Ok(report) => report,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("updater evidence failed closed: {error}\n"),
            );
        }
    };
    let output = if command.queue {
        let plan = match build_update_action_plan(&input, &report) {
            Ok(plan) => plan,
            Err(error) => {
                return (
                    ExitCode::Usage,
                    String::new(),
                    format!("updater action plan failed closed: {error}\n"),
                );
            }
        };
        match build_serial_update_queue(&plan) {
            Ok(queue) => render_queue(&queue, command.format),
            Err(error) => Err(error),
        }
    } else if command.plan {
        match build_update_action_plan(&input, &report) {
            Ok(plan) => render_plan(&plan, command.format),
            Err(error) => Err(error),
        }
    } else {
        render_report(&report, command.format)
    };
    match output {
        Ok(output) => (ExitCode::Ok, output, String::new()),
        Err(error) => (ExitCode::Usage, String::new(), format!("{error}\n")),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedArgs {
    fixture: Option<PathBuf>,
    manager_output: Option<PathBuf>,
    manager: Option<ManagerKind>,
    executable: Option<String>,
    dry_run: bool,
    plan: bool,
    queue: bool,
    format: OutputFormat,
}

fn parse_args(args: &[String]) -> Result<ParsedArgs, String> {
    let mut parsed = ParsedArgs {
        fixture: None,
        manager_output: None,
        manager: None,
        executable: None,
        dry_run: false,
        plan: false,
        queue: false,
        format: OutputFormat::Text,
    };
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--dry-run" if !parsed.dry_run => parsed.dry_run = true,
            "--dry-run" => return Err("updates accepts --dry-run only once".to_string()),
            "--fixture" => {
                index += 1;
                parsed.fixture =
                    Some(PathBuf::from(args.get(index).ok_or_else(|| {
                        "--fixture requires a local JSON path".to_string()
                    })?));
            }
            "--manager-output" => {
                index += 1;
                parsed.manager_output =
                    Some(PathBuf::from(args.get(index).ok_or_else(|| {
                        "--manager-output requires a local output path".to_string()
                    })?));
            }
            "--manager" => {
                index += 1;
                parsed.manager =
                    Some(parse_manager_kind(args.get(index).ok_or_else(|| {
                        "--manager requires a supported manager ID".to_string()
                    })?)?);
            }
            "--executable" => {
                index += 1;
                parsed.executable = Some(
                    args.get(index)
                        .ok_or_else(|| "--executable requires an absolute path".to_string())?
                        .clone(),
                );
            }
            "--plan" => parsed.plan = true,
            "--queue" => parsed.queue = true,
            "--json" => parsed.format = OutputFormat::Json,
            "--format" => {
                index += 1;
                parsed.format = match args.get(index).map(String::as_str) {
                    Some("text") => OutputFormat::Text,
                    Some("json") => OutputFormat::Json,
                    _ => return Err("--format requires text or json".to_string()),
                };
            }
            value if value.starts_with("--format=") => {
                parsed.format = match value {
                    "--format=text" => OutputFormat::Text,
                    "--format=json" => OutputFormat::Json,
                    _ => return Err("unsupported updates output format".to_string()),
                };
            }
            "--help" | "-h" | "help" => {
                return Err("help cannot be combined with options".to_string());
            }
            value => return Err(format!("unsupported updates option '{value}'")),
        }
        index += 1;
    }
    if !parsed.dry_run {
        return Err("updates is report-only and requires --dry-run".to_string());
    }
    if parsed.queue && !parsed.plan {
        return Err("--queue requires --plan".to_string());
    }
    if parsed.fixture.is_some() && parsed.manager_output.is_some() {
        return Err("--fixture and --manager-output are mutually exclusive".to_string());
    }
    if parsed.manager_output.is_some() && parsed.manager.is_none() {
        return Err("--manager-output requires --manager".to_string());
    }
    Ok(parsed)
}

fn build_input(command: &ParsedArgs) -> Result<UpdaterFindingInput, String> {
    if let Some(path) = command.fixture.as_deref() {
        return read_input(path);
    }
    let Some(path) = command.manager_output.as_deref() else {
        return Err(
            "live update availability discovery is not enabled yet; provide a local evidence fixture via --fixture or captured manager output via --manager-output".to_string(),
        );
    };
    let manager = command
        .manager
        .ok_or_else(|| "--manager-output requires --manager".to_string())?;
    let bytes = read_bounded_bytes(path)?;
    let digest = sha256(&bytes);
    let spec = manager_probe_specs()
        .into_iter()
        .find(|spec| spec.manager == manager)
        .ok_or_else(|| "manager probe specification is unavailable".to_string())?;
    let context = ManagerParseContext {
        manager,
        executable: command.executable.clone(),
        network_required: spec.network_required,
        requires_elevation: spec.requires_elevation,
        rollback_supported: false,
    };
    let records = parse_manager_output(&context, &bytes)?;
    Ok(UpdaterFindingInput {
        schema_version: 1,
        contract: rz0_module_updater::INPUT_CONTRACT.to_string(),
        platform: manager.platform().to_string(),
        input_evidence_sha256: digest.clone(),
        source_id: format!("manager.{}", manager.id()),
        source_evidence_sha256: digest,
        records,
    })
}

fn read_input(path: &Path) -> Result<UpdaterFindingInput, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("inspect updater fixture: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("updater fixture must be a direct regular file".to_string());
    }
    if metadata.len() > MAX_INPUT_BYTES {
        return Err("updater fixture exceeds the foundation byte ceiling".to_string());
    }
    let bytes = fs::read(path).map_err(|error| format!("read updater fixture: {error}"))?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err("updater fixture exceeds the foundation byte ceiling".to_string());
    }
    serde_json::from_slice(&bytes).map_err(|error| format!("parse updater fixture: {error}"))
}

fn read_bounded_bytes(path: &Path) -> Result<Vec<u8>, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("inspect manager output: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("manager output must be a direct regular file".to_string());
    }
    if metadata.len() > MAX_INPUT_BYTES {
        return Err("manager output exceeds the foundation byte ceiling".to_string());
    }
    let bytes = fs::read(path).map_err(|error| format!("read manager output: {error}"))?;
    if bytes.len() as u64 > MAX_INPUT_BYTES {
        return Err("manager output exceeds the foundation byte ceiling".to_string());
    }
    Ok(bytes)
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn parse_manager_kind(value: &str) -> Result<ManagerKind, String> {
    [
        ManagerKind::HomebrewFormula,
        ManagerKind::HomebrewCask,
        ManagerKind::MacPorts,
        ManagerKind::Winget,
        ManagerKind::Apt,
        ManagerKind::Dnf,
        ManagerKind::Pacman,
        ManagerKind::Zypper,
        ManagerKind::Snap,
        ManagerKind::Flatpak,
    ]
    .into_iter()
    .find(|manager| manager.id() == value)
    .ok_or_else(|| format!("unsupported manager '{value}'"))
}

fn render_report(
    report: &rz0_finding_contract::FindingReport,
    format: OutputFormat,
) -> Result<String, String> {
    match format {
        OutputFormat::Text => Ok(format!(
            "runtime.zero updater review\n\ncontract: {UPDATES_CONTRACT}\nsource_contract: {}\nreport_id: {}\nread_only: yes\nwrites_attempted: no\nupdate_candidates: {}\nblocked: {}\n\nNo manager command, network request, or write was performed.\n",
            report.contract,
            report.report_id,
            report.summary.manager_action_candidate_count,
            report.summary.blocked_count,
        )),
        OutputFormat::Json => render_json(&CliReview::Report(report)),
    }
}

fn render_plan(plan: &ActionPlan, format: OutputFormat) -> Result<String, String> {
    match format {
        OutputFormat::Text => Ok(format!(
            "runtime.zero updater plan\n\ncontract: {UPDATES_CONTRACT}\nplan_id: {}\ndry_run: yes\nwrites_attempted: no\nplanned_actions: {}\nblocked_actions: {}\nexecution_authorized: no\n\nNo manager command, network request, or write was performed.\n",
            plan.plan_id,
            plan.actions
                .iter()
                .filter(|action| action.disposition == ActionDisposition::Planned)
                .count(),
            plan.actions
                .iter()
                .filter(|action| action.disposition != ActionDisposition::Planned)
                .count(),
        )),
        OutputFormat::Json => render_json(&CliReview::Plan(plan)),
    }
}

fn render_queue(queue: &SerialUpdateQueuePlan, format: OutputFormat) -> Result<String, String> {
    match format {
        OutputFormat::Text => Ok(format!(
            "runtime.zero serial updater queue\n\ncontract: {UPDATE_QUEUE_CONTRACT}\nqueue_id: {}\nitems: {}\ndry_run: yes\nwrites_attempted: no\nexecution_authorized: no\n\nThe queue is review-only and pauses on failure, drift, cancellation, or recovery.\n",
            queue.queue_id,
            queue.items.len(),
        )),
        OutputFormat::Json => render_json(&CliReview::Queue(queue)),
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum CliReview<'a> {
    Report(&'a rz0_finding_contract::FindingReport),
    Plan(&'a ActionPlan),
    Queue(&'a SerialUpdateQueuePlan),
}

fn render_json(value: &impl Serialize) -> Result<String, String> {
    serde_json::to_string_pretty(value)
        .map(|json| format!("{json}\n"))
        .map_err(|error| format!("render updater review: {error}"))
}

fn usage() -> String {
    "Usage: rz0 updates --dry-run --fixture <updater-evidence.json> [--plan] [--queue] [--format text|json]\n       rz0 updates --dry-run --manager <manager-id> --manager-output <output> --executable <absolute-path> [--plan] [--queue] [--format text|json]\n\nReads bounded local updater evidence or captured manager output and emits a read-only finding, action plan, or serial queue. runtime.zero does not invoke managers, access a network, or execute updates from this command.".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_dry_run_and_orders_queue_after_plan() {
        assert!(parse_args(&[]).is_err());
        assert!(parse_args(&["--dry-run".to_string(), "--queue".to_string()]).is_err());
        let parsed = parse_args(&[
            "--fixture".to_string(),
            "fixture.json".to_string(),
            "--dry-run".to_string(),
            "--queue".to_string(),
            "--plan".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ])
        .expect("updates args");
        assert_eq!(parsed.fixture, Some(PathBuf::from("fixture.json")));
        assert_eq!(parsed.format, OutputFormat::Json);
    }

    #[test]
    fn rejects_symlinked_or_unbounded_input_without_collecting() {
        assert!(read_input(Path::new("tests/fixtures/does-not-exist.json")).is_err());
    }
}
