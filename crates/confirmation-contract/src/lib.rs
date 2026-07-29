use std::collections::BTreeSet;

use rz0_capability_contract::Capability;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CONFIRMATION_SCHEMA_VERSION: u16 = 1;
pub const CONFIRMATION_CHALLENGE_CONTRACT: &str = "plan_confirmation_challenge";
pub const CONFIRMATION_RESPONSE_CONTRACT: &str = "plan_confirmation_response";
pub const CONFIRMATION_CONSUMPTION_CONTRACT: &str = "plan_confirmation_consumption";
pub const MAX_CONFIRMATION_TTL_SECONDS: u64 = 300;
pub const MAX_CONFIRMED_ACTIONS: u16 = 128;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmationChallenge {
    pub schema_version: u16,
    pub contract: String,
    pub challenge_id: String,
    pub plan_id: String,
    pub plan_sha256: String,
    pub dry_run_sha256: String,
    pub write_set_sha256: String,
    pub before_state_sha256: Option<String>,
    pub expected_after_state_sha256: String,
    pub risk: ConfirmationRisk,
    pub action_count: u16,
    pub capabilities: Vec<Capability>,
    pub issued_unix_seconds: u64,
    pub expires_unix_seconds: u64,
    pub dry_run_completed: bool,
    pub dry_run_writes_attempted: bool,
    pub rollback_available: bool,
    pub quarantine_available: bool,
    pub expected_phrase: String,
    pub challenge_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationRisk {
    Mutating,
    Destructive,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmationResponse {
    pub schema_version: u16,
    pub contract: String,
    pub challenge_id: String,
    pub challenge_sha256: String,
    pub confirmed_unix_seconds: u64,
    pub surface: ConfirmationSurface,
    pub phrase: String,
    pub interactive: bool,
    pub single_use: bool,
    pub execution_authorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfirmationSurface {
    Cli,
    Tui,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmationAssessment {
    pub valid: bool,
    pub plan_confirmed: bool,
    pub execution_authorized: bool,
    pub durable_consumption_required: bool,
    pub challenge_sha256: String,
    pub response_sha256: String,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfirmationConsumption {
    pub schema_version: u16,
    pub contract: String,
    pub transaction_id: String,
    pub plan_id: String,
    pub challenge_sha256: String,
    pub response_sha256: String,
    pub consumed_unix_seconds: u64,
    pub single_use_consumed: bool,
    pub execution_authorized: bool,
    pub binding_sha256: String,
}

pub fn seal_confirmation_challenge(challenge: &mut ConfirmationChallenge) {
    let digest = challenge_digest(challenge);
    challenge.expected_phrase = confirmation_phrase(&challenge.plan_id, &digest);
    challenge.challenge_sha256 = digest;
}

pub fn validate_confirmation(
    challenge: &ConfirmationChallenge,
    response: &ConfirmationResponse,
    now_unix_seconds: u64,
) -> ConfirmationAssessment {
    let mut errors = Vec::new();
    validate_challenge(challenge, now_unix_seconds, &mut errors);
    if response.schema_version != CONFIRMATION_SCHEMA_VERSION {
        errors.push(format!(
            "response schema_version must be {CONFIRMATION_SCHEMA_VERSION}"
        ));
    }
    if response.contract != CONFIRMATION_RESPONSE_CONTRACT {
        errors.push(format!(
            "response contract must be {CONFIRMATION_RESPONSE_CONTRACT}"
        ));
    }
    if response.challenge_id != challenge.challenge_id
        || response.challenge_sha256 != challenge.challenge_sha256
    {
        errors.push("response does not bind the exact challenge".to_string());
    }
    if response.phrase != challenge.expected_phrase {
        errors.push("response phrase does not match the exact plan challenge".to_string());
    }
    if response.confirmed_unix_seconds < challenge.issued_unix_seconds
        || response.confirmed_unix_seconds > challenge.expires_unix_seconds
        || response.confirmed_unix_seconds > now_unix_seconds
    {
        errors.push("response confirmation time is outside the challenge window".to_string());
    }
    if !response.interactive || !response.single_use {
        errors.push("schema-1 confirmation must be interactive and single-use".to_string());
    }
    if response.execution_authorized {
        errors.push("confirmation cannot authorize execution".to_string());
    }
    errors.sort();
    errors.dedup();
    let valid = errors.is_empty();
    ConfirmationAssessment {
        valid,
        plan_confirmed: valid,
        execution_authorized: false,
        durable_consumption_required: true,
        challenge_sha256: challenge.challenge_sha256.clone(),
        response_sha256: confirmation_response_sha256(response),
        errors,
    }
}

pub fn seal_confirmation_consumption(consumption: &mut ConfirmationConsumption) {
    consumption.binding_sha256 = confirmation_consumption_sha256(consumption);
}

pub fn validate_confirmation_consumption(
    consumption: &ConfirmationConsumption,
    challenge: &ConfirmationChallenge,
    response: &ConfirmationResponse,
) -> ConfirmationAssessment {
    let mut assessment =
        validate_confirmation(challenge, response, consumption.consumed_unix_seconds);
    if consumption.schema_version != CONFIRMATION_SCHEMA_VERSION {
        assessment.errors.push(format!(
            "consumption schema_version must be {CONFIRMATION_SCHEMA_VERSION}"
        ));
    }
    if consumption.contract != CONFIRMATION_CONSUMPTION_CONTRACT {
        assessment.errors.push(format!(
            "consumption contract must be {CONFIRMATION_CONSUMPTION_CONTRACT}"
        ));
    }
    if !rz0_validation_contract::valid_ledger_id(&consumption.transaction_id, 96)
        || consumption.plan_id != challenge.plan_id
    {
        assessment
            .errors
            .push("consumption transaction or plan identity is invalid".to_string());
    }
    if consumption.challenge_sha256 != challenge.challenge_sha256
        || consumption.response_sha256 != confirmation_response_sha256(response)
    {
        assessment
            .errors
            .push("consumption does not bind the exact challenge and response".to_string());
    }
    if !consumption.single_use_consumed || consumption.execution_authorized {
        assessment
            .errors
            .push("consumption must be single-use and cannot authorize execution".to_string());
    }
    if !rz0_validation_contract::valid_sha256(&consumption.binding_sha256)
        || consumption.binding_sha256 != confirmation_consumption_sha256(consumption)
    {
        assessment
            .errors
            .push("consumption binding digest is invalid".to_string());
    }
    assessment.errors.sort();
    assessment.errors.dedup();
    assessment.valid = assessment.errors.is_empty();
    assessment.plan_confirmed = assessment.valid;
    assessment.execution_authorized = false;
    assessment
}

pub fn confirmation_consumption_file_name(consumption: &ConfirmationConsumption) -> String {
    format!("{}.json", consumption.response_sha256)
}

pub fn confirmation_response_sha256(response: &ConfirmationResponse) -> String {
    let mut digest = Sha256::new();
    digest.update(b"runtime.zero.plan-confirmation-response.v1\0");
    put(&mut digest, &response.challenge_id);
    put(&mut digest, &response.challenge_sha256);
    digest.update(response.confirmed_unix_seconds.to_be_bytes());
    put(
        &mut digest,
        match response.surface {
            ConfirmationSurface::Cli => "cli",
            ConfirmationSurface::Tui => "tui",
        },
    );
    put(&mut digest, &response.phrase);
    for value in [
        response.interactive,
        response.single_use,
        response.execution_authorized,
    ] {
        digest.update([u8::from(value)]);
    }
    format!("{:x}", digest.finalize())
}

fn confirmation_consumption_sha256(consumption: &ConfirmationConsumption) -> String {
    let mut digest = Sha256::new();
    digest.update(b"runtime.zero.plan-confirmation-consumption.v1\0");
    put(&mut digest, &consumption.transaction_id);
    put(&mut digest, &consumption.plan_id);
    put(&mut digest, &consumption.challenge_sha256);
    put(&mut digest, &consumption.response_sha256);
    digest.update(consumption.consumed_unix_seconds.to_be_bytes());
    digest.update([u8::from(consumption.single_use_consumed)]);
    digest.update([u8::from(consumption.execution_authorized)]);
    format!("{:x}", digest.finalize())
}

fn validate_challenge(
    challenge: &ConfirmationChallenge,
    now_unix_seconds: u64,
    errors: &mut Vec<String>,
) {
    if challenge.schema_version != CONFIRMATION_SCHEMA_VERSION {
        errors.push(format!(
            "challenge schema_version must be {CONFIRMATION_SCHEMA_VERSION}"
        ));
    }
    if challenge.contract != CONFIRMATION_CHALLENGE_CONTRACT {
        errors.push(format!(
            "challenge contract must be {CONFIRMATION_CHALLENGE_CONTRACT}"
        ));
    }
    if !rz0_validation_contract::valid_ledger_id(&challenge.challenge_id, 96) {
        errors.push("challenge_id is invalid".to_string());
    }
    if !rz0_validation_contract::valid_dotted_id(&challenge.plan_id, 100) {
        errors.push("plan_id is invalid".to_string());
    }
    for (field, value) in [
        ("plan_sha256", Some(&challenge.plan_sha256)),
        ("dry_run_sha256", Some(&challenge.dry_run_sha256)),
        ("write_set_sha256", Some(&challenge.write_set_sha256)),
        (
            "before_state_sha256",
            challenge.before_state_sha256.as_ref(),
        ),
        (
            "expected_after_state_sha256",
            Some(&challenge.expected_after_state_sha256),
        ),
        ("challenge_sha256", Some(&challenge.challenge_sha256)),
    ] {
        if value.is_some_and(|value| !rz0_validation_contract::valid_sha256(value)) {
            errors.push(format!("{field} must be canonical SHA-256"));
        }
    }
    if challenge.action_count == 0 || challenge.action_count > MAX_CONFIRMED_ACTIONS {
        errors.push(format!("action_count must be 1..={MAX_CONFIRMED_ACTIONS}"));
    }
    let capabilities = challenge
        .capabilities
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    if capabilities.is_empty()
        || capabilities.len() != challenge.capabilities.len()
        || challenge
            .capabilities
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        || capabilities
            .iter()
            .any(|capability| !capability.is_schema1_action_capability())
    {
        errors.push("capabilities must be unique sorted schema-1 action capabilities".to_string());
    }
    if !capabilities
        .iter()
        .any(|capability| capability.is_mutating())
    {
        errors.push("confirmation requires at least one mutating capability".to_string());
    }
    let ttl = challenge
        .expires_unix_seconds
        .checked_sub(challenge.issued_unix_seconds);
    if ttl.is_none_or(|ttl| ttl == 0 || ttl > MAX_CONFIRMATION_TTL_SECONDS)
        || now_unix_seconds > challenge.expires_unix_seconds
    {
        errors.push("challenge is expired or exceeds the five-minute lifetime".to_string());
    }
    if !challenge.dry_run_completed || challenge.dry_run_writes_attempted {
        errors.push("confirmation requires a completed no-write dry run".to_string());
    }
    if !challenge.rollback_available && !challenge.quarantine_available {
        errors.push("confirmation requires rollback or quarantine".to_string());
    }
    let expected_digest = challenge_digest(challenge);
    if challenge.challenge_sha256 != expected_digest
        || challenge.expected_phrase != confirmation_phrase(&challenge.plan_id, &expected_digest)
    {
        errors.push("challenge digest or phrase is invalid".to_string());
    }
}

fn challenge_digest(challenge: &ConfirmationChallenge) -> String {
    let mut digest = Sha256::new();
    digest.update(b"runtime.zero.plan-confirmation-challenge.v1\0");
    put(&mut digest, &challenge.challenge_id);
    put(&mut digest, &challenge.plan_id);
    put(&mut digest, &challenge.plan_sha256);
    put(&mut digest, &challenge.dry_run_sha256);
    put(&mut digest, &challenge.write_set_sha256);
    put_optional(&mut digest, challenge.before_state_sha256.as_deref());
    put(&mut digest, &challenge.expected_after_state_sha256);
    put(
        &mut digest,
        match challenge.risk {
            ConfirmationRisk::Mutating => "mutating",
            ConfirmationRisk::Destructive => "destructive",
        },
    );
    digest.update(challenge.action_count.to_be_bytes());
    digest.update((challenge.capabilities.len() as u64).to_be_bytes());
    for capability in &challenge.capabilities {
        put(&mut digest, capability_name(*capability));
    }
    digest.update(challenge.issued_unix_seconds.to_be_bytes());
    digest.update(challenge.expires_unix_seconds.to_be_bytes());
    for value in [
        challenge.dry_run_completed,
        challenge.dry_run_writes_attempted,
        challenge.rollback_available,
        challenge.quarantine_available,
    ] {
        digest.update([u8::from(value)]);
    }
    format!("{:x}", digest.finalize())
}

fn confirmation_phrase(plan_id: &str, digest: &str) -> String {
    format!("confirm {plan_id} {}", &digest[..12])
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

fn capability_name(capability: Capability) -> &'static str {
    match capability {
        Capability::ProcessEnvironmentRead => "process_environment_read",
        Capability::FilesystemMetadataRead => "filesystem_metadata_read",
        Capability::PersistedEnvironmentRegistryRead => "persisted_environment_registry_read",
        Capability::ApplicationRegistryRead => "application_registry_read",
        Capability::ApplicationFilesystemRead => "application_filesystem_read",
        Capability::ExactCommandProbe => "exact_command_probe",
        Capability::NetworkMetadata => "network_metadata",
        Capability::ManagerExecution => "manager_execution",
        Capability::ElevatedManagerAction => "elevated_manager_action",
        Capability::RuntimeStateWrite => "runtime_state_write",
        Capability::QuarantineWrite => "quarantine_write",
        Capability::RestoreWrite => "restore_write",
    }
}
