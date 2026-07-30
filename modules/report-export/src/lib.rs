use rz0_diagnostics_contract::DiagnosticReport;
use rz0_inventory_contract::InventoryReport;
use rz0_support_contract::{SupportReport, build_support_report, support_json, support_text};
use serde::{Deserialize, Serialize};

pub const REPORT_EXPORT_INPUT_SCHEMA_VERSION: u16 = 1;
pub const REPORT_EXPORT_INPUT_CONTRACT: &str = "report_export_input";
pub const MAX_REPORT_EXPORT_INPUT_BYTES: u64 = rz0_resource_contract::MAX_INVENTORY_REPORT_BYTES
    + rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReportExportInput {
    pub schema_version: u16,
    pub contract: String,
    pub inventory: InventoryReport,
    pub diagnostics: DiagnosticReport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Text,
    Json,
}

pub fn decode_export_input(bytes: &[u8]) -> Result<ReportExportInput, String> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_REPORT_EXPORT_INPUT_BYTES {
        return Err(format!(
            "report export input must contain 1 to {MAX_REPORT_EXPORT_INPUT_BYTES} bytes"
        ));
    }
    let input: ReportExportInput = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid report export input JSON: {error}"))?;
    if input.schema_version != REPORT_EXPORT_INPUT_SCHEMA_VERSION {
        return Err(format!(
            "schema_version must be {REPORT_EXPORT_INPUT_SCHEMA_VERSION}"
        ));
    }
    if input.contract != REPORT_EXPORT_INPUT_CONTRACT {
        return Err(format!("contract must be {REPORT_EXPORT_INPUT_CONTRACT}"));
    }
    build_support_report(&input.inventory, &input.diagnostics)?;
    Ok(input)
}

pub fn build_export(input: &ReportExportInput) -> Result<SupportReport, String> {
    if input.schema_version != REPORT_EXPORT_INPUT_SCHEMA_VERSION
        || input.contract != REPORT_EXPORT_INPUT_CONTRACT
    {
        return Err("report export input identity is invalid".to_string());
    }
    build_support_report(&input.inventory, &input.diagnostics)
}

pub fn render_export(report: &SupportReport, format: ExportFormat) -> Result<String, String> {
    match format {
        ExportFormat::Text => support_text(report),
        ExportFormat::Json => support_json(report),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rz0_diagnostics_contract::foundation_diagnostics;
    use rz0_inventory_contract::{InventoryHost, InventoryReport, InventoryRuntime};

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
            diagnostics: foundation_diagnostics(
                "runtime.zero",
                "rz0",
                "0.1.0",
                "test-os",
                "test-arch",
            ),
        }
    }

    #[test]
    fn strict_input_builds_deterministic_text_and_json() {
        let input = input();
        let bytes = serde_json::to_vec(&input).unwrap();
        let decoded = decode_export_input(&bytes).unwrap();
        let report = build_export(&decoded).unwrap();
        let text = render_export(&report, ExportFormat::Text).unwrap();
        let json = render_export(&report, ExportFormat::Json).unwrap();
        assert!(text.contains("external_sharing_authorized: false"));
        assert!(json.contains("\"local_export_ready\": true"));
        assert!(!json.contains("runtime identity available"));
    }

    #[test]
    fn raw_paths_unknown_fields_and_oversized_input_fail_closed() {
        let mut raw_input = input();
        raw_input
            .inventory
            .path_entries
            .push(rz0_inventory_contract::PathEntry {
                path: "/private/example".to_string(),
                scope: "process".to_string(),
                order: 0,
                exists: false,
                entry_kind: "missing".to_string(),
                warnings: Vec::new(),
            });
        raw_input.inventory.recalculate_summary();
        assert!(decode_export_input(&serde_json::to_vec(&raw_input).unwrap()).is_err());

        let mut value = serde_json::to_value(input()).unwrap();
        value["future"] = serde_json::Value::Bool(true);
        assert!(decode_export_input(&serde_json::to_vec(&value).unwrap()).is_err());
        assert!(
            decode_export_input(&vec![b'x'; MAX_REPORT_EXPORT_INPUT_BYTES as usize + 1]).is_err()
        );
    }
}
