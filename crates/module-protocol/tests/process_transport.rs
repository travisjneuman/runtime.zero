#![cfg(feature = "protocol-test-child")]

mod support;

use std::path::Path;

use rz0_module_protocol::{
    InvocationPlan, ProtocolCapability, ProtocolPlatform,
    test_transport::{
        TEST_HELPER_ID, TEST_TRANSPORT_REQUEST_CONTRACT, TEST_TRANSPORT_SCHEMA_VERSION,
        TEST_WORK_MARKER, TestChildBehavior, TestTransportRequest, validate_test_transport_request,
        validate_test_transport_response,
    },
};

use support::{
    host::run_test_transport,
    temp_root::{TestRoot, sha256_file},
};

#[cfg(unix)]
use support::host::run_test_transport_with_inheritable_descriptor;

fn compiled_helper() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_rz0-protocol-test-child"))
}

fn request(root: &TestRoot, behavior: TestChildBehavior) -> TestTransportRequest {
    let mut preview: InvocationPlan =
        serde_json::from_str(include_str!("fixtures/valid-inventory-plan.json"))
            .expect("valid invocation preview");
    preview.request_id = format!("transport-{}", behavior_name(behavior));
    preview.executable.relative_path = root.executable_relative_path();
    preview.executable.sha256 = sha256_file(root.executable()).expect("helper digest");
    preview.executable.size_bytes = std::fs::metadata(root.executable())
        .expect("helper metadata")
        .len();
    preview.limits.timeout_ms = 2_000;
    preview.limits.stdout_bytes = 64 * 1024;
    preview.limits.stderr_bytes = 64 * 1024;
    configure_platform(&mut preview);

    TestTransportRequest {
        schema_version: TEST_TRANSPORT_SCHEMA_VERSION,
        contract: TEST_TRANSPORT_REQUEST_CONTRACT.to_string(),
        test_only: true,
        test_execution_authorized: true,
        helper_id: TEST_HELPER_ID.to_string(),
        request_id: preview.request_id.clone(),
        behavior,
        expected_environment_names: preview.environment.allowed_names.clone(),
        expected_working_directory_marker: TEST_WORK_MARKER.to_string(),
        preview,
    }
}

#[test]
fn exact_helper_transport_clears_environment_and_sets_working_directory() {
    let root = TestRoot::new(compiled_helper());
    let request = request(&root, TestChildBehavior::Respond);
    assert!(!request.preview.execution_authorized);
    let validation = validate_test_transport_request(&request);
    assert!(validation.valid, "{:?}", validation.errors);

    let result = run_test_transport(&root, &request, &root.environment()).expect("test transport");
    assert_eq!(result.exit_code, Some(0));
    assert!(result.stdout_bytes > 0);
    assert_eq!(result.stderr_bytes, 0);
    assert_eq!(
        result.response.observed_environment_names,
        request.expected_environment_names
    );
    assert!(result.response.working_directory_marker_present);

    let mut fabricated_write = result.response;
    fabricated_write.writes_attempted = true;
    assert!(!validate_test_transport_response(&request, &fabricated_write).valid);
}

#[test]
fn concurrent_stderr_drain_prevents_pipe_deadlock() {
    let root = TestRoot::new(compiled_helper());
    let request = request(&root, TestChildBehavior::StderrBurst);
    let result = run_test_transport(&root, &request, &root.environment()).expect("test transport");
    assert_eq!(result.exit_code, Some(0));
    assert_eq!(result.stderr_bytes, 32 * 1024);
}

#[test]
fn timeout_kills_and_reaps_direct_test_child() {
    let root = TestRoot::new(compiled_helper());
    let mut request = request(&root, TestChildBehavior::Sleep);
    request.preview.limits.timeout_ms = 100;
    let failure = run_test_transport(&root, &request, &root.environment()).unwrap_err();
    assert_eq!(failure.code, "timed_out", "{}", failure.detail);
    assert!(failure.timed_out);
}

#[cfg(unix)]
#[test]
fn timeout_terminates_descendant_process_group_and_closes_pipes() {
    let root = TestRoot::new(compiled_helper());
    let mut request = request(&root, TestChildBehavior::DescendantSleep);
    request.preview.limits.timeout_ms = 500;
    let failure = run_test_transport(&root, &request, &root.environment()).unwrap_err();
    assert_eq!(failure.code, "timed_out", "{}", failure.detail);
    assert!(failure.timed_out);
    assert!(
        failure.stderr_bytes > 0,
        "descendant did not report startup"
    );
}

#[cfg(unix)]
#[test]
fn nonstandard_inheritable_descriptor_blocks_spawn() {
    use std::{fs::File, os::fd::AsRawFd};

    let root = TestRoot::new(compiled_helper());
    let inherited = File::open(root.work().join(TEST_WORK_MARKER)).expect("open marker descriptor");
    let descriptor = inherited.as_raw_fd();
    let request = request(&root, TestChildBehavior::Respond);
    let failure = run_test_transport_with_inheritable_descriptor(
        &root,
        &request,
        &root.environment(),
        descriptor,
    )
    .unwrap_err();
    assert_eq!(failure.code, "preflight_failed", "{}", failure.detail);
    assert!(
        failure
            .detail
            .contains("inheritable descriptor audit failed")
    );
}

#[test]
fn stdout_and_stderr_are_drained_but_fail_when_limits_are_exceeded() {
    let root = TestRoot::new(compiled_helper());
    let mut stdout_request = request(&root, TestChildBehavior::StdoutFlood);
    stdout_request.preview.limits.stdout_bytes = 8 * 1024;
    let stdout_failure =
        run_test_transport(&root, &stdout_request, &root.environment()).unwrap_err();
    assert_eq!(stdout_failure.code, "stdout_limit_exceeded");
    assert!(stdout_failure.stdout_bytes > stdout_request.preview.limits.stdout_bytes);

    let mut stderr_request = request(&root, TestChildBehavior::StderrFlood);
    stderr_request.preview.limits.stderr_bytes = 8 * 1024;
    let stderr_failure =
        run_test_transport(&root, &stderr_request, &root.environment()).unwrap_err();
    assert_eq!(stderr_failure.code, "stderr_limit_exceeded");
    assert!(stderr_failure.stderr_bytes > stderr_request.preview.limits.stderr_bytes);
}

#[test]
fn malformed_frames_and_nonzero_exit_fail_closed() {
    let root = TestRoot::new(compiled_helper());
    let malformed = request(&root, TestChildBehavior::Malformed);
    let failure = run_test_transport(&root, &malformed, &root.environment()).unwrap_err();
    assert_eq!(failure.code, "response_parse_failed");
    assert_eq!(failure.exit_code, Some(0));

    let exit_failure = request(&root, TestChildBehavior::ExitFailure);
    let failure = run_test_transport(&root, &exit_failure, &root.environment()).unwrap_err();
    assert_eq!(failure.code, "child_exit_failed");
    assert_eq!(failure.exit_code, Some(7));
}

#[test]
fn authorization_identity_digest_and_environment_drift_fail_before_spawn() {
    let root = TestRoot::new(compiled_helper());
    let mut unauthorized = request(&root, TestChildBehavior::Respond);
    unauthorized.test_execution_authorized = false;
    assert!(!validate_test_transport_request(&unauthorized).valid);

    let mut wrong_helper = request(&root, TestChildBehavior::Respond);
    wrong_helper.helper_id = "rz0-inventory".to_string();
    let failure = run_test_transport(&root, &wrong_helper, &root.environment()).unwrap_err();
    assert_eq!(failure.code, "preflight_failed");

    let mut wrong_digest = request(&root, TestChildBehavior::Respond);
    wrong_digest.preview.executable.sha256 = "0".repeat(64);
    let failure = run_test_transport(&root, &wrong_digest, &root.environment()).unwrap_err();
    assert_eq!(failure.code, "preflight_failed");

    let mut extra_environment = root.environment();
    extra_environment.insert("RZ0_PARENT_SECRET".to_string(), "must-not-pass".into());
    let valid = request(&root, TestChildBehavior::Respond);
    let failure = run_test_transport(&root, &valid, &extra_environment).unwrap_err();
    assert_eq!(failure.code, "preflight_failed");
}

#[cfg(unix)]
#[test]
fn symlinked_test_helper_fails_before_spawn() {
    use std::{fs, os::unix::fs::symlink};

    let root = TestRoot::new(compiled_helper());
    let request = request(&root, TestChildBehavior::Respond);
    fs::remove_file(root.executable()).expect("remove copied helper");
    symlink(compiled_helper(), root.executable()).expect("symlink test helper");
    let failure = run_test_transport(&root, &request, &root.environment()).unwrap_err();
    assert_eq!(failure.code, "preflight_failed");
}

fn configure_platform(preview: &mut InvocationPlan) {
    #[cfg(target_os = "windows")]
    {
        preview.platform = ProtocolPlatform::Windows;
        preview.environment.allowed_names = vec!["PATH".to_string(), "SystemRoot".to_string()];
        preview.capabilities = vec![
            ProtocolCapability::ProcessEnvironmentRead,
            ProtocolCapability::FilesystemMetadataRead,
            ProtocolCapability::PersistedEnvironmentRegistryRead,
            ProtocolCapability::ApplicationRegistryRead,
        ];
    }
    #[cfg(target_os = "macos")]
    {
        preview.platform = ProtocolPlatform::Macos;
        preview.environment.allowed_names = vec!["HOME".to_string(), "PATH".to_string()];
        preview.capabilities = vec![
            ProtocolCapability::ProcessEnvironmentRead,
            ProtocolCapability::FilesystemMetadataRead,
            ProtocolCapability::ApplicationFilesystemRead,
        ];
    }
    #[cfg(target_os = "linux")]
    {
        preview.platform = ProtocolPlatform::Linux;
        preview.environment.allowed_names = vec!["HOME".to_string(), "PATH".to_string()];
        preview.capabilities = vec![
            ProtocolCapability::ProcessEnvironmentRead,
            ProtocolCapability::FilesystemMetadataRead,
            ProtocolCapability::ApplicationFilesystemRead,
        ];
    }
}

fn behavior_name(behavior: TestChildBehavior) -> &'static str {
    match behavior {
        TestChildBehavior::Respond => "respond",
        TestChildBehavior::StderrBurst => "stderr-burst",
        TestChildBehavior::StdoutFlood => "stdout-flood",
        TestChildBehavior::StderrFlood => "stderr-flood",
        TestChildBehavior::Sleep => "sleep",
        TestChildBehavior::DescendantSleep => "descendant-sleep",
        TestChildBehavior::Malformed => "malformed",
        TestChildBehavior::ExitFailure => "exit-failure",
    }
}
