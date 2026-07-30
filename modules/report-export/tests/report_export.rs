use std::io::Write as _;
use std::process::{Command, Stdio};

use rz0_diagnostics_contract::foundation_diagnostics;
use rz0_inventory_contract::{InventoryHost, InventoryReport, InventoryRuntime, PathEntry};
use rz0_module_report_export::{
    REPORT_EXPORT_INPUT_CONTRACT, REPORT_EXPORT_INPUT_SCHEMA_VERSION, ReportExportInput,
};

fn input() -> ReportExportInput {
    ReportExportInput {
        schema_version: REPORT_EXPORT_INPUT_SCHEMA_VERSION,
        contract: REPORT_EXPORT_INPUT_CONTRACT.to_string(),
        inventory: InventoryReport::empty(
            InventoryHost {
                os: "test-os".to_string(),
                arch: "test-arch".to_string(),
                hostname_included: false,
                current_user_included: false,
            },
            InventoryRuntime {
                title: "runtime.zero".to_string(),
                command: "rz0".to_string(),
                version: "0.1.0".to_string(),
                scan_mode: "dry_run".to_string(),
                mutation_capability: "disabled".to_string(),
                module_schema_version: 1,
                module_id: None,
            },
        ),
        diagnostics: foundation_diagnostics("runtime.zero", "rz0", "0.1.0", "test-os", "test-arch"),
    }
}

fn invoke(input: &ReportExportInput) -> std::process::Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rz0-report-export"))
        .args(["--format", "json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn report export helper");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(input).unwrap())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn final_module_binary_emits_only_valid_non_authorizing_summary() {
    let output = invoke(&input());
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let report = rz0_support_contract::decode_support_report(&output.stdout).unwrap();
    assert!(report.local_export_ready);
    assert!(!report.external_sharing_authorized);
    assert!(!report.product_execution_authorized);
    assert!(!report.release_authorized);
}

#[test]
fn raw_path_input_fails_without_echoing_the_path() {
    let mut input = input();
    let private_path = "/private/do-not-echo/example";
    input.inventory.path_entries.push(PathEntry {
        path: private_path.to_string(),
        scope: "process".to_string(),
        order: 0,
        exists: false,
        entry_kind: "missing".to_string(),
        warnings: Vec::new(),
    });
    input.inventory.recalculate_summary();
    let output = invoke(&input);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains(private_path));
}
