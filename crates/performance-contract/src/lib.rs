use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub const PERFORMANCE_SCHEMA_VERSION: u16 = 2;
pub const PERFORMANCE_CONTRACT: &str = "final_artifact_performance";
pub const MIN_PERFORMANCE_SAMPLES: u32 = 10;
pub const MAX_PERFORMANCE_SAMPLES: u32 = rz0_resource_contract::MAX_PERFORMANCE_SAMPLES;
pub const MAX_PERFORMANCE_OPERATIONS: usize = rz0_resource_contract::MAX_PERFORMANCE_OPERATIONS;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PerformanceEvidence {
    pub schema_version: u16,
    pub contract: String,
    pub evidence_id: String,
    pub target: String,
    pub source_commit: String,
    pub artifact_sha256: String,
    pub sample_count: u32,
    pub decision: PerformanceDecision,
    pub release_authorized: bool,
    pub budget: PerformanceBudget,
    pub operations: Vec<OperationMeasurement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PerformanceDecision {
    Pass,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PerformanceBudget {
    pub p95_wall_time_us: u64,
    pub maximum_wall_time_us: u64,
    pub maximum_resident_bytes: u64,
    pub maximum_output_bytes: u64,
}

impl PerformanceBudget {
    pub const FINAL_ARTIFACT_BASELINE: Self = Self {
        p95_wall_time_us: 1_000_000,
        maximum_wall_time_us: 2_000_000,
        maximum_resident_bytes: 64 * 1024 * 1024,
        maximum_output_bytes: 2 * 1024 * 1024,
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OperationMeasurement {
    pub operation: PerformanceOperation,
    pub p50_wall_time_us: u64,
    pub p95_wall_time_us: u64,
    pub maximum_wall_time_us: u64,
    pub maximum_resident_bytes: u64,
    pub maximum_stdout_bytes: u64,
    pub maximum_stderr_bytes: u64,
    pub successful_samples: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PerformanceOperation {
    Version,
    DoctorText,
    DoctorJson,
    CoreScanText,
    CoreScanJson,
    AppsJson,
    MonitorJson,
    ReportJson,
    DashboardJson,
}

pub const CANONICAL_PERFORMANCE_OPERATIONS: [PerformanceOperation; 9] = [
    PerformanceOperation::Version,
    PerformanceOperation::DoctorText,
    PerformanceOperation::DoctorJson,
    PerformanceOperation::CoreScanText,
    PerformanceOperation::CoreScanJson,
    PerformanceOperation::AppsJson,
    PerformanceOperation::MonitorJson,
    PerformanceOperation::ReportJson,
    PerformanceOperation::DashboardJson,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerformanceValidation {
    pub valid: bool,
    pub errors: Vec<String>,
}

pub fn validate_performance_evidence(evidence: &PerformanceEvidence) -> PerformanceValidation {
    let mut errors = Vec::new();
    if evidence.schema_version != PERFORMANCE_SCHEMA_VERSION {
        errors.push(format!(
            "schema_version must be {PERFORMANCE_SCHEMA_VERSION}"
        ));
    }
    if evidence.contract != PERFORMANCE_CONTRACT {
        errors.push(format!("contract must be {PERFORMANCE_CONTRACT}"));
    }
    if !rz0_validation_contract::valid_evidence_reference(&evidence.evidence_id, 120) {
        errors.push("evidence_id is invalid".to_string());
    }
    if !rz0_validation_contract::valid_dotted_id(&evidence.target, 100) {
        errors.push("target is invalid".to_string());
    }
    if !rz0_validation_contract::valid_lower_hex(&evidence.source_commit, 40) {
        errors.push("source_commit must be a full lowercase Git SHA-1".to_string());
    }
    if !rz0_validation_contract::valid_sha256(&evidence.artifact_sha256) {
        errors.push("artifact_sha256 is invalid".to_string());
    }
    if !(MIN_PERFORMANCE_SAMPLES..=MAX_PERFORMANCE_SAMPLES).contains(&evidence.sample_count) {
        errors.push(format!(
            "sample_count must be {MIN_PERFORMANCE_SAMPLES}..={MAX_PERFORMANCE_SAMPLES}"
        ));
    }
    if evidence.release_authorized {
        errors.push("performance evidence cannot authorize release".to_string());
    }
    if evidence.budget != PerformanceBudget::FINAL_ARTIFACT_BASELINE {
        errors.push("schema-2 performance evidence must use the canonical budget".to_string());
    }
    if evidence.operations.len() > MAX_PERFORMANCE_OPERATIONS {
        errors.push("performance evidence exceeds the operation ceiling".to_string());
    }
    let observed = evidence
        .operations
        .iter()
        .map(|measurement| measurement.operation)
        .collect::<Vec<_>>();
    let unique = observed.iter().copied().collect::<BTreeSet<_>>();
    if observed.as_slice() != CANONICAL_PERFORMANCE_OPERATIONS || unique.len() != observed.len() {
        errors.push("performance evidence must contain the exact canonical operations".to_string());
    }

    let mut within_budget = true;
    for measurement in &evidence.operations {
        if measurement.successful_samples != evidence.sample_count {
            errors
                .push("every operation must contain the exact successful sample count".to_string());
        }
        if measurement.p50_wall_time_us > measurement.p95_wall_time_us
            || measurement.p95_wall_time_us > measurement.maximum_wall_time_us
        {
            errors.push("operation timing percentiles are inconsistent".to_string());
        }
        let output = measurement
            .maximum_stdout_bytes
            .saturating_add(measurement.maximum_stderr_bytes);
        within_budget &= measurement.p95_wall_time_us <= evidence.budget.p95_wall_time_us
            && measurement.maximum_wall_time_us <= evidence.budget.maximum_wall_time_us
            && measurement.maximum_resident_bytes <= evidence.budget.maximum_resident_bytes
            && output <= evidence.budget.maximum_output_bytes;
    }
    match (evidence.decision, within_budget) {
        (PerformanceDecision::Pass, false) => {
            errors.push("passing performance evidence exceeds its budget".to_string());
        }
        (PerformanceDecision::Blocked, true) => {
            errors.push(
                "blocked performance evidence must identify a measured budget failure".to_string(),
            );
        }
        _ => {}
    }
    errors.sort();
    errors.dedup();
    PerformanceValidation {
        valid: errors.is_empty(),
        errors,
    }
}

pub fn decode_performance_evidence(bytes: &[u8]) -> Result<PerformanceEvidence, String> {
    if bytes.is_empty() || bytes.len() as u64 > rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES {
        return Err("performance evidence is empty or oversized".to_string());
    }
    let evidence: PerformanceEvidence = serde_json::from_slice(bytes)
        .map_err(|error| format!("parse performance evidence: {error}"))?;
    let validation = validate_performance_evidence(&evidence);
    if validation.valid {
        Ok(evidence)
    } else {
        Err(validation.errors.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence() -> PerformanceEvidence {
        PerformanceEvidence {
            schema_version: PERFORMANCE_SCHEMA_VERSION,
            contract: PERFORMANCE_CONTRACT.to_string(),
            evidence_id: "perf:macos_aarch64-001".to_string(),
            target: "aarch64-apple-darwin".to_string(),
            source_commit: "a".repeat(40),
            artifact_sha256: "b".repeat(64),
            sample_count: 25,
            decision: PerformanceDecision::Pass,
            release_authorized: false,
            budget: PerformanceBudget::FINAL_ARTIFACT_BASELINE,
            operations: CANONICAL_PERFORMANCE_OPERATIONS
                .into_iter()
                .map(|operation| OperationMeasurement {
                    operation,
                    p50_wall_time_us: 1_000,
                    p95_wall_time_us: 2_000,
                    maximum_wall_time_us: 3_000,
                    maximum_resident_bytes: 4 * 1024 * 1024,
                    maximum_stdout_bytes: 2_000,
                    maximum_stderr_bytes: 0,
                    successful_samples: 25,
                })
                .collect(),
        }
    }

    #[test]
    fn canonical_final_artifact_evidence_is_valid_and_non_authorizing() {
        let evidence = evidence();
        let validation = validate_performance_evidence(&evidence);
        assert!(validation.valid, "{:?}", validation.errors);
        assert!(!evidence.release_authorized);
    }

    #[test]
    fn budget_failure_cannot_claim_pass() {
        let mut evidence = evidence();
        evidence.operations[0].maximum_resident_bytes = 65 * 1024 * 1024;
        assert!(!validate_performance_evidence(&evidence).valid);
        evidence.decision = PerformanceDecision::Blocked;
        assert!(validate_performance_evidence(&evidence).valid);
    }

    #[test]
    fn missing_reordered_or_failed_samples_fail_closed() {
        let mut evidence = evidence();
        evidence.operations.swap(0, 1);
        evidence.operations[0].successful_samples = 24;
        let validation = validate_performance_evidence(&evidence);
        assert!(!validation.valid);
        assert!(validation.errors.len() >= 2);
    }

    #[test]
    fn unknown_fields_and_oversized_documents_fail_closed() {
        let json = serde_json::to_string(&evidence()).unwrap();
        let drifted = json.replacen(
            "\"schema_version\":2",
            "\"schema_version\":2,\"future\":true",
            1,
        );
        assert!(decode_performance_evidence(drifted.as_bytes()).is_err());
        assert!(decode_performance_evidence(&vec![b'x'; 64 * 1024 + 1]).is_err());
    }
}
