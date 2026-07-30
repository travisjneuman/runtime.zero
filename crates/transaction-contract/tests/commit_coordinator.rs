#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use rz0_action_plan::{ActionPlan, action_plan_digests};
use rz0_confirmation_contract::{
    CONFIRMATION_CHALLENGE_CONTRACT, CONFIRMATION_CONSUMPTION_CONTRACT,
    CONFIRMATION_RESPONSE_CONTRACT, CONFIRMATION_SCHEMA_VERSION, ConfirmationChallenge,
    ConfirmationConsumption, ConfirmationResponse, ConfirmationRisk, ConfirmationSurface,
    confirmation_response_sha256, seal_confirmation_challenge, seal_confirmation_consumption,
};
use rz0_registry_contract::{InstalledModuleRecord, InstalledRegistry, canonical_registry_bytes};
use rz0_transaction_contract::{
    COMMIT_RECEIPT_CONTRACT, COMMIT_RECEIPT_SCHEMA_VERSION, CommitCoordinatorInput,
    CommitPublicationRequirements, CommitPublicationStatus, CommitRecoveryDecision,
    ConfirmationPublication, CoordinatorErrorCode, DurabilityRequirements,
    EvidencePublicationStatus, TRANSACTION_CONTRACT, TRANSACTION_SCHEMA_VERSION,
    TransactionCommitReceipt, TransactionEvent, TransactionEventKind, TransactionJournal,
    TransactionOperation, TransactionState, assess_commit_recovery, publish_committed_state,
    publish_confirmation_consumption, publish_journal_snapshot, seal_commit_receipt,
    seal_transaction_journal,
};

const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

#[test]
fn exact_confirmation_receipt_and_registry_publish_in_order_and_are_idempotent() {
    let root = TestRoot::new();
    let plan = plan();
    let next_registry = InstalledRegistry {
        schema_version: 1,
        modules: Vec::new(),
    };
    let (mut journal, challenge, response, consumption) =
        prepared_evidence(root.path(), &plan, None, &next_registry);
    let confirmation = publish_confirmation_consumption(
        root.path(),
        &journal,
        &plan,
        &challenge,
        &response,
        &consumption,
    )
    .expect("publish confirmation");
    assert_eq!(confirmation.status, EvidencePublicationStatus::Published);
    assert!(!root.path().join("installed-modules.json").exists());

    commit_journal(root.path(), &mut journal);
    let receipt = receipt(
        &journal,
        &plan,
        &challenge,
        &response,
        &consumption,
        None,
        &next_registry,
    );
    let input = || CommitCoordinatorInput {
        committed_journal: &journal,
        action_plan: &plan,
        challenge: &challenge,
        response: &response,
        consumption: &consumption,
        receipt: &receipt,
        next_registry: &next_registry,
    };
    let publication = publish_committed_state(root.path(), input()).expect("commit state");
    assert_eq!(publication.status, CommitPublicationStatus::Committed);
    assert!(!publication.automatic_mutation_authorized);
    assert_eq!(
        fs::read(root.path().join("installed-modules.json")).unwrap(),
        canonical_registry_bytes(&next_registry).unwrap()
    );
    assert!(
        root.path()
            .join("receipts")
            .join(format!("{}.json", plan.plan_id))
            .is_file()
    );
    assert!(
        root.path()
            .join("transactions")
            .join(&journal.transaction_id)
            .join("confirmation.json")
            .is_file()
    );

    let duplicate = publish_committed_state(root.path(), input()).expect("idempotent commit");
    assert_eq!(duplicate.status, CommitPublicationStatus::AlreadyCommitted);
}

#[test]
fn confirmation_is_single_value_and_registry_drift_blocks_receipt_publication() {
    let root = TestRoot::new();
    let plan = plan();
    let next_registry = InstalledRegistry {
        schema_version: 1,
        modules: Vec::new(),
    };
    let (mut journal, challenge, response, consumption) =
        prepared_evidence(root.path(), &plan, None, &next_registry);
    let first = publish_confirmation_consumption(
        root.path(),
        &journal,
        &plan,
        &challenge,
        &response,
        &consumption,
    )
    .expect("publish confirmation");
    assert_eq!(first.status, EvidencePublicationStatus::Published);
    let duplicate: ConfirmationPublication = publish_confirmation_consumption(
        root.path(),
        &journal,
        &plan,
        &challenge,
        &response,
        &consumption,
    )
    .expect("idempotent confirmation");
    assert_eq!(
        duplicate.status,
        EvidencePublicationStatus::AlreadyPublished
    );

    write_private(&root.path().join("installed-modules.json"), b"unexpected");
    commit_journal(root.path(), &mut journal);
    let receipt = receipt(
        &journal,
        &plan,
        &challenge,
        &response,
        &consumption,
        None,
        &next_registry,
    );
    let error = publish_committed_state(
        root.path(),
        CommitCoordinatorInput {
            committed_journal: &journal,
            action_plan: &plan,
            challenge: &challenge,
            response: &response,
            consumption: &consumption,
            receipt: &receipt,
            next_registry: &next_registry,
        },
    )
    .expect_err("drift must block");
    assert_eq!(error.code, CoordinatorErrorCode::Conflict);
    assert!(
        !root
            .path()
            .join("receipts")
            .join(format!("{}.json", plan.plan_id))
            .exists()
    );
}

#[test]
fn receipt_before_registry_interruption_requires_explicit_recovery_completion() {
    let root = TestRoot::new();
    let plan = plan();
    let next_registry = InstalledRegistry {
        schema_version: 1,
        modules: Vec::new(),
    };
    let (mut journal, challenge, response, consumption) =
        prepared_evidence(root.path(), &plan, None, &next_registry);
    publish_confirmation_consumption(
        root.path(),
        &journal,
        &plan,
        &challenge,
        &response,
        &consumption,
    )
    .expect("confirmation");
    commit_journal(root.path(), &mut journal);
    let receipt = receipt(
        &journal,
        &plan,
        &challenge,
        &response,
        &consumption,
        None,
        &next_registry,
    );
    let mut receipt_bytes = serde_json::to_vec(&receipt).expect("receipt bytes");
    receipt_bytes.push(b'\n');
    write_private(
        &root
            .path()
            .join("receipts")
            .join(format!("{}.json", plan.plan_id)),
        &receipt_bytes,
    );
    write_private(
        &root
            .path()
            .join("transactions")
            .join(&journal.transaction_id)
            .join("registry-next.json"),
        &canonical_registry_bytes(&next_registry).unwrap(),
    );

    let assessment = assess_commit_recovery(
        root.path(),
        &journal,
        &consumption,
        &receipt,
        &next_registry,
    )
    .expect("assess interruption");
    assert_eq!(
        assessment.decision,
        CommitRecoveryDecision::CompleteRegistryPublicationWithExplicitApproval
    );
    assert!(!assessment.automatic_mutation_authorized);
    assert!(!root.path().join("installed-modules.json").exists());
}

#[test]
fn existing_registry_is_copied_for_recovery_before_atomic_replacement() {
    let root = TestRoot::new();
    let plan = plan();
    let prior_registry = InstalledRegistry {
        schema_version: 1,
        modules: vec![InstalledModuleRecord {
            id: "first-party.inventory".to_string(),
            version: "0.1.0".to_string(),
            manifest_path: "modules/first-party.inventory/0.1.0/rz0-module.json".to_string(),
            receipt_path: "receipts/rz0plan-inventory.json".to_string(),
            module_dir: Some("modules/first-party.inventory/0.1.0".to_string()),
        }],
    };
    let prior_bytes = canonical_registry_bytes(&prior_registry).expect("prior registry");
    let next_registry = InstalledRegistry {
        schema_version: 1,
        modules: Vec::new(),
    };
    let (mut journal, challenge, response, consumption) =
        prepared_evidence(root.path(), &plan, Some(&prior_bytes), &next_registry);
    write_private(&root.path().join("installed-modules.json"), &prior_bytes);
    publish_confirmation_consumption(
        root.path(),
        &journal,
        &plan,
        &challenge,
        &response,
        &consumption,
    )
    .expect("confirmation");
    commit_journal(root.path(), &mut journal);
    let receipt = receipt(
        &journal,
        &plan,
        &challenge,
        &response,
        &consumption,
        Some(&prior_bytes),
        &next_registry,
    );
    publish_committed_state(
        root.path(),
        CommitCoordinatorInput {
            committed_journal: &journal,
            action_plan: &plan,
            challenge: &challenge,
            response: &response,
            consumption: &consumption,
            receipt: &receipt,
            next_registry: &next_registry,
        },
    )
    .expect("replace registry");
    let transaction = root
        .path()
        .join("transactions")
        .join(&journal.transaction_id);
    assert_eq!(
        fs::read(transaction.join("registry-before.json")).unwrap(),
        prior_bytes
    );
    assert_eq!(
        fs::read(root.path().join("installed-modules.json")).unwrap(),
        canonical_registry_bytes(&next_registry).unwrap()
    );
    assert!(!transaction.join("registry-next.json").exists());
}

fn prepared_evidence(
    state_root: &Path,
    plan: &ActionPlan,
    before_registry: Option<&[u8]>,
    next_registry: &InstalledRegistry,
) -> (
    TransactionJournal,
    ConfirmationChallenge,
    ConfirmationResponse,
    ConfirmationConsumption,
) {
    fs::create_dir(state_root.join("transactions")).expect("transactions");
    fs::create_dir(state_root.join("receipts")).expect("receipts");
    fs::set_permissions(state_root, fs::Permissions::from_mode(0o700)).expect("private state root");
    fs::set_permissions(
        state_root.join("transactions"),
        fs::Permissions::from_mode(0o700),
    )
    .expect("private transactions");
    fs::set_permissions(
        state_root.join("receipts"),
        fs::Permissions::from_mode(0o700),
    )
    .expect("private receipts");
    let mut journal = TransactionJournal {
        schema_version: TRANSACTION_SCHEMA_VERSION,
        contract: TRANSACTION_CONTRACT.to_string(),
        transaction_id: "rz0tx-commit-coordinator".to_string(),
        plan_id: plan.plan_id.clone(),
        operation: TransactionOperation::Quarantine,
        state: TransactionState::Prepared,
        durability: DurabilityRequirements::schema_one(),
        events: vec![event(TransactionEventKind::Prepared, None, None, None)],
    };
    seal_transaction_journal(&mut journal);
    publish_journal_snapshot(&state_root.join("transactions"), &journal).expect("prepared head");

    let digests = action_plan_digests(plan).expect("plan digests");
    let registry_sha256 = rz0_registry_contract::bytes_sha256(
        &canonical_registry_bytes(next_registry).expect("registry bytes"),
    );
    let mut challenge = ConfirmationChallenge {
        schema_version: CONFIRMATION_SCHEMA_VERSION,
        contract: CONFIRMATION_CHALLENGE_CONTRACT.to_string(),
        challenge_id: "challenge-commit-coordinator".to_string(),
        plan_id: plan.plan_id.clone(),
        plan_sha256: digests.plan_sha256,
        dry_run_sha256: A.to_string(),
        write_set_sha256: digests.write_set_sha256,
        before_state_sha256: before_registry.map(rz0_registry_contract::bytes_sha256),
        expected_after_state_sha256: registry_sha256,
        risk: ConfirmationRisk::Mutating,
        action_count: plan.actions.len() as u16,
        capabilities: plan.actions[0].capabilities.clone(),
        issued_unix_seconds: 1_000,
        expires_unix_seconds: 1_200,
        dry_run_completed: true,
        dry_run_writes_attempted: false,
        rollback_available: true,
        quarantine_available: true,
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
    (journal, challenge, response, consumption)
}

fn commit_journal(root: &Path, journal: &mut TransactionJournal) {
    let transaction_root = root.join("transactions");
    for item in [
        event(TransactionEventKind::ApplyStarted, None, None, None),
        event(
            TransactionEventKind::WriteIntent,
            Some("quarantine-stale-shim"),
            Some("quarantine/rz0plan-quarantine-example/payload.bin"),
            Some(A),
        ),
        event(
            TransactionEventKind::WriteVerified,
            Some("quarantine-stale-shim"),
            Some("quarantine/rz0plan-quarantine-example/payload.bin"),
            Some(A),
        ),
        event(
            TransactionEventKind::WriteIntent,
            Some("quarantine-stale-shim"),
            Some("quarantine/rz0plan-quarantine-example/quarantine.json"),
            Some(B),
        ),
        event(
            TransactionEventKind::WriteVerified,
            Some("quarantine-stale-shim"),
            Some("quarantine/rz0plan-quarantine-example/quarantine.json"),
            Some(B),
        ),
        event(TransactionEventKind::CommitStarted, None, None, None),
        event(TransactionEventKind::Committed, None, None, None),
    ] {
        journal.events.push(item);
        journal.state = match journal.events.last().unwrap().kind {
            TransactionEventKind::ApplyStarted
            | TransactionEventKind::WriteIntent
            | TransactionEventKind::WriteVerified => TransactionState::Applying,
            TransactionEventKind::CommitStarted => TransactionState::CommitPending,
            TransactionEventKind::Committed => TransactionState::Committed,
            _ => unreachable!(),
        };
        seal_transaction_journal(journal);
        publish_journal_snapshot(&transaction_root, journal).expect("publish successor");
    }
}

fn receipt(
    journal: &TransactionJournal,
    plan: &ActionPlan,
    challenge: &ConfirmationChallenge,
    response: &ConfirmationResponse,
    consumption: &ConfirmationConsumption,
    before_registry: Option<&[u8]>,
    registry: &InstalledRegistry,
) -> TransactionCommitReceipt {
    let head = journal.events.last().expect("head");
    let digests = action_plan_digests(plan).expect("digests");
    let mut receipt = TransactionCommitReceipt {
        schema_version: COMMIT_RECEIPT_SCHEMA_VERSION,
        contract: COMMIT_RECEIPT_CONTRACT.to_string(),
        transaction_id: journal.transaction_id.clone(),
        plan_id: plan.plan_id.clone(),
        operation: journal.operation,
        committed_event_sequence: head.sequence,
        committed_event_sha256: head.event_sha256.clone(),
        journal_snapshot_name: format!("{:04}-{}.json", head.sequence, head.event_sha256),
        action_plan_sha256: digests.plan_sha256,
        write_set_sha256: digests.write_set_sha256,
        confirmation_challenge_sha256: challenge.challenge_sha256.clone(),
        confirmation_response_sha256: confirmation_response_sha256(response),
        confirmation_consumption_sha256: consumption.binding_sha256.clone(),
        confirmation_consumed: true,
        registry_before_sha256: before_registry.map(rz0_registry_contract::bytes_sha256),
        registry_after_sha256: rz0_registry_contract::bytes_sha256(
            &canonical_registry_bytes(registry).expect("registry bytes"),
        ),
        publication: CommitPublicationRequirements::schema_one(),
        binding_sha256: String::new(),
        automatic_mutation_authorized: false,
    };
    seal_commit_receipt(&mut receipt);
    receipt
}

fn event(
    kind: TransactionEventKind,
    action: Option<&str>,
    path: Option<&str>,
    after: Option<&str>,
) -> TransactionEvent {
    TransactionEvent {
        sequence: 0,
        kind,
        action_id: action.map(str::to_string),
        path: path.map(str::to_string),
        before_sha256: None,
        after_sha256: after.map(str::to_string),
        previous_event_sha256: String::new(),
        event_sha256: String::new(),
    }
}

fn write_private(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).expect("write private test document");
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .expect("set private test document mode");
}

fn plan() -> ActionPlan {
    serde_json::from_str(include_str!(
        "../../action-plan/tests/fixtures/valid-quarantine.json"
    ))
    .expect("valid quarantine fixture")
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
            "rz0-commit-coordinator-{}-{nanos}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).expect("create state root");
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
