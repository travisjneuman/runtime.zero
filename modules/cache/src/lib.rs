use rz0_finding_contract::{
    ExactFindingEvidence, Finding, FindingCategory, FindingConfidence, FindingDataClass,
    FindingDisposition, FindingOwnership, FindingReport, FindingRisk, FindingSource,
    FindingSourceStatus, build_finding_report,
};
use serde::{Deserialize, Serialize};

pub const MODULE_ID: &str = "first-party.cache";
pub const INPUT_CONTRACT: &str = "cache_finding_input";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CacheFindingInput {
    pub schema_version: u16,
    pub contract: String,
    pub platform: String,
    pub input_evidence_sha256: String,
    pub source_id: String,
    pub source_evidence_sha256: String,
    pub records: Vec<CacheRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CacheRecord {
    pub finding_id: String,
    pub subject_reference: String,
    pub ownership: FindingOwnership,
    pub exact_evidence: Option<ExactFindingEvidence>,
}

pub fn classify_caches(input: &CacheFindingInput) -> Result<FindingReport, String> {
    if input.schema_version != 1 || input.contract != INPUT_CONTRACT {
        return Err("cache finding input identity is invalid".to_string());
    }
    if input.records.len() > rz0_resource_contract::MAX_FINDINGS {
        return Err("cache finding input exceeds the foundation ceiling".to_string());
    }
    let findings = input
        .records
        .iter()
        .map(|record| {
            let (confidence, risk, disposition) = match record.ownership {
                FindingOwnership::RuntimeOwned if record.exact_evidence.is_some() => (
                    FindingConfidence::ExactEvidence,
                    FindingRisk::Low,
                    FindingDisposition::QuarantineCandidate,
                ),
                FindingOwnership::ManagerOwned => (
                    if record.exact_evidence.is_some() {
                        FindingConfidence::ExactEvidence
                    } else {
                        FindingConfidence::Corroborated
                    },
                    FindingRisk::Medium,
                    FindingDisposition::ReportOnly,
                ),
                FindingOwnership::RuntimeOwned
                | FindingOwnership::SystemOwned
                | FindingOwnership::UserOwned => (
                    FindingConfidence::Heuristic,
                    FindingRisk::High,
                    FindingDisposition::ReportOnly,
                ),
                FindingOwnership::Unknown => (
                    FindingConfidence::Unknown,
                    FindingRisk::Blocked,
                    FindingDisposition::Blocked,
                ),
            };
            Finding {
                finding_id: record.finding_id.clone(),
                category: FindingCategory::CacheCandidate,
                subject_reference: record.subject_reference.clone(),
                source_ids: vec![input.source_id.clone()],
                ownership: record.ownership,
                data_class: FindingDataClass::CacheData,
                confidence,
                risk,
                disposition,
                exact_evidence: record.exact_evidence.clone(),
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
    fn runtime_exact_cache_is_quarantine_candidate_but_manager_cache_is_report_only() {
        let input = CacheFindingInput {
            schema_version: 1,
            contract: INPUT_CONTRACT.to_string(),
            platform: "test".to_string(),
            input_evidence_sha256: A.to_string(),
            source_id: "cache.fixture".to_string(),
            source_evidence_sha256: A.to_string(),
            records: vec![
                CacheRecord {
                    finding_id: "cache.runtime".to_string(),
                    subject_reference: "subject:runtime-cache".to_string(),
                    ownership: FindingOwnership::RuntimeOwned,
                    exact_evidence: Some(ExactFindingEvidence {
                        sha256: A.to_string(),
                        size_bytes: 12,
                    }),
                },
                CacheRecord {
                    finding_id: "cache.manager".to_string(),
                    subject_reference: "subject:manager-cache".to_string(),
                    ownership: FindingOwnership::ManagerOwned,
                    exact_evidence: None,
                },
            ],
        };
        let report = classify_caches(&input).unwrap();
        assert_eq!(report.summary.quarantine_candidate_count, 1);
        assert_eq!(report.summary.report_only_count, 1);
    }
}
