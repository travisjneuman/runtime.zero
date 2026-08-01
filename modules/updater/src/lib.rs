use rz0_action_plan::{
    ActionDisposition, ActionKind, ActionPlan, ActionRisk, PlanAction, RollbackPlan,
    validate_action_plan,
};
use rz0_capability_contract::Capability;
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
    #[serde(default)]
    pub installed_version: Option<String>,
    #[serde(default)]
    pub available_version: Option<String>,
    #[serde(default)]
    pub manager: Option<String>,
    #[serde(default)]
    pub executable: Option<String>,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default)]
    pub network_required: bool,
    #[serde(default)]
    pub requires_elevation: bool,
    #[serde(default)]
    pub rollback_supported: bool,
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

/// Binds update candidates to the foundation action-plan contract without
/// granting execution authority. Exact manager/executable metadata is required
/// before an action can even be represented as a planned candidate.
pub fn build_update_action_plan(
    input: &UpdaterFindingInput,
    report: &FindingReport,
) -> Result<ActionPlan, String> {
    let report_validation = rz0_finding_contract::validate_finding_report(report);
    if !report_validation.valid
        || report.producer_module_id != MODULE_ID
        || report.contract != rz0_finding_contract::FINDING_CONTRACT
    {
        return Err(
            "update action plan requires a valid sealed updater finding report".to_string(),
        );
    }
    let actions = input
        .records
        .iter()
        .filter(|record| {
            record.installed && record.manager_record_present && record.update_available
        })
        .map(build_update_action)
        .collect::<Vec<_>>();
    if actions.is_empty() {
        return Err("update action plan contains no update candidates".to_string());
    }
    let plan = ActionPlan {
        schema_version: rz0_action_plan::ACTION_PLAN_SCHEMA_VERSION,
        plan_id: format!("update.plan.{}", report.report_id.trim_start_matches("findings:")),
        module_id: MODULE_ID.to_string(),
        created_at: None,
        expires_at: None,
        dry_run: true,
        writes_attempted: false,
        evidence_contract: report.contract.clone(),
        evidence_report_id: report.report_id.clone(),
        evidence_sha256: report.input_evidence_sha256.clone(),
        actions,
        warnings: vec![
            "schema-1 update plans are review-only; no manager command may execute".to_string(),
            "each action must be revalidated against fresh installed evidence before any future execution".to_string(),
        ],
    };
    let validation = validate_action_plan(&plan);
    if validation.valid {
        Ok(plan)
    } else {
        Err(validation.errors.join("; "))
    }
}

fn build_update_action(record: &UpdateRecord) -> PlanAction {
    let exact_command = record
        .manager
        .as_deref()
        .filter(|value| !value.is_empty())
        .is_some()
        && record
            .executable
            .as_deref()
            .is_some_and(rz0_validation_contract::is_absolute_local_path);
    let planned = exact_command;
    let mut capabilities = Vec::new();
    if planned && record.network_required {
        capabilities.push(Capability::NetworkMetadata);
    }
    if planned {
        capabilities.push(Capability::ManagerExecution);
    }
    if planned && record.requires_elevation {
        capabilities.push(Capability::ElevatedManagerAction);
    }
    let disposition = if planned {
        ActionDisposition::Planned
    } else {
        ActionDisposition::Blocked
    };
    let risk = if planned {
        ActionRisk::Medium
    } else {
        ActionRisk::Blocked
    };
    let target = match record.available_version.as_deref() {
        Some(version) => format!("{}@{version}", record.subject_reference),
        None => record.subject_reference.clone(),
    };
    PlanAction {
        action_id: format!("update.{}", record.finding_id),
        finding_id: record.finding_id.clone(),
        kind: ActionKind::Update,
        disposition,
        target,
        source: None,
        manager: record.manager.clone(),
        executable: record.executable.clone(),
        arguments: record.arguments.clone(),
        would_write: false,
        requires_confirmation: planned,
        requires_elevation: planned && record.requires_elevation,
        network_required: planned && record.network_required,
        risk,
        capabilities,
        forbidden_path_classes: Vec::new(),
        write_set: Vec::new(),
        rollback: RollbackPlan {
            supported: record.rollback_supported,
            quarantine_required: false,
            description: if record.rollback_supported {
                "manager-native rollback evidence must be recorded before execution".to_string()
            } else {
                "rollback evidence is not established; execution must remain blocked".to_string()
            },
        },
    }
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
                    installed_version: Some("1.0".to_string()),
                    available_version: Some("1.1".to_string()),
                    manager: Some("homebrew".to_string()),
                    executable: Some("/opt/homebrew/bin/brew".to_string()),
                    arguments: vec!["upgrade".to_string(), "alpha".to_string()],
                    network_required: true,
                    requires_elevation: false,
                    rollback_supported: true,
                },
                UpdateRecord {
                    finding_id: "update.missing".to_string(),
                    subject_reference: "package:missing".to_string(),
                    installed: false,
                    manager_record_present: false,
                    update_available: true,
                    installed_version: None,
                    available_version: Some("1.1".to_string()),
                    manager: None,
                    executable: None,
                    arguments: Vec::new(),
                    network_required: false,
                    requires_elevation: false,
                    rollback_supported: false,
                },
            ],
        };
        let report = classify_updates(&input).unwrap();
        assert_eq!(report.summary.manager_action_candidate_count, 1);
        assert_eq!(report.summary.blocked_count, 1);
        assert!(!report.action_authorized);
        let plan = build_update_action_plan(&input, &report).expect("update action plan");
        assert_eq!(plan.actions.len(), 1);
        assert!(!plan.actions[0].would_write);
        assert!(rz0_action_plan::validate_action_plan(&plan).valid);
    }

    #[test]
    fn missing_exact_manager_identity_stays_blocked_in_the_plan() {
        let input = UpdaterFindingInput {
            schema_version: 1,
            contract: INPUT_CONTRACT.to_string(),
            platform: "test".to_string(),
            input_evidence_sha256: A.to_string(),
            source_id: "manager.fixture".to_string(),
            source_evidence_sha256: A.to_string(),
            records: vec![UpdateRecord {
                finding_id: "update.missing-command".to_string(),
                subject_reference: "package:alpha".to_string(),
                installed: true,
                manager_record_present: true,
                update_available: true,
                installed_version: Some("1.0".to_string()),
                available_version: Some("1.1".to_string()),
                manager: Some("homebrew".to_string()),
                executable: None,
                arguments: Vec::new(),
                network_required: true,
                requires_elevation: false,
                rollback_supported: false,
            }],
        };
        let report = classify_updates(&input).expect("finding report");
        let plan = build_update_action_plan(&input, &report).expect("blocked plan");
        assert_eq!(plan.actions[0].disposition, ActionDisposition::Blocked);
        assert!(!plan.actions[0].requires_confirmation);
    }
}
