use rz0_finding_contract::{
    ExactFindingEvidence, Finding, FindingCategory, FindingConfidence, FindingDataClass,
    FindingDisposition, FindingOwnership, FindingReport, FindingRisk, FindingSource,
    FindingSourceStatus, build_finding_report,
};
use serde::{Deserialize, Serialize};

pub const MODULE_ID: &str = "first-party.leftovers";
pub const INPUT_CONTRACT: &str = "leftover_finding_input";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LeftoverFindingInput {
    pub schema_version: u16,
    pub contract: String,
    pub platform: String,
    pub input_evidence_sha256: String,
    pub source_id: String,
    pub source_evidence_sha256: String,
    pub records: Vec<LeftoverRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LeftoverRecord {
    pub finding_id: String,
    pub subject_reference: String,
    pub ownership: FindingOwnership,
    pub data_class: FindingDataClass,
    pub exact_evidence: Option<ExactFindingEvidence>,
}

pub fn classify_leftovers(input: &LeftoverFindingInput) -> Result<FindingReport, String> {
    validate_input(input)?;
    let findings = input
        .records
        .iter()
        .map(|record| {
            let protected = protected(record.data_class);
            let exact_quarantine_class = matches!(
                record.data_class,
                FindingDataClass::OrphanedData | FindingDataClass::ExecutableArtifact
            );
            let (confidence, risk, disposition) = if protected
                || matches!(
                    record.ownership,
                    FindingOwnership::UserOwned
                        | FindingOwnership::SystemOwned
                        | FindingOwnership::Unknown
                ) {
                (
                    FindingConfidence::Unknown,
                    FindingRisk::Blocked,
                    FindingDisposition::Blocked,
                )
            } else if record.ownership == FindingOwnership::RuntimeOwned
                && exact_quarantine_class
                && record.exact_evidence.is_some()
            {
                (
                    FindingConfidence::ExactEvidence,
                    FindingRisk::Medium,
                    FindingDisposition::QuarantineCandidate,
                )
            } else {
                (
                    if record.exact_evidence.is_some() {
                        FindingConfidence::ExactEvidence
                    } else {
                        FindingConfidence::Heuristic
                    },
                    FindingRisk::Medium,
                    FindingDisposition::ReportOnly,
                )
            };
            Finding {
                finding_id: record.finding_id.clone(),
                category: FindingCategory::LeftoverCandidate,
                subject_reference: record.subject_reference.clone(),
                source_ids: vec![input.source_id.clone()],
                ownership: record.ownership,
                data_class: record.data_class,
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

fn validate_input(input: &LeftoverFindingInput) -> Result<(), String> {
    if input.schema_version != 1 || input.contract != INPUT_CONTRACT {
        return Err("leftover finding input identity is invalid".to_string());
    }
    if input.records.len() > rz0_resource_contract::MAX_FINDINGS {
        return Err("leftover finding input exceeds the foundation ceiling".to_string());
    }
    Ok(())
}

const fn protected(data_class: FindingDataClass) -> bool {
    matches!(
        data_class,
        FindingDataClass::CredentialOrSession
            | FindingDataClass::BrowserProfile
            | FindingDataClass::ProjectWorkspace
            | FindingDataClass::Backup
            | FindingDataClass::UserContent
            | FindingDataClass::Unknown
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn only_exact_runtime_owned_leftovers_become_quarantine_candidates() {
        let input = LeftoverFindingInput {
            schema_version: 1,
            contract: INPUT_CONTRACT.to_string(),
            platform: "test".to_string(),
            input_evidence_sha256: A.to_string(),
            source_id: "leftover.fixture".to_string(),
            source_evidence_sha256: A.to_string(),
            records: vec![
                LeftoverRecord {
                    finding_id: "leftover.runtime-shim".to_string(),
                    subject_reference: "subject:shim".to_string(),
                    ownership: FindingOwnership::RuntimeOwned,
                    data_class: FindingDataClass::ExecutableArtifact,
                    exact_evidence: Some(ExactFindingEvidence {
                        sha256: A.to_string(),
                        size_bytes: 12,
                    }),
                },
                LeftoverRecord {
                    finding_id: "leftover.user-workspace".to_string(),
                    subject_reference: "subject:workspace".to_string(),
                    ownership: FindingOwnership::UserOwned,
                    data_class: FindingDataClass::ProjectWorkspace,
                    exact_evidence: None,
                },
            ],
        };
        let report = classify_leftovers(&input).unwrap();
        assert_eq!(report.summary.quarantine_candidate_count, 1);
        assert_eq!(report.summary.blocked_count, 1);
    }
}
