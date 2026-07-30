use std::{collections::BTreeSet, fmt::Write as _};

use rz0_error_contract::FoundationErrorCode;
use serde::{Deserialize, Serialize};

pub const DIAGNOSTICS_SCHEMA_VERSION: u16 = 1;
pub const DIAGNOSTICS_CONTRACT: &str = "foundation_diagnostics";
pub const MAX_DIAGNOSTIC_CHECKS: usize = rz0_resource_contract::MAX_DIAGNOSTIC_CHECKS;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticReport {
    pub schema_version: u16,
    pub contract: String,
    pub product: String,
    pub command: String,
    pub version: String,
    pub configuration_sha256: String,
    pub platform: DiagnosticPlatform,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub production_execution_authorized: bool,
    pub privacy: DiagnosticPrivacy,
    pub checks: Vec<DiagnosticCheck>,
    pub summary: DiagnosticSummary,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticPlatform {
    pub os: String,
    pub arch: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticPrivacy {
    pub hostname_included: bool,
    pub current_user_included: bool,
    pub current_directory_included: bool,
    pub environment_values_included: bool,
    pub raw_paths_included: bool,
}

impl DiagnosticPrivacy {
    pub const fn private_by_default() -> Self {
        Self {
            hostname_included: false,
            current_user_included: false,
            current_directory_included: false,
            environment_values_included: false,
            raw_paths_included: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticCheck {
    pub id: DiagnosticCheckId,
    pub status: DiagnosticStatus,
    pub detail: String,
    pub error_code: Option<FoundationErrorCode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCheckId {
    RuntimeIdentity,
    PlatformIdentity,
    ConfigurationPolicy,
    SafetyPosture,
    StoreMutationPolicy,
    ModuleExecutionPolicy,
    NetworkPolicy,
    ExternalAutomationPolicy,
    PrivacyDefault,
}

pub const CANONICAL_DIAGNOSTIC_CHECKS: [DiagnosticCheckId; 9] = [
    DiagnosticCheckId::RuntimeIdentity,
    DiagnosticCheckId::PlatformIdentity,
    DiagnosticCheckId::ConfigurationPolicy,
    DiagnosticCheckId::SafetyPosture,
    DiagnosticCheckId::StoreMutationPolicy,
    DiagnosticCheckId::ModuleExecutionPolicy,
    DiagnosticCheckId::NetworkPolicy,
    DiagnosticCheckId::ExternalAutomationPolicy,
    DiagnosticCheckId::PrivacyDefault,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticStatus {
    Pass,
    Blocked,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DiagnosticSummary {
    pub check_count: usize,
    pub pass_count: usize,
    pub blocked_count: usize,
    pub unavailable_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticValidation {
    pub valid: bool,
    pub errors: Vec<String>,
}

pub fn foundation_diagnostics(
    product: &str,
    command: &str,
    version: &str,
    os: &str,
    arch: &str,
) -> DiagnosticReport {
    let configuration = rz0_configuration_contract::default_configuration();
    let configuration_sha256 = rz0_configuration_contract::configuration_sha256(&configuration)
        .expect("built-in foundation configuration is canonical");
    let checks = vec![
        pass(
            DiagnosticCheckId::RuntimeIdentity,
            "runtime identity available",
        ),
        pass(
            DiagnosticCheckId::PlatformIdentity,
            "platform identity available",
        ),
        pass(
            DiagnosticCheckId::ConfigurationPolicy,
            "built-in fail-closed configuration validated",
        ),
        pass(
            DiagnosticCheckId::SafetyPosture,
            "report first, dry run first, quarantine first",
        ),
        pass(
            DiagnosticCheckId::StoreMutationPolicy,
            "store initialization requires explicit confirmation",
        ),
        blocked(
            DiagnosticCheckId::ModuleExecutionPolicy,
            "production module execution is disabled",
            FoundationErrorCode::ExecutionNotAuthorized,
        ),
        blocked(
            DiagnosticCheckId::NetworkPolicy,
            "network access is disabled by default",
            FoundationErrorCode::CapabilityDenied,
        ),
        blocked(
            DiagnosticCheckId::ExternalAutomationPolicy,
            "external automation is not configured",
            FoundationErrorCode::ExecutionNotAuthorized,
        ),
        pass(
            DiagnosticCheckId::PrivacyDefault,
            "host, user, environment, directory, and raw path values are omitted",
        ),
    ];
    let summary = summarize(&checks);
    DiagnosticReport {
        schema_version: DIAGNOSTICS_SCHEMA_VERSION,
        contract: DIAGNOSTICS_CONTRACT.to_string(),
        product: product.to_string(),
        command: command.to_string(),
        version: version.to_string(),
        configuration_sha256,
        platform: DiagnosticPlatform {
            os: os.to_string(),
            arch: arch.to_string(),
        },
        read_only: true,
        writes_attempted: false,
        production_execution_authorized: false,
        privacy: DiagnosticPrivacy::private_by_default(),
        checks,
        summary,
    }
}

pub fn validate_diagnostic_report(report: &DiagnosticReport) -> DiagnosticValidation {
    let mut errors = Vec::new();
    if report.schema_version != DIAGNOSTICS_SCHEMA_VERSION {
        errors.push(format!(
            "schema_version must be {DIAGNOSTICS_SCHEMA_VERSION}"
        ));
    }
    if report.contract != DIAGNOSTICS_CONTRACT {
        errors.push(format!("contract must be {DIAGNOSTICS_CONTRACT}"));
    }
    for (field, value, max) in [
        ("product", report.product.as_str(), 80),
        ("command", report.command.as_str(), 32),
        ("version", report.version.as_str(), 64),
        ("platform.os", report.platform.os.as_str(), 32),
        ("platform.arch", report.platform.arch.as_str(), 32),
    ] {
        if !rz0_validation_contract::valid_ascii_text(value, max) {
            errors.push(format!("{field} is invalid"));
        }
    }
    if !rz0_validation_contract::valid_sha256(&report.configuration_sha256) {
        errors.push("configuration_sha256 is invalid".to_string());
    }
    let expected_configuration_sha256 = rz0_configuration_contract::configuration_sha256(
        &rz0_configuration_contract::default_configuration(),
    );
    if expected_configuration_sha256.as_deref() != Ok(report.configuration_sha256.as_str()) {
        errors.push("diagnostics do not bind the canonical configuration".to_string());
    }
    if !report.read_only || report.writes_attempted || report.production_execution_authorized {
        errors.push("schema-1 diagnostics must remain read-only and unauthorized".to_string());
    }
    if report.privacy != DiagnosticPrivacy::private_by_default() {
        errors.push("schema-1 diagnostics must omit sensitive host values".to_string());
    }
    if report.checks.len() > MAX_DIAGNOSTIC_CHECKS {
        errors.push(format!(
            "diagnostics exceed the {MAX_DIAGNOSTIC_CHECKS}-check ceiling"
        ));
    }
    let ids = report
        .checks
        .iter()
        .map(|check| check.id)
        .collect::<Vec<_>>();
    let unique = ids.iter().copied().collect::<BTreeSet<_>>();
    if ids.as_slice() != CANONICAL_DIAGNOSTIC_CHECKS || unique.len() != ids.len() {
        errors.push("diagnostics must contain the exact canonical check set".to_string());
    }
    for check in &report.checks {
        if !rz0_validation_contract::valid_ascii_text(&check.detail, 160) {
            errors.push("diagnostic detail is invalid".to_string());
        }
        match check.status {
            DiagnosticStatus::Pass if check.error_code.is_some() => {
                errors.push("passing diagnostics cannot contain an error code".to_string());
            }
            DiagnosticStatus::Blocked | DiagnosticStatus::Unavailable
                if check.error_code.is_none() =>
            {
                errors.push("unresolved diagnostics require an error code".to_string());
            }
            _ => {}
        }
    }
    if report.summary != summarize(&report.checks) {
        errors.push("diagnostic summary does not match checks".to_string());
    }
    errors.sort();
    errors.dedup();
    DiagnosticValidation {
        valid: errors.is_empty(),
        errors,
    }
}

pub fn diagnostic_json(report: &DiagnosticReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report).map(|json| format!("{json}\n"))
}

pub fn diagnostic_text(report: &DiagnosticReport) -> String {
    let mut output = format!(
        "{} doctor\n\ncontract: {}\nschema_version: {}\ncommand: {}\nversion: {}\nconfiguration_sha256: {}\nos: {}\narch: {}\nread_only: {}\nwrites_attempted: {}\nproduction_execution_authorized: {}\n",
        report.product,
        report.contract,
        report.schema_version,
        report.command,
        report.version,
        report.configuration_sha256,
        report.platform.os,
        report.platform.arch,
        report.read_only,
        report.writes_attempted,
        report.production_execution_authorized,
    );
    let _ = writeln!(
        output,
        "privacy: host={} user={} current_directory={} environment_values={} raw_paths={}",
        report.privacy.hostname_included,
        report.privacy.current_user_included,
        report.privacy.current_directory_included,
        report.privacy.environment_values_included,
        report.privacy.raw_paths_included,
    );
    let _ = writeln!(output, "checks:");
    for check in &report.checks {
        let _ = writeln!(
            output,
            "  {}: {} - {}",
            check_id_name(check.id),
            status_name(check.status),
            check.detail
        );
    }
    let _ = writeln!(
        output,
        "summary: checks={} pass={} blocked={} unavailable={}",
        report.summary.check_count,
        report.summary.pass_count,
        report.summary.blocked_count,
        report.summary.unavailable_count
    );
    output
}

fn pass(id: DiagnosticCheckId, detail: &str) -> DiagnosticCheck {
    DiagnosticCheck {
        id,
        status: DiagnosticStatus::Pass,
        detail: detail.to_string(),
        error_code: None,
    }
}

fn blocked(
    id: DiagnosticCheckId,
    detail: &str,
    error_code: FoundationErrorCode,
) -> DiagnosticCheck {
    DiagnosticCheck {
        id,
        status: DiagnosticStatus::Blocked,
        detail: detail.to_string(),
        error_code: Some(error_code),
    }
}

fn summarize(checks: &[DiagnosticCheck]) -> DiagnosticSummary {
    DiagnosticSummary {
        check_count: checks.len(),
        pass_count: checks
            .iter()
            .filter(|check| check.status == DiagnosticStatus::Pass)
            .count(),
        blocked_count: checks
            .iter()
            .filter(|check| check.status == DiagnosticStatus::Blocked)
            .count(),
        unavailable_count: checks
            .iter()
            .filter(|check| check.status == DiagnosticStatus::Unavailable)
            .count(),
    }
}

const fn check_id_name(id: DiagnosticCheckId) -> &'static str {
    match id {
        DiagnosticCheckId::RuntimeIdentity => "runtime_identity",
        DiagnosticCheckId::PlatformIdentity => "platform_identity",
        DiagnosticCheckId::ConfigurationPolicy => "configuration_policy",
        DiagnosticCheckId::SafetyPosture => "safety_posture",
        DiagnosticCheckId::StoreMutationPolicy => "store_mutation_policy",
        DiagnosticCheckId::ModuleExecutionPolicy => "module_execution_policy",
        DiagnosticCheckId::NetworkPolicy => "network_policy",
        DiagnosticCheckId::ExternalAutomationPolicy => "external_automation_policy",
        DiagnosticCheckId::PrivacyDefault => "privacy_default",
    }
}

const fn status_name(status: DiagnosticStatus) -> &'static str {
    match status {
        DiagnosticStatus::Pass => "pass",
        DiagnosticStatus::Blocked => "blocked",
        DiagnosticStatus::Unavailable => "unavailable",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report() -> DiagnosticReport {
        foundation_diagnostics("runtime.zero", "rz0", "0.1.0", "test-os", "test-arch")
    }

    #[test]
    fn canonical_diagnostics_are_private_bounded_and_read_only() {
        let report = report();
        let validation = validate_diagnostic_report(&report);
        assert!(validation.valid, "{:?}", validation.errors);
        assert_eq!(report.summary.check_count, 9);
        assert_eq!(report.summary.pass_count, 6);
        assert_eq!(report.summary.blocked_count, 3);
        assert!(!report.production_execution_authorized);
    }

    #[test]
    fn json_and_text_omit_host_values_and_local_paths() {
        let report = report();
        let json = diagnostic_json(&report).unwrap();
        let text = diagnostic_text(&report);
        for output in [&json, &text] {
            assert!(!output.contains("private-host-value"));
            assert!(!output.contains("private-user-value"));
            assert!(!output.contains("/Users/"));
        }
        assert!(json.contains("\"hostname_included\": false"));
        assert!(json.contains("\"current_directory_included\": false"));
    }

    #[test]
    fn drift_and_fabricated_authority_fail_closed() {
        let mut report = report();
        report.production_execution_authorized = true;
        report.checks.swap(0, 1);
        report.summary.pass_count = 99;
        let validation = validate_diagnostic_report(&report);
        assert!(!validation.valid);
        assert!(validation.errors.len() >= 3);
    }

    #[test]
    fn unknown_fields_fail_deserialization() {
        let json = diagnostic_json(&report()).unwrap();
        let drifted = json.replacen(
            "\"schema_version\": 1",
            "\"schema_version\": 1,\n  \"future\": true",
            1,
        );
        assert!(serde_json::from_str::<DiagnosticReport>(&drifted).is_err());
    }
}
