use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rz0_action_plan::{
    ActionDisposition, ActionExecutableIdentity, ActionKind, ActionPlan, PlanAction,
    action_plan_digests,
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
    pub plan_id: String,
    pub action_id: String,
    pub plan_sha256: String,
    pub expected_phrase: String,
    pub issued_unix_seconds: u64,
    pub expires_unix_seconds: u64,
    pub rollback_available: bool,
    pub manual_recovery_acknowledged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateExecutionReport {
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
        risk: ConfirmationRisk::Mutating,
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
        plan_id: challenge.plan_id.clone(),
        action_id: action.action_id.clone(),
        plan_sha256: challenge.plan_sha256.clone(),
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
    let response = ConfirmationResponse {
        schema_version: rz0_confirmation_contract::CONFIRMATION_SCHEMA_VERSION,
        contract: CONFIRMATION_RESPONSE_CONTRACT.to_string(),
        challenge_id: challenge.challenge_id.clone(),
        challenge_sha256: challenge.challenge_sha256.clone(),
        confirmed_unix_seconds: now_unix_seconds,
        surface: ConfirmationSurface::Cli,
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
    F: FnOnce() -> Result<String, String>,
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
    F: FnOnce() -> Result<String, String>,
{
    let UpdateExecutionRequest {
        state_root,
        plan,
        action,
        challenge,
        response,
        now_unix_seconds,
        environment,
        cancellation,
        verify_after,
    } = request;
    validate_execution_inputs(plan, action, challenge, response, now_unix_seconds)?;
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
    if action.requires_elevation && !is_effective_root() {
        return Err(
            "this manager requires elevation; runtime.zero will not invoke sudo or an interactive privilege helper"
                .to_string(),
        );
    }
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
    let executable_binding_name = executable_binding.mechanism().as_str().to_string();
    refuse_if_cancelled(cancellation, "before transaction preparation")?;

    let transaction_id = format!(
        "tx.update.{}.{}",
        short_digest(plan.plan_id.as_bytes()),
        now_unix_seconds
    );
    ensure_new_transaction(state_root, &transaction_id)?;
    let prepared = journal(
        &transaction_id,
        &plan.plan_id,
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
    let write_path = format!("manager/{manager}/{}", action.finding_id);
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

    let process = rz0_process_host::run_bound_mutating_process(
        &rz0_process_host::ProcessRequest {
            executable: PathBuf::from(executable),
            arguments: action.arguments.clone(),
            working_directory: PathBuf::from("/"),
            environment,
            timeout: Duration::from_secs(MAX_EXECUTION_SECONDS),
            output_limit: rz0_resource_contract::MAX_FINDING_REPORT_BYTES,
        },
        &executable_binding,
        cancellation,
    )
    .map_err(|error| {
        let recovery = append(&intent, event(TransactionEventKind::RecoveryRequired));
        let _ = publish_journal_snapshot(&transactions_root, &recovery);
        format!("manager update process failed; recovery is required: {error}")
    })?;
    drop(executable_binding);
    if let Err(error) = revalidate_verified_executable(&mut verified_executable) {
        let recovery = append(&intent, event(TransactionEventKind::RecoveryRequired));
        let _ = publish_journal_snapshot(&transactions_root, &recovery);
        return Err(format!(
            "manager executable identity changed across spawn; recovery is required: {error}"
        ));
    }
    let stdout_sha256 = sha256(&process.stdout.bytes);
    let stderr_sha256 = sha256(&process.stderr.bytes);
    let exit_code = process.status.code();
    if !process.status.success() || process.cancellation_reason.is_some() {
        let recovery = append(&intent, event(TransactionEventKind::RecoveryRequired));
        let _ = publish_journal_snapshot(&transactions_root, &recovery);
        return Err(format!(
            "manager update did not complete successfully (exit={exit_code:?}, cancellation={:?}); recovery is required",
            process.cancellation_reason
        ));
    }

    refuse_if_cancelled(cancellation, "before post-update verification").map_err(|error| {
        let recovery = append(&intent, event(TransactionEventKind::RecoveryRequired));
        let _ = publish_journal_snapshot(&transactions_root, &recovery);
        format!("{error}; recovery is required")
    })?;
    let verification = match verify_after() {
        Ok(verification) => verification,
        Err(error) => {
            let recovery = append(&intent, event(TransactionEventKind::RecoveryRequired));
            let _ = publish_journal_snapshot(&transactions_root, &recovery);
            return Err(format!(
                "post-update verification failed; recovery is required: {error}"
            ));
        }
    };
    refuse_if_cancelled(cancellation, "during post-update verification").map_err(|error| {
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
            "publish update commit-pending journal failed; recovery is required: {error}"
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
        operation: TransactionOperation::Update,
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
            "external-effect receipt is durable but final committed journal publication failed; recovery is required: {error}"
        )
    })?;

    Ok(UpdateExecutionReport {
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

fn validate_execution_inputs(
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
    if action.kind != ActionKind::Update || action.disposition != ActionDisposition::Planned {
        return Err("only one planned update action may execute".to_string());
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
    mut events: Vec<TransactionEvent>,
) -> TransactionJournal {
    let mut journal = TransactionJournal {
        schema_version: rz0_transaction_contract::TRANSACTION_SCHEMA_VERSION,
        contract: rz0_transaction_contract::TRANSACTION_CONTRACT.to_string(),
        transaction_id: transaction_id.to_string(),
        plan_id: plan_id.to_string(),
        operation: TransactionOperation::Update,
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
    Err(
        "Windows manager execution is blocked before transaction preparation until exact process-image binding and race-free Job Object containment are implemented"
            .to_string(),
    )
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
    fn macos_refuses_update_execution_before_consuming_confirmation_without_exact_spawn_binding() {
        let executable = Path::new("/opt/homebrew/bin/brew");
        if !executable.is_file() {
            return;
        }
        let hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
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
            evidence_sha256: hash.to_string(),
            actions: vec![action.clone()],
            warnings: Vec::new(),
        };
        assert!(rz0_action_plan::validate_action_plan(&plan).valid);
        let now = unix_seconds();
        let (challenge, _) = build_update_challenge(&plan, &action, true, now).expect("challenge");
        let response = validate_update_confirmation(&challenge, &challenge.expected_phrase, now)
            .expect("confirmation");
        let root =
            std::env::temp_dir().join(format!("runtime-zero-update-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("transactions")).expect("transactions");
        std::fs::create_dir_all(root.join("receipts")).expect("receipts");
        use std::os::unix::fs::PermissionsExt;
        for directory in [&root, &root.join("transactions"), &root.join("receipts")] {
            std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700))
                .expect("private test directory");
        }
        let (_, cancellation) = rz0_cancellation_contract::cancellation_pair();
        let error = execute_update_action(UpdateExecutionRequest {
            state_root: &root,
            plan: &plan,
            action: &action,
            challenge: &challenge,
            response: &response,
            now_unix_seconds: now,
            environment: Vec::new(),
            cancellation: &cancellation,
            verify_after: || Ok("must not verify".to_string()),
        })
        .expect_err("macOS exact identity-to-spawn remains blocked");
        assert!(error.contains("bind verified manager executable"));
        assert!(
            std::fs::read_dir(root.join("transactions"))
                .expect("transactions")
                .next()
                .is_none()
        );
        assert!(
            std::fs::read_dir(root.join("receipts"))
                .expect("receipts")
                .next()
                .is_none()
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
