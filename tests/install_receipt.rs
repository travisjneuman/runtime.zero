use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use runtime_zero::install_receipt::{
    InstallReceiptState, ReceiptInventoryState, install_receipt_report, receipt_inventory_report,
};
use runtime_zero::installed_registry::InstalledRegistryRecordStatus;

#[test]
fn absent_receipt_reports_absent() {
    let path = unique_temp_dir("absent")
        .join("receipts")
        .join("missing.json");
    let report = install_receipt_report(
        &path,
        "receipts/missing.json",
        "first-party.inventory",
        "0.1.0",
    );
    assert_eq!(report.status, InstallReceiptState::Absent);
}

#[test]
fn valid_receipt_matches_registry_module() {
    let report = install_receipt_report(
        &fixture("valid.json"),
        "receipts/rz0plan_inventory.json",
        "first-party.inventory",
        "0.1.0",
    );
    assert_eq!(report.status, InstallReceiptState::Valid);
    assert!(report.module_matches_registry);
    assert_eq!(report.write_entry_count, 3);
}

#[test]
fn module_id_mismatch_is_invalid() {
    let report = install_receipt_report(
        &fixture("valid.json"),
        "receipts/rz0plan_inventory.json",
        "first-party.leftovers",
        "0.1.0",
    );
    assert_eq!(report.status, InstallReceiptState::Invalid);
    assert!(!report.module_matches_registry);
}

#[test]
fn unsupported_schema_is_reported_separately() {
    let report = install_receipt_report(
        &fixture("unsupported-schema.json"),
        "receipts/unsupported.json",
        "first-party.inventory",
        "0.1.0",
    );
    assert_eq!(report.status, InstallReceiptState::UnsupportedSchema);
}

#[test]
fn malformed_json_is_invalid() {
    let report = install_receipt_report(
        &fixture("malformed-json.json"),
        "receipts/malformed.json",
        "first-party.inventory",
        "0.1.0",
    );
    assert_eq!(report.status, InstallReceiptState::Invalid);
    assert!(report.errors[0].contains("malformed receipt JSON"));
}

#[test]
fn unsafe_paths_are_invalid() {
    let report = install_receipt_report(
        &fixture("unsafe-path.json"),
        "receipts/unsafe.json",
        "first-party.inventory",
        "0.1.0",
    );
    assert_eq!(report.status, InstallReceiptState::Invalid);
    assert!(report.unsafe_path_count > 0);
}

#[test]
fn inventory_cross_checks_existing_receipts() {
    let state_root = unique_temp_dir("inventory-valid");
    let receipts_dir = state_root.join("receipts");
    fs::create_dir_all(&receipts_dir).expect("receipts dir created");
    fs::copy(
        fixture("valid.json"),
        receipts_dir.join("rz0plan_inventory.json"),
    )
    .expect("fixture copied");
    let records = vec![record("receipts/rz0plan_inventory.json")];
    let report = receipt_inventory_report(&state_root, &records);
    fs::remove_dir_all(&state_root).expect("temp state removed");
    assert_eq!(report.overall_state, ReceiptInventoryState::Valid);
    assert_eq!(report.valid_count, 1);
}

#[test]
fn inventory_reports_absent_referenced_receipt() {
    let state_root = unique_temp_dir("inventory-absent");
    let records = vec![record("receipts/missing.json")];
    let report = receipt_inventory_report(&state_root, &records);
    assert_eq!(report.overall_state, ReceiptInventoryState::Absent);
    assert_eq!(report.absent_count, 1);
}

fn record(receipt_path: &str) -> InstalledRegistryRecordStatus {
    InstalledRegistryRecordStatus {
        id: "first-party.inventory".to_string(),
        version: "0.1.0".to_string(),
        manifest_path: "modules/first-party.inventory/0.1.0/rz0-module.json".to_string(),
        receipt_path: receipt_path.to_string(),
        module_dir: Some("modules/first-party.inventory/0.1.0".to_string()),
        valid: true,
        errors: Vec::new(),
    }
}

fn fixture(name: &str) -> PathBuf {
    Path::new("tests")
        .join("fixtures")
        .join("install-receipts")
        .join(name)
}

fn unique_temp_dir(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("rz0-receipt-{name}-{nanos}"))
}
