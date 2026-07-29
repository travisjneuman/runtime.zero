use rz0_module_protocol::{
    ExecutionGateStatus, ProductionExecutionAssessment, ProtocolPlatform,
    validate_production_execution_assessment,
};

fn blocked_assessment() -> ProductionExecutionAssessment {
    serde_json::from_str(include_str!("fixtures/blocked-production-execution.json"))
        .expect("blocked production assessment fixture")
}

#[test]
fn current_production_execution_assessment_is_valid_and_blocked() {
    let assessment = blocked_assessment();
    let validation = validate_production_execution_assessment(&assessment);
    assert!(validation.valid, "{:?}", validation.errors);
    assert!(!assessment.product_execution_authorized);
    assert!(
        validation
            .warnings
            .iter()
            .any(|warning| warning.contains("27 production gates"))
    );
}

#[test]
fn the_same_fail_closed_contract_applies_to_every_platform() {
    for platform in [
        ProtocolPlatform::Windows,
        ProtocolPlatform::Macos,
        ProtocolPlatform::Linux,
    ] {
        let mut assessment = blocked_assessment();
        assessment.platform = platform;
        assert!(validate_production_execution_assessment(&assessment).valid);
    }
}

#[test]
fn missing_duplicate_or_unsorted_gate_sets_fail_closed() {
    let mut missing = blocked_assessment();
    missing.gates.pop();
    assert!(!validate_production_execution_assessment(&missing).valid);

    let mut duplicate = blocked_assessment();
    duplicate.gates[1] = duplicate.gates[0].clone();
    assert!(!validate_production_execution_assessment(&duplicate).valid);

    let mut unsorted = blocked_assessment();
    unsorted.gates.swap(0, 1);
    assert!(!validate_production_execution_assessment(&unsorted).valid);
}

#[test]
fn proof_requires_a_bounded_mechanism_and_reference() {
    let mut missing_reference = blocked_assessment();
    missing_reference.gates[0].status = ExecutionGateStatus::Proven;
    let validation = validate_production_execution_assessment(&missing_reference);
    assert!(!validation.valid);
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("require mechanism and evidence_reference"))
    );

    let mut unresolved_with_proof = blocked_assessment();
    unresolved_with_proof.gates[0].evidence_reference = Some("fabricated-proof".to_string());
    let validation = validate_production_execution_assessment(&unresolved_with_proof);
    assert!(!validation.valid);
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("cannot cite proof"))
    );
}

#[test]
fn schema_one_cannot_authorize_product_execution_even_if_all_gates_are_proven() {
    let mut assessment = blocked_assessment();
    for evidence in &mut assessment.gates {
        evidence.status = ExecutionGateStatus::Proven;
        evidence.mechanism = Some("synthetic complete evidence for contract test".to_string());
        evidence.evidence_reference = Some("synthetic-contract-proof".to_string());
    }
    let validation = validate_production_execution_assessment(&assessment);
    assert!(validation.valid, "{:?}", validation.errors);
    assert!(!assessment.product_execution_authorized);
    assert!(
        validation
            .warnings
            .iter()
            .any(|warning| warning.contains("0 production gates"))
    );

    assessment.product_execution_authorized = true;
    let validation = validate_production_execution_assessment(&assessment);
    assert!(!validation.valid);
    assert!(
        validation
            .errors
            .iter()
            .any(|error| error.contains("blocked and unauthorized"))
    );
}

#[test]
fn unknown_fields_and_authorized_decisions_fail_deserialization() {
    let source = include_str!("fixtures/blocked-production-execution.json");
    let unknown = source.replacen(
        "\"schema_version\": 1,",
        "\"schema_version\": 1, \"surprise\": true,",
        1,
    );
    assert!(serde_json::from_str::<ProductionExecutionAssessment>(&unknown).is_err());

    let authorized = source.replacen(
        "\"decision\": \"blocked\"",
        "\"decision\": \"authorized\"",
        1,
    );
    assert!(serde_json::from_str::<ProductionExecutionAssessment>(&authorized).is_err());
}
