use std::fs;
use std::io::{self, Cursor, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};

use rz0_action_plan::{ActionDisposition, ActionPlan};
use rz0_module_updater::{
    ManagerKind, ManagerParseContext, ProviderProbeSpec, SerialUpdateItemStatus,
    SerialUpdateQueuePlan, UPDATE_QUEUE_CONTRACT, UpdaterFindingInput, build_serial_update_queue,
    build_update_action_plan, classify_updates, discover_provider_specs_for_platform,
    dynamic_provider_ids, manager_probe_specs, parse_manager_output,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::ExitCode;
use crate::module_store::module_store_plan;
use crate::update_execution::{
    UpdateChallengeView, UpdateExecutionReport, UpdateExecutionRequest, build_update_challenge,
    execute_update_action, make_single_action_plan, observe_manager_executable,
    validate_update_confirmation,
};

const MAX_INPUT_BYTES: u64 = rz0_resource_contract::MAX_FINDING_REPORT_BYTES;
const MAX_PROVIDER_WARNINGS: usize = rz0_resource_contract::MAX_INVENTORY_WARNINGS;
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
    if command.recovery_status {
        return recovery_status_command(&command);
    }
    let built_input = match build_input(&command) {
        Ok(input) => input,
        Err(error) => return (ExitCode::Usage, String::new(), format!("{error}\n")),
    };
    let input = &built_input.input;
    let report = match classify_updates(input) {
        Ok(report) => report,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("updater evidence failed closed: {error}\n"),
            );
        }
    };
    if command.apply {
        return apply_update_command(&command, input, &report);
    }
    let output = if command.queue {
        let plan = match build_update_action_plan(input, &report) {
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
            Ok(queue) => render_queue(&queue, command.format, &built_input),
            Err(error) => Err(error),
        }
    } else if command.plan {
        match build_update_action_plan(input, &report) {
            Ok(plan) => render_plan(&plan, command.format, &built_input),
            Err(error) => Err(error),
        }
    } else {
        render_report(&report, command.format, &built_input)
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
    probe: bool,
    allow_network_read: bool,
    dry_run: bool,
    plan: bool,
    queue: bool,
    apply: bool,
    action: Option<String>,
    all: bool,
    all_providers: bool,
    confirm: Option<String>,
    challenge_issued_unix_seconds: Option<u64>,
    accept_no_rollback: bool,
    allow_network_write: bool,
    recovery_status: bool,
    transaction: Option<String>,
    format: OutputFormat,
}

fn parse_args(args: &[String]) -> Result<ParsedArgs, String> {
    let mut parsed = ParsedArgs {
        fixture: None,
        manager_output: None,
        manager: None,
        executable: None,
        probe: false,
        allow_network_read: false,
        dry_run: false,
        plan: false,
        queue: false,
        apply: false,
        action: None,
        all: false,
        all_providers: false,
        confirm: None,
        challenge_issued_unix_seconds: None,
        accept_no_rollback: false,
        allow_network_write: false,
        recovery_status: false,
        transaction: None,
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
            "--probe" => parsed.probe = true,
            "--allow-network-read" => parsed.allow_network_read = true,
            "--plan" => parsed.plan = true,
            "--queue" => parsed.queue = true,
            "--apply" if !parsed.apply => parsed.apply = true,
            "--apply" => return Err("updates accepts --apply only once".to_string()),
            "--action" => {
                index += 1;
                parsed.action = Some(
                    args.get(index)
                        .ok_or_else(|| "--action requires an exact action ID".to_string())?
                        .clone(),
                );
            }
            "--all" => parsed.all = true,
            "--all-providers" => parsed.all_providers = true,
            "--confirm" => {
                index += 1;
                parsed.confirm = Some(
                    args.get(index)
                        .ok_or_else(|| "--confirm requires the exact challenge phrase".to_string())?
                        .clone(),
                );
            }
            "--challenge-issued-unix-seconds" => {
                index += 1;
                parsed.challenge_issued_unix_seconds = Some(
                    args.get(index)
                        .ok_or_else(|| {
                            "--challenge-issued-unix-seconds requires a Unix timestamp".to_string()
                        })?
                        .parse::<u64>()
                        .map_err(|_| {
                            "--challenge-issued-unix-seconds requires a Unix timestamp".to_string()
                        })?,
                );
            }
            "--accept-no-rollback" => parsed.accept_no_rollback = true,
            "--allow-network-write" => parsed.allow_network_write = true,
            "--recovery-status" if !parsed.recovery_status => parsed.recovery_status = true,
            "--recovery-status" => {
                return Err("updates accepts --recovery-status only once".to_string());
            }
            "--transaction" => {
                index += 1;
                if parsed.transaction.is_some() {
                    return Err("updates accepts --transaction only once".to_string());
                }
                parsed.transaction = Some(
                    args.get(index)
                        .ok_or_else(|| {
                            "--transaction requires an exact transaction ID".to_string()
                        })?
                        .clone(),
                );
            }
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
    if parsed.recovery_status {
        if parsed
            .transaction
            .as_deref()
            .is_none_or(|transaction| !rz0_validation_contract::valid_ledger_id(transaction, 96))
        {
            return Err("--recovery-status requires a valid exact --transaction ID".to_string());
        }
        if parsed.dry_run
            || parsed.apply
            || parsed.fixture.is_some()
            || parsed.manager_output.is_some()
            || parsed.manager.is_some()
            || parsed.executable.is_some()
            || parsed.probe
            || parsed.allow_network_read
            || parsed.plan
            || parsed.queue
            || parsed.action.is_some()
            || parsed.all
            || parsed.all_providers
            || parsed.confirm.is_some()
            || parsed.challenge_issued_unix_seconds.is_some()
            || parsed.accept_no_rollback
            || parsed.allow_network_write
        {
            return Err(
                "--recovery-status can be combined only with --transaction and output format"
                    .to_string(),
            );
        }
        return Ok(parsed);
    }
    if parsed.transaction.is_some() {
        return Err("--transaction requires --recovery-status".to_string());
    }
    if parsed.apply {
        if parsed.dry_run {
            return Err(
                "--apply performs its own fresh dry-run; do not combine it with --dry-run"
                    .to_string(),
            );
        }
        if !parsed.probe && !parsed.all_providers {
            return Err("--apply requires an explicit live --probe or --all-providers".to_string());
        }
        if !parsed.allow_network_write {
            return Err("--apply requires --allow-network-write".to_string());
        }
        if parsed.plan || parsed.queue {
            return Err("--apply cannot be combined with --plan or --queue".to_string());
        }
        let selectors = usize::from(parsed.action.is_some())
            + usize::from(parsed.all)
            + usize::from(parsed.all_providers);
        if selectors != 1 {
            return Err(
                "--apply requires exactly one --action ID, --all, or --all-providers".to_string(),
            );
        }
    } else if !parsed.dry_run {
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
    if parsed.probe && parsed.fixture.is_some() {
        return Err("--probe cannot be combined with --fixture".to_string());
    }
    if parsed.probe && parsed.manager_output.is_some() {
        return Err("--probe cannot be combined with --manager-output".to_string());
    }
    if parsed.allow_network_read && !parsed.probe && !parsed.all_providers {
        return Err("--allow-network-read requires --probe or --all-providers".to_string());
    }
    if parsed.allow_network_write && !parsed.apply {
        return Err("--allow-network-write requires --apply".to_string());
    }
    if parsed.accept_no_rollback && !parsed.apply {
        return Err("--accept-no-rollback requires --apply".to_string());
    }
    if parsed.action.is_some() && !parsed.apply {
        return Err("--action requires --apply".to_string());
    }
    if parsed.all && !parsed.apply {
        return Err("--all requires --apply".to_string());
    }
    if parsed.all_providers {
        if !parsed.allow_network_read {
            return Err("--all-providers requires --allow-network-read".to_string());
        }
        if parsed.fixture.is_some()
            || parsed.manager_output.is_some()
            || parsed.manager.is_some()
            || parsed.executable.is_some()
            || parsed.probe
        {
            return Err(
                "--all-providers is a live system-provider probe and cannot be combined with a fixture, manager selection, or --probe details"
                    .to_string(),
            );
        }
    }
    if (parsed.action.is_some() && parsed.all)
        || (parsed.action.is_some() && parsed.all_providers)
        || (parsed.all && parsed.all_providers)
    {
        return Err("--action, --all, and --all-providers are mutually exclusive".to_string());
    }
    if parsed.confirm.is_some() && !parsed.apply {
        return Err("--confirm requires --apply".to_string());
    }
    if parsed.challenge_issued_unix_seconds.is_some() && !parsed.apply {
        return Err("--challenge-issued-unix-seconds requires --apply".to_string());
    }
    if parsed.confirm.is_some() && parsed.challenge_issued_unix_seconds.is_none() {
        return Err(
            "--confirm requires --challenge-issued-unix-seconds from the challenge output"
                .to_string(),
        );
    }
    if parsed.all && (parsed.confirm.is_some() || parsed.challenge_issued_unix_seconds.is_some()) {
        return Err(
            "--all requires one interactive confirmation per item and cannot take --confirm"
                .to_string(),
        );
    }
    if parsed.all_providers
        && (parsed.confirm.is_some() || parsed.challenge_issued_unix_seconds.is_some())
    {
        return Err(
            "--all-providers requires one interactive confirmation per item and cannot take --confirm"
                .to_string(),
        );
    }
    if parsed.probe && parsed.manager.is_none() {
        return Err("--probe requires --manager".to_string());
    }
    if parsed.probe && parsed.executable.is_none() {
        return Err("--probe requires an exact --executable path".to_string());
    }
    if parsed.manager.is_some() && !parsed.probe && parsed.manager_output.is_none() {
        return Err("--manager requires --probe or --manager-output".to_string());
    }
    Ok(parsed)
}

fn recovery_status_command(command: &ParsedArgs) -> (ExitCode, String, String) {
    let transaction = command.transaction.as_deref().unwrap_or_default();
    let state_root = PathBuf::from(module_store_plan(None, None, "update recovery").state_root);
    if !state_root.is_dir() {
        return (
            ExitCode::Usage,
            String::new(),
            "runtime.zero state store is not initialized; run `rz0 store status` for local details\n"
                .to_string(),
        );
    }
    let assessment =
        match rz0_transaction_contract::assess_external_effect_recovery(&state_root, transaction) {
            Ok(assessment) => assessment,
            Err(error) => {
                return (
                    ExitCode::Usage,
                    String::new(),
                    format!("update recovery assessment failed closed: {error}\n"),
                );
            }
        };
    let output = match command.format {
        OutputFormat::Json => serde_json::to_string_pretty(&assessment)
            .map(|json| format!("{json}\n"))
            .map_err(|error| format!("render update recovery status: {error}")),
        OutputFormat::Text => Ok(format!(
            "runtime.zero update recovery status\n\ncontract: {}\nschema_version: {}\nread_only: yes\nwrites_attempted: no\ntransaction_id: {}\njournal_state: {:?}\nreceipt_present: {}\nreceipt_valid: {}\ndecision: {:?}\nautomatic_mutation_authorized: no\n\n{}\n",
            assessment.contract,
            assessment.schema_version,
            assessment.transaction_id,
            assessment.journal_state,
            assessment.receipt_present,
            assessment.receipt_valid,
            assessment.decision,
            recovery_guidance(assessment.decision),
        )),
    };
    match output {
        Ok(output) => (ExitCode::Ok, output, String::new()),
        Err(error) => (ExitCode::Usage, String::new(), format!("{error}\n")),
    }
}

fn recovery_guidance(
    decision: rz0_transaction_contract::ExternalEffectRecoveryDecision,
) -> &'static str {
    use rz0_transaction_contract::ExternalEffectRecoveryDecision as Decision;
    match decision {
        Decision::AbortWithoutWrites => {
            "No manager write started. Preserve the transaction evidence; no automatic cleanup is authorized."
        }
        Decision::VerifyExternalEffect => {
            "The manager outcome is uncertain. Do not rerun it; inspect the manager's installed/available state and preserve all evidence."
        }
        Decision::CompleteJournalCommitWithExplicitApproval => {
            "A verified external-effect receipt exists but final journal state is incomplete. Completion requires a separately implemented exact recovery approval; do not edit state files manually."
        }
        Decision::NoAction => {
            "The verified receipt and committed journal agree. No recovery mutation is indicated."
        }
        Decision::RefuseInconsistentEvidence => {
            "Evidence is missing, malformed, or conflicting. Refuse automatic action and retain the state root for review."
        }
    }
}

#[derive(Debug, Clone)]
struct BuiltInput {
    input: UpdaterFindingInput,
    source_count: usize,
    source_ok_count: usize,
    sources: Vec<ProviderSourceStatus>,
    warnings: Vec<String>,
    aggregate: bool,
    live_probe: bool,
    network_read_requested: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ProviderSourceStatus {
    provider: String,
    status: String,
    candidate_count: usize,
}

fn provider_context(built: &BuiltInput) -> Option<&BuiltInput> {
    built.aggregate.then_some(built)
}

fn build_input(command: &ParsedArgs) -> Result<BuiltInput, String> {
    if command.all_providers {
        return build_all_provider_input(command);
    }
    if let Some(path) = command.fixture.as_deref() {
        return Ok(BuiltInput {
            input: read_input(path)?,
            source_count: 1,
            source_ok_count: 1,
            sources: Vec::new(),
            warnings: Vec::new(),
            aggregate: false,
            live_probe: false,
            network_read_requested: false,
        });
    }
    let manager = command.manager.ok_or_else(|| {
        "provide a local evidence fixture via --fixture, captured manager output via --manager-output, or explicit --probe".to_string()
    })?;
    let spec = manager_probe_specs()
        .into_iter()
        .find(|spec| spec.manager == manager)
        .ok_or_else(|| "manager probe specification is unavailable".to_string())?;
    let (bytes, executable, executable_identity) = if command.probe {
        if manager.platform() != "any" && manager.platform() != std::env::consts::OS {
            return Err(format!(
                "manager '{}' is for {} and cannot be probed on {}",
                manager.id(),
                manager.platform(),
                std::env::consts::OS
            ));
        }
        let executable = command
            .executable
            .clone()
            .ok_or_else(|| "--probe requires an exact --executable path".to_string())?;
        if !Path::new(&executable).is_absolute() {
            return Err("--executable must be an absolute path".to_string());
        }
        if !rz0_module_updater::manager_executable_allowed(
            manager.manager_name(),
            if manager.platform() == "any" {
                std::env::consts::OS
            } else {
                manager.platform()
            },
            &executable,
        ) {
            return Err("--executable is not the allowlisted path for this manager".to_string());
        }
        let (bytes, executable_identity) =
            probe_manager_output(&spec, Path::new(&executable), command.allow_network_read)?;
        (bytes, executable, Some(executable_identity))
    } else {
        let path = command
            .manager_output
            .as_deref()
            .ok_or_else(|| "manager output path is required".to_string())?;
        let executable = command.executable.clone().unwrap_or_default();
        let executable_identity = if executable.is_empty() {
            None
        } else {
            Some(observe_manager_executable(Path::new(&executable))?)
        };
        (read_bounded_bytes(path)?, executable, executable_identity)
    };
    let records = parse_manager_records(&spec, &bytes, executable, executable_identity)?;
    Ok(single_built_input(
        manager,
        records,
        command.probe,
        command.probe && command.allow_network_read,
    ))
}

fn build_all_provider_input(command: &ParsedArgs) -> Result<BuiltInput, String> {
    let platform = std::env::consts::OS;
    let static_specs = rz0_module_updater::manager_probe_specs_for_platform(platform);
    let providers = discover_provider_specs_for_platform(platform);
    let mut records = Vec::new();
    let mut sources = Vec::new();
    let mut warnings = vec![
        "coverage is provider-driven: installed managers, language environments, and known self-updaters are inspected when their exact read-only adapter is available; unknown or mutation-only providers remain visible as bounded gaps".to_string(),
    ];
    let mut source_ok_count = 0usize;
    for spec in static_specs
        .iter()
        .filter(|spec| !spec.executable_candidates.is_empty())
    {
        let Some(provider) = providers
            .iter()
            .find(|provider| provider.manager == spec.manager)
        else {
            sources.push(ProviderSourceStatus {
                provider: spec.manager.id().to_string(),
                status: "missing".to_string(),
                candidate_count: 0,
            });
            warnings.push(format!(
                "{} availability source executable was not found",
                spec.manager.id()
            ));
            continue;
        };
        let (bytes, identity) = match probe_provider_output(provider, command.allow_network_read) {
            Ok(value) => value,
            Err(error) => {
                sources.push(ProviderSourceStatus {
                    provider: provider.instance_id.clone(),
                    status: "unavailable".to_string(),
                    candidate_count: 0,
                });
                warnings.push(format!(
                    "{} availability source unavailable: {error}",
                    provider.instance_id
                ));
                continue;
            }
        };
        match parse_provider_records(provider, &bytes, Some(identity)) {
            Ok(mut source_records) => {
                bind_provider_instance(&mut source_records, provider);
                source_ok_count = source_ok_count.saturating_add(1);
                sources.push(ProviderSourceStatus {
                    provider: provider.instance_id.clone(),
                    status: "ok".to_string(),
                    candidate_count: source_records.len(),
                });
                records.append(&mut source_records);
            }
            Err(error) => {
                sources.push(ProviderSourceStatus {
                    provider: provider.instance_id.clone(),
                    status: "unavailable".to_string(),
                    candidate_count: 0,
                });
                warnings.push(format!(
                    "{} availability output unavailable: {error}",
                    provider.instance_id
                ));
            }
        }
    }

    for manager_id in dynamic_provider_ids() {
        if !providers
            .iter()
            .any(|provider| provider.manager.id() == *manager_id)
        {
            sources.push(ProviderSourceStatus {
                provider: (*manager_id).to_string(),
                status: "missing".to_string(),
                candidate_count: 0,
            });
            warnings.push(format!(
                "{manager_id} provider executable was not found or has no exact adapter"
            ));
        }
    }

    for provider in providers
        .iter()
        .filter(|provider| provider.manager.platform() == "any")
    {
        if sources
            .iter()
            .any(|source| source.provider == provider.instance_id)
        {
            continue;
        }
        if matches!(
            provider.manager,
            ManagerKind::Warp | ManagerKind::Aiup | ManagerKind::CargoInstall
        ) {
            sources.push(ProviderSourceStatus {
                provider: provider.instance_id.clone(),
                status: "observed_only".to_string(),
                candidate_count: 0,
            });
            warnings.push(observed_only_provider_warning(provider.manager));
            continue;
        }
        let (bytes, identity) = match probe_provider_output(provider, command.allow_network_read) {
            Ok(value) => value,
            Err(error) => {
                sources.push(ProviderSourceStatus {
                    provider: provider.instance_id.clone(),
                    status: "unavailable".to_string(),
                    candidate_count: 0,
                });
                warnings.push(format!(
                    "{} availability source unavailable: {error}",
                    provider.instance_id
                ));
                continue;
            }
        };
        match parse_provider_records(provider, &bytes, Some(identity)) {
            Ok(mut source_records) => {
                bind_provider_instance(&mut source_records, provider);
                source_ok_count = source_ok_count.saturating_add(1);
                sources.push(ProviderSourceStatus {
                    provider: provider.instance_id.clone(),
                    status: "ok".to_string(),
                    candidate_count: source_records.len(),
                });
                records.append(&mut source_records);
            }
            Err(error) => {
                sources.push(ProviderSourceStatus {
                    provider: provider.instance_id.clone(),
                    status: "unavailable".to_string(),
                    candidate_count: 0,
                });
                warnings.push(format!(
                    "{} availability output unavailable: {error}",
                    provider.instance_id
                ));
            }
        }
    }

    collect_macos_application_updates(
        command.allow_network_read,
        &mut records,
        &mut sources,
        &mut warnings,
        &mut source_ok_count,
    );

    records.sort_by(|left, right| left.finding_id.cmp(&right.finding_id));
    records.dedup_by(|left, right| left.finding_id == right.finding_id);
    warnings.truncate(MAX_PROVIDER_WARNINGS);
    let normalized = serde_json::to_vec(&records)
        .map_err(|error| format!("normalize aggregate manager evidence: {error}"))?;
    let evidence_digest = sha256(&normalized);
    Ok(BuiltInput {
        input: UpdaterFindingInput {
            schema_version: 1,
            contract: rz0_module_updater::INPUT_CONTRACT.to_string(),
            platform: platform.to_string(),
            input_evidence_sha256: evidence_digest.clone(),
            source_id: format!("system.{}.providers", platform),
            source_evidence_sha256: evidence_digest,
            records,
        },
        source_count: sources.len(),
        source_ok_count,
        sources,
        warnings,
        aggregate: true,
        live_probe: true,
        network_read_requested: command.allow_network_read,
    })
}

fn observed_only_provider_warning(manager: ManagerKind) -> String {
    match manager {
        ManagerKind::Warp => {
            "warp is installed, but its documented TUI command has no read-only availability adapter; use the signed Warp application/provider channel".to_string()
        }
        ManagerKind::Aiup => {
            "aiup is installed as a high-level tool orchestrator; its managed npm/native channels are probed separately, and its dry-run is not treated as independent availability evidence".to_string()
        }
        ManagerKind::CargoInstall => {
            "cargo is installed, but cargo has no built-in read-only outdated query for cargo-installed binaries; they remain observed-only unless an owner adapter is available".to_string()
        }
        _ => format!(
            "{} is installed but has no reviewed read-only availability adapter",
            manager.id()
        ),
    }
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
struct ElectronAppUpdateSpec {
    bundle_id: String,
    name: String,
    installed_version: String,
    owner: String,
    repository: String,
    prerelease: bool,
}

#[cfg(target_os = "macos")]
fn collect_macos_application_updates(
    allow_network_read: bool,
    records: &mut Vec<rz0_module_updater::UpdateRecord>,
    sources: &mut Vec<ProviderSourceStatus>,
    warnings: &mut Vec<String>,
    source_ok_count: &mut usize,
) {
    let sparkle_apps = discover_sparkle_apps();
    if !sparkle_apps.is_empty() {
        sources.push(ProviderSourceStatus {
            provider: "macos.sparkle.apps".to_string(),
            status: "observed_only".to_string(),
            candidate_count: 0,
        });
        warnings.push(format!(
            "{} installed application bundle(s) expose Sparkle updater metadata ({}); no external update command was assumed, so they remain on their signed in-app update channel",
            sparkle_apps.len(),
            sparkle_apps.join(", ")
        ));
    }
    let apps = discover_electron_app_specs();
    if apps.is_empty() {
        return;
    }
    let app_count = apps.len();
    let curl = resolve_probe_executable(&["/usr/bin/curl", "/opt/homebrew/bin/curl"]);
    for app in apps.into_iter().take(48) {
        let provider_id = format!("electron:{}", app.bundle_id);
        let bundle_path = app.bundle_id.clone();
        if !allow_network_read {
            sources.push(ProviderSourceStatus {
                provider: provider_id,
                status: "blocked".to_string(),
                candidate_count: 0,
            });
            warnings.push(format!(
                "{} application release metadata requires --allow-network-read",
                bundle_path
            ));
            continue;
        }
        let Some(curl) = curl.as_deref() else {
            sources.push(ProviderSourceStatus {
                provider: provider_id,
                status: "missing".to_string(),
                candidate_count: 0,
            });
            warnings.push(format!(
                "{} application updater metadata was found, but curl is unavailable",
                bundle_path
            ));
            continue;
        };
        let identity = match observe_manager_executable(curl) {
            Ok(identity) => identity,
            Err(error) => {
                sources.push(ProviderSourceStatus {
                    provider: provider_id,
                    status: "unavailable".to_string(),
                    candidate_count: 0,
                });
                warnings.push(format!(
                    "{} application release probe executable unavailable: {error}",
                    bundle_path
                ));
                continue;
            }
        };
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases?per_page=20",
            app.owner, app.repository
        );
        let output =
            rz0_process_host::run_read_only_process(&rz0_process_host::ReadOnlyProcessRequest {
                executable: curl.to_path_buf(),
                arguments: vec![
                    "--fail".to_string(),
                    "--silent".to_string(),
                    "--show-error".to_string(),
                    "--max-time".to_string(),
                    "20".to_string(),
                    "--header".to_string(),
                    "Accept: application/vnd.github+json".to_string(),
                    "--header".to_string(),
                    "User-Agent: runtime.zero".to_string(),
                    url,
                ],
                working_directory: PathBuf::from("/"),
                environment: probe_environment(),
                timeout: std::time::Duration::from_secs(25),
                output_limit: MAX_INPUT_BYTES,
            });
        let _ = observe_manager_executable(curl).map(|after| {
            if after != identity {
                warnings.push(format!(
                    "{} application release probe executable identity changed",
                    bundle_path
                ));
            }
        });
        let output = match output {
            Ok(output) if output.status.success() => output,
            Ok(output) => {
                sources.push(ProviderSourceStatus {
                    provider: provider_id,
                    status: "unavailable".to_string(),
                    candidate_count: 0,
                });
                warnings.push(format!(
                    "{} application release metadata probe failed with {}",
                    bundle_path, output.status
                ));
                continue;
            }
            Err(error) => {
                sources.push(ProviderSourceStatus {
                    provider: provider_id,
                    status: "unavailable".to_string(),
                    candidate_count: 0,
                });
                warnings.push(format!(
                    "{} application release metadata probe failed: {error}",
                    bundle_path
                ));
                continue;
            }
        };
        let bytes = if output.stdout.bytes.is_empty() {
            &output.stderr.bytes
        } else {
            &output.stdout.bytes
        };
        match parse_electron_release(bytes, &app) {
            Ok(Some(available_version)) => {
                let digest = sha256(format!("{}:{available_version}", app.bundle_id).as_bytes());
                records.push(rz0_module_updater::UpdateRecord {
                    finding_id: format!("update.electron-app.{}", &digest[..16]),
                    subject_reference: format!("application:{}", app.bundle_id),
                    installed: true,
                    manager_record_present: true,
                    update_available: true,
                    installed_version: Some(app.installed_version.clone()),
                    available_version: Some(available_version),
                    manager: None,
                    executable: None,
                    executable_sha256: None,
                    executable_size_bytes: None,
                    arguments: Vec::new(),
                    network_required: true,
                    requires_elevation: false,
                    rollback_supported: false,
                });
                sources.push(ProviderSourceStatus {
                    provider: provider_id,
                    status: "ok".to_string(),
                    candidate_count: 1,
                });
                *source_ok_count = source_ok_count.saturating_add(1);
            }
            Ok(None) => {
                sources.push(ProviderSourceStatus {
                    provider: provider_id,
                    status: "ok".to_string(),
                    candidate_count: 0,
                });
                *source_ok_count = source_ok_count.saturating_add(1);
            }
            Err(error) => {
                sources.push(ProviderSourceStatus {
                    provider: provider_id,
                    status: "unavailable".to_string(),
                    candidate_count: 0,
                });
                warnings.push(format!(
                    "{} application release metadata was not parseable: {error}",
                    bundle_path
                ));
            }
        }
    }
    if app_count > 48 {
        warnings.push(format!(
            "{} Electron application updater declarations were found; the bounded live review inspected the first 48",
            app_count
        ));
    }
}

#[cfg(target_os = "macos")]
fn discover_sparkle_apps() -> Vec<String> {
    let mut roots = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/Applications/Utilities"),
        PathBuf::from("/System/Applications"),
        PathBuf::from("/System/Applications/Utilities"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home).join("Applications"));
    }
    let mut apps = Vec::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten().take(512) {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("app")
                || !path.join("Contents/Frameworks/Sparkle.framework").is_dir()
            {
                continue;
            }
            if let Some(name) = path.file_stem().and_then(|value| value.to_str()) {
                if !name.is_empty() && !name.chars().any(char::is_control) {
                    apps.push(name.to_string());
                }
            }
        }
    }
    apps.sort();
    apps.dedup();
    apps
}

#[cfg(not(target_os = "macos"))]
fn discover_sparkle_apps() -> Vec<String> {
    Vec::new()
}

#[cfg(not(target_os = "macos"))]
fn collect_macos_application_updates(
    _allow_network_read: bool,
    _records: &mut Vec<rz0_module_updater::UpdateRecord>,
    _sources: &mut Vec<ProviderSourceStatus>,
    _warnings: &mut Vec<String>,
    _source_ok_count: &mut usize,
) {
}

#[cfg(target_os = "macos")]
fn discover_electron_app_specs() -> Vec<ElectronAppUpdateSpec> {
    let mut roots = vec![
        PathBuf::from("/Applications"),
        PathBuf::from("/Applications/Utilities"),
        PathBuf::from("/System/Applications"),
        PathBuf::from("/System/Applications/Utilities"),
    ];
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home).join("Applications"));
    }
    let mut specs = Vec::new();
    for root in roots {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten().take(512) {
            let path = entry.path();
            let is_app = path.extension().and_then(|value| value.to_str()) == Some("app");
            if !is_app || !fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.is_dir()) {
                continue;
            }
            let info_path = path.join("Contents/Info.plist");
            let Ok(bytes) = read_small_direct_file(&info_path) else {
                continue;
            };
            let Ok(value) = plist::Value::from_reader(Cursor::new(bytes)) else {
                continue;
            };
            let Some(dictionary) = value.as_dictionary() else {
                continue;
            };
            let Some(bundle_id) = dictionary
                .get("CFBundleIdentifier")
                .and_then(plist::Value::as_string)
                .and_then(|value| valid_app_field(value, 240))
            else {
                continue;
            };
            let name = dictionary
                .get("CFBundleDisplayName")
                .or_else(|| dictionary.get("CFBundleName"))
                .and_then(plist::Value::as_string)
                .and_then(|value| valid_app_field(value, 160))
                .or_else(|| {
                    path.file_stem()
                        .and_then(|value| value.to_str())
                        .and_then(|value| valid_app_field(value, 160))
                })
                .unwrap_or_else(|| bundle_id.clone());
            let Some(installed_version) = ["CFBundleShortVersionString", "CFBundleVersion"]
                .into_iter()
                .find_map(|key| {
                    dictionary
                        .get(key)
                        .and_then(plist::Value::as_string)
                        .and_then(|value| valid_app_field(value, 120))
                })
            else {
                continue;
            };
            let update_manifest = path.join("Contents/Resources/app-update.yml");
            if let Ok(manifest) = read_small_direct_file(&update_manifest)
                && let Ok(manifest) = std::str::from_utf8(&manifest)
                && let (Some(provider), Some(owner), Some(repository)) = (
                    yaml_field(manifest, "provider"),
                    yaml_field(manifest, "owner"),
                    yaml_field(manifest, "repo"),
                )
                && provider == "github"
            {
                let prerelease = yaml_field(manifest, "releaseType")
                    .is_some_and(|value| value == "prerelease")
                    || yaml_field(manifest, "channel").is_some_and(|value| value == "nightly");
                specs.push(ElectronAppUpdateSpec {
                    bundle_id,
                    name,
                    installed_version,
                    owner,
                    repository,
                    prerelease,
                });
            }
        }
    }
    specs.sort_by(|left, right| left.bundle_id.cmp(&right.bundle_id));
    specs.dedup_by(|left, right| left.bundle_id == right.bundle_id);
    specs
}

#[cfg(target_os = "macos")]
fn parse_electron_release(
    bytes: &[u8],
    app: &ElectronAppUpdateSpec,
) -> Result<Option<String>, String> {
    let releases: Vec<serde_json::Value> = serde_json::from_slice(bytes)
        .map_err(|error| format!("parse GitHub release JSON: {error}"))?;
    for release in releases {
        if release
            .get("draft")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true)
        {
            continue;
        }
        let prerelease = release
            .get("prerelease")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        if prerelease != app.prerelease {
            continue;
        }
        let Some(version) = release
            .get("tag_name")
            .or_else(|| release.get("name"))
            .and_then(serde_json::Value::as_str)
            .and_then(|value| valid_app_field(value, 120))
        else {
            continue;
        };
        if version != app.installed_version {
            return Ok(Some(version));
        }
        return Ok(None);
    }
    Err(format!(
        "no matching {} GitHub release was returned for {}",
        if app.prerelease {
            "prerelease"
        } else {
            "stable"
        },
        app.name
    ))
}

#[cfg(target_os = "macos")]
fn read_small_direct_file(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| format!("inspect file: {error}"))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES
    {
        return Err("file is not a bounded direct regular file".to_string());
    }
    fs::read(path).map_err(|error| format!("read file: {error}"))
}

#[cfg(target_os = "macos")]
fn yaml_field(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (candidate, value) = line.split_once(':')?;
        if candidate.trim() != key {
            return None;
        }
        valid_app_field(value.trim().trim_matches(['\'', '"']), 160)
    })
}

#[cfg(target_os = "macos")]
fn valid_app_field(value: &str, maximum: usize) -> Option<String> {
    if value.is_empty()
        || value.len() > maximum
        || value.chars().any(char::is_control)
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b' '))
    {
        None
    } else {
        Some(value.to_string())
    }
}

fn single_built_input(
    manager: ManagerKind,
    records: Vec<rz0_module_updater::UpdateRecord>,
    live_probe: bool,
    network_read_requested: bool,
) -> BuiltInput {
    let normalized = serde_json::to_vec(&records).unwrap_or_default();
    let evidence_digest = sha256(&normalized);
    BuiltInput {
        input: UpdaterFindingInput {
            schema_version: 1,
            contract: rz0_module_updater::INPUT_CONTRACT.to_string(),
            platform: if manager.platform() == "any" {
                std::env::consts::OS.to_string()
            } else {
                manager.platform().to_string()
            },
            input_evidence_sha256: evidence_digest.clone(),
            source_id: format!("manager.{}", manager.id()),
            source_evidence_sha256: evidence_digest,
            records,
        },
        source_count: 1,
        source_ok_count: 1,
        sources: Vec::new(),
        warnings: Vec::new(),
        aggregate: false,
        live_probe,
        network_read_requested,
    }
}

fn probe_manager_output(
    spec: &rz0_module_updater::ManagerProbeSpec,
    executable: &Path,
    allow_network_read: bool,
) -> Result<(Vec<u8>, rz0_action_plan::ActionExecutableIdentity), String> {
    if !allow_network_read && spec.network_required {
        return Err(
            "this manager probe may access network metadata; pass --allow-network-read explicitly"
                .to_string(),
        );
    }
    let executable_identity = observe_manager_executable(executable)?;
    let output =
        rz0_process_host::run_read_only_process(&rz0_process_host::ReadOnlyProcessRequest {
            executable: executable.to_path_buf(),
            arguments: spec
                .query_arguments
                .iter()
                .map(|argument| (*argument).to_string())
                .collect(),
            working_directory: PathBuf::from("/"),
            environment: probe_environment(),
            timeout: std::time::Duration::from_secs(10),
            output_limit: MAX_INPUT_BYTES,
        })
        .map_err(|error| format!("manager probe failed closed: {error}"))?;
    let accepted_nonzero = accepted_probe_status(spec.manager, output.status.code());
    if !output.status.success() && !accepted_nonzero {
        return Err(format!(
            "manager probe exited with an unaccepted status: {}",
            output.status
        ));
    }
    let identity_after = observe_manager_executable(executable)?;
    if identity_after != executable_identity {
        return Err(
            "manager executable identity changed during the live availability probe".to_string(),
        );
    }
    let bytes = if output.stdout.bytes.is_empty() {
        output.stderr.bytes
    } else {
        output.stdout.bytes
    };
    Ok((bytes, executable_identity))
}

fn probe_provider_output(
    spec: &ProviderProbeSpec,
    allow_network_read: bool,
) -> Result<(Vec<u8>, rz0_action_plan::ActionExecutableIdentity), String> {
    if !allow_network_read && spec.network_required {
        return Err(
            "this provider probe may access network metadata; pass --allow-network-read explicitly"
                .to_string(),
        );
    }
    let executable_identity = observe_manager_executable(&spec.executable)?;
    let npm_cache = (spec.manager == ManagerKind::NpmGlobal).then(|| {
        std::env::temp_dir().join(format!(
            "runtime-zero-npm-probe-{}-{}",
            std::process::id(),
            sha256(spec.instance_id.as_bytes())
        ))
    });
    if let Some(cache) = npm_cache.as_deref() {
        fs::create_dir_all(cache)
            .map_err(|error| format!("create isolated npm probe cache: {error}"))?;
    }
    let mut environment = probe_environment();
    if let Some(cache) = npm_cache.as_deref() {
        environment.push(("NPM_CONFIG_CACHE".to_string(), cache.display().to_string()));
        environment.push((
            "NPM_CONFIG_UPDATE_NOTIFIER".to_string(),
            "false".to_string(),
        ));
    }
    let result =
        rz0_process_host::run_read_only_process(&rz0_process_host::ReadOnlyProcessRequest {
            executable: spec.executable.clone(),
            arguments: spec.query_arguments.clone(),
            working_directory: PathBuf::from("/"),
            environment,
            timeout: std::time::Duration::from_secs(30),
            output_limit: MAX_INPUT_BYTES,
        });
    if let Some(cache) = npm_cache {
        let _ = fs::remove_dir_all(cache);
    }
    let output = result.map_err(|error| format!("provider probe failed closed: {error}"))?;
    let accepted_nonzero = accepted_probe_status(spec.manager, output.status.code());
    if !output.status.success() && !accepted_nonzero {
        return Err(format!(
            "provider probe exited with an unaccepted status: {}",
            output.status
        ));
    }
    let identity_after = observe_manager_executable(&spec.executable)?;
    if identity_after != executable_identity {
        return Err(
            "provider executable identity changed during the live availability probe".to_string(),
        );
    }
    let bytes = if output.stdout.bytes.is_empty() {
        output.stderr.bytes
    } else {
        output.stdout.bytes
    };
    Ok((bytes, executable_identity))
}

fn accepted_probe_status(manager: ManagerKind, code: Option<i32>) -> bool {
    (manager == ManagerKind::Dnf && code == Some(100))
        || (manager == ManagerKind::NpmGlobal && code == Some(1))
        || (manager == ManagerKind::Rustup && code == Some(100))
}

fn parse_manager_records(
    spec: &rz0_module_updater::ManagerProbeSpec,
    bytes: &[u8],
    executable: String,
    executable_identity: Option<rz0_action_plan::ActionExecutableIdentity>,
) -> Result<Vec<rz0_module_updater::UpdateRecord>, String> {
    let context = ManagerParseContext {
        manager: spec.manager,
        executable: (!executable.is_empty()).then_some(executable),
        executable_sha256: executable_identity
            .as_ref()
            .map(|identity| identity.sha256.clone()),
        executable_size_bytes: executable_identity.map(|identity| identity.size_bytes),
        network_required: spec.network_required,
        requires_elevation: spec.requires_elevation,
        rollback_supported: false,
    };
    let mut records = parse_manager_output(&context, bytes)?;
    records.sort_by(|left, right| left.finding_id.cmp(&right.finding_id));
    Ok(records)
}

fn parse_provider_records(
    spec: &ProviderProbeSpec,
    bytes: &[u8],
    executable_identity: Option<rz0_action_plan::ActionExecutableIdentity>,
) -> Result<Vec<rz0_module_updater::UpdateRecord>, String> {
    let context = ManagerParseContext {
        manager: spec.manager,
        executable: Some(spec.executable.display().to_string()),
        executable_sha256: executable_identity
            .as_ref()
            .map(|identity| identity.sha256.clone()),
        executable_size_bytes: executable_identity.map(|identity| identity.size_bytes),
        network_required: spec.network_required,
        requires_elevation: spec.requires_elevation,
        rollback_supported: false,
    };
    let mut records = parse_manager_output(&context, bytes)?;
    records.sort_by(|left, right| left.finding_id.cmp(&right.finding_id));
    Ok(records)
}

fn bind_provider_instance(
    records: &mut [rz0_module_updater::UpdateRecord],
    provider: &ProviderProbeSpec,
) {
    let namespace = sha256(provider.instance_id.as_bytes())[..12].to_string();
    for record in records {
        let suffix = record
            .finding_id
            .strip_prefix("update.")
            .unwrap_or(&record.finding_id);
        record.finding_id = format!("update.{namespace}.{suffix}");
        if let Some(prefix) = provider.update_prefix.as_ref()
            && provider.manager == ManagerKind::NpmGlobal
        {
            record
                .arguments
                .splice(2..2, ["--prefix".to_string(), prefix.display().to_string()]);
        }
    }
}

fn resolve_probe_executable(candidates: &[&str]) -> Option<PathBuf> {
    candidates.iter().map(Path::new).find_map(|path| {
        let metadata = fs::symlink_metadata(path).ok()?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return None;
        }
        Some(path.to_path_buf())
    })
}

fn apply_update_command(
    command: &ParsedArgs,
    input: &UpdaterFindingInput,
    report: &rz0_finding_contract::FindingReport,
) -> (ExitCode, String, String) {
    if command.all || command.all_providers {
        return apply_all_update_command(command, input, report);
    }
    apply_one_update_command(
        command,
        input,
        report,
        command.action.as_deref().unwrap_or_default(),
    )
}

fn apply_one_update_command(
    command: &ParsedArgs,
    input: &UpdaterFindingInput,
    report: &rz0_finding_contract::FindingReport,
    action_id: &str,
) -> (ExitCode, String, String) {
    let plan = match build_update_action_plan(input, report) {
        Ok(plan) => plan,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("updater action plan failed closed: {error}\n"),
            );
        }
    };
    let Some(action) = plan
        .actions
        .iter()
        .find(|action| action.action_id == action_id)
    else {
        return (
            ExitCode::Usage,
            String::new(),
            format!("exact planned update action '{action_id}' was not found\n"),
        );
    };
    if action.disposition != ActionDisposition::Planned {
        return (
            ExitCode::Usage,
            String::new(),
            format!("update action '{action_id}' is blocked and cannot execute\n"),
        );
    }
    if !action.rollback.supported && !command.accept_no_rollback {
        return (
            ExitCode::Usage,
            String::new(),
            "this manager has no proven rollback path; pass --accept-no-rollback to acknowledge manual recovery risk\n".to_string(),
        );
    }
    let single_plan = match make_single_action_plan(&plan, action) {
        Ok(plan) => plan,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("single-action update plan failed closed: {error}\n"),
            );
        }
    };
    let single_action = &single_plan.actions[0];
    let now = command
        .challenge_issued_unix_seconds
        .unwrap_or_else(unix_seconds);
    let (challenge, view) = match build_update_challenge(
        &single_plan,
        single_action,
        command.accept_no_rollback,
        now,
    ) {
        Ok(challenge) => challenge,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("update confirmation challenge failed closed: {error}\n"),
            );
        }
    };
    let Some(phrase) = command.confirm.as_deref() else {
        return (
            ExitCode::Ok,
            render_challenge(&view, command.format),
            String::new(),
        );
    };
    let response = match validate_update_confirmation(&challenge, phrase, unix_seconds()) {
        Ok(response) => response,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("update confirmation rejected: {error}\n"),
            );
        }
    };
    let state_root = PathBuf::from(module_store_plan(None, None, "update execution").state_root);
    if !Path::new(&state_root).is_dir() {
        return (
            ExitCode::Usage,
            String::new(),
            "runtime.zero state store is not initialized; run `rz0 store init --dry-run` before any explicit initialization\n"
                .to_string(),
        );
    }
    let finding_id = single_action.finding_id.clone();
    let (controller, cancellation) = rz0_cancellation_contract::cancellation_pair();
    let _interrupt = match InterruptBridge::install(controller) {
        Ok(bridge) => bridge,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("install update cancellation bridge: {error}\n"),
            );
        }
    };
    let execution = execute_update_action(UpdateExecutionRequest {
        state_root: &state_root,
        plan: &single_plan,
        action: single_action,
        challenge: &challenge,
        response: &response,
        now_unix_seconds: unix_seconds(),
        environment: probe_environment(),
        cancellation: &cancellation,
        verify_after: || verify_update_after(command, &finding_id),
    });
    match execution {
        Ok(report) => (
            ExitCode::Ok,
            render_execution(&report, command.format),
            String::new(),
        ),
        Err(error) => (ExitCode::Usage, String::new(), format!("{error}\n")),
    }
}

fn apply_all_update_command(
    command: &ParsedArgs,
    input: &UpdaterFindingInput,
    report: &rz0_finding_contract::FindingReport,
) -> (ExitCode, String, String) {
    if command.format == OutputFormat::Json
        || !io::stdin().is_terminal()
        || !io::stdout().is_terminal()
    {
        return (
            ExitCode::Usage,
            String::new(),
            "--all requires an interactive text terminal for one confirmation per item".to_string(),
        );
    }
    let state_root = PathBuf::from(module_store_plan(None, None, "update execution").state_root);
    if !state_root.is_dir() {
        return (
            ExitCode::Usage,
            String::new(),
            "runtime.zero state store is not initialized; run `rz0 store init --dry-run` before any explicit initialization\n"
                .to_string(),
        );
    }
    let plan = match build_update_action_plan(input, report) {
        Ok(plan) => plan,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("updater action plan failed closed: {error}\n"),
            );
        }
    };
    let action_ids = plan
        .actions
        .iter()
        .filter(|action| action.disposition == ActionDisposition::Planned)
        .map(|action| action.action_id.clone())
        .collect::<Vec<_>>();
    if action_ids.is_empty() {
        return (
            ExitCode::Usage,
            String::new(),
            "serial update queue contains no executable candidates".to_string(),
        );
    }
    let mut output = String::new();
    for action_id in action_ids {
        let fresh_built = match build_input(command) {
            Ok(input) => input,
            Err(error) => return (ExitCode::Usage, output, format!("{error}\n")),
        };
        let fresh_input = &fresh_built.input;
        let fresh_report = match classify_updates(fresh_input) {
            Ok(report) => report,
            Err(error) => return (ExitCode::Usage, output, format!("{error}\n")),
        };
        let fresh_plan = match build_update_action_plan(fresh_input, &fresh_report) {
            Ok(plan) => plan,
            Err(error) => return (ExitCode::Usage, output, format!("{error}\n")),
        };
        let Some(action) = fresh_plan.actions.iter().find(|action| {
            action.action_id == action_id && action.disposition == ActionDisposition::Planned
        }) else {
            output.push_str(&format!(
                "skipped {action_id}: fresh evidence no longer lists it\n"
            ));
            continue;
        };
        if !action.rollback.supported && !command.accept_no_rollback {
            return (
                ExitCode::Usage,
                output,
                format!("{action_id} lacks rollback proof; pass --accept-no-rollback\n"),
            );
        }
        let single_plan = match make_single_action_plan(&fresh_plan, action) {
            Ok(plan) => plan,
            Err(error) => return (ExitCode::Usage, output, format!("{error}\n")),
        };
        let single_action = &single_plan.actions[0];
        let issued = unix_seconds();
        let (_challenge, view) = match build_update_challenge(
            &single_plan,
            single_action,
            command.accept_no_rollback,
            issued,
        ) {
            Ok(value) => value,
            Err(error) => return (ExitCode::Usage, output, format!("{error}\n")),
        };
        if write!(
            io::stdout(),
            "{}Type the phrase to continue (or `cancel`): ",
            render_challenge(&view, OutputFormat::Text)
        )
        .and_then(|()| io::stdout().flush())
        .is_err()
        {
            return (
                ExitCode::Usage,
                output,
                "failed to present the exact update confirmation; no action was started\n"
                    .to_string(),
            );
        }
        let mut phrase = String::new();
        if io::stdin().read_line(&mut phrase).is_err() {
            return (
                ExitCode::Usage,
                output,
                "failed to read update confirmation\n".to_string(),
            );
        }
        let phrase = phrase.trim().to_string();
        if phrase.eq_ignore_ascii_case("cancel") {
            output.push_str("serial update queue cancelled before the next item\n");
            return (ExitCode::Ok, output, String::new());
        }
        let mut per_item = command.clone();
        per_item.action = Some(action_id);
        per_item.all = false;
        per_item.confirm = Some(phrase);
        per_item.challenge_issued_unix_seconds = Some(issued);
        let (code, stdout, stderr) = apply_one_update_command(
            &per_item,
            fresh_input,
            &fresh_report,
            per_item.action.as_deref().unwrap_or_default(),
        );
        output.push_str(&stdout);
        if !stderr.is_empty() {
            return (code, output, stderr);
        }
        if code != ExitCode::Ok {
            return (
                code,
                output,
                "serial update queue paused after the item failure\n".to_string(),
            );
        }
    }
    output.push_str("serial update queue completed its current evidence set\n");
    (ExitCode::Ok, output, String::new())
}

fn verify_update_after(command: &ParsedArgs, finding_id: &str) -> Result<String, String> {
    let fresh_built = build_input(command)?;
    let fresh_input = &fresh_built.input;
    let fresh_report = classify_updates(fresh_input)?;
    let fresh_plan = build_update_action_plan(fresh_input, &fresh_report)
        .map_err(|error| format!("fresh update verification plan failed closed: {error}"))?;
    verify_candidate_absent(&fresh_plan, finding_id)
}

fn verify_candidate_absent(plan: &ActionPlan, finding_id: &str) -> Result<String, String> {
    if plan
        .actions
        .iter()
        .any(|action| action.finding_id == finding_id)
    {
        Err("fresh availability evidence still reports the exact update candidate".to_string())
    } else {
        Ok("fresh manager availability evidence no longer reports the exact candidate".to_string())
    }
}

fn render_challenge(view: &UpdateChallengeView, format: OutputFormat) -> String {
    match format {
        OutputFormat::Text => format!(
            "runtime.zero update confirmation\n\nplan_id: {}\naction_id: {}\nplan_sha256: {}\nissued_unix_seconds: {}\nexpires_unix_seconds: {}\nrollback_available: {}\nmanual_recovery_acknowledged: {}\n\nType this exact phrase in a new command invocation and pass --challenge-issued-unix-seconds {}:\n{}\n\nNo manager command was executed.\n",
            view.plan_id,
            view.action_id,
            view.plan_sha256,
            view.issued_unix_seconds,
            view.expires_unix_seconds,
            view.rollback_available,
            view.manual_recovery_acknowledged,
            view.issued_unix_seconds,
            view.expected_phrase,
        ),
        OutputFormat::Json => serde_json::to_string_pretty(view).map_or_else(
            |error| format!("challenge serialization failed: {error}\n"),
            |json| format!("{json}\n"),
        ),
    }
}

fn render_execution(report: &UpdateExecutionReport, format: OutputFormat) -> String {
    match format {
        OutputFormat::Text => format!(
            "runtime.zero update execution\n\ntransaction_id: {}\naction_id: {}\nmanager: {}\ntarget: {}\nexecutable_sha256: {}\nexecutable_size_bytes: {}\nexecutable_binding: {}\nstatus: {:?}\nexit_code: {:?}\nverification: {}\nstdout_bytes: {}\nstdout_sha256: {}\nstderr_bytes: {}\nstderr_sha256: {}\nreceipt_reference: {}\nwrites_attempted: yes\nproduct_execution_authorized: yes\n",
            report.transaction_id,
            report.action_id,
            report.manager,
            report.target,
            report.executable_sha256,
            report.executable_size_bytes,
            report.executable_binding,
            report.status,
            report.exit_code,
            report.verification,
            report.stdout_bytes,
            report.stdout_sha256,
            report.stderr_bytes,
            report.stderr_sha256,
            report.receipt_reference,
        ),
        OutputFormat::Json => serde_json::to_string_pretty(report).map_or_else(
            |error| format!("execution serialization failed: {error}\n"),
            |json| format!("{json}\n"),
        ),
    }
}

#[cfg(unix)]
struct InterruptBridge {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    signal_id: signal_hook::SigId,
    thread: Option<std::thread::JoinHandle<()>>,
}

#[cfg(unix)]
impl InterruptBridge {
    fn install(
        controller: rz0_cancellation_contract::CancellationController,
    ) -> Result<Self, String> {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        let interrupted = Arc::new(AtomicBool::new(false));
        let signal_id = signal_hook::flag::register(
            signal_hook::consts::signal::SIGINT,
            Arc::clone(&interrupted),
        )
        .map_err(|error| format!("register SIGINT flag: {error}"))?;
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let thread = std::thread::Builder::new()
            .name("rz0-update-cancellation".to_string())
            .spawn(move || {
                while !thread_stop.load(Ordering::Acquire) {
                    if interrupted.load(Ordering::Acquire) {
                        controller
                            .cancel(rz0_cancellation_contract::CancellationReason::UserRequested);
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            })
            .map_err(|error| {
                signal_hook::low_level::unregister(signal_id);
                format!("spawn SIGINT cancellation bridge: {error}")
            })?;
        Ok(Self {
            stop,
            signal_id,
            thread: Some(thread),
        })
    }
}

#[cfg(unix)]
impl Drop for InterruptBridge {
    fn drop(&mut self) {
        use std::sync::atomic::Ordering;
        self.stop.store(true, Ordering::Release);
        signal_hook::low_level::unregister(self.signal_id);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(not(unix))]
struct InterruptBridge;

#[cfg(not(unix))]
impl InterruptBridge {
    fn install(
        _controller: rz0_cancellation_contract::CancellationController,
    ) -> Result<Self, String> {
        Err("interactive update cancellation is not implemented on this platform".to_string())
    }
}

fn unix_seconds() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn probe_environment() -> Vec<(String, String)> {
    let mut environment = Vec::new();
    if let Some(home) = std::env::var_os("HOME").and_then(|value| value.into_string().ok()) {
        environment.push(("HOME".to_string(), home));
    }
    let path = match std::env::consts::OS {
        "macos" => "/usr/bin:/bin:/opt/homebrew/bin:/usr/local/bin",
        "linux" => "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
        _ => "",
    };
    if !path.is_empty() {
        environment.push(("PATH".to_string(), path.to_string()));
    }
    if std::env::consts::OS == "macos" {
        environment.push(("LANG".to_string(), "C".to_string()));
        environment.push(("LC_ALL".to_string(), "C".to_string()));
        environment.push(("LC_CTYPE".to_string(), "C".to_string()));
        environment.push(("LANGUAGE".to_string(), "C".to_string()));
        environment.push(("HOMEBREW_NO_AUTO_UPDATE".to_string(), "1".to_string()));
        environment.push(("HOMEBREW_NO_ENV_HINTS".to_string(), "1".to_string()));
    }
    environment
}

fn read_input(path: &Path) -> Result<UpdaterFindingInput, String> {
    let bytes = read_bounded_direct_file(path, "updater fixture")?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse updater fixture: {error}"))
}

fn read_bounded_bytes(path: &Path) -> Result<Vec<u8>, String> {
    read_bounded_direct_file(path, "manager output")
}

fn read_bounded_direct_file(path: &Path, label: &str) -> Result<Vec<u8>, String> {
    let observed =
        fs::symlink_metadata(path).map_err(|error| format!("inspect {label}: {error}"))?;
    if observed.file_type().is_symlink() || !observed.is_file() {
        return Err(format!("{label} must be a direct regular file"));
    }
    if observed.len() > MAX_INPUT_BYTES {
        return Err(format!("{label} exceeds the foundation byte ceiling"));
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    }
    let mut file = options
        .open(path)
        .map_err(|error| format!("open direct {label}: {error}"))?;
    let opened = file
        .metadata()
        .map_err(|error| format!("inspect opened {label}: {error}"))?;
    if !opened.is_file() || opened.len() != observed.len() || !same_file(&observed, &opened) {
        return Err(format!("{label} identity changed while opening"));
    }
    let mut bytes = Vec::with_capacity(usize::try_from(opened.len()).unwrap_or(0));
    (&mut file)
        .take(MAX_INPUT_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| format!("read direct {label}: {error}"))?;
    let final_metadata = file
        .metadata()
        .map_err(|error| format!("reinspect opened {label}: {error}"))?;
    if bytes.len() as u64 != opened.len()
        || bytes.len() as u64 > MAX_INPUT_BYTES
        || final_metadata.len() != opened.len()
        || !same_file(&opened, &final_metadata)
    {
        return Err(format!(
            "{label} changed or exceeded its bound while reading"
        ));
    }
    Ok(bytes)
}

#[cfg(unix)]
fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(not(unix))]
fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.len() == right.len()
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn parse_manager_kind(value: &str) -> Result<ManagerKind, String> {
    [
        ManagerKind::HomebrewFormula,
        ManagerKind::HomebrewCask,
        ManagerKind::MacPorts,
        ManagerKind::MacAppStore,
        ManagerKind::AppleSoftwareUpdate,
        ManagerKind::Winget,
        ManagerKind::Apt,
        ManagerKind::Dnf,
        ManagerKind::Pacman,
        ManagerKind::Zypper,
        ManagerKind::Snap,
        ManagerKind::Flatpak,
        ManagerKind::NpmGlobal,
        ManagerKind::Pip,
        ManagerKind::RubyGems,
        ManagerKind::Grok,
        ManagerKind::Hermes,
        ManagerKind::OhMyPi,
        ManagerKind::Warp,
        ManagerKind::Rustup,
        ManagerKind::UvTools,
        ManagerKind::Deno,
        ManagerKind::Aiup,
        ManagerKind::CargoInstall,
    ]
    .into_iter()
    .find(|manager| manager.id() == value)
    .ok_or_else(|| format!("unsupported manager '{value}'"))
}

fn render_report(
    report: &rz0_finding_contract::FindingReport,
    format: OutputFormat,
    built: &BuiltInput,
) -> Result<String, String> {
    let provider = provider_context(built);
    match format {
        OutputFormat::Text => Ok(format!(
            "runtime.zero updater review\n\ncontract: {UPDATES_CONTRACT}\nsource_contract: {}\nreport_id: {}\nread_only: yes\nwrites_attempted: no\nupdate_candidates: {}\nblocked: {}\n{}{}\n",
            report.contract,
            report.report_id,
            report.summary.manager_action_candidate_count,
            report.summary.blocked_count,
            render_provider_text(provider),
            render_probe_text(built),
        )),
        OutputFormat::Json => match provider {
            Some(provider) => render_json(&ProviderReview {
                coverage: ProviderCoverage::from(provider),
                result: report,
            }),
            None => render_json(&CliReview::Report(report)),
        },
    }
}

fn render_plan(
    plan: &ActionPlan,
    format: OutputFormat,
    built: &BuiltInput,
) -> Result<String, String> {
    let provider = provider_context(built);
    match format {
        OutputFormat::Text => Ok(format!(
            "runtime.zero updater plan\n\ncontract: {UPDATES_CONTRACT}\nplan_id: {}\ndry_run: yes\nwrites_attempted: no\nplanned_actions: {}\nblocked_actions: {}\nexecution_authorized: no\n{}{}\n",
            plan.plan_id,
            plan.actions
                .iter()
                .filter(|action| action.disposition == ActionDisposition::Planned)
                .count(),
            plan.actions
                .iter()
                .filter(|action| action.disposition != ActionDisposition::Planned)
                .count(),
            render_provider_text(provider),
            render_probe_text(built),
        )),
        OutputFormat::Json => match provider {
            Some(provider) => render_json(&ProviderReview {
                coverage: ProviderCoverage::from(provider),
                result: plan,
            }),
            None => render_json(&CliReview::Plan(plan)),
        },
    }
}

fn render_queue(
    queue: &SerialUpdateQueuePlan,
    format: OutputFormat,
    built: &BuiltInput,
) -> Result<String, String> {
    let provider = provider_context(built);
    match format {
        OutputFormat::Text => Ok(format!(
            "runtime.zero serial updater queue\n\ncontract: {UPDATE_QUEUE_CONTRACT}\nqueue_id: {}\nitems: {}\npending: {}\nblocked: {}\ndry_run: yes\nwrites_attempted: no\nexecution_authorized: no\n{}{}\nThe queue is review-only and pauses on failure, drift, cancellation, or recovery.\n",
            queue.queue_id,
            queue.items.len(),
            queue
                .items
                .iter()
                .filter(|item| item.status == SerialUpdateItemStatus::Pending)
                .count(),
            queue
                .items
                .iter()
                .filter(|item| item.status == SerialUpdateItemStatus::Blocked)
                .count(),
            render_provider_text(provider),
            render_probe_text(built),
        )),
        OutputFormat::Json => match provider {
            Some(provider) => render_json(&ProviderReview {
                coverage: ProviderCoverage::from(provider),
                result: queue,
            }),
            None => render_json(&CliReview::Queue(queue)),
        },
    }
}

fn render_provider_text(provider: Option<&BuiltInput>) -> String {
    let Some(provider) = provider else {
        return String::new();
    };
    let mut output = format!(
        "provider_sources: {}/{} succeeded\ncoverage: bounded\n",
        provider.source_ok_count, provider.source_count
    );
    if !provider.sources.is_empty() {
        output.push_str("provider_status:\n");
        for source in &provider.sources {
            output.push_str(&format!(
                "- {}: {} ({} candidates)\n",
                source.provider, source.status, source.candidate_count
            ));
        }
    }
    if !provider.warnings.is_empty() {
        output.push_str("provider_warnings:\n");
        for warning in &provider.warnings {
            output.push_str("- ");
            output.push_str(warning);
            output.push('\n');
        }
    }
    output
}

fn render_probe_text(built: &BuiltInput) -> String {
    if built.live_probe {
        format!(
            "live_read_only_probe: yes\nnetwork_read_requested: {}\nLive provider commands were run for availability only; no update write was performed.\n",
            if built.network_read_requested {
                "yes"
            } else {
                "no"
            },
        )
    } else {
        "live_read_only_probe: no\nnetwork_read_requested: no\nNo manager command, network request, or write was performed.\n".to_string()
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum CliReview<'a> {
    Report(&'a rz0_finding_contract::FindingReport),
    Plan(&'a ActionPlan),
    Queue(&'a SerialUpdateQueuePlan),
}

#[derive(Serialize)]
struct ProviderCoverage<'a> {
    source_count: usize,
    source_ok_count: usize,
    coverage: &'static str,
    live_read_only_probe: bool,
    network_read_requested: bool,
    sources: &'a [ProviderSourceStatus],
    warnings: &'a [String],
}

impl<'a> From<&'a BuiltInput> for ProviderCoverage<'a> {
    fn from(value: &'a BuiltInput) -> Self {
        Self {
            source_count: value.source_count,
            source_ok_count: value.source_ok_count,
            coverage: "bounded",
            live_read_only_probe: value.live_probe,
            network_read_requested: value.network_read_requested,
            sources: &value.sources,
            warnings: &value.warnings,
        }
    }
}

#[derive(Serialize)]
struct ProviderReview<'a, T: Serialize> {
    coverage: ProviderCoverage<'a>,
    result: &'a T,
}

fn render_json(value: &impl Serialize) -> Result<String, String> {
    serde_json::to_string_pretty(value)
        .map(|json| format!("{json}\n"))
        .map_err(|error| format!("render updater review: {error}"))
}

fn usage() -> String {
    "Usage: rz0 updates --dry-run --fixture <updater-evidence.json> [--plan] [--queue] [--format text|json]\n       rz0 updates --dry-run --manager <manager-id> --manager-output <output> --executable <absolute-path> [--plan] [--queue] [--format text|json]\n       rz0 updates --dry-run --probe --manager <manager-id> --executable <absolute-path> --allow-network-read [--plan] [--queue] [--format text|json]\n       rz0 updates --dry-run --all-providers --allow-network-read [--plan] [--queue] [--format text|json]\n       rz0 updates --apply --probe --manager <manager-id> --executable <absolute-path> --allow-network-read --allow-network-write (--action <exact-action-id> | --all) [--accept-no-rollback] [--challenge-issued-unix-seconds <unix-seconds>] [--confirm <exact-phrase>] [--format text|json]\n       rz0 updates --apply --all-providers --allow-network-read --allow-network-write [--accept-no-rollback] [--format text]\n       rz0 updates --recovery-status --transaction <transaction-id> [--format text|json]\n\n--all-providers performs a provider-driven live review of installed system managers, language/package environments, known self-updaters, and declared application update metadata. On macOS this includes Homebrew, Apple Software Update, npm global prefixes, pip, RubyGems, Grok, oh-my-pi, Electron GitHub metadata, and observed Sparkle channels when present; missing, observed-only, and unsupported providers remain explicit. --apply performs a fresh availability dry-run, requires explicit network-write approval and exact interactive confirmation. Execution additionally requires a plan-sealed executable identity and a reviewed platform binding; macOS currently fails closed before transaction creation because that binding is not implemented. --recovery-status is read-only and never completes or retries an interrupted manager effect.".to_string()
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
        let probe = parse_args(&[
            "--dry-run".to_string(),
            "--probe".to_string(),
            "--manager".to_string(),
            "homebrew-formula".to_string(),
            "--executable".to_string(),
            "/opt/homebrew/bin/brew".to_string(),
            "--allow-network-read".to_string(),
        ])
        .expect("probe args");
        assert!(probe.probe);
        assert!(probe.allow_network_read);
        let apply = parse_args(&[
            "--apply".to_string(),
            "--probe".to_string(),
            "--manager".to_string(),
            "homebrew-formula".to_string(),
            "--executable".to_string(),
            "/opt/homebrew/bin/brew".to_string(),
            "--allow-network-read".to_string(),
            "--allow-network-write".to_string(),
            "--action".to_string(),
            "update.update.homebrew-formula.alpha".to_string(),
            "--accept-no-rollback".to_string(),
            "--challenge-issued-unix-seconds".to_string(),
            "1700000000".to_string(),
            "--confirm".to_string(),
            "confirm update.plan.item abcdef123456".to_string(),
        ])
        .expect("apply args");
        assert!(apply.apply);
        assert!(apply.allow_network_write);
        assert_eq!(
            apply.action.as_deref(),
            Some("update.update.homebrew-formula.alpha")
        );
        assert!(apply.confirm.is_some());
        let all = parse_args(&[
            "--apply".to_string(),
            "--all".to_string(),
            "--probe".to_string(),
            "--manager".to_string(),
            "homebrew-formula".to_string(),
            "--executable".to_string(),
            "/opt/homebrew/bin/brew".to_string(),
            "--allow-network-read".to_string(),
            "--allow-network-write".to_string(),
        ])
        .expect("serial apply args");
        assert!(all.all);
        assert!(all.action.is_none());
        let all_providers = parse_args(&[
            "--dry-run".to_string(),
            "--all-providers".to_string(),
            "--allow-network-read".to_string(),
            "--plan".to_string(),
            "--queue".to_string(),
        ])
        .expect("all-provider review args");
        assert!(all_providers.all_providers);
        assert!(!all_providers.probe);
        assert!(parse_args(&["--dry-run".to_string(), "--all-providers".to_string(),]).is_err());
        let recovery = parse_args(&[
            "--recovery-status".to_string(),
            "--transaction".to_string(),
            "tx.update.example.1700000000".to_string(),
            "--format".to_string(),
            "json".to_string(),
        ])
        .expect("recovery args");
        assert!(recovery.recovery_status);
        assert_eq!(
            recovery.transaction.as_deref(),
            Some("tx.update.example.1700000000")
        );
        assert!(
            parse_args(&[
                "--recovery-status".to_string(),
                "--transaction".to_string(),
                "tx.update.example.1700000000".to_string(),
                "--dry-run".to_string(),
            ])
            .is_err()
        );
        assert!(
            parse_args(&[
                "--apply".to_string(),
                "--dry-run".to_string(),
                "--probe".to_string(),
                "--manager".to_string(),
                "homebrew-formula".to_string(),
                "--executable".to_string(),
                "/opt/homebrew/bin/brew".to_string(),
                "--allow-network-read".to_string(),
                "--allow-network-write".to_string(),
                "--action".to_string(),
                "action".to_string(),
            ])
            .is_err()
        );
    }

    #[test]
    fn post_update_verification_requires_a_valid_plan_with_the_candidate_absent() {
        let input =
            read_input(Path::new("tests/fixtures/updater/evidence.json")).expect("updater fixture");
        let report = classify_updates(&input).expect("finding report");
        let mut plan = build_update_action_plan(&input, &report).expect("action plan");
        let finding_id = plan.actions[0].finding_id.clone();
        assert!(verify_candidate_absent(&plan, &finding_id).is_err());
        plan.actions
            .retain(|action| action.finding_id != finding_id);
        assert!(verify_candidate_absent(&plan, &finding_id).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn interrupt_bridge_installs_and_restores_without_cancelling() {
        let (controller, token) = rz0_cancellation_contract::cancellation_pair();
        let bridge = InterruptBridge::install(controller).expect("interrupt bridge");
        assert_eq!(token.reason(), None);
        drop(bridge);
        assert_eq!(token.reason(), None);
    }

    #[test]
    fn rejects_symlinked_or_unbounded_input_without_collecting() {
        assert!(read_input(Path::new("tests/fixtures/does-not-exist.json")).is_err());
        assert!(
            parse_args(&["--dry-run".to_string(), "--allow-network-read".to_string(),]).is_err()
        );
        assert!(
            parse_args(&[
                "--transaction".to_string(),
                "tx.update.example.1700000000".to_string(),
            ])
            .is_err()
        );
    }
}
