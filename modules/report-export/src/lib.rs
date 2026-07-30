pub use rz0_support_contract::{
    MAX_SUPPORT_INPUT_BYTES as MAX_REPORT_EXPORT_INPUT_BYTES,
    SUPPORT_INPUT_CONTRACT as REPORT_EXPORT_INPUT_CONTRACT,
    SUPPORT_SCHEMA_VERSION as REPORT_EXPORT_INPUT_SCHEMA_VERSION,
    SupportReportInput as ReportExportInput,
};
use rz0_support_contract::{
    SupportReport, build_support_report_from_input, decode_support_input, support_json,
    support_text,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Text,
    Json,
}

pub fn decode_export_input(bytes: &[u8]) -> Result<ReportExportInput, String> {
    decode_support_input(bytes)
}

pub fn build_export(input: &ReportExportInput) -> Result<SupportReport, String> {
    build_support_report_from_input(input)
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
    fn foundation_input_builds_deterministic_text_and_json() {
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
    fn raw_paths_fail_closed() {
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
    }
}
