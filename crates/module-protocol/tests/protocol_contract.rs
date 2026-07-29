use rz0_module_protocol::{
    InvocationPlan, InvocationResponse, InvocationStatus, ProtocolCapability, ProtocolPlatform,
    validate_invocation_plan, validate_invocation_response,
};

fn valid_plan() -> InvocationPlan {
    serde_json::from_str(include_str!("fixtures/valid-inventory-plan.json"))
        .expect("valid invocation plan")
}

fn not_executed_response() -> InvocationResponse {
    serde_json::from_str(include_str!("fixtures/not-executed-response.json"))
        .expect("not-executed response")
}

#[test]
fn validates_read_only_unexecuted_inventory_protocol() {
    let plan = valid_plan();
    let validation = validate_invocation_plan(&plan);
    assert!(validation.valid, "{:?}", validation.errors);
    assert!(!plan.execution_authorized);
    assert!(!plan.execution_attempted);
    assert!(plan.inventory.redact_paths);

    let response = not_executed_response();
    let validation = validate_invocation_response(&plan, &response);
    assert!(validation.valid, "{:?}", validation.errors);
    assert_eq!(response.status, InvocationStatus::NotExecuted);
}

#[test]
fn rejects_authorization_mutation_network_environment_and_limit_drift() {
    let plan: InvocationPlan =
        serde_json::from_str(include_str!("fixtures/invalid-authorized-plan.json"))
            .expect("invalid policy fixture shape");
    let validation = validate_invocation_plan(&plan);
    assert!(!validation.valid);
    for expected in [
        "unauthorized",
        "executable path",
        "signature",
        "timeout_ms",
        "environment",
        "capability",
        "redaction",
    ] {
        assert!(
            validation
                .errors
                .iter()
                .any(|error| error.contains(expected)),
            "missing {expected}: {:?}",
            validation.errors
        );
    }
}

#[test]
fn platform_options_require_exact_least_privilege_grants() {
    let mut windows = valid_plan();
    windows.platform = ProtocolPlatform::Windows;
    windows.environment.allowed_names = vec!["PATH".to_string(), "SystemRoot".to_string()];
    windows.capabilities = vec![
        ProtocolCapability::ProcessEnvironmentRead,
        ProtocolCapability::FilesystemMetadataRead,
        ProtocolCapability::PersistedEnvironmentRegistryRead,
        ProtocolCapability::ApplicationRegistryRead,
    ];
    assert!(validate_invocation_plan(&windows).valid);

    let mut over_granted = windows.clone();
    over_granted
        .capabilities
        .push(ProtocolCapability::ManagerExecution);
    let validation = validate_invocation_plan(&over_granted);
    assert!(!validation.valid);
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("non-read capability"))
    );

    windows.inventory.probe_versions = true;
    let validation = validate_invocation_plan(&windows);
    assert!(!validation.valid);
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("probe option"))
    );
}

#[test]
fn fabricated_execution_response_is_rejected() {
    let plan = valid_plan();
    let mut response = not_executed_response();
    response.status = InvocationStatus::Success;
    response.exit_code = Some(0);
    response.stdout_bytes = 128;
    response.payload_sha256 = Some("a".repeat(64));
    response.error_code = None;
    let validation = validate_invocation_response(&plan, &response);
    assert!(!validation.valid);
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("not authorized"))
    );
}

#[test]
fn unknown_fields_fail_closed() {
    let source = include_str!("fixtures/valid-inventory-plan.json").replacen(
        "\"schema_version\": 1,",
        "\"schema_version\": 1, \"surprise\": true,",
        1,
    );
    assert!(serde_json::from_str::<InvocationPlan>(&source).is_err());
}
