use std::collections::BTreeMap;

use rz0_inventory_contract::{InventoryEvent, InventoryReport};

pub fn redact_path_values(report: &mut InventoryReport) {
    let mut replacements = BTreeMap::new();
    let mut next = 1usize;

    for entry in &mut report.path_entries {
        entry.path = replacement(&entry.path, &mut replacements, &mut next);
    }
    for tool in &mut report.tools {
        if let Some(path) = &mut tool.executable_path {
            *path = replacement(path, &mut replacements, &mut next);
        }
    }
    for app in &mut report.apps {
        if let Some(path) = &mut app.install_location {
            *path = replacement(path, &mut replacements, &mut next);
        }
    }

    report.path_values_redacted = true;
    report.events.push(InventoryEvent {
        level: "info".to_string(),
        code: "path_values_redacted".to_string(),
        source_id: None,
        message: "local path values were replaced with stable report-local placeholders"
            .to_string(),
    });
}

fn replacement(
    value: &str,
    replacements: &mut BTreeMap<String, String>,
    next: &mut usize,
) -> String {
    if let Some(existing) = replacements.get(value) {
        return existing.clone();
    }
    let token = format!("<redacted:path:{:04}>", *next);
    *next = next.saturating_add(1);
    replacements.insert(value.to_string(), token.clone());
    token
}
