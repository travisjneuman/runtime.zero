mod support;

use std::fs;

use rz0_action_plan::ActionPlan;
use support::quarantine_fs::{
    FailurePoint, QuarantineRecord, SimulationRoot, copy_original_fixture, simulate_quarantine,
    simulate_restore,
};

#[test]
fn quarantine_then_restore_round_trip_preserves_verified_copy() {
    let quarantine_plan = plan(include_str!("fixtures/valid-quarantine.json"));
    let restore_plan = plan(include_str!("fixtures/valid-restore.json"));
    let root = SimulationRoot::new();
    copy_original_fixture(root.path());

    let record = simulate_quarantine(root.path(), &quarantine_plan, FailurePoint::None)
        .expect("quarantine simulation");
    let original = root.path().join(&record.original_path);
    let quarantined = root.path().join(&record.quarantine_path);
    assert!(record.simulation_only);
    assert!(record.original_removed_after_verified_copy);
    assert!(!original.exists());
    assert!(quarantined.is_file());
    let record_path = root
        .path()
        .join("quarantine/rz0plan-quarantine-example/quarantine.json");
    assert!(record_path.is_file());
    let recorded: QuarantineRecord =
        serde_json::from_slice(&fs::read(record_path).expect("record bytes"))
            .expect("quarantine record");
    assert_eq!(recorded, record);

    let restored = simulate_restore(root.path(), &restore_plan).expect("restore simulation");
    assert_eq!(restored, original);
    assert!(restored.is_file());
    assert!(quarantined.is_file());
    assert_eq!(
        fs::read(restored).expect("restored bytes"),
        b"Synthetic stale shim fixture.\n"
    );
}

#[test]
fn injected_failure_after_verified_copy_keeps_original_and_copy() {
    let plan = plan(include_str!("fixtures/valid-quarantine.json"));
    let root = SimulationRoot::new();
    copy_original_fixture(root.path());

    let error = simulate_quarantine(root.path(), &plan, FailurePoint::AfterVerifiedCopy)
        .expect_err("injected failure");
    assert!(error.contains("injected failure"));
    assert!(root.path().join("workspace/stale-shim.bin").is_file());
    assert!(
        root.path()
            .join("quarantine/rz0plan-quarantine-example/payload.bin")
            .is_file()
    );
    assert!(
        !root
            .path()
            .join("quarantine/rz0plan-quarantine-example/quarantine.json")
            .exists()
    );
}

#[test]
fn restore_conflict_never_overwrites_existing_destination() {
    let quarantine_plan = plan(include_str!("fixtures/valid-quarantine.json"));
    let restore_plan = plan(include_str!("fixtures/valid-restore.json"));
    let root = SimulationRoot::new();
    copy_original_fixture(root.path());
    simulate_quarantine(root.path(), &quarantine_plan, FailurePoint::None)
        .expect("quarantine simulation");
    let original = root.path().join("workspace/stale-shim.bin");
    fs::write(&original, b"occupied destination\n").expect("restore conflict");

    let error = simulate_restore(root.path(), &restore_plan).expect_err("restore conflict");
    assert!(error.contains("already exists"));
    assert_eq!(
        fs::read(original).expect("occupied bytes"),
        b"occupied destination\n"
    );
    assert!(
        root.path()
            .join("quarantine/rz0plan-quarantine-example/payload.bin")
            .is_file()
    );
}

#[test]
fn tampered_source_never_creates_quarantine_output() {
    let plan = plan(include_str!("fixtures/valid-quarantine.json"));
    let root = SimulationRoot::new();
    copy_original_fixture(root.path());
    fs::write(
        root.path().join("workspace/stale-shim.bin"),
        b"tampered source\n",
    )
    .expect("tampered source");

    let error =
        simulate_quarantine(root.path(), &plan, FailurePoint::None).expect_err("source mismatch");
    assert!(error.contains("hash or size mismatch"));
    assert!(root.path().join("workspace/stale-shim.bin").is_file());
    assert!(!root.path().join("quarantine").exists());
}

#[cfg(unix)]
#[test]
fn symlinked_source_fails_closed() {
    use std::os::unix::fs::symlink;

    let plan = plan(include_str!("fixtures/valid-quarantine.json"));
    let root = SimulationRoot::new();
    let workspace = root.path().join("workspace");
    fs::create_dir_all(&workspace).expect("workspace");
    symlink(
        root.path().join(".rz0-transaction-simulation-v1"),
        workspace.join("stale-shim.bin"),
    )
    .expect("source symlink");

    let error =
        simulate_quarantine(root.path(), &plan, FailurePoint::None).expect_err("symlink rejection");
    assert!(error.contains("symlink"));
    assert!(!root.path().join("quarantine").exists());
}

fn plan(source: &str) -> ActionPlan {
    serde_json::from_str(source).expect("valid transaction plan fixture")
}
