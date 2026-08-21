use std::collections::BTreeSet;
use std::ffi::OsStr;
#[cfg(target_os = "macos")]
use std::fs;
#[cfg(target_os = "macos")]
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rz0_action_plan::{
    ActionCapability, ActionDisposition, ActionExecutableIdentity, ActionKind, ActionPlan,
    PlanAction, action_plan_digests,
};
use rz0_artifact_identity::{
    ArtifactExpectation, VerifiedArtifact, bind_verified_executable, open_observed_executable,
    open_verified_executable, revalidate_verified_executable,
};
use rz0_cancellation_contract::CancellationToken;
use rz0_confirmation_contract::{
    CONFIRMATION_CHALLENGE_CONTRACT, CONFIRMATION_CONSUMPTION_CONTRACT,
    CONFIRMATION_RESPONSE_CONTRACT, ConfirmationChallenge, ConfirmationConsumption,
    ConfirmationResponse, ConfirmationRisk, ConfirmationSurface, seal_confirmation_challenge,
    seal_confirmation_consumption, validate_confirmation,
};
use rz0_module_updater::manager_executable_allowed;
use rz0_secure_fs::SecureDirectory;
use rz0_transaction_contract::{
    DurabilityRequirements, EXTERNAL_EFFECT_RECEIPT_CONTRACT,
    EXTERNAL_EFFECT_RECEIPT_SCHEMA_VERSION, ExternalEffectPublicationInput, ExternalEffectReceipt,
    ExternalEffectStatus, TransactionEvent, TransactionEventKind, TransactionJournal,
    TransactionOperation, TransactionState, arguments_sha256, publish_confirmation_consumption,
    publish_external_effect_receipt_cancellable, publish_journal_snapshot,
    seal_external_effect_receipt, seal_transaction_journal,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

const MAX_EXECUTION_SECONDS: u64 = 30 * 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateChallengeView {
    pub operation: TransactionOperation,
    pub plan_id: String,
    pub action_id: String,
    pub plan_sha256: String,
    pub manager: Option<String>,
    pub target: String,
    pub arguments: Vec<String>,
    pub risk: ConfirmationRisk,
    pub requires_elevation: bool,
    pub network_required: bool,
    pub executable_sha256: Option<String>,
    pub executable_size_bytes: Option<u64>,
    pub capabilities: Vec<ActionCapability>,
    pub expected_phrase: String,
    pub issued_unix_seconds: u64,
    pub expires_unix_seconds: u64,
    pub rollback_available: bool,
    pub manual_recovery_acknowledged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateExecutionReport {
    pub operation: TransactionOperation,
    pub transaction_id: String,
    pub action_id: String,
    pub manager: String,
    pub target: String,
    pub executable_sha256: String,
    pub executable_size_bytes: u64,
    pub executable_binding: String,
    pub status: UpdateExecutionStatus,
    pub exit_code: Option<i32>,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub verification: String,
    pub receipt_reference: String,
    pub writes_attempted: bool,
    pub product_execution_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateExecutionStatus {
    Committed,
    RecoveryRequired,
}

pub fn make_single_action_plan(
    plan: &ActionPlan,
    action: &PlanAction,
) -> Result<ActionPlan, String> {
    if !plan.actions.iter().any(|candidate| candidate == action) {
        return Err("selected update action is not part of the exact action plan".to_string());
    }
    let mut single = plan.clone();
    let action_digest = short_digest(action.action_id.as_bytes());
    single.plan_id = format!("{}.item.{}", plan.plan_id, action_digest);
    single.actions = vec![action.clone()];
    let validation = rz0_action_plan::validate_action_plan(&single);
    if validation.valid {
        Ok(single)
    } else {
        Err(validation.errors.join("; "))
    }
}

pub fn build_update_challenge(
    plan: &ActionPlan,
    action: &PlanAction,
    manual_recovery_acknowledged: bool,
    now_unix_seconds: u64,
) -> Result<(ConfirmationChallenge, UpdateChallengeView), String> {
    let digests = action_plan_digests(plan).map_err(|errors| errors.join("; "))?;
    let capabilities = action
        .capabilities
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let before_state = digest_text(&format!("{}\0{}", action.target, "before"));
    let after_state = digest_text(&format!("{}\0{}", action.target, "after"));
    let mut challenge = ConfirmationChallenge {
        schema_version: rz0_confirmation_contract::CONFIRMATION_SCHEMA_VERSION,
        contract: CONFIRMATION_CHALLENGE_CONTRACT.to_string(),
        challenge_id: format!("challenge.update.{}", short_digest(plan.plan_id.as_bytes())),
        plan_id: plan.plan_id.clone(),
        plan_sha256: digests.plan_sha256.clone(),
        dry_run_sha256: digests.plan_sha256,
        write_set_sha256: digests.write_set_sha256,
        before_state_sha256: Some(before_state),
        expected_after_state_sha256: after_state,
        risk: if action.kind == ActionKind::Uninstall {
            ConfirmationRisk::Destructive
        } else {
            ConfirmationRisk::Mutating
        },
        action_count: 1,
        capabilities,
        issued_unix_seconds: now_unix_seconds,
        expires_unix_seconds: now_unix_seconds.saturating_add(300),
        dry_run_completed: true,
        dry_run_writes_attempted: false,
        rollback_available: action.rollback.supported,
        quarantine_available: false,
        manual_recovery_acknowledged,
        expected_phrase: String::new(),
        challenge_sha256: String::new(),
    };
    seal_confirmation_challenge(&mut challenge);
    let view = UpdateChallengeView {
        operation: operation_for_action(action)?,
        plan_id: challenge.plan_id.clone(),
        action_id: action.action_id.clone(),
        plan_sha256: challenge.plan_sha256.clone(),
        manager: action.manager.clone(),
        target: action.target.clone(),
        arguments: action.arguments.clone(),
        risk: challenge.risk,
        requires_elevation: action.requires_elevation,
        network_required: action.network_required,
        executable_sha256: action
            .executable_identity
            .as_ref()
            .map(|identity| identity.sha256.clone()),
        executable_size_bytes: action
            .executable_identity
            .as_ref()
            .map(|identity| identity.size_bytes),
        capabilities: action.capabilities.clone(),
        expected_phrase: challenge.expected_phrase.clone(),
        issued_unix_seconds: challenge.issued_unix_seconds,
        expires_unix_seconds: challenge.expires_unix_seconds,
        rollback_available: challenge.rollback_available,
        manual_recovery_acknowledged,
    };
    Ok((challenge, view))
}

pub fn validate_update_confirmation(
    challenge: &ConfirmationChallenge,
    phrase: &str,
    now_unix_seconds: u64,
) -> Result<ConfirmationResponse, String> {
    validate_update_confirmation_on_surface(
        challenge,
        phrase,
        now_unix_seconds,
        ConfirmationSurface::Cli,
    )
}

pub fn validate_tui_update_confirmation(
    challenge: &ConfirmationChallenge,
    phrase: &str,
    now_unix_seconds: u64,
) -> Result<ConfirmationResponse, String> {
    validate_update_confirmation_on_surface(
        challenge,
        phrase,
        now_unix_seconds,
        ConfirmationSurface::Tui,
    )
}

fn validate_update_confirmation_on_surface(
    challenge: &ConfirmationChallenge,
    phrase: &str,
    now_unix_seconds: u64,
    surface: ConfirmationSurface,
) -> Result<ConfirmationResponse, String> {
    let response = ConfirmationResponse {
        schema_version: rz0_confirmation_contract::CONFIRMATION_SCHEMA_VERSION,
        contract: CONFIRMATION_RESPONSE_CONTRACT.to_string(),
        challenge_id: challenge.challenge_id.clone(),
        challenge_sha256: challenge.challenge_sha256.clone(),
        confirmed_unix_seconds: now_unix_seconds,
        surface,
        phrase: phrase.to_string(),
        interactive: true,
        single_use: true,
        execution_authorized: false,
    };
    let assessment = validate_confirmation(challenge, &response, now_unix_seconds);
    if assessment.valid {
        Ok(response)
    } else {
        Err(assessment.errors.join("; "))
    }
}

pub struct UpdateExecutionRequest<'a, F>
where
    F: FnOnce(&CancellationToken) -> Result<String, String>,
{
    pub state_root: &'a Path,
    pub plan: &'a ActionPlan,
    pub action: &'a PlanAction,
    pub challenge: &'a ConfirmationChallenge,
    pub response: &'a ConfirmationResponse,
    pub now_unix_seconds: u64,
    pub environment: Vec<(String, String)>,
    pub cancellation: &'a CancellationToken,
    pub verify_after: F,
}

pub fn execute_update_action<F>(
    request: UpdateExecutionRequest<'_, F>,
) -> Result<UpdateExecutionReport, String>
where
    F: FnOnce(&CancellationToken) -> Result<String, String>,
{
    execute_manager_action(request)
}

pub fn execute_uninstall_action<F>(
    request: UpdateExecutionRequest<'_, F>,
) -> Result<UpdateExecutionReport, String>
where
    F: FnOnce(&CancellationToken) -> Result<String, String>,
{
    execute_manager_action(request)
}

fn execute_manager_action<F>(
    request: UpdateExecutionRequest<'_, F>,
) -> Result<UpdateExecutionReport, String>
where
    F: FnOnce(&CancellationToken) -> Result<String, String>,
{
    let UpdateExecutionRequest {
        state_root,
        plan,
        action,
        challenge,
        response,
        now_unix_seconds,
        mut environment,
        cancellation,
        verify_after,
    } = request;
    let operation = operation_for_action(action)?;
    let operation_label = operation_label(operation);
    validate_execution_inputs(
        operation,
        plan,
        action,
        challenge,
        response,
        now_unix_seconds,
    )?;
    if !state_root.is_absolute() {
        return Err("update execution state root must be absolute".to_string());
    }
    let manager = action
        .manager
        .as_deref()
        .ok_or_else(|| "update action has no manager identity".to_string())?;
    let executable = action
        .executable
        .as_deref()
        .ok_or_else(|| "update action has no exact executable".to_string())?;
    if !manager_executable_allowed(manager, std::env::consts::OS, executable) {
        return Err("update executable is not an allowlisted manager path".to_string());
    }
    if action.arguments.is_empty() || action.arguments.len() > 16 {
        return Err("update action has no bounded manager arguments".to_string());
    }
    if action
        .arguments
        .iter()
        .any(|argument| argument.is_empty() || argument.chars().any(char::is_control))
    {
        return Err("update action arguments contain unsafe text".to_string());
    }
    let use_sudo = action.requires_elevation && !is_effective_root();
    refuse_if_cancelled(cancellation, "before executable identity binding")?;
    let executable_identity = action
        .executable_identity
        .as_ref()
        .ok_or_else(|| "update action has no sealed executable identity".to_string())?;
    let mut verified_executable =
        open_manager_executable(Path::new(executable), executable_identity)?;
    validate_platform_manager_execution(&verified_executable)?;
    let executable_binding = bind_verified_executable(&verified_executable)
        .map_err(|error| format!("bind verified manager executable through spawn: {error}"))?;
    let mut executable_binding_name = if use_sudo {
        format!("{};wrapper=sudo", executable_binding.mechanism().as_str())
    } else {
        executable_binding.mechanism().as_str().to_string()
    };
    let launch_executable = if use_sudo {
        PathBuf::from("/usr/bin/sudo")
    } else {
        PathBuf::from(executable)
    };
    let mut launch_arguments = if use_sudo {
        let mut arguments = vec!["-n".to_string(), "--".to_string(), executable.to_string()];
        arguments.extend(action.arguments.clone());
        arguments
    } else {
        action.arguments.clone()
    };
    if use_sudo {
        let sudo_metadata = std::fs::symlink_metadata(&launch_executable).map_err(|error| {
            format!(
                "inspect elevation helper {}: {error}",
                launch_executable.display()
            )
        })?;
        if sudo_metadata.file_type().is_symlink() || !sudo_metadata.is_file() {
            return Err("the fixed /usr/bin/sudo elevation helper is unavailable".to_string());
        }
    }
    refuse_if_cancelled(cancellation, "before transaction preparation")?;

    let transaction_id = format!(
        "tx.{operation_label}.{}.{}",
        short_digest(plan.plan_id.as_bytes()),
        now_unix_seconds
    );
    let mut manager_environment =
        prepare_manager_environment(manager, &transaction_id, &mut environment)?;
    ensure_new_transaction(state_root, &transaction_id)?;
    let prepared = journal(
        &transaction_id,
        &plan.plan_id,
        operation,
        vec![event(TransactionEventKind::Prepared)],
    );
    let transactions_root = state_root.join("transactions");
    publish_journal_snapshot(&transactions_root, &prepared)
        .map_err(|error| format!("publish prepared update journal: {error}"))?;

    let response_sha256 = rz0_confirmation_contract::confirmation_response_sha256(response);
    let mut consumption = ConfirmationConsumption {
        schema_version: rz0_confirmation_contract::CONFIRMATION_SCHEMA_VERSION,
        contract: CONFIRMATION_CONSUMPTION_CONTRACT.to_string(),
        transaction_id: transaction_id.clone(),
        plan_id: plan.plan_id.clone(),
        challenge_sha256: challenge.challenge_sha256.clone(),
        response_sha256,
        consumed_unix_seconds: now_unix_seconds,
        single_use_consumed: true,
        execution_authorized: false,
        binding_sha256: String::new(),
    };
    seal_confirmation_consumption(&mut consumption);
    publish_confirmation_consumption(
        state_root,
        &prepared,
        plan,
        challenge,
        response,
        &consumption,
    )
    .map_err(|error| format!("publish update confirmation: {error}"))?;

    let applying = append(&prepared, event(TransactionEventKind::ApplyStarted));
    publish_journal_snapshot(&transactions_root, &applying)
        .map_err(|error| format!("publish update apply journal: {error}"))?;
    let write_path = format!("manager/{operation_label}/{manager}/{}", action.finding_id);
    let intent = append(
        &applying,
        manager_write_event(
            TransactionEventKind::WriteIntent,
            action,
            challenge,
            &write_path,
        ),
    );
    publish_journal_snapshot(&transactions_root, &intent)
        .map_err(|error| format!("publish exact manager write intent: {error}"))?;

    if operation == TransactionOperation::Update && manager == "electron-squirrel" {
        let request_path = match prepare_electron_squirrel_update(
            action,
            &transaction_id,
            &environment,
            cancellation,
        ) {
            Ok((request_path, staging_path)) => {
                manager_environment.cleanup_paths.push(staging_path);
                request_path
            }
            Err(error) => {
                let recovery = append(&intent, event(TransactionEventKind::RecoveryRequired));
                let _ = publish_journal_snapshot(&transactions_root, &recovery);
                return Err(format!(
                    "prepare Electron/Squirrel update failed; recovery is required: {error}"
                ));
            }
        };
        launch_arguments = vec![
            action.arguments[0].clone(),
            request_path.display().to_string(),
        ];
    }
    let warp_update_binary = if operation == TransactionOperation::Update
        && manager == "warp-agent-cli"
    {
        match prepare_warp_agent_cli_update(action, &transaction_id, &environment, cancellation) {
            Ok((staging_path, binary_path)) => {
                manager_environment.cleanup_paths.push(staging_path);
                Some(binary_path)
            }
            Err(error) => {
                let recovery = append(&intent, event(TransactionEventKind::RecoveryRequired));
                let _ = publish_journal_snapshot(&transactions_root, &recovery);
                return Err(format!(
                    "prepare Warp Agent CLI update failed; recovery is required: {error}"
                ));
            }
        }
    } else {
        None
    };

    let process_request = rz0_process_host::ProcessRequest {
        executable: launch_executable,
        arguments: launch_arguments,
        working_directory: PathBuf::from("/"),
        environment: environment.clone(),
        timeout: Duration::from_secs(MAX_EXECUTION_SECONDS),
        output_limit: rz0_resource_contract::MAX_FINDING_REPORT_BYTES,
    };
    if use_sudo {
        executable_binding.verify_spawn_path().map_err(|error| {
            format!("revalidate manager executable before sudo launch: {error}")
        })?;
    }
    let process = if let Some(binary_path) = warp_update_binary {
        executable_binding_name.push_str(";warp-native");
        run_bound_macos_tool(
            Path::new("/usr/bin/codesign"),
            vec![
                "--verify".to_string(),
                "--strict".to_string(),
                "--verbose=2".to_string(),
                binary_path.display().to_string(),
            ],
            &environment,
            cancellation,
        )
        .map_err(|error| format!("verify Warp Agent CLI code signature: {error}"))
        .and_then(|verification| {
            require_successful_tool("codesign Warp Agent CLI update", &verification)
                .map(|()| verification)
        })
    } else if use_sudo {
        rz0_process_host::run_mutating_process_cancellable(&process_request, cancellation)
            .map_err(|error| error.to_string())
    } else {
        rz0_process_host::run_bound_mutating_process(
            &process_request,
            &executable_binding,
            cancellation,
        )
        .map_err(|error| error.to_string())
    }
    .map_err(|error| {
        let recovery = append(&intent, event(TransactionEventKind::RecoveryRequired));
        let _ = publish_journal_snapshot(&transactions_root, &recovery);
        format!("manager {operation_label} process failed; recovery is required: {error}")
    })?;
    drop(executable_binding);
    if let Err(error) = revalidate_verified_executable(&mut verified_executable) {
        if manager_supports_self_replacement(manager)
            && matches!(
                error.code,
                rz0_artifact_identity::ArtifactIdentityErrorCode::IdentityChanged
                    | rz0_artifact_identity::ArtifactIdentityErrorCode::DigestMismatch
                    | rz0_artifact_identity::ArtifactIdentityErrorCode::SizeMismatch
            )
        {
            executable_binding_name.push_str(";self-replaced");
        } else {
            let recovery = append(&intent, event(TransactionEventKind::RecoveryRequired));
            let _ = publish_journal_snapshot(&transactions_root, &recovery);
            return Err(format!(
                "manager {operation_label} executable identity changed across spawn; recovery is required: {error}"
            ));
        }
    }
    let stdout_sha256 = sha256(&process.stdout.bytes);
    let stderr_sha256 = sha256(&process.stderr.bytes);
    let exit_code = process.status.code();
    if !process.status.success() || process.cancellation_reason.is_some() {
        let recovery = append(&intent, event(TransactionEventKind::RecoveryRequired));
        let _ = publish_journal_snapshot(&transactions_root, &recovery);
        return Err(format!(
            "manager {operation_label} did not complete successfully (exit={exit_code:?}, cancellation={:?}, stderr={:?}); recovery is required",
            process.cancellation_reason,
            String::from_utf8_lossy(&process.stderr.bytes)
        ));
    }

    refuse_if_cancelled(
        cancellation,
        &format!("before post-{operation_label} verification"),
    )
    .map_err(|error| {
        let recovery = append(&intent, event(TransactionEventKind::RecoveryRequired));
        let _ = publish_journal_snapshot(&transactions_root, &recovery);
        format!("{error}; recovery is required")
    })?;
    let verification = match verify_after(cancellation) {
        Ok(verification) => verification,
        Err(error) => {
            let recovery = append(&intent, event(TransactionEventKind::RecoveryRequired));
            let _ = publish_journal_snapshot(&transactions_root, &recovery);
            return Err(format!(
                "post-{operation_label} verification failed; recovery is required: {error}"
            ));
        }
    };
    refuse_if_cancelled(
        cancellation,
        &format!("during post-{operation_label} verification"),
    )
    .map_err(|error| {
        let recovery = append(&intent, event(TransactionEventKind::RecoveryRequired));
        let _ = publish_journal_snapshot(&transactions_root, &recovery);
        format!("{error}; recovery is required")
    })?;

    let verified = append(
        &intent,
        manager_write_event(
            TransactionEventKind::WriteVerified,
            action,
            challenge,
            &write_path,
        ),
    );
    if let Err(error) = publish_journal_snapshot(&transactions_root, &verified) {
        let recovery = append(&intent, event(TransactionEventKind::RecoveryRequired));
        let _ = publish_journal_snapshot(&transactions_root, &recovery);
        return Err(format!(
            "publish verified manager outcome journal failed; recovery is required: {error}"
        ));
    }
    let committing = append(&verified, event(TransactionEventKind::CommitStarted));
    if let Err(error) = publish_journal_snapshot(&transactions_root, &committing) {
        let recovery = append(&verified, event(TransactionEventKind::RecoveryRequired));
        let _ = publish_journal_snapshot(&transactions_root, &recovery);
        return Err(format!(
            "publish {operation_label} commit-pending journal failed; recovery is required: {error}"
        ));
    }

    let committed_exit_code = exit_code.ok_or_else(|| {
        let recovery = append(&committing, event(TransactionEventKind::RecoveryRequired));
        let _ = publish_journal_snapshot(&transactions_root, &recovery);
        "successful manager process had no portable exit code; recovery is required".to_string()
    })?;
    let head = committing
        .events
        .last()
        .ok_or_else(|| "commit-pending update journal has no head".to_string())?;
    let digests = action_plan_digests(plan).map_err(|errors| errors.join("; "))?;
    let mut receipt = ExternalEffectReceipt {
        schema_version: EXTERNAL_EFFECT_RECEIPT_SCHEMA_VERSION,
        contract: EXTERNAL_EFFECT_RECEIPT_CONTRACT.to_string(),
        transaction_id: transaction_id.clone(),
        plan_id: plan.plan_id.clone(),
        action_id: action.action_id.clone(),
        operation,
        manager: manager.to_string(),
        target: action.target.clone(),
        executable_sha256: executable_identity.sha256.clone(),
        executable_size_bytes: executable_identity.size_bytes,
        executable_binding: executable_binding_name.clone(),
        arguments_sha256: arguments_sha256(&action.arguments),
        started_unix_seconds: now_unix_seconds,
        completed_unix_seconds: unix_seconds(),
        exit_code: committed_exit_code,
        stdout_bytes: process.stdout.total_bytes,
        stderr_bytes: process.stderr.total_bytes,
        stdout_sha256: stdout_sha256.clone(),
        stderr_sha256: stderr_sha256.clone(),
        verification_sha256: sha256(verification.as_bytes()),
        commit_pending_sequence: head.sequence,
        commit_pending_event_sha256: head.event_sha256.clone(),
        commit_pending_snapshot_name: format!("{:04}-{}.json", head.sequence, head.event_sha256),
        action_plan_sha256: digests.plan_sha256,
        write_set_sha256: digests.write_set_sha256,
        confirmation_challenge_sha256: challenge.challenge_sha256.clone(),
        confirmation_response_sha256: rz0_confirmation_contract::confirmation_response_sha256(
            response,
        ),
        confirmation_consumption_sha256: consumption.binding_sha256.clone(),
        rollback_supported: action.rollback.supported,
        status: ExternalEffectStatus::Verified,
        writes_attempted: true,
        automatic_mutation_authorized: false,
        binding_sha256: String::new(),
    };
    seal_external_effect_receipt(&mut receipt);
    let publication = match publish_external_effect_receipt_cancellable(
        state_root,
        ExternalEffectPublicationInput {
            commit_pending_journal: &committing,
            action_plan: plan,
            challenge,
            response,
            consumption: &consumption,
            receipt: &receipt,
        },
        cancellation,
    ) {
        Ok(publication) => publication,
        Err(error) => {
            let recovery = append(&committing, event(TransactionEventKind::RecoveryRequired));
            let _ = publish_journal_snapshot(&transactions_root, &recovery);
            return Err(format!(
                "publish canonical external-effect receipt failed; recovery is required: {error}"
            ));
        }
    };
    let committed = append(&committing, event(TransactionEventKind::Committed));
    publish_journal_snapshot(&transactions_root, &committed).map_err(|error| {
        format!(
            "external-effect {operation_label} receipt is durable but final committed journal publication failed; recovery is required: {error}"
        )
    })?;

    Ok(UpdateExecutionReport {
        operation,
        transaction_id,
        action_id: action.action_id.clone(),
        manager: manager.to_string(),
        target: action.target.clone(),
        executable_sha256: executable_identity.sha256.clone(),
        executable_size_bytes: executable_identity.size_bytes,
        executable_binding: executable_binding_name,
        status: UpdateExecutionStatus::Committed,
        exit_code,
        stdout_bytes: process.stdout.total_bytes,
        stderr_bytes: process.stderr.total_bytes,
        stdout_sha256,
        stderr_sha256,
        verification,
        receipt_reference: format!("receipts/{}", publication.receipt_name),
        writes_attempted: true,
        product_execution_authorized: true,
    })
}

fn operation_for_action(action: &PlanAction) -> Result<TransactionOperation, String> {
    match action.kind {
        ActionKind::Update => Ok(TransactionOperation::Update),
        ActionKind::Uninstall => Ok(TransactionOperation::Uninstall),
        _ => Err("manager executor accepts only update or uninstall actions".to_string()),
    }
}

const fn operation_label(operation: TransactionOperation) -> &'static str {
    match operation {
        TransactionOperation::Update => "update",
        TransactionOperation::Uninstall => "uninstall",
        _ => "manager-action",
    }
}

fn ensure_new_transaction(state_root: &Path, transaction_id: &str) -> Result<(), String> {
    let state = SecureDirectory::open(state_root)
        .map_err(|error| format!("open update state for replay check: {error}"))?;
    state
        .verify_private()
        .map_err(|error| format!("verify update state for replay check: {error}"))?;
    let transactions = state
        .open_child_directory(OsStr::new("transactions"))
        .map_err(|error| format!("open update transactions for replay check: {error}"))?;
    if transactions
        .open_child_directory(OsStr::new(transaction_id))
        .is_ok()
    {
        return Err("the exact update confirmation has already been consumed".to_string());
    }
    Ok(())
}

struct ManagerEnvironmentGuard {
    temporary_cache: Option<PathBuf>,
    cleanup_paths: Vec<PathBuf>,
}

impl Drop for ManagerEnvironmentGuard {
    fn drop(&mut self) {
        if let Some(path) = self.temporary_cache.take() {
            let _ = std::fs::remove_dir_all(path);
        }
        for path in self.cleanup_paths.drain(..) {
            let _ = std::fs::remove_dir_all(path);
        }
    }
}

fn prepare_manager_environment(
    manager: &str,
    transaction_id: &str,
    environment: &mut Vec<(String, String)>,
) -> Result<ManagerEnvironmentGuard, String> {
    if manager != "npm" {
        return Ok(ManagerEnvironmentGuard {
            temporary_cache: None,
            cleanup_paths: Vec::new(),
        });
    }
    let cache = std::env::temp_dir().join(format!(
        "runtime-zero-npm-cache-{}-{}",
        std::process::id(),
        short_digest(transaction_id.as_bytes())
    ));
    if cache.exists() {
        return Err(format!(
            "refusing to reuse an existing npm cache path: {}",
            cache.display()
        ));
    }
    std::fs::create_dir(&cache)
        .map_err(|error| format!("create isolated npm update cache: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&cache, std::fs::Permissions::from_mode(0o700)).map_err(
            |error| {
                let _ = std::fs::remove_dir_all(&cache);
                format!("set isolated npm cache permissions: {error}")
            },
        )?;
    }
    environment.retain(|(key, _)| key != "NPM_CONFIG_CACHE" && key != "NPM_CONFIG_UPDATE_NOTIFIER");
    environment.push(("NPM_CONFIG_CACHE".to_string(), cache.display().to_string()));
    environment.push((
        "NPM_CONFIG_UPDATE_NOTIFIER".to_string(),
        "false".to_string(),
    ));
    Ok(ManagerEnvironmentGuard {
        temporary_cache: Some(cache),
        cleanup_paths: Vec::new(),
    })
}

#[cfg(target_os = "macos")]
const MAX_ELECTRON_UPDATE_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;

#[cfg(target_os = "macos")]
fn prepare_electron_squirrel_update(
    action: &PlanAction,
    transaction_id: &str,
    environment: &[(String, String)],
    cancellation: &CancellationToken,
) -> Result<(PathBuf, PathBuf), String> {
    if action.arguments.len() != 6 {
        return Err(
            "Electron/Squirrel action must contain an exact six-field update descriptor"
                .to_string(),
        );
    }
    let job_label = &action.arguments[0];
    let target_path = PathBuf::from(&action.arguments[1]);
    let download_url = &action.arguments[2];
    let installed_version = &action.arguments[3];
    let target_version = &action.arguments[4];
    let bundle_id = &action.arguments[5];
    if job_label != &format!("{bundle_id}.ShipIt")
        || !target_path.is_absolute()
        || target_path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
        || !download_url.starts_with("https://github.com/")
        || download_url.len() > 2048
        || download_url.chars().any(char::is_control)
        || bundle_id.is_empty()
        || bundle_id.len() > 240
        || bundle_id.chars().any(char::is_control)
        || installed_version.is_empty()
        || installed_version.len() > 120
        || installed_version.chars().any(char::is_control)
        || target_version.is_empty()
        || target_version.len() > 120
        || target_version.chars().any(char::is_control)
    {
        return Err(
            "Electron/Squirrel update descriptor is not an exact signed GitHub application request"
                .to_string(),
        );
    }
    let target_metadata = fs::symlink_metadata(&target_path)
        .map_err(|error| format!("inspect Electron target bundle: {error}"))?;
    if target_metadata.file_type().is_symlink()
        || !target_metadata.is_dir()
        || target_path.extension().and_then(|value| value.to_str()) != Some("app")
    {
        return Err(
            "Electron target bundle must be a direct regular application directory".to_string(),
        );
    }
    let target_path = fs::canonicalize(&target_path)
        .map_err(|error| format!("canonicalize Electron target bundle: {error}"))?;
    if !electron_bundle_matches(&target_path, bundle_id, installed_version)? {
        return Err(
            "Electron target bundle identity or installed version no longer matches the exact update plan"
                .to_string(),
        );
    }
    let staging = std::env::temp_dir().join(format!(
        "runtime-zero-electron-{}-{}",
        std::process::id(),
        short_digest(transaction_id.as_bytes())
    ));
    if staging.exists() {
        return Err(format!(
            "refusing to reuse an existing Electron staging path: {}",
            staging.display()
        ));
    }
    fs::create_dir(&staging)
        .map_err(|error| format!("create Electron staging directory: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(error) = fs::set_permissions(&staging, fs::Permissions::from_mode(0o700)) {
            let _ = fs::remove_dir_all(&staging);
            return Err(format!("set Electron staging permissions: {error}"));
        }
    }
    let result = (|| {
        let archive = staging.join("update.zip");
        let download = run_bound_macos_tool(
            Path::new("/usr/bin/curl"),
            vec![
                "--fail".to_string(),
                "--location".to_string(),
                "--silent".to_string(),
                "--show-error".to_string(),
                "--max-time".to_string(),
                "1800".to_string(),
                "--output".to_string(),
                archive.display().to_string(),
                download_url.clone(),
            ],
            environment,
            cancellation,
        )?;
        require_successful_tool("curl Electron release archive", &download)?;
        let archive_metadata = fs::symlink_metadata(&archive)
            .map_err(|error| format!("inspect downloaded Electron archive: {error}"))?;
        if archive_metadata.file_type().is_symlink()
            || !archive_metadata.is_file()
            || archive_metadata.len() == 0
            || archive_metadata.len() > MAX_ELECTRON_UPDATE_ARCHIVE_BYTES
        {
            return Err(
                "downloaded Electron archive is missing, oversized, or not a direct file"
                    .to_string(),
            );
        }
        let extracted = staging.join("extracted");
        fs::create_dir(&extracted)
            .map_err(|error| format!("create Electron extraction directory: {error}"))?;
        let extraction = run_bound_macos_tool(
            Path::new("/usr/bin/ditto"),
            vec![
                "-x".to_string(),
                "-k".to_string(),
                archive.display().to_string(),
                extracted.display().to_string(),
            ],
            environment,
            cancellation,
        )?;
        require_successful_tool("ditto Electron release archive", &extraction)?;
        let update_bundle = find_electron_bundle(&extracted, bundle_id, target_version)?;
        let request_path = staging.join("ShipItState.json");
        let request = serde_json::json!({
            "updateBundleURL": file_url(&update_bundle)?,
            "targetBundleURL": file_url(&target_path)?,
            "bundleIdentifier": bundle_id,
            "launchAfterInstallation": true,
            "useUpdateBundleName": false
        });
        let bytes = serde_json::to_vec(&request)
            .map_err(|error| format!("encode Electron/Squirrel update request: {error}"))?;
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&request_path)
            .map_err(|error| format!("create Electron/Squirrel update request: {error}"))?;
        use std::io::Write as _;
        file.write_all(&bytes)
            .map_err(|error| format!("write Electron/Squirrel update request: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("sync Electron/Squirrel update request: {error}"))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&request_path, fs::Permissions::from_mode(0o600))
                .map_err(|error| format!("set Electron/Squirrel request permissions: {error}"))?;
        }
        Ok(request_path)
    })();
    match result {
        Ok(request_path) => Ok((request_path, staging)),
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            Err(error)
        }
    }
}

#[cfg(target_os = "macos")]
const MAX_WARP_AGENT_ARCHIVE_BYTES: u64 = 512 * 1024 * 1024;

#[cfg(target_os = "macos")]
fn prepare_warp_agent_cli_update(
    action: &PlanAction,
    transaction_id: &str,
    environment: &[(String, String)],
    cancellation: &CancellationToken,
) -> Result<(PathBuf, PathBuf), String> {
    if action.arguments.len() != 7 {
        return Err(
            "Warp Agent CLI action must contain an exact seven-field update descriptor".to_string(),
        );
    }
    let initial_url = &action.arguments[0];
    let current_link = PathBuf::from(&action.arguments[1]);
    let versions_root = PathBuf::from(&action.arguments[2]);
    let installed_version = &action.arguments[3];
    let target_version = &action.arguments[4];
    let binary_name = &action.arguments[5];
    let download_url = &action.arguments[6];
    let arch = match std::env::consts::ARCH {
        "aarch64" => "aarch64",
        "x86_64" => "x86_64",
        _ => return Err("Warp Agent CLI has no supported macOS architecture lane".to_string()),
    };
    let expected_initial_url =
        format!("https://app.warp.dev/download/cli?arch={arch}&os=macos&package=tar");
    let expected_download_prefix = "https://releases.warp.dev/stable/v";
    let expected_download_suffix = format!("/cli/macos/{arch}/oz-stable-macos-{arch}.tar.gz");
    if initial_url != &expected_initial_url
        || !download_url.starts_with(expected_download_prefix)
        || !download_url.ends_with(&expected_download_suffix)
        || target_version.is_empty()
        || target_version.len() > 120
        || !target_version.starts_with('v')
        || target_version.chars().any(|value| {
            value.is_control()
                || !(value.is_ascii_alphanumeric() || matches!(value, '.' | '_' | '-'))
        })
        || !installed_version.starts_with('v')
        || installed_version.len() > 120
        || installed_version.chars().any(|value| {
            value.is_control()
                || !(value.is_ascii_alphanumeric() || matches!(value, '.' | '_' | '-'))
        })
        || binary_name != "warp-tui-stable"
        || download_url.len() > 2048
        || download_url.chars().any(char::is_control)
    {
        return Err(
            "Warp Agent CLI update descriptor is not an exact official archive request".to_string(),
        );
    }
    let url_version = download_url
        .strip_prefix(expected_download_prefix)
        .and_then(|value| value.strip_suffix(&expected_download_suffix))
        .map(|value| format!("v{value}"))
        .ok_or_else(|| "Warp Agent CLI release URL has an invalid version segment".to_string())?;
    if url_version != *target_version {
        return Err("Warp Agent CLI release URL and target version disagree".to_string());
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| "HOME is unavailable for the Warp Agent CLI update".to_string())?;
    if !versions_root.starts_with(home.join(".warp/tui/versions"))
        || versions_root.file_name().and_then(|value| value.to_str()) != Some("versions")
        || current_link
            != versions_root
                .parent()
                .unwrap_or(Path::new("/"))
                .join("current")
    {
        return Err(
            "Warp Agent CLI paths are outside the exact standalone version store".to_string(),
        );
    }
    let versions_metadata = fs::symlink_metadata(&versions_root)
        .map_err(|error| format!("inspect Warp Agent CLI versions root: {error}"))?;
    if versions_metadata.file_type().is_symlink() || !versions_metadata.is_dir() {
        return Err("Warp Agent CLI versions root must be a direct directory".to_string());
    }
    let current_metadata = fs::symlink_metadata(&current_link)
        .map_err(|error| format!("inspect Warp Agent CLI current link: {error}"))?;
    if !current_metadata.file_type().is_symlink() {
        return Err("Warp Agent CLI current selector must be a symlink".to_string());
    }
    let current_binary = fs::canonicalize(current_link.join(binary_name))
        .map_err(|error| format!("resolve Warp Agent CLI current binary: {error}"))?;
    if action.executable.as_deref() != current_binary.to_str()
        || read_warp_version(&current_binary).as_deref() != Some(installed_version)
    {
        return Err("Warp Agent CLI current binary drifted from the exact update plan".to_string());
    }
    let target_root = versions_root.join(target_version);
    if target_root.exists() {
        return Err(format!(
            "refusing to reuse existing Warp Agent CLI version path: {}",
            target_root.display()
        ));
    }
    let tui_root = versions_root
        .parent()
        .ok_or_else(|| "Warp Agent CLI version root has no parent".to_string())?;
    let staging = std::env::temp_dir().join(format!(
        "runtime-zero-warp-{}-{}",
        std::process::id(),
        short_digest(transaction_id.as_bytes())
    ));
    if staging.exists() {
        return Err(format!(
            "refusing to reuse an existing Warp Agent CLI staging path: {}",
            staging.display()
        ));
    }
    fs::create_dir(&staging)
        .map_err(|error| format!("create Warp Agent CLI staging directory: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(error) = fs::set_permissions(&staging, fs::Permissions::from_mode(0o700)) {
            let _ = fs::remove_dir_all(&staging);
            return Err(format!("set Warp Agent CLI staging permissions: {error}"));
        }
    }
    let result = (|| {
        let archive = staging.join("warp.tar.gz");
        let download = run_bound_macos_tool(
            Path::new("/usr/bin/curl"),
            vec![
                "--fail".to_string(),
                "--location".to_string(),
                "--silent".to_string(),
                "--show-error".to_string(),
                "--max-time".to_string(),
                "1800".to_string(),
                "--output".to_string(),
                archive.display().to_string(),
                download_url.clone(),
            ],
            environment,
            cancellation,
        )?;
        require_successful_tool("curl Warp Agent CLI archive", &download)?;
        let archive_metadata = fs::symlink_metadata(&archive)
            .map_err(|error| format!("inspect Warp Agent CLI archive: {error}"))?;
        if archive_metadata.file_type().is_symlink()
            || !archive_metadata.is_file()
            || archive_metadata.len() == 0
            || archive_metadata.len() > MAX_WARP_AGENT_ARCHIVE_BYTES
        {
            return Err(
                "Warp Agent CLI archive is missing, oversized, or not a direct file".to_string(),
            );
        }
        let listing = run_bound_macos_tool(
            Path::new("/usr/bin/bsdtar"),
            vec!["-tvzf".to_string(), archive.display().to_string()],
            environment,
            cancellation,
        )?;
        require_successful_tool("inspect Warp Agent CLI archive", &listing)?;
        validate_warp_archive_listing(&listing.stdout.bytes)?;
        let extracted = staging.join("extracted");
        fs::create_dir(&extracted)
            .map_err(|error| format!("create Warp Agent CLI extraction directory: {error}"))?;
        let extraction = run_bound_macos_tool(
            Path::new("/usr/bin/bsdtar"),
            vec![
                "-xzf".to_string(),
                archive.display().to_string(),
                "-C".to_string(),
                extracted.display().to_string(),
            ],
            environment,
            cancellation,
        )?;
        require_successful_tool("extract Warp Agent CLI archive", &extraction)?;
        let extracted_binary = extracted.join("oz-stable");
        let extracted_metadata = fs::symlink_metadata(&extracted_binary)
            .map_err(|error| format!("inspect extracted Warp Agent CLI binary: {error}"))?;
        if extracted_metadata.file_type().is_symlink() || !extracted_metadata.is_file() {
            return Err("extracted Warp Agent CLI binary is not a direct file".to_string());
        }
        if read_warp_version(&extracted_binary).as_deref() != Some(target_version) {
            return Err(
                "extracted Warp Agent CLI version metadata disagrees with the target".to_string(),
            );
        }
        let signature = run_bound_macos_tool(
            Path::new("/usr/bin/codesign"),
            vec![
                "--verify".to_string(),
                "--strict".to_string(),
                "--verbose=2".to_string(),
                extracted_binary.display().to_string(),
            ],
            environment,
            cancellation,
        )?;
        require_successful_tool("verify extracted Warp Agent CLI signature", &signature)?;
        fs::create_dir(&target_root)
            .map_err(|error| format!("create Warp Agent CLI version directory: {error}"))?;
        let copied = run_bound_macos_tool(
            Path::new("/usr/bin/ditto"),
            vec![
                extracted.display().to_string(),
                target_root.display().to_string(),
            ],
            environment,
            cancellation,
        )?;
        require_successful_tool("install Warp Agent CLI version", &copied)?;
        let extracted_installed_binary = target_root.join("oz-stable");
        let installed_binary = target_root.join(binary_name);
        fs::rename(&extracted_installed_binary, &installed_binary).map_err(|error| {
            format!("rename installed Warp Agent CLI binary to the stable selector name: {error}")
        })?;
        let installed_metadata = fs::symlink_metadata(&installed_binary)
            .map_err(|error| format!("inspect installed Warp Agent CLI binary: {error}"))?;
        if installed_metadata.file_type().is_symlink()
            || !installed_metadata.is_file()
            || read_warp_version(&installed_binary).as_deref() != Some(target_version)
        {
            return Err(
                "installed Warp Agent CLI binary failed exact identity verification".to_string(),
            );
        }
        let replacement = tui_root.join(format!(
            "current.runtime-zero-{}",
            short_digest(transaction_id.as_bytes())
        ));
        if replacement.exists() {
            return Err("refusing to reuse Warp Agent CLI selector replacement".to_string());
        }
        use std::os::unix::fs::symlink;
        symlink(Path::new("versions").join(target_version), &replacement)
            .map_err(|error| format!("stage Warp Agent CLI current selector: {error}"))?;
        if let Err(error) = fs::rename(&replacement, &current_link) {
            let _ = fs::remove_file(&replacement);
            return Err(format!("publish Warp Agent CLI current selector: {error}"));
        }
        let directory = fs::File::open(tui_root)
            .map_err(|error| format!("open Warp Agent CLI selector directory: {error}"))?;
        directory
            .sync_all()
            .map_err(|error| format!("sync Warp Agent CLI selector directory: {error}"))?;
        Ok(installed_binary)
    })();
    match result {
        Ok(binary) => Ok((staging, binary)),
        Err(error) => {
            let _ = fs::remove_dir_all(&staging);
            Err(error)
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn prepare_warp_agent_cli_update(
    _action: &PlanAction,
    _transaction_id: &str,
    _environment: &[(String, String)],
    _cancellation: &CancellationToken,
) -> Result<(PathBuf, PathBuf), String> {
    Err("Warp Agent CLI updates are currently implemented only on macOS".to_string())
}

#[cfg(target_os = "macos")]
fn read_warp_version(path: &Path) -> Option<String> {
    let root = path.parent()?;
    let bytes = fs::read(root.join("resources/bundled/metadata/version.json")).ok()?;
    if bytes.len() as u64 > rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES {
        return None;
    }
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let version = value.get("warp_version")?.as_str()?.trim();
    if version.is_empty()
        || version.len() > 120
        || version.chars().any(char::is_control)
        || !version
            .chars()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, '.' | '_' | '-'))
    {
        None
    } else {
        Some(version.to_string())
    }
}

#[cfg(target_os = "macos")]
fn validate_warp_archive_listing(bytes: &[u8]) -> Result<(), String> {
    let text = std::str::from_utf8(bytes)
        .map_err(|error| format!("Warp Agent CLI archive listing is not UTF-8: {error}"))?;
    let mut entries = 0usize;
    let mut has_binary = false;
    let mut has_resources = false;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        entries = entries.saturating_add(1);
        if entries > 2048 || line.len() > 1024 || line.chars().any(char::is_control) {
            return Err("Warp Agent CLI archive listing exceeds its bounded shape".to_string());
        }
        let kind = line.as_bytes().first().copied();
        if !matches!(kind, Some(b'-' | b'd')) {
            return Err("Warp Agent CLI archive contains a link or special-file entry".to_string());
        }
        let Some(name) = line.split_whitespace().last() else {
            return Err("Warp Agent CLI archive listing has an empty entry".to_string());
        };
        if name.starts_with('/')
            || name.contains("../")
            || name.contains("\\")
            || name == ".."
            || !(name == "oz-stable" || name == "resources" || name.starts_with("resources/"))
        {
            return Err("Warp Agent CLI archive contains an unexpected path".to_string());
        }
        has_binary |= name == "oz-stable";
        has_resources |= name == "resources" || name.starts_with("resources/");
    }
    if !has_binary || !has_resources {
        return Err("Warp Agent CLI archive lacks its exact binary/resources roots".to_string());
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn prepare_electron_squirrel_update(
    _action: &PlanAction,
    _transaction_id: &str,
    _environment: &[(String, String)],
    _cancellation: &CancellationToken,
) -> Result<(PathBuf, PathBuf), String> {
    Err("Electron/Squirrel application updates are currently implemented only on macOS".to_string())
}

#[cfg(target_os = "macos")]
fn run_bound_macos_tool(
    executable: &Path,
    arguments: Vec<String>,
    environment: &[(String, String)],
    cancellation: &CancellationToken,
) -> Result<rz0_process_host::ProcessOutput, String> {
    let (root, relative) = executable_root_and_name(executable)?;
    let observed = open_observed_executable(root, relative).map_err(|error| {
        format!(
            "observe helper executable {}: {error}",
            executable.display()
        )
    })?;
    if observed.canonical_path != executable {
        return Err(format!(
            "helper executable path is not canonical and direct: {}",
            executable.display()
        ));
    }
    let verified = open_verified_executable(
        root,
        relative,
        &ArtifactExpectation {
            sha256: observed.sha256,
            size_bytes: observed.size_bytes,
        },
    )
    .map_err(|error| format!("verify helper executable {}: {error}", executable.display()))?;
    let binding = bind_verified_executable(&verified)
        .map_err(|error| format!("bind helper executable {}: {error}", executable.display()))?;
    let request = rz0_process_host::ProcessRequest {
        executable: executable.to_path_buf(),
        arguments,
        working_directory: PathBuf::from("/"),
        environment: environment.to_vec(),
        timeout: Duration::from_secs(MAX_EXECUTION_SECONDS),
        output_limit: rz0_resource_contract::MAX_FINDING_REPORT_BYTES,
    };
    let result = rz0_process_host::run_bound_mutating_process(&request, &binding, cancellation)
        .map_err(|error| format!("run helper executable {}: {error}", executable.display()));
    drop(binding);
    result
}

#[cfg(not(target_os = "macos"))]
fn run_bound_macos_tool(
    _executable: &Path,
    _arguments: Vec<String>,
    _environment: &[(String, String)],
    _cancellation: &CancellationToken,
) -> Result<rz0_process_host::ProcessOutput, String> {
    Err("this native helper lane is currently implemented only on macOS".to_string())
}

#[cfg(target_os = "macos")]
fn require_successful_tool(
    operation: &str,
    output: &rz0_process_host::ProcessOutput,
) -> Result<(), String> {
    if output.status.success() && output.cancellation_reason.is_none() {
        return Ok(());
    }
    Err(format!(
        "{operation} failed (exit={:?}, cancellation={:?}, stderr={:?})",
        output.status.code(),
        output.cancellation_reason,
        String::from_utf8_lossy(&output.stderr.bytes)
    ))
}

#[cfg(not(target_os = "macos"))]
fn require_successful_tool(
    _operation: &str,
    _output: &rz0_process_host::ProcessOutput,
) -> Result<(), String> {
    Err("this native helper lane is currently implemented only on macOS".to_string())
}

#[cfg(target_os = "macos")]
fn find_electron_bundle(
    root: &Path,
    bundle_id: &str,
    expected_version: &str,
) -> Result<PathBuf, String> {
    let mut pending = vec![(root.to_path_buf(), 0usize)];
    let mut inspected = 0usize;
    while let Some((directory, depth)) = pending.pop() {
        let entries = fs::read_dir(&directory)
            .map_err(|error| format!("inspect extracted Electron directory: {error}"))?;
        for entry in entries.take(256) {
            inspected = inspected.saturating_add(1);
            if inspected > 512 {
                return Err(
                    "extracted Electron archive exceeds the bundle discovery ceiling".to_string(),
                );
            }
            let entry = entry.map_err(|error| format!("read extracted Electron entry: {error}"))?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)
                .map_err(|error| format!("inspect extracted Electron entry: {error}"))?;
            if metadata.file_type().is_symlink() {
                continue;
            }
            if metadata.is_dir() && path.extension().and_then(|value| value.to_str()) == Some("app")
            {
                if electron_bundle_matches(&path, bundle_id, expected_version)? {
                    return fs::canonicalize(&path).map_err(|error| {
                        format!("canonicalize extracted Electron bundle: {error}")
                    });
                }
                continue;
            }
            if metadata.is_dir() && depth < 3 {
                pending.push((path, depth + 1));
            }
        }
    }
    Err(format!(
        "extracted Electron archive contains no signed bundle for {bundle_id}@{expected_version}"
    ))
}

#[cfg(target_os = "macos")]
fn electron_bundle_matches(
    path: &Path,
    bundle_id: &str,
    expected_version: &str,
) -> Result<bool, String> {
    let info = path.join("Contents/Info.plist");
    let metadata = fs::symlink_metadata(&info)
        .map_err(|error| format!("inspect extracted Electron Info.plist: {error}"))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES
    {
        return Ok(false);
    }
    let bytes =
        fs::read(&info).map_err(|error| format!("read extracted Electron Info.plist: {error}"))?;
    let value = plist::Value::from_reader(Cursor::new(bytes))
        .map_err(|error| format!("parse extracted Electron Info.plist: {error}"))?;
    let Some(dictionary) = value.as_dictionary() else {
        return Ok(false);
    };
    let actual_id = dictionary
        .get("CFBundleIdentifier")
        .and_then(plist::Value::as_string);
    let actual_version = dictionary
        .get("CFBundleShortVersionString")
        .or_else(|| dictionary.get("CFBundleVersion"))
        .and_then(plist::Value::as_string)
        .map(|value| value.trim_start_matches('v'));
    Ok(actual_id == Some(bundle_id) && actual_version == Some(expected_version))
}

#[cfg(target_os = "macos")]
fn file_url(path: &Path) -> Result<String, String> {
    let text = path
        .to_str()
        .ok_or_else(|| "Electron update path is not valid UTF-8".to_string())?;
    let mut url = String::from("file://");
    for byte in text.as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'-' | b'_' | b'~') {
            url.push(*byte as char);
        } else {
            url.push('%');
            url.push(char::from(b"0123456789ABCDEF"[(byte >> 4) as usize]));
            url.push(char::from(b"0123456789ABCDEF"[(byte & 0x0f) as usize]));
        }
    }
    Ok(url)
}

fn validate_execution_inputs(
    operation: TransactionOperation,
    plan: &ActionPlan,
    action: &PlanAction,
    challenge: &ConfirmationChallenge,
    response: &ConfirmationResponse,
    now_unix_seconds: u64,
) -> Result<(), String> {
    let plan_validation = rz0_action_plan::validate_action_plan(plan);
    if !plan_validation.valid {
        return Err(format!(
            "exact update plan is invalid: {:?}",
            plan_validation.errors
        ));
    }
    let expected_kind = match operation {
        TransactionOperation::Update => ActionKind::Update,
        TransactionOperation::Uninstall => ActionKind::Uninstall,
        _ => return Err("unsupported manager transaction operation".to_string()),
    };
    if action.kind != expected_kind || action.disposition != ActionDisposition::Planned {
        return Err(format!(
            "only one planned {} action may execute",
            operation_label(operation)
        ));
    }
    if plan.actions.len() != 1 || plan.actions[0] != *action {
        return Err("execution requires a single-action exact plan".to_string());
    }
    let digests = action_plan_digests(plan).map_err(|errors| errors.join("; "))?;
    if challenge.plan_id != plan.plan_id
        || challenge.plan_sha256 != digests.plan_sha256
        || challenge.write_set_sha256 != digests.write_set_sha256
    {
        return Err("confirmation does not bind the exact update plan".to_string());
    }
    let assessment = validate_confirmation(challenge, response, now_unix_seconds);
    if !assessment.valid {
        return Err(format!(
            "update confirmation is invalid: {:?}",
            assessment.errors
        ));
    }
    if challenge
        .expires_unix_seconds
        .saturating_sub(challenge.issued_unix_seconds)
        > 300
    {
        return Err("update confirmation lifetime exceeds the foundation ceiling".to_string());
    }
    Ok(())
}

fn journal(
    transaction_id: &str,
    plan_id: &str,
    operation: TransactionOperation,
    mut events: Vec<TransactionEvent>,
) -> TransactionJournal {
    let mut journal = TransactionJournal {
        schema_version: rz0_transaction_contract::TRANSACTION_SCHEMA_VERSION,
        contract: rz0_transaction_contract::TRANSACTION_CONTRACT.to_string(),
        transaction_id: transaction_id.to_string(),
        plan_id: plan_id.to_string(),
        operation,
        state: TransactionState::Prepared,
        durability: DurabilityRequirements::schema_one(),
        events: Vec::new(),
    };
    journal.events.append(&mut events);
    seal_transaction_journal(&mut journal);
    journal
}

fn append(previous: &TransactionJournal, event: TransactionEvent) -> TransactionJournal {
    let mut next = previous.clone();
    next.events.push(event);
    next.state = match next.events.last().map(|event| event.kind) {
        Some(TransactionEventKind::Prepared) => TransactionState::Prepared,
        Some(TransactionEventKind::ApplyStarted) => TransactionState::Applying,
        Some(TransactionEventKind::CommitStarted) => TransactionState::CommitPending,
        Some(TransactionEventKind::Committed) => TransactionState::Committed,
        Some(TransactionEventKind::RecoveryRequired) => TransactionState::RecoveryRequired,
        _ => next.state,
    };
    seal_transaction_journal(&mut next);
    next
}

fn event(kind: TransactionEventKind) -> TransactionEvent {
    TransactionEvent {
        sequence: 0,
        kind,
        action_id: None,
        path: None,
        before_sha256: None,
        after_sha256: None,
        previous_event_sha256: String::new(),
        event_sha256: String::new(),
    }
}

fn manager_write_event(
    kind: TransactionEventKind,
    action: &PlanAction,
    challenge: &ConfirmationChallenge,
    path: &str,
) -> TransactionEvent {
    debug_assert!(matches!(
        kind,
        TransactionEventKind::WriteIntent | TransactionEventKind::WriteVerified
    ));
    TransactionEvent {
        sequence: 0,
        kind,
        action_id: Some(action.action_id.clone()),
        path: Some(path.to_string()),
        before_sha256: challenge.before_state_sha256.clone(),
        after_sha256: Some(challenge.expected_after_state_sha256.clone()),
        previous_event_sha256: String::new(),
        event_sha256: String::new(),
    }
}

pub(crate) fn observe_manager_executable(
    executable: &Path,
) -> Result<ActionExecutableIdentity, String> {
    let (root, relative) = executable_root_and_name(executable)?;
    let observed = open_observed_executable(root, relative)
        .map_err(|error| format!("observe manager executable identity: {error}"))?;
    if observed.canonical_path != executable {
        return Err("manager executable path must already be canonical and direct".to_string());
    }
    Ok(ActionExecutableIdentity {
        sha256: observed.sha256,
        size_bytes: observed.size_bytes,
    })
}

fn open_manager_executable(
    executable: &Path,
    identity: &ActionExecutableIdentity,
) -> Result<VerifiedArtifact, String> {
    let (root, relative) = executable_root_and_name(executable)?;
    let verified = open_verified_executable(
        root,
        relative,
        &ArtifactExpectation {
            sha256: identity.sha256.clone(),
            size_bytes: identity.size_bytes,
        },
    )
    .map_err(|error| format!("verify sealed manager executable identity: {error}"))?;
    if verified.canonical_path != executable {
        return Err("sealed manager executable path is not canonical and direct".to_string());
    }
    Ok(verified)
}

fn executable_root_and_name(executable: &Path) -> Result<(&Path, &str), String> {
    if !executable.is_absolute() {
        return Err("manager executable path must be absolute".to_string());
    }
    let root = executable
        .parent()
        .ok_or_else(|| "manager executable has no parent directory".to_string())?;
    let relative = executable
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "manager executable name must be portable UTF-8".to_string())?;
    if relative.is_empty() {
        return Err("manager executable name is empty".to_string());
    }
    Ok((root, relative))
}

#[cfg(target_os = "linux")]
fn validate_platform_manager_execution(artifact: &VerifiedArtifact) -> Result<(), String> {
    use std::os::unix::fs::FileExt as _;

    let mut magic = [0u8; 4];
    let count = artifact
        .file()
        .read_at(&mut magic, 0)
        .map_err(|error| format!("read manager executable format: {error}"))?;
    if count != magic.len() || !is_native_elf_magic(&magic) {
        return Err(
            "Linux identity-bound manager execution currently supports direct native ELF executables only; scripts and interpreter chains remain blocked"
                .to_string(),
        );
    }
    Ok(())
}

#[cfg(any(target_os = "linux", all(test, target_os = "macos")))]
fn is_native_elf_magic(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x7f, b'E', b'L', b'F'])
}

#[cfg(windows)]
fn validate_platform_manager_execution(_artifact: &VerifiedArtifact) -> Result<(), String> {
    // Windows now launches through the process-host CreateProcessW path, which
    // attaches a kill-on-close Job Object and explicit inherited-handle list
    // before the child begins. The artifact binding retains the deny-write/
    // delete lease through that spawn. Runtime proof, reparse/ACL guarantees,
    // and broader capability isolation remain release gates, but this path no
    // longer needs the old unconditional pre-transaction block.
    Ok(())
}

#[cfg(not(any(target_os = "linux", windows)))]
fn validate_platform_manager_execution(_artifact: &VerifiedArtifact) -> Result<(), String> {
    Ok(())
}

fn refuse_if_cancelled(cancellation: &CancellationToken, boundary: &str) -> Result<(), String> {
    match cancellation.reason() {
        Some(reason) => Err(format!("update cancelled {boundary}: {reason:?}")),
        None => Ok(()),
    }
}

fn digest_text(value: &str) -> String {
    sha256(value.as_bytes())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn short_digest(bytes: &[u8]) -> String {
    sha256(bytes)[..16].to_string()
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(unix)]
fn is_effective_root() -> bool {
    // SAFETY: geteuid has no preconditions and only returns the current uid.
    unsafe { libc::geteuid() == 0 }
}

#[cfg(not(unix))]
fn is_effective_root() -> bool {
    false
}

fn manager_supports_self_replacement(manager: &str) -> bool {
    matches!(manager, "omp" | "grok" | "hermes" | "deno")
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use rz0_action_plan::{
        ActionDisposition, ActionKind, ActionPlan, ActionRisk, PlanAction, RollbackPlan,
    };

    #[test]
    fn native_elf_detection_rejects_script_interpreter_chains() {
        assert!(is_native_elf_magic(b"\x7fELFrest"));
        assert!(!is_native_elf_magic(b"#!/bin/sh"));
        assert!(!is_native_elf_magic(b"MZ"));
    }

    #[test]
    fn squirrel_update_file_urls_encode_bundle_paths() {
        assert_eq!(
            file_url(Path::new("/Applications/T3 Code (Nightly).app")).expect("file URL"),
            "file:///Applications/T3%20Code%20%28Nightly%29.app"
        );
    }

    #[test]
    fn warp_archive_listing_rejects_links_and_traversal() {
        assert!(
            validate_warp_archive_listing(
                b"-rwxr-xr-x 0 staff 10 oz-stable\ndrwxr-xr-x 0 staff 0 resources/\n"
            )
            .is_ok()
        );
        assert!(
            validate_warp_archive_listing(b"lrwxr-xr-x 0 staff 10 oz-stable -> /tmp/evil\n")
                .is_err()
        );
        assert!(validate_warp_archive_listing(b"-rwxr-xr-x 0 staff 10 ../oz-stable\n").is_err());
    }

    fn harmless_manager_test_plan(executable: &Path) -> (ActionPlan, PlanAction) {
        let action = PlanAction {
            action_id: "update.execution-test".to_string(),
            finding_id: "update.execution-test".to_string(),
            kind: ActionKind::Update,
            disposition: ActionDisposition::Planned,
            target: "package:homebrew-formula:brew@0.1.0".to_string(),
            source: None,
            manager: Some("homebrew".to_string()),
            executable: Some(executable.display().to_string()),
            executable_identity: Some(
                observe_manager_executable(executable).expect("manager identity"),
            ),
            arguments: vec!["--version".to_string()],
            would_write: false,
            requires_confirmation: true,
            requires_elevation: false,
            network_required: false,
            risk: ActionRisk::Medium,
            capabilities: vec![rz0_capability_contract::Capability::ManagerExecution],
            forbidden_path_classes: Vec::new(),
            write_set: Vec::new(),
            rollback: RollbackPlan {
                supported: false,
                quarantine_required: false,
                description: "test command acknowledges manual recovery posture".to_string(),
            },
        };
        let plan = ActionPlan {
            schema_version: rz0_action_plan::ACTION_PLAN_SCHEMA_VERSION,
            plan_id: "update.plan.execution-test".to_string(),
            module_id: "first-party.updater".to_string(),
            created_at: None,
            expires_at: None,
            dry_run: true,
            writes_attempted: false,
            evidence_contract: rz0_finding_contract::FINDING_CONTRACT.to_string(),
            evidence_report_id: "findings:execution-test".to_string(),
            evidence_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
            actions: vec![action.clone()],
            warnings: Vec::new(),
        };
        assert!(rz0_action_plan::validate_action_plan(&plan).valid);
        (plan, action)
    }

    fn harmless_manager_uninstall_test_plan(executable: &Path) -> (ActionPlan, PlanAction) {
        let (mut plan, mut action) = harmless_manager_test_plan(executable);
        action.action_id = "uninstall.execution-test".to_string();
        action.finding_id = "uninstall.execution-test".to_string();
        action.kind = ActionKind::Uninstall;
        plan.plan_id = "uninstall.plan.execution-test".to_string();
        plan.module_id = "first-party.uninstall".to_string();
        plan.evidence_report_id = "findings:uninstall-execution-test".to_string();
        plan.actions = vec![action.clone()];
        assert!(rz0_action_plan::validate_action_plan(&plan).valid);
        (plan, action)
    }

    fn private_execution_root(label: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "runtime-zero-update-{label}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("transactions")).expect("transactions");
        std::fs::create_dir_all(root.join("receipts")).expect("receipts");
        use std::os::unix::fs::PermissionsExt;
        for directory in [&root, &root.join("transactions"), &root.join("receipts")] {
            std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700))
                .expect("private test directory");
        }
        root
    }

    #[test]
    fn macos_executes_a_path_revalidated_manager_before_post_action_verification() {
        let executable = Path::new("/opt/homebrew/bin/brew");
        if !executable.is_file() {
            return;
        }
        let (plan, action) = harmless_manager_test_plan(executable);
        let now = unix_seconds();
        let (challenge, _) = build_update_challenge(&plan, &action, true, now).expect("challenge");
        let response = validate_update_confirmation(&challenge, &challenge.expected_phrase, now)
            .expect("confirmation");
        let root = private_execution_root("verification-failure");
        let (_, cancellation) = rz0_cancellation_contract::cancellation_pair();
        let error = execute_update_action(UpdateExecutionRequest {
            state_root: &root,
            plan: &plan,
            action: &action,
            challenge: &challenge,
            response: &response,
            now_unix_seconds: now,
            environment: vec![
                (
                    "PATH".to_string(),
                    "/opt/homebrew/bin:/usr/bin:/bin".to_string(),
                ),
                ("HOME".to_string(), "/Users/tjn".to_string()),
            ],
            cancellation: &cancellation,
            verify_after: |_| Err("test stops before committing the external effect".to_string()),
        })
        .expect_err("test verification should force recovery");
        assert!(
            !error.contains("bind verified manager executable"),
            "{error}"
        );
        assert!(error.contains("post-update verification failed"), "{error}");
        assert!(
            std::fs::read_dir(root.join("transactions"))
                .expect("transactions")
                .next()
                .is_some()
        );
        assert!(
            std::fs::read_dir(root.join("receipts"))
                .expect("receipts")
                .next()
                .is_none()
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn macos_commits_a_verified_receipt_for_a_harmless_manager_invocation() {
        let executable = Path::new("/opt/homebrew/bin/brew");
        if !executable.is_file() {
            return;
        }
        let (plan, action) = harmless_manager_test_plan(executable);
        let now = unix_seconds();
        let (challenge, _) = build_update_challenge(&plan, &action, true, now).expect("challenge");
        let response = validate_update_confirmation(&challenge, &challenge.expected_phrase, now)
            .expect("confirmation");
        let root = private_execution_root("verified");
        let (_, cancellation) = rz0_cancellation_contract::cancellation_pair();
        let report = execute_update_action(UpdateExecutionRequest {
            state_root: &root,
            plan: &plan,
            action: &action,
            challenge: &challenge,
            response: &response,
            now_unix_seconds: now,
            environment: vec![
                (
                    "PATH".to_string(),
                    "/opt/homebrew/bin:/usr/bin:/bin".to_string(),
                ),
                ("HOME".to_string(), "/Users/tjn".to_string()),
            ],
            cancellation: &cancellation,
            verify_after: |_| Ok("harmless manager invocation verified".to_string()),
        })
        .expect("committed receipt");
        assert_eq!(report.status, UpdateExecutionStatus::Committed);
        assert!(report.writes_attempted);
        assert!(report.product_execution_authorized);
        let receipt_path = root.join(&report.receipt_reference);
        let receipt: ExternalEffectReceipt =
            serde_json::from_slice(&std::fs::read(&receipt_path).expect("receipt bytes"))
                .expect("receipt JSON");
        assert_eq!(receipt.transaction_id, report.transaction_id);
        assert_eq!(receipt.status, ExternalEffectStatus::Verified);
        assert!(receipt.writes_attempted);
        assert!(!receipt.automatic_mutation_authorized);
        assert_eq!(receipt.exit_code, 0);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn manager_executor_binds_uninstall_operation_into_receipt_and_journal() {
        let executable = Path::new("/opt/homebrew/bin/brew");
        if !executable.is_file() {
            return;
        }
        let (plan, action) = harmless_manager_uninstall_test_plan(executable);
        let now = unix_seconds();
        let (challenge, view) =
            build_update_challenge(&plan, &action, true, now).expect("challenge");
        assert_eq!(challenge.risk, ConfirmationRisk::Destructive);
        assert_eq!(view.operation, TransactionOperation::Uninstall);
        assert_eq!(view.manager.as_deref(), Some("homebrew"));
        assert_eq!(view.target, action.target);
        assert_eq!(view.arguments, action.arguments);
        assert_eq!(view.risk, ConfirmationRisk::Destructive);
        assert_eq!(
            view.executable_size_bytes,
            Some(
                action
                    .executable_identity
                    .as_ref()
                    .expect("identity")
                    .size_bytes
            )
        );
        let response = validate_update_confirmation(&challenge, &challenge.expected_phrase, now)
            .expect("confirmation");
        let root = private_execution_root("uninstall-verified");
        let (_, cancellation) = rz0_cancellation_contract::cancellation_pair();
        let report = execute_uninstall_action(UpdateExecutionRequest {
            state_root: &root,
            plan: &plan,
            action: &action,
            challenge: &challenge,
            response: &response,
            now_unix_seconds: now,
            environment: vec![
                (
                    "PATH".to_string(),
                    "/opt/homebrew/bin:/usr/bin:/bin".to_string(),
                ),
                ("HOME".to_string(), "/Users/tjn".to_string()),
            ],
            cancellation: &cancellation,
            verify_after: |_| Ok("harmless manager uninstall invocation verified".to_string()),
        })
        .expect("committed uninstall receipt");
        assert_eq!(report.operation, TransactionOperation::Uninstall);
        let receipt_path = root.join(&report.receipt_reference);
        let receipt: ExternalEffectReceipt =
            serde_json::from_slice(&std::fs::read(&receipt_path).expect("receipt bytes"))
                .expect("receipt JSON");
        assert_eq!(receipt.operation, TransactionOperation::Uninstall);
        assert_eq!(receipt.transaction_id, report.transaction_id);
        let _ = std::fs::remove_dir_all(root);
    }
}
