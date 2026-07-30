use rz0_resource_contract::{MAX_SMALL_DOCUMENT_BYTES, ProcessLimitCeilings, ProcessLimits};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CONFIGURATION_SCHEMA_VERSION: u16 = 1;
pub const CONFIGURATION_CONTRACT: &str = "foundation_configuration";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FoundationConfiguration {
    pub schema_version: u16,
    pub contract: String,
    pub source: ConfigurationSource,
    pub privacy: PrivacyConfiguration,
    pub execution: ExecutionConfiguration,
    pub mutation: MutationConfiguration,
    pub lifecycle: LifecycleConfiguration,
    pub configuration_authorizes_execution: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConfigurationSource {
    BuiltInDefaults,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PrivacyConfiguration {
    pub redact_local_paths_by_default: bool,
    pub collect_hostname: bool,
    pub collect_current_user: bool,
    pub collect_environment_values: bool,
    pub telemetry_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionConfiguration {
    pub production_modules: DisabledSetting,
    pub remote_execution: DisabledSetting,
    pub shell_execution: DisabledSetting,
    pub network_default: NetworkDefault,
    pub maximum_parallel_module_processes: u16,
    pub process_limits: ProcessLimits,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MutationConfiguration {
    pub dry_run_required: bool,
    pub exact_confirmation_required: bool,
    pub quarantine_before_removal: bool,
    pub automatic_retry: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleConfiguration {
    pub automatic_update: bool,
    pub background_service: bool,
    pub implicit_migration: bool,
    pub startup_repair: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DisabledSetting {
    Disabled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkDefault {
    Deny,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigurationValidation {
    pub valid: bool,
    pub errors: Vec<String>,
}

pub fn default_configuration() -> FoundationConfiguration {
    let limits = ProcessLimitCeilings::MODULE_SCHEMA_ONE;
    FoundationConfiguration {
        schema_version: CONFIGURATION_SCHEMA_VERSION,
        contract: CONFIGURATION_CONTRACT.to_string(),
        source: ConfigurationSource::BuiltInDefaults,
        privacy: PrivacyConfiguration {
            redact_local_paths_by_default: true,
            collect_hostname: false,
            collect_current_user: false,
            collect_environment_values: false,
            telemetry_enabled: false,
        },
        execution: ExecutionConfiguration {
            production_modules: DisabledSetting::Disabled,
            remote_execution: DisabledSetting::Disabled,
            shell_execution: DisabledSetting::Disabled,
            network_default: NetworkDefault::Deny,
            maximum_parallel_module_processes: 1,
            process_limits: ProcessLimits {
                timeout_ms: limits.timeout_ms,
                stdin_bytes: limits.stdin_bytes,
                stdout_bytes: limits.stdout_bytes,
                stderr_bytes: limits.stderr_bytes,
            },
        },
        mutation: MutationConfiguration {
            dry_run_required: true,
            exact_confirmation_required: true,
            quarantine_before_removal: true,
            automatic_retry: false,
        },
        lifecycle: LifecycleConfiguration {
            automatic_update: false,
            background_service: false,
            implicit_migration: false,
            startup_repair: false,
        },
        configuration_authorizes_execution: false,
    }
}

pub fn validate_configuration(configuration: &FoundationConfiguration) -> ConfigurationValidation {
    let expected = default_configuration();
    let mut errors = Vec::new();
    if configuration.schema_version != CONFIGURATION_SCHEMA_VERSION {
        errors.push(format!(
            "schema_version must be {CONFIGURATION_SCHEMA_VERSION}"
        ));
    }
    if configuration.contract != CONFIGURATION_CONTRACT {
        errors.push(format!("contract must be {CONFIGURATION_CONTRACT}"));
    }
    if configuration.source != ConfigurationSource::BuiltInDefaults {
        errors.push("schema-1 configuration source must be built-in defaults".to_string());
    }
    if configuration.privacy != expected.privacy {
        errors.push("schema-1 privacy settings cannot be weakened".to_string());
    }
    if configuration.execution != expected.execution {
        errors.push("schema-1 execution settings cannot be enabled or expanded".to_string());
    }
    if configuration.mutation != expected.mutation {
        errors.push("schema-1 mutation safeguards cannot be disabled".to_string());
    }
    if configuration.lifecycle != expected.lifecycle {
        errors.push("schema-1 lifecycle automation must remain disabled".to_string());
    }
    if configuration.configuration_authorizes_execution {
        errors.push("configuration cannot authorize execution".to_string());
    }
    errors.sort();
    errors.dedup();
    ConfigurationValidation {
        valid: errors.is_empty(),
        errors,
    }
}

pub fn canonical_configuration_bytes(
    configuration: &FoundationConfiguration,
) -> Result<Vec<u8>, String> {
    let validation = validate_configuration(configuration);
    if !validation.valid {
        return Err(validation.errors.join("; "));
    }
    let mut bytes = serde_json::to_vec(configuration)
        .map_err(|error| format!("serialize foundation configuration: {error}"))?;
    bytes.push(b'\n');
    if bytes.len() as u64 > MAX_SMALL_DOCUMENT_BYTES {
        return Err("foundation configuration exceeds its byte ceiling".to_string());
    }
    Ok(bytes)
}

pub fn configuration_sha256(configuration: &FoundationConfiguration) -> Result<String, String> {
    canonical_configuration_bytes(configuration).map(|bytes| format!("{:x}", Sha256::digest(bytes)))
}

pub fn decode_configuration(bytes: &[u8]) -> Result<FoundationConfiguration, String> {
    if bytes.is_empty() || bytes.len() as u64 > MAX_SMALL_DOCUMENT_BYTES {
        return Err("foundation configuration is empty or oversized".to_string());
    }
    let configuration: FoundationConfiguration = serde_json::from_slice(bytes)
        .map_err(|error| format!("parse foundation configuration: {error}"))?;
    let validation = validate_configuration(&configuration);
    if validation.valid {
        Ok(configuration)
    } else {
        Err(validation.errors.join("; "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_private_offline_bounded_and_non_authorizing() {
        let configuration = default_configuration();
        let validation = validate_configuration(&configuration);
        assert!(validation.valid, "{:?}", validation.errors);
        assert!(configuration.privacy.redact_local_paths_by_default);
        assert!(!configuration.privacy.telemetry_enabled);
        assert_eq!(
            configuration.execution.network_default,
            NetworkDefault::Deny
        );
        assert_eq!(configuration.execution.maximum_parallel_module_processes, 1);
        assert!(!configuration.configuration_authorizes_execution);
    }

    #[test]
    fn canonical_bytes_and_digest_are_deterministic() {
        let configuration = default_configuration();
        let first = canonical_configuration_bytes(&configuration).unwrap();
        let second = canonical_configuration_bytes(&configuration).unwrap();
        assert_eq!(first, second);
        assert!(first.ends_with(b"\n"));
        assert_eq!(
            configuration_sha256(&configuration).unwrap(),
            configuration_sha256(&configuration).unwrap()
        );
        assert_eq!(decode_configuration(&first).unwrap(), configuration);
    }

    #[test]
    fn permissive_drift_fails_closed() {
        let mut configuration = default_configuration();
        configuration.privacy.redact_local_paths_by_default = false;
        configuration.execution.maximum_parallel_module_processes = 2;
        configuration.mutation.automatic_retry = true;
        configuration.lifecycle.automatic_update = true;
        configuration.configuration_authorizes_execution = true;
        let validation = validate_configuration(&configuration);
        assert!(!validation.valid);
        assert_eq!(validation.errors.len(), 5);
        assert!(canonical_configuration_bytes(&configuration).is_err());
    }

    #[test]
    fn unknown_fields_and_oversized_documents_fail_closed() {
        let bytes = canonical_configuration_bytes(&default_configuration()).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let drifted = text.replacen(
            "\"schema_version\":1",
            "\"schema_version\":1,\"future\":true",
            1,
        );
        assert!(decode_configuration(drifted.as_bytes()).is_err());
        assert!(decode_configuration(&vec![b'x'; MAX_SMALL_DOCUMENT_BYTES as usize + 1]).is_err());
    }
}
