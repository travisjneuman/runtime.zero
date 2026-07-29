use std::{collections::BTreeMap, ffi::OsString, fs};

use rz0_artifact_identity::{ArtifactExpectation, VerifiedArtifact, open_verified_artifact};
use rz0_module_protocol::test_transport::{TestTransportRequest, validate_test_transport_request};

use super::{process_isolation::audit_inheritable_descriptors, temp_root::TestRoot};

pub fn validate_preflight(
    root: &TestRoot,
    request: &TestTransportRequest,
    environment: &BTreeMap<String, OsString>,
) -> Result<VerifiedArtifact, String> {
    root.validate()?;
    let validation = validate_test_transport_request(request);
    if !validation.valid {
        return Err(format!("invalid test request: {:?}", validation.errors));
    }
    let names = environment.keys().cloned().collect::<Vec<_>>();
    if names != request.expected_environment_names
        || environment
            .values()
            .any(|value| value.to_string_lossy().contains('\0'))
    {
        return Err("environment values do not match the exact name allowlist".to_string());
    }
    let expectation = ArtifactExpectation {
        sha256: request.preview.executable.sha256.clone(),
        size_bytes: request.preview.executable.size_bytes,
    };
    let verified = open_verified_artifact(
        root.receipt(),
        &request.preview.executable.relative_path,
        &expectation,
    )
    .map_err(|error| format!("test helper artifact identity failed: {error}"))?;
    if fs::canonicalize(root.executable()).ok().as_deref()
        != Some(verified.canonical_path.as_path())
    {
        return Err("plan executable does not identify the copied test helper".to_string());
    }
    audit_inheritable_descriptors()
        .map_err(|error| format!("inheritable descriptor audit failed: {error}"))?;
    Ok(verified)
}
