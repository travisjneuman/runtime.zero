use rz0_module_protocol::test_transport::TestTransportResponse;

#[derive(Debug)]
pub struct TransportSuccess {
    pub response: TestTransportResponse,
    pub exit_code: Option<i32>,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
}

#[derive(Debug)]
pub struct TransportFailure {
    pub code: &'static str,
    pub detail: String,
    pub timed_out: bool,
    pub exit_code: Option<i32>,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
}

pub fn failure(code: &'static str, detail: String) -> TransportFailure {
    failure_with_state(code, detail, false, None, 0, 0)
}

pub fn failure_with_state(
    code: &'static str,
    detail: String,
    timed_out: bool,
    exit_code: Option<i32>,
    stdout_bytes: u64,
    stderr_bytes: u64,
) -> TransportFailure {
    TransportFailure {
        code,
        detail,
        timed_out,
        exit_code,
        stdout_bytes,
        stderr_bytes,
    }
}
