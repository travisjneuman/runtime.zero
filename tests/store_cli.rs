use std::path::Path;

use runtime_zero::ExitCode;
use runtime_zero::store_cli::store_command;
use serde_json::Value;

#[test]
fn store_plan_text_reports_read_only_contract() {
    let args = vec!["plan".to_string()];
    let (code, out, err) = store_command(&args);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("mode: read-only"));
    assert!(out.contains("writes_attempted: no"));
    assert!(out.contains("registry_path:"));
    assert!(out.contains("forbidden_path_classes:"));
    assert!(out.contains("launch_mode: cli_subcommand"));
}

#[test]
fn store_plan_json_reports_stable_contract_shape() {
    let args = vec![
        "plan".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];
    let (code, out, err) = store_command(&args);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    let json: Value = serde_json::from_str(&out).expect("store plan should be JSON");
    assert_eq!(json["store"]["store_schema_version"], 1);
    assert_eq!(json["writes_attempted"], false);
    assert_eq!(json["launch_context"]["launch_mode"], "cli_subcommand");
    assert!(json["store"]["registry_path"].is_string());
    assert!(json["store"]["receipt_path"].is_string());
    assert!(json["store"]["forbidden_path_classes"].is_array());
}

#[test]
fn store_status_text_reports_read_only_inventory() {
    let args = vec!["status".to_string()];
    let (code, out, err) = store_command(&args);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("mode: read-only"));
    assert!(out.contains("writes_attempted: no"));
    assert!(out.contains("overall_state:"));
    assert!(out.contains("registry_path:"));
    assert!(out.contains("registry:"));
    assert!(out.contains("installed_module_count:"));
    assert!(out.contains("receipts:"));
    assert!(out.contains("checked_count:"));
    assert!(out.contains("launch_mode: cli_subcommand"));
}

#[test]
fn store_status_json_reports_stable_inventory_shape() {
    let args = vec![
        "status".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];
    let (code, out, err) = store_command(&args);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    let json: Value = serde_json::from_str(&out).expect("store status should be JSON");
    assert_eq!(json["command"], "store status");
    assert_eq!(json["writes_attempted"], false);
    assert!(json["overall_state"].is_string());
    assert!(json["store"]["registry_path"].is_string());
    assert!(json["registry"]["installed_module_count"].is_number());
    assert!(json["receipts"]["checked_count"].is_number());
    assert!(
        json["paths"]
            .as_array()
            .unwrap()
            .iter()
            .any(|path| { path["role"] == "transactions_dir" })
    );
    assert!(
        json["paths"]
            .as_array()
            .unwrap()
            .iter()
            .any(|path| { path["role"] == "receipts_dir" })
    );
}

#[test]
fn store_status_json_accepts_read_only_store_root_fixture() {
    let args = store_status_fixture_args("valid-registry-valid-receipt", true);
    let (code, out, err) = store_command(&args);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    let json: Value = serde_json::from_str(&out).expect("store status should be JSON");
    assert_eq!(json["store_root_override"].is_string(), true);
    assert_eq!(json["registry"]["status"], "valid");
    assert_eq!(json["registry"]["installed_module_count"], 1);
    assert_eq!(json["receipts"]["overall_state"], "valid");
    assert_eq!(json["receipts"]["valid_count"], 1);
}

#[test]
fn store_status_text_accepts_read_only_store_root_fixture() {
    let args = store_status_fixture_args("valid-empty-registry", false);
    let (code, out, err) = store_command(&args);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    assert!(out.contains("store_root_override:"));
    assert!(out.contains("registry:"));
    assert!(out.contains("status: valid"));
    assert!(out.contains("installed_module_count: 0"));
    assert!(out.contains("receipts:"));
    assert!(out.contains("status: not_referenced"));
}

#[test]
fn store_status_override_surfaces_missing_receipt_as_invalid() {
    let args = store_status_fixture_args("missing-receipt", true);
    let (code, out, err) = store_command(&args);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    let json: Value = serde_json::from_str(&out).expect("store status should be JSON");
    assert_eq!(json["registry"]["status"], "valid");
    assert_eq!(json["receipts"]["overall_state"], "absent");
    assert_eq!(json["receipts"]["absent_count"], 1);
    assert_eq!(json["overall_state"], "invalid");
}

#[test]
fn store_status_override_surfaces_invalid_receipt() {
    let args = store_status_fixture_args("invalid-receipt", true);
    let (code, out, err) = store_command(&args);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    let json: Value = serde_json::from_str(&out).expect("store status should be JSON");
    assert_eq!(json["receipts"]["overall_state"], "invalid");
    assert_eq!(json["receipts"]["invalid_count"], 1);
    assert_eq!(json["overall_state"], "invalid");
}

#[test]
fn store_init_requires_explicit_plan_or_apply_flag() {
    let args = vec!["init".to_string()];
    let (code, out, err) = store_command(&args);
    assert_eq!(code, ExitCode::Usage);
    assert!(out.is_empty());
    assert!(err.contains("requires --dry-run or --yes"));
}

#[test]
fn store_init_dry_run_reports_scaffold_plan() {
    let args = vec![
        "init".to_string(),
        "--dry-run".to_string(),
        "--format".to_string(),
        "json".to_string(),
    ];
    let (code, out, err) = store_command(&args);
    assert_eq!(code, ExitCode::Ok);
    assert!(err.is_empty());
    let json: Value = serde_json::from_str(&out).expect("store init should be JSON");
    assert_eq!(json["command"], "store init");
    assert_eq!(json["dry_run"], true);
    assert_eq!(json["writes_attempted"], false);
    assert!(
        json["steps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|step| { step["role"] == "registry_path" })
    );
    assert!(
        json["steps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|step| { step["role"] == "init_marker_path" })
    );
}

fn store_status_fixture_args(name: &str, json: bool) -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("store-roots")
        .join(name);
    let mut args = vec![
        "status".to_string(),
        "--store-root".to_string(),
        root.display().to_string(),
    ];
    if json {
        args.extend(["--format".to_string(), "json".to_string()]);
    }
    args
}
