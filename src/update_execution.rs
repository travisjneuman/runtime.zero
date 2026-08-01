use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rz0_action_plan::{ActionDisposition, ActionKind, ActionPlan, PlanAction, action_plan_digests};
use rz0_confirmation_contract::{
    CONFIRMATION_CHALLENGE_CONTRACT, CONFIRMATION_CONSUMPTION_CONTRACT,
    CONFIRMATION_RESPONSE_CONTRACT, ConfirmationChallenge, ConfirmationConsumption,
    ConfirmationResponse, ConfirmationRisk, ConfirmationSurface, seal_confirmation_challenge,
    seal_confirmation_consumption, validate_confirmation,
};
use rz0_module_updater::manager_executable_allowed;
use rz0_secure_fs::SecureDirectory;
use rz0_transaction_contract::{
    DurabilityRequirements, TransactionEvent, TransactionEventKind, TransactionJournal,
    TransactionOperation, TransactionState, publish_confirmation_consumption,
    publish_journal_snapshot, seal_transaction_journal,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const UPDATES_DIRECTORY: &str = "updates";
const UPDATE_RECEIPT_SCHEMA_VERSION: u16 = 1;
const UPDATE_RECEIPT_CONTRACT: &str = "manager_update_receipt";
const MAX_EXECUTION_SECONDS: u64 = 30 * 60;
const MAX_RECEIPT_BYTES: u64 = rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES;

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
    pub status: UpdateExecutionStatus,
    pub exit_code: Option<i32>,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub verification: String,
    pub receipt_path: String,
    pub writes_attempted: bool,
    pub product_execution_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UpdateExecutionStatus {
    Committed,
    RecoveryRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct UpdateExecutionReceipt {
    schema_version: u16,
    contract: String,
    transaction_id: String,
    plan_id: String,
    action_id: String,
    manager: String,
    executable: String,
    arguments: Vec<String>,
    target: String,
    available_version: Option<String>,
    started_unix_seconds: u64,
    completed_unix_seconds: u64,
    exit_code: Option<i32>,
    stdout_bytes: u64,
    stderr_bytes: u64,
    stdout_sha256: String,
    stderr_sha256: String,
    verification: String,
    rollback_supported: bool,
    manual_recovery_acknowledged: bool,
    writes_attempted: bool,
    product_execution_authorized: bool,
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

    let process = rz0_process_host::run_mutating_process(&rz0_process_host::ProcessRequest {
        executable: PathBuf::from(executable),
        arguments: action.arguments.clone(),
        working_directory: PathBuf::from("/"),
        environment,
        timeout: Duration::from_secs(MAX_EXECUTION_SECONDS),
        output_limit: rz0_resource_contract::MAX_FINDING_REPORT_BYTES,
    })
    .map_err(|error| {
        let recovery = append(&applying, event(TransactionEventKind::RecoveryRequired));
        let _ = publish_journal_snapshot(&transactions_root, &recovery);
        format!("manager update process failed; recovery is required: {error}")
    })?;
    let stdout_sha256 = sha256(&process.stdout.bytes);
    let stderr_sha256 = sha256(&process.stderr.bytes);
    let exit_code = process.status.code();
    if !process.status.success() || process.timed_out {
        let recovery = append(&applying, event(TransactionEventKind::RecoveryRequired));
        let _ = publish_journal_snapshot(&transactions_root, &recovery);
        return Err(format!(
            "manager update did not complete successfully (exit={exit_code:?}, timed_out={}); recovery is required",
            process.timed_out
        ));
    }

    let verification = match verify_after() {
        Ok(verification) => verification,
        Err(error) => {
            let recovery = append(&applying, event(TransactionEventKind::RecoveryRequired));
            let _ = publish_journal_snapshot(&transactions_root, &recovery);
            return Err(format!(
                "post-update verification failed; recovery is required: {error}"
            ));
        }
    };

    let committing = append(&applying, event(TransactionEventKind::CommitStarted));
    publish_journal_snapshot(&transactions_root, &committing)
        .map_err(|error| format!("publish update commit journal: {error}"))?;
    let committed = append(&committing, event(TransactionEventKind::Committed));
    publish_journal_snapshot(&transactions_root, &committed)
        .map_err(|error| format!("publish committed update journal: {error}"))?;

    let receipt = UpdateExecutionReceipt {
        schema_version: UPDATE_RECEIPT_SCHEMA_VERSION,
        contract: UPDATE_RECEIPT_CONTRACT.to_string(),
        transaction_id: transaction_id.clone(),
        plan_id: plan.plan_id.clone(),
        action_id: action.action_id.clone(),
        manager: manager.to_string(),
        executable: executable.to_string(),
        arguments: action.arguments.clone(),
        target: action.target.clone(),
        available_version: action
            .target
            .rsplit_once('@')
            .map(|(_, version)| version.to_string()),
        started_unix_seconds: now_unix_seconds,
        completed_unix_seconds: unix_seconds(),
        exit_code,
        stdout_bytes: process.stdout.total_bytes,
        stderr_bytes: process.stderr.total_bytes,
        stdout_sha256,
        stderr_sha256,
        verification: verification.clone(),
        rollback_supported: action.rollback.supported,
        manual_recovery_acknowledged: challenge.manual_recovery_acknowledged,
        writes_attempted: true,
        product_execution_authorized: true,
    };
    let receipt_bytes = serde_json::to_vec(&receipt)
        .map_err(|error| format!("serialize update receipt: {error}"))?;
    if receipt_bytes.len() as u64 > MAX_RECEIPT_BYTES {
        return Err("update receipt exceeds the foundation ceiling after commit".to_string());
    }
    let state = SecureDirectory::open(state_root)
        .map_err(|error| format!("open update state root: {error}"))?;
    state
        .verify_private()
        .map_err(|error| format!("verify update state root: {error}"))?;
    state
        .open_child_directory(OsStr::new("receipts"))
        .map_err(|error| format!("open update receipts: {error}"))?;
    let updates = match state.open_child_directory(OsStr::new(UPDATES_DIRECTORY)) {
        Ok(directory) => directory,
        Err(_) => state
            .create_child_directory(OsStr::new(UPDATES_DIRECTORY))
            .map_err(|error| format!("create update receipt directory: {error}"))?,
    };
    let receipt_name = format!("{transaction_id}.json");
    updates
        .write_new_child(OsStr::new(&receipt_name), &receipt_bytes, MAX_RECEIPT_BYTES)
        .map_err(|error| format!("publish update receipt: {error}"))?;
    Ok(UpdateExecutionReport {
        transaction_id,
        action_id: action.action_id.clone(),
        manager: manager.to_string(),
        target: action.target.clone(),
        status: UpdateExecutionStatus::Committed,
        exit_code,
        stdout_bytes: process.stdout.total_bytes,
        stderr_bytes: process.stderr.total_bytes,
        stdout_sha256: receipt.stdout_sha256.clone(),
        stderr_sha256: receipt.stderr_sha256.clone(),
        verification,
        receipt_path: state_root
            .join(UPDATES_DIRECTORY)
            .join(receipt_name)
            .display()
            .to_string(),
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
    fn explicit_manager_execution_commits_a_verified_receipt_in_a_private_test_store() {
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
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))
                .expect("root private");
            std::fs::set_permissions(
                root.join("transactions"),
                std::fs::Permissions::from_mode(0o700),
            )
            .expect("transactions private");
            std::fs::set_permissions(
                root.join("receipts"),
                std::fs::Permissions::from_mode(0o700),
            )
            .expect("receipts private");
        }
        let result = execute_update_action(UpdateExecutionRequest {
            state_root: &root,
            plan: &plan,
            action: &action,
            challenge: &challenge,
            response: &response,
            now_unix_seconds: now,
            environment: vec![
                (
                    "HOME".to_string(),
                    std::env::var("HOME").expect("test HOME"),
                ),
                (
                    "PATH".to_string(),
                    "/usr/bin:/bin:/opt/homebrew/bin".to_string(),
                ),
            ],
            verify_after: || Ok("test post-action verification".to_string()),
        })
        .expect("explicit manager execution");
        assert_eq!(result.status, UpdateExecutionStatus::Committed);
        assert!(result.writes_attempted);
        assert!(result.product_execution_authorized);
        assert!(Path::new(&result.receipt_path).is_file());
        let replay = execute_update_action(UpdateExecutionRequest {
            state_root: &root,
            plan: &plan,
            action: &action,
            challenge: &challenge,
            response: &response,
            now_unix_seconds: now,
            environment: Vec::new(),
            verify_after: || Ok("replay must not verify".to_string()),
        });
        assert!(
            replay
                .expect_err("single-use update confirmation")
                .contains("already been consumed")
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
