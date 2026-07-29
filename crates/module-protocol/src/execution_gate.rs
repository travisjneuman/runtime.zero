use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{
    ProtocolPlatform, ProtocolValidation,
    policy::{valid_id, valid_version},
};

pub const EXECUTION_GATE_SCHEMA_VERSION: u16 = 1;
pub const EXECUTION_GATE_CONTRACT: &str = "production_execution_assessment";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProductionExecutionAssessment {
    pub schema_version: u16,
    pub contract: String,
    pub assessment_id: String,
    pub module_id: String,
    pub module_version: String,
    pub platform: ProtocolPlatform,
    pub decision: ExecutionDecision,
    pub product_execution_authorized: bool,
    pub gates: Vec<ExecutionGateEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionDecision {
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionGateEvidence {
    pub gate: ExecutionGate,
    pub status: ExecutionGateStatus,
    pub mechanism: Option<String>,
    pub evidence_reference: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionGateStatus {
    Missing,
    Unsupported,
    Proven,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionGate {
    ArtifactImmutableStaging,
    ArtifactPackageIntegrity,
    ArtifactProductionSignature,
    ArtifactProvenance,
    ArtifactReceiptBinding,
    ArtifactRevocation,
    ConfirmationExactPlanBinding,
    ConfirmationSingleUseConsumption,
    CapabilityDeclaration,
    CapabilityFilesystemEnforcement,
    CapabilityNetworkEnforcement,
    CapabilityPrivilegeEnforcement,
    CapabilityRegistryEnforcement,
    ExecutableIdentityPinned,
    ExecutableReplacementRaceClosed,
    ProcessBoundedStdio,
    ProcessCancellation,
    ProcessEnvironmentCleared,
    ProcessHandleContainment,
    ProcessPlatformSandbox,
    ProcessProcessTreeContainment,
    ProcessTimeoutTeardown,
    RuntimeCoreRouting,
    RuntimePlatformProof,
    TransactionAtomicRegistry,
    TransactionCrashRecovery,
    TransactionJournal,
    TransactionQuarantineRestore,
    TransactionRollback,
}

pub const ALL_EXECUTION_GATES: [ExecutionGate; 29] = [
    ExecutionGate::ArtifactImmutableStaging,
    ExecutionGate::ArtifactPackageIntegrity,
    ExecutionGate::ArtifactProductionSignature,
    ExecutionGate::ArtifactProvenance,
    ExecutionGate::ArtifactReceiptBinding,
    ExecutionGate::ArtifactRevocation,
    ExecutionGate::ConfirmationExactPlanBinding,
    ExecutionGate::ConfirmationSingleUseConsumption,
    ExecutionGate::CapabilityDeclaration,
    ExecutionGate::CapabilityFilesystemEnforcement,
    ExecutionGate::CapabilityNetworkEnforcement,
    ExecutionGate::CapabilityPrivilegeEnforcement,
    ExecutionGate::CapabilityRegistryEnforcement,
    ExecutionGate::ExecutableIdentityPinned,
    ExecutionGate::ExecutableReplacementRaceClosed,
    ExecutionGate::ProcessBoundedStdio,
    ExecutionGate::ProcessCancellation,
    ExecutionGate::ProcessEnvironmentCleared,
    ExecutionGate::ProcessHandleContainment,
    ExecutionGate::ProcessPlatformSandbox,
    ExecutionGate::ProcessProcessTreeContainment,
    ExecutionGate::ProcessTimeoutTeardown,
    ExecutionGate::RuntimeCoreRouting,
    ExecutionGate::RuntimePlatformProof,
    ExecutionGate::TransactionAtomicRegistry,
    ExecutionGate::TransactionCrashRecovery,
    ExecutionGate::TransactionJournal,
    ExecutionGate::TransactionQuarantineRestore,
    ExecutionGate::TransactionRollback,
];

pub fn validate_production_execution_assessment(
    assessment: &ProductionExecutionAssessment,
) -> ProtocolValidation {
    let mut errors = Vec::new();
    if assessment.schema_version != EXECUTION_GATE_SCHEMA_VERSION {
        errors.push(format!(
            "execution assessment schema_version must be {EXECUTION_GATE_SCHEMA_VERSION}"
        ));
    }
    if assessment.contract != EXECUTION_GATE_CONTRACT {
        errors.push(format!(
            "execution assessment contract must be {EXECUTION_GATE_CONTRACT}"
        ));
    }
    if !valid_id(&assessment.assessment_id) {
        errors.push("execution assessment_id is invalid".to_string());
    }
    if !valid_id(&assessment.module_id) {
        errors.push("execution assessment module_id is invalid".to_string());
    }
    if !valid_version(&assessment.module_version) {
        errors.push("execution assessment module_version is invalid".to_string());
    }
    if assessment.decision != ExecutionDecision::Blocked || assessment.product_execution_authorized
    {
        errors.push(
            "schema-1 production execution assessment must remain blocked and unauthorized"
                .to_string(),
        );
    }
    validate_gate_set(&assessment.gates, &mut errors);

    let unresolved = assessment
        .gates
        .iter()
        .filter(|evidence| evidence.status != ExecutionGateStatus::Proven)
        .count();
    ProtocolValidation {
        valid: errors.is_empty(),
        errors,
        warnings: vec![format!(
            "schema-1 assessment cannot authorize execution; {unresolved} production gates remain unresolved"
        )],
    }
}

fn validate_gate_set(gates: &[ExecutionGateEvidence], errors: &mut Vec<String>) {
    let observed = gates
        .iter()
        .map(|evidence| evidence.gate)
        .collect::<Vec<_>>();
    let unique = observed.iter().copied().collect::<BTreeSet<_>>();
    if gates.len() != ALL_EXECUTION_GATES.len()
        || unique.len() != gates.len()
        || observed.as_slice() != ALL_EXECUTION_GATES
    {
        errors.push(
            "execution assessment gates must contain the exact unique canonical gate set"
                .to_string(),
        );
    }
    for evidence in gates {
        if evidence
            .mechanism
            .as_deref()
            .is_some_and(|value| !valid_detail(value))
        {
            errors.push("execution gate mechanism is invalid".to_string());
        }
        if evidence
            .evidence_reference
            .as_deref()
            .is_some_and(|value| !valid_reference(value))
        {
            errors.push("execution gate evidence_reference is invalid".to_string());
        }
        match evidence.status {
            ExecutionGateStatus::Proven => {
                if evidence.mechanism.is_none() || evidence.evidence_reference.is_none() {
                    errors.push(
                        "proven execution gates require mechanism and evidence_reference"
                            .to_string(),
                    );
                }
            }
            ExecutionGateStatus::Missing | ExecutionGateStatus::Unsupported => {
                if evidence.evidence_reference.is_some() {
                    errors
                        .push("unresolved execution gates cannot cite proof evidence".to_string());
                }
            }
        }
    }
}

fn valid_detail(value: &str) -> bool {
    rz0_validation_contract::valid_ascii_text(value, 160)
}

fn valid_reference(value: &str) -> bool {
    rz0_validation_contract::valid_evidence_reference(value, 120)
}
