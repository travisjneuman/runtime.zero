use rz0_module_lifecycle::{
    LifecycleFoundationGate as Gate, MODULE_LIFECYCLE_CONTRACT, MODULE_LIFECYCLE_SCHEMA_VERSION,
    ModuleLifecycleOperation as Operation, ModuleLifecyclePlan, ModuleLifecycleState as State,
    seal_module_lifecycle_plan, validate_module_lifecycle_plan,
};

#[test]
fn every_safe_lifecycle_transition_uses_foundation_gates() {
    for mut plan in [
        plan(
            Operation::Install,
            State::Absent,
            State::InstalledInactive,
            None,
            Some("1.0.0"),
        ),
        plan(
            Operation::Activate,
            State::InstalledInactive,
            State::Active,
            Some("1.0.0"),
            Some("1.0.0"),
        ),
        plan(
            Operation::Invoke,
            State::Active,
            State::Active,
            Some("1.0.0"),
            Some("1.0.0"),
        ),
        plan(
            Operation::Deactivate,
            State::Active,
            State::InstalledInactive,
            Some("1.0.0"),
            Some("1.0.0"),
        ),
        plan(
            Operation::Repair,
            State::Degraded,
            State::InstalledInactive,
            Some("1.0.0"),
            Some("1.0.0"),
        ),
        plan(
            Operation::Migrate,
            State::InstalledInactive,
            State::InstalledInactive,
            Some("1.0.0"),
            Some("1.0.0"),
        ),
        plan(
            Operation::Upgrade,
            State::InstalledInactive,
            State::InstalledInactive,
            Some("1.0.0"),
            Some("2.0.0"),
        ),
        plan(
            Operation::Uninstall,
            State::InstalledInactive,
            State::Absent,
            Some("1.0.0"),
            None,
        ),
    ] {
        seal_module_lifecycle_plan(&mut plan);
        let validation = validate_module_lifecycle_plan(&plan);
        assert!(
            validation.valid,
            "{:?}: {:?}",
            plan.operation, validation.errors
        );
        assert!(!plan.product_execution_authorized);
    }
}

#[test]
fn active_modules_must_deactivate_before_upgrade_or_uninstall() {
    for operation in [Operation::Upgrade, Operation::Uninstall] {
        let mut plan = plan(
            operation,
            State::Active,
            State::InstalledInactive,
            Some("1.0.0"),
            Some("2.0.0"),
        );
        seal_module_lifecycle_plan(&mut plan);
        assert!(!validate_module_lifecycle_plan(&plan).valid);
    }
}

#[test]
fn invoke_is_nonmutating_but_still_requires_identity_capability_isolation_and_trust() {
    let mut plan = plan(
        Operation::Invoke,
        State::Active,
        State::Active,
        Some("1.0.0"),
        Some("1.0.0"),
    );
    assert_eq!(
        plan.required_gates,
        [
            Gate::ArtifactIdentity,
            Gate::CapabilityPolicy,
            Gate::ProcessIsolation,
            Gate::Trust,
        ]
    );
    assert!(!plan.would_mutate);
    assert!(!plan.rollback_required);
    assert!(!plan.explicit_confirmation_required);
    seal_module_lifecycle_plan(&mut plan);
    assert!(validate_module_lifecycle_plan(&plan).valid);
}

#[test]
fn gate_flag_digest_and_unknown_field_drift_fail_closed() {
    let mut baseline = plan(
        Operation::Install,
        State::Absent,
        State::InstalledInactive,
        None,
        Some("1.0.0"),
    );
    seal_module_lifecycle_plan(&mut baseline);
    for mutate in [
        |plan: &mut ModuleLifecyclePlan| plan.required_gates.reverse(),
        |plan: &mut ModuleLifecyclePlan| plan.rollback_required = false,
        |plan: &mut ModuleLifecyclePlan| plan.product_execution_authorized = true,
        |plan: &mut ModuleLifecyclePlan| plan.plan_sha256 = "0".repeat(64),
    ] {
        let mut drifted = baseline.clone();
        mutate(&mut drifted);
        assert!(!validate_module_lifecycle_plan(&drifted).valid);
    }
    let json = serde_json::to_string(&baseline).unwrap().replacen(
        "\"schema_version\":1",
        "\"schema_version\":1,\"activated\":true",
        1,
    );
    assert!(serde_json::from_str::<ModuleLifecyclePlan>(&json).is_err());
}

fn plan(
    operation: Operation,
    from_state: State,
    to_state: State,
    from_version: Option<&str>,
    to_version: Option<&str>,
) -> ModuleLifecyclePlan {
    let mutation = operation != Operation::Invoke;
    ModuleLifecyclePlan {
        schema_version: MODULE_LIFECYCLE_SCHEMA_VERSION,
        contract: MODULE_LIFECYCLE_CONTRACT.to_string(),
        transition_id: format!("transition-{operation:?}").to_ascii_lowercase(),
        module_id: "first-party.inventory".to_string(),
        operation,
        from_state,
        to_state,
        from_version: from_version.map(str::to_string),
        to_version: to_version.map(str::to_string),
        required_gates: gates(operation),
        dry_run: true,
        writes_attempted: false,
        would_mutate: mutation,
        rollback_required: mutation,
        explicit_confirmation_required: mutation,
        product_execution_authorized: false,
        plan_sha256: String::new(),
    }
}

fn gates(operation: Operation) -> Vec<Gate> {
    match operation {
        Operation::Invoke => vec![
            Gate::ArtifactIdentity,
            Gate::CapabilityPolicy,
            Gate::ProcessIsolation,
            Gate::Trust,
        ],
        Operation::Activate | Operation::Deactivate | Operation::Migrate => vec![
            Gate::ArtifactIdentity,
            Gate::CapabilityPolicy,
            Gate::Confirmation,
            Gate::Rollback,
            Gate::Transaction,
            Gate::Trust,
        ],
        Operation::Install
        | Operation::Repair
        | Operation::Upgrade
        | Operation::Uninstall
        | Operation::Recover => vec![
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
