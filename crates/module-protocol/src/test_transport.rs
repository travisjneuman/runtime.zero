//! Explicit-feature contracts for the integration-test process helper.
//!
//! These shapes authorize only the Cargo-built `rz0-protocol-test-child` test
//! artifact. They are not module-execution authorization and are not compiled
//! by default.

use serde::{Deserialize, Serialize};

use crate::{InvocationPlan, ProtocolValidation, policy::valid_id, validate_invocation_plan};

pub const TEST_TRANSPORT_SCHEMA_VERSION: u16 = 1;
pub const TEST_TRANSPORT_REQUEST_CONTRACT: &str = "module_process_test_request";
pub const TEST_TRANSPORT_RESPONSE_CONTRACT: &str = "module_process_test_response";
pub const TEST_HELPER_ID: &str = "rz0-protocol-test-child";
pub const TEST_WORK_MARKER: &str = ".rz0-protocol-test-work-v1";
pub const TEST_WORK_MARKER_CONTENT: &[u8] = b"schema_version=1\ntest_only=true\n";
pub const MAX_TEST_FRAME_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TestTransportRequest {
    pub schema_version: u16,
    pub contract: String,
    pub test_only: bool,
    pub test_execution_authorized: bool,
    pub helper_id: String,
    pub request_id: String,
    pub behavior: TestChildBehavior,
    pub expected_environment_names: Vec<String>,
    pub expected_working_directory_marker: String,
    pub preview: InvocationPlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TestChildBehavior {
    Respond,
    StderrBurst,
    StdoutFlood,
    StderrFlood,
    Sleep,
    DescendantSleep,
    Malformed,
    ExitFailure,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TestTransportResponse {
    pub schema_version: u16,
    pub contract: String,
    pub test_only: bool,
    pub helper_id: String,
    pub request_id: String,
    pub status: TestTransportStatus,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub network_attempted: bool,
    pub observed_environment_names: Vec<String>,
    pub argument_count: u64,
    pub working_directory_marker_present: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TestTransportStatus {
    Completed,
}

pub fn validate_test_transport_request(request: &TestTransportRequest) -> ProtocolValidation {
    let mut validation = validate_invocation_plan(&request.preview);
    if request.schema_version != TEST_TRANSPORT_SCHEMA_VERSION {
        validation.errors.push(format!(
            "test transport schema_version must be {TEST_TRANSPORT_SCHEMA_VERSION}"
        ));
    }
    if request.contract != TEST_TRANSPORT_REQUEST_CONTRACT {
        validation.errors.push(format!(
            "test transport contract must be {TEST_TRANSPORT_REQUEST_CONTRACT}"
        ));
    }
    if !request.test_only || !request.test_execution_authorized {
        validation
            .errors
            .push("test transport must be explicitly test-only and test-authorized".to_string());
    }
    if request.helper_id != TEST_HELPER_ID {
        validation
            .errors
            .push("test transport helper identity is invalid".to_string());
    }
    if !valid_id(&request.request_id) || request.request_id != request.preview.request_id {
        validation
            .errors
            .push("test transport request identity is invalid".to_string());
    }
    if request.expected_environment_names != request.preview.environment.allowed_names {
        validation
            .errors
            .push("test transport environment names must match the preview allowlist".to_string());
    }
    if request.expected_working_directory_marker != TEST_WORK_MARKER {
        validation
            .errors
            .push("test transport working-directory marker is invalid".to_string());
    }
    validation.warnings = vec![
        "explicit-feature Cargo test helper only; module execution remains unauthorized"
            .to_string(),
    ];
    validation.valid = validation.errors.is_empty();
    validation
}

pub fn validate_test_transport_response(
    request: &TestTransportRequest,
    response: &TestTransportResponse,
) -> ProtocolValidation {
    let mut validation = validate_test_transport_request(request);
    if response.schema_version != TEST_TRANSPORT_SCHEMA_VERSION
        || response.contract != TEST_TRANSPORT_RESPONSE_CONTRACT
    {
        validation
            .errors
            .push("test transport response schema or contract is invalid".to_string());
    }
    if !response.test_only
        || response.helper_id != request.helper_id
        || response.request_id != request.request_id
    {
        validation
            .errors
            .push("test transport response identity is invalid".to_string());
    }
    if response.status != TestTransportStatus::Completed
        || !response.read_only
        || response.writes_attempted
        || response.network_attempted
    {
        validation
            .errors
            .push("test transport response violates the read-only posture".to_string());
    }
    if response.observed_environment_names != request.expected_environment_names {
        validation
            .errors
            .push("test child environment does not match the exact allowlist".to_string());
    }
    if response.argument_count != 1 || !response.working_directory_marker_present {
        validation
            .errors
            .push("test child received arguments or the wrong working directory".to_string());
    }
    validation.valid = validation.errors.is_empty();
    validation
}
