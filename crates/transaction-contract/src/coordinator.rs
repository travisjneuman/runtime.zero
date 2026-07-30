use std::{collections::BTreeSet, ffi::OsStr, fmt, path::Path};

use rz0_action_plan::{ActionKind, ActionPlan, action_plan_digests};
use rz0_confirmation_contract::{
    ConfirmationChallenge, ConfirmationConsumption, ConfirmationResponse, ConfirmationRisk,
    confirmation_response_sha256, validate_confirmation_consumption,
};
use rz0_registry_contract::{
    InstalledRegistry, bytes_sha256, canonical_registry_bytes, parse_registry_document,
};
use rz0_secure_fs::{SecureDirectory, SecureFileLock, SecureFsError, SecureFsErrorCode};

use crate::{
    TransactionCommitReceipt, TransactionJournal, TransactionOperation, TransactionState,
    validate_commit_receipt, validate_transaction_journal,
};

const TRANSACTIONS_DIRECTORY: &str = "transactions";
const RECEIPTS_DIRECTORY: &str = "receipts";
const HEADS_DIRECTORY: &str = "heads";
const REGISTRY_NAME: &str = "installed-modules.json";
const COMMIT_LOCK_NAME: &str = ".commit.lock";
const CONFIRMATION_LOCK_NAME: &str = ".confirmation.lock";
const CONFIRMATION_NAME: &str = "confirmation.json";
const REGISTRY_BEFORE_NAME: &str = "registry-before.json";
const REGISTRY_NEXT_NAME: &str = "registry-next.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordinatorErrorCode {
    InvalidEvidence,
    EvidenceMissing,
    UnsafeFilesystem,
    Conflict,
    RecoveryRequired,
    LimitExceeded,
    Unsupported,
    Io,
}

#[derive(Debug)]
pub struct CoordinatorError {
    pub code: CoordinatorErrorCode,
    detail: String,
}

impl fmt::Display for CoordinatorError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for CoordinatorError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidencePublicationStatus {
    Published,
    AlreadyPublished,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmationPublication {
    pub status: EvidencePublicationStatus,
    pub transaction_id: String,
    pub confirmation_name: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitPublicationStatus {
    Committed,
    AlreadyCommitted,
}

#[derive(Debug, Clone, Copy)]
pub struct CommitCoordinatorInput<'a> {
    pub committed_journal: &'a TransactionJournal,
    pub action_plan: &'a ActionPlan,
    pub challenge: &'a ConfirmationChallenge,
    pub response: &'a ConfirmationResponse,
    pub consumption: &'a ConfirmationConsumption,
    pub receipt: &'a TransactionCommitReceipt,
    pub next_registry: &'a InstalledRegistry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitPublication {
    pub status: CommitPublicationStatus,
    pub transaction_id: String,
    pub receipt_name: String,
    pub registry_sha256: String,
    pub registry_bytes: u64,
    pub automatic_mutation_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitRecoveryDecision {
    NoAction,
    CompleteRegistryPublicationWithExplicitApproval,
    DiscardUncommittedPendingWithExplicitApproval,
    RefuseInconsistentEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRecoveryAssessment {
    pub transaction_id: String,
    pub decision: CommitRecoveryDecision,
    pub committed_journal_present: bool,
    pub confirmation_present: bool,
    pub receipt_present: bool,
    pub registry_matches_after: bool,
    pub pending_registry_present: bool,
    pub rollback_registry_present: bool,
    pub automatic_mutation_authorized: bool,
}

/// Durably consumes one exact confirmation after the prepared journal exists
/// and before any production execution can be considered. The evidence itself
/// never authorizes execution.
pub fn publish_confirmation_consumption(
    state_root: &Path,
    prepared_journal: &TransactionJournal,
    action_plan: &ActionPlan,
    challenge: &ConfirmationChallenge,
    response: &ConfirmationResponse,
    consumption: &ConfirmationConsumption,
) -> Result<ConfirmationPublication, CoordinatorError> {
    validate_prepared_confirmation(
        prepared_journal,
        action_plan,
        challenge,
        response,
        consumption,
    )?;
    let bytes = canonical_line(consumption, "serialize confirmation consumption")?;
    ensure_small_document(&bytes, "confirmation consumption")?;

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
        .open_child_directory(OsStr::new(&prepared_journal.transaction_id))
        .map_err(secure("open transaction directory"))?;
    transaction
        .verify_private()
        .map_err(secure("verify private transaction directory"))?;
    let lock_file = transaction
        .open_or_create_lock_file(OsStr::new(CONFIRMATION_LOCK_NAME))
        .map_err(secure("open confirmation writer lock"))?;
    let _lock = SecureFileLock::try_exclusive(lock_file)
        .map_err(secure("acquire confirmation writer lock"))?;
    verify_journal_head(&transaction, prepared_journal)?;

    match transaction.read_child(
        OsStr::new(CONFIRMATION_NAME),
        rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES,
    ) {
        Ok(existing) if existing == bytes => Ok(ConfirmationPublication {
            status: EvidencePublicationStatus::AlreadyPublished,
            transaction_id: prepared_journal.transaction_id.clone(),
            confirmation_name: CONFIRMATION_NAME.to_string(),
            bytes: bytes.len() as u64,
        }),
        Ok(_) => Err(error(
            CoordinatorErrorCode::RecoveryRequired,
            "transaction contains conflicting confirmation consumption evidence",
        )),
        Err(error) if error.code == SecureFsErrorCode::NotFound => {
            transaction
                .write_new_child(
                    OsStr::new(CONFIRMATION_NAME),
                    &bytes,
                    rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES,
                )
                .map_err(secure("publish confirmation consumption"))?;
            Ok(ConfirmationPublication {
                status: EvidencePublicationStatus::Published,
                transaction_id: prepared_journal.transaction_id.clone(),
                confirmation_name: CONFIRMATION_NAME.to_string(),
                bytes: bytes.len() as u64,
            })
        }
        Err(error) => Err(secure("read confirmation consumption")(error)),
    }
}

/// Publishes commit evidence and the canonical installed registry in the fixed
/// order: committed journal, consumed confirmation, rollback copy/pending
/// registry, receipt, then atomic registry replacement last.
pub fn publish_committed_state(
    state_root: &Path,
    input: CommitCoordinatorInput<'_>,
) -> Result<CommitPublication, CoordinatorError> {
    let CommitCoordinatorInput {
        committed_journal,
        action_plan,
        challenge,
        response,
        consumption,
        receipt,
        next_registry,
    } = input;
    let registry_bytes =
        canonical_registry_bytes(next_registry).map_err(|error| CoordinatorError {
            code: CoordinatorErrorCode::InvalidEvidence,
            detail: format!("canonical next registry: {error}"),
        })?;
    let receipt_bytes = canonical_line(receipt, "serialize commit receipt")?;
    ensure_small_document(&receipt_bytes, "commit receipt")?;
    validate_commit_evidence(
        committed_journal,
        action_plan,
        challenge,
        response,
        consumption,
        receipt,
        &registry_bytes,
    )?;

    let state = SecureDirectory::open(state_root).map_err(secure("open state root"))?;
    state
        .verify_private()
        .map_err(secure("verify private state root"))?;
    let lock_file = state
        .open_or_create_lock_file(OsStr::new(COMMIT_LOCK_NAME))
        .map_err(secure("open commit coordinator lock"))?;
    let _lock = SecureFileLock::try_exclusive(lock_file)
        .map_err(secure("acquire commit coordinator lock"))?;
    let transactions = state
        .open_child_directory(OsStr::new(TRANSACTIONS_DIRECTORY))
        .map_err(secure("open transactions directory"))?;
    let transaction = transactions
        .open_child_directory(OsStr::new(&committed_journal.transaction_id))
        .map_err(secure("open transaction directory"))?;
    transactions
        .verify_private()
        .map_err(secure("verify private transactions directory"))?;
    transaction
        .verify_private()
        .map_err(secure("verify private transaction directory"))?;
    let receipts = state
        .open_child_directory(OsStr::new(RECEIPTS_DIRECTORY))
        .map_err(secure("open receipts directory"))?;
    receipts
        .verify_private()
        .map_err(secure("verify private receipts directory"))?;
    verify_journal_head(&transaction, committed_journal)?;
    verify_confirmation(&transaction, consumption)?;

    let receipt_name = format!("{}.json", receipt.plan_id);
    let prior_receipt = read_optional(
        &receipts,
        &receipt_name,
        rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES,
    )?;
    let current_registry = read_optional(
        &state,
        REGISTRY_NAME,
        rz0_resource_contract::MAX_REGISTRY_DOCUMENT_BYTES,
    )?;
    if prior_receipt.as_deref() == Some(receipt_bytes.as_slice())
        && current_registry.as_deref() == Some(registry_bytes.as_slice())
    {
        return Ok(commit_publication(
            CommitPublicationStatus::AlreadyCommitted,
            receipt,
            &receipt_name,
            &registry_bytes,
        ));
    }
    if prior_receipt.is_some() {
        return Err(error(
            CoordinatorErrorCode::RecoveryRequired,
            "commit receipt exists without the exact final registry state",
        ));
    }

    validate_prior_registry(current_registry.as_deref(), receipt, challenge)?;
    if let Some(current) = current_registry.as_deref() {
        write_recovery_document(
            &transaction,
            REGISTRY_BEFORE_NAME,
            current,
            rz0_resource_contract::MAX_REGISTRY_DOCUMENT_BYTES,
            "registry rollback copy",
        )?;
    }
    write_recovery_document(
        &transaction,
        REGISTRY_NEXT_NAME,
        &registry_bytes,
        rz0_resource_contract::MAX_REGISTRY_DOCUMENT_BYTES,
        "pending next registry",
    )?;
    receipts
        .write_new_child(
            OsStr::new(&receipt_name),
            &receipt_bytes,
            rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES,
        )
        .map_err(publication_error("publish commit receipt"))?;

    if current_registry.is_some() {
        transaction
            .replace_child_atomic(
                OsStr::new(REGISTRY_NEXT_NAME),
                &state,
                OsStr::new(REGISTRY_NAME),
            )
            .map_err(publication_error("atomically replace installed registry"))?;
    } else {
        transaction
            .publish_child_noreplace(
                OsStr::new(REGISTRY_NEXT_NAME),
                &state,
                OsStr::new(REGISTRY_NAME),
            )
            .map_err(publication_error("publish initial installed registry"))?;
    }
    let published = state
        .read_child(
            OsStr::new(REGISTRY_NAME),
            rz0_resource_contract::MAX_REGISTRY_DOCUMENT_BYTES,
        )
        .map_err(secure("verify published installed registry"))?;
    if published != registry_bytes {
        return Err(error(
            CoordinatorErrorCode::RecoveryRequired,
            "published registry bytes do not match the exact committed registry",
        ));
    }
    Ok(commit_publication(
        CommitPublicationStatus::Committed,
        receipt,
        &receipt_name,
        &registry_bytes,
    ))
}

/// Classifies an interrupted commit from exact durable evidence. The result is
/// read-only and can never authorize automatic cleanup, rollback, or completion.
pub fn assess_commit_recovery(
    state_root: &Path,
    committed_journal: &TransactionJournal,
    consumption: &ConfirmationConsumption,
    receipt: &TransactionCommitReceipt,
    next_registry: &InstalledRegistry,
) -> Result<CommitRecoveryAssessment, CoordinatorError> {
    if !validate_commit_receipt(receipt, committed_journal).valid
        || committed_journal.state != TransactionState::Committed
    {
        return Err(error(
            CoordinatorErrorCode::InvalidEvidence,
            "recovery assessment requires an exact valid committed journal and receipt",
        ));
    }
    let registry_after =
        canonical_registry_bytes(next_registry).map_err(|error| CoordinatorError {
            code: CoordinatorErrorCode::InvalidEvidence,
            detail: format!("canonical recovery registry: {error}"),
        })?;
    if bytes_sha256(&registry_after) != receipt.registry_after_sha256
        || consumption.transaction_id != committed_journal.transaction_id
        || consumption.binding_sha256 != receipt.confirmation_consumption_sha256
    {
        return Err(error(
            CoordinatorErrorCode::InvalidEvidence,
            "recovery inputs do not bind the exact transaction state",
        ));
    }

    let state = SecureDirectory::open(state_root).map_err(secure("open recovery state root"))?;
    state
        .verify_private()
        .map_err(secure("verify private recovery state root"))?;
    let transactions = state
        .open_child_directory(OsStr::new(TRANSACTIONS_DIRECTORY))
        .map_err(secure("open recovery transactions directory"))?;
    transactions
        .verify_private()
        .map_err(secure("verify private recovery transactions directory"))?;
    let transaction = transactions
        .open_child_directory(OsStr::new(&committed_journal.transaction_id))
        .map_err(secure("open recovery transaction directory"))?;
    transaction
        .verify_private()
        .map_err(secure("verify private recovery transaction directory"))?;
    let receipts = state
        .open_child_directory(OsStr::new(RECEIPTS_DIRECTORY))
        .map_err(secure("open recovery receipts directory"))?;
    receipts
        .verify_private()
        .map_err(secure("verify private recovery receipts directory"))?;
    verify_journal_head(&transaction, committed_journal)?;
    verify_confirmation(&transaction, consumption)?;

    let receipt_name = format!("{}.json", receipt.plan_id);
    let receipt_bytes = canonical_line(receipt, "serialize recovery receipt")?;
    let durable_receipt = read_optional(
        &receipts,
        &receipt_name,
        rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES,
    )?;
    let registry = read_optional(
        &state,
        REGISTRY_NAME,
        rz0_resource_contract::MAX_REGISTRY_DOCUMENT_BYTES,
    )?;
    let pending = read_optional(
        &transaction,
        REGISTRY_NEXT_NAME,
        rz0_resource_contract::MAX_REGISTRY_DOCUMENT_BYTES,
    )?;
    let rollback = read_optional(
        &transaction,
        REGISTRY_BEFORE_NAME,
        rz0_resource_contract::MAX_REGISTRY_DOCUMENT_BYTES,
    )?;
    let receipt_present = durable_receipt.as_deref() == Some(receipt_bytes.as_slice());
    let registry_matches_after = registry.as_deref() == Some(registry_after.as_slice());
    let pending_registry_present = pending.as_deref() == Some(registry_after.as_slice());
    let prior_matches = match (
        registry.as_deref(),
        receipt.registry_before_sha256.as_deref(),
    ) {
        (None, None) => true,
        (Some(bytes), Some(expected)) => bytes_sha256(bytes) == expected,
        _ => false,
    };
    let rollback_registry_present = rollback.as_deref().is_some_and(|bytes| {
        receipt
            .registry_before_sha256
            .as_deref()
            .is_some_and(|expected| bytes_sha256(bytes) == expected)
    });
    let decision = if receipt_present && registry_matches_after && pending.is_none() {
        CommitRecoveryDecision::NoAction
    } else if receipt_present && pending_registry_present && prior_matches {
        CommitRecoveryDecision::CompleteRegistryPublicationWithExplicitApproval
    } else if !receipt_present && pending_registry_present && prior_matches {
        CommitRecoveryDecision::DiscardUncommittedPendingWithExplicitApproval
    } else {
        CommitRecoveryDecision::RefuseInconsistentEvidence
    };
    Ok(CommitRecoveryAssessment {
        transaction_id: committed_journal.transaction_id.clone(),
        decision,
        committed_journal_present: true,
        confirmation_present: true,
        receipt_present,
        registry_matches_after,
        pending_registry_present,
        rollback_registry_present,
        automatic_mutation_authorized: false,
    })
}

fn validate_prepared_confirmation(
    journal: &TransactionJournal,
    plan: &ActionPlan,
    challenge: &ConfirmationChallenge,
    response: &ConfirmationResponse,
    consumption: &ConfirmationConsumption,
) -> Result<(), CoordinatorError> {
    let journal_validation = validate_transaction_journal(journal);
    if !journal_validation.valid || journal.state != TransactionState::Prepared {
        return Err(error(
            CoordinatorErrorCode::InvalidEvidence,
            "confirmation consumption requires the exact valid prepared journal",
        ));
    }
    validate_shared_evidence(journal, plan, challenge, response, consumption)
}

fn validate_commit_evidence(
    journal: &TransactionJournal,
    plan: &ActionPlan,
    challenge: &ConfirmationChallenge,
    response: &ConfirmationResponse,
    consumption: &ConfirmationConsumption,
    receipt: &TransactionCommitReceipt,
    registry_bytes: &[u8],
) -> Result<(), CoordinatorError> {
    validate_shared_evidence(journal, plan, challenge, response, consumption)?;
    let validation = validate_commit_receipt(receipt, journal);
    if !validation.valid || journal.state != TransactionState::Committed {
        return Err(error(
            CoordinatorErrorCode::InvalidEvidence,
            format!(
                "commit receipt or journal is invalid: {:?}",
                validation.errors
            ),
        ));
    }
    let digests = action_plan_digests(plan).map_err(|errors| {
        error(
            CoordinatorErrorCode::InvalidEvidence,
            format!("action plan digest validation failed: {errors:?}"),
        )
    })?;
    let response_sha256 = confirmation_response_sha256(response);
    if receipt.action_plan_sha256 != digests.plan_sha256
        || receipt.write_set_sha256 != digests.write_set_sha256
        || receipt.confirmation_challenge_sha256 != challenge.challenge_sha256
        || receipt.confirmation_response_sha256 != response_sha256
        || receipt.confirmation_consumption_sha256 != consumption.binding_sha256
        || receipt.registry_after_sha256 != bytes_sha256(registry_bytes)
        || challenge.before_state_sha256 != receipt.registry_before_sha256
        || challenge.expected_after_state_sha256 != receipt.registry_after_sha256
    {
        return Err(error(
            CoordinatorErrorCode::InvalidEvidence,
            "commit evidence does not bind the exact plan, confirmation, and registry states",
        ));
    }
    Ok(())
}

fn validate_shared_evidence(
    journal: &TransactionJournal,
    plan: &ActionPlan,
    challenge: &ConfirmationChallenge,
    response: &ConfirmationResponse,
    consumption: &ConfirmationConsumption,
) -> Result<(), CoordinatorError> {
    let digests = action_plan_digests(plan).map_err(|errors| {
        error(
            CoordinatorErrorCode::InvalidEvidence,
            format!("action plan validation failed: {errors:?}"),
        )
    })?;
    let confirmation = validate_confirmation_consumption(consumption, challenge, response);
    let capabilities = plan
        .actions
        .iter()
        .flat_map(|action| action.capabilities.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let expected_risk = if plan
        .actions
        .iter()
        .any(|action| action.kind == ActionKind::Uninstall)
    {
        ConfirmationRisk::Destructive
    } else {
        ConfirmationRisk::Mutating
    };
    if !confirmation.valid
        || journal.plan_id != plan.plan_id
        || journal.transaction_id != consumption.transaction_id
        || consumption.plan_id != plan.plan_id
        || challenge.plan_sha256 != digests.plan_sha256
        || challenge.write_set_sha256 != digests.write_set_sha256
        || usize::from(challenge.action_count) != plan.actions.len()
        || challenge.capabilities != capabilities
        || challenge.risk != expected_risk
        || !operation_matches_plan(journal.operation, plan)
    {
        return Err(error(
            CoordinatorErrorCode::InvalidEvidence,
            format!(
                "journal, plan, or confirmation binding is invalid: {:?}",
                confirmation.errors
            ),
        ));
    }
    Ok(())
}

fn operation_matches_plan(operation: TransactionOperation, plan: &ActionPlan) -> bool {
    let expected = match operation {
        TransactionOperation::Update => Some(ActionKind::Update),
        TransactionOperation::Uninstall => Some(ActionKind::Uninstall),
        TransactionOperation::Quarantine => Some(ActionKind::Quarantine),
        TransactionOperation::Restore => Some(ActionKind::Restore),
        TransactionOperation::ModuleInstall
        | TransactionOperation::ModuleUpgrade
        | TransactionOperation::ModuleRepair
        | TransactionOperation::ModuleUninstall => None,
    };
    expected.is_none_or(|kind| plan.actions.iter().all(|action| action.kind == kind))
}

fn verify_journal_head(
    transaction: &SecureDirectory,
    journal: &TransactionJournal,
) -> Result<(), CoordinatorError> {
    let heads = transaction
        .open_child_directory(OsStr::new(HEADS_DIRECTORY))
        .map_err(secure("open immutable journal heads"))?;
    heads
        .verify_private()
        .map_err(secure("verify private journal heads"))?;
    let head = journal.events.last().ok_or_else(|| {
        error(
            CoordinatorErrorCode::InvalidEvidence,
            "journal has no committed head",
        )
    })?;
    let name = format!("{:04}-{}.json", head.sequence, head.event_sha256);
    let bytes = heads
        .read_child(
            OsStr::new(&name),
            rz0_resource_contract::MAX_JOURNAL_SNAPSHOT_BYTES,
        )
        .map_err(secure("read exact immutable journal head"))?;
    let durable =
        serde_json::from_slice::<TransactionJournal>(&bytes).map_err(|error| CoordinatorError {
            code: CoordinatorErrorCode::InvalidEvidence,
            detail: format!("parse exact immutable journal head: {error}"),
        })?;
    if durable != *journal || !validate_transaction_journal(&durable).valid {
        return Err(error(
            CoordinatorErrorCode::InvalidEvidence,
            "durable journal head does not match the exact supplied journal",
        ));
    }
    Ok(())
}

fn verify_confirmation(
    transaction: &SecureDirectory,
    consumption: &ConfirmationConsumption,
) -> Result<(), CoordinatorError> {
    let expected = canonical_line(consumption, "serialize confirmation consumption")?;
    let durable = transaction
        .read_child(
            OsStr::new(CONFIRMATION_NAME),
            rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES,
        )
        .map_err(secure("read durable confirmation consumption"))?;
    if durable != expected {
        return Err(error(
            CoordinatorErrorCode::InvalidEvidence,
            "durable confirmation consumption does not match exact commit evidence",
        ));
    }
    Ok(())
}

fn validate_prior_registry(
    current: Option<&[u8]>,
    receipt: &TransactionCommitReceipt,
    challenge: &ConfirmationChallenge,
) -> Result<(), CoordinatorError> {
    match (current, receipt.registry_before_sha256.as_deref()) {
        (None, None) => Ok(()),
        (Some(bytes), Some(expected)) => {
            parse_registry_document(bytes).map_err(|error| CoordinatorError {
                code: CoordinatorErrorCode::InvalidEvidence,
                detail: format!("current installed registry is invalid: {error}"),
            })?;
            if bytes_sha256(bytes) == expected
                && challenge.before_state_sha256.as_deref() == Some(expected)
            {
                Ok(())
            } else {
                Err(error(
                    CoordinatorErrorCode::Conflict,
                    "current installed registry changed after confirmation",
                ))
            }
        }
        _ => Err(error(
            CoordinatorErrorCode::Conflict,
            "installed registry presence changed after confirmation",
        )),
    }
}

fn write_recovery_document(
    directory: &SecureDirectory,
    name: &str,
    bytes: &[u8],
    maximum_bytes: u64,
    context: &'static str,
) -> Result<(), CoordinatorError> {
    match directory.write_new_child(OsStr::new(name), bytes, maximum_bytes) {
        Ok(_) => Ok(()),
        Err(error) if error.code == SecureFsErrorCode::AlreadyExists => Err(CoordinatorError {
            code: CoordinatorErrorCode::RecoveryRequired,
            detail: format!("{context} already exists; explicit recovery assessment is required"),
        }),
        Err(error) => Err(secure(context)(error)),
    }
}

fn read_optional(
    directory: &SecureDirectory,
    name: &str,
    maximum_bytes: u64,
) -> Result<Option<Vec<u8>>, CoordinatorError> {
    match directory.read_child(OsStr::new(name), maximum_bytes) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.code == SecureFsErrorCode::NotFound => Ok(None),
        Err(error) => Err(secure("read optional transaction document")(error)),
    }
}

fn canonical_line<T: serde::Serialize>(
    value: &T,
    context: &str,
) -> Result<Vec<u8>, CoordinatorError> {
    let mut bytes = serde_json::to_vec(value).map_err(|error| CoordinatorError {
        code: CoordinatorErrorCode::InvalidEvidence,
        detail: format!("{context}: {error}"),
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn ensure_small_document(bytes: &[u8], name: &str) -> Result<(), CoordinatorError> {
    if bytes.len() as u64 <= rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES {
        Ok(())
    } else {
        Err(error(
            CoordinatorErrorCode::LimitExceeded,
            format!("{name} exceeds the foundation document ceiling"),
        ))
    }
}

fn commit_publication(
    status: CommitPublicationStatus,
    receipt: &TransactionCommitReceipt,
    receipt_name: &str,
    registry_bytes: &[u8],
) -> CommitPublication {
    CommitPublication {
        status,
        transaction_id: receipt.transaction_id.clone(),
        receipt_name: receipt_name.to_string(),
        registry_sha256: bytes_sha256(registry_bytes),
        registry_bytes: registry_bytes.len() as u64,
        automatic_mutation_authorized: false,
    }
}

fn secure(
    context: &'static str,
) -> impl FnOnce(SecureFsError) -> CoordinatorError + Copy + 'static {
    move |error| CoordinatorError {
        code: match error.code {
            SecureFsErrorCode::UnsafeName | SecureFsErrorCode::IdentityChanged => {
                CoordinatorErrorCode::UnsafeFilesystem
            }
            SecureFsErrorCode::UnsafeDirectory => CoordinatorErrorCode::UnsafeFilesystem,
            SecureFsErrorCode::UnsupportedOperation => CoordinatorErrorCode::Unsupported,
            SecureFsErrorCode::NotFound => CoordinatorErrorCode::EvidenceMissing,
            SecureFsErrorCode::AlreadyExists | SecureFsErrorCode::LockBusy => {
                CoordinatorErrorCode::Conflict
            }
            SecureFsErrorCode::LimitExceeded => CoordinatorErrorCode::LimitExceeded,
            SecureFsErrorCode::PublicationIncomplete => CoordinatorErrorCode::RecoveryRequired,
            SecureFsErrorCode::Io => CoordinatorErrorCode::Io,
        },
        detail: format!("{context}: {error}"),
    }
}

fn publication_error(
    context: &'static str,
) -> impl FnOnce(SecureFsError) -> CoordinatorError + Copy + 'static {
    move |error| {
        let mut mapped = secure(context)(error);
        if matches!(
            mapped.code,
            CoordinatorErrorCode::Conflict | CoordinatorErrorCode::Io
        ) {
            mapped.code = CoordinatorErrorCode::RecoveryRequired;
        }
        mapped
    }
}

fn error(code: CoordinatorErrorCode, detail: impl Into<String>) -> CoordinatorError {
    CoordinatorError {
        code,
        detail: detail.into(),
    }
}
