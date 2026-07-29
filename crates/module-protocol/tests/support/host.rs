use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    io::Write,
    path::{Component, Path},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use rz0_module_protocol::test_transport::{
    TestTransportRequest, TestTransportResponse, validate_test_transport_request,
    validate_test_transport_response,
};

use super::{
    capture::{Capture, drain_bounded},
    outcome::{TransportFailure, TransportSuccess, failure, failure_with_state},
    temp_root::{TestRoot, sha256_file},
};

pub fn run_test_transport(
    root: &TestRoot,
    request: &TestTransportRequest,
    environment: &BTreeMap<String, OsString>,
) -> Result<TransportSuccess, TransportFailure> {
    preflight(root, request, environment).map_err(|detail| failure("preflight_failed", detail))?;
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
                let _ = child.kill();
                break child.wait().map_err(|error| {
                    failure_with_state("reap_failed", error.to_string(), true, None, 0, 0)
                })?;
            }
            Err(error) => {
                let _ = child.kill();
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

fn preflight(
    root: &TestRoot,
    request: &TestTransportRequest,
    environment: &BTreeMap<String, OsString>,
) -> Result<(), String> {
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
    ensure_direct_executable(root.receipt(), &request.preview.executable.relative_path)?;
    if fs::canonicalize(
        root.receipt()
            .join(&request.preview.executable.relative_path),
    )
    .ok()
    .as_deref()
        != fs::canonicalize(root.executable()).ok().as_deref()
    {
        return Err("plan executable does not identify the copied test helper".to_string());
    }
    if sha256_file(root.executable())? != request.preview.executable.sha256 {
        return Err("test helper digest does not match the plan".to_string());
    }
    Ok(())
}

fn ensure_direct_executable(receipt: &Path, relative: &str) -> Result<(), String> {
    let canonical_receipt =
        fs::canonicalize(receipt).map_err(|error| format!("canonicalize receipt: {error}"))?;
    let mut current = receipt.to_path_buf();
    let components = Path::new(relative).components().collect::<Vec<_>>();
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(component) = component else {
            return Err("executable path has a non-normal component".to_string());
        };
        current.push(component);
        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| format!("read executable path metadata: {error}"))?;
        if metadata.file_type().is_symlink()
            || (index + 1 == components.len() && !metadata.is_file())
            || (index + 1 < components.len() && !metadata.is_dir())
        {
            return Err("executable path includes an unsafe filesystem type".to_string());
        }
    }
    let canonical_executable =
        fs::canonicalize(current).map_err(|error| format!("canonicalize executable: {error}"))?;
    if !canonical_executable.starts_with(&canonical_receipt) {
        return Err("executable escaped the receipt root".to_string());
    }
    Ok(())
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
