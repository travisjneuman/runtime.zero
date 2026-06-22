use runtime_zero::{ExitCode, run};

#[test]
fn version_includes_brand_and_command() {
    let (code, out, err) = run(["--version"]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("runtime.zero"));
    assert!(out.contains("rz0"));
}

#[test]
fn doctor_is_read_only_bootstrap_diagnostic() {
    let (code, out, err) = run(["doctor"]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("mutation_capability: explicit_store_init_only"));
    assert!(out.contains("module_mutation_capability: disabled"));
    assert!(out.contains("github_actions: not configured"));
}

#[test]
fn modules_show_planned_leftover_scanner() {
    let (code, out, err) = run(["modules"]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("installed modules:\n  none"));
    assert!(out.contains("first-party.leftovers"));
    assert!(out.contains("planned"));
}

#[test]
fn modules_json_shows_empty_installed_registry() {
    let (code, out, err) = run(["modules", "--format", "json"]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("\"schema_version\": 1"));
    assert!(out.contains("\"installed_modules\": []"));
    assert!(out.contains("\"remote_execution_allowed\": false"));
}

#[test]
fn modules_reject_unknown_options() {
    let (code, out, err) = run(["modules", "--install"]);
    assert_eq!(code, ExitCode::Usage);
    assert!(out.is_empty());
    assert!(err.contains("unsupported modules option"));
}

#[test]
fn modules_validate_rejects_missing_manifest() {
    let (code, out, err) = run(["modules", "validate", "missing-rz0-module.json"]);
    assert_eq!(code, ExitCode::Usage);
    assert!(err.is_empty());
    assert!(out.contains("status: invalid"));
}

#[test]
fn modules_validate_accepts_fixture_package_integrity() {
    let (code, out, err) = run([
        "modules",
        "validate",
        "tests/fixtures/module-packages/valid-inventory/rz0-module.json",
    ]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("status: valid"));
}

#[test]
fn modules_validate_rejects_fixture_hash_mismatch() {
    let (code, out, err) = run([
        "modules",
        "validate",
        "tests/fixtures/module-packages/hash-mismatch/rz0-module.json",
    ]);
    assert_eq!(code, ExitCode::Usage);
    assert!(err.is_empty());
    assert!(out.contains("hash mismatch"));
}

#[test]
fn modules_install_dry_run_plans_valid_fixture_without_writes() {
    let (code, out, err) = run([
        "modules",
        "install",
        "--dry-run",
        "tests/fixtures/module-packages/valid-inventory",
    ]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("status: valid"));
    assert!(out.contains("writes_attempted: no"));
    assert!(out.contains("copy_package_file"));
}

#[test]
fn modules_install_dry_run_rejects_bad_fixture() {
    let (code, out, err) = run([
        "modules",
        "install",
        "--dry-run",
        "tests/fixtures/module-packages/hash-mismatch",
    ]);
    assert_eq!(code, ExitCode::Usage);
    assert!(err.is_empty());
    assert!(out.contains("status: invalid"));
    assert!(out.contains("hash mismatch"));
}

#[test]
fn modules_install_requires_dry_run() {
    let (code, out, err) = run([
        "modules",
        "install",
        "tests/fixtures/module-packages/valid-inventory",
    ]);
    assert_eq!(code, ExitCode::Usage);
    assert!(out.is_empty());
    assert!(err.contains("dry-run only"));
}

#[test]
fn store_plan_reports_read_only_contract() {
    let (code, out, err) = run(["store", "plan"]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("writes_attempted: no"));
    assert!(out.contains("registry_path:"));
    assert!(out.contains("launch_mode: cli_subcommand"));
}

#[test]
fn store_init_dry_run_is_scriptable() {
    let (code, out, err) = run(["store", "init", "--dry-run"]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("mode: dry-run"));
    assert!(out.contains("writes_attempted: no"));
}

#[test]
fn store_plan_json_reports_contract_shape() {
    let (code, out, err) = run(["store", "plan", "--format", "json"]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("\"store_schema_version\": 1"));
    assert!(out.contains("\"writes_attempted\": false"));
    assert!(out.contains("\"launch_mode\": \"cli_subcommand\""));
}

#[test]
fn store_status_reports_read_only_inventory() {
    let (code, out, err) = run(["store", "status"]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("writes_attempted: no"));
    assert!(out.contains("overall_state:"));
    assert!(out.contains("registry_path:"));
}

#[test]
fn store_status_json_reports_inventory_shape() {
    let (code, out, err) = run(["store", "status", "--format", "json"]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("\"command\": \"store status\""));
    assert!(out.contains("\"writes_attempted\": false"));
    assert!(out.contains("\"overall_state\""));
    assert!(out.contains("\"transactions_dir\""));
}

#[test]
fn scan_requires_dry_run() {
    let (code, out, err) = run(["scan"]);
    assert_eq!(code, ExitCode::Usage);
    assert!(out.is_empty());
    assert!(err.contains("--dry-run"));
}

#[test]
fn scan_dry_run_attempts_no_changes() {
    let (code, out, err) = run(["scan", "--dry-run"]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("mode: dry-run"));
    assert!(out.contains("no system changes were attempted"));
}
