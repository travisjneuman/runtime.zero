use std::collections::BTreeSet;

use rz0_resource_contract::{ProcessLimitCeilings, ProcessLimitField};

use crate::model::{
    INVOCATION_PLAN_CONTRACT, INVOCATION_RESPONSE_CONTRACT, InvocationPlan, InvocationResponse,
    InvocationStatus, PROTOCOL_SCHEMA_VERSION, ProtocolCapability, ProtocolErrorCode,
    ProtocolPlatform, ProtocolValidation,
};
use crate::policy::{valid_id, valid_relative_path, valid_sha256, valid_version};

const MAX_EXECUTABLE_BYTES: u64 = rz0_resource_contract::MAX_ARTIFACT_BYTES;
const MAX_ENV_NAMES: usize = 16;
const MAX_CAPABILITIES: usize = 16;

pub fn validate_invocation_plan(plan: &InvocationPlan) -> ProtocolValidation {
    let mut errors = Vec::new();
    if plan.schema_version != PROTOCOL_SCHEMA_VERSION {
        errors.push(format!(
            "invocation schema_version must be {PROTOCOL_SCHEMA_VERSION}"
        ));
    }
    if plan.contract != INVOCATION_PLAN_CONTRACT {
        errors.push(format!(
            "invocation contract must be {INVOCATION_PLAN_CONTRACT}"
        ));
    }
    if !valid_id(&plan.request_id) {
        errors.push("request_id is invalid".to_string());
    }
    if plan.module_id != "first-party.inventory" {
        errors.push("schema 1 supports only first-party.inventory".to_string());
    }
    if !valid_version(&plan.module_version) {
        errors.push("module_version is invalid".to_string());
    }
    if !plan.dry_run
        || !plan.read_only
        || plan.execution_authorized
        || plan.execution_attempted
        || plan.mutation_allowed
        || plan.network_allowed
    {
        errors.push(
            "schema-1 invocation plans must remain read-only, dry-run, unauthorized, unattempted, offline previews"
                .to_string(),
        );
    }
    validate_executable(plan, &mut errors);
    validate_signature(plan, &mut errors);
    validate_limits(plan, &mut errors);
    validate_environment(plan, &mut errors);
    validate_capabilities(plan, &mut errors);
    if !plan.inventory.redact_paths {
        errors.push("schema-1 inventory protocol requires path redaction".to_string());
    }
    finish(errors)
}

pub fn validate_invocation_response(
    plan: &InvocationPlan,
    response: &InvocationResponse,
) -> ProtocolValidation {
    let mut validation = validate_invocation_plan(plan);
    let errors = &mut validation.errors;
    if response.schema_version != PROTOCOL_SCHEMA_VERSION {
        errors.push(format!(
            "response schema_version must be {PROTOCOL_SCHEMA_VERSION}"
        ));
    }
    if response.contract != INVOCATION_RESPONSE_CONTRACT {
        errors.push(format!(
            "response contract must be {INVOCATION_RESPONSE_CONTRACT}"
        ));
    }
    if response.request_id != plan.request_id || response.module_id != plan.module_id {
        errors.push("response identity does not match invocation plan".to_string());
    }
    if !response.read_only || response.writes_attempted || response.network_attempted {
        errors.push("response violates the read-only/offline posture".to_string());
    }
    if response.stdout_bytes > plan.limits.stdout_bytes
        || response.stderr_bytes > plan.limits.stderr_bytes
    {
        errors.push("response byte counts exceed invocation limits".to_string());
    }
    if response.status != InvocationStatus::NotExecuted
        || response.timed_out
        || response.exit_code.is_some()
        || response.stdout_bytes != 0
        || response.stderr_bytes != 0
        || response.output_truncated
        || response.payload_sha256.is_some()
        || response.error_code != Some(ProtocolErrorCode::ExecutionNotAuthorized)
    {
        errors.push(
            "schema-1 response must report that execution was not authorized or attempted"
                .to_string(),
        );
    }
    validation.valid = validation.errors.is_empty();
    validation
}

fn validate_executable(plan: &InvocationPlan, errors: &mut Vec<String>) {
    if !valid_relative_path(&plan.executable.relative_path)
        || !plan.executable.relative_path.starts_with("bin/")
    {
        errors.push("executable path must be a safe receipt-relative bin/ path".to_string());
    }
    if !valid_sha256(&plan.executable.sha256) {
        errors.push("executable sha256 is invalid".to_string());
    }
    if plan.executable.size_bytes == 0 || plan.executable.size_bytes > MAX_EXECUTABLE_BYTES {
        errors.push(format!(
            "executable size_bytes must be between 1 and {MAX_EXECUTABLE_BYTES}"
        ));
    }
}

fn validate_signature(plan: &InvocationPlan, errors: &mut Vec<String>) {
    if !plan.signature.verified || !plan.signature.test_key_only {
        errors.push("invocation preview requires verified test-key metadata".to_string());
    }
    if !valid_id(&plan.signature.key_id) {
        errors.push("signature key_id is invalid".to_string());
    }
    if !valid_sha256(&plan.signature.manifest_sha256) {
        errors.push("signature manifest_sha256 is invalid".to_string());
    }
}

fn validate_limits(plan: &InvocationPlan, errors: &mut Vec<String>) {
    let ceilings = ProcessLimitCeilings::MODULE_SCHEMA_ONE;
    for field in plan.limits.violations(ceilings) {
        let (name, maximum) = match field {
            ProcessLimitField::TimeoutMs => ("timeout_ms", ceilings.timeout_ms),
            ProcessLimitField::StdinBytes => ("stdin_bytes", ceilings.stdin_bytes),
            ProcessLimitField::StdoutBytes => ("stdout_bytes", ceilings.stdout_bytes),
            ProcessLimitField::StderrBytes => ("stderr_bytes", ceilings.stderr_bytes),
        };
        errors.push(format!("{name} must be between 1 and {maximum}"));
    }
}

fn validate_environment(plan: &InvocationPlan, errors: &mut Vec<String>) {
    let environment = &plan.environment;
    if !environment.clear_parent || environment.inherit_parent {
        errors.push("child environment must be cleared and never inherited wholesale".to_string());
    }
    if environment.allowed_names.len() > MAX_ENV_NAMES {
        errors.push(format!(
            "environment allowlist exceeds {MAX_ENV_NAMES} names"
        ));
    }
    let names = environment
        .allowed_names
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    if names.len() != environment.allowed_names.len()
        || environment
            .allowed_names
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
    {
        errors.push("environment allowlist must be unique and sorted".to_string());
    }
    let (allowed, required): (&[&str], &[&str]) = match plan.platform {
        ProtocolPlatform::Windows => (&["PATH", "SystemRoot", "WINDIR"], &["PATH", "SystemRoot"]),
        ProtocolPlatform::Macos => (&["HOME", "PATH"], &["HOME", "PATH"]),
        ProtocolPlatform::Linux => (
            &["HOME", "PATH", "XDG_DATA_DIRS", "XDG_DATA_HOME"],
            &["HOME", "PATH"],
        ),
    };
    if names.iter().any(|name| !allowed.contains(name))
        || required.iter().any(|name| !names.contains(name))
    {
        errors.push("environment allowlist is not minimal for the target platform".to_string());
    }
}

fn validate_capabilities(plan: &InvocationPlan, errors: &mut Vec<String>) {
    let validation = rz0_capability_contract::validate_schema_one_protocol_grants(
        &plan.capabilities,
        MAX_CAPABILITIES,
    );
    errors.extend(validation.errors.into_iter().map(|error| {
        if error == "capability grant is outside its schema family" {
            "schema-1 protocol grant includes a non-read capability".to_string()
        } else {
            error.to_string()
        }
    }));
    let capabilities = plan.capabilities.iter().copied().collect::<BTreeSet<_>>();
    for required in [
        ProtocolCapability::ProcessEnvironmentRead,
        ProtocolCapability::FilesystemMetadataRead,
    ] {
        if !capabilities.contains(&required) {
            errors.push("capability grant omits a required read-only capability".to_string());
            break;
        }
    }

    let persisted = capabilities.contains(&ProtocolCapability::PersistedEnvironmentRegistryRead);
    let app_registry = capabilities.contains(&ProtocolCapability::ApplicationRegistryRead);
    let app_files = capabilities.contains(&ProtocolCapability::ApplicationFilesystemRead);
    let probes = capabilities.contains(&ProtocolCapability::ExactCommandProbe);
    match plan.platform {
        ProtocolPlatform::Windows => {
            if !persisted || app_files {
                errors.push("Windows capability grant has invalid platform reads".to_string());
            }
            if plan.inventory.include_apps != app_registry {
                errors.push("Windows app option and registry grant must match".to_string());
            }
        }
        ProtocolPlatform::Macos | ProtocolPlatform::Linux => {
            if persisted || app_registry {
                errors.push("non-Windows capability grant includes registry reads".to_string());
            }
            if plan.inventory.include_apps != app_files {
                errors.push("app option and filesystem grant must match".to_string());
            }
        }
    }
    if plan.inventory.probe_versions != probes {
        errors.push("probe option and exact-command grant must match".to_string());
    }
}

fn finish(errors: Vec<String>) -> ProtocolValidation {
    ProtocolValidation {
        valid: errors.is_empty(),
        errors,
        warnings: vec![
            "fixture contract only; core process execution remains unauthorized".to_string(),
        ],
    }
}
