use std::{
    env, fs,
    io::{self, Read, Write},
    path::Path,
    process, thread,
    time::Duration,
};

#[cfg(unix)]
use std::process::{Command, Stdio};

use rz0_module_protocol::test_transport::{
    MAX_TEST_FRAME_BYTES, TEST_HELPER_ID, TEST_TRANSPORT_RESPONSE_CONTRACT,
    TEST_TRANSPORT_SCHEMA_VERSION, TEST_WORK_MARKER_CONTENT, TestChildBehavior,
    TestTransportRequest, TestTransportResponse, TestTransportStatus,
    validate_test_transport_request, validate_test_transport_response,
};

const BURST_BYTES: usize = 32 * 1024;
const FLOOD_BYTES: usize = 256 * 1024;

fn main() {
    if env::var_os("RZ0_PROTOCOL_TEST_DESCENDANT").is_some() {
        thread::sleep(Duration::from_secs(10));
        return;
    }
    if let Err(error) = run() {
        let _ = writeln!(io::stderr().lock(), "test-child-error:{error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut input = Vec::new();
    io::stdin()
        .take(MAX_TEST_FRAME_BYTES + 1)
        .read_to_end(&mut input)
        .map_err(|error| format!("read request: {error}"))?;
    if input.len() as u64 > MAX_TEST_FRAME_BYTES {
        return Err("request frame exceeds test helper limit".to_string());
    }
    let request: TestTransportRequest =
        serde_json::from_slice(&input).map_err(|error| format!("parse request: {error}"))?;
    let validation = validate_test_transport_request(&request);
    if !validation.valid {
        return Err(format!("invalid request: {:?}", validation.errors));
    }
    let response = response_for(&request)?;
    let validation = validate_test_transport_response(&request, &response);
    if !validation.valid {
        return Err(format!("invalid test boundary: {:?}", validation.errors));
    }

    match request.behavior {
        TestChildBehavior::Sleep => {
            thread::sleep(Duration::from_secs(2));
            return Ok(());
        }
        TestChildBehavior::DescendantSleep => {
            spawn_sleeping_descendant()?;
            thread::sleep(Duration::from_secs(2));
            return Ok(());
        }
        TestChildBehavior::StdoutFlood => {
            write_repeated(io::stdout().lock(), b'o', FLOOD_BYTES)?;
            return Ok(());
        }
        TestChildBehavior::StderrFlood => {
            write_repeated(io::stderr().lock(), b'e', FLOOD_BYTES)?;
        }
        TestChildBehavior::StderrBurst => {
            write_repeated(io::stderr().lock(), b'e', BURST_BYTES)?;
        }
        TestChildBehavior::Malformed => {
            io::stdout()
                .write_all(b"not-json\n")
                .map_err(|error| format!("write malformed response: {error}"))?;
            return Ok(());
        }
        TestChildBehavior::ExitFailure => {
            io::stderr()
                .write_all(b"intentional-test-child-failure\n")
                .map_err(|error| format!("write failure: {error}"))?;
            process::exit(7);
        }
        TestChildBehavior::Respond => {}
    }

    let mut output =
        serde_json::to_vec(&response).map_err(|error| format!("serialize response: {error}"))?;
    output.push(b'\n');
    io::stdout()
        .write_all(&output)
        .map_err(|error| format!("write response: {error}"))?;
    Ok(())
}

fn response_for(request: &TestTransportRequest) -> Result<TestTransportResponse, String> {
    let mut observed_environment_names = Vec::new();
    for (name, _) in env::vars_os() {
        observed_environment_names.push(
            name.into_string()
                .map_err(|_| "non-Unicode environment name".to_string())?,
        );
    }
    observed_environment_names.sort();
    let argument_count = env::args_os().count() as u64;
    let marker = Path::new(&request.expected_working_directory_marker);
    let marker_is_direct_file = fs::symlink_metadata(marker)
        .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink());
    let working_directory_marker_present =
        marker_is_direct_file && fs::read(marker).ok().as_deref() == Some(TEST_WORK_MARKER_CONTENT);

    Ok(TestTransportResponse {
        schema_version: TEST_TRANSPORT_SCHEMA_VERSION,
        contract: TEST_TRANSPORT_RESPONSE_CONTRACT.to_string(),
        test_only: true,
        helper_id: TEST_HELPER_ID.to_string(),
        request_id: request.request_id.clone(),
        status: TestTransportStatus::Completed,
        read_only: true,
        writes_attempted: false,
        network_attempted: false,
        observed_environment_names,
        argument_count,
        working_directory_marker_present,
    })
}

#[cfg(unix)]
fn spawn_sleeping_descendant() -> Result<(), String> {
    let executable = env::current_exe().map_err(|error| format!("resolve test helper: {error}"))?;
    Command::new(executable)
        .env_clear()
        .env("RZ0_PROTOCOL_TEST_DESCENDANT", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|error| format!("spawn test descendant: {error}"))?;
    io::stderr()
        .write_all(b"test-descendant-started\n")
        .map_err(|error| format!("write descendant marker: {error}"))?;
    io::stderr()
        .flush()
        .map_err(|error| format!("flush descendant marker: {error}"))
}

#[cfg(not(unix))]
fn spawn_sleeping_descendant() -> Result<(), String> {
    Err("test descendant behavior is unsupported on this platform".to_string())
}

fn write_repeated(mut writer: impl Write, byte: u8, count: usize) -> Result<(), String> {
    let block = vec![byte; 8 * 1024];
    for _ in 0..count / block.len() {
        writer
            .write_all(&block)
            .map_err(|error| format!("write output: {error}"))?;
    }
    writer
        .flush()
        .map_err(|error| format!("flush output: {error}"))
}
