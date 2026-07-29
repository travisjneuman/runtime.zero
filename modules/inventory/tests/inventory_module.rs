use std::path::{Path, PathBuf};

use rz0_module_inventory::{InventoryOptions, collect_inventory, render_json};

#[test]
fn valid_fixture_produces_deterministic_read_only_report() {
    let report = collect_fixture("valid.json", false).expect("valid fixture");
    assert!(report.read_only);
    assert!(!report.writes_attempted);
    assert!(report.generated_at.is_none());
    assert_eq!(
        report.runtime.module_id.as_deref(),
        Some("first-party.inventory")
    );
    assert_eq!(report.summary.source_count, 1);
    assert_eq!(report.summary.path_entry_count, 2);
    assert_eq!(report.sources[0].status, "ok");
    assert!(!report.raw_registry_keys_included);
}

#[test]
fn duplicate_fixture_is_case_insensitive_for_windows() {
    let report = collect_fixture("duplicate.json", false).expect("duplicate fixture");
    assert_eq!(report.sources[0].status, "partial");
    assert!(
        report.path_entries[1]
            .warnings
            .iter()
            .any(|warning| warning.contains("duplicate"))
    );
}

#[test]
fn missing_fixture_preserves_missing_evidence() {
    let report = collect_fixture("missing.json", false).expect("missing fixture");
    assert!(!report.path_entries[0].exists);
    assert_eq!(report.path_entries[0].entry_kind, "missing");
    assert_eq!(report.sources[0].status, "partial");
}

#[test]
fn malformed_and_unsupported_fixtures_fail_closed() {
    for name in [
        "malformed.json",
        "unsupported-platform.json",
        "invalid-entry.json",
    ] {
        let error = collect_fixture(name, false).expect_err(name);
        assert!(!error.is_empty());
    }
}

#[test]
fn redaction_reuses_report_local_tokens_without_original_paths() {
    let report = collect_fixture("duplicate.json", true).expect("redacted fixture");
    assert!(report.path_values_redacted);
    assert!(
        report
            .path_entries
            .iter()
            .all(|entry| entry.path.starts_with("<redacted:path:"))
    );
    let json = render_json(&report).expect("inventory JSON");
    assert!(json.contains("<redacted:path:0001>"));
    assert!(!json.contains("Tools"));
    assert!(!json.contains("\\\\Git"));
    assert!(!json.contains("\u{1b}["));
}

#[cfg(not(windows))]
#[test]
fn windows_app_inventory_fails_closed_on_other_platforms() {
    let error = collect_inventory(&InventoryOptions {
        include_apps: true,
        ..InventoryOptions::default()
    })
    .expect_err("Windows-only option");
    assert!(error.contains("only on Windows"));
}

fn collect_fixture(
    name: &str,
    redact_paths: bool,
) -> Result<rz0_inventory_contract::InventoryReport, String> {
    collect_inventory(&InventoryOptions {
        fixture: Some(fixture_path(name)),
        redact_paths,
        ..InventoryOptions::default()
    })
}

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}
