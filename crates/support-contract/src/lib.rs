use std::collections::BTreeSet;

use rz0_diagnostics_contract::{DiagnosticReport, DiagnosticSummary, validate_diagnostic_report};
use rz0_inventory_contract::{InventoryReport, InventorySummary, validate_inventory_report};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SUPPORT_SCHEMA_VERSION: u16 = 1;
pub const SUPPORT_CONTRACT: &str = "privacy_reviewed_support_report";
pub const SUPPORT_INPUT_CONTRACT: &str = "support_report_input";
pub const MAX_SUPPORT_REPORT_BYTES: u64 = rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES;
pub const MAX_SUPPORT_INPUT_BYTES: u64 = rz0_resource_contract::MAX_INVENTORY_REPORT_BYTES
    + rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupportReportInput {
    pub schema_version: u16,
    pub contract: String,
    pub inventory: InventoryReport,
    pub diagnostics: DiagnosticReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupportReport {
    pub schema_version: u16,
    pub contract: String,
    pub report_id: String,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub product_execution_authorized: bool,
    pub release_authorized: bool,
    pub external_sharing_authorized: bool,
    pub local_export_ready: bool,
    pub platform: SupportPlatform,
    pub configuration_sha256: String,
    pub input_digests: SupportInputDigests,
    pub privacy: SupportPrivacy,
    pub inventory: InventorySummary,
    pub inventory_sources: Vec<SupportSource>,
    pub diagnostics: DiagnosticSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupportPlatform {
    pub os: String,
    pub arch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupportInputDigests {
    pub inventory_sha256: String,
    pub diagnostics_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupportPrivacy {
    pub privacy_reviewed: bool,
    pub raw_inventory_embedded: bool,
    pub raw_diagnostics_embedded: bool,
    pub local_paths_included: bool,
    pub host_identity_included: bool,
    pub user_identity_included: bool,
    pub environment_values_included: bool,
    pub process_output_included: bool,
    pub application_names_included: bool,
    pub free_form_warnings_included: bool,
}

impl SupportPrivacy {
    pub const fn summary_only() -> Self {
        Self {
            privacy_reviewed: true,
            raw_inventory_embedded: false,
            raw_diagnostics_embedded: false,
            local_paths_included: false,
            host_identity_included: false,
            user_identity_included: false,
            environment_values_included: false,
            process_output_included: false,
            application_names_included: false,
            free_form_warnings_included: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SupportSource {
    pub id: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportValidation {
    pub valid: bool,
    pub errors: Vec<String>,
}

pub fn decode_support_input(bytes: &[u8]) -> Result<SupportReportInput, String> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_SUPPORT_INPUT_BYTES {
        return Err(format!(
            "support input must contain 1 to {MAX_SUPPORT_INPUT_BYTES} bytes"
        ));
    }
    let input: SupportReportInput = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid support input JSON: {error}"))?;
    if input.schema_version != SUPPORT_SCHEMA_VERSION {
        return Err(format!("schema_version must be {SUPPORT_SCHEMA_VERSION}"));
    }
    if input.contract != SUPPORT_INPUT_CONTRACT {
        return Err(format!("contract must be {SUPPORT_INPUT_CONTRACT}"));
    }
    build_support_report(&input.inventory, &input.diagnostics)?;
    Ok(input)
}

pub fn build_support_report_from_input(
    input: &SupportReportInput,
) -> Result<SupportReport, String> {
    if input.schema_version != SUPPORT_SCHEMA_VERSION || input.contract != SUPPORT_INPUT_CONTRACT {
        return Err("support input identity is invalid".to_string());
    }
    build_support_report(&input.inventory, &input.diagnostics)
}

pub fn build_support_report(
    inventory: &InventoryReport,
    diagnostics: &DiagnosticReport,
) -> Result<SupportReport, String> {
    let inventory_validation = validate_inventory_report(inventory);
    let diagnostics_validation = validate_diagnostic_report(diagnostics);
    let mut errors = inventory_validation.errors;
    errors.extend(inventory_validation.privacy_errors);
    errors.extend(diagnostics_validation.errors);
    if inventory.host.os != diagnostics.platform.os
        || inventory.host.arch != diagnostics.platform.arch
    {
        errors.push("inventory and diagnostics platform identity disagree".to_string());
    }
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors.join("; "));
    }

    let inventory_sha256 = input_digest(
        b"runtime.zero.support-input.inventory.v1\0",
        &serde_json::to_vec(inventory).map_err(|error| error.to_string())?,
    );
    let diagnostics_sha256 = input_digest(
        b"runtime.zero.support-input.diagnostics.v1\0",
        &serde_json::to_vec(diagnostics).map_err(|error| error.to_string())?,
    );
    let report = SupportReport {
        schema_version: SUPPORT_SCHEMA_VERSION,
        contract: SUPPORT_CONTRACT.to_string(),
        report_id: report_id(&inventory_sha256, &diagnostics_sha256),
        read_only: true,
        writes_attempted: false,
        product_execution_authorized: false,
        release_authorized: false,
        external_sharing_authorized: false,
        local_export_ready: true,
        platform: SupportPlatform {
            os: diagnostics.platform.os.clone(),
            arch: diagnostics.platform.arch.clone(),
        },
        configuration_sha256: diagnostics.configuration_sha256.clone(),
        input_digests: SupportInputDigests {
            inventory_sha256,
            diagnostics_sha256,
        },
        privacy: SupportPrivacy::summary_only(),
        inventory: inventory.summary.clone(),
        inventory_sources: inventory
            .sources
            .iter()
            .map(|source| SupportSource {
                id: source.id.clone(),
                status: source.status.clone(),
            })
            .collect(),
        diagnostics: diagnostics.summary.clone(),
    };
    let validation = validate_support_report(&report);
    if validation.valid {
        Ok(report)
    } else {
        Err(validation.errors.join("; "))
    }
}

pub fn validate_support_report(report: &SupportReport) -> SupportValidation {
    let mut errors = Vec::new();
    if report.schema_version != SUPPORT_SCHEMA_VERSION {
        errors.push(format!("schema_version must be {SUPPORT_SCHEMA_VERSION}"));
    }
    if report.contract != SUPPORT_CONTRACT {
        errors.push(format!("contract must be {SUPPORT_CONTRACT}"));
    }
    if !rz0_validation_contract::valid_evidence_reference(&report.report_id, 120) {
        errors.push("report_id is invalid".to_string());
    }
    if !report.read_only
        || report.writes_attempted
        || report.product_execution_authorized
        || report.release_authorized
        || report.external_sharing_authorized
        || !report.local_export_ready
    {
        errors.push("support report authority or write posture is invalid".to_string());
    }
    if report.privacy != SupportPrivacy::summary_only() {
        errors.push("support report must use the exact summary-only privacy posture".to_string());
    }
    for (field, value, maximum) in [
        ("platform.os", report.platform.os.as_str(), 32),
        ("platform.arch", report.platform.arch.as_str(), 32),
    ] {
        if !rz0_validation_contract::valid_ascii_text(value, maximum) {
            errors.push(format!("{field} is invalid"));
        }
    }
    for digest in [
        &report.configuration_sha256,
        &report.input_digests.inventory_sha256,
        &report.input_digests.diagnostics_sha256,
    ] {
        if !rz0_validation_contract::valid_sha256(digest) {
            errors.push("support report digest is invalid".to_string());
        }
    }
    if rz0_validation_contract::valid_sha256(&report.input_digests.inventory_sha256)
        && rz0_validation_contract::valid_sha256(&report.input_digests.diagnostics_sha256)
        && report.report_id
            != report_id(
                &report.input_digests.inventory_sha256,
                &report.input_digests.diagnostics_sha256,
            )
    {
        errors.push("report_id does not bind both input digests".to_string());
    }

    if report.inventory_sources.len() > rz0_resource_contract::MAX_INVENTORY_SOURCES
        || report.inventory_sources.len() != report.inventory.source_count
    {
        errors.push("inventory source summary count is invalid".to_string());
    }
    let mut source_ids = BTreeSet::new();
    for source in &report.inventory_sources {
        if !source_ids.insert(source.id.as_str())
            || !rz0_validation_contract::valid_ascii_text(&source.id, 100)
            || !matches!(
                source.status.as_str(),
                "ok" | "partial" | "skipped" | "error" | "unavailable"
            )
        {
            errors.push("inventory source summary is invalid".to_string());
        }
    }
    if report.inventory.source_count > rz0_resource_contract::MAX_INVENTORY_SOURCES
        || report.inventory.path_entry_count > rz0_resource_contract::MAX_INVENTORY_PATH_ENTRIES
        || report.inventory.tool_count > rz0_resource_contract::MAX_INVENTORY_TOOL_RECORDS
        || report.inventory.app_count > rz0_resource_contract::MAX_INVENTORY_APP_RECORDS
        || report.inventory.event_count > rz0_resource_contract::MAX_INVENTORY_EVENTS
        || report.inventory.warning_count > rz0_resource_contract::MAX_INVENTORY_WARNINGS
        || report.inventory.source_ok_count > report.inventory.source_count
    {
        errors.push("inventory summary exceeds foundation bounds".to_string());
    }
    if report.diagnostics.check_count > rz0_resource_contract::MAX_DIAGNOSTIC_CHECKS
        || report.diagnostics.pass_count
            + report.diagnostics.blocked_count
            + report.diagnostics.unavailable_count
            != report.diagnostics.check_count
    {
        errors.push("diagnostic summary is invalid".to_string());
    }

    errors.sort();
    errors.dedup();
    SupportValidation {
        valid: errors.is_empty(),
        errors,
    }
}

pub fn support_json(report: &SupportReport) -> Result<String, String> {
    let validation = validate_support_report(report);
    if !validation.valid {
        return Err(validation.errors.join("; "));
    }
    serde_json::to_string_pretty(report)
        .map(|json| format!("{json}\n"))
        .map_err(|error| format!("render support report: {error}"))
}

pub fn support_text(report: &SupportReport) -> Result<String, String> {
    let validation = validate_support_report(report);
    if !validation.valid {
        return Err(validation.errors.join("; "));
    }
    Ok(format!(
        "runtime.zero support report\n\ncontract: {}\nschema_version: {}\nreport_id: {}\nos: {}\narch: {}\nread_only: true\nwrites_attempted: false\nlocal_export_ready: true\nexternal_sharing_authorized: false\nproduct_execution_authorized: false\nrelease_authorized: false\ninventory: sources={} paths={} tools={} apps={} events={} warnings={}\ndiagnostics: checks={} pass={} blocked={} unavailable={}\nprivacy: summary-only; raw reports, paths, identities, application names, process output, and free-form warnings omitted\n",
        report.contract,
        report.schema_version,
        report.report_id,
        report.platform.os,
        report.platform.arch,
        report.inventory.source_count,
        report.inventory.path_entry_count,
        report.inventory.tool_count,
        report.inventory.app_count,
        report.inventory.event_count,
        report.inventory.warning_count,
        report.diagnostics.check_count,
        report.diagnostics.pass_count,
        report.diagnostics.blocked_count,
        report.diagnostics.unavailable_count,
    ))
}

pub fn decode_support_report(bytes: &[u8]) -> Result<SupportReport, String> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_SUPPORT_REPORT_BYTES {
        return Err("support report is empty or oversized".to_string());
    }
    let report: SupportReport =
        serde_json::from_slice(bytes).map_err(|error| format!("parse support report: {error}"))?;
    let validation = validate_support_report(&report);
    if validation.valid {
        Ok(report)
    } else {
        Err(validation.errors.join("; "))
    }
}

fn input_digest(domain: &[u8], bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(domain);
    digest.update((bytes.len() as u64).to_be_bytes());
    digest.update(bytes);
    format!("{:x}", digest.finalize())
}

fn report_id(inventory_sha256: &str, diagnostics_sha256: &str) -> String {
    format!(
        "support:{}-{}",
        &inventory_sha256[..inventory_sha256.len().min(12)],
        &diagnostics_sha256[..diagnostics_sha256.len().min(12)]
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use rz0_diagnostics_contract::foundation_diagnostics;
    use rz0_inventory_contract::{InventoryHost, InventoryRuntime};

    fn inputs() -> (InventoryReport, DiagnosticReport) {
        let inventory = InventoryReport::empty(
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
        );
        let diagnostics =
            foundation_diagnostics("runtime.zero", "rz0", "0.1.0", "test-os", "test-arch");
        (inventory, diagnostics)
    }

    #[test]
    fn strict_input_envelope_is_bounded_and_foundation_owned() {
        let (inventory, diagnostics) = inputs();
        let input = SupportReportInput {
            schema_version: SUPPORT_SCHEMA_VERSION,
            contract: SUPPORT_INPUT_CONTRACT.to_string(),
            inventory,
            diagnostics,
        };
        let bytes = serde_json::to_vec(&input).unwrap();
        let decoded = decode_support_input(&bytes).unwrap();
        assert_eq!(
            build_support_report_from_input(&decoded).unwrap(),
            build_support_report(&decoded.inventory, &decoded.diagnostics).unwrap()
        );

        let mut value = serde_json::to_value(input).unwrap();
        value["future"] = serde_json::Value::Bool(true);
        assert!(decode_support_input(&serde_json::to_vec(&value).unwrap()).is_err());
        assert!(decode_support_input(&vec![b'x'; MAX_SUPPORT_INPUT_BYTES as usize + 1]).is_err());
    }

    #[test]
    fn deterministic_summary_omits_raw_inputs_and_authority() {
        let (inventory, diagnostics) = inputs();
        let first = build_support_report(&inventory, &diagnostics).unwrap();
        let second = build_support_report(&inventory, &diagnostics).unwrap();
        assert_eq!(first, second);
        assert!(!first.external_sharing_authorized);
        assert!(!first.product_execution_authorized);
        assert!(!first.release_authorized);
        let json = support_json(&first).unwrap();
        assert!(!json.contains("runtime identity available"));
        assert!(!json.contains("path_entries\""));
        assert_eq!(decode_support_report(json.as_bytes()).unwrap(), first);
    }

    #[test]
    fn private_input_and_platform_disagreement_fail_closed() {
        let (mut inventory, diagnostics) = inputs();
        inventory
            .path_entries
            .push(rz0_inventory_contract::PathEntry {
                path: "/private/example".to_string(),
                scope: "process".to_string(),
                order: 0,
                exists: false,
                entry_kind: "missing".to_string(),
                warnings: Vec::new(),
            });
        inventory.recalculate_summary();
        assert!(build_support_report(&inventory, &diagnostics).is_err());

        let (mut inventory, diagnostics) = inputs();
        inventory.host.os = "other-os".to_string();
        assert!(build_support_report(&inventory, &diagnostics).is_err());
    }

    #[test]
    fn fabricated_authority_summary_drift_and_unknown_fields_fail_closed() {
        let (inventory, diagnostics) = inputs();
        let mut report = build_support_report(&inventory, &diagnostics).unwrap();
        report.external_sharing_authorized = true;
        report.inventory.source_count = 1;
        assert!(!validate_support_report(&report).valid);

        let valid = build_support_report(&inventory, &diagnostics).unwrap();
        let json = support_json(&valid).unwrap();
        let drifted = json.replacen(
            "\"schema_version\": 1",
            "\"schema_version\": 1,\n  \"future\": true",
            1,
        );
        assert!(decode_support_report(drifted.as_bytes()).is_err());
    }
}
