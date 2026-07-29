pub use rz0_capability_contract::Capability as ProtocolCapability;
pub use rz0_error_contract::FoundationErrorCode as ProtocolErrorCode;
pub use rz0_resource_contract::ProcessLimits;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_SCHEMA_VERSION: u16 = 1;
pub const INVOCATION_PLAN_CONTRACT: &str = "module_invocation_plan";
pub const INVOCATION_RESPONSE_CONTRACT: &str = "module_invocation_response";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationPlan {
    pub schema_version: u16,
    pub contract: String,
    pub request_id: String,
    pub module_id: String,
    pub module_version: String,
    pub platform: ProtocolPlatform,
    pub operation: ProtocolOperation,
    pub dry_run: bool,
    pub read_only: bool,
    pub execution_authorized: bool,
    pub execution_attempted: bool,
    pub mutation_allowed: bool,
    pub network_allowed: bool,
    pub executable: ExecutableBinding,
    pub signature: SignatureBinding,
    pub limits: ProcessLimits,
    pub environment: EnvironmentPolicy,
    pub capabilities: Vec<ProtocolCapability>,
    pub inventory: InventoryInvocation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolPlatform {
    Windows,
    Macos,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolOperation {
    CollectInventory,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutableBinding {
    pub source: ExecutablePathSource,
    pub relative_path: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutablePathSource {
    VerifiedReceipt,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SignatureBinding {
    pub verified: bool,
    pub test_key_only: bool,
    pub key_id: String,
    pub manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentPolicy {
    pub clear_parent: bool,
    pub inherit_parent: bool,
    pub allowed_names: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InventoryInvocation {
    pub include_apps: bool,
    pub probe_versions: bool,
    pub redact_paths: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InvocationResponse {
    pub schema_version: u16,
    pub contract: String,
    pub request_id: String,
    pub module_id: String,
    pub status: InvocationStatus,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub network_attempted: bool,
    pub timed_out: bool,
    pub exit_code: Option<i32>,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub output_truncated: bool,
    pub payload_sha256: Option<String>,
    pub error_code: Option<ProtocolErrorCode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InvocationStatus {
    NotExecuted,
    Success,
    Partial,
    Failed,
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProtocolValidation {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}
