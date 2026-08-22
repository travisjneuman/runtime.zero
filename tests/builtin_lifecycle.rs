use std::fs;
use std::path::PathBuf;

use runtime_zero::builtin_modules::{
    BuiltinLifecycleMode, BuiltinLifecycleRequest, BuiltinOperation, lifecycle_report,
    require_enabled,
};
use runtime_zero::store_init::{StoreInitMode, StoreInitOptions, store_init_report};

#[test]
fn built_in_disable_and_enable_are_real_local_availability_transitions() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/rz0-builtin-lifecycle");
    if root.exists() {
        fs::remove_dir_all(&root).expect("remove prior owned fixture");
    }
    let init = store_init_report(
        &["store".to_string(), "init".to_string()],
        StoreInitOptions::with_store_root(StoreInitMode::Apply, root.clone()),
    );
    assert!(!init.status.is_blocked(), "{:#?}", init.steps);

    let disable = lifecycle_report(&BuiltinLifecycleRequest {
        operation: BuiltinOperation::Disable,
        module_id: "first-party.updater".to_string(),
        store_root: Some(root.clone()),
        mode: BuiltinLifecycleMode::DryRun,
    });
    assert!(disable.valid, "{:?}", disable.errors);
    let challenge = disable.challenge.as_ref().expect("disable challenge");
    let applied = lifecycle_report(&BuiltinLifecycleRequest {
        operation: BuiltinOperation::Disable,
        module_id: "first-party.updater".to_string(),
        store_root: Some(root.clone()),
        mode: BuiltinLifecycleMode::Apply {
            challenge_issued_unix_seconds: challenge.issued_unix_seconds,
            confirmation: challenge.expected_phrase.clone(),
        },
    });
    assert!(applied.valid, "{:?}", applied.errors);
    assert!(require_enabled("first-party.updater", Some(&root)).is_err());

    let enable = lifecycle_report(&BuiltinLifecycleRequest {
        operation: BuiltinOperation::Enable,
        module_id: "first-party.updater".to_string(),
        store_root: Some(root.clone()),
        mode: BuiltinLifecycleMode::DryRun,
    });
    let challenge = enable.challenge.as_ref().expect("enable challenge");
    let applied = lifecycle_report(&BuiltinLifecycleRequest {
        operation: BuiltinOperation::Enable,
        module_id: "first-party.updater".to_string(),
        store_root: Some(root.clone()),
        mode: BuiltinLifecycleMode::Apply {
            challenge_issued_unix_seconds: challenge.issued_unix_seconds,
            confirmation: challenge.expected_phrase.clone(),
        },
    });
    assert!(applied.valid, "{:?}", applied.errors);
    assert!(require_enabled("first-party.updater", Some(&root)).is_ok());
    fs::remove_dir_all(root).expect("remove owned fixture");
}
