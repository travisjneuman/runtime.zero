use runtime_zero::{ExitCode, dashboard_cli, run};

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
    assert!(out.contains("contract: foundation_diagnostics"));
    assert!(out.contains("read_only: true"));
    assert!(out.contains("production_execution_authorized: false"));
    assert!(out.contains("module_execution_policy: blocked"));
    assert!(!out.contains("current_dir:"));
}

#[test]
fn doctor_json_is_versioned_and_private_by_default() {
    let (code, out, err) = run(["doctor", "--format", "json"]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    let value: serde_json::Value = serde_json::from_str(&out).expect("doctor JSON");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["contract"], "foundation_diagnostics");
    assert_eq!(value["read_only"], true);
    assert_eq!(value["writes_attempted"], false);
    assert!(
        value["configuration_sha256"]
            .as_str()
            .is_some_and(|digest| {
                digest.len() == 64
                    && digest
                        .bytes()
                        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
            })
    );
    assert_eq!(value["privacy"]["hostname_included"], false);
    assert_eq!(value["privacy"]["current_directory_included"], false);
    assert!(!out.contains("/Users/"));
}

#[test]
fn subcommand_help_is_scriptable_and_successful() {
    let (code, out, err) = run(["modules", "--help"]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("rz0 modules install --dry-run"));
    assert!(out.contains("modules are not executed or fetched"));

    let (code, out, err) = run(["store", "--help"]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("rz0 store status [--store-root <path>]"));
    assert!(out.contains("store status and plan are read-only"));
}

#[test]
fn root_help_mentions_store_root_override() {
    let (code, out, err) = run(["--help"]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("store status [--store-root <path>]"));
    assert!(out.contains("--color auto|always|never"));
    assert!(out.contains("rz0 --tui"));
    assert!(out.contains("rz0 apps [--format json]"));
    assert!(out.contains("rz0 uninstall plan <installed-software-id>"));
}

#[test]
fn global_color_flag_is_accepted_without_changing_subcommand_contract() {
    let (code, out, err) = run(["store", "status", "--color=never"]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("mode: read-only"));

    let (code, out, err) = run(["--color", "always", "modules", "--help"]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("modules are not executed or fetched"));
}

#[test]
fn invalid_global_color_flag_fails_before_actions() {
    let (code, out, err) = run(["--color=neon"]);
    assert_eq!(code, ExitCode::Usage);
    assert!(out.is_empty());
    assert!(err.contains("unsupported --color value"));
}

#[test]
fn dashboard_json_reports_versioned_read_only_contract() {
    let (code, out, err) = dashboard_cli::dashboard_json();
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    let value: serde_json::Value = serde_json::from_str(&out).expect("dashboard json");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["contract"], "foundation_dashboard");
    assert_eq!(value["read_only"], true);
    assert_eq!(value["writes_attempted"], false);
    assert_eq!(value["installed_module_count"], 0);
    assert_eq!(value["installed_software_count"], 0);
    assert_eq!(value["inventory_status"], "private summary");
    assert!(!out.contains("[APP]"));
    assert!(!out.contains("\u{1b}["));
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
fn inventory_source_manifest_declares_explicit_sensitive_reads() {
    let (code, out, err) = run([
        "modules",
        "validate",
        "modules/inventory/rz0-module.json",
        "--format",
        "json",
    ]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    let value: serde_json::Value = serde_json::from_str(&out).expect("manifest JSON");
    assert_eq!(value["valid"], true);
    let permissions = &value["manifest"]["permissions"];
    assert!(
        permissions["explicit_grants"]
            .as_array()
            .is_some_and(|grants| grants.iter().any(|grant| grant == "exact_command_probe"))
    );
    assert!(
        permissions["explicit_grants"]
            .as_array()
            .is_some_and(|grants| grants
                .iter()
                .any(|grant| grant == "application_registry_read"))
    );
    assert!(
        permissions["explicit_grants"]
            .as_array()
            .is_some_and(|grants| grants
                .iter()
                .any(|grant| grant == "application_filesystem_read"))
    );
}

#[test]
fn every_first_party_source_manifest_validates_without_execution() {
    for family in [
        "updater",
        "uninstall",
        "leftovers",
        "cache",
        "security-integrity",
        "report-export",
    ] {
        let manifest = format!("modules/{family}/rz0-module.json");
        let (code, out, err) = run(["modules", "validate", &manifest, "--format", "json"]);
        assert_eq!(code, ExitCode::Ok, "{family}: {err}");
        assert!(err.is_empty());
        let value: serde_json::Value = serde_json::from_str(&out).expect("manifest JSON");
        assert_eq!(value["valid"], true, "{family}: {out}");
        assert_eq!(value["manifest"]["status"], "planned");
        assert_eq!(
            value["manifest"]["permissions"]["declared"],
            serde_json::json!([])
        );
    }
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
    assert!(out.contains("contract: inventory_report"));
    assert!(out.contains("writes_attempted: no"));
    assert!(out.contains("no system changes were attempted"));
}

#[test]
fn scan_json_exposes_live_private_read_only_inventory_contract() {
    let (code, out, err) = run(["scan", "--dry-run", "--format", "json"]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());

    let value: serde_json::Value = serde_json::from_str(&out).expect("inventory json");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["contract"], "inventory_report");
    assert_eq!(value["read_only"], true);
    assert_eq!(value["writes_attempted"], false);
    assert_eq!(value["host"]["hostname_included"], false);
    assert_eq!(value["host"]["current_user_included"], false);
    assert!(
        value["sources"]
            .as_array()
            .is_some_and(|sources| !sources.is_empty())
    );
    assert_eq!(value["path_values_redacted"], true);
    assert!(value["summary"]["source_count"].as_u64().unwrap_or(0) > 0);
    assert!(!out.contains("\u{1b}["));
}

#[test]
fn apps_command_exposes_path_free_live_software_catalog() {
    let (code, out, err) = run(["apps", "--format", "json"]);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    let value: serde_json::Value = serde_json::from_str(&out).expect("apps json");
    assert_eq!(value["contract"], "installed_software_catalog");
    assert_eq!(value["read_only"], true);
    assert_eq!(value["writes_attempted"], false);
    assert!(value["source_count"].as_u64().unwrap_or(0) > 0);
    assert!(!out.contains("install_location"));
    assert!(!out.contains("/Users/"));
}
