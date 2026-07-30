use rz0_finding_contract::{
    ExactFindingEvidence, Finding, FindingCategory, FindingConfidence, FindingDataClass,
    FindingDisposition, FindingOwnership, FindingReport, FindingRisk, FindingSource,
    FindingSourceStatus, build_finding_report,
};
use serde::{Deserialize, Serialize};

pub const MODULE_ID: &str = "first-party.security-integrity";
pub const INPUT_CONTRACT: &str = "integrity_finding_input";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegrityFindingInput {
    pub schema_version: u16,
    pub contract: String,
    pub platform: String,
    pub input_evidence_sha256: String,
    pub source_id: String,
    pub source_evidence_sha256: String,
    pub records: Vec<IntegrityRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IntegrityRecord {
    pub finding_id: String,
    pub subject_reference: String,
    pub ownership: FindingOwnership,
    pub data_class: FindingDataClass,
    pub expected_digest_matches: bool,
    pub exact_evidence: ExactFindingEvidence,
}

pub fn classify_integrity(input: &IntegrityFindingInput) -> Result<FindingReport, String> {
    if input.schema_version != 1 || input.contract != INPUT_CONTRACT {
        return Err("integrity finding input identity is invalid".to_string());
    }
    if input.records.len() > rz0_resource_contract::MAX_FINDINGS {
        return Err("integrity finding input exceeds the foundation ceiling".to_string());
    }
    let findings = input
        .records
        .iter()
        .map(|record| {
            let (confidence, risk, disposition) = if record.ownership == FindingOwnership::Unknown {
                (
                    FindingConfidence::Unknown,
                    FindingRisk::Blocked,
                    FindingDisposition::Blocked,
                )
            } else {
                (
                    FindingConfidence::ExactEvidence,
                    if record.expected_digest_matches {
                        FindingRisk::Low
                    } else {
                        FindingRisk::High
                    },
                    FindingDisposition::ReportOnly,
                )
            };
            Finding {
                finding_id: record.finding_id.clone(),
                category: FindingCategory::IntegrityObservation,
                subject_reference: record.subject_reference.clone(),
                source_ids: vec![input.source_id.clone()],
                ownership: record.ownership,
                data_class: record.data_class,
                confidence,
                risk,
                disposition,
                exact_evidence: Some(record.exact_evidence.clone()),
            }
        })
        .collect();
    build_finding_report(
        MODULE_ID,
        &input.platform,
        &input.input_evidence_sha256,
        vec![FindingSource {
            id: input.source_id.clone(),
            status: FindingSourceStatus::Ok,
            evidence_sha256: input.source_evidence_sha256.clone(),
        }],
        findings,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn digest_mismatch_is_high_risk_evidence_but_never_an_action() {
        let input = IntegrityFindingInput {
            schema_version: 1,
            contract: INPUT_CONTRACT.to_string(),
            platform: "test".to_string(),
            input_evidence_sha256: A.to_string(),
            source_id: "integrity.fixture".to_string(),
            source_evidence_sha256: A.to_string(),
            records: vec![IntegrityRecord {
                finding_id: "integrity.mismatch".to_string(),
                subject_reference: "artifact:alpha".to_string(),
                ownership: FindingOwnership::SystemOwned,
                data_class: FindingDataClass::ExecutableArtifact,
                expected_digest_matches: false,
                exact_evidence: ExactFindingEvidence {
                    sha256: A.to_string(),
                    size_bytes: 12,
                },
            }],
        };
        let report = classify_integrity(&input).unwrap();
        assert_eq!(report.findings[0].risk, FindingRisk::High);
        assert_eq!(
            report.findings[0].disposition,
            FindingDisposition::ReportOnly
        );
        assert!(!report.action_authorized);
    }
}
