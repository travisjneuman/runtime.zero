use std::{collections::BTreeSet, ffi::OsStr, fmt, path::Path};

use rz0_action_plan::{ActionKind, ActionPlan, action_plan_digests};
use rz0_cancellation_contract::CancellationToken;
use rz0_confirmation_contract::{
    ConfirmationChallenge, ConfirmationConsumption, ConfirmationResponse, ConfirmationRisk,
    confirmation_response_sha256, validate_confirmation_consumption,
};
use rz0_secure_fs::{SecureDirectory, SecureFileLock, SecureFsError, SecureFsErrorCode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    TransactionJournal, TransactionOperation, TransactionState, recover_journal_head,
    validate_transaction_journal,
};

pub const EXTERNAL_EFFECT_RECEIPT_SCHEMA_VERSION: u16 = 1;
pub const EXTERNAL_EFFECT_RECEIPT_CONTRACT: &str = "external_effect_commit_receipt";
pub const EXTERNAL_EFFECT_RECOVERY_CONTRACT: &str = "external_effect_recovery_assessment";
const TRANSACTIONS_DIRECTORY: &str = "transactions";
const RECEIPTS_DIRECTORY: &str = "receipts";
const CONFIRMATION_NAME: &str = "confirmation.json";
const PUBLICATION_LOCK_NAME: &str = ".external-effect.lock";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalEffectStatus {
    Verified,
}

/// Tamper-evident outcome evidence for a manager-owned effect that cannot be
/// made atomic with runtime.zero's local state.
///
/// The receipt binds the exact commit-pending journal, plan, confirmation,
/// executable, bounded process result, and post-action verification. It is
/// published before the final `committed` journal event so an interruption
/// after a successful external command remains deterministically detectable.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExternalEffectReceipt {
    pub schema_version: u16,
    pub contract: String,
    pub transaction_id: String,
    pub plan_id: String,
    pub action_id: String,
    pub operation: TransactionOperation,
    pub manager: String,
    pub target: String,
    pub executable_sha256: String,
    pub executable_size_bytes: u64,
    pub executable_binding: String,
    pub arguments_sha256: String,
    pub started_unix_seconds: u64,
    pub completed_unix_seconds: u64,
    pub exit_code: i32,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub stdout_sha256: String,
    pub stderr_sha256: String,
    pub verification_sha256: String,
    pub commit_pending_sequence: u32,
    pub commit_pending_event_sha256: String,
    pub commit_pending_snapshot_name: String,
    pub action_plan_sha256: String,
    pub write_set_sha256: String,
    pub confirmation_challenge_sha256: String,
    pub confirmation_response_sha256: String,
    pub confirmation_consumption_sha256: String,
    pub rollback_supported: bool,
    pub status: ExternalEffectStatus,
    pub writes_attempted: bool,
    pub automatic_mutation_authorized: bool,
    pub binding_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalEffectValidation {
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct ExternalEffectPublicationInput<'a> {
    pub commit_pending_journal: &'a TransactionJournal,
    pub action_plan: &'a ActionPlan,
    pub challenge: &'a ConfirmationChallenge,
    pub response: &'a ConfirmationResponse,
    pub consumption: &'a ConfirmationConsumption,
    pub receipt: &'a ExternalEffectReceipt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalEffectPublicationStatus {
    Published,
    AlreadyPublished,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalEffectPublication {
    pub status: ExternalEffectPublicationStatus,
    pub transaction_id: String,
    pub receipt_name: String,
    pub receipt_bytes: u64,
    pub automatic_mutation_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExternalEffectRecoveryDecision {
    AbortWithoutWrites,
    VerifyExternalEffect,
    CompleteJournalCommitWithExplicitApproval,
    NoAction,
    RefuseInconsistentEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExternalEffectRecoveryAssessment {
    pub schema_version: u16,
    pub contract: String,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub transaction_id: String,
    pub journal_state: TransactionState,
    pub receipt_present: bool,
    pub receipt_valid: bool,
    pub decision: ExternalEffectRecoveryDecision,
    pub automatic_mutation_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExternalEffectErrorCode {
    InvalidEvidence,
    EvidenceMissing,
    UnsafeFilesystem,
    Conflict,
    Cancelled,
    RecoveryRequired,
    LimitExceeded,
    Unsupported,
    Io,
}

#[derive(Debug)]
pub struct ExternalEffectError {
    pub code: ExternalEffectErrorCode,
    detail: String,
}

impl ExternalEffectError {
    pub const fn foundation_code(&self) -> rz0_error_contract::FoundationErrorCode {
        match self.code {
            ExternalEffectErrorCode::InvalidEvidence => {
                rz0_error_contract::FoundationErrorCode::TransactionInvalid
            }
            ExternalEffectErrorCode::EvidenceMissing => {
                rz0_error_contract::FoundationErrorCode::InvalidContract
            }
            ExternalEffectErrorCode::UnsafeFilesystem => {
                rz0_error_contract::FoundationErrorCode::PermissionDenied
            }
            ExternalEffectErrorCode::Conflict => rz0_error_contract::FoundationErrorCode::Conflict,
            ExternalEffectErrorCode::Cancelled => {
                rz0_error_contract::FoundationErrorCode::Cancelled
            }
            ExternalEffectErrorCode::RecoveryRequired => {
                rz0_error_contract::FoundationErrorCode::RecoveryRequired
            }
            ExternalEffectErrorCode::LimitExceeded => {
                rz0_error_contract::FoundationErrorCode::InputLimitExceeded
            }
            ExternalEffectErrorCode::Unsupported => {
                rz0_error_contract::FoundationErrorCode::UnsupportedOperation
            }
            ExternalEffectErrorCode::Io => rz0_error_contract::FoundationErrorCode::IoUnavailable,
        }
    }
}

impl fmt::Display for ExternalEffectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for ExternalEffectError {}

pub fn arguments_sha256(arguments: &[String]) -> String {
    let mut digest = Sha256::new();
    digest.update(b"runtime.zero.external-effect.arguments.v1\0");
    digest.update((arguments.len() as u64).to_be_bytes());
    for argument in arguments {
        put(&mut digest, argument);
    }
    format!("{:x}", digest.finalize())
}

pub fn seal_external_effect_receipt(receipt: &mut ExternalEffectReceipt) {
    receipt.binding_sha256 = external_effect_receipt_sha256(receipt);
}

pub fn validate_external_effect_receipt(
    receipt: &ExternalEffectReceipt,
    journal: &TransactionJournal,
) -> ExternalEffectValidation {
    let mut errors = Vec::new();
    let journal_validation = validate_transaction_journal(journal);
    if !journal_validation.valid || journal.state != TransactionState::CommitPending {
        errors.push("external effect receipt requires a valid commit-pending journal".to_string());
    }
    let head = journal.events.last();
    if receipt.schema_version != EXTERNAL_EFFECT_RECEIPT_SCHEMA_VERSION {
        errors.push(format!(
            "schema_version must be {EXTERNAL_EFFECT_RECEIPT_SCHEMA_VERSION}"
        ));
    }
    if receipt.contract != EXTERNAL_EFFECT_RECEIPT_CONTRACT {
        errors.push(format!(
            "contract must be {EXTERNAL_EFFECT_RECEIPT_CONTRACT}"
        ));
    }
    if receipt.transaction_id != journal.transaction_id
        || receipt.plan_id != journal.plan_id
        || receipt.operation != journal.operation
        || head.is_none_or(|event| {
            receipt.commit_pending_sequence != event.sequence
                || receipt.commit_pending_event_sha256 != event.event_sha256
                || receipt.commit_pending_snapshot_name
                    != format!("{:04}-{}.json", event.sequence, event.event_sha256)
        })
    {
        errors.push("receipt does not bind the exact commit-pending journal head".to_string());
    }
    if !matches!(
        receipt.operation,
        TransactionOperation::Update | TransactionOperation::Uninstall
    ) {
        errors.push("external effect receipt supports only manager update/uninstall".to_string());
    }
    for (name, value, maximum) in [
        ("action_id", receipt.action_id.as_str(), 100),
        ("manager", receipt.manager.as_str(), 80),
        ("target", receipt.target.as_str(), 240),
        (
            "executable_binding",
            receipt.executable_binding.as_str(),
            80,
        ),
    ] {
        if value.trim().is_empty() || value.len() > maximum || value.chars().any(char::is_control) {
            errors.push(format!("receipt {name} is invalid"));
        }
    }
    if !rz0_validation_contract::valid_ledger_id(&receipt.action_id, 100) {
        errors.push("receipt action_id is invalid".to_string());
    }
    for (name, digest) in [
        ("executable_sha256", &receipt.executable_sha256),
        ("arguments_sha256", &receipt.arguments_sha256),
        ("stdout_sha256", &receipt.stdout_sha256),
        ("stderr_sha256", &receipt.stderr_sha256),
        ("verification_sha256", &receipt.verification_sha256),
        (
            "commit_pending_event_sha256",
            &receipt.commit_pending_event_sha256,
        ),
        ("action_plan_sha256", &receipt.action_plan_sha256),
        ("write_set_sha256", &receipt.write_set_sha256),
        (
            "confirmation_challenge_sha256",
            &receipt.confirmation_challenge_sha256,
        ),
        (
            "confirmation_response_sha256",
            &receipt.confirmation_response_sha256,
        ),
        (
            "confirmation_consumption_sha256",
            &receipt.confirmation_consumption_sha256,
        ),
        ("binding_sha256", &receipt.binding_sha256),
    ] {
        if !rz0_validation_contract::valid_sha256(digest) {
            errors.push(format!("receipt {name} is not canonical SHA-256"));
        }
    }
    if receipt.executable_size_bytes == 0
        || receipt.executable_size_bytes > rz0_resource_contract::MAX_EXECUTABLE_BYTES
        || receipt.stdout_bytes > rz0_resource_contract::MAX_FINDING_REPORT_BYTES
        || receipt.stderr_bytes > rz0_resource_contract::MAX_FINDING_REPORT_BYTES
    {
        errors.push("receipt executable or output size exceeds foundation bounds".to_string());
    }
    if receipt.completed_unix_seconds < receipt.started_unix_seconds {
        errors.push("receipt completion precedes start".to_string());
    }
    if !valid_executable_binding(&receipt.executable_binding) {
        errors.push("receipt executable binding mechanism is unsupported".to_string());
    }
    if receipt.exit_code != 0 {
        errors.push("verified external effect requires a zero exit code".to_string());
    }
    if !receipt.writes_attempted || receipt.automatic_mutation_authorized {
        errors.push("receipt write/authority posture is invalid".to_string());
    }
    if receipt.binding_sha256 != external_effect_receipt_sha256(receipt) {
        errors.push("receipt binding digest is invalid".to_string());
    }
    errors.sort();
    errors.dedup();
    ExternalEffectValidation {
        valid: errors.is_empty(),
        errors,
    }
}

fn valid_executable_binding(value: &str) -> bool {
    let (base, suffix) = value.split_once(';').unwrap_or((value, ""));
    let valid_base = matches!(
        base,
        "proc_held_descriptor_path" | "path_identity_revalidated" | "deny_write_delete_handle"
    );
    let valid_suffix = suffix.is_empty()
        || suffix == "wrapper=sudo"
        || suffix == "self-replaced"
        || suffix == "warp-native"
        || suffix == "wrapper=sudo;self-replaced"
        || suffix == "wrapper=sudo;warp-native"
        || suffix == "warp-native;self-replaced"
        || suffix == "wrapper=sudo;warp-native;self-replaced";
    valid_base && valid_suffix
}

pub fn publish_external_effect_receipt(
    state_root: &Path,
    input: ExternalEffectPublicationInput<'_>,
) -> Result<ExternalEffectPublication, ExternalEffectError> {
    publish_external_effect_receipt_inner(state_root, input, None)
}

pub fn publish_external_effect_receipt_cancellable(
    state_root: &Path,
    input: ExternalEffectPublicationInput<'_>,
    cancellation: &CancellationToken,
) -> Result<ExternalEffectPublication, ExternalEffectError> {
    publish_external_effect_receipt_inner(state_root, input, Some(cancellation))
}

fn publish_external_effect_receipt_inner(
    state_root: &Path,
    input: ExternalEffectPublicationInput<'_>,
    cancellation: Option<&CancellationToken>,
) -> Result<ExternalEffectPublication, ExternalEffectError> {
    validate_publication_evidence(input)?;
    check_cancellation(cancellation, false, "before external effect publication")?;
    let bytes = canonical_line(input.receipt)?;
    if bytes.len() as u64 > rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES {
        return Err(error(
            ExternalEffectErrorCode::LimitExceeded,
            "external effect receipt exceeds the document ceiling",
        ));
    }

    let state = SecureDirectory::open(state_root).map_err(secure("open state root"))?;
    state
        .verify_private()
        .map_err(secure("verify private state root"))?;
    let transactions = state
        .open_child_directory(OsStr::new(TRANSACTIONS_DIRECTORY))
        .map_err(secure("open transactions directory"))?;
    transactions
        .verify_private()
        .map_err(secure("verify private transactions directory"))?;
    let transaction = transactions
        .open_child_directory(OsStr::new(&input.receipt.transaction_id))
        .map_err(secure("open transaction directory"))?;
    transaction
        .verify_private()
        .map_err(secure("verify private transaction directory"))?;
    let lock_file = transaction
        .open_or_create_lock_file(OsStr::new(PUBLICATION_LOCK_NAME))
        .map_err(secure("open external effect publication lock"))?;
    let _lock = SecureFileLock::try_exclusive(lock_file)
        .map_err(secure("acquire external effect publication lock"))?;
    check_cancellation(cancellation, false, "after external effect lock")?;

    let recovered = recover_journal_head(
        &state_root.join(TRANSACTIONS_DIRECTORY),
        &input.receipt.transaction_id,
    )
    .map_err(|journal_error| {
        error(
            ExternalEffectErrorCode::InvalidEvidence,
            format!("recover exact external effect journal: {journal_error}"),
        )
    })?;
    if recovered.journal != *input.commit_pending_journal {
        return Err(error(
            ExternalEffectErrorCode::Conflict,
            "durable journal head changed before external effect receipt publication",
        ));
    }
    let expected_confirmation = canonical_line(input.consumption)?;
    let durable_confirmation = transaction
        .read_child(
            OsStr::new(CONFIRMATION_NAME),
            rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES,
        )
        .map_err(secure("read durable confirmation consumption"))?;
    if durable_confirmation != expected_confirmation {
        return Err(error(
            ExternalEffectErrorCode::InvalidEvidence,
            "durable confirmation does not match external effect receipt evidence",
        ));
    }

    let receipts = state
        .open_child_directory(OsStr::new(RECEIPTS_DIRECTORY))
        .map_err(secure("open receipts directory"))?;
    receipts
        .verify_private()
        .map_err(secure("verify private receipts directory"))?;
    let receipt_name = external_effect_receipt_name(&input.receipt.transaction_id);
    match receipts.read_child(
        OsStr::new(&receipt_name),
        rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES,
    ) {
        Ok(existing) if existing == bytes => Ok(publication(
            ExternalEffectPublicationStatus::AlreadyPublished,
            input.receipt,
            receipt_name,
            bytes.len(),
        )),
        Ok(_) => Err(error(
            ExternalEffectErrorCode::RecoveryRequired,
            "conflicting external effect receipt already exists",
        )),
        Err(read_error) if read_error.code == SecureFsErrorCode::NotFound => {
            check_cancellation(cancellation, false, "before external effect receipt write")?;
            receipts
                .write_new_child(
                    OsStr::new(&receipt_name),
                    &bytes,
                    rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES,
                )
                .map_err(publication_error("publish external effect receipt"))?;
            check_cancellation(cancellation, true, "after external effect receipt write")?;
            Ok(publication(
                ExternalEffectPublicationStatus::Published,
                input.receipt,
                receipt_name,
                bytes.len(),
            ))
        }
        Err(read_error) => Err(secure("read external effect receipt")(read_error)),
    }
}

/// Classifies exact durable journal/receipt state without completing, retrying,
/// rolling back, or otherwise mutating the external manager effect.
pub fn assess_external_effect_recovery(
    state_root: &Path,
    transaction_id: &str,
) -> Result<ExternalEffectRecoveryAssessment, ExternalEffectError> {
    if !rz0_validation_contract::valid_ledger_id(transaction_id, 96) {
        return Err(error(
            ExternalEffectErrorCode::InvalidEvidence,
            "external effect transaction ID is invalid",
        ));
    }
    let recovered = recover_journal_head(&state_root.join(TRANSACTIONS_DIRECTORY), transaction_id)
        .map_err(|journal_error| {
            error(
                ExternalEffectErrorCode::InvalidEvidence,
                format!("recover external effect journal: {journal_error}"),
            )
        })?;
    let state = SecureDirectory::open(state_root).map_err(secure("open recovery state root"))?;
    state
        .verify_private()
        .map_err(secure("verify private recovery state root"))?;
    let receipts = state
        .open_child_directory(OsStr::new(RECEIPTS_DIRECTORY))
        .map_err(secure("open recovery receipts directory"))?;
    receipts
        .verify_private()
        .map_err(secure("verify private recovery receipts directory"))?;
    let name = external_effect_receipt_name(transaction_id);
    let receipt_bytes = match receipts.read_child(
        OsStr::new(&name),
        rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES,
    ) {
        Ok(bytes) => Some(bytes),
        Err(read_error) if read_error.code == SecureFsErrorCode::NotFound => None,
        Err(read_error) => return Err(secure("read recovery external effect receipt")(read_error)),
    };
    let mut receipt_valid = false;
    if let Some(bytes) = receipt_bytes.as_deref()
        && let Ok(receipt) = serde_json::from_slice::<ExternalEffectReceipt>(bytes)
        && let Some(commit_pending) = commit_pending_prefix(&recovered.journal)
    {
        receipt_valid = validate_external_effect_receipt(&receipt, &commit_pending).valid;
    }
    let receipt_present = receipt_bytes.is_some();
    let decision = match recovered.journal.state {
        TransactionState::Prepared if !receipt_present => {
            ExternalEffectRecoveryDecision::AbortWithoutWrites
        }
        TransactionState::Applying | TransactionState::RecoveryRequired if !receipt_present => {
            ExternalEffectRecoveryDecision::VerifyExternalEffect
        }
        TransactionState::CommitPending | TransactionState::RecoveryRequired if receipt_valid => {
            ExternalEffectRecoveryDecision::CompleteJournalCommitWithExplicitApproval
        }
        TransactionState::Committed if receipt_valid => ExternalEffectRecoveryDecision::NoAction,
        _ => ExternalEffectRecoveryDecision::RefuseInconsistentEvidence,
    };
    Ok(ExternalEffectRecoveryAssessment {
        schema_version: EXTERNAL_EFFECT_RECEIPT_SCHEMA_VERSION,
        contract: EXTERNAL_EFFECT_RECOVERY_CONTRACT.to_string(),
        read_only: true,
        writes_attempted: false,
        transaction_id: transaction_id.to_string(),
        journal_state: recovered.journal.state,
        receipt_present,
        receipt_valid,
        decision,
        automatic_mutation_authorized: false,
    })
}

fn validate_publication_evidence(
    input: ExternalEffectPublicationInput<'_>,
) -> Result<(), ExternalEffectError> {
    let validation = validate_external_effect_receipt(input.receipt, input.commit_pending_journal);
    let digests = action_plan_digests(input.action_plan).map_err(|errors| {
        error(
            ExternalEffectErrorCode::InvalidEvidence,
            format!("external effect action plan is invalid: {errors:?}"),
        )
    })?;
    let confirmation =
        validate_confirmation_consumption(input.consumption, input.challenge, input.response);
    let capabilities = input
        .action_plan
        .actions
        .iter()
        .flat_map(|action| action.capabilities.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let expected_kind = match input.commit_pending_journal.operation {
        TransactionOperation::Update => Some(ActionKind::Update),
        TransactionOperation::Uninstall => Some(ActionKind::Uninstall),
        _ => None,
    };
    let action = input.action_plan.actions.first();
    let receipt_matches_action = action.is_some_and(|action| {
        input.action_plan.actions.len() == 1
            && input.receipt.action_id == action.action_id
            && input.receipt.target == action.target
            && input.receipt.manager.as_str() == action.manager.as_deref().unwrap_or_default()
            && input.receipt.executable_sha256
                == action
                    .executable_identity
                    .as_ref()
                    .map(|identity| identity.sha256.as_str())
                    .unwrap_or_default()
            && input.receipt.executable_size_bytes
                == action
                    .executable_identity
                    .as_ref()
                    .map(|identity| identity.size_bytes)
                    .unwrap_or_default()
            && input.receipt.arguments_sha256 == arguments_sha256(&action.arguments)
            && input.receipt.rollback_supported == action.rollback.supported
    });
    let valid = validation.valid
        && confirmation.valid
        && input.commit_pending_journal.plan_id == input.action_plan.plan_id
        && input.commit_pending_journal.transaction_id == input.consumption.transaction_id
        && input.consumption.plan_id == input.action_plan.plan_id
        && input.challenge.plan_sha256 == digests.plan_sha256
        && input.challenge.write_set_sha256 == digests.write_set_sha256
        && input.challenge.capabilities == capabilities
        && input.challenge.risk
            == if expected_kind == Some(ActionKind::Uninstall) {
                ConfirmationRisk::Destructive
            } else {
                ConfirmationRisk::Mutating
            }
        && expected_kind.is_some_and(|kind| {
            input
                .action_plan
                .actions
                .iter()
                .all(|action| action.kind == kind)
        })
        && receipt_matches_action
        && input.receipt.started_unix_seconds >= input.response.confirmed_unix_seconds
        && input.receipt.action_plan_sha256 == digests.plan_sha256
        && input.receipt.write_set_sha256 == digests.write_set_sha256
        && input.receipt.confirmation_challenge_sha256 == input.challenge.challenge_sha256
        && input.receipt.confirmation_response_sha256
            == confirmation_response_sha256(input.response)
        && input.receipt.confirmation_consumption_sha256 == input.consumption.binding_sha256;
    if valid {
        Ok(())
    } else {
        Err(error(
            ExternalEffectErrorCode::InvalidEvidence,
            format!(
                "external effect publication evidence is invalid: receipt={:?}; confirmation={:?}",
                validation.errors, confirmation.errors
            ),
        ))
    }
}

fn commit_pending_prefix(journal: &TransactionJournal) -> Option<TransactionJournal> {
    match journal.state {
        TransactionState::CommitPending => Some(journal.clone()),
        TransactionState::Committed | TransactionState::RecoveryRequired => {
            let mut prefix = journal.clone();
            let final_event = prefix.events.pop()?;
            let expected = match journal.state {
                TransactionState::Committed => crate::TransactionEventKind::Committed,
                TransactionState::RecoveryRequired => crate::TransactionEventKind::RecoveryRequired,
                _ => unreachable!(),
            };
            if final_event.kind != expected
                || prefix.events.last()?.kind != crate::TransactionEventKind::CommitStarted
            {
                return None;
            }
            prefix.state = TransactionState::CommitPending;
            Some(prefix)
        }
        _ => None,
    }
}

fn external_effect_receipt_name(transaction_id: &str) -> String {
    format!("external-effect.{transaction_id}.json")
}

fn publication(
    status: ExternalEffectPublicationStatus,
    receipt: &ExternalEffectReceipt,
    receipt_name: String,
    bytes: usize,
) -> ExternalEffectPublication {
    ExternalEffectPublication {
        status,
        transaction_id: receipt.transaction_id.clone(),
        receipt_name,
        receipt_bytes: bytes as u64,
        automatic_mutation_authorized: false,
    }
}

fn canonical_line<T: Serialize>(value: &T) -> Result<Vec<u8>, ExternalEffectError> {
    let mut bytes = serde_json::to_vec(value).map_err(|serialization_error| {
        error(
            ExternalEffectErrorCode::InvalidEvidence,
            format!("serialize external effect evidence: {serialization_error}"),
        )
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn external_effect_receipt_sha256(receipt: &ExternalEffectReceipt) -> String {
    let mut digest = Sha256::new();
    digest.update(b"runtime.zero.external-effect-receipt.v1\0");
    put(&mut digest, &receipt.transaction_id);
    put(&mut digest, &receipt.plan_id);
    put(&mut digest, &receipt.action_id);
    put(&mut digest, operation_name(receipt.operation));
    put(&mut digest, &receipt.manager);
    put(&mut digest, &receipt.target);
    put(&mut digest, &receipt.executable_sha256);
    digest.update(receipt.executable_size_bytes.to_be_bytes());
    put(&mut digest, &receipt.executable_binding);
    put(&mut digest, &receipt.arguments_sha256);
    digest.update(receipt.started_unix_seconds.to_be_bytes());
    digest.update(receipt.completed_unix_seconds.to_be_bytes());
    digest.update(receipt.exit_code.to_be_bytes());
    digest.update(receipt.stdout_bytes.to_be_bytes());
    digest.update(receipt.stderr_bytes.to_be_bytes());
    put(&mut digest, &receipt.stdout_sha256);
    put(&mut digest, &receipt.stderr_sha256);
    put(&mut digest, &receipt.verification_sha256);
    digest.update(receipt.commit_pending_sequence.to_be_bytes());
    put(&mut digest, &receipt.commit_pending_event_sha256);
    put(&mut digest, &receipt.commit_pending_snapshot_name);
    put(&mut digest, &receipt.action_plan_sha256);
    put(&mut digest, &receipt.write_set_sha256);
    put(&mut digest, &receipt.confirmation_challenge_sha256);
    put(&mut digest, &receipt.confirmation_response_sha256);
    put(&mut digest, &receipt.confirmation_consumption_sha256);
    for value in [
        receipt.rollback_supported,
        receipt.writes_attempted,
        receipt.automatic_mutation_authorized,
    ] {
        digest.update([u8::from(value)]);
    }
    put(&mut digest, "verified");
    format!("{:x}", digest.finalize())
}

fn operation_name(operation: TransactionOperation) -> &'static str {
    match operation {
        TransactionOperation::Update => "update",
        TransactionOperation::Uninstall => "uninstall",
        _ => "unsupported",
    }
}

fn put(digest: &mut Sha256, value: &str) {
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value.as_bytes());
}

fn check_cancellation(
    cancellation: Option<&CancellationToken>,
    partial: bool,
    boundary: &str,
) -> Result<(), ExternalEffectError> {
    let Some(reason) = cancellation.and_then(CancellationToken::reason) else {
        return Ok(());
    };
    Err(error(
        if partial {
            ExternalEffectErrorCode::RecoveryRequired
        } else {
            ExternalEffectErrorCode::Cancelled
        },
        format!("external effect publication cancelled {boundary}: {reason:?}"),
    ))
}

fn secure(
    context: &'static str,
) -> impl FnOnce(SecureFsError) -> ExternalEffectError + Copy + 'static {
    move |filesystem_error| ExternalEffectError {
        code: match filesystem_error.code {
            SecureFsErrorCode::UnsafeName
            | SecureFsErrorCode::UnsafeDirectory
            | SecureFsErrorCode::IdentityChanged => ExternalEffectErrorCode::UnsafeFilesystem,
            SecureFsErrorCode::UnsupportedOperation => ExternalEffectErrorCode::Unsupported,
            SecureFsErrorCode::NotFound => ExternalEffectErrorCode::EvidenceMissing,
            SecureFsErrorCode::AlreadyExists | SecureFsErrorCode::LockBusy => {
                ExternalEffectErrorCode::Conflict
            }
            SecureFsErrorCode::LimitExceeded => ExternalEffectErrorCode::LimitExceeded,
            SecureFsErrorCode::PublicationIncomplete => ExternalEffectErrorCode::RecoveryRequired,
            SecureFsErrorCode::Io => ExternalEffectErrorCode::Io,
        },
        detail: format!("{context}: {filesystem_error}"),
    }
}

fn publication_error(
    context: &'static str,
) -> impl FnOnce(SecureFsError) -> ExternalEffectError + Copy + 'static {
    move |filesystem_error| {
        let mut mapped = secure(context)(filesystem_error);
        if matches!(
            mapped.code,
            ExternalEffectErrorCode::Conflict | ExternalEffectErrorCode::Io
        ) {
            mapped.code = ExternalEffectErrorCode::RecoveryRequired;
        }
        mapped
    }
}

fn error(code: ExternalEffectErrorCode, detail: impl Into<String>) -> ExternalEffectError {
    ExternalEffectError {
        code,
        detail: detail.into(),
    }
}
