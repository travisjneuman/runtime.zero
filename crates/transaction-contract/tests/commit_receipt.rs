use rz0_transaction_contract::{
    COMMIT_RECEIPT_CONTRACT, COMMIT_RECEIPT_SCHEMA_VERSION, CommitPublicationRequirements,
    DurabilityRequirements, TRANSACTION_CONTRACT, TRANSACTION_SCHEMA_VERSION,
    TransactionCommitReceipt, TransactionEvent, TransactionEventKind, TransactionJournal,
    TransactionOperation, TransactionState, seal_commit_receipt, seal_transaction_journal,
    validate_commit_receipt,
};

const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

#[test]
fn exact_committed_head_plan_write_set_and_registry_are_bound() {
    let journal = committed_journal();
    let receipt = receipt(&journal);
    let validation = validate_commit_receipt(&receipt, &journal);
    assert!(validation.valid, "{:?}", validation.errors);
    assert!(!receipt.automatic_mutation_authorized);
}

#[test]
fn journal_identity_or_head_drift_invalidates_the_receipt() {
    let journal = committed_journal();
    let receipt = receipt(&journal);
    let mut drifted = journal.clone();
    drifted.plan_id = "rz0plan-other".to_string();
    seal_transaction_journal(&mut drifted);
    let validation = validate_commit_receipt(&receipt, &drifted);
    assert!(!validation.valid);
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("exact"))
    );
}

#[test]
fn registry_plan_and_write_set_tampering_breaks_the_binding_digest() {
    let journal = committed_journal();
    for mutate in [
        |receipt: &mut TransactionCommitReceipt| receipt.action_plan_sha256 = DIGEST_B.to_string(),
        |receipt: &mut TransactionCommitReceipt| receipt.write_set_sha256 = DIGEST_C.to_string(),
        |receipt: &mut TransactionCommitReceipt| {
            receipt.registry_after_sha256 = DIGEST_A.to_string()
        },
    ] {
        let mut receipt = receipt(&journal);
        mutate(&mut receipt);
        let validation = validate_commit_receipt(&receipt, &journal);
        assert!(!validation.valid);
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains("binding"))
        );
    }
}

#[test]
fn ordering_claims_and_automatic_mutation_fail_closed() {
    let journal = committed_journal();
    let mut receipt = receipt(&journal);
    receipt.publication.registry_published_last = false;
    receipt.automatic_mutation_authorized = true;
    seal_commit_receipt(&mut receipt);
    let validation = validate_commit_receipt(&receipt, &journal);
    assert!(!validation.valid);
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("ordering"))
    );
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("authorize"))
    );
}

#[test]
fn unknown_receipt_fields_fail_deserialization() {
    let journal = committed_journal();
    let json = serde_json::to_string(&receipt(&journal)).expect("serialize receipt");
    let drifted = json.replacen(
        "\"schema_version\":1",
        "\"schema_version\":1,\"unexpected\":true",
        1,
    );
    assert!(serde_json::from_str::<TransactionCommitReceipt>(&drifted).is_err());
}

fn receipt(journal: &TransactionJournal) -> TransactionCommitReceipt {
    let head = journal.events.last().expect("head");
    let mut receipt = TransactionCommitReceipt {
        schema_version: COMMIT_RECEIPT_SCHEMA_VERSION,
        contract: COMMIT_RECEIPT_CONTRACT.to_string(),
        transaction_id: journal.transaction_id.clone(),
        plan_id: journal.plan_id.clone(),
        operation: journal.operation,
        committed_event_sequence: head.sequence,
        committed_event_sha256: head.event_sha256.clone(),
        journal_snapshot_name: format!("{:04}-{}.json", head.sequence, head.event_sha256),
        action_plan_sha256: DIGEST_A.to_string(),
        write_set_sha256: DIGEST_B.to_string(),
        registry_before_sha256: None,
        registry_after_sha256: DIGEST_C.to_string(),
        publication: CommitPublicationRequirements::schema_one(),
        binding_sha256: String::new(),
        automatic_mutation_authorized: false,
    };
    seal_commit_receipt(&mut receipt);
    receipt
}

fn committed_journal() -> TransactionJournal {
    let mut journal = TransactionJournal {
        schema_version: TRANSACTION_SCHEMA_VERSION,
        contract: TRANSACTION_CONTRACT.to_string(),
        transaction_id: "rz0tx-receipt".to_string(),
        plan_id: "rz0plan-receipt".to_string(),
        operation: TransactionOperation::ModuleInstall,
        state: TransactionState::Committed,
        durability: DurabilityRequirements::schema_one(),
        events: vec![
            event(TransactionEventKind::Prepared),
            event(TransactionEventKind::ApplyStarted),
            event(TransactionEventKind::CommitStarted),
            event(TransactionEventKind::Committed),
        ],
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
