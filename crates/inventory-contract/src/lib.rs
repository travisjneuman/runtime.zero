use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const INVENTORY_SCHEMA_VERSION: u16 = 1;
pub const INVENTORY_CONTRACT: &str = "inventory_report";
pub const MAX_INVENTORY_REPORT_BYTES: u64 = rz0_resource_contract::MAX_INVENTORY_REPORT_BYTES;
pub const MAX_INVENTORY_SOURCES: usize = rz0_resource_contract::MAX_INVENTORY_SOURCES;
pub const MAX_INVENTORY_PATH_ENTRIES: usize = rz0_resource_contract::MAX_INVENTORY_PATH_ENTRIES;
pub const MAX_INVENTORY_TOOL_RECORDS: usize = rz0_resource_contract::MAX_INVENTORY_TOOL_RECORDS;
pub const MAX_INVENTORY_APP_RECORDS: usize = rz0_resource_contract::MAX_INVENTORY_APP_RECORDS;
pub const MAX_INVENTORY_SERVICE_RECORDS: usize =
    rz0_resource_contract::MAX_INVENTORY_SERVICE_RECORDS;
pub const MAX_INVENTORY_EVENTS: usize = rz0_resource_contract::MAX_INVENTORY_EVENTS;
pub const MAX_INVENTORY_WARNINGS: usize = rz0_resource_contract::MAX_INVENTORY_WARNINGS;
pub const MAX_SOFTWARE_IDENTIFIERS_PER_APP: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryReport {
    pub schema_version: u16,
    pub contract: String,
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
    #[serde(default)]
    pub services: Vec<ServiceRecord>,
    pub events: Vec<InventoryEvent>,
    pub warnings: Vec<String>,
    pub summary: InventorySummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryHost {
    pub os: String,
    pub arch: String,
    pub hostname_included: bool,
    pub current_user_included: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryRuntime {
    pub title: String,
    pub command: String,
    pub version: String,
    pub scan_mode: String,
    pub mutation_capability: String,
    pub module_schema_version: u16,
    pub module_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InventorySource {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub duration_ms: Option<u64>,
    pub read_only: bool,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PathEntry {
    pub path: String,
    pub scope: String,
    pub order: u32,
    pub exists: bool,
    pub entry_kind: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AppRecord {
    pub id: String,
    pub name: String,
    pub source_id: String,
    pub version: Option<String>,
    pub publisher: Option<String>,
    #[serde(default)]
    pub identifiers: Vec<SoftwareIdentifier>,
    pub install_location: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SoftwareIdentifier {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceRecord {
    pub id: String,
    pub name: String,
    pub source_id: String,
    pub kind: String,
    pub scope: String,
    pub enabled: Option<bool>,
    pub location: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryEvent {
    pub level: String,
    pub code: String,
    pub source_id: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InventorySummary {
    pub source_count: usize,
    pub source_ok_count: usize,
    pub path_entry_count: usize,
    pub tool_count: usize,
    pub app_count: usize,
    #[serde(default)]
    pub service_count: usize,
    pub event_count: usize,
    pub warning_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryValidation {
    pub valid: bool,
    pub private_for_export: bool,
    pub errors: Vec<String>,
    pub privacy_errors: Vec<String>,
}

impl InventoryReport {
    pub fn empty(host: InventoryHost, runtime: InventoryRuntime) -> Self {
        Self {
            schema_version: INVENTORY_SCHEMA_VERSION,
            contract: INVENTORY_CONTRACT.to_string(),
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
            services: Vec::new(),
            events: Vec::new(),
            warnings: Vec::new(),
            summary: InventorySummary {
                source_count: 0,
                source_ok_count: 0,
                path_entry_count: 0,
                tool_count: 0,
                app_count: 0,
                service_count: 0,
                event_count: 0,
                warning_count: 0,
            },
        }
    }

    pub fn recalculate_summary(&mut self) {
        self.summary = summarize(self);
    }
}

pub fn parse_inventory_report(bytes: &[u8]) -> Result<InventoryReport, String> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_INVENTORY_REPORT_BYTES {
        return Err(format!(
            "inventory report must contain 1 to {MAX_INVENTORY_REPORT_BYTES} bytes"
        ));
    }
    serde_json::from_slice(bytes).map_err(|error| format!("invalid inventory report JSON: {error}"))
}

pub fn validate_inventory_report(report: &InventoryReport) -> InventoryValidation {
    let mut errors = Vec::new();
    let mut privacy_errors = Vec::new();

    if report.schema_version != INVENTORY_SCHEMA_VERSION {
        errors.push(format!("schema_version must be {INVENTORY_SCHEMA_VERSION}"));
    }
    if report.contract != INVENTORY_CONTRACT {
        errors.push(format!("contract must be {INVENTORY_CONTRACT}"));
    }
    if !report.read_only || report.writes_attempted {
        errors.push("schema-1 inventory must remain read-only".to_string());
    }
    if report.host.hostname_included || report.host.current_user_included {
        errors.push("inventory must omit host and user identity".to_string());
        privacy_errors.push("host or user identity is included".to_string());
    }
    if report.raw_registry_keys_included {
        errors.push("schema-1 inventory must omit raw registry keys".to_string());
        privacy_errors.push("raw registry keys are included".to_string());
    }
    validate_ascii(&report.host.os, "host.os", 32, &mut errors);
    validate_ascii(&report.host.arch, "host.arch", 32, &mut errors);
    validate_text(&report.runtime.title, "runtime.title", 80, &mut errors);
    validate_ascii(&report.runtime.command, "runtime.command", 32, &mut errors);
    validate_ascii(&report.runtime.version, "runtime.version", 64, &mut errors);
    validate_ascii(
        &report.runtime.scan_mode,
        "runtime.scan_mode",
        64,
        &mut errors,
    );
    validate_ascii(
        &report.runtime.mutation_capability,
        "runtime.mutation_capability",
        32,
        &mut errors,
    );
    if report.runtime.module_schema_version != 1 {
        errors.push("runtime.module_schema_version must be 1".to_string());
    }
    if let Some(module_id) = &report.runtime.module_id
        && !rz0_validation_contract::valid_dotted_id(module_id, 100)
    {
        errors.push("runtime.module_id is invalid".to_string());
    }
    if let Some(generated_at) = &report.generated_at {
        validate_ascii(generated_at, "generated_at", 64, &mut errors);
    }

    validate_count(
        report.sources.len(),
        MAX_INVENTORY_SOURCES,
        "sources",
        &mut errors,
    );
    validate_count(
        report.path_entries.len(),
        MAX_INVENTORY_PATH_ENTRIES,
        "path_entries",
        &mut errors,
    );
    validate_count(
        report.tools.len(),
        MAX_INVENTORY_TOOL_RECORDS,
        "tools",
        &mut errors,
    );
    validate_count(
        report.apps.len(),
        MAX_INVENTORY_APP_RECORDS,
        "apps",
        &mut errors,
    );
    validate_count(
        report.services.len(),
        MAX_INVENTORY_SERVICE_RECORDS,
        "services",
        &mut errors,
    );
    validate_count(
        report.events.len(),
        MAX_INVENTORY_EVENTS,
        "events",
        &mut errors,
    );

    let mut source_ids = BTreeSet::new();
    for source in report.sources.iter().take(MAX_INVENTORY_SOURCES) {
        let unique = source_ids.insert(source.id.as_str());
        if !valid_inventory_id(&source.id, 100) || !unique {
            errors.push("source IDs must be valid and unique".to_string());
        }
        validate_ascii(&source.kind, "source.kind", 32, &mut errors);
        if !matches!(
            source.status.as_str(),
            "ok" | "partial" | "skipped" | "error" | "unavailable"
        ) {
            errors.push("source.status is invalid".to_string());
        }
        if !source.read_only {
            errors.push("inventory sources must remain read-only".to_string());
        }
        validate_warnings(&source.warnings, &mut errors);
    }

    let mut next_order = BTreeMap::<&str, u32>::new();
    for entry in report.path_entries.iter().take(MAX_INVENTORY_PATH_ENTRIES) {
        if !matches!(
            entry.scope.as_str(),
            "process" | "user" | "machine" | "fixture"
        ) {
            errors.push("path entry scope is invalid".to_string());
        }
        let expected = next_order.entry(entry.scope.as_str()).or_default();
        if entry.order != *expected {
            errors.push("path entry order must be contiguous within each scope".to_string());
        }
        *expected = expected.saturating_add(1);
        validate_text(&entry.path, "path entry", 2048, &mut errors);
        if report.path_values_redacted && !valid_path_redaction(&entry.path) {
            errors.push("redacted path entry does not use a canonical token".to_string());
        }
        if !matches!(
            entry.entry_kind.as_str(),
            "directory" | "file" | "missing" | "unknown"
        ) {
            errors.push("path entry kind is invalid".to_string());
        }
        if (entry.exists && entry.entry_kind == "missing")
            || (!entry.exists && matches!(entry.entry_kind.as_str(), "directory" | "file"))
        {
            errors.push("path existence and kind disagree".to_string());
        }
        validate_warnings(&entry.warnings, &mut errors);
    }

    let mut tool_ids = BTreeSet::new();
    for tool in report.tools.iter().take(MAX_INVENTORY_TOOL_RECORDS) {
        let unique = tool_ids.insert(tool.id.as_str());
        if !valid_inventory_id(&tool.id, 100) || !unique {
            errors.push("tool IDs must be valid and unique".to_string());
        }
        validate_text(&tool.display_name, "tool.display_name", 160, &mut errors);
        validate_ascii(&tool.category, "tool.category", 64, &mut errors);
        validate_ascii(&tool.confidence, "tool.confidence", 32, &mut errors);
        if let Some(path) = &tool.executable_path {
            validate_text(path, "tool.executable_path", 2048, &mut errors);
            if report.path_values_redacted && !valid_path_redaction(path) {
                errors.push("redacted tool path does not use a canonical token".to_string());
            }
        }
        if let Some(version) = &tool.version {
            validate_text(version, "tool.version", 256, &mut errors);
        }
        if tool.source_ids.is_empty()
            || tool.source_ids.len() > MAX_INVENTORY_SOURCES
            || !strict_unique(&tool.source_ids)
            || tool
                .source_ids
                .iter()
                .any(|source_id| !source_ids.contains(source_id.as_str()))
        {
            errors.push("tool source IDs must be bounded, unique, and present".to_string());
        }
        validate_warnings(&tool.warnings, &mut errors);
    }

    let mut app_ids = BTreeSet::new();
    for app in report.apps.iter().take(MAX_INVENTORY_APP_RECORDS) {
        let unique = app_ids.insert(app.id.as_str());
        if !valid_inventory_id(&app.id, 100) || !unique {
            errors.push("application IDs must be valid and unique".to_string());
        }
        validate_text(&app.name, "app.name", 240, &mut errors);
        if !source_ids.contains(app.source_id.as_str()) {
            errors.push("application source ID is absent".to_string());
        }
        for (field, value) in [
            ("app.version", app.version.as_deref()),
            ("app.publisher", app.publisher.as_deref()),
        ] {
            if let Some(value) = value {
                validate_text(value, field, 256, &mut errors);
            }
        }
        if app.identifiers.len() > MAX_SOFTWARE_IDENTIFIERS_PER_APP {
            errors.push("application identifiers exceed the per-record ceiling".to_string());
        }
        let mut identifiers = BTreeSet::new();
        for identifier in app
            .identifiers
            .iter()
            .take(MAX_SOFTWARE_IDENTIFIERS_PER_APP)
        {
            if !matches!(
                identifier.kind.as_str(),
                "bundle_id"
                    | "desktop_id"
                    | "manager_package"
                    | "package_id"
                    | "product_code"
                    | "receipt_id"
                    | "registry_product_key_digest"
            ) || identifier.value.trim().is_empty()
                || identifier.value.len() > 256
                || identifier.value.chars().any(char::is_control)
                || !identifiers.insert((&identifier.kind, &identifier.value))
            {
                errors.push("application identifier is invalid or duplicated".to_string());
            }
        }
        if app.identifiers.windows(2).any(|pair| pair[0] >= pair[1]) {
            errors.push("application identifiers must be sorted and unique".to_string());
        }
        if let Some(path) = &app.install_location {
            validate_text(path, "app.install_location", 2048, &mut errors);
            if report.path_values_redacted && !valid_path_redaction(path) {
                errors.push("redacted application path does not use a canonical token".to_string());
            }
        }
        validate_warnings(&app.warnings, &mut errors);
    }

    let mut service_ids = BTreeSet::new();
    for service in report.services.iter().take(MAX_INVENTORY_SERVICE_RECORDS) {
        if !valid_inventory_id(&service.id, 100) || !service_ids.insert(service.id.as_str()) {
            errors.push("service IDs must be valid and unique".to_string());
        }
        validate_text(&service.name, "service.name", 240, &mut errors);
        if !source_ids.contains(service.source_id.as_str()) {
            errors.push("service source ID is absent".to_string());
        }
        if !matches!(service.kind.as_str(), "service" | "persistence") {
            errors.push("service kind is invalid".to_string());
        }
        if !matches!(service.scope.as_str(), "system" | "user") {
            errors.push("service scope is invalid".to_string());
        }
        if let Some(path) = &service.location {
            validate_text(path, "service.location", 2048, &mut errors);
            if report.path_values_redacted && !valid_path_redaction(path) {
                errors.push("redacted service path does not use a canonical token".to_string());
            }
        }
        validate_warnings(&service.warnings, &mut errors);
    }

    for event in report.events.iter().take(MAX_INVENTORY_EVENTS) {
        if !matches!(event.level.as_str(), "info" | "warning" | "error") {
            errors.push("event level is invalid".to_string());
        }
        if !valid_snake_id(&event.code, 64) {
            errors.push("event code is invalid".to_string());
        }
        if event
            .source_id
            .as_ref()
            .is_some_and(|source_id| !source_ids.contains(source_id.as_str()))
        {
            errors.push("event source ID is absent".to_string());
        }
        validate_text(&event.message, "event.message", 512, &mut errors);
    }
    validate_warnings(&report.warnings, &mut errors);

    let warning_count = total_warnings(report);
    if warning_count > MAX_INVENTORY_WARNINGS {
        errors.push(format!(
            "inventory warnings exceed {MAX_INVENTORY_WARNINGS} entries"
        ));
    }
    if report.summary != summarize(report) {
        errors.push("inventory summary does not match report contents".to_string());
    }

    let has_paths = !report.path_entries.is_empty()
        || report
            .tools
            .iter()
            .any(|tool| tool.executable_path.is_some())
        || report.apps.iter().any(|app| app.install_location.is_some())
        || report
            .services
            .iter()
            .any(|service| service.location.is_some());
    if has_paths && !report.path_values_redacted {
        privacy_errors.push("inventory contains unredacted path values".to_string());
    }

    errors.sort();
    errors.dedup();
    privacy_errors.sort();
    privacy_errors.dedup();
    InventoryValidation {
        valid: errors.is_empty(),
        private_for_export: errors.is_empty() && privacy_errors.is_empty(),
        errors,
        privacy_errors,
    }
}

fn summarize(report: &InventoryReport) -> InventorySummary {
    InventorySummary {
        source_count: report.sources.len(),
        source_ok_count: report
            .sources
            .iter()
            .filter(|source| source.status == "ok")
            .count(),
        path_entry_count: report.path_entries.len(),
        tool_count: report.tools.len(),
        app_count: report.apps.len(),
        service_count: report.services.len(),
        event_count: report.events.len(),
        warning_count: total_warnings(report),
    }
}

fn total_warnings(report: &InventoryReport) -> usize {
    report.warnings.len()
        + report
            .sources
            .iter()
            .map(|source| source.warnings.len())
            .sum::<usize>()
        + report
            .path_entries
            .iter()
            .map(|entry| entry.warnings.len())
            .sum::<usize>()
        + report
            .tools
            .iter()
            .map(|tool| tool.warnings.len())
            .sum::<usize>()
        + report
            .apps
            .iter()
            .map(|app| app.warnings.len())
            .sum::<usize>()
        + report
            .services
            .iter()
            .map(|service| service.warnings.len())
            .sum::<usize>()
}

fn validate_count(count: usize, maximum: usize, field: &str, errors: &mut Vec<String>) {
    if count > maximum {
        errors.push(format!("{field} exceed {maximum} entries"));
    }
}

fn validate_warnings(warnings: &[String], errors: &mut Vec<String>) {
    if warnings.len() > MAX_INVENTORY_WARNINGS {
        errors.push("one warning list exceeds the foundation ceiling".to_string());
    }
    for warning in warnings.iter().take(MAX_INVENTORY_WARNINGS) {
        validate_text(warning, "warning", 512, errors);
    }
}

fn validate_ascii(value: &str, field: &str, maximum: usize, errors: &mut Vec<String>) {
    if !rz0_validation_contract::valid_ascii_text(value, maximum) {
        errors.push(format!("{field} is invalid"));
    }
}

fn validate_text(value: &str, field: &str, maximum: usize, errors: &mut Vec<String>) {
    if value.is_empty() || value.len() > maximum || value.chars().any(char::is_control) {
        errors.push(format!("{field} is invalid"));
    }
}

fn strict_unique(values: &[String]) -> bool {
    let set = values.iter().map(String::as_str).collect::<BTreeSet<_>>();
    set.len() == values.len()
}

fn valid_inventory_id(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        })
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.as_bytes()[value.len() - 1].is_ascii_alphanumeric()
        && !value.as_bytes().windows(2).any(|pair| {
            matches!(pair[0], b'.' | b'-' | b'_') && matches!(pair[1], b'.' | b'-' | b'_')
        })
}

fn valid_snake_id(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
        && value.as_bytes()[0].is_ascii_lowercase()
        && value.as_bytes()[value.len() - 1].is_ascii_alphanumeric()
        && !value.as_bytes().windows(2).any(|pair| pair == b"__")
}

fn valid_path_redaction(value: &str) -> bool {
    value.len() == "<redacted:path:0000>".len()
        && value.starts_with("<redacted:path:")
        && value.ends_with('>')
        && value[15..19].bytes().all(|byte| byte.is_ascii_digit())
        && &value[15..19] != "0000"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report() -> InventoryReport {
        InventoryReport::empty(
            InventoryHost {
                os: "test".to_string(),
                arch: "test".to_string(),
                hostname_included: false,
                current_user_included: false,
            },
            InventoryRuntime {
                title: "runtime.zero".to_string(),
                command: "test".to_string(),
                version: "0.0.0".to_string(),
                scan_mode: "test".to_string(),
                mutation_capability: "disabled".to_string(),
                module_schema_version: 1,
                module_id: None,
            },
        )
    }

    #[test]
    fn empty_report_is_read_only_private_and_round_trips() {
        let report = report();
        let validation = validate_inventory_report(&report);
        assert!(validation.valid, "{:?}", validation.errors);
        assert!(validation.private_for_export);
        let bytes = serde_json::to_vec(&report).unwrap();
        assert_eq!(parse_inventory_report(&bytes).unwrap(), report);
    }

    #[test]
    fn exact_redacted_paths_are_private_for_export() {
        let mut report = report();
        report.path_values_redacted = true;
        report.path_entries.push(PathEntry {
            path: "<redacted:path:0001>".to_string(),
            scope: "process".to_string(),
            order: 0,
            exists: true,
            entry_kind: "directory".to_string(),
            warnings: Vec::new(),
        });
        report.recalculate_summary();
        let validation = validate_inventory_report(&report);
        assert!(validation.valid, "{:?}", validation.errors);
        assert!(validation.private_for_export);
    }

    #[test]
    fn redacted_service_locations_are_export_private_but_labels_remain_metadata() {
        let mut report = report();
        report.path_values_redacted = true;
        report.sources.push(InventorySource {
            id: "service.fixture".to_string(),
            kind: "filesystem_metadata".to_string(),
            status: "ok".to_string(),
            duration_ms: Some(1),
            read_only: true,
            warnings: Vec::new(),
        });
        report.services.push(ServiceRecord {
            id: "service.fixture.alpha".to_string(),
            name: "alpha.service".to_string(),
            source_id: "service.fixture".to_string(),
            kind: "service".to_string(),
            scope: "system".to_string(),
            enabled: None,
            location: Some("<redacted:path:0001>".to_string()),
            warnings: Vec::new(),
        });
        report.recalculate_summary();
        let validation = validate_inventory_report(&report);
        assert!(validation.valid, "{:?}", validation.errors);
        assert!(validation.private_for_export);
        assert_eq!(report.summary.service_count, 1);
    }

    #[test]
    fn raw_paths_and_identity_are_not_private_for_export() {
        let mut report = report();
        report.host.hostname_included = true;
        report.path_entries.push(PathEntry {
            path: "/private/example".to_string(),
            scope: "process".to_string(),
            order: 0,
            exists: false,
            entry_kind: "missing".to_string(),
            warnings: Vec::new(),
        });
        report.recalculate_summary();
        let validation = validate_inventory_report(&report);
        assert!(!validation.valid);
        assert!(!validation.private_for_export);
        assert_eq!(validation.privacy_errors.len(), 2);
    }

    #[test]
    fn summary_cross_references_and_unknown_fields_fail_closed() {
        let mut invalid = report();
        invalid.summary.app_count = 1;
        invalid.events.push(InventoryEvent {
            level: "info".to_string(),
            code: "source_completed".to_string(),
            source_id: Some("absent.source".to_string()),
            message: "source completed".to_string(),
        });
        let validation = validate_inventory_report(&invalid);
        assert!(!validation.valid);
        assert!(validation.errors.len() >= 2);

        let mut value = serde_json::to_value(report()).unwrap();
        value["future"] = serde_json::Value::Bool(true);
        assert!(parse_inventory_report(&serde_json::to_vec(&value).unwrap()).is_err());
    }

    #[test]
    fn document_and_collection_limits_fail_closed() {
        assert!(parse_inventory_report(&[]).is_err());
        assert!(
            parse_inventory_report(&vec![b'x'; MAX_INVENTORY_REPORT_BYTES as usize + 1]).is_err()
        );
        let mut report = report();
        report.sources = (0..=MAX_INVENTORY_SOURCES)
            .map(|index| InventorySource {
                id: format!("source.{index}"),
                kind: "fixture".to_string(),
                status: "ok".to_string(),
                duration_ms: None,
                read_only: true,
                warnings: Vec::new(),
            })
            .collect();
        report.recalculate_summary();
        assert!(!validate_inventory_report(&report).valid);
    }
}
