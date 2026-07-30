use std::fmt::Write as FmtWrite;

use rz0_inventory_contract::InventoryReport;

pub fn render_json(report: &InventoryReport) -> Result<String, String> {
    let validation = rz0_inventory_contract::validate_inventory_report(report);
    if !validation.valid {
        return Err(format!(
            "inventory report failed its shared contract: {}",
            validation.errors.join("; ")
        ));
    }
    serde_json::to_string_pretty(report)
        .map(|json| format!("{json}\n"))
        .map_err(|error| format!("failed to render inventory JSON: {error}"))
}

pub fn render_text(report: &InventoryReport) -> String {
    let mut out = String::from("runtime.zero inventory\n\n");
    let _ = writeln!(out, "module: first-party.inventory");
    let _ = writeln!(out, "mode: read-only");
    let _ = writeln!(out, "writes_attempted: no");
    let _ = writeln!(out, "path_values_redacted: {}", report.path_values_redacted);
    let _ = writeln!(
        out,
        "generated_at: {}",
        report.generated_at.as_deref().unwrap_or("not-recorded")
    );
    let _ = writeln!(out, "sources: {}", report.summary.source_count);
    let _ = writeln!(out, "path_entries: {}", report.summary.path_entry_count);
    let _ = writeln!(out, "tools: {}", report.summary.tool_count);
    let _ = writeln!(out, "apps: {}", report.summary.app_count);
    let _ = writeln!(out, "warnings: {}", report.summary.warning_count);

    let _ = writeln!(out, "\nevidence sources:");
    for source in &report.sources {
        let _ = writeln!(
            out,
            "  {:<24} {:<11} read_only={} duration_ms={}",
            source.id,
            source.status,
            source.read_only,
            source.duration_ms.unwrap_or_default()
        );
        for warning in &source.warnings {
            let _ = writeln!(out, "    warning: {warning}");
        }
    }

    let _ = writeln!(out, "\nPATH evidence:");
    if report.path_entries.is_empty() {
        let _ = writeln!(out, "  none");
    }
    for entry in &report.path_entries {
        let _ = writeln!(
            out,
            "  [{}:{:03}] {} ({}, exists={})",
            entry.scope, entry.order, entry.path, entry.entry_kind, entry.exists
        );
        for warning in &entry.warnings {
            let _ = writeln!(out, "    warning: {warning}");
        }
    }

    let _ = writeln!(out, "\nknown tools:");
    if report.tools.is_empty() {
        let _ = writeln!(out, "  none");
    }
    for tool in &report.tools {
        let _ = writeln!(
            out,
            "  {} version={} path={}",
            tool.display_name,
            tool.version.as_deref().unwrap_or("not-probed"),
            tool.executable_path.as_deref().unwrap_or("not-recorded")
        );
        for warning in &tool.warnings {
            let _ = writeln!(out, "    warning: {warning}");
        }
    }

    let _ = writeln!(out, "\ninstalled applications:");
    if report.apps.is_empty() {
        let _ = writeln!(out, "  not requested or none reported");
    }
    for app in &report.apps {
        let _ = writeln!(
            out,
            "  {} version={} publisher={}",
            app.name,
            app.version.as_deref().unwrap_or("unknown"),
            app.publisher.as_deref().unwrap_or("unknown")
        );
    }

    let _ = writeln!(
        out,
        "\nsafety: report only; no PATH/registry/package/app state was changed"
    );
    out
}
