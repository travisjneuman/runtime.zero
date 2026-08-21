use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use runtime_zero::module_stage::{
    DeveloperStageMode, DeveloperStageRequest, developer_stage_report,
};
use runtime_zero::store_init::{StoreInitMode, StoreInitOptions, store_init_report};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn developer_trial_stages_verified_bytes_without_publishing_installation() {
    let root = temp_root("developer-trial");
    let init = store_init_report(
        &["store".to_string(), "init".to_string()],
        StoreInitOptions::with_store_root(StoreInitMode::Apply, root.clone()),
    );
    assert!(!init.status.is_blocked(), "{:?}", init.steps);

    let request = request(&root, DeveloperStageMode::DryRun);
    let dry_run = developer_stage_report(&request);
    assert!(dry_run.valid, "{:?}", dry_run.errors);
    assert!(dry_run.dry_run);
    assert!(!dry_run.writes_attempted);
    assert!(!dry_run.product_execution_authorized);
    assert_eq!(
        dry_run.source_manifest_path,
        "<local-package>/rz0-module.json"
    );
    let challenge = dry_run.challenge.expect("dry-run challenge");

    let applied = developer_stage_report(&request_with_mode(
        &root,
        DeveloperStageMode::Apply {
            challenge_issued_unix_seconds: challenge.issued_unix_seconds,
            confirmation: challenge.expected_phrase,
        },
    ));
    assert!(applied.valid, "{:?}", applied.errors);
    assert!(!applied.dry_run);
    assert!(applied.writes_attempted);
    assert!(!applied.activation_authorized);
    assert!(!applied.invocation_authorized);
    assert!(!applied.product_execution_authorized);

    let module_root = root
        .join("modules")
        .join("first-party.inventory-fixture")
        .join("0.1.0");
    assert!(module_root.join("rz0-module.json").is_file());
    assert_eq!(
        fs::read_to_string(module_root.join("payload.txt")).unwrap(),
        "runtime.zero fixture payload\n"
    );
    assert!(root.join("state/staging-receipts").is_dir());
    assert!(root.join("state/receipts").is_dir());
    let registry: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("state/installed-modules.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(registry["modules"].as_array().map(Vec::len), Some(0));

    remove_owned_root(&root);
}

fn request(root: &Path, mode: DeveloperStageMode) -> DeveloperStageRequest {
    request_with_mode(root, mode)
}

fn request_with_mode(root: &Path, mode: DeveloperStageMode) -> DeveloperStageRequest {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    DeveloperStageRequest {
        package_path: repository.join("tests/fixtures/module-packages/valid-inventory"),
        signature_path: repository.join("tests/fixtures/module-stage/valid-envelope.json"),
        trusted_key_path: repository.join("tests/fixtures/module-stage/trusted-test-key.json"),
        store_root: root.to_path_buf(),
        mode,
    }
}

fn temp_root(label: &str) -> PathBuf {
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("rz0-{label}-{}-{sequence}", std::process::id()));
    fs::create_dir(&path).expect("create owned temp root");
    #[cfg(unix)]
    std::fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o700))
        .expect("restrict owned temp root");
    path
}

fn remove_owned_root(root: &PathBuf) {
    let temp = fs::canonicalize(std::env::temp_dir()).expect("canonical temp root");
    let root = fs::canonicalize(root).expect("canonical owned root");
    assert_eq!(root.parent(), Some(temp.as_path()));
    assert!(
        root.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("rz0-developer-trial-"))
    );
    fs::remove_dir_all(root).expect("remove owned temp root");
}
