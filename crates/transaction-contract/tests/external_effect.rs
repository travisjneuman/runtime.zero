#![cfg(unix)]

use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use rz0_action_plan::{ActionPlan, action_plan_digests};
use rz0_cancellation_contract::{CancellationReason, cancellation_pair};
use rz0_confirmation_contract::{
    CONFIRMATION_CHALLENGE_CONTRACT, CONFIRMATION_CONSUMPTION_CONTRACT,
    CONFIRMATION_RESPONSE_CONTRACT, CONFIRMATION_SCHEMA_VERSION, ConfirmationChallenge,
    ConfirmationConsumption, ConfirmationResponse, ConfirmationRisk, ConfirmationSurface,
    confirmation_response_sha256, seal_confirmation_challenge, seal_confirmation_consumption,
};
use rz0_transaction_contract::{
    DurabilityRequirements, EXTERNAL_EFFECT_RECEIPT_CONTRACT,
    EXTERNAL_EFFECT_RECEIPT_SCHEMA_VERSION, ExternalEffectErrorCode,
    ExternalEffectPublicationInput, ExternalEffectPublicationStatus, ExternalEffectReceipt,
    ExternalEffectRecoveryDecision, ExternalEffectStatus, TRANSACTION_CONTRACT,
    TRANSACTION_SCHEMA_VERSION, TransactionEvent, TransactionEventKind, TransactionJournal,
    TransactionOperation, TransactionState, arguments_sha256, assess_external_effect_recovery,
    publish_confirmation_consumption, publish_external_effect_receipt,
    publish_external_effect_receipt_cancellable, publish_journal_snapshot,
    seal_external_effect_receipt, seal_transaction_journal, validate_external_effect_receipt,
};
use sha2::{Digest, Sha256};

const BEFORE: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const AFTER: &str = "2222222222222222222222222222222222222222222222222222222222222222";

#[test]
fn external_effect_receipt_publishes_before_final_commit_and_recovers_deterministically() {
    let root = TestRoot::new();
    let evidence = Evidence::new(root.path());
    let receipt = evidence.receipt();
    assert!(
        validate_external_effect_receipt(&receipt, &evidence.journal).valid,
        "{:?}",
        validate_external_effect_receipt(&receipt, &evidence.journal).errors
    );
    let input = || ExternalEffectPublicationInput {
        commit_pending_journal: &evidence.journal,
        action_plan: &evidence.plan,
        challenge: &evidence.challenge,
        response: &evidence.response,
        consumption: &evidence.consumption,
        receipt: &receipt,
    };
    let publication =
        publish_external_effect_receipt(root.path(), input()).expect("publish effect receipt");
    assert_eq!(
        publication.status,
        ExternalEffectPublicationStatus::Published
    );
    assert!(!publication.automatic_mutation_authorized);
    assert!(
        root.path()
            .join("receipts")
            .join(&publication.receipt_name)
            .is_file()
    );
    let duplicate =
        publish_external_effect_receipt(root.path(), input()).expect("idempotent effect receipt");
    assert_eq!(
        duplicate.status,
        ExternalEffectPublicationStatus::AlreadyPublished
    );

    let interrupted =
        assess_external_effect_recovery(root.path(), &evidence.journal.transaction_id)
            .expect("assess commit-pending effect");
    assert_eq!(
        interrupted.decision,
        ExternalEffectRecoveryDecision::CompleteJournalCommitWithExplicitApproval
    );
    assert!(interrupted.receipt_valid);
    assert!(interrupted.read_only);
    assert!(!interrupted.writes_attempted);
    assert_eq!(
        interrupted.contract,
        rz0_transaction_contract::EXTERNAL_EFFECT_RECOVERY_CONTRACT
    );
    assert!(!interrupted.automatic_mutation_authorized);

    let mut committed = evidence.journal.clone();
    append_and_publish(
        root.path(),
        &mut committed,
        event(TransactionEventKind::Committed),
    );
    let complete = assess_external_effect_recovery(root.path(), &committed.transaction_id)
        .expect("assess completed effect");
    assert_eq!(complete.decision, ExternalEffectRecoveryDecision::NoAction);
    assert!(complete.receipt_valid);
}

#[test]
fn cancellation_and_tampering_fail_closed_without_fabricating_outcome_evidence() {
    let root = TestRoot::new();
    let evidence = Evidence::new(root.path());
    let mut receipt = evidence.receipt();
    receipt.stdout_sha256 = "f".repeat(64);
    assert!(!validate_external_effect_receipt(&receipt, &evidence.journal).valid);
    let mut unsupported_binding = evidence.receipt();
    unsupported_binding.executable_binding = "fixture_binding".to_string();
    seal_external_effect_receipt(&mut unsupported_binding);
    assert!(!validate_external_effect_receipt(&unsupported_binding, &evidence.journal).valid);
    let mut transplanted = evidence.receipt();
    transplanted.manager = "other-manager".to_string();
    seal_external_effect_receipt(&mut transplanted);
    let transplant_error = publish_external_effect_receipt(
        root.path(),
        ExternalEffectPublicationInput {
            commit_pending_journal: &evidence.journal,
            action_plan: &evidence.plan,
            challenge: &evidence.challenge,
            response: &evidence.response,
            consumption: &evidence.consumption,
            receipt: &transplanted,
        },
    )
    .expect_err("receipt identity must match its exact action");
    assert_eq!(
        transplant_error.code,
        ExternalEffectErrorCode::InvalidEvidence
    );

    let receipt = evidence.receipt();
    let (controller, token) = cancellation_pair();
    controller.cancel(CancellationReason::UserRequested);
    let failure = publish_external_effect_receipt_cancellable(
        root.path(),
        ExternalEffectPublicationInput {
            commit_pending_journal: &evidence.journal,
            action_plan: &evidence.plan,
            challenge: &evidence.challenge,
            response: &evidence.response,
            consumption: &evidence.consumption,
            receipt: &receipt,
        },
        &token,
    )
    .expect_err("pre-publication cancellation");
    assert_eq!(failure.code, ExternalEffectErrorCode::Cancelled);
    assert_eq!(
        failure.foundation_code(),
        rz0_error_contract::FoundationErrorCode::Cancelled
    );
    assert!(
        fs::read_dir(root.path().join("receipts"))
            .expect("receipts")
            .next()
            .is_none()
    );
}

struct Evidence {
    plan: ActionPlan,
    journal: TransactionJournal,
    challenge: ConfirmationChallenge,
    response: ConfirmationResponse,
    consumption: ConfirmationConsumption,
}

impl Evidence {
    fn new(root: &Path) -> Self {
        create_store(root);
        let plan: ActionPlan = serde_json::from_str(include_str!(
            "../../action-plan/tests/fixtures/valid-update.json"
        ))
        .expect("update plan");
        let mut journal = TransactionJournal {
            schema_version: TRANSACTION_SCHEMA_VERSION,
            contract: TRANSACTION_CONTRACT.to_string(),
            transaction_id: "rz0tx-external-effect".to_string(),
            plan_id: plan.plan_id.clone(),
            operation: TransactionOperation::Update,
            state: TransactionState::Prepared,
            durability: DurabilityRequirements::schema_one(),
            events: vec![event(TransactionEventKind::Prepared)],
        };
        seal_transaction_journal(&mut journal);
        publish_journal_snapshot(&root.join("transactions"), &journal).expect("prepared journal");

        let digests = action_plan_digests(&plan).expect("plan digests");
        let capabilities = plan
            .actions
            .iter()
            .flat_map(|action| action.capabilities.iter().copied())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let mut challenge = ConfirmationChallenge {
            schema_version: CONFIRMATION_SCHEMA_VERSION,
            contract: CONFIRMATION_CHALLENGE_CONTRACT.to_string(),
            challenge_id: "challenge-external-effect".to_string(),
            plan_id: plan.plan_id.clone(),
            plan_sha256: digests.plan_sha256,
            dry_run_sha256: "3".repeat(64),
            write_set_sha256: digests.write_set_sha256,
            before_state_sha256: Some(BEFORE.to_string()),
            expected_after_state_sha256: AFTER.to_string(),
            risk: ConfirmationRisk::Mutating,
            action_count: 1,
            capabilities,
            issued_unix_seconds: 1_000,
            expires_unix_seconds: 1_300,
            dry_run_completed: true,
            dry_run_writes_attempted: false,
            rollback_available: false,
            quarantine_available: false,
            manual_recovery_acknowledged: true,
            expected_phrase: String::new(),
            challenge_sha256: String::new(),
        };
        seal_confirmation_challenge(&mut challenge);
        let response = ConfirmationResponse {
            schema_version: CONFIRMATION_SCHEMA_VERSION,
            contract: CONFIRMATION_RESPONSE_CONTRACT.to_string(),
            challenge_id: challenge.challenge_id.clone(),
            challenge_sha256: challenge.challenge_sha256.clone(),
            confirmed_unix_seconds: 1_100,
            surface: ConfirmationSurface::Cli,
            phrase: challenge.expected_phrase.clone(),
            interactive: true,
            single_use: true,
            execution_authorized: false,
        };
        let mut consumption = ConfirmationConsumption {
            schema_version: CONFIRMATION_SCHEMA_VERSION,
            contract: CONFIRMATION_CONSUMPTION_CONTRACT.to_string(),
            transaction_id: journal.transaction_id.clone(),
            plan_id: plan.plan_id.clone(),
            challenge_sha256: challenge.challenge_sha256.clone(),
            response_sha256: confirmation_response_sha256(&response),
            consumed_unix_seconds: 1_110,
            single_use_consumed: true,
            execution_authorized: false,
            binding_sha256: String::new(),
        };
        seal_confirmation_consumption(&mut consumption);
        publish_confirmation_consumption(
            root,
            &journal,
            &plan,
            &challenge,
            &response,
            &consumption,
        )
        .expect("confirmation consumption");

        for next in [
            event(TransactionEventKind::ApplyStarted),
            write_event(TransactionEventKind::WriteIntent),
            write_event(TransactionEventKind::WriteVerified),
            event(TransactionEventKind::CommitStarted),
        ] {
            append_and_publish(root, &mut journal, next);
        }
        Self {
            plan,
            journal,
            challenge,
            response,
            consumption,
        }
    }

    fn receipt(&self) -> ExternalEffectReceipt {
        let head = self.journal.events.last().expect("commit-pending head");
        let digests = action_plan_digests(&self.plan).expect("plan digests");
        let action = &self.plan.actions[0];
        let executable = action
            .executable_identity
            .as_ref()
            .expect("executable identity");
        let mut receipt = ExternalEffectReceipt {
            schema_version: EXTERNAL_EFFECT_RECEIPT_SCHEMA_VERSION,
            contract: EXTERNAL_EFFECT_RECEIPT_CONTRACT.to_string(),
            transaction_id: self.journal.transaction_id.clone(),
            plan_id: self.plan.plan_id.clone(),
            action_id: action.action_id.clone(),
            operation: TransactionOperation::Update,
            manager: action.manager.clone().expect("manager"),
            target: action.target.clone(),
            executable_sha256: executable.sha256.clone(),
            executable_size_bytes: executable.size_bytes,
            executable_binding: "proc_held_descriptor_path".to_string(),
            arguments_sha256: arguments_sha256(&action.arguments),
            started_unix_seconds: 1_120,
            completed_unix_seconds: 1_200,
            exit_code: 0,
            stdout_bytes: 4,
            stderr_bytes: 0,
            stdout_sha256: sha256(b"done"),
            stderr_sha256: sha256(b""),
            verification_sha256: sha256(b"verified"),
            commit_pending_sequence: head.sequence,
            commit_pending_event_sha256: head.event_sha256.clone(),
            commit_pending_snapshot_name: format!(
                "{:04}-{}.json",
                head.sequence, head.event_sha256
            ),
            action_plan_sha256: digests.plan_sha256,
            write_set_sha256: digests.write_set_sha256,
            confirmation_challenge_sha256: self.challenge.challenge_sha256.clone(),
            confirmation_response_sha256: confirmation_response_sha256(&self.response),
            confirmation_consumption_sha256: self.consumption.binding_sha256.clone(),
            rollback_supported: false,
            status: ExternalEffectStatus::Verified,
            writes_attempted: true,
            automatic_mutation_authorized: false,
            binding_sha256: String::new(),
        };
        seal_external_effect_receipt(&mut receipt);
        receipt
    }
}

fn append_and_publish(root: &Path, journal: &mut TransactionJournal, next: TransactionEvent) {
    journal.events.push(next);
    journal.state = match journal.events.last().expect("event").kind {
        TransactionEventKind::ApplyStarted
        | TransactionEventKind::WriteIntent
        | TransactionEventKind::WriteVerified => TransactionState::Applying,
        TransactionEventKind::CommitStarted => TransactionState::CommitPending,
        TransactionEventKind::Committed => TransactionState::Committed,
        _ => unreachable!(),
    };
    seal_transaction_journal(journal);
    publish_journal_snapshot(&root.join("transactions"), journal).expect("journal successor");
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

fn write_event(kind: TransactionEventKind) -> TransactionEvent {
    TransactionEvent {
        sequence: 0,
        kind,
        action_id: Some("update-example-tool".to_string()),
        path: Some("manager/example-manager/update-example-tool".to_string()),
        before_sha256: Some(BEFORE.to_string()),
        after_sha256: Some(AFTER.to_string()),
        previous_event_sha256: String::new(),
        event_sha256: String::new(),
    }
}

fn create_store(root: &Path) {
    fs::create_dir(root.join("transactions")).expect("transactions");
    fs::create_dir(root.join("receipts")).expect("receipts");
    for directory in [root, &root.join("transactions"), &root.join("receipts")] {
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
            .expect("private directory");
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "rz0-external-effect-{}-{nanos}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("test root");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
