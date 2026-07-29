use std::fmt::Write as FmtWrite;

use serde::Serialize;

use crate::{brand, module_manifest::MODULE_SCHEMA_VERSION};

pub const INVENTORY_SCHEMA_VERSION: u16 = 1;
pub const INVENTORY_CONTRACT: &str = "inventory_report";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InventoryReport {
    pub schema_version: u16,
    pub contract: &'static str,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub generated_at: Option<String>,
    pub host: InventoryHost,
    pub runtime: InventoryRuntime,
    pub sources: Vec<InventorySource>,
    pub path_entries: Vec<PathEntry>,
    pub tools: Vec<ToolRecord>,
    pub apps: Vec<AppRecord>,
    pub warnings: Vec<String>,
    pub summary: InventorySummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InventoryHost {
    pub os: &'static str,
    pub arch: &'static str,
    pub hostname_included: bool,
    pub current_user_included: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InventoryRuntime {
    pub title: &'static str,
    pub command: &'static str,
    pub version: &'static str,
    pub scan_mode: &'static str,
    pub mutation_capability: &'static str,
    pub module_schema_version: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InventorySource {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub duration_ms: Option<u64>,
    pub read_only: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PathEntry {
    pub path: String,
    pub scope: String,
    pub order: u32,
    pub exists: bool,
    pub entry_kind: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ToolRecord {
    pub id: String,
    pub display_name: String,
    pub category: String,
    pub executable_path: Option<String>,
    pub version: Option<String>,
    pub source_ids: Vec<String>,
    pub confidence: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppRecord {
    pub id: String,
    pub name: String,
    pub source_id: String,
    pub version: Option<String>,
    pub publisher: Option<String>,
    pub install_location: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InventorySummary {
    pub source_count: usize,
    pub source_ok_count: usize,
    pub path_entry_count: usize,
    pub tool_count: usize,
    pub app_count: usize,
    pub warning_count: usize,
}

pub fn contract_report() -> InventoryReport {
    let warnings =
        vec!["inventory collectors are not implemented; no local evidence was read".to_string()];

    InventoryReport {
        schema_version: INVENTORY_SCHEMA_VERSION,
        contract: INVENTORY_CONTRACT,
        read_only: true,
        writes_attempted: false,
        generated_at: None,
        host: InventoryHost {
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            hostname_included: false,
            current_user_included: false,
        },
        runtime: InventoryRuntime {
            title: brand::TITLE,
            command: brand::COMMAND,
            version: env!("CARGO_PKG_VERSION"),
            scan_mode: "dry_run",
            mutation_capability: "disabled",
            module_schema_version: MODULE_SCHEMA_VERSION,
        },
        sources: Vec::new(),
        path_entries: Vec::new(),
        tools: Vec::new(),
        apps: Vec::new(),
        summary: InventorySummary {
            source_count: 0,
            source_ok_count: 0,
            path_entry_count: 0,
            tool_count: 0,
            app_count: 0,
            warning_count: warnings.len(),
        },
        warnings,
    }
}

pub fn contract_text(report: &InventoryReport) -> String {
    let mut out = format!("{} scan plan\n\n", brand::TITLE);
    let _ = writeln!(out, "mode: dry-run");
    let _ = writeln!(out, "contract: {}", report.contract);
    let _ = writeln!(out, "schema_version: {}", report.schema_version);
    let _ = writeln!(out, "mutation_capability: disabled");
    let _ = writeln!(out, "writes_attempted: no");
    let _ = writeln!(out, "sources_collected: {}", report.summary.source_count);
    let _ = writeln!(out, "result: no system changes were attempted");
    for warning in &report.warnings {
        let _ = writeln!(out, "warning: {warning}");
    }
    let _ = writeln!(out, "next: fixture-backed Windows PATH inventory evidence");
    out
}

pub fn contract_json(report: &InventoryReport) -> Result<String, String> {
    serde_json::to_string_pretty(report)
        .map(|json| format!("{json}\n"))
        .map_err(|err| format!("failed to render inventory JSON: {err}\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_report_is_empty_private_and_read_only() {
        let report = contract_report();
        assert_eq!(report.schema_version, 1);
        assert_eq!(report.contract, "inventory_report");
        assert!(report.read_only);
        assert!(!report.writes_attempted);
        assert!(!report.host.hostname_included);
        assert!(!report.host.current_user_included);
        assert!(report.sources.is_empty());
        assert!(report.path_entries.is_empty());
        assert!(report.tools.is_empty());
        assert!(report.apps.is_empty());
        assert_eq!(report.summary.warning_count, 1);
    }

    #[test]
    fn contract_json_is_ansi_free() {
        let json = contract_json(&contract_report()).expect("inventory JSON");
        assert!(json.contains("\"contract\": \"inventory_report\""));
        assert!(json.contains("\"writes_attempted\": false"));
        assert!(!json.contains("\u{1b}["));
    }
}
