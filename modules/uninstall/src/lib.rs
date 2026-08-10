use rz0_action_plan::{
    ActionDisposition, ActionExecutableIdentity, ActionKind, ActionPlan, ActionRisk, PlanAction,
    RollbackPlan, validate_action_plan,
};
use rz0_capability_contract::Capability;
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
    #[serde(default)]
    pub ownership: UninstallOwnership,
    #[serde(default)]
    pub manager: Option<String>,
    #[serde(default)]
    pub executable: Option<String>,
    #[serde(default)]
    pub executable_sha256: Option<String>,
    #[serde(default)]
    pub executable_size_bytes: Option<u64>,
    #[serde(default)]
    pub arguments: Vec<String>,
    #[serde(default)]
    pub requires_elevation: bool,
    #[serde(default)]
    pub rollback_supported: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UninstallOwnership {
    Manager,
    System,
    User,
    #[default]
    Unknown,
}

pub fn classify_uninstalls(input: &UninstallFindingInput) -> Result<FindingReport, String> {
    validate_input(input)?;
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
            } else if record.manager_record_present
                && record.ownership == UninstallOwnership::Manager
            {
                (
                    FindingOwnership::ManagerOwned,
                    FindingConfidence::Corroborated,
                    FindingRisk::High,
                    FindingDisposition::ManagerActionCandidate,
                )
            } else {
                match record.ownership {
                    UninstallOwnership::System => (
                        FindingOwnership::SystemOwned,
                        FindingConfidence::Corroborated,
                        FindingRisk::Blocked,
                        FindingDisposition::Blocked,
                    ),
                    UninstallOwnership::User => (
                        FindingOwnership::UserOwned,
                        FindingConfidence::Heuristic,
                        FindingRisk::High,
                        FindingDisposition::ReportOnly,
                    ),
                    UninstallOwnership::Manager | UninstallOwnership::Unknown => (
                        FindingOwnership::Unknown,
                        FindingConfidence::Heuristic,
                        FindingRisk::Blocked,
                        FindingDisposition::Blocked,
                    ),
                }
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

/// Builds a finding-bound, non-authorizing manager uninstall plan. Missing or
/// unsealed manager identity remains represented as a blocked action rather
/// than falling back to PATH, shell execution, or direct file deletion.
pub fn build_uninstall_action_plan(
    input: &UninstallFindingInput,
    report: &FindingReport,
) -> Result<ActionPlan, String> {
    let expected = classify_uninstalls(input)?;
    let validation = rz0_finding_contract::validate_finding_report(report);
    if !validation.valid || report != &expected {
        return Err(
            "uninstall action plan requires the report sealed from the exact catalog evidence"
                .to_string(),
        );
    }
    let actions = input
        .records
        .iter()
        .filter(|record| {
            record.installed
                && record.manager_record_present
                && record.ownership == UninstallOwnership::Manager
        })
        .map(|record| build_manager_action(record, &input.platform))
        .collect::<Vec<_>>();
    if actions.is_empty() {
        return Err("uninstall evidence contains no manager-owned action candidates".to_string());
    }
    let plan = ActionPlan {
        schema_version: rz0_action_plan::ACTION_PLAN_SCHEMA_VERSION,
        plan_id: format!(
            "uninstall.plan.{}",
            report.report_id.trim_start_matches("findings:")
        ),
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
            "uninstall plans are review-only; no manager command or filesystem removal is authorized"
                .to_string(),
            "dependents, shared components, rollback, and fresh installed-state verification remain mandatory before execution"
                .to_string(),
        ],
    };
    let validation = validate_action_plan(&plan);
    if validation.valid {
        Ok(plan)
    } else {
        Err(validation.errors.join("; "))
    }
}

fn validate_input(input: &UninstallFindingInput) -> Result<(), String> {
    if input.schema_version != 1 || input.contract != INPUT_CONTRACT {
        return Err("uninstall finding input identity is invalid".to_string());
    }
    if input.records.is_empty() || input.records.len() > rz0_resource_contract::MAX_FINDINGS {
        return Err("uninstall finding input exceeds the foundation record bounds".to_string());
    }
    if !rz0_validation_contract::valid_dotted_id(&input.platform, 100)
        || !rz0_validation_contract::valid_ledger_id(&input.source_id, 100)
        || !rz0_validation_contract::valid_sha256(&input.input_evidence_sha256)
        || !rz0_validation_contract::valid_sha256(&input.source_evidence_sha256)
    {
        return Err("uninstall finding input provenance is invalid".to_string());
    }
    for record in &input.records {
        if !rz0_validation_contract::valid_ledger_id(&record.finding_id, 120)
            || !rz0_validation_contract::valid_evidence_reference(&record.subject_reference, 240)
            || record
                .manager
                .as_ref()
                .is_some_and(|value| unsafe_text(value, 80))
            || record
                .executable
                .as_ref()
                .is_some_and(|value| unsafe_text(value, 1_024))
            || record.arguments.len() > rz0_action_plan::MAX_ARGUMENTS
            || record.arguments.iter().any(|value| unsafe_text(value, 512))
            || record.executable_sha256.is_some() != record.executable_size_bytes.is_some()
            || record.executable_sha256.as_ref().is_some_and(|digest| {
                !rz0_validation_contract::valid_sha256(digest)
                    || record.executable_size_bytes.is_none_or(|size| {
                        size == 0 || size > rz0_resource_contract::MAX_ARTIFACT_BYTES
                    })
            })
        {
            return Err(
                "uninstall finding record identity or command evidence is invalid".to_string(),
            );
        }
        if !record.manager_record_present
            && (record.manager.is_some()
                || record.executable.is_some()
                || record.executable_sha256.is_some()
                || !record.arguments.is_empty())
        {
            return Err(
                "unowned uninstall evidence cannot attach manager command fields".to_string(),
            );
        }
    }
    Ok(())
}

fn unsafe_text(value: &str, maximum: usize) -> bool {
    value.trim().is_empty() || value.len() > maximum || value.chars().any(char::is_control)
}

pub fn manager_executable_allowed(manager: &str, platform: &str, executable: &str) -> bool {
    matches!(
        (manager, platform, executable),
        (
            "homebrew",
            "macos",
            "/opt/homebrew/bin/brew" | "/usr/local/bin/brew"
        ) | ("macports", "macos", "/opt/local/bin/port")
            | ("winget", "windows", r"C:\Windows\System32\winget.exe")
            | ("apt", "linux", "/usr/bin/apt")
            | ("dnf", "linux", "/usr/bin/dnf")
            | ("pacman", "linux", "/usr/bin/pacman")
            | ("zypper", "linux", "/usr/bin/zypper")
            | ("snap", "linux", "/usr/bin/snap")
            | ("flatpak", "linux", "/usr/bin/flatpak")
    )
}

fn build_manager_action(record: &UninstallRecord, platform: &str) -> PlanAction {
    let identity = record
        .executable_sha256
        .as_deref()
        .zip(record.executable_size_bytes)
        .filter(|(sha256, size)| {
            rz0_validation_contract::valid_sha256(sha256)
                && *size > 0
                && *size <= rz0_resource_contract::MAX_ARTIFACT_BYTES
        });
    let exact = record.manager.as_deref().is_some_and(|manager| {
        record.executable.as_deref().is_some_and(|executable| {
            rz0_validation_contract::is_absolute_local_path(executable)
                && manager_executable_allowed(manager, platform, executable)
        })
    }) && identity.is_some()
        && !record.arguments.is_empty()
        && record.arguments.len() <= rz0_action_plan::MAX_ARGUMENTS
        && record
            .arguments
            .iter()
            .all(|argument| !argument.is_empty() && !argument.chars().any(char::is_control));
    let disposition = if exact {
        ActionDisposition::Planned
    } else {
        ActionDisposition::Blocked
    };
    let mut capabilities = Vec::new();
    if exact {
        capabilities.push(Capability::ManagerExecution);
    }
    if exact && record.requires_elevation {
        capabilities.push(Capability::ElevatedManagerAction);
    }
    PlanAction {
        action_id: format!("uninstall.{}", record.finding_id),
        finding_id: record.finding_id.clone(),
        kind: ActionKind::Uninstall,
        disposition,
        target: record.subject_reference.clone(),
        source: None,
        manager: record.manager.clone(),
        executable: record.executable.clone(),
        executable_identity: exact.then(|| ActionExecutableIdentity {
            sha256: record.executable_sha256.clone().unwrap_or_default(),
            size_bytes: record.executable_size_bytes.unwrap_or_default(),
        }),
        arguments: record.arguments.clone(),
        would_write: false,
        requires_confirmation: exact,
        requires_elevation: exact && record.requires_elevation,
        network_required: false,
        risk: if exact {
            ActionRisk::High
        } else {
            ActionRisk::Blocked
        },
        capabilities,
        forbidden_path_classes: Vec::new(),
        write_set: Vec::new(),
        rollback: RollbackPlan {
            supported: exact && record.rollback_supported,
            quarantine_required: false,
            description: if record.rollback_supported {
                "manager-native rollback evidence must be revalidated before execution".to_string()
            } else {
                "no manager rollback is proven; execution remains gated by manual recovery and dependent/shared-component review".to_string()
            },
        },
    }
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
                    ownership: UninstallOwnership::Manager,
                    manager: None,
                    executable: None,
                    executable_sha256: None,
                    executable_size_bytes: None,
                    arguments: Vec::new(),
                    requires_elevation: false,
                    rollback_supported: false,
                },
                UninstallRecord {
                    finding_id: "uninstall.unmanaged".to_string(),
                    subject_reference: "package:unknown".to_string(),
                    installed: true,
                    manager_record_present: false,
                    ownership: UninstallOwnership::Unknown,
                    manager: None,
                    executable: None,
                    executable_sha256: None,
                    executable_size_bytes: None,
                    arguments: Vec::new(),
                    requires_elevation: false,
                    rollback_supported: false,
                },
            ],
        };
        let report = classify_uninstalls(&input).unwrap();
        assert_eq!(report.summary.manager_action_candidate_count, 1);
        assert_eq!(report.summary.blocked_count, 1);
        let plan = build_uninstall_action_plan(&input, &report).expect("blocked shared plan");
        assert_eq!(plan.actions[0].disposition, ActionDisposition::Blocked);
        assert!(!plan.actions[0].requires_confirmation);
    }

    #[test]
    fn exact_homebrew_identity_builds_a_non_authorizing_manager_plan() {
        let input = UninstallFindingInput {
            schema_version: 1,
            contract: INPUT_CONTRACT.to_string(),
            platform: "macos".to_string(),
            input_evidence_sha256: A.to_string(),
            source_id: "catalog.homebrew".to_string(),
            source_evidence_sha256: A.to_string(),
            records: vec![UninstallRecord {
                finding_id: "uninstall.homebrew.alpha".to_string(),
                subject_reference: "software:alpha".to_string(),
                installed: true,
                manager_record_present: true,
                ownership: UninstallOwnership::Manager,
                manager: Some("homebrew".to_string()),
                executable: Some("/opt/homebrew/bin/brew".to_string()),
                executable_sha256: Some(A.to_string()),
                executable_size_bytes: Some(4096),
                arguments: vec!["uninstall".to_string(), "alpha".to_string()],
                requires_elevation: false,
                rollback_supported: false,
            }],
        };
        let report = classify_uninstalls(&input).expect("finding report");
        let plan = build_uninstall_action_plan(&input, &report).expect("manager plan");
        assert_eq!(plan.actions[0].disposition, ActionDisposition::Planned);
        assert!(plan.actions[0].requires_confirmation);
        assert!(!plan.actions[0].would_write);
        assert!(plan.actions[0].executable_identity.is_some());
    }
}
