//! Receipt-bound, root-relative quarantine and restore execution.
//!
//! This crate deliberately accepts only one already-validated action, one
//! durable confirmation consumption, and two explicit private roots. It never
//! follows links, overwrites a destination, recursively deletes data, or
//! authorizes an action by itself.

use std::{
    ffi::{OsStr, OsString},
    fmt,
    path::{Component, Path, PathBuf},
};

use rz0_action_plan::{
    ActionDisposition, ActionKind, ActionPlan, PlanAction, WriteKind, action_plan_digests,
    validate_action_plan,
};
use rz0_cancellation_contract::CancellationToken;
use rz0_confirmation_contract::{
    ConfirmationChallenge, ConfirmationConsumption, ConfirmationResponse, ConfirmationRisk,
    confirmation_response_sha256, validate_confirmation_consumption,
};
use rz0_secure_fs::{SecureDirectory, SecureFileLock, SecureFsError, SecureFsErrorCode};
use rz0_transaction_contract::{
    DurabilityRequirements, TransactionEvent, TransactionEventKind, TransactionJournal,
    TransactionOperation, TransactionState, publish_confirmation_consumption,
    publish_journal_snapshot, seal_transaction_journal, validate_transaction_journal,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const FILESYSTEM_EFFECT_RECEIPT_SCHEMA_VERSION: u16 = 1;
pub const FILESYSTEM_EFFECT_RECEIPT_CONTRACT: &str = "filesystem_effect_receipt";
const TRANSACTIONS_DIRECTORY: &str = "transactions";
const RECEIPTS_DIRECTORY: &str = "receipts";
const QUARANTINE_LOCK_NAME: &str = ".quarantine.lock";
const MAX_RECORD_BYTES: u64 = rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesystemEffectErrorCode {
    InvalidEvidence,
    InvalidPlan,
    UnsafeRoot,
    UnsafePath,
    Conflict,
    SourceChanged,
    RecoveryRequired,
    LimitExceeded,
    Unsupported,
    Cancelled,
    Io,
}

#[derive(Debug)]
pub struct FilesystemEffectError {
    pub code: FilesystemEffectErrorCode,
    detail: String,
}

impl FilesystemEffectError {
    fn new(code: FilesystemEffectErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }

    pub fn invalid_plan(detail: impl Into<String>) -> Self {
        Self::new(FilesystemEffectErrorCode::InvalidPlan, detail)
    }

    pub fn unsafe_path(detail: impl Into<String>) -> Self {
        Self::new(FilesystemEffectErrorCode::UnsafePath, detail)
    }

    pub const fn foundation_code(&self) -> rz0_error_contract::FoundationErrorCode {
        match self.code {
            FilesystemEffectErrorCode::InvalidEvidence | FilesystemEffectErrorCode::InvalidPlan => {
                rz0_error_contract::FoundationErrorCode::TransactionInvalid
            }
            FilesystemEffectErrorCode::UnsafeRoot | FilesystemEffectErrorCode::UnsafePath => {
                rz0_error_contract::FoundationErrorCode::PermissionDenied
            }
            FilesystemEffectErrorCode::Conflict => {
                rz0_error_contract::FoundationErrorCode::Conflict
            }
            FilesystemEffectErrorCode::SourceChanged => {
                rz0_error_contract::FoundationErrorCode::ArtifactIdentityChanged
            }
            FilesystemEffectErrorCode::RecoveryRequired => {
                rz0_error_contract::FoundationErrorCode::RecoveryRequired
            }
            FilesystemEffectErrorCode::LimitExceeded => {
                rz0_error_contract::FoundationErrorCode::InputLimitExceeded
            }
            FilesystemEffectErrorCode::Unsupported => {
                rz0_error_contract::FoundationErrorCode::UnsupportedOperation
            }
            FilesystemEffectErrorCode::Cancelled => {
                rz0_error_contract::FoundationErrorCode::Cancelled
            }
            FilesystemEffectErrorCode::Io => rz0_error_contract::FoundationErrorCode::IoUnavailable,
        }
    }
}

impl fmt::Display for FilesystemEffectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for FilesystemEffectError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FilesystemEffectStatus {
    Committed,
    RecoveryRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FilesystemEffectReport {
    pub transaction_id: String,
    pub plan_id: String,
    pub action_id: String,
    pub operation: TransactionOperation,
    pub status: FilesystemEffectStatus,
    pub source_sha256: String,
    pub source_size_bytes: u64,
    pub source_removed: bool,
    pub destination_verified: bool,
    pub receipt_reference: String,
    pub writes_attempted: bool,
    pub product_execution_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuarantineRecord {
    pub schema_version: u16,
    pub contract: String,
    pub transaction_id: String,
    pub plan_id: String,
    pub action_id: String,
    pub original_path: String,
    pub quarantine_path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub created_unix_seconds: u64,
    pub binding_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FilesystemEffectReceipt {
    pub schema_version: u16,
    pub contract: String,
    pub transaction_id: String,
    pub plan_id: String,
    pub action_id: String,
    pub operation: TransactionOperation,
    pub source_path: String,
    pub destination_path: String,
    pub source_sha256: String,
    pub source_size_bytes: u64,
    pub commit_sequence: u32,
    pub commit_event_sha256: String,
    pub commit_snapshot_name: String,
    pub action_plan_sha256: String,
    pub write_set_sha256: String,
    pub confirmation_challenge_sha256: String,
    pub confirmation_response_sha256: String,
    pub confirmation_consumption_sha256: String,
    pub source_removed: bool,
    pub destination_verified: bool,
    pub writes_attempted: bool,
    pub product_execution_authorized: bool,
    pub binding_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesystemEffectValidation {
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct FilesystemEffectRequest<'a> {
    pub state_root: &'a Path,
    pub source_root: &'a Path,
    pub quarantine_root: &'a Path,
    pub plan: &'a ActionPlan,
    pub action: &'a PlanAction,
    pub challenge: &'a ConfirmationChallenge,
    pub response: &'a ConfirmationResponse,
    pub consumption: &'a ConfirmationConsumption,
    /// Logical namespace prefix for the non-quarantine side of the effect.
    /// `None` preserves the schema-one `workspace/` default. A cache-backed
    /// caller may bind `workspace/cache` to a separate physical root.
    pub workspace_namespace: Option<&'a str>,
    pub cancellation: Option<&'a CancellationToken>,
    pub now_unix_seconds: u64,
}

pub fn execute_filesystem_effect(
    request: FilesystemEffectRequest<'_>,
) -> Result<FilesystemEffectReport, FilesystemEffectError> {
    validate_request(&request)?;
    check_cancellation(
        request.cancellation,
        false,
        "before filesystem-effect transaction",
    )?;
    let digests = action_plan_digests(request.plan).map_err(|errors| {
        error(
            FilesystemEffectErrorCode::InvalidPlan,
            format!("action plan digest failed: {errors:?}"),
        )
    })?;
    let operation = operation_for(request.action.kind);
    let transaction_id = filesystem_effect_transaction_id(
        request.action.kind,
        &request.plan.plan_id,
        request.now_unix_seconds,
    );
    let state_root = open_private_root(request.state_root, "state")?;
    let source_root = open_private_root(request.source_root, "source")?;
    let quarantine_root = open_private_root(request.quarantine_root, "quarantine")?;
    let _lock = lock_root(&quarantine_root)?;
    let transactions = open_private_child(&state_root, TRANSACTIONS_DIRECTORY, "transactions")?;
    let receipts = open_private_child(&state_root, RECEIPTS_DIRECTORY, "receipts")?;
    let transaction = prepare_transaction(
        request.state_root,
        &transactions,
        &request.plan.plan_id,
        operation,
        &transaction_id,
    )?;
    publish_confirmation_consumption(
        request.state_root,
        &transaction,
        request.plan,
        request.challenge,
        request.response,
        request.consumption,
    )
    .map_err(|cause| {
        error(
            FilesystemEffectErrorCode::InvalidEvidence,
            format!("publish confirmation consumption: {cause}"),
        )
    })?;
    check_cancellation(
        request.cancellation,
        false,
        "before filesystem-effect write intent",
    )?;

    let source = request.action.source.as_ref().ok_or_else(|| {
        error(
            FilesystemEffectErrorCode::InvalidPlan,
            "filesystem action has no source evidence",
        )
    })?;
    let destination = write_path(request.action, destination_kind(request.action.kind))?;
    let default_workspace_namespace = "workspace";
    let workspace_namespace = request
        .workspace_namespace
        .unwrap_or(default_workspace_namespace);
    let (source_namespace, destination_namespace) = match request.action.kind {
        ActionKind::Quarantine => (workspace_namespace, "quarantine"),
        ActionKind::Restore => ("quarantine", workspace_namespace),
        _ => unreachable!("validated filesystem effect kind"),
    };
    let source_relative = strip_namespace(&source.path, source_namespace)?;
    let destination_relative = strip_namespace(destination, destination_namespace)?;
    let source_display = source.path.clone();
    let destination_display = destination.to_string();
    let source_container = if request.action.kind == ActionKind::Quarantine {
        &source_root
    } else {
        &quarantine_root
    };
    let source_parent = open_relative_parent(source_container, &source_relative, "source")?;
    let source_name = source_relative.file_name().ok_or_else(|| {
        error(
            FilesystemEffectErrorCode::UnsafePath,
            "source path has no file name",
        )
    })?;
    let source_bytes = read_verified(
        &source_parent,
        source_name,
        source.size_bytes,
        &source.sha256,
        "source",
    )?;
    if request.action.kind == ActionKind::Restore {
        validate_restore_record(
            &quarantine_root,
            &source.path,
            destination,
            &source.sha256,
            source.size_bytes,
        )?;
    }

    let mut journal = append_transaction(
        &transaction,
        event(TransactionEventKind::ApplyStarted, None, None, None),
    );
    publish_snapshot(request.state_root, &journal)?;
    journal = append_transaction(
        &journal,
        write_event(
            TransactionEventKind::WriteIntent,
            request.action,
            &destination_display,
            &source.sha256,
            &source.sha256,
        ),
    );
    publish_snapshot(request.state_root, &journal)?;
    if let Err(cause) = check_cancellation(
        request.cancellation,
        false,
        "after filesystem-effect write intent",
    ) {
        let recovery = append_transaction(
            &journal,
            event(TransactionEventKind::RecoveryRequired, None, None, None),
        );
        let _ = publish_snapshot(request.state_root, &recovery);
        return Err(cause);
    }

    let result = move_verified(
        request.action.kind,
        &source_root,
        &quarantine_root,
        &source_relative,
        &destination_relative,
        MoveVerification {
            bytes: &source_bytes,
            sha256: &source.sha256,
            size: source.size_bytes,
        },
    );
    let (source_removed, destination_verified) = match result {
        Ok(value) => value,
        Err(error) => {
            let recovery = append_transaction(
                &journal,
                event(TransactionEventKind::RecoveryRequired, None, None, None),
            );
            let _ = publish_snapshot(request.state_root, &recovery);
            return Err(error);
        }
    };
    if let Err(cause) = check_cancellation(
        request.cancellation,
        true,
        "after filesystem-effect payload move",
    ) {
        return Err(recovery_required_after_effect(
            request.state_root,
            &journal,
            "filesystem-effect cancellation",
            cause,
        ));
    }

    journal = append_transaction(
        &journal,
        write_event(
            TransactionEventKind::WriteVerified,
            request.action,
            &destination_display,
            &source.sha256,
            &source.sha256,
        ),
    );
    if let Err(cause) = publish_snapshot(request.state_root, &journal) {
        return Err(recovery_required_after_effect(
            request.state_root,
            &journal,
            "publish verified filesystem move",
            cause,
        ));
    }

    if request.action.kind == ActionKind::Quarantine {
        let mut record = QuarantineRecord {
            schema_version: 1,
            contract: "quarantine_record".to_string(),
            transaction_id: transaction_id.clone(),
            plan_id: request.plan.plan_id.clone(),
            action_id: request.action.action_id.clone(),
            original_path: source.path.clone(),
            quarantine_path: destination.to_string(),
            sha256: source.sha256.clone(),
            size_bytes: source.size_bytes,
            created_unix_seconds: request.now_unix_seconds,
            binding_sha256: String::new(),
        };
        seal_quarantine_record(&mut record);
        let record_display = record_path(destination).display().to_string();
        journal = append_transaction(
            &journal,
            write_event(
                TransactionEventKind::WriteIntent,
                request.action,
                &record_display,
                &source.sha256,
                &record.binding_sha256,
            ),
        );
        if let Err(cause) = publish_snapshot(request.state_root, &journal) {
            return Err(recovery_required_after_effect(
                request.state_root,
                &journal,
                "publish quarantine-record write intent",
                cause,
            ));
        }
        if let Err(cause) = write_quarantine_record(&quarantine_root, destination, &record) {
            return Err(recovery_required_after_effect(
                request.state_root,
                &journal,
                "publish quarantine record",
                cause,
            ));
        }
        if let Err(cause) = check_cancellation(
            request.cancellation,
            true,
            "after quarantine record publication",
        ) {
            return Err(recovery_required_after_effect(
                request.state_root,
                &journal,
                "filesystem-effect cancellation",
                cause,
            ));
        }
        journal = append_transaction(
            &journal,
            write_event(
                TransactionEventKind::WriteVerified,
                request.action,
                &record_display,
                &source.sha256,
                &record.binding_sha256,
            ),
        );
        if let Err(cause) = publish_snapshot(request.state_root, &journal) {
            return Err(recovery_required_after_effect(
                request.state_root,
                &journal,
                "publish verified quarantine record",
                cause,
            ));
        }
    }

    let commit_pending = append_transaction(
        &journal,
        event(TransactionEventKind::CommitStarted, None, None, None),
    );
    if let Err(cause) = publish_snapshot(request.state_root, &commit_pending) {
        return Err(recovery_required_after_effect(
            request.state_root,
            &commit_pending,
            "publish filesystem-effect commit intent",
            cause,
        ));
    }
    if let Err(cause) = check_cancellation(
        request.cancellation,
        true,
        "after filesystem-effect commit intent",
    ) {
        return Err(recovery_required_after_effect(
            request.state_root,
            &commit_pending,
            "filesystem-effect cancellation",
            cause,
        ));
    }
    let committed = append_transaction(
        &commit_pending,
        event(TransactionEventKind::Committed, None, None, None),
    );
    if let Err(cause) = publish_snapshot(request.state_root, &committed) {
        return Err(recovery_required_after_effect(
            request.state_root,
            &commit_pending,
            "publish committed filesystem-effect journal",
            cause,
        ));
    }
    let mut receipt = FilesystemEffectReceipt {
        schema_version: FILESYSTEM_EFFECT_RECEIPT_SCHEMA_VERSION,
        contract: FILESYSTEM_EFFECT_RECEIPT_CONTRACT.to_string(),
        transaction_id: transaction_id.clone(),
        plan_id: request.plan.plan_id.clone(),
        action_id: request.action.action_id.clone(),
        operation,
        source_path: source_display,
        destination_path: destination_display,
        source_sha256: source.sha256.clone(),
        source_size_bytes: source.size_bytes,
        commit_sequence: committed.events.last().map_or(0, |event| event.sequence),
        commit_event_sha256: committed
            .events
            .last()
            .map_or_else(String::new, |event| event.event_sha256.clone()),
        commit_snapshot_name: committed_snapshot_name(&committed),
        action_plan_sha256: digests.plan_sha256,
        write_set_sha256: digests.write_set_sha256,
        confirmation_challenge_sha256: request.challenge.challenge_sha256.clone(),
        confirmation_response_sha256: confirmation_response_sha256(request.response),
        confirmation_consumption_sha256: request.consumption.binding_sha256.clone(),
        source_removed,
        destination_verified,
        writes_attempted: true,
        product_execution_authorized: true,
        binding_sha256: String::new(),
    };
    seal_filesystem_effect_receipt(&mut receipt);
    if let Err(cause) = write_receipt(&receipts, &receipt) {
        return Err(error(
            FilesystemEffectErrorCode::RecoveryRequired,
            format!(
                "filesystem effect committed without a receipt; manual verification required: {cause}"
            ),
        ));
    }

    Ok(FilesystemEffectReport {
        transaction_id,
        plan_id: request.plan.plan_id.clone(),
        action_id: request.action.action_id.clone(),
        operation,
        status: FilesystemEffectStatus::Committed,
        source_sha256: source.sha256.clone(),
        source_size_bytes: source.size_bytes,
        source_removed,
        destination_verified,
        receipt_reference: format!("receipts/{}.json", request.plan.plan_id),
        writes_attempted: true,
        product_execution_authorized: true,
    })
}

pub fn seal_quarantine_record(record: &mut QuarantineRecord) {
    record.binding_sha256 = quarantine_record_digest(record);
}

pub fn validate_quarantine_record(record: &QuarantineRecord) -> bool {
    record.schema_version == 1
        && record.contract == "quarantine_record"
        && rz0_validation_contract::valid_ledger_id(&record.transaction_id, 96)
        && rz0_validation_contract::valid_dotted_id(&record.plan_id, 100)
        && rz0_validation_contract::valid_ledger_id(&record.action_id, 96)
        && record.original_path.starts_with("workspace/")
        && record.quarantine_path.starts_with("quarantine/")
        && valid_logical_path(&record.original_path)
        && valid_logical_path(&record.quarantine_path)
        && rz0_validation_contract::valid_sha256(&record.sha256)
        && record.size_bytes <= rz0_action_plan::MAX_ACTION_SOURCE_BYTES
        && rz0_validation_contract::valid_sha256(&record.binding_sha256)
        && record.binding_sha256 == quarantine_record_digest(record)
}

pub fn seal_filesystem_effect_receipt(receipt: &mut FilesystemEffectReceipt) {
    receipt.binding_sha256 = filesystem_effect_receipt_digest(receipt);
}

pub fn validate_filesystem_effect_receipt(
    receipt: &FilesystemEffectReceipt,
    committed_journal: &TransactionJournal,
) -> FilesystemEffectValidation {
    let mut errors = Vec::new();
    if receipt.schema_version != FILESYSTEM_EFFECT_RECEIPT_SCHEMA_VERSION {
        errors.push("receipt schema version is unsupported".to_string());
    }
    if receipt.contract != FILESYSTEM_EFFECT_RECEIPT_CONTRACT {
        errors.push("receipt contract is invalid".to_string());
    }
    let journal = validate_transaction_journal(committed_journal);
    if !journal.valid || committed_journal.state != TransactionState::Committed {
        errors.push("receipt requires a valid committed journal".to_string());
    }
    let head = committed_journal.events.last();
    if receipt.transaction_id != committed_journal.transaction_id
        || receipt.operation != committed_journal.operation
        || receipt.commit_sequence != head.map_or(0, |event| event.sequence)
        || receipt.commit_event_sha256 != head.map_or("", |event| event.event_sha256.as_str())
        || receipt.commit_snapshot_name != committed_snapshot_name(committed_journal)
    {
        errors.push("receipt does not bind the committed journal head".to_string());
    }
    if !rz0_validation_contract::valid_dotted_id(&receipt.plan_id, 100)
        || !rz0_validation_contract::valid_ledger_id(&receipt.action_id, 96)
        || !valid_logical_path(&receipt.source_path)
        || !valid_logical_path(&receipt.destination_path)
        || !rz0_validation_contract::valid_sha256(&receipt.source_sha256)
        || receipt.source_size_bytes > rz0_action_plan::MAX_ACTION_SOURCE_BYTES
        || !rz0_validation_contract::valid_sha256(&receipt.action_plan_sha256)
        || !rz0_validation_contract::valid_sha256(&receipt.write_set_sha256)
        || !rz0_validation_contract::valid_sha256(&receipt.confirmation_challenge_sha256)
        || !rz0_validation_contract::valid_sha256(&receipt.confirmation_response_sha256)
        || !rz0_validation_contract::valid_sha256(&receipt.confirmation_consumption_sha256)
        || !rz0_validation_contract::valid_sha256(&receipt.binding_sha256)
    {
        errors.push("receipt contains an invalid bounded field".to_string());
    }
    if !receipt.source_removed
        || !receipt.destination_verified
        || !receipt.writes_attempted
        || !receipt.product_execution_authorized
    {
        errors.push("receipt does not prove the completed filesystem effect".to_string());
    }
    if receipt.binding_sha256 != filesystem_effect_receipt_digest(receipt) {
        errors.push("receipt binding digest is invalid".to_string());
    }
    errors.sort();
    errors.dedup();
    FilesystemEffectValidation {
        valid: errors.is_empty(),
        errors,
    }
}

fn validate_request(request: &FilesystemEffectRequest<'_>) -> Result<(), FilesystemEffectError> {
    let plan_validation = validate_action_plan(request.plan);
    if !plan_validation.valid {
        return Err(error(
            FilesystemEffectErrorCode::InvalidPlan,
            format!("action plan is invalid: {:?}", plan_validation.errors),
        ));
    }
    if request.plan.actions.len() != 1 || request.plan.actions[0] != *request.action {
        return Err(error(
            FilesystemEffectErrorCode::InvalidPlan,
            "filesystem execution requires one exact action",
        ));
    }
    if request.action.disposition != ActionDisposition::Planned
        || !matches!(
            request.action.kind,
            ActionKind::Quarantine | ActionKind::Restore
        )
    {
        return Err(error(
            FilesystemEffectErrorCode::InvalidPlan,
            "only one planned quarantine or restore action may execute",
        ));
    }
    let digests = action_plan_digests(request.plan).map_err(|errors| {
        error(
            FilesystemEffectErrorCode::InvalidPlan,
            format!("action plan digest failed: {errors:?}"),
        )
    })?;
    let expected_capabilities = request.action.capabilities.to_vec();
    if request.challenge.plan_id != request.plan.plan_id
        || request.challenge.plan_sha256 != digests.plan_sha256
        || request.challenge.write_set_sha256 != digests.write_set_sha256
        || request.challenge.action_count != 1
        || request.challenge.capabilities != expected_capabilities
        || request.challenge.risk != ConfirmationRisk::Mutating
        || !request.challenge.quarantine_available
    {
        return Err(error(
            FilesystemEffectErrorCode::InvalidEvidence,
            "confirmation does not bind the exact filesystem action",
        ));
    }
    let confirmation =
        validate_confirmation_consumption(request.consumption, request.challenge, request.response);
    if !confirmation.valid {
        return Err(error(
            FilesystemEffectErrorCode::InvalidEvidence,
            format!(
                "confirmation consumption is invalid: {:?}",
                confirmation.errors
            ),
        ));
    }
    for root in [
        request.state_root,
        request.source_root,
        request.quarantine_root,
    ] {
        if !root.is_absolute() {
            return Err(error(
                FilesystemEffectErrorCode::UnsafeRoot,
                "filesystem effect roots must be absolute",
            ));
        }
    }
    if request.action.kind == ActionKind::Restore && request.action.rollback.quarantine_required {
        return Err(error(
            FilesystemEffectErrorCode::InvalidPlan,
            "restore action cannot require another quarantine",
        ));
    }
    Ok(())
}

fn move_verified(
    kind: ActionKind,
    source_root: &SecureDirectory,
    quarantine_root: &SecureDirectory,
    source_relative: &Path,
    destination_relative: &Path,
    verification: MoveVerification<'_>,
) -> Result<(bool, bool), FilesystemEffectError> {
    let (from_root, to_root) = if kind == ActionKind::Quarantine {
        (source_root, quarantine_root)
    } else {
        (quarantine_root, source_root)
    };
    let from = open_relative_parent(from_root, source_relative, "effect source")?;
    let to = ensure_relative_parent(to_root, destination_relative, "effect destination")?;
    let source_name = source_relative.file_name().ok_or_else(|| {
        error(
            FilesystemEffectErrorCode::UnsafePath,
            "effect source has no file name",
        )
    })?;
    let destination_name = destination_relative.file_name().ok_or_else(|| {
        error(
            FilesystemEffectErrorCode::UnsafePath,
            "effect destination has no file name",
        )
    })?;
    from.publish_child_noreplace(source_name, &to, destination_name)
        .map_err(map_secure("move exact filesystem payload"))?;
    let destination_bytes = read_relative(
        to_root,
        destination_relative,
        verification.size,
        "effect destination",
    )?;
    if destination_bytes != verification.bytes || sha256(&destination_bytes) != verification.sha256
    {
        return Err(error(
            FilesystemEffectErrorCode::RecoveryRequired,
            "destination verification failed after filesystem move",
        ));
    }
    Ok((kind == ActionKind::Quarantine, true))
}

struct MoveVerification<'a> {
    bytes: &'a [u8],
    sha256: &'a str,
    size: u64,
}

fn write_quarantine_record(
    quarantine_root: &SecureDirectory,
    payload_path: &str,
    record: &QuarantineRecord,
) -> Result<(), FilesystemEffectError> {
    let path = record_path(payload_path);
    let relative = strip_namespace(&path.to_string_lossy(), "quarantine")?;
    let parent = ensure_relative_parent(quarantine_root, &relative, "quarantine record")?;
    let name = relative.file_name().ok_or_else(|| {
        error(
            FilesystemEffectErrorCode::UnsafePath,
            "quarantine record has no file name",
        )
    })?;
    let mut bytes = serde_json::to_vec_pretty(record).map_err(|cause| {
        error(
            FilesystemEffectErrorCode::Io,
            format!("serialize quarantine record: {cause}"),
        )
    })?;
    bytes.push(b'\n');
    parent
        .write_new_child(name, &bytes, MAX_RECORD_BYTES)
        .map_err(map_secure("publish quarantine record"))?;
    Ok(())
}

fn validate_restore_record(
    quarantine_root: &SecureDirectory,
    payload_path: &str,
    destination_path: &str,
    expected_sha256: &str,
    expected_size: u64,
) -> Result<(), FilesystemEffectError> {
    let path = record_path(payload_path);
    let relative = strip_namespace(&path.to_string_lossy(), "quarantine")?;
    let bytes = read_relative(
        quarantine_root,
        &relative,
        MAX_RECORD_BYTES,
        "quarantine record",
    )?;
    let record = serde_json::from_slice::<QuarantineRecord>(&bytes).map_err(|cause| {
        error(
            FilesystemEffectErrorCode::InvalidEvidence,
            format!("decode quarantine record: {cause}"),
        )
    })?;
    if !validate_quarantine_record(&record)
        || record.quarantine_path != payload_path
        || record.original_path != destination_path
        || record.sha256 != expected_sha256
        || record.size_bytes != expected_size
    {
        return Err(error(
            FilesystemEffectErrorCode::InvalidEvidence,
            "quarantine record does not bind the exact restore source and destination",
        ));
    }
    Ok(())
}

fn write_receipt(
    receipts: &SecureDirectory,
    receipt: &FilesystemEffectReceipt,
) -> Result<(), FilesystemEffectError> {
    let name = OsString::from(format!("{}.json", receipt.plan_id));
    let mut bytes = serde_json::to_vec_pretty(receipt).map_err(|cause| {
        error(
            FilesystemEffectErrorCode::Io,
            format!("serialize filesystem effect receipt: {cause}"),
        )
    })?;
    bytes.push(b'\n');
    receipts
        .write_new_child(&name, &bytes, MAX_RECORD_BYTES)
        .map_err(map_secure("publish filesystem effect receipt"))?;
    Ok(())
}

fn prepare_transaction(
    state_root: &Path,
    transactions: &SecureDirectory,
    plan_id: &str,
    operation: TransactionOperation,
    transaction_id: &str,
) -> Result<TransactionJournal, FilesystemEffectError> {
    if transactions
        .open_child_directory(OsStr::new(transaction_id))
        .is_ok()
    {
        return Err(error(
            FilesystemEffectErrorCode::Conflict,
            "filesystem effect transaction already exists",
        ));
    }
    let mut prepared = TransactionJournal {
        schema_version: rz0_transaction_contract::TRANSACTION_SCHEMA_VERSION,
        contract: rz0_transaction_contract::TRANSACTION_CONTRACT.to_string(),
        transaction_id: transaction_id.to_string(),
        plan_id: plan_id.to_string(),
        operation,
        state: TransactionState::Prepared,
        durability: DurabilityRequirements::schema_one(),
        events: vec![event(TransactionEventKind::Prepared, None, None, None)],
    };
    seal_transaction_journal(&mut prepared);
    publish_snapshot(state_root, &prepared)?;
    Ok(prepared)
}

fn publish_snapshot(
    state_root: &Path,
    journal: &TransactionJournal,
) -> Result<(), FilesystemEffectError> {
    publish_journal_snapshot(&state_root.join(TRANSACTIONS_DIRECTORY), journal)
        .map(|_| ())
        .map_err(|cause| {
            error(
                FilesystemEffectErrorCode::RecoveryRequired,
                format!("publish filesystem effect journal: {cause}"),
            )
        })
}

fn append_transaction(
    previous: &TransactionJournal,
    event: TransactionEvent,
) -> TransactionJournal {
    let mut next = previous.clone();
    next.events.push(event);
    next.state = state_for(
        next.events
            .last()
            .map_or(TransactionEventKind::Prepared, |event| event.kind),
    );
    seal_transaction_journal(&mut next);
    next
}

fn event(
    kind: TransactionEventKind,
    action_id: Option<String>,
    path: Option<String>,
    before: Option<String>,
) -> TransactionEvent {
    TransactionEvent {
        sequence: 0,
        kind,
        action_id,
        path,
        before_sha256: before,
        after_sha256: None,
        previous_event_sha256: String::new(),
        event_sha256: String::new(),
    }
}

fn write_event(
    kind: TransactionEventKind,
    action: &PlanAction,
    path: &str,
    before: &str,
    after: &str,
) -> TransactionEvent {
    TransactionEvent {
        sequence: 0,
        kind,
        action_id: Some(action.action_id.clone()),
        path: Some(path.to_string()),
        before_sha256: Some(before.to_string()),
        after_sha256: Some(after.to_string()),
        previous_event_sha256: String::new(),
        event_sha256: String::new(),
    }
}

fn state_for(kind: TransactionEventKind) -> TransactionState {
    match kind {
        TransactionEventKind::Prepared => TransactionState::Prepared,
        TransactionEventKind::ApplyStarted
        | TransactionEventKind::WriteIntent
        | TransactionEventKind::WriteVerified
        | TransactionEventKind::RollbackStarted
        | TransactionEventKind::RollbackVerified => TransactionState::Applying,
        TransactionEventKind::CommitStarted => TransactionState::CommitPending,
        TransactionEventKind::Committed => TransactionState::Committed,
        TransactionEventKind::RecoveryRequired => TransactionState::RecoveryRequired,
        TransactionEventKind::RolledBack => TransactionState::RolledBack,
    }
}

fn open_private_root(path: &Path, label: &str) -> Result<SecureDirectory, FilesystemEffectError> {
    let directory =
        SecureDirectory::open(path).map_err(map_secure("open filesystem effect root"))?;
    directory.verify_private().map_err(|cause| {
        error(
            FilesystemEffectErrorCode::UnsafeRoot,
            format!("verify private {label} root: {cause}"),
        )
    })?;
    Ok(directory)
}

fn open_private_child(
    parent: &SecureDirectory,
    name: &str,
    label: &str,
) -> Result<SecureDirectory, FilesystemEffectError> {
    let child = parent
        .open_child_directory(OsStr::new(name))
        .map_err(map_secure("open filesystem effect child"))?;
    child.verify_private().map_err(|cause| {
        error(
            FilesystemEffectErrorCode::UnsafeRoot,
            format!("verify private {label}: {cause}"),
        )
    })?;
    Ok(child)
}

fn lock_root(root: &SecureDirectory) -> Result<SecureFileLock, FilesystemEffectError> {
    let file = root
        .open_or_create_lock_file(OsStr::new(QUARANTINE_LOCK_NAME))
        .map_err(map_secure("open quarantine lock"))?;
    SecureFileLock::try_exclusive(file).map_err(map_secure("acquire quarantine lock"))
}

fn open_relative_parent(
    root: &SecureDirectory,
    relative: &Path,
    label: &str,
) -> Result<SecureDirectory, FilesystemEffectError> {
    let components = normal_components(relative)?;
    let mut current = root
        .try_clone()
        .map_err(map_secure("clone filesystem root handle"))?;
    for component in &components[..components.len() - 1] {
        current = current
            .open_child_directory(component)
            .map_err(map_secure(label))?;
        current
            .verify_private()
            .map_err(map_secure("verify relative filesystem directory"))?;
    }
    Ok(current)
}

fn ensure_relative_parent(
    root: &SecureDirectory,
    relative: &Path,
    label: &str,
) -> Result<SecureDirectory, FilesystemEffectError> {
    let components = normal_components(relative)?;
    let mut current = root
        .try_clone()
        .map_err(map_secure("clone filesystem root handle"))?;
    for component in &components[..components.len() - 1] {
        current = current
            .open_or_create_child_directory(component)
            .map_err(map_secure(label))?;
        current
            .verify_private()
            .map_err(map_secure("verify created filesystem directory"))?;
    }
    Ok(current)
}

fn read_relative(
    root: &SecureDirectory,
    relative: &Path,
    expected_size: u64,
    label: &str,
) -> Result<Vec<u8>, FilesystemEffectError> {
    let components = normal_components(relative)?;
    let parent = open_relative_parent(root, relative, label)?;
    let name = components.last().ok_or_else(|| {
        error(
            FilesystemEffectErrorCode::UnsafePath,
            "relative file path is empty",
        )
    })?;
    parent
        .read_child(name, expected_size)
        .map_err(map_secure(label))
}

fn read_verified(
    parent: &SecureDirectory,
    name: &OsStr,
    expected_size: u64,
    expected_sha256: &str,
    label: &str,
) -> Result<Vec<u8>, FilesystemEffectError> {
    let bytes = parent
        .read_child(name, expected_size)
        .map_err(map_secure(label))?;
    if bytes.len() as u64 != expected_size || sha256(&bytes) != expected_sha256 {
        return Err(error(
            FilesystemEffectErrorCode::SourceChanged,
            format!("{label} changed before the verified move"),
        ));
    }
    Ok(bytes)
}

fn normal_components(path: &Path) -> Result<Vec<OsString>, FilesystemEffectError> {
    if path.is_absolute() {
        return Err(error(
            FilesystemEffectErrorCode::UnsafePath,
            "filesystem effect path must be relative",
        ));
    }
    let components = path
        .components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value.to_os_string()),
            _ => Err(error(
                FilesystemEffectErrorCode::UnsafePath,
                "filesystem effect path contains a non-normal component",
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if components.is_empty() || components.len() > 32 {
        return Err(error(
            FilesystemEffectErrorCode::UnsafePath,
            "filesystem effect path is empty or too deep",
        ));
    }
    Ok(components)
}

fn strip_namespace(value: &str, namespace: &str) -> Result<PathBuf, FilesystemEffectError> {
    let path = Path::new(value);
    let relative = path.strip_prefix(Path::new(namespace)).map_err(|_| {
        error(
            FilesystemEffectErrorCode::UnsafePath,
            format!("path must remain under {namespace}"),
        )
    })?;
    normal_components(relative).map(|components| components.into_iter().collect())
}

fn write_path(action: &PlanAction, kind: WriteKind) -> Result<&str, FilesystemEffectError> {
    let mut matching = action.write_set.iter().filter(|entry| entry.kind == kind);
    let path = matching
        .next()
        .map(|entry| entry.path.as_str())
        .ok_or_else(|| {
            error(
                FilesystemEffectErrorCode::InvalidPlan,
                "filesystem action lacks its required write-set path",
            )
        })?;
    if matching.next().is_some() {
        return Err(error(
            FilesystemEffectErrorCode::InvalidPlan,
            "filesystem action has duplicate write-set kinds",
        ));
    }
    Ok(path)
}

fn destination_kind(kind: ActionKind) -> WriteKind {
    match kind {
        ActionKind::Quarantine => WriteKind::QuarantinedPayload,
        ActionKind::Restore => WriteKind::RestoredPayload,
        _ => unreachable!("validated filesystem effect kind"),
    }
}

fn record_path(payload: &str) -> PathBuf {
    let path = Path::new(payload);
    path.parent()
        .unwrap_or_else(|| Path::new("quarantine"))
        .join("quarantine.json")
}

fn valid_logical_path(value: &str) -> bool {
    let path = Path::new(value);
    !path.is_absolute() && normal_components(path).is_ok()
}

pub fn filesystem_effect_transaction_id(kind: ActionKind, plan_id: &str, now: u64) -> String {
    let operation = match kind {
        ActionKind::Quarantine => "quarantine",
        ActionKind::Restore => "restore",
        _ => "filesystem",
    };
    format!(
        "tx.{operation}.{}.{}",
        short_digest(plan_id.as_bytes()),
        now
    )
}

fn short_digest(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    format!("{:x}", digest)[..16].to_string()
}

fn committed_snapshot_name(journal: &TransactionJournal) -> String {
    journal.events.last().map_or_else(String::new, |event| {
        format!("{:04}-{}.json", event.sequence, event.event_sha256)
    })
}

/// Converts any publication failure after the payload has moved into an
/// explicit recovery state. The recovery snapshot is best effort: if the
/// state root is itself unavailable, the returned error still remains
/// `RecoveryRequired` and explains that the durable marker could not be
/// published.
fn recovery_required_after_effect(
    state_root: &Path,
    journal: &TransactionJournal,
    operation: &str,
    cause: impl fmt::Display,
) -> FilesystemEffectError {
    let recovery = append_transaction(
        journal,
        event(TransactionEventKind::RecoveryRequired, None, None, None),
    );
    let recovery_result = publish_snapshot(state_root, &recovery);
    let detail = match recovery_result {
        Ok(()) => format!(
            "{operation} failed after a filesystem move; recovery marker published: {cause}"
        ),
        Err(recovery_cause) => format!(
            "{operation} failed after a filesystem move and recovery marker publication also failed: {cause}; {recovery_cause}"
        ),
    };
    error(FilesystemEffectErrorCode::RecoveryRequired, detail)
}

fn check_cancellation(
    cancellation: Option<&CancellationToken>,
    partial: bool,
    boundary: &str,
) -> Result<(), FilesystemEffectError> {
    let Some(reason) = cancellation.and_then(CancellationToken::reason) else {
        return Ok(());
    };
    Err(error(
        if partial {
            FilesystemEffectErrorCode::RecoveryRequired
        } else {
            FilesystemEffectErrorCode::Cancelled
        },
        format!("filesystem effect cancelled {boundary}: {reason:?}"),
    ))
}

fn quarantine_record_digest(record: &QuarantineRecord) -> String {
    let mut digest = Sha256::new();
    digest.update(b"runtime.zero.quarantine-record.v1\0");
    put(&mut digest, &record.transaction_id);
    put(&mut digest, &record.plan_id);
    put(&mut digest, &record.action_id);
    put(&mut digest, &record.original_path);
    put(&mut digest, &record.quarantine_path);
    put(&mut digest, &record.sha256);
    digest.update(record.size_bytes.to_be_bytes());
    digest.update(record.created_unix_seconds.to_be_bytes());
    format!("{:x}", digest.finalize())
}

fn filesystem_effect_receipt_digest(receipt: &FilesystemEffectReceipt) -> String {
    let mut digest = Sha256::new();
    digest.update(b"runtime.zero.filesystem-effect-receipt.v1\0");
    put(&mut digest, &receipt.transaction_id);
    put(&mut digest, &receipt.plan_id);
    put(&mut digest, &receipt.action_id);
    put(&mut digest, operation_name(receipt.operation));
    put(&mut digest, &receipt.source_path);
    put(&mut digest, &receipt.destination_path);
    put(&mut digest, &receipt.source_sha256);
    digest.update(receipt.source_size_bytes.to_be_bytes());
    digest.update(receipt.commit_sequence.to_be_bytes());
    put(&mut digest, &receipt.commit_event_sha256);
    put(&mut digest, &receipt.commit_snapshot_name);
    put(&mut digest, &receipt.action_plan_sha256);
    put(&mut digest, &receipt.write_set_sha256);
    put(&mut digest, &receipt.confirmation_challenge_sha256);
    put(&mut digest, &receipt.confirmation_response_sha256);
    put(&mut digest, &receipt.confirmation_consumption_sha256);
    for value in [
        receipt.source_removed,
        receipt.destination_verified,
        receipt.writes_attempted,
        receipt.product_execution_authorized,
    ] {
        digest.update([u8::from(value)]);
    }
    format!("{:x}", digest.finalize())
}

fn put(digest: &mut Sha256, value: &str) {
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value.as_bytes());
}

fn operation_for(kind: ActionKind) -> TransactionOperation {
    match kind {
        ActionKind::Quarantine => TransactionOperation::Quarantine,
        ActionKind::Restore => TransactionOperation::Restore,
        _ => unreachable!("validated filesystem effect kind"),
    }
}

fn operation_name(operation: TransactionOperation) -> &'static str {
    match operation {
        TransactionOperation::Quarantine => "quarantine",
        TransactionOperation::Restore => "restore",
        _ => "unsupported",
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn error(code: FilesystemEffectErrorCode, detail: impl Into<String>) -> FilesystemEffectError {
    FilesystemEffectError::new(code, detail)
}

fn map_secure(context: impl Into<String>) -> impl FnOnce(SecureFsError) -> FilesystemEffectError {
    let context = context.into();
    move |filesystem_error| {
        let code = match filesystem_error.code {
            SecureFsErrorCode::AlreadyExists | SecureFsErrorCode::LockBusy => {
                FilesystemEffectErrorCode::Conflict
            }
            SecureFsErrorCode::UnsafeName | SecureFsErrorCode::UnsafeDirectory => {
                FilesystemEffectErrorCode::UnsafeRoot
            }
            SecureFsErrorCode::IdentityChanged => FilesystemEffectErrorCode::SourceChanged,
            SecureFsErrorCode::UnsupportedOperation => FilesystemEffectErrorCode::Unsupported,
            SecureFsErrorCode::LimitExceeded => FilesystemEffectErrorCode::LimitExceeded,
            SecureFsErrorCode::PublicationIncomplete => FilesystemEffectErrorCode::RecoveryRequired,
            SecureFsErrorCode::NotFound | SecureFsErrorCode::Io => FilesystemEffectErrorCode::Io,
        };
        error(code, format!("{context}: {filesystem_error}"))
    }
}
