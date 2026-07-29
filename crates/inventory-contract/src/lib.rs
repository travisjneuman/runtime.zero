use serde::Serialize;

pub const INVENTORY_SCHEMA_VERSION: u16 = 1;
pub const INVENTORY_CONTRACT: &str = "inventory_report";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InventoryReport {
    pub schema_version: u16,
    pub contract: &'static str,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub generated_at: Option<String>,
    pub path_values_redacted: bool,
    pub raw_registry_keys_included: bool,
    pub host: InventoryHost,
    pub runtime: InventoryRuntime,
    pub sources: Vec<InventorySource>,
    pub path_entries: Vec<PathEntry>,
    pub tools: Vec<ToolRecord>,
    pub apps: Vec<AppRecord>,
    pub events: Vec<InventoryEvent>,
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
    pub module_id: Option<String>,
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
pub struct InventoryEvent {
    pub level: String,
    pub code: String,
    pub source_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InventorySummary {
    pub source_count: usize,
    pub source_ok_count: usize,
    pub path_entry_count: usize,
    pub tool_count: usize,
    pub app_count: usize,
    pub event_count: usize,
    pub warning_count: usize,
}

impl InventoryReport {
    pub fn empty(host: InventoryHost, runtime: InventoryRuntime) -> Self {
        Self {
            schema_version: INVENTORY_SCHEMA_VERSION,
            contract: INVENTORY_CONTRACT,
            read_only: true,
            writes_attempted: false,
            generated_at: None,
            path_values_redacted: false,
            raw_registry_keys_included: false,
            host,
            runtime,
            sources: Vec::new(),
            path_entries: Vec::new(),
            tools: Vec::new(),
            apps: Vec::new(),
            events: Vec::new(),
            warnings: Vec::new(),
            summary: InventorySummary {
                source_count: 0,
                source_ok_count: 0,
                path_entry_count: 0,
                tool_count: 0,
                app_count: 0,
                event_count: 0,
                warning_count: 0,
            },
        }
    }

    pub fn recalculate_summary(&mut self) {
        self.summary = InventorySummary {
            source_count: self.sources.len(),
            source_ok_count: self
                .sources
                .iter()
                .filter(|source| source.status == "ok")
                .count(),
            path_entry_count: self.path_entries.len(),
            tool_count: self.tools.len(),
            app_count: self.apps.len(),
            event_count: self.events.len(),
            warning_count: self.warnings.len()
                + self
                    .sources
                    .iter()
                    .map(|source| source.warnings.len())
                    .sum::<usize>()
                + self
                    .path_entries
                    .iter()
                    .map(|entry| entry.warnings.len())
                    .sum::<usize>()
                + self
                    .tools
                    .iter()
                    .map(|tool| tool.warnings.len())
                    .sum::<usize>()
                + self
                    .apps
                    .iter()
                    .map(|app| app.warnings.len())
                    .sum::<usize>(),
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_report_is_read_only_and_private_by_default() {
        let report = InventoryReport::empty(
            InventoryHost {
                os: "test",
                arch: "test",
                hostname_included: false,
                current_user_included: false,
            },
            InventoryRuntime {
                title: "runtime.zero",
                command: "test",
                version: "0.0.0",
                scan_mode: "test",
                mutation_capability: "disabled",
                module_schema_version: 1,
                module_id: None,
            },
        );
        assert!(report.read_only);
        assert!(!report.writes_attempted);
        assert!(!report.raw_registry_keys_included);
        assert_eq!(report.summary.source_count, 0);
    }
}
