use rz0_action_plan::{
    ActionDisposition, ActionKind, ActionPlan, ActionRisk, PlanAction, RollbackPlan,
    validate_action_plan,
};
use rz0_capability_contract::Capability;
mod adapters;

pub use adapters::{
    ManagerKind, ManagerParseContext, ManagerProbeSpec, manager_executable_allowed,
    manager_probe_specs, manager_probe_specs_for_platform, parse_manager_output,
};

use rz0_finding_contract::{
    Finding, FindingCategory, FindingConfidence, FindingDataClass, FindingDisposition,
    FindingOwnership, FindingReport, FindingRisk, FindingSource, FindingSourceStatus,
    build_finding_report,
};
use serde::{Deserialize, Serialize};

pub const MODULE_ID: &str = "first-party.updater";
pub const INPUT_CONTRACT: &str = "updater_finding_input";
pub const UPDATE_QUEUE_CONTRACT: &str = "serial_update_queue_plan";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SerialUpdateQueuePlan {
    pub schema_version: u16,
    pub contract: String,
    pub queue_id: String,
    pub action_plan_sha256: String,
    pub write_set_sha256: String,
    pub dry_run: bool,
    pub writes_attempted: bool,
    pub product_execution_authorized: bool,
    pub items: Vec<SerialUpdateItem>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SerialUpdateItem {
    pub sequence: u32,
    pub action_id: String,
    pub finding_id: String,
    pub target: String,
    pub status: SerialUpdateItemStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SerialUpdateItemStatus {
    Pending,
    Blocked,
}

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
    let expected_report = classify_updates(input)?;
    if !report_validation.valid
        || report.producer_module_id != MODULE_ID
        || report.contract != rz0_finding_contract::FINDING_CONTRACT
        || report.report_id != expected_report.report_id
    {
        return Err(
            "update action plan requires a valid report sealed from the exact input evidence"
                .to_string(),
        );
    }
    let actions = input
        .records
        .iter()
        .filter(|record| {
            record.installed && record.manager_record_present && record.update_available
        })
        .map(|record| build_update_action(record, &input.platform))
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

pub fn build_serial_update_queue(plan: &ActionPlan) -> Result<SerialUpdateQueuePlan, String> {
    let validation = validate_action_plan(plan);
    if !validation.valid {
        return Err(validation.errors.join("; "));
    }
    let digests = rz0_action_plan::action_plan_digests(plan).map_err(|errors| errors.join("; "))?;
    let items = plan
        .actions
        .iter()
        .filter(|action| action.disposition == ActionDisposition::Planned)
        .enumerate()
        .map(|(index, action)| SerialUpdateItem {
            sequence: index as u32 + 1,
            action_id: action.action_id.clone(),
            finding_id: action.finding_id.clone(),
            target: action.target.clone(),
            status: if action.rollback.supported {
                SerialUpdateItemStatus::Pending
            } else {
                SerialUpdateItemStatus::Blocked
            },
        })
        .collect::<Vec<_>>();
    if items.is_empty() {
        return Err("serial update queue contains no planned actions".to_string());
    }
    Ok(SerialUpdateQueuePlan {
        schema_version: 1,
        contract: UPDATE_QUEUE_CONTRACT.to_string(),
        queue_id: format!("update.queue.{}", &digests.plan_sha256[..24]),
        action_plan_sha256: digests.plan_sha256,
        write_set_sha256: digests.write_set_sha256,
        dry_run: true,
        writes_attempted: false,
        product_execution_authorized: false,
        items,
        warnings: vec![
            "queue items are serial and each item requires fresh evidence before any future execution".to_string(),
            "items without proven manager rollback remain blocked even when their dry-run action is valid".to_string(),
            "a failure, drift, cancellation, or recovery requirement pauses the queue".to_string(),
        ],
    })
}

fn build_update_action(record: &UpdateRecord, platform: &str) -> PlanAction {
    let manager = record.manager.as_deref();
    let executable = record.executable.as_deref();
    let known_platform = matches!(platform, "windows" | "macos" | "linux");
    let exact_command = manager.filter(|value| !value.is_empty()).is_some()
        && executable.is_some_and(rz0_validation_contract::is_absolute_local_path)
        && (!known_platform
            || manager_executable_allowed(
                manager.unwrap_or_default(),
                platform,
                executable.unwrap_or_default(),
            ));
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
    let risk = if !planned {
        ActionRisk::Blocked
    } else if record.rollback_supported {
        ActionRisk::Medium
    } else {
        ActionRisk::High
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
                "rollback evidence is not established; explicit execution requires a manual-recovery acknowledgement".to_string()
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
        assert_eq!(plan.actions[0].risk, ActionRisk::Medium);
        assert!(!plan.actions[0].would_write);
        assert!(rz0_action_plan::validate_action_plan(&plan).valid);
        let queue = build_serial_update_queue(&plan).expect("serial update queue");
        assert_eq!(queue.items.len(), 1);
        assert_eq!(queue.items[0].sequence, 1);
        assert!(!queue.product_execution_authorized);
    }

    #[test]
    fn rejects_a_report_that_does_not_match_the_input_evidence() {
        let input = UpdaterFindingInput {
            schema_version: 1,
            contract: INPUT_CONTRACT.to_string(),
            platform: "test".to_string(),
            input_evidence_sha256: A.to_string(),
            source_id: "manager.fixture".to_string(),
            source_evidence_sha256: A.to_string(),
            records: Vec::new(),
        };
        let other = UpdaterFindingInput {
            records: vec![UpdateRecord {
                finding_id: "update.other".to_string(),
                subject_reference: "package:other".to_string(),
                installed: true,
                manager_record_present: true,
                update_available: true,
                installed_version: Some("1.0".to_string()),
                available_version: Some("2.0".to_string()),
                manager: Some("manager".to_string()),
                executable: Some("/usr/bin/manager".to_string()),
                arguments: vec!["upgrade".to_string(), "other".to_string()],
                network_required: false,
                requires_elevation: false,
                rollback_supported: true,
            }],
            ..input.clone()
        };
        let report = classify_updates(&other).expect("other report");
        assert!(build_update_action_plan(&input, &report).is_err());
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
        assert_eq!(plan.actions[0].risk, ActionRisk::Blocked);
        assert!(!plan.actions[0].requires_confirmation);
    }
}
