use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    TransactionJournal, TransactionOperation, TransactionState, validate_transaction_journal,
};

pub const COMMIT_RECEIPT_SCHEMA_VERSION: u16 = 1;
pub const COMMIT_RECEIPT_CONTRACT: &str = "transaction_commit_receipt";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionCommitReceipt {
    pub schema_version: u16,
    pub contract: String,
    pub transaction_id: String,
    pub plan_id: String,
    pub operation: TransactionOperation,
    pub committed_event_sequence: u32,
    pub committed_event_sha256: String,
    pub journal_snapshot_name: String,
    pub action_plan_sha256: String,
    pub write_set_sha256: String,
    pub confirmation_challenge_sha256: String,
    pub confirmation_response_sha256: String,
    pub confirmation_consumption_sha256: String,
    pub confirmation_consumed: bool,
    pub registry_before_sha256: Option<String>,
    pub registry_after_sha256: String,
    pub publication: CommitPublicationRequirements,
    pub binding_sha256: String,
    pub automatic_mutation_authorized: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommitPublicationRequirements {
    pub committed_journal_synced_first: bool,
    pub receipt_synced_second: bool,
    pub registry_published_last: bool,
    pub registry_atomic_replace_required: bool,
}

impl CommitPublicationRequirements {
    pub const fn schema_one() -> Self {
        Self {
            committed_journal_synced_first: true,
            receipt_synced_second: true,
            registry_published_last: true,
            registry_atomic_replace_required: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitReceiptValidation {
    pub valid: bool,
    pub errors: Vec<String>,
}

pub fn seal_commit_receipt(receipt: &mut TransactionCommitReceipt) {
    receipt.binding_sha256 = commit_receipt_digest(receipt);
}

pub fn validate_commit_receipt(
    receipt: &TransactionCommitReceipt,
    journal: &TransactionJournal,
) -> CommitReceiptValidation {
    let mut errors = Vec::new();
    if receipt.schema_version != COMMIT_RECEIPT_SCHEMA_VERSION {
        errors.push(format!(
            "receipt schema_version must be {COMMIT_RECEIPT_SCHEMA_VERSION}"
        ));
    }
    if receipt.contract != COMMIT_RECEIPT_CONTRACT {
        errors.push(format!(
            "receipt contract must be {COMMIT_RECEIPT_CONTRACT}"
        ));
    }
    let journal_validation = validate_transaction_journal(journal);
    if !journal_validation.valid || journal.state != TransactionState::Committed {
        errors.push("receipt requires a valid committed transaction journal".to_string());
    }
    let head = journal.events.last();
    if receipt.transaction_id != journal.transaction_id
        || receipt.plan_id != journal.plan_id
        || receipt.operation != journal.operation
        || head.is_none_or(|event| {
            receipt.committed_event_sequence != event.sequence
                || receipt.committed_event_sha256 != event.event_sha256
                || receipt.journal_snapshot_name
                    != snapshot_name(event.sequence, &event.event_sha256)
        })
    {
        errors.push("receipt identity does not bind the exact committed journal head".to_string());
    }
    for (name, value) in [
        ("action_plan_sha256", Some(&receipt.action_plan_sha256)),
        ("write_set_sha256", Some(&receipt.write_set_sha256)),
        (
            "confirmation_challenge_sha256",
            Some(&receipt.confirmation_challenge_sha256),
        ),
        (
            "confirmation_response_sha256",
            Some(&receipt.confirmation_response_sha256),
        ),
        (
            "confirmation_consumption_sha256",
            Some(&receipt.confirmation_consumption_sha256),
        ),
        (
            "registry_before_sha256",
            receipt.registry_before_sha256.as_ref(),
        ),
        (
            "registry_after_sha256",
            Some(&receipt.registry_after_sha256),
        ),
        ("binding_sha256", Some(&receipt.binding_sha256)),
    ] {
        if value.is_some_and(|value| !rz0_validation_contract::valid_sha256(value)) {
            errors.push(format!("receipt {name} is not canonical SHA-256"));
        }
    }
    if !receipt.confirmation_consumed {
        errors.push("receipt requires durable single-use confirmation consumption".to_string());
    }
    if receipt.publication != CommitPublicationRequirements::schema_one() {
        errors.push("receipt publication ordering requirements must all be enabled".to_string());
    }
    if receipt.automatic_mutation_authorized {
        errors.push("receipt cannot authorize automatic mutation".to_string());
    }
    if receipt.binding_sha256 != commit_receipt_digest(receipt) {
        errors.push("receipt binding digest is invalid".to_string());
    }
    errors.sort();
    errors.dedup();
    CommitReceiptValidation {
        valid: errors.is_empty(),
        errors,
    }
}

fn commit_receipt_digest(receipt: &TransactionCommitReceipt) -> String {
    let mut digest = Sha256::new();
    digest.update(b"runtime.zero.transaction-commit-receipt.v1\0");
    put(&mut digest, &receipt.transaction_id);
    put(&mut digest, &receipt.plan_id);
    put(&mut digest, operation_name(receipt.operation));
    digest.update(receipt.committed_event_sequence.to_be_bytes());
    put(&mut digest, &receipt.committed_event_sha256);
    put(&mut digest, &receipt.journal_snapshot_name);
    put(&mut digest, &receipt.action_plan_sha256);
    put(&mut digest, &receipt.write_set_sha256);
    put(&mut digest, &receipt.confirmation_challenge_sha256);
    put(&mut digest, &receipt.confirmation_response_sha256);
    put(&mut digest, &receipt.confirmation_consumption_sha256);
    digest.update([u8::from(receipt.confirmation_consumed)]);
    put_optional(&mut digest, receipt.registry_before_sha256.as_deref());
    put(&mut digest, &receipt.registry_after_sha256);
    for enabled in [
        receipt.publication.committed_journal_synced_first,
        receipt.publication.receipt_synced_second,
        receipt.publication.registry_published_last,
        receipt.publication.registry_atomic_replace_required,
        receipt.automatic_mutation_authorized,
    ] {
        digest.update([u8::from(enabled)]);
    }
    format!("{:x}", digest.finalize())
}

fn snapshot_name(sequence: u32, event_sha256: &str) -> String {
    format!("{sequence:04}-{event_sha256}.json")
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
