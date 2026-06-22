use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use runtime_zero::installed_registry::{InstalledRegistryState, installed_registry_report};

#[test]
fn absent_registry_reports_absent() {
    let report = installed_registry_report(&unique_temp_path("missing"));
    assert_eq!(report.status, InstalledRegistryState::Absent);
    assert_eq!(report.installed_module_count, 0);
}

#[test]
fn empty_registry_reports_valid_empty() {
    let report = installed_registry_report(&fixture("valid-empty.json"));
    assert_eq!(report.status, InstalledRegistryState::Valid);
    assert_eq!(report.installed_module_count, 0);
}

#[test]
fn valid_registry_counts_two_modules() {
    let report = installed_registry_report(&fixture("valid-two.json"));
    assert_eq!(report.status, InstalledRegistryState::Valid);
    assert_eq!(report.installed_module_count, 2);
    assert!(report.duplicate_ids.is_empty());
}

#[test]
fn duplicate_ids_are_invalid() {
    let report = installed_registry_report(&fixture("duplicate-ids.json"));
    assert_eq!(report.status, InstalledRegistryState::Invalid);
    assert_eq!(report.duplicate_ids, vec!["first-party.inventory"]);
    assert_eq!(report.malformed_record_count, 2);
}

#[test]
fn unsupported_schema_is_invalid() {
    let report = installed_registry_report(&fixture("unsupported-schema.json"));
    assert_eq!(report.status, InstalledRegistryState::Invalid);
    assert_eq!(report.schema_version, Some(99));
}

#[test]
fn malformed_json_is_invalid() {
    let report = installed_registry_report(&fixture("malformed-json.json"));
    assert_eq!(report.status, InstalledRegistryState::Invalid);
    assert!(report.errors[0].contains("malformed registry JSON"));
}

#[test]
fn wrong_shape_is_invalid() {
    let report = installed_registry_report(&fixture("wrong-shape.json"));
    assert_eq!(report.status, InstalledRegistryState::Invalid);
}

#[test]
fn unsafe_paths_are_invalid() {
    let report = installed_registry_report(&fixture("unsafe-path.json"));
    assert_eq!(report.status, InstalledRegistryState::Invalid);
    assert!(report.unsafe_path_count > 0);
}

fn fixture(name: &str) -> PathBuf {
    Path::new("tests")
        .join("fixtures")
        .join("installed-registries")
        .join(name)
}

fn unique_temp_path(name: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("rz0-registry-{name}-{nanos}.json"))
}
