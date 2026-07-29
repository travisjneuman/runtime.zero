use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::Write,
    process::{Command, Stdio},
    sync::{Mutex, MutexGuard},
    thread,
    time::{Duration, Instant},
};

use rz0_module_protocol::test_transport::{
    TestTransportRequest, TestTransportResponse, validate_test_transport_response,
};

use super::{
    capture::{Capture, drain_bounded},
    outcome::{TransportFailure, TransportSuccess, failure, failure_with_state},
    preflight::validate_preflight,
    process_isolation::{configure_test_process, terminate_test_process},
    temp_root::{TestRoot, sha256_file},
};

static PROCESS_TEST_LOCK: Mutex<()> = Mutex::new(());

pub fn run_test_transport(
    root: &TestRoot,
    request: &TestTransportRequest,
    environment: &BTreeMap<String, OsString>,
) -> Result<TransportSuccess, TransportFailure> {
    let _guard = process_test_lock();
    run_test_transport_inner(root, request, environment)
}

#[cfg(unix)]
pub fn run_test_transport_with_inheritable_descriptor(
    root: &TestRoot,
    request: &TestTransportRequest,
    environment: &BTreeMap<String, OsString>,
    descriptor: i32,
) -> Result<TransportSuccess, TransportFailure> {
    use super::process_isolation::InheritableDescriptorGuard;

    let _guard = process_test_lock();
    let _descriptor = InheritableDescriptorGuard::new(descriptor)
        .map_err(|error| failure("descriptor_setup_failed", error.to_string()))?;
    run_test_transport_inner(root, request, environment)
}

fn process_test_lock() -> MutexGuard<'static, ()> {
    PROCESS_TEST_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn run_test_transport_inner(
    root: &TestRoot,
    request: &TestTransportRequest,
    environment: &BTreeMap<String, OsString>,
) -> Result<TransportSuccess, TransportFailure> {
    let _verified_artifact = validate_preflight(root, request, environment)
        .map_err(|detail| failure("preflight_failed", detail))?;
    let mut input = serde_json::to_vec(request)
        .map_err(|error| failure("request_serialization_failed", error.to_string()))?;
    input.push(b'\n');
    if input.len() as u64 > request.preview.limits.stdin_bytes {
        return Err(failure(
            "stdin_limit_exceeded",
            "serialized request exceeds the invocation ceiling".to_string(),
        ));
    }

    let executable = fs::canonicalize(root.executable())
        .map_err(|error| failure("preflight_failed", error.to_string()))?;
    let working_directory = fs::canonicalize(root.work())
        .map_err(|error| failure("preflight_failed", error.to_string()))?;
    let mut command = Command::new(executable);
    command
        .current_dir(working_directory)
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, value) in environment {
        command.env(name, value);
    }
    configure_test_process(&mut command);
    let mut child = command
        .spawn()
        .map_err(|error| failure("spawn_failed", error.to_string()))?;
    let mut child_stdin = child.stdin.take().expect("piped child stdin");
    let child_stdout = child.stdout.take().expect("piped child stdout");
    let child_stderr = child.stderr.take().expect("piped child stderr");

    let writer = thread::spawn(move || -> Result<(), std::io::Error> {
        child_stdin.write_all(&input)?;
        child_stdin.flush()
    });
    let stdout_limit = request.preview.limits.stdout_bytes;
    let stdout_reader = thread::spawn(move || drain_bounded(child_stdout, stdout_limit));
    let stderr_limit = request.preview.limits.stderr_bytes;
    let stderr_reader = thread::spawn(move || drain_bounded(child_stderr, stderr_limit));

    let deadline = Instant::now() + Duration::from_millis(request.preview.limits.timeout_ms);
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            Ok(None) => {
                timed_out = true;
                let _ = terminate_test_process(&mut child);
                break child.wait().map_err(|error| {
                    failure_with_state("reap_failed", error.to_string(), true, None, 0, 0)
                })?;
            }
            Err(error) => {
                let _ = terminate_test_process(&mut child);
                let _ = child.wait();
                return Err(failure("wait_failed", error.to_string()));
            }
        }
    };

    let writer_result = writer.join().map_err(|_| {
        failure_with_state(
            "stdin_thread_failed",
            "stdin thread panicked".to_string(),
            timed_out,
            status.code(),
            0,
            0,
        )
    })?;
    let stdout = join_capture(
        stdout_reader,
        "stdout_thread_failed",
        timed_out,
        status.code(),
    )?;
    let stderr = join_capture(
        stderr_reader,
        "stderr_thread_failed",
        timed_out,
        status.code(),
    )?;
    if timed_out {
        return Err(failure_with_state(
            "timed_out",
            "test child exceeded the invocation deadline".to_string(),
            true,
            status.code(),
            stdout.total_bytes,
            stderr.total_bytes,
        ));
    }
    if let Err(error) = writer_result {
        return Err(failure_with_state(
            "stdin_write_failed",
            error.to_string(),
            false,
            status.code(),
            stdout.total_bytes,
            stderr.total_bytes,
        ));
    }
    if stdout.truncated || stderr.truncated {
        let code = if stdout.truncated {
            "stdout_limit_exceeded"
        } else {
            "stderr_limit_exceeded"
        };
        return Err(failure_with_state(
            code,
            "test child output exceeded the retained byte ceiling".to_string(),
            false,
            status.code(),
            stdout.total_bytes,
            stderr.total_bytes,
        ));
    }
    if !status.success() {
        return Err(failure_with_state(
            "child_exit_failed",
            "test child returned a non-success status".to_string(),
            false,
            status.code(),
            stdout.total_bytes,
            stderr.total_bytes,
        ));
    }

    let response: TestTransportResponse =
        serde_json::from_slice(&stdout.bytes).map_err(|error| {
            failure_with_state(
                "response_parse_failed",
                error.to_string(),
                false,
                status.code(),
                stdout.total_bytes,
                stderr.total_bytes,
            )
        })?;
    let validation = validate_test_transport_response(request, &response);
    if !validation.valid {
        return Err(failure_with_state(
            "response_validation_failed",
            format!("{:?}", validation.errors),
            false,
            status.code(),
            stdout.total_bytes,
            stderr.total_bytes,
        ));
    }
    let final_digest = sha256_file(root.executable())
        .map_err(|detail| failure("post_spawn_verification_failed", detail))?;
    if final_digest != request.preview.executable.sha256 {
        return Err(failure(
            "post_spawn_verification_failed",
            "test helper digest changed across execution".to_string(),
        ));
    }
    Ok(TransportSuccess {
        response,
        exit_code: status.code(),
        stdout_bytes: stdout.total_bytes,
        stderr_bytes: stderr.total_bytes,
    })
}

fn join_capture(
    handle: thread::JoinHandle<std::io::Result<Capture>>,
    code: &'static str,
    timed_out: bool,
    exit_code: Option<i32>,
) -> Result<Capture, TransportFailure> {
    handle
        .join()
        .map_err(|_| {
            failure_with_state(
                code,
                "capture thread panicked".to_string(),
                timed_out,
                exit_code,
                0,
                0,
            )
        })?
        .map_err(|error| failure_with_state(code, error.to_string(), timed_out, exit_code, 0, 0))
}
