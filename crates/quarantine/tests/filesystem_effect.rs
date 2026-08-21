#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use rz0_action_plan::ActionPlan;
use rz0_cancellation_contract::{CancellationReason, cancellation_pair};
use rz0_confirmation_contract::{
    ConfirmationChallenge, ConfirmationConsumption, ConfirmationResponse, ConfirmationRisk,
    ConfirmationSurface, confirmation_response_sha256, seal_confirmation_challenge,
    seal_confirmation_consumption,
};
use rz0_quarantine::{
    FilesystemEffectRequest, FilesystemEffectStatus, execute_filesystem_effect,
    validate_filesystem_effect_receipt, validate_quarantine_record,
};
use rz0_transaction_contract::{TransactionEventKind, TransactionState, recover_journal_head};
use sha2::{Digest, Sha256};

#[test]
fn quarantine_and_restore_round_trip_is_receipt_bound() {
    let roots = TestRoots::new();
    write_private_file(
        &roots.source.join("stale-shim.bin"),
        b"Synthetic stale shim fixture.\n",
    );
    let quarantine_plan = plan("valid-quarantine.json");
    let quarantine_evidence = evidence(&quarantine_plan, 1_100);
    let quarantine = execute_filesystem_effect(FilesystemEffectRequest {
        state_root: &roots.state,
        source_root: &roots.source,
        quarantine_root: &roots.quarantine,
        plan: &quarantine_plan,
        action: &quarantine_plan.actions[0],
        challenge: &quarantine_evidence.challenge,
        response: &quarantine_evidence.response,
        consumption: &quarantine_evidence.consumption,
        workspace_namespace: None,
        cancellation: None,
        now_unix_seconds: 1_100,
    })
    .expect("quarantine effect");
    assert_eq!(quarantine.status, FilesystemEffectStatus::Committed);
    assert!(!roots.source.join("stale-shim.bin").exists());
    assert!(
        roots
            .quarantine
            .join("rz0plan-quarantine-example/payload.bin")
            .is_file()
    );
    let record_bytes = fs::read(
        roots
            .quarantine
            .join("rz0plan-quarantine-example/quarantine.json"),
    )
    .expect("record bytes");
    let record: rz0_quarantine::QuarantineRecord =
        serde_json::from_slice(&record_bytes).expect("record");
    assert!(validate_quarantine_record(&record));

    let journal = recover_journal_head(
        &roots.state.join("transactions"),
        &quarantine.transaction_id,
    )
    .expect("quarantine journal");
    let receipt: rz0_quarantine::FilesystemEffectReceipt = serde_json::from_slice(
        &fs::read(roots.state.join("receipts/rz0plan-quarantine-example.json"))
            .expect("receipt bytes"),
    )
    .expect("receipt");
    assert!(validate_filesystem_effect_receipt(&receipt, &journal.journal).valid);

    let restore_plan = plan("valid-restore.json");
    let restore_evidence = evidence(&restore_plan, 2_100);
    let restore = execute_filesystem_effect(FilesystemEffectRequest {
        state_root: &roots.state,
        source_root: &roots.source,
        quarantine_root: &roots.quarantine,
        plan: &restore_plan,
        action: &restore_plan.actions[0],
        challenge: &restore_evidence.challenge,
        response: &restore_evidence.response,
        consumption: &restore_evidence.consumption,
        workspace_namespace: None,
        cancellation: None,
        now_unix_seconds: 2_100,
    })
    .expect("restore effect");
    assert_eq!(restore.status, FilesystemEffectStatus::Committed);
    assert_eq!(
        fs::read(roots.source.join("stale-shim.bin")).expect("restored bytes"),
        b"Synthetic stale shim fixture.\n"
    );
    assert!(
        !roots
            .quarantine
            .join("rz0plan-quarantine-example/payload.bin")
            .exists()
    );
}

#[test]
fn source_drift_is_rejected_before_any_destination_is_created() {
    let roots = TestRoots::new();
    write_private_file(&roots.source.join("stale-shim.bin"), b"tampered\n");
    let plan = plan("valid-quarantine.json");
    let evidence = evidence(&plan, 1_100);
    let error = execute_filesystem_effect(FilesystemEffectRequest {
        state_root: &roots.state,
        source_root: &roots.source,
        quarantine_root: &roots.quarantine,
        plan: &plan,
        action: &plan.actions[0],
        challenge: &evidence.challenge,
        response: &evidence.response,
        consumption: &evidence.consumption,
        workspace_namespace: None,
        cancellation: None,
        now_unix_seconds: 1_100,
    })
    .expect_err("source drift");
    assert_eq!(
        error.code,
        rz0_quarantine::FilesystemEffectErrorCode::SourceChanged
    );
    assert!(roots.source.join("stale-shim.bin").is_file());
    assert!(!roots.quarantine.join("rz0plan-quarantine-example").exists());
}

#[test]
fn occupied_destination_fails_closed_without_removing_source() {
    let roots = TestRoots::new();
    write_private_file(
        &roots.source.join("stale-shim.bin"),
        b"Synthetic stale shim fixture.\n",
    );
    fs::create_dir_all(roots.quarantine.join("rz0plan-quarantine-example"))
        .expect("quarantine destination");
    fs::set_permissions(
        roots.quarantine.join("rz0plan-quarantine-example"),
        fs::Permissions::from_mode(0o700),
    )
    .expect("private quarantine destination");
    write_private_file(
        &roots
            .quarantine
            .join("rz0plan-quarantine-example/payload.bin"),
        b"occupied\n",
    );
    let plan = plan("valid-quarantine.json");
    let evidence = evidence(&plan, 1_100);
    let error = execute_filesystem_effect(FilesystemEffectRequest {
        state_root: &roots.state,
        source_root: &roots.source,
        quarantine_root: &roots.quarantine,
        plan: &plan,
        action: &plan.actions[0],
        challenge: &evidence.challenge,
        response: &evidence.response,
        consumption: &evidence.consumption,
        workspace_namespace: None,
        cancellation: None,
        now_unix_seconds: 1_100,
    })
    .expect_err("occupied destination");
    assert_eq!(
        error.code,
        rz0_quarantine::FilesystemEffectErrorCode::Conflict
    );
    assert!(roots.source.join("stale-shim.bin").is_file());
    assert_eq!(
        fs::read(
            roots
                .quarantine
                .join("rz0plan-quarantine-example/payload.bin")
        )
        .expect("occupied bytes"),
        b"occupied\n"
    );
}

#[test]
fn invalid_confirmation_is_rejected_before_transaction_creation() {
    let roots = TestRoots::new();
    write_private_file(
        &roots.source.join("stale-shim.bin"),
        b"Synthetic stale shim fixture.\n",
    );
    let plan = plan("valid-quarantine.json");
    let mut evidence = evidence(&plan, 1_100);
    evidence.response.phrase = "wrong".to_string();
    let error = execute_filesystem_effect(FilesystemEffectRequest {
        state_root: &roots.state,
        source_root: &roots.source,
        quarantine_root: &roots.quarantine,
        plan: &plan,
        action: &plan.actions[0],
        challenge: &evidence.challenge,
        response: &evidence.response,
        consumption: &evidence.consumption,
        workspace_namespace: None,
        cancellation: None,
        now_unix_seconds: 1_100,
    })
    .expect_err("invalid confirmation");
    assert_eq!(
        error.code,
        rz0_quarantine::FilesystemEffectErrorCode::InvalidEvidence
    );
    assert!(
        fs::read_dir(roots.state.join("transactions"))
            .expect("transactions directory")
            .next()
            .is_none()
    );
}

#[test]
fn quarantine_record_failure_is_marked_for_recovery_after_payload_move() {
    let roots = TestRoots::new();
    write_private_file(
        &roots.source.join("stale-shim.bin"),
        b"Synthetic stale shim fixture.\n",
    );
    let occupied_record = roots
        .quarantine
        .join("rz0plan-quarantine-example/quarantine.json");
    fs::create_dir_all(occupied_record.parent().expect("record parent")).expect("record parent");
    fs::set_permissions(
        occupied_record.parent().expect("record parent"),
        fs::Permissions::from_mode(0o700),
    )
    .expect("record parent permissions");
    write_private_file(&occupied_record, b"occupied record\n");

    let plan = plan("valid-quarantine.json");
    let evidence = evidence(&plan, 1_100);
    let error = execute_filesystem_effect(FilesystemEffectRequest {
        state_root: &roots.state,
        source_root: &roots.source,
        quarantine_root: &roots.quarantine,
        plan: &plan,
        action: &plan.actions[0],
        challenge: &evidence.challenge,
        response: &evidence.response,
        consumption: &evidence.consumption,
        workspace_namespace: None,
        cancellation: None,
        now_unix_seconds: 1_100,
    })
    .expect_err("occupied quarantine record");
    assert_eq!(
        error.code,
        rz0_quarantine::FilesystemEffectErrorCode::RecoveryRequired
    );
    assert!(!roots.source.join("stale-shim.bin").exists());
    assert!(
        roots
            .quarantine
            .join("rz0plan-quarantine-example/payload.bin")
            .is_file()
    );
    let journal = recover_journal_head(
        &roots.state.join("transactions"),
        &transaction_id(&plan, 1_100),
    )
    .expect("recovery journal")
    .journal;
    assert_eq!(journal.state, TransactionState::RecoveryRequired);
    assert_eq!(
        journal.events.last().expect("recovery event").kind,
        TransactionEventKind::RecoveryRequired
    );
}

#[test]
fn receipt_publication_failure_keeps_committed_journal_for_manual_verification() {
    let roots = TestRoots::new();
    write_private_file(
        &roots.source.join("stale-shim.bin"),
        b"Synthetic stale shim fixture.\n",
    );
    write_private_file(
        &roots.state.join("receipts/rz0plan-quarantine-example.json"),
        b"occupied receipt\n",
    );

    let plan = plan("valid-quarantine.json");
    let evidence = evidence(&plan, 1_100);
    let error = execute_filesystem_effect(FilesystemEffectRequest {
        state_root: &roots.state,
        source_root: &roots.source,
        quarantine_root: &roots.quarantine,
        plan: &plan,
        action: &plan.actions[0],
        challenge: &evidence.challenge,
        response: &evidence.response,
        consumption: &evidence.consumption,
        workspace_namespace: None,
        cancellation: None,
        now_unix_seconds: 1_100,
    })
    .expect_err("occupied receipt");
    assert_eq!(
        error.code,
        rz0_quarantine::FilesystemEffectErrorCode::RecoveryRequired
    );
    assert!(!roots.source.join("stale-shim.bin").exists());
    let journal = recover_journal_head(
        &roots.state.join("transactions"),
        &transaction_id(&plan, 1_100),
    )
    .expect("committed journal")
    .journal;
    assert_eq!(journal.state, TransactionState::Committed);
    assert_eq!(
        fs::read(roots.state.join("receipts/rz0plan-quarantine-example.json"))
            .expect("occupied receipt bytes"),
        b"occupied receipt\n"
    );
}

#[test]
fn cancellation_before_transaction_creation_is_typed_and_write_free() {
    let roots = TestRoots::new();
    write_private_file(
        &roots.source.join("stale-shim.bin"),
        b"Synthetic stale shim fixture.\n",
    );
    let plan = plan("valid-quarantine.json");
    let evidence = evidence(&plan, 1_100);
    let (controller, token) = cancellation_pair();
    assert_eq!(
        controller.cancel(CancellationReason::UserRequested),
        rz0_cancellation_contract::CancelOutcome::Won(CancellationReason::UserRequested)
    );
    let error = execute_filesystem_effect(FilesystemEffectRequest {
        state_root: &roots.state,
        source_root: &roots.source,
        quarantine_root: &roots.quarantine,
        plan: &plan,
        action: &plan.actions[0],
        challenge: &evidence.challenge,
        response: &evidence.response,
        consumption: &evidence.consumption,
        workspace_namespace: None,
        cancellation: Some(&token),
        now_unix_seconds: 1_100,
    })
    .expect_err("cancelled effect");
    assert_eq!(
        error.code,
        rz0_quarantine::FilesystemEffectErrorCode::Cancelled
    );
    assert!(roots.source.join("stale-shim.bin").is_file());
    assert!(
        fs::read_dir(roots.state.join("transactions"))
            .expect("transactions directory")
            .next()
            .is_none()
    );
}

struct Evidence {
    challenge: ConfirmationChallenge,
    response: ConfirmationResponse,
    consumption: ConfirmationConsumption,
}

fn evidence(plan: &ActionPlan, now: u64) -> Evidence {
    let digests = rz0_action_plan::action_plan_digests(plan).expect("plan digests");
    let mut challenge = ConfirmationChallenge {
        schema_version: rz0_confirmation_contract::CONFIRMATION_SCHEMA_VERSION,
        contract: rz0_confirmation_contract::CONFIRMATION_CHALLENGE_CONTRACT.to_string(),
        challenge_id: format!("challenge.{}", plan.plan_id),
        plan_id: plan.plan_id.clone(),
        plan_sha256: digests.plan_sha256.clone(),
        dry_run_sha256: digests.plan_sha256,
        write_set_sha256: digests.write_set_sha256,
        before_state_sha256: Some("a".repeat(64)),
        expected_after_state_sha256: "b".repeat(64),
        risk: ConfirmationRisk::Mutating,
        action_count: 1,
        capabilities: plan.actions[0].capabilities.clone(),
        issued_unix_seconds: now - 100,
        expires_unix_seconds: now + 100,
        dry_run_completed: true,
        dry_run_writes_attempted: false,
        rollback_available: true,
        quarantine_available: true,
        manual_recovery_acknowledged: false,
        expected_phrase: String::new(),
        challenge_sha256: String::new(),
    };
    seal_confirmation_challenge(&mut challenge);
    let response = ConfirmationResponse {
        schema_version: rz0_confirmation_contract::CONFIRMATION_SCHEMA_VERSION,
        contract: rz0_confirmation_contract::CONFIRMATION_RESPONSE_CONTRACT.to_string(),
        challenge_id: challenge.challenge_id.clone(),
        challenge_sha256: challenge.challenge_sha256.clone(),
        confirmed_unix_seconds: now - 50,
        surface: ConfirmationSurface::Cli,
        phrase: challenge.expected_phrase.clone(),
        interactive: true,
        single_use: true,
        execution_authorized: false,
    };
    let transaction_id = transaction_id(plan, now);
    let mut consumption = ConfirmationConsumption {
        schema_version: rz0_confirmation_contract::CONFIRMATION_SCHEMA_VERSION,
        contract: rz0_confirmation_contract::CONFIRMATION_CONSUMPTION_CONTRACT.to_string(),
        transaction_id,
        plan_id: plan.plan_id.clone(),
        challenge_sha256: challenge.challenge_sha256.clone(),
        response_sha256: confirmation_response_sha256(&response),
        consumed_unix_seconds: now - 40,
        single_use_consumed: true,
        execution_authorized: false,
        binding_sha256: String::new(),
    };
    seal_confirmation_consumption(&mut consumption);
    Evidence {
        challenge,
        response,
        consumption,
    }
}

fn transaction_id(plan: &ActionPlan, now: u64) -> String {
    let operation = if plan.actions[0].kind == rz0_action_plan::ActionKind::Quarantine {
        "quarantine"
    } else {
        "restore"
    };
    let digest = format!("{:x}", Sha256::digest(plan.plan_id.as_bytes()));
    format!("tx.{operation}.{}.{}", &digest[..16], now)
}

fn write_private_file(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).expect("private fixture bytes");
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).expect("private fixture file");
}

fn plan(name: &str) -> ActionPlan {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../action-plan/tests/fixtures")
        .join(name);
    serde_json::from_slice(&fs::read(path).expect("plan fixture")).expect("action plan")
}

struct TestRoots {
    root: PathBuf,
    state: PathBuf,
    source: PathBuf,
    quarantine: PathBuf,
}

impl TestRoots {
    fn new() -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "rz0-quarantine-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("test root");
        let state = root.join("state");
        let source = root.join("source");
        let quarantine = root.join("quarantine");
        for directory in [&state, &source, &quarantine] {
            fs::create_dir(directory).expect("test directory");
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
                .expect("private test directory");
        }
        let transactions = state.join("transactions");
        let receipts = state.join("receipts");
        fs::create_dir(&transactions).expect("transactions");
        fs::create_dir(&receipts).expect("receipts");
        for directory in [&transactions, &receipts] {
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
                .expect("private state child");
        }
        Self {
            root,
            state,
            source,
            quarantine,
        }
    }
}

impl Drop for TestRoots {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
