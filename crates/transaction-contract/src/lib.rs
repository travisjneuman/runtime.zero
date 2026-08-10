use std::collections::BTreeSet;

mod coordinator;
mod durable;
mod external_effect;
mod receipt;

pub use coordinator::*;
pub use durable::*;
pub use external_effect::*;
pub use receipt::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const TRANSACTION_SCHEMA_VERSION: u16 = 1;
pub const TRANSACTION_CONTRACT: &str = "transaction_journal";
pub const MAX_EVENTS: usize = 1024;
pub const ZERO_DIGEST: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionJournal {
    pub schema_version: u16,
    pub contract: String,
    pub transaction_id: String,
    pub plan_id: String,
    pub operation: TransactionOperation,
    pub state: TransactionState,
    pub durability: DurabilityRequirements,
    pub events: Vec<TransactionEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionOperation {
    ModuleInstall,
    ModuleUpgrade,
    ModuleRepair,
    ModuleUninstall,
    Update,
    Uninstall,
    Quarantine,
    Restore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionState {
    Prepared,
    Applying,
    CommitPending,
    Committed,
    RollingBack,
    RolledBack,
    RecoveryRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DurabilityRequirements {
    pub append_only_events: bool,
    pub sync_each_event: bool,
    pub atomic_head_publication: bool,
    pub receipt_head_binding_required: bool,
}

impl DurabilityRequirements {
    pub const fn schema_one() -> Self {
        Self {
            append_only_events: true,
            sync_each_event: true,
            atomic_head_publication: true,
            receipt_head_binding_required: true,
        }
    }

    fn is_schema_one(&self) -> bool {
        *self == Self::schema_one()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionEvent {
    pub sequence: u32,
    pub kind: TransactionEventKind,
    pub action_id: Option<String>,
    pub path: Option<String>,
    pub before_sha256: Option<String>,
    pub after_sha256: Option<String>,
    pub previous_event_sha256: String,
    pub event_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionEventKind {
    Prepared,
    ApplyStarted,
    WriteIntent,
    WriteVerified,
    CommitStarted,
    Committed,
    RecoveryRequired,
    RollbackStarted,
    RollbackVerified,
    RolledBack,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionValidation {
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryDecision {
    AbortWithoutWrites,
    RollBackVerifiedWrites,
    VerifyCommittedState,
    ResumeRollback,
    NoAction,
    RefuseInvalidJournal,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryAssessment {
    pub transaction_id: String,
    pub decision: RecoveryDecision,
    pub last_verified_sequence: u32,
    pub verified_write_count: u32,
    pub automatic_mutation_authorized: bool,
}

pub fn seal_transaction_journal(journal: &mut TransactionJournal) {
    let mut previous = ZERO_DIGEST.to_string();
    for (index, event) in journal.events.iter_mut().enumerate() {
        event.sequence = index as u32 + 1;
        event.previous_event_sha256.clone_from(&previous);
        event.event_sha256 = event_digest(
            &journal.transaction_id,
            &journal.plan_id,
            journal.operation,
            event,
        );
        previous.clone_from(&event.event_sha256);
    }
}

pub fn validate_transaction_journal(journal: &TransactionJournal) -> TransactionValidation {
    let mut errors = Vec::new();
    if journal.schema_version != TRANSACTION_SCHEMA_VERSION {
        errors.push(format!(
            "schema_version must be {TRANSACTION_SCHEMA_VERSION}"
        ));
    }
    if journal.contract != TRANSACTION_CONTRACT {
        errors.push(format!("contract must be {TRANSACTION_CONTRACT}"));
    }
    validate_id(&journal.transaction_id, "transaction_id", &mut errors);
    validate_id(&journal.plan_id, "plan_id", &mut errors);
    if !journal.durability.is_schema_one() {
        errors.push("schema-1 durability requirements must all be enabled".to_string());
    }
    if journal.events.is_empty() || journal.events.len() > MAX_EVENTS {
        errors.push(format!("journal must contain 1..={MAX_EVENTS} events"));
    }

    let mut hashes = BTreeSet::new();
    let mut previous_kind = None;
    let mut expected_previous = ZERO_DIGEST;
    for (index, event) in journal.events.iter().take(MAX_EVENTS + 1).enumerate() {
        if event.sequence != index as u32 + 1 {
            errors.push("event sequences must be contiguous and one-based".to_string());
        }
        if event.previous_event_sha256 != expected_previous {
            errors.push("event hash chain is discontinuous".to_string());
        }
        if !valid_hash(&event.event_sha256)
            || event.event_sha256
                != event_digest(
                    &journal.transaction_id,
                    &journal.plan_id,
                    journal.operation,
                    event,
                )
        {
            errors.push("event digest is invalid".to_string());
        }
        if !hashes.insert(event.event_sha256.as_str()) {
            errors.push("event digests must be unique".to_string());
        }
        validate_event_shape(
            event,
            previous_kind,
            journal.events.get(index.wrapping_sub(1)),
            &mut errors,
        );
        if let Some(previous) = previous_kind
            && !valid_transition(previous, event.kind)
        {
            errors.push(format!(
                "invalid transaction transition from {} to {}",
                event_kind_name(previous),
                event_kind_name(event.kind)
            ));
        }
        previous_kind = Some(event.kind);
        expected_previous = &event.event_sha256;
    }
    if journal.events.first().map(|event| event.kind) != Some(TransactionEventKind::Prepared) {
        errors.push("the first event must be prepared".to_string());
    }
    if journal.events.last().map(|event| state_for(event.kind)) != Some(journal.state) {
        errors.push("journal state does not match the last event".to_string());
    }

    errors.sort();
    errors.dedup();
    TransactionValidation {
        valid: errors.is_empty(),
        errors,
    }
}

pub fn assess_recovery(journal: &TransactionJournal) -> RecoveryAssessment {
    let validation = validate_transaction_journal(journal);
    if !validation.valid {
        return RecoveryAssessment {
            transaction_id: journal.transaction_id.clone(),
            decision: RecoveryDecision::RefuseInvalidJournal,
            last_verified_sequence: 0,
            verified_write_count: 0,
            automatic_mutation_authorized: false,
        };
    }
    let decision = match journal.state {
        TransactionState::Prepared => RecoveryDecision::AbortWithoutWrites,
        TransactionState::Applying
        | TransactionState::CommitPending
        | TransactionState::RecoveryRequired => RecoveryDecision::RollBackVerifiedWrites,
        TransactionState::Committed => RecoveryDecision::VerifyCommittedState,
        TransactionState::RollingBack => RecoveryDecision::ResumeRollback,
        TransactionState::RolledBack => RecoveryDecision::NoAction,
    };
    RecoveryAssessment {
        transaction_id: journal.transaction_id.clone(),
        decision,
        last_verified_sequence: journal.events.last().map_or(0, |event| event.sequence),
        verified_write_count: journal
            .events
            .iter()
            .filter(|event| event.kind == TransactionEventKind::WriteVerified)
            .count() as u32,
        automatic_mutation_authorized: false,
    }
}

fn validate_event_shape(
    event: &TransactionEvent,
    previous_kind: Option<TransactionEventKind>,
    previous_event: Option<&TransactionEvent>,
    errors: &mut Vec<String>,
) {
    if let Some(action_id) = event.action_id.as_deref() {
        validate_id(action_id, "event.action_id", errors);
    }
    if let Some(path) = event.path.as_deref()
        && !valid_relative_path(path)
    {
        errors.push("event path must be normalized and relative".to_string());
    }
    for digest in [
        event.before_sha256.as_deref(),
        event.after_sha256.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        if !valid_hash(digest) {
            errors.push("event evidence digest is invalid".to_string());
        }
    }
    let has_write_shape = event.action_id.is_some() && event.path.is_some();
    match event.kind {
        TransactionEventKind::WriteIntent => {
            if !has_write_shape || event.after_sha256.is_none() {
                errors.push(
                    "write_intent requires action, path, and expected after digest".to_string(),
                );
            }
        }
        TransactionEventKind::WriteVerified => {
            if !has_write_shape || event.after_sha256.is_none() {
                errors.push("write_verified requires action, path, and after digest".to_string());
            }
            if previous_kind != Some(TransactionEventKind::WriteIntent)
                || previous_event.is_none_or(|previous| {
                    previous.action_id != event.action_id
                        || previous.path != event.path
                        || previous.before_sha256 != event.before_sha256
                        || previous.after_sha256 != event.after_sha256
                })
            {
                errors.push(
                    "write_verified must exactly match the preceding write_intent".to_string(),
                );
            }
        }
        TransactionEventKind::RollbackVerified => {
            if !has_write_shape || event.after_sha256.is_some() {
                errors.push(
                    "rollback_verified requires action/path and cannot contain an after digest"
                        .to_string(),
                );
            }
        }
        _ => {
            if event.path.is_some() || event.before_sha256.is_some() || event.after_sha256.is_some()
            {
                errors.push("non-write events cannot contain path or digest evidence".to_string());
            }
        }
    }
}

fn valid_transition(previous: TransactionEventKind, next: TransactionEventKind) -> bool {
    use TransactionEventKind::*;
    matches!(
        (previous, next),
        (Prepared, ApplyStarted | RollbackStarted)
            | (
                ApplyStarted,
                WriteIntent | CommitStarted | RecoveryRequired | RollbackStarted
            )
            | (
                WriteIntent,
                WriteVerified | RecoveryRequired | RollbackStarted
            )
            | (
                WriteVerified,
                WriteIntent | CommitStarted | RecoveryRequired | RollbackStarted
            )
            | (
                CommitStarted,
                Committed | RecoveryRequired | RollbackStarted
            )
            | (RecoveryRequired, RollbackStarted)
            | (
                RollbackStarted,
                RollbackVerified | RolledBack | RecoveryRequired
            )
            | (
                RollbackVerified,
                RollbackVerified | RolledBack | RecoveryRequired
            )
    )
}

fn state_for(kind: TransactionEventKind) -> TransactionState {
    match kind {
        TransactionEventKind::Prepared => TransactionState::Prepared,
        TransactionEventKind::ApplyStarted
        | TransactionEventKind::WriteIntent
        | TransactionEventKind::WriteVerified => TransactionState::Applying,
        TransactionEventKind::CommitStarted => TransactionState::CommitPending,
        TransactionEventKind::Committed => TransactionState::Committed,
        TransactionEventKind::RecoveryRequired => TransactionState::RecoveryRequired,
        TransactionEventKind::RollbackStarted | TransactionEventKind::RollbackVerified => {
            TransactionState::RollingBack
        }
        TransactionEventKind::RolledBack => TransactionState::RolledBack,
    }
}

fn event_digest(
    transaction_id: &str,
    plan_id: &str,
    operation: TransactionOperation,
    event: &TransactionEvent,
) -> String {
    let mut digest = Sha256::new();
    digest.update(b"runtime.zero.transaction-event.v1\0");
    put(&mut digest, transaction_id);
    put(&mut digest, plan_id);
    put(&mut digest, operation_name(operation));
    digest.update(event.sequence.to_be_bytes());
    put(&mut digest, event_kind_name(event.kind));
    put_optional(&mut digest, event.action_id.as_deref());
    put_optional(&mut digest, event.path.as_deref());
    put_optional(&mut digest, event.before_sha256.as_deref());
    put_optional(&mut digest, event.after_sha256.as_deref());
    put(&mut digest, &event.previous_event_sha256);
    format!("{:x}", digest.finalize())
}

fn put(digest: &mut Sha256, value: &str) {
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value.as_bytes());
}

fn put_optional(digest: &mut Sha256, value: Option<&str>) {
    digest.update([u8::from(value.is_some())]);
    if let Some(value) = value {
        put(digest, value);
    }
}

fn operation_name(operation: TransactionOperation) -> &'static str {
    match operation {
        TransactionOperation::ModuleInstall => "module_install",
        TransactionOperation::ModuleUpgrade => "module_upgrade",
        TransactionOperation::ModuleRepair => "module_repair",
        TransactionOperation::ModuleUninstall => "module_uninstall",
        TransactionOperation::Update => "update",
        TransactionOperation::Uninstall => "uninstall",
        TransactionOperation::Quarantine => "quarantine",
        TransactionOperation::Restore => "restore",
    }
}

fn event_kind_name(kind: TransactionEventKind) -> &'static str {
    match kind {
        TransactionEventKind::Prepared => "prepared",
        TransactionEventKind::ApplyStarted => "apply_started",
        TransactionEventKind::WriteIntent => "write_intent",
        TransactionEventKind::WriteVerified => "write_verified",
        TransactionEventKind::CommitStarted => "commit_started",
        TransactionEventKind::Committed => "committed",
        TransactionEventKind::RecoveryRequired => "recovery_required",
        TransactionEventKind::RollbackStarted => "rollback_started",
        TransactionEventKind::RollbackVerified => "rollback_verified",
        TransactionEventKind::RolledBack => "rolled_back",
    }
}

fn validate_id(value: &str, field: &str, errors: &mut Vec<String>) {
    if !rz0_validation_contract::valid_ledger_id(value, 96) {
        errors.push(format!("{field} is invalid"));
    }
}

fn valid_hash(value: &str) -> bool {
    rz0_validation_contract::valid_sha256(value)
}

fn valid_relative_path(value: &str) -> bool {
    !value.starts_with('.') && rz0_validation_contract::valid_contract_relative_path(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BEFORE: &str = "1111111111111111111111111111111111111111111111111111111111111111";
    const AFTER: &str = "2222222222222222222222222222222222222222222222222222222222222222";

    #[test]
    fn validates_a_chained_committed_journal() {
        let journal = committed_journal();
        let validation = validate_transaction_journal(&journal);
        assert!(validation.valid, "{:?}", validation.errors);
        let recovery = assess_recovery(&journal);
        assert_eq!(recovery.decision, RecoveryDecision::VerifyCommittedState);
        assert_eq!(recovery.last_verified_sequence, 6);
        assert_eq!(recovery.verified_write_count, 1);
        assert!(!recovery.automatic_mutation_authorized);
    }

    #[test]
    fn tampering_breaks_the_digest_chain_and_refuses_recovery() {
        let mut journal = committed_journal();
        journal.events[2].path = Some("modules/replaced.bin".to_string());
        let validation = validate_transaction_journal(&journal);
        assert!(!validation.valid);
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("digest"))
        );
        assert_eq!(
            assess_recovery(&journal).decision,
            RecoveryDecision::RefuseInvalidJournal
        );
    }

    #[test]
    fn journal_identity_and_operation_are_bound_into_every_event() {
        let mut journal = committed_journal();
        journal.plan_id = "rz0plan-other".to_string();
        journal.operation = TransactionOperation::ModuleUpgrade;
        let validation = validate_transaction_journal(&journal);
        assert!(!validation.valid);
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("digest"))
        );
    }

    #[test]
    fn write_verification_must_match_the_immediately_preceding_intent() {
        let mut journal = committed_journal();
        journal.events[3].after_sha256 = Some(BEFORE.to_string());
        seal_transaction_journal(&mut journal);
        let validation = validate_transaction_journal(&journal);
        assert!(!validation.valid);
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("exactly match"))
        );
    }

    #[test]
    fn state_and_transition_drift_fail_closed() {
        let mut journal = committed_journal();
        journal.state = TransactionState::Applying;
        assert!(!validate_transaction_journal(&journal).valid);

        journal.state = TransactionState::Committed;
        journal.events[4].kind = TransactionEventKind::ApplyStarted;
        seal_transaction_journal(&mut journal);
        let validation = validate_transaction_journal(&journal);
        assert!(!validation.valid);
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("transition"))
        );
    }

    #[test]
    fn recovery_decisions_never_authorize_automatic_mutation() {
        let scenarios = [
            (
                vec![event(TransactionEventKind::Prepared)],
                TransactionState::Prepared,
                RecoveryDecision::AbortWithoutWrites,
            ),
            (
                vec![
                    event(TransactionEventKind::Prepared),
                    event(TransactionEventKind::ApplyStarted),
                ],
                TransactionState::Applying,
                RecoveryDecision::RollBackVerifiedWrites,
            ),
            (
                vec![
                    event(TransactionEventKind::Prepared),
                    event(TransactionEventKind::RollbackStarted),
                ],
                TransactionState::RollingBack,
                RecoveryDecision::ResumeRollback,
            ),
            (
                vec![
                    event(TransactionEventKind::Prepared),
                    event(TransactionEventKind::RollbackStarted),
                    event(TransactionEventKind::RolledBack),
                ],
                TransactionState::RolledBack,
                RecoveryDecision::NoAction,
            ),
        ];
        for (events, state, decision) in scenarios {
            let journal = journal(events, state);
            let assessment = assess_recovery(&journal);
            assert_eq!(assessment.decision, decision);
            assert!(!assessment.automatic_mutation_authorized);
        }
    }

    #[test]
    fn unknown_fields_fail_deserialization() {
        let value = serde_json::to_string(&committed_journal()).expect("serialize journal");
        let drifted = value.replacen(
            "\"schema_version\":1",
            "\"schema_version\":1,\"unexpected\":true",
            1,
        );
        assert!(serde_json::from_str::<TransactionJournal>(&drifted).is_err());
    }

    fn committed_journal() -> TransactionJournal {
        let mut intent = event(TransactionEventKind::WriteIntent);
        intent.action_id = Some("install-manifest".to_string());
        intent.path = Some("modules/first-party.inventory/0.1.0/rz0-module.json".to_string());
        intent.before_sha256 = Some(BEFORE.to_string());
        intent.after_sha256 = Some(AFTER.to_string());
        let mut verified = intent.clone();
        verified.kind = TransactionEventKind::WriteVerified;
        let events = vec![
            event(TransactionEventKind::Prepared),
            event(TransactionEventKind::ApplyStarted),
            intent,
            verified,
            event(TransactionEventKind::CommitStarted),
            event(TransactionEventKind::Committed),
        ];
        journal(events, TransactionState::Committed)
    }

    fn journal(events: Vec<TransactionEvent>, state: TransactionState) -> TransactionJournal {
        let mut journal = TransactionJournal {
            schema_version: TRANSACTION_SCHEMA_VERSION,
            contract: TRANSACTION_CONTRACT.to_string(),
            transaction_id: "rz0tx-example".to_string(),
            plan_id: "rz0plan-example".to_string(),
            operation: TransactionOperation::ModuleInstall,
            state,
            durability: DurabilityRequirements::schema_one(),
            events,
        };
        seal_transaction_journal(&mut journal);
        journal
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
}
