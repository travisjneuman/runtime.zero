use rz0_capability_contract::Capability;
use rz0_confirmation_contract::{
    CONFIRMATION_CHALLENGE_CONTRACT, CONFIRMATION_CONSUMPTION_CONTRACT,
    CONFIRMATION_RESPONSE_CONTRACT, CONFIRMATION_SCHEMA_VERSION, ConfirmationChallenge,
    ConfirmationConsumption, ConfirmationResponse, ConfirmationRisk, ConfirmationSurface,
    confirmation_consumption_file_name, confirmation_response_sha256, seal_confirmation_challenge,
    seal_confirmation_consumption, validate_confirmation, validate_confirmation_consumption,
};

const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";

#[test]
fn exact_interactive_plan_confirmation_is_valid_but_never_execution_authority() {
    let challenge = challenge();
    let response = valid_response(&challenge);
    let assessment = validate_confirmation(&challenge, &response, 1_120);
    assert!(assessment.valid, "{:?}", assessment.errors);
    assert!(assessment.plan_confirmed);
    assert!(!assessment.execution_authorized);
    assert!(assessment.durable_consumption_required);
    assert_eq!(assessment.challenge_sha256, challenge.challenge_sha256);
    assert_eq!(assessment.response_sha256.len(), 64);
}

#[test]
fn any_plan_preview_write_set_or_state_drift_invalidates_the_challenge() {
    let baseline = challenge();
    for mutate in [
        |value: &mut ConfirmationChallenge| value.plan_sha256 = B.to_string(),
        |value: &mut ConfirmationChallenge| value.dry_run_sha256 = C.to_string(),
        |value: &mut ConfirmationChallenge| value.write_set_sha256 = D.to_string(),
        |value: &mut ConfirmationChallenge| value.expected_after_state_sha256 = A.to_string(),
    ] {
        let mut drifted = baseline.clone();
        mutate(&mut drifted);
        let assessment = validate_confirmation(&drifted, &valid_response(&baseline), 1_120);
        assert!(!assessment.valid);
        assert!(
            assessment
                .errors
                .iter()
                .any(|error| error.contains("digest"))
        );
    }
}

#[test]
fn expiry_future_confirmation_and_phrase_reuse_fail_closed() {
    let challenge = challenge();
    let mut response = valid_response(&challenge);
    response.confirmed_unix_seconds = 1_201;
    assert!(!validate_confirmation(&challenge, &response, 1_201).valid);

    let response = valid_response(&challenge);
    assert!(!validate_confirmation(&challenge, &response, 1_201).valid);

    let mut response = valid_response(&challenge);
    response.phrase.push_str(" other");
    assert!(!validate_confirmation(&challenge, &response, 1_120).valid);
}

#[test]
fn confirmation_requires_no_write_dry_run_and_rollback_story() {
    let mut challenge = challenge();
    challenge.dry_run_completed = false;
    challenge.dry_run_writes_attempted = true;
    challenge.rollback_available = false;
    challenge.quarantine_available = false;
    seal_confirmation_challenge(&mut challenge);
    let assessment = validate_confirmation(&challenge, &valid_response(&challenge), 1_120);
    assert!(!assessment.valid);
    assert!(
        assessment
            .errors
            .iter()
            .any(|error| error.contains("dry run"))
    );
    assert!(
        assessment
            .errors
            .iter()
            .any(|error| error.contains("rollback"))
    );
}

#[test]
fn duplicate_unsorted_or_read_only_capabilities_fail_closed() {
    for capabilities in [
        vec![Capability::RuntimeStateWrite, Capability::RuntimeStateWrite],
        vec![Capability::RuntimeStateWrite, Capability::NetworkMetadata],
        vec![Capability::FilesystemMetadataRead],
    ] {
        let mut challenge = challenge();
        challenge.capabilities = capabilities;
        seal_confirmation_challenge(&mut challenge);
        assert!(!validate_confirmation(&challenge, &valid_response(&challenge), 1_120).valid);
    }
}

#[test]
fn durable_consumption_binds_one_response_to_one_transaction() {
    let challenge = challenge();
    let response = valid_response(&challenge);
    let consumption = consumption(&challenge, &response);
    let assessment = validate_confirmation_consumption(&consumption, &challenge, &response);
    assert!(assessment.valid, "{:?}", assessment.errors);
    assert_eq!(
        confirmation_consumption_file_name(&consumption),
        format!("{}.json", confirmation_response_sha256(&response))
    );
}

#[test]
fn consumption_identity_tampering_and_fabricated_authority_fail_closed() {
    let challenge = challenge();
    let response = valid_response(&challenge);
    let mut consumption = consumption(&challenge, &response);
    consumption.transaction_id = "rz0tx-other".to_string();
    consumption.execution_authorized = true;
    let assessment = validate_confirmation_consumption(&consumption, &challenge, &response);
    assert!(!assessment.valid);
    assert!(!assessment.execution_authorized);
    assert!(
        assessment
            .errors
            .iter()
            .any(|error| error.contains("binding"))
    );
}

#[test]
fn unknown_fields_or_fabricated_execution_authority_are_rejected() {
    let challenge = challenge();
    let response = valid_response(&challenge);
    let json = serde_json::to_string(&response).expect("serialize response");
    let unknown = json.replacen(
        "\"schema_version\":1",
        "\"schema_version\":1,\"unexpected\":true",
        1,
    );
    assert!(serde_json::from_str::<ConfirmationResponse>(&unknown).is_err());

    let mut fabricated = response;
    fabricated.execution_authorized = true;
    fabricated.interactive = false;
    fabricated.single_use = false;
    let assessment = validate_confirmation(&challenge, &fabricated, 1_120);
    assert!(!assessment.valid);
    assert!(!assessment.execution_authorized);
}

fn challenge() -> ConfirmationChallenge {
    let mut challenge = ConfirmationChallenge {
        schema_version: CONFIRMATION_SCHEMA_VERSION,
        contract: CONFIRMATION_CHALLENGE_CONTRACT.to_string(),
        challenge_id: "challenge-001".to_string(),
        plan_id: "rz0plan-update-example".to_string(),
        plan_sha256: A.to_string(),
        dry_run_sha256: B.to_string(),
        write_set_sha256: C.to_string(),
        before_state_sha256: None,
        expected_after_state_sha256: D.to_string(),
        risk: ConfirmationRisk::Mutating,
        action_count: 1,
        capabilities: vec![
            Capability::NetworkMetadata,
            Capability::ManagerExecution,
            Capability::RuntimeStateWrite,
        ],
        issued_unix_seconds: 1_000,
        expires_unix_seconds: 1_200,
        dry_run_completed: true,
        dry_run_writes_attempted: false,
        rollback_available: true,
        quarantine_available: false,
        expected_phrase: String::new(),
        challenge_sha256: String::new(),
    };
    seal_confirmation_challenge(&mut challenge);
    challenge
}

fn consumption(
    challenge: &ConfirmationChallenge,
    response: &ConfirmationResponse,
) -> ConfirmationConsumption {
    let mut consumption = ConfirmationConsumption {
        schema_version: CONFIRMATION_SCHEMA_VERSION,
        contract: CONFIRMATION_CONSUMPTION_CONTRACT.to_string(),
        transaction_id: "rz0tx-confirmed".to_string(),
        plan_id: challenge.plan_id.clone(),
        challenge_sha256: challenge.challenge_sha256.clone(),
        response_sha256: confirmation_response_sha256(response),
        consumed_unix_seconds: 1_110,
        single_use_consumed: true,
        execution_authorized: false,
        binding_sha256: String::new(),
    };
    seal_confirmation_consumption(&mut consumption);
    consumption
}

fn valid_response(challenge: &ConfirmationChallenge) -> ConfirmationResponse {
    ConfirmationResponse {
        schema_version: CONFIRMATION_SCHEMA_VERSION,
        contract: CONFIRMATION_RESPONSE_CONTRACT.to_string(),
        challenge_id: challenge.challenge_id.clone(),
        challenge_sha256: challenge.challenge_sha256.clone(),
        confirmed_unix_seconds: 1_100,
        surface: ConfirmationSurface::Cli,
        phrase: challenge.expected_phrase.clone(),
        interactive: true,
        single_use: true,
        execution_authorized: false,
    }
}
