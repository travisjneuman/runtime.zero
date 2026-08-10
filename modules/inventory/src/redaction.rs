use rz0_inventory_contract::{InventoryEvent, InventoryReport};
use rz0_privacy_contract::{RedactionContext, SensitiveValueClass};

pub fn redact_path_values(report: &mut InventoryReport) -> Result<(), String> {
    let mut context = RedactionContext::default();

    for entry in &mut report.path_entries {
        entry.path = context
            .redact(SensitiveValueClass::LocalPath, &entry.path)
            .map_err(|error| error.to_string())?;
    }
    for tool in &mut report.tools {
        context
            .redact_optional(SensitiveValueClass::LocalPath, &mut tool.executable_path)
            .map_err(|error| error.to_string())?;
    }
    for app in &mut report.apps {
        context
            .redact_optional(SensitiveValueClass::LocalPath, &mut app.install_location)
            .map_err(|error| error.to_string())?;
    }
    for service in &mut report.services {
        context
            .redact_optional(SensitiveValueClass::LocalPath, &mut service.location)
            .map_err(|error| error.to_string())?;
    }

    report.path_values_redacted = true;
    report.events.push(InventoryEvent {
        level: "info".to_string(),
        code: "path_values_redacted".to_string(),
        source_id: None,
        message: format!(
            "local path values were replaced with {} stable report-local placeholders",
            context.token_count()
        ),
    });
    Ok(())
}
