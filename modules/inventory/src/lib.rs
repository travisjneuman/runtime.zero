mod command_probe;
mod fixture;
mod path_inventory;
#[cfg(any(target_os = "macos", target_os = "linux", test))]
mod platform_apps;
mod redaction;
mod render;
mod tool_specs;
mod tools;
#[cfg(windows)]
mod windows_registry;

use std::path::PathBuf;

use rz0_inventory_contract::{
    InventoryEvent, InventoryHost, InventoryReport, InventoryRuntime, InventorySource,
    validate_inventory_report,
};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

pub use render::{render_json, render_text};

pub const MODULE_ID: &str = "first-party.inventory";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryOptions {
    pub fixture: Option<PathBuf>,
    pub redact_paths: bool,
    pub probe_versions: bool,
    pub include_apps: bool,
}

impl Default for InventoryOptions {
    fn default() -> Self {
        Self {
            fixture: None,
            redact_paths: true,
            probe_versions: false,
            include_apps: false,
        }
    }
}

pub fn collect_inventory(options: &InventoryOptions) -> Result<InventoryReport, String> {
    validate_options(options)?;
    let mut report = module_report(options.fixture.is_none());

    if let Some(path) = &options.fixture {
        let collection = fixture::load_path_fixture(path)?;
        record_source_event(&mut report, &collection.source);
        report.sources.push(collection.source);
        report.path_entries.extend(collection.entries);
    } else {
        let process_path = path_inventory::collect_process_path();
        record_source_event(&mut report, &process_path.source);
        report.sources.push(process_path.source);
        report.path_entries.extend(process_path.entries);

        #[cfg(windows)]
        {
            for collection in windows_registry::collect_persisted_paths() {
                record_source_event(&mut report, &collection.source);
                report.sources.push(collection.source);
                report.path_entries.extend(collection.entries);
            }
        }
    }

    if options.fixture.is_none() {
        let tools = tools::discover_known_tools(&report.path_entries, options.probe_versions);
        record_source_event(&mut report, &tools.source);
        report.sources.push(tools.source);
        report.tools.extend(tools.tools);
    }

    #[cfg(any(windows, target_os = "macos", target_os = "linux"))]
    if options.include_apps {
        #[cfg(windows)]
        let apps = windows_registry::collect_installed_apps();
        #[cfg(any(target_os = "macos", target_os = "linux"))]
        let apps = platform_apps::collect_installed_apps();

        record_source_event(&mut report, &apps.source);
        report.sources.push(apps.source);
        report.apps.extend(apps.apps);
        report.warnings.push(
            "installed application names may be sensitive; review them before sharing output"
                .to_string(),
        );
    }

    if options.redact_paths {
        redaction::redact_path_values(&mut report)?;
    }
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

fn validate_options(options: &InventoryOptions) -> Result<(), String> {
    if options.fixture.is_some() && options.probe_versions {
        return Err("--probe-versions cannot be combined with --fixture".to_string());
    }
    if options.fixture.is_some() && options.include_apps {
        return Err("--include-apps cannot be combined with --fixture".to_string());
    }
    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    if options.include_apps {
        return Err("--include-apps is not supported on this platform".to_string());
    }
    Ok(())
}

fn module_report(include_timestamp: bool) -> InventoryReport {
    let mut report = InventoryReport::empty(
        InventoryHost {
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            hostname_included: false,
            current_user_included: false,
        },
        InventoryRuntime {
            title: "runtime.zero".to_string(),
            command: "rz0-inventory".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            scan_mode: "read_only_inventory".to_string(),
            mutation_capability: "disabled".to_string(),
            module_schema_version: 1,
            module_id: Some(MODULE_ID.to_string()),
        },
    );
    report.generated_at = include_timestamp
        .then(current_timestamp)
        .transpose()
        .ok()
        .flatten();
    report.events.push(InventoryEvent {
        level: "info".to_string(),
        code: "module_started".to_string(),
        source_id: None,
        message: "read-only inventory collection started".to_string(),
    });
    report
}

fn current_timestamp() -> Result<String, String> {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .map_err(|error| format!("failed to format inventory timestamp: {error}"))
}

fn record_source_event(report: &mut InventoryReport, source: &InventorySource) {
    report.events.push(InventoryEvent {
        level: if matches!(source.status.as_str(), "error" | "unavailable") {
            "warning".to_string()
        } else {
            "info".to_string()
        },
        code: "source_completed".to_string(),
        source_id: Some(source.id.clone()),
        message: format!("source completed with status {}", source.status),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixture_and_probe_flags_are_mutually_exclusive() {
        let options = InventoryOptions {
            fixture: Some(PathBuf::from("fixture.json")),
            probe_versions: true,
            ..InventoryOptions::default()
        };
        assert!(collect_inventory(&options).is_err());
    }

    #[test]
    fn module_report_preserves_read_only_contract() {
        let report = module_report(false);
        assert!(report.read_only);
        assert!(!report.writes_attempted);
        assert!(!report.raw_registry_keys_included);
        assert_eq!(report.runtime.module_id.as_deref(), Some(MODULE_ID));
        assert!(report.generated_at.is_none());
    }
}
