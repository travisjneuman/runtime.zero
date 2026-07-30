use rz0_finding_contract::{
    Finding, FindingCategory, FindingConfidence, FindingDataClass, FindingDisposition,
    FindingOwnership, FindingReport, FindingRisk, FindingSource, FindingSourceStatus,
    build_finding_report,
};
use serde::{Deserialize, Serialize};

pub const MODULE_ID: &str = "first-party.updater";
pub const INPUT_CONTRACT: &str = "updater_finding_input";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpdaterFindingInput {
    pub schema_version: u16,
    pub contract: String,
    pub platform: String,
    pub input_evidence_sha256: String,
    pub source_id: String,
    pub source_evidence_sha256: String,
    pub records: Vec<UpdateRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateRecord {
    pub finding_id: String,
    pub subject_reference: String,
    pub installed: bool,
    pub manager_record_present: bool,
    pub update_available: bool,
}

pub fn classify_updates(input: &UpdaterFindingInput) -> Result<FindingReport, String> {
    validate_header(input.schema_version, &input.contract, input.records.len())?;
    let findings = input
        .records
        .iter()
        .map(|record| {
            let (ownership, confidence, risk, disposition) = if !record.installed {
                (
                    FindingOwnership::Unknown,
                    FindingConfidence::Unknown,
                    FindingRisk::Blocked,
                    FindingDisposition::Blocked,
                )
            } else if !record.manager_record_present {
                (
                    FindingOwnership::Unknown,
                    FindingConfidence::Heuristic,
                    FindingRisk::Blocked,
                    FindingDisposition::Blocked,
                )
            } else if record.update_available {
                (
                    FindingOwnership::ManagerOwned,
                    FindingConfidence::Corroborated,
                    FindingRisk::Medium,
                    FindingDisposition::ManagerActionCandidate,
                )
            } else {
                (
                    FindingOwnership::ManagerOwned,
                    FindingConfidence::Corroborated,
                    FindingRisk::Low,
                    FindingDisposition::Ignore,
                )
            };
            Finding {
                finding_id: record.finding_id.clone(),
                category: FindingCategory::UpdateCandidate,
                subject_reference: record.subject_reference.clone(),
                source_ids: vec![input.source_id.clone()],
                ownership,
                data_class: FindingDataClass::PackageMetadata,
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

fn validate_header(schema_version: u16, contract: &str, records: usize) -> Result<(), String> {
    if schema_version != 1 || contract != INPUT_CONTRACT {
        return Err("updater finding input identity is invalid".to_string());
    }
    if records > rz0_resource_contract::MAX_FINDINGS {
        return Err("updater finding input exceeds the foundation ceiling".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn only_installed_manager_owned_updates_become_manager_candidates() {
        let input = UpdaterFindingInput {
            schema_version: 1,
            contract: INPUT_CONTRACT.to_string(),
            platform: "test".to_string(),
            input_evidence_sha256: A.to_string(),
            source_id: "manager.fixture".to_string(),
            source_evidence_sha256: A.to_string(),
            records: vec![
                UpdateRecord {
                    finding_id: "update.available".to_string(),
                    subject_reference: "package:alpha".to_string(),
                    installed: true,
                    manager_record_present: true,
                    update_available: true,
                },
                UpdateRecord {
                    finding_id: "update.missing".to_string(),
                    subject_reference: "package:missing".to_string(),
                    installed: false,
                    manager_record_present: false,
                    update_available: true,
                },
            ],
        };
        let report = classify_updates(&input).unwrap();
        assert_eq!(report.summary.manager_action_candidate_count, 1);
        assert_eq!(report.summary.blocked_count, 1);
        assert!(!report.action_authorized);
    }
}
