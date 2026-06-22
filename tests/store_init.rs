use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use runtime_zero::installed_registry::InstalledRegistryState;
use runtime_zero::store_init::{
    StoreInitMode, StoreInitOptions, StoreInitStatus, StoreInitStepState, store_init_report,
};

#[test]
fn dry_run_does_not_create_temp_store_root() {
    let root = unique_temp_root();
    let report = store_init_report(
        &["init".to_string(), "--dry-run".to_string()],
        StoreInitOptions::with_store_root(StoreInitMode::DryRun, root.clone()),
    );
    assert_eq!(report.status, StoreInitStatus::Ready);
    assert!(!report.writes_attempted);
    assert!(!root.exists());
}

#[test]
fn apply_creates_only_store_scaffold_under_temp_root() {
    let root = unique_temp_root();
    let report = store_init_report(
        &["init".to_string(), "--yes".to_string()],
        StoreInitOptions::with_store_root(StoreInitMode::Apply, root.clone()),
    );
    assert_eq!(report.status, StoreInitStatus::Applied);
    assert!(report.writes_attempted);
    assert!(root.join("state").join("installed-modules.json").is_file());
    assert!(root.join("state").join("store-init.json").is_file());
    assert!(root.join("state").join("transactions").is_dir());
    assert!(root.join("state").join("receipts").is_dir());
    cleanup(root);
}

#[test]
fn apply_is_idempotent_after_valid_initialization() {
    let root = unique_temp_root();
    let first = store_init_report(
        &["init".to_string(), "--yes".to_string()],
        StoreInitOptions::with_store_root(StoreInitMode::Apply, root.clone()),
    );
    assert_eq!(first.status, StoreInitStatus::Applied);
    let second = store_init_report(
        &["init".to_string(), "--yes".to_string()],
        StoreInitOptions::with_store_root(StoreInitMode::Apply, root.clone()),
    );
    assert_eq!(second.status, StoreInitStatus::AlreadyInitialized);
    assert!(!second.writes_attempted);
    assert_eq!(second.registry_status, InstalledRegistryState::Valid);
    cleanup(root);
}

#[test]
fn invalid_existing_registry_blocks_without_repair() {
    let root = unique_temp_root();
    fs::create_dir_all(root.join("state")).expect("state dir should be created for test");
    fs::write(
        root.join("state").join("installed-modules.json"),
        "{ bad json",
    )
    .expect("invalid registry should be written for test");
    let report = store_init_report(
        &["init".to_string(), "--yes".to_string()],
        StoreInitOptions::with_store_root(StoreInitMode::Apply, root.clone()),
    );
    assert_eq!(report.status, StoreInitStatus::Blocked);
    assert!(!report.writes_attempted);
    assert!(report.steps.iter().any(|step| {
        step.state == StoreInitStepState::Blocked && step.path.ends_with("installed-modules.json")
    }));
    cleanup(root);
}

#[test]
fn invalid_existing_init_marker_blocks_without_repair() {
    let root = unique_temp_root();
    fs::create_dir_all(root.join("state")).expect("state dir should be created for test");
    fs::write(
        root.join("state").join("installed-modules.json"),
        "{\n  \"schema_version\": 1,\n  \"modules\": []\n}\n",
    )
    .expect("valid registry should be written for test");
    fs::write(root.join("state").join("store-init.json"), "{}")
        .expect("invalid marker should be written for test");
    let report = store_init_report(
        &["init".to_string(), "--yes".to_string()],
        StoreInitOptions::with_store_root(StoreInitMode::Apply, root.clone()),
    );
    assert_eq!(report.status, StoreInitStatus::Blocked);
    assert!(!report.writes_attempted);
    assert!(report.steps.iter().any(|step| {
        step.state == StoreInitStepState::Blocked && step.path.ends_with("store-init.json")
    }));
    cleanup(root);
}

fn unique_temp_root() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "rz0-store-init-test-{}-{nanos}",
        std::process::id()
    ))
}

fn cleanup(root: PathBuf) {
    if root.exists() {
        fs::remove_dir_all(root).expect("temp root cleanup should succeed");
    }
}
