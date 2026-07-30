use rz0_finding_contract::{
    Finding, FindingCategory, FindingConfidence, FindingDataClass, FindingDisposition,
    FindingOwnership, FindingReport, FindingRisk, FindingSource, FindingSourceStatus,
    build_finding_report,
};
use serde::{Deserialize, Serialize};

pub const MODULE_ID: &str = "first-party.uninstall";
pub const INPUT_CONTRACT: &str = "uninstall_finding_input";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UninstallFindingInput {
    pub schema_version: u16,
    pub contract: String,
    pub platform: String,
    pub input_evidence_sha256: String,
    pub source_id: String,
    pub source_evidence_sha256: String,
    pub records: Vec<UninstallRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UninstallRecord {
    pub finding_id: String,
    pub subject_reference: String,
    pub installed: bool,
    pub manager_record_present: bool,
}

pub fn classify_uninstalls(input: &UninstallFindingInput) -> Result<FindingReport, String> {
    if input.schema_version != 1 || input.contract != INPUT_CONTRACT {
        return Err("uninstall finding input identity is invalid".to_string());
    }
    if input.records.len() > rz0_resource_contract::MAX_FINDINGS {
        return Err("uninstall finding input exceeds the foundation ceiling".to_string());
    }
    let findings = input
        .records
        .iter()
        .map(|record| {
            let (ownership, confidence, risk, disposition) = if !record.installed {
                (
                    FindingOwnership::ManagerOwned,
                    FindingConfidence::Corroborated,
                    FindingRisk::Low,
                    FindingDisposition::Ignore,
                )
            } else if record.manager_record_present {
                (
                    FindingOwnership::ManagerOwned,
                    FindingConfidence::Corroborated,
                    FindingRisk::High,
                    FindingDisposition::ManagerActionCandidate,
                )
            } else {
                (
                    FindingOwnership::Unknown,
                    FindingConfidence::Heuristic,
                    FindingRisk::Blocked,
                    FindingDisposition::Blocked,
                )
            };
            Finding {
                finding_id: record.finding_id.clone(),
                category: FindingCategory::UninstallCandidate,
                subject_reference: record.subject_reference.clone(),
                source_ids: vec![input.source_id.clone()],
                ownership,
                data_class: FindingDataClass::ApplicationMetadata,
                confidence,
                risk,
                disposition,
                exact_evidence: None,
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
    fn installed_manager_record_is_required_for_an_uninstall_candidate() {
        let input = UninstallFindingInput {
            schema_version: 1,
            contract: INPUT_CONTRACT.to_string(),
            platform: "test".to_string(),
            input_evidence_sha256: A.to_string(),
            source_id: "manager.fixture".to_string(),
            source_evidence_sha256: A.to_string(),
            records: vec![
                UninstallRecord {
                    finding_id: "uninstall.managed".to_string(),
                    subject_reference: "package:alpha".to_string(),
                    installed: true,
                    manager_record_present: true,
                },
                UninstallRecord {
                    finding_id: "uninstall.unmanaged".to_string(),
                    subject_reference: "package:unknown".to_string(),
                    installed: true,
                    manager_record_present: false,
                },
            ],
        };
        let report = classify_uninstalls(&input).unwrap();
        assert_eq!(report.summary.manager_action_candidate_count, 1);
        assert_eq!(report.summary.blocked_count, 1);
    }
}
