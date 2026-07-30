use std::fmt::Write as FmtWrite;

pub use rz0_inventory_contract::*;

use crate::{brand, module_manifest::MODULE_SCHEMA_VERSION};

pub fn contract_report() -> InventoryReport {
    let mut report = InventoryReport::empty(
        InventoryHost {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            hostname_included: false,
            current_user_included: false,
        },
        InventoryRuntime {
            title: brand::TITLE.to_string(),
            command: brand::COMMAND.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            scan_mode: "dry_run".to_string(),
            mutation_capability: "disabled".to_string(),
            module_schema_version: MODULE_SCHEMA_VERSION,
            module_id: None,
        },
    );
    report.warnings.push(
        "core scan does not collect local evidence; the separate inventory module is not loaded"
            .to_string(),
    );
    report.recalculate_summary();
    report
}

pub fn live_report(redact_paths: bool) -> Result<InventoryReport, String> {
    let mut report =
        rz0_module_inventory::collect_inventory(&rz0_module_inventory::InventoryOptions {
            fixture: None,
            redact_paths,
            probe_versions: false,
            include_apps: true,
        })?;
    report.runtime.command = brand::COMMAND.to_string();
    report.runtime.scan_mode = "dry_run".to_string();
    report.recalculate_summary();
    let validation = validate_inventory_report(&report);
    if !validation.valid {
        return Err(format!(
            "inventory report failed its shared contract: {}",
            validation.errors.join("; ")
        ));
    }
    Ok(report)
}

pub fn recalculate_summary(report: &mut InventoryReport) {
    report.recalculate_summary();
}

pub fn contract_text(report: &InventoryReport) -> String {
    let mut out = format!("{} scan plan\n\n", brand::TITLE);
    let _ = writeln!(out, "mode: dry-run");
    let _ = writeln!(out, "contract: {}", report.contract);
    let _ = writeln!(out, "schema_version: {}", report.schema_version);
    let _ = writeln!(out, "mutation_capability: disabled");
    let _ = writeln!(out, "writes_attempted: no");
    let _ = writeln!(out, "sources_collected: {}", report.summary.source_count);
    let _ = writeln!(out, "known_tools: {}", report.summary.tool_count);
    let _ = writeln!(out, "installed_software: {}", report.summary.app_count);
    let _ = writeln!(out, "result: no system changes were attempted");
    for warning in &report.warnings {
        let _ = writeln!(out, "warning: {warning}");
    }
    let _ = writeln!(out, "\ninstalled software:");
    if report.apps.is_empty() {
        let _ = writeln!(out, "  none reported");
    }
    for app in &report.apps {
        let _ = writeln!(
            out,
            "  {}\tversion={}\tsource={}",
            app.name,
            app.version.as_deref().unwrap_or("unknown"),
            app.source_id
        );
    }
    let _ = writeln!(out, "\nknown tools:");
    if report.tools.is_empty() {
        let _ = writeln!(out, "  none reported");
    }
    for tool in &report.tools {
        let _ = writeln!(
            out,
            "  {}\tversion={}",
            tool.display_name,
            tool.version.as_deref().unwrap_or("not-probed")
        );
    }
    let _ = writeln!(
        out,
        "\ninventory_adapter: built-in first-party read-only collector"
    );
    out
}

pub fn contract_json(report: &InventoryReport) -> Result<String, String> {
    let validation = validate_inventory_report(report);
    if !validation.valid {
        return Err(format!(
            "inventory report failed its shared contract: {}\n",
            validation.errors.join("; ")
        ));
    }
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
        assert!(!report.path_values_redacted);
        assert!(!report.raw_registry_keys_included);
        assert!(report.runtime.module_id.is_none());
        assert!(report.sources.is_empty());
        assert!(report.path_entries.is_empty());
        assert!(report.tools.is_empty());
        assert!(report.apps.is_empty());
        assert!(report.events.is_empty());
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
