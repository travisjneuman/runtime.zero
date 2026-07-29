mod support;

use std::fs;

use rz0_module_trust::{
    SignatureEnvelope, StagingPlan, TrustedTestKey, validate_staging_plan,
    validate_staging_plan_with_signature, verify_detached_signature,
};
use support::staging_fs::{SimulationRoot, copy_fixture_input, simulate_staging};

#[test]
fn verifies_then_stages_fixture_bytes_with_atomic_publication() {
    let plan = staging_plan();
    let verification = verify_detached_signature(&envelope(), &trusted_key());
    let validation = validate_staging_plan_with_signature(&plan, &verification);
    assert!(validation.valid, "{:?}", validation.errors);

    let root = SimulationRoot::new();
    copy_fixture_input(root.path());
    let receipt = simulate_staging(root.path(), &plan).expect("staging simulation");
    assert!(receipt.simulation_only);
    assert!(!receipt.production_writes_attempted);
    assert_eq!(receipt.published_root, plan.publication_root);
    assert_eq!(receipt.files.len(), 2);
    assert!(!root.path().join(&plan.staging_root).exists());
    assert!(root.path().join(&plan.publication_root).is_dir());
    assert_eq!(
        fs::read(root.path().join(&plan.publication_root).join("payload.txt"))
            .expect("published payload"),
        b"Synthetic immutable staging payload.\n"
    );
}

#[test]
fn tampered_input_preserves_failed_stage_and_never_publishes() {
    let plan = staging_plan();
    let root = SimulationRoot::new();
    copy_fixture_input(root.path());
    let input_payload = root.path().join(&plan.source_root).join("payload.txt");
    fs::write(&input_payload, b"tampered fixture\n").expect("tamper fixture");

    let error = simulate_staging(root.path(), &plan).expect_err("hash mismatch");
    assert!(error.contains("hash or size mismatch"));
    assert!(input_payload.exists());
    assert!(!root.path().join(&plan.publication_root).exists());
    assert!(root.path().join(&plan.staging_root).exists());
}

#[test]
fn occupied_publication_destination_fails_before_staging() {
    let plan = staging_plan();
    let root = SimulationRoot::new();
    copy_fixture_input(root.path());
    fs::create_dir_all(root.path().join(&plan.publication_root)).expect("occupied destination");

    let error = simulate_staging(root.path(), &plan).expect_err("destination conflict");
    assert!(error.contains("publication destination already exists"));
    assert!(!root.path().join(&plan.staging_root).exists());
}

#[test]
fn staging_contract_rejects_path_identity_and_signature_proof_drift() {
    let mut plan = staging_plan();
    plan.files[1].path = "../outside".to_string();
    plan.publication_root = "published/first-party.other/0.1.0".to_string();
    plan.signature_proof.verified = false;
    for index in 0..8 {
        let mut oversized_total = plan.files[1].clone();
        oversized_total.path = format!("payload-{index}.bin");
        oversized_total.size_bytes = 64 * 1024 * 1024;
        plan.files.push(oversized_total);
    }
    let validation = validate_staging_plan(&plan);
    assert!(!validation.valid);
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("file path"))
    );
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("publication_root"))
    );
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("signature proof"))
    );
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("total bytes"))
    );
}

#[cfg(unix)]
#[test]
fn symlinked_source_file_fails_closed() {
    use std::os::unix::fs::symlink;

    let plan = staging_plan();
    let root = SimulationRoot::new();
    copy_fixture_input(root.path());
    let payload = root.path().join(&plan.source_root).join("payload.txt");
    fs::remove_file(&payload).expect("remove fixture payload");
    symlink("rz0-module.json", &payload).expect("fixture symlink");

    let error = simulate_staging(root.path(), &plan).expect_err("symlink rejection");
    assert!(error.contains("symlink"));
    assert!(!root.path().join(&plan.publication_root).exists());
}

fn staging_plan() -> StagingPlan {
    serde_json::from_str(include_str!("fixtures/staging/valid-plan.json"))
        .expect("valid staging plan")
}

fn envelope() -> SignatureEnvelope {
    serde_json::from_str(include_str!("fixtures/valid-envelope.json")).expect("valid envelope")
}

fn trusted_key() -> TrustedTestKey {
    serde_json::from_str(include_str!("fixtures/trusted-test-key.json")).expect("trusted key")
}
