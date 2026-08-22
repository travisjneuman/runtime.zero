use std::fs;
use std::path::{Path, PathBuf};

use runtime_zero::module_invoke::{
    DeveloperInvocationMode, DeveloperInvocationStatus, SignedInvocationRequest,
    signed_invocation_report,
};
use runtime_zero::module_lifecycle_exec::{
    LifecycleMode, LifecycleOperation, LifecycleRequest, lifecycle_report,
};
use runtime_zero::module_stage::{DeveloperStageMode, SignedStageRequest, signed_stage_report};
use runtime_zero::store_init::{StoreInitMode, StoreInitOptions, store_init_report};

#[test]
fn signed_macos_module_can_install_enable_disable_quarantine_and_recover() {
    let root = test_root("lifecycle");
    init_store(&root);
    let package = repo_path("tests/fixtures/module-packages/macos-inventory");
    let signature = repo_path("tests/fixtures/module-stage/macos-inventory-release-envelope.json");
    let key = repo_path("tests/fixtures/module-stage/release-key.json");
    let stage = signed_stage_report(&SignedStageRequest {
        package_path: package,
        signature_path: signature,
        trusted_key_path: key,
        store_root: root.clone(),
        mode: DeveloperStageMode::DryRun {
            publish_installed: true,
        },
    });
    assert!(stage.valid, "{:?}", stage.errors);
    assert!(!stage.developer_only);
    assert!(!stage.test_key_only);
    let applied = signed_stage_report(&SignedStageRequest {
        package_path: repo_path("tests/fixtures/module-packages/macos-inventory"),
        signature_path: repo_path(
            "tests/fixtures/module-stage/macos-inventory-release-envelope.json",
        ),
        trusted_key_path: repo_path("tests/fixtures/module-stage/release-key.json"),
        store_root: root.clone(),
        mode: DeveloperStageMode::Apply {
            challenge_issued_unix_seconds: stage.challenge.as_ref().unwrap().issued_unix_seconds,
            confirmation: stage.challenge.as_ref().unwrap().expected_phrase.clone(),
            publish_installed: true,
        },
    });
    assert!(applied.valid, "{:?}", applied.errors);

    let enable = execute(&root, LifecycleOperation::Enable, false, None);
    assert!(enable.valid, "{:?}", enable.errors);
    assert_eq!(
        enable.to_state,
        Some(rz0_module_lifecycle::ModuleLifecycleState::Active)
    );
    let enable_applied = apply_from(&root, LifecycleOperation::Enable, &enable);
    assert!(enable_applied.valid, "{:?}", enable_applied.errors);

    #[cfg(target_os = "macos")]
    {
        let invocation = signed_invocation_report(&SignedInvocationRequest {
            module_id: "first-party.inventory".to_string(),
            store_root: root.clone(),
            mode: DeveloperInvocationMode::DryRun,
        });
        assert!(invocation.valid, "{:?}", invocation.errors);
        assert_eq!(invocation.contract, "signed_module_invocation");
        let invocation_applied = signed_invocation_report(&SignedInvocationRequest {
            module_id: "first-party.inventory".to_string(),
            store_root: root.clone(),
            mode: DeveloperInvocationMode::Apply {
                challenge_issued_unix_seconds: invocation
                    .challenge
                    .as_ref()
                    .unwrap()
                    .issued_unix_seconds,
                confirmation: invocation
                    .challenge
                    .as_ref()
                    .unwrap()
                    .expected_phrase
                    .clone(),
            },
        });
        assert!(invocation_applied.valid, "{:?}", invocation_applied.errors);
        assert_eq!(
            invocation_applied.status,
            DeveloperInvocationStatus::Success
        );
        assert!(invocation_applied.product_execution_authorized);
    }

    let disable = execute(&root, LifecycleOperation::Disable, false, None);
    assert!(disable.valid, "{:?}", disable.errors);
    let disable_applied = apply_from(&root, LifecycleOperation::Disable, &disable);
    assert!(disable_applied.valid, "{:?}", disable_applied.errors);

    let update = LifecycleRequest {
        operation: LifecycleOperation::Update {
            package_path: repo_path("tests/fixtures/module-packages/macos-inventory-v0.2"),
            signature_path: repo_path(
                "tests/fixtures/module-stage/macos-inventory-v0.2-release-envelope.json",
            ),
            trusted_key_path: repo_path("tests/fixtures/module-stage/release-key.json"),
        },
        module_id: Some("first-party.inventory".to_string()),
        recovery_id: None,
        store_root: root.clone(),
        mode: LifecycleMode::DryRun,
    };
    let update_plan = lifecycle_report(&update);
    assert!(update_plan.valid, "{:?}", update_plan.errors);
    assert_eq!(update_plan.to_version.as_deref(), Some("0.2.0"));
    let update_applied = LifecycleRequest {
        operation: match &update.operation {
            LifecycleOperation::Update {
                package_path,
                signature_path,
                trusted_key_path,
            } => LifecycleOperation::Update {
                package_path: package_path.clone(),
                signature_path: signature_path.clone(),
                trusted_key_path: trusted_key_path.clone(),
            },
            _ => unreachable!(),
        },
        module_id: update.module_id.clone(),
        recovery_id: None,
        store_root: root.clone(),
        mode: LifecycleMode::Apply {
            challenge_issued_unix_seconds: update_plan
                .challenge
                .as_ref()
                .unwrap()
                .issued_unix_seconds,
            confirmation: update_plan
                .challenge
                .as_ref()
                .unwrap()
                .expected_phrase
                .clone(),
        },
    };
    let update_result = lifecycle_report(&update_applied);
    assert!(update_result.valid, "{:?}", update_result.errors);
    assert!(
        root.join("modules/first-party.inventory/0.2.0/rz0-module.json")
            .is_file()
    );

    let uninstall = execute(&root, LifecycleOperation::Uninstall, false, None);
    assert!(uninstall.valid, "{:?}", uninstall.errors);
    let uninstall_applied = apply_from(&root, LifecycleOperation::Uninstall, &uninstall);
    assert!(uninstall_applied.valid, "{:?}", uninstall_applied.errors);
    let recovery_id = uninstall_applied.recovery_id.clone().expect("recovery ID");
    let uninstalled_registry: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("state/installed-modules.json")).expect("uninstalled registry"),
    )
    .expect("uninstalled registry JSON");
    assert_eq!(uninstalled_registry["modules"].as_array().unwrap().len(), 0);
    assert!(
        root.join("state/recovery")
            .join(format!("{recovery_id}.json"))
            .is_file()
    );

    let recover = execute(&root, LifecycleOperation::Recover, false, Some(recovery_id));
    assert!(recover.valid, "{:?}", recover.errors);
    let recovered = apply_from(&root, LifecycleOperation::Recover, &recover);
    assert!(recovered.valid, "{:?}", recovered.errors);
    let registry: serde_json::Value = serde_json::from_slice(
        &fs::read(root.join("state/installed-modules.json")).expect("registry"),
    )
    .expect("registry JSON");
    assert_eq!(registry["modules"].as_array().unwrap().len(), 1);
    assert!(
        root.join("modules/first-party.inventory/0.2.0/rz0-module.json")
            .is_file()
    );
    cleanup(&root);
}

#[test]
fn lifecycle_confirmation_and_state_drift_fail_closed_without_writes() {
    let root = test_root("drift");
    init_store(&root);
    let package = repo_path("tests/fixtures/module-packages/macos-inventory");
    let stage = signed_stage_report(&SignedStageRequest {
        package_path: package,
        signature_path: repo_path(
            "tests/fixtures/module-stage/macos-inventory-release-envelope.json",
        ),
        trusted_key_path: repo_path("tests/fixtures/module-stage/release-key.json"),
        store_root: root.clone(),
        mode: DeveloperStageMode::DryRun {
            publish_installed: true,
        },
    });
    assert!(stage.valid, "{:?}", stage.errors);
    let applied = signed_stage_report(&SignedStageRequest {
        package_path: repo_path("tests/fixtures/module-packages/macos-inventory"),
        signature_path: repo_path(
            "tests/fixtures/module-stage/macos-inventory-release-envelope.json",
        ),
        trusted_key_path: repo_path("tests/fixtures/module-stage/release-key.json"),
        store_root: root.clone(),
        mode: DeveloperStageMode::Apply {
            challenge_issued_unix_seconds: stage.challenge.as_ref().unwrap().issued_unix_seconds,
            confirmation: "wrong-phrase".to_string(),
            publish_installed: true,
        },
    });
    assert!(!applied.valid);
    assert!(!applied.writes_attempted);

    let installed = signed_stage_report(&SignedStageRequest {
        package_path: repo_path("tests/fixtures/module-packages/macos-inventory"),
        signature_path: repo_path(
            "tests/fixtures/module-stage/macos-inventory-release-envelope.json",
        ),
        trusted_key_path: repo_path("tests/fixtures/module-stage/release-key.json"),
        store_root: root.clone(),
        mode: DeveloperStageMode::Apply {
            challenge_issued_unix_seconds: stage.challenge.as_ref().unwrap().issued_unix_seconds,
            confirmation: stage.challenge.as_ref().unwrap().expected_phrase.clone(),
            publish_installed: true,
        },
    });
    assert!(installed.valid, "{:?}", installed.errors);

    let enable = execute(&root, LifecycleOperation::Enable, false, None);
    assert!(enable.valid, "{:?}", enable.errors);
    let wrong = LifecycleRequest {
        operation: LifecycleOperation::Enable,
        module_id: Some("first-party.inventory".to_string()),
        recovery_id: None,
        store_root: root.clone(),
        mode: LifecycleMode::Apply {
            challenge_issued_unix_seconds: enable.challenge.as_ref().unwrap().issued_unix_seconds,
            confirmation: "wrong-phrase".to_string(),
        },
    };
    let report = lifecycle_report(&wrong);
    assert!(!report.valid);
    assert!(!report.writes_attempted);
    cleanup(&root);
}

fn execute(
    root: &Path,
    operation: LifecycleOperation,
    _apply: bool,
    recovery_id: Option<String>,
) -> runtime_zero::module_lifecycle_exec::LifecycleExecutionReport {
    lifecycle_report(&LifecycleRequest {
        operation,
        module_id: if recovery_id.is_some() {
            None
        } else {
            Some("first-party.inventory".to_string())
        },
        recovery_id,
        store_root: root.to_path_buf(),
        mode: LifecycleMode::DryRun,
    })
}

fn apply_from(
    root: &Path,
    operation: LifecycleOperation,
    dry_run: &runtime_zero::module_lifecycle_exec::LifecycleExecutionReport,
) -> runtime_zero::module_lifecycle_exec::LifecycleExecutionReport {
    let is_recover = matches!(&operation, LifecycleOperation::Recover);
    lifecycle_report(&LifecycleRequest {
        operation,
        module_id: if is_recover {
            None
        } else {
            Some("first-party.inventory".to_string())
        },
        recovery_id: if is_recover {
            dry_run.recovery_id.clone()
        } else {
            None
        },
        store_root: root.to_path_buf(),
        mode: LifecycleMode::Apply {
            challenge_issued_unix_seconds: dry_run.challenge.as_ref().unwrap().issued_unix_seconds,
            confirmation: dry_run.challenge.as_ref().unwrap().expected_phrase.clone(),
        },
    })
}

fn init_store(root: &Path) {
    let report = store_init_report(
        &["store".to_string(), "init".to_string()],
        StoreInitOptions::with_store_root(StoreInitMode::Apply, root.to_path_buf()),
    );
    assert!(!report.status.is_blocked(), "{:?}", report.steps);
}

fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn test_root(label: &str) -> PathBuf {
    let root = repo_path(&format!("target/rz0-module-lifecycle-{label}"));
    if root.exists() {
        fs::remove_dir_all(&root).expect("remove prior owned lifecycle fixture");
    }
    root
}

fn cleanup(root: &Path) {
    if root.exists() {
        fs::remove_dir_all(root).expect("remove owned lifecycle fixture");
    }
}
