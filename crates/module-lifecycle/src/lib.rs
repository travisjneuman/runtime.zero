use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MODULE_LIFECYCLE_SCHEMA_VERSION: u16 = 1;
pub const MODULE_LIFECYCLE_CONTRACT: &str = "module_lifecycle_plan";
pub const MAX_LIFECYCLE_GATES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleLifecycleState {
    Absent,
    Staged,
    InstalledInactive,
    Active,
    Degraded,
    Quarantined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleLifecycleOperation {
    Install,
    Activate,
    Invoke,
    Deactivate,
    Repair,
    Migrate,
    Upgrade,
    Uninstall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleFoundationGate {
    ArtifactIdentity,
    CapabilityPolicy,
    Confirmation,
    ProcessIsolation,
    Rollback,
    Transaction,
    Trust,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleLifecyclePlan {
    pub schema_version: u16,
    pub contract: String,
    pub transition_id: String,
    pub module_id: String,
    pub operation: ModuleLifecycleOperation,
    pub from_state: ModuleLifecycleState,
    pub to_state: ModuleLifecycleState,
    pub from_version: Option<String>,
    pub to_version: Option<String>,
    pub required_gates: Vec<LifecycleFoundationGate>,
    pub dry_run: bool,
    pub writes_attempted: bool,
    pub would_mutate: bool,
    pub rollback_required: bool,
    pub explicit_confirmation_required: bool,
    pub product_execution_authorized: bool,
    pub plan_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleLifecycleValidation {
    pub valid: bool,
    pub errors: Vec<String>,
}

pub fn module_lifecycle_plan(
    transition_id: impl Into<String>,
    module_id: impl Into<String>,
    operation: ModuleLifecycleOperation,
    from_state: ModuleLifecycleState,
    to_state: ModuleLifecycleState,
    from_version: Option<String>,
    to_version: Option<String>,
) -> Result<ModuleLifecyclePlan, ModuleLifecycleValidation> {
    let mutation = operation != ModuleLifecycleOperation::Invoke;
    let mut plan = ModuleLifecyclePlan {
        schema_version: MODULE_LIFECYCLE_SCHEMA_VERSION,
        contract: MODULE_LIFECYCLE_CONTRACT.to_string(),
        transition_id: transition_id.into(),
        module_id: module_id.into(),
        operation,
        from_state,
        to_state,
        from_version,
        to_version,
        required_gates: expected_gates(operation).to_vec(),
        dry_run: true,
        writes_attempted: false,
        would_mutate: mutation,
        rollback_required: mutation,
        explicit_confirmation_required: mutation,
        product_execution_authorized: false,
        plan_sha256: String::new(),
    };
    seal_module_lifecycle_plan(&mut plan);
    let validation = validate_module_lifecycle_plan(&plan);
    if validation.valid {
        Ok(plan)
    } else {
        Err(validation)
    }
}

pub fn seal_module_lifecycle_plan(plan: &mut ModuleLifecyclePlan) {
    plan.plan_sha256 = lifecycle_plan_sha256(plan);
}

pub fn validate_module_lifecycle_plan(plan: &ModuleLifecyclePlan) -> ModuleLifecycleValidation {
    let mut errors = Vec::new();
    if plan.schema_version != MODULE_LIFECYCLE_SCHEMA_VERSION {
        errors.push(format!(
            "schema_version must be {MODULE_LIFECYCLE_SCHEMA_VERSION}"
        ));
    }
    if plan.contract != MODULE_LIFECYCLE_CONTRACT {
        errors.push(format!("contract must be {MODULE_LIFECYCLE_CONTRACT}"));
    }
    if !rz0_validation_contract::valid_ledger_id(&plan.transition_id, 96) {
        errors.push("transition_id is invalid".to_string());
    }
    if plan.module_id.starts_with("core.")
        || !rz0_validation_contract::valid_module_id(&plan.module_id)
    {
        errors.push("module_id is invalid or reserved".to_string());
    }
    for (name, version) in [
        ("from_version", plan.from_version.as_deref()),
        ("to_version", plan.to_version.as_deref()),
    ] {
        if version.is_some_and(|value| !rz0_validation_contract::valid_version(value)) {
            errors.push(format!("{name} is invalid"));
        }
    }
    if !plan.dry_run || plan.writes_attempted || plan.product_execution_authorized {
        errors.push(
            "schema-1 lifecycle plans must remain dry-run, unattempted, and unauthorized"
                .to_string(),
        );
    }
    validate_gates(plan, &mut errors);
    validate_transition(plan, &mut errors);
    if !rz0_validation_contract::valid_sha256(&plan.plan_sha256)
        || plan.plan_sha256 != lifecycle_plan_sha256(plan)
    {
        errors.push("lifecycle plan digest is invalid".to_string());
    }
    errors.sort();
    errors.dedup();
    ModuleLifecycleValidation {
        valid: errors.is_empty(),
        errors,
    }
}

fn validate_gates(plan: &ModuleLifecyclePlan, errors: &mut Vec<String>) {
    let observed = plan.required_gates.iter().copied().collect::<BTreeSet<_>>();
    if plan.required_gates.is_empty()
        || plan.required_gates.len() > MAX_LIFECYCLE_GATES
        || observed.len() != plan.required_gates.len()
        || plan
            .required_gates
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        errors.push("required_gates must be a unique ascending bounded set".to_string());
        return;
    }
    let expected = expected_gates(plan.operation);
    if plan.required_gates.as_slice() != expected {
        errors.push("operation does not use the exact foundation gate set".to_string());
    }
}

fn validate_transition(plan: &ModuleLifecyclePlan, errors: &mut Vec<String>) {
    use ModuleLifecycleOperation as Operation;
    use ModuleLifecycleState as State;

    let same_version = plan.from_version.is_some() && plan.from_version == plan.to_version;
    let valid = match plan.operation {
        Operation::Install => {
            plan.from_state == State::Absent
                && plan.to_state == State::InstalledInactive
                && plan.from_version.is_none()
                && plan.to_version.is_some()
        }
        Operation::Activate => {
            plan.from_state == State::InstalledInactive
                && plan.to_state == State::Active
                && same_version
        }
        Operation::Invoke => {
            plan.from_state == State::Active && plan.to_state == State::Active && same_version
        }
        Operation::Deactivate => {
            plan.from_state == State::Active
                && plan.to_state == State::InstalledInactive
                && same_version
        }
        Operation::Repair => {
            matches!(plan.from_state, State::Degraded | State::InstalledInactive)
                && plan.to_state == State::InstalledInactive
                && same_version
        }
        Operation::Migrate => {
            plan.from_state == State::InstalledInactive
                && plan.to_state == State::InstalledInactive
                && same_version
        }
        Operation::Upgrade => {
            plan.from_state == State::InstalledInactive
                && plan.to_state == State::InstalledInactive
                && plan.from_version.is_some()
                && plan.to_version.is_some()
                && plan.from_version != plan.to_version
        }
        Operation::Uninstall => {
            plan.from_state == State::InstalledInactive
                && plan.to_state == State::Absent
                && plan.from_version.is_some()
                && plan.to_version.is_none()
        }
    };
    if !valid {
        errors.push("operation uses an unsafe or impossible lifecycle transition".to_string());
    }

    let mutation = plan.operation != Operation::Invoke;
    if plan.would_mutate != mutation
        || plan.rollback_required != mutation
        || plan.explicit_confirmation_required != mutation
    {
        errors.push("lifecycle mutation/rollback/confirmation flags are inconsistent".to_string());
    }
}

fn expected_gates(operation: ModuleLifecycleOperation) -> &'static [LifecycleFoundationGate] {
    use LifecycleFoundationGate as Gate;
    use ModuleLifecycleOperation as Operation;

    match operation {
        Operation::Invoke => &[
            Gate::ArtifactIdentity,
            Gate::CapabilityPolicy,
            Gate::ProcessIsolation,
            Gate::Trust,
        ],
        Operation::Activate | Operation::Deactivate | Operation::Migrate => &[
            Gate::ArtifactIdentity,
            Gate::CapabilityPolicy,
            Gate::Confirmation,
            Gate::Rollback,
            Gate::Transaction,
            Gate::Trust,
        ],
        Operation::Install | Operation::Repair | Operation::Upgrade | Operation::Uninstall => &[
            Gate::ArtifactIdentity,
            Gate::CapabilityPolicy,
            Gate::Confirmation,
            Gate::ProcessIsolation,
            Gate::Rollback,
            Gate::Transaction,
            Gate::Trust,
        ],
    }
}

fn lifecycle_plan_sha256(plan: &ModuleLifecyclePlan) -> String {
    let mut digest = Sha256::new();
    digest.update(b"runtime.zero.module-lifecycle-plan.v1\0");
    put(&mut digest, &plan.transition_id);
    put(&mut digest, &plan.module_id);
    put(&mut digest, operation_name(plan.operation));
    put(&mut digest, state_name(plan.from_state));
    put(&mut digest, state_name(plan.to_state));
    put_optional(&mut digest, plan.from_version.as_deref());
    put_optional(&mut digest, plan.to_version.as_deref());
    digest.update((plan.required_gates.len() as u64).to_be_bytes());
    for gate in &plan.required_gates {
        put(&mut digest, gate_name(*gate));
    }
    for value in [
        plan.dry_run,
        plan.writes_attempted,
        plan.would_mutate,
        plan.rollback_required,
        plan.explicit_confirmation_required,
        plan.product_execution_authorized,
    ] {
        digest.update([u8::from(value)]);
    }
    format!("{:x}", digest.finalize())
}

fn put(digest: &mut Sha256, value: &str) {
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value.as_bytes());
}

fn put_optional(digest: &mut Sha256, value: Option<&str>) {
    digest.update([u8::from(value.is_some())]);
    if let Some(value) = value {
        put(digest, value);
    }
}

fn operation_name(operation: ModuleLifecycleOperation) -> &'static str {
    match operation {
        ModuleLifecycleOperation::Install => "install",
        ModuleLifecycleOperation::Activate => "activate",
        ModuleLifecycleOperation::Invoke => "invoke",
        ModuleLifecycleOperation::Deactivate => "deactivate",
        ModuleLifecycleOperation::Repair => "repair",
        ModuleLifecycleOperation::Migrate => "migrate",
        ModuleLifecycleOperation::Upgrade => "upgrade",
        ModuleLifecycleOperation::Uninstall => "uninstall",
    }
}

fn state_name(state: ModuleLifecycleState) -> &'static str {
    match state {
        ModuleLifecycleState::Absent => "absent",
        ModuleLifecycleState::Staged => "staged",
        ModuleLifecycleState::InstalledInactive => "installed_inactive",
        ModuleLifecycleState::Active => "active",
        ModuleLifecycleState::Degraded => "degraded",
        ModuleLifecycleState::Quarantined => "quarantined",
    }
}

fn gate_name(gate: LifecycleFoundationGate) -> &'static str {
    match gate {
        LifecycleFoundationGate::ArtifactIdentity => "artifact_identity",
        LifecycleFoundationGate::CapabilityPolicy => "capability_policy",
        LifecycleFoundationGate::Confirmation => "confirmation",
        LifecycleFoundationGate::ProcessIsolation => "process_isolation",
        LifecycleFoundationGate::Rollback => "rollback",
        LifecycleFoundationGate::Transaction => "transaction",
        LifecycleFoundationGate::Trust => "trust",
    }
}
