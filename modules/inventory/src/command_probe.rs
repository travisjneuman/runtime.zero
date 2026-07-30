use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use rz0_cancellation_contract::{ProcessDeadline, cancellation_pair};
use rz0_process_host::{
    BoundedCapture, audit_inheritable_process_handles, configure_child_process_group,
    drain_bounded, terminate_child_process_group,
};

const PROBE_TIMEOUT: Duration =
    Duration::from_millis(rz0_resource_contract::VERSION_PROBE_TIMEOUT_MS);
const READER_GRACE: Duration =
    Duration::from_millis(rz0_resource_contract::VERSION_PROBE_READER_GRACE_MS);
const MAX_CAPTURE_BYTES: usize = rz0_resource_contract::MAX_VERSION_OUTPUT_BYTES;

pub(crate) fn run_version_probe(path: &Path, args: &[&str]) -> Result<String, String> {
    audit_inheritable_process_handles()
        .map_err(|error| format!("version probe handle audit failed: {error}"))?;
    let mut command = Command::new(path);
    command
        .args(args)
        .env_clear()
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    configure_child_process_group(&mut command)
        .map_err(|error| format!("version probe containment is unavailable: {error}"))?;
    let mut child = command
        .spawn()
        .map_err(|error| format!("version probe could not start: {error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "version probe stdout was unavailable".to_string())?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| "version probe stderr was unavailable".to_string())?;
    let stdout_reader = spawn_reader(stdout);
    let stderr_reader = spawn_reader(stderr);
    let started = Instant::now();
    let (_, cancellation) = cancellation_pair();
    let deadline = ProcessDeadline::new(
        0,
        PROBE_TIMEOUT.as_millis() as u64,
        rz0_resource_contract::ProcessLimitCeilings::MODULE_SCHEMA_ONE.timeout_ms,
    )
    .map_err(|_| "version probe deadline is outside the foundation ceiling".to_string())?;

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                let elapsed_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
                if cancellation.poll(elapsed_ms, deadline).is_some() {
                    terminate_and_reap(&mut child);
                    drain_after_termination(stdout_reader, stderr_reader);
                    return Err("version probe exceeded the 2 second timeout".to_string());
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err(error) => {
                terminate_and_reap(&mut child);
                drain_after_termination(stdout_reader, stderr_reader);
                return Err(format!("version probe wait failed: {error}"));
            }
        }
    };

    let stdout = receive_output(stdout_reader, "stdout")?;
    let stderr = receive_output(stderr_reader, "stderr")?;
    if stdout.truncated || stderr.truncated {
        return Err("version probe output exceeded the 64 KiB stream ceiling".to_string());
    }
    if !status.success() {
        return Err(format!("version probe exited with status {status}"));
    }
    let selected = if stdout.bytes.iter().any(|byte| !byte.is_ascii_whitespace()) {
        stdout.bytes
    } else {
        stderr.bytes
    };
    let text = String::from_utf8_lossy(&selected);
    let sanitized = sanitize_version(&text);
    if sanitized.is_empty() {
        Err("version probe returned no usable version text".to_string())
    } else {
        Ok(sanitized)
    }
}

fn spawn_reader(reader: impl Read + Send + 'static) -> Receiver<Result<BoundedCapture, String>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = drain_bounded(reader, MAX_CAPTURE_BYTES as u64)
            .map_err(|error| format!("version output read failed: {error}"));
        let _ = sender.send(result);
    });
    receiver
}

fn receive_output(
    receiver: Receiver<Result<BoundedCapture, String>>,
    stream: &str,
) -> Result<BoundedCapture, String> {
    receiver
        .recv_timeout(READER_GRACE)
        .map_err(|_| format!("version probe {stream} did not close after process exit"))?
}

fn terminate_and_reap(child: &mut std::process::Child) {
    let _ = terminate_child_process_group(child);
    let _ = child.wait();
}

fn drain_after_termination(
    stdout: Receiver<Result<BoundedCapture, String>>,
    stderr: Receiver<Result<BoundedCapture, String>>,
) {
    let _ = stdout.recv_timeout(READER_GRACE);
    let _ = stderr.recv_timeout(READER_GRACE);
}

fn sanitize_version(value: &str) -> String {
    value
        .lines()
        .next()
        .unwrap_or_default()
        .chars()
        .filter(|character| !character.is_control())
        .take(160)
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn kills_probe_group_after_timeout() {
        let started = Instant::now();
        let error = run_version_probe(Path::new("/bin/sh"), &["-c", "sleep 30 & wait"])
            .expect_err("sleeping process group should exceed timeout");
        assert!(error.contains("2 second timeout"));
        assert!(started.elapsed() < Duration::from_secs(3));
    }

    #[test]
    fn sanitizes_version_to_one_bounded_line() {
        assert_eq!(sanitize_version("tool 1.2.3\nignored"), "tool 1.2.3");
        assert_eq!(sanitize_version("bad\u{0}value"), "badvalue");
        assert!(sanitize_version(&"x".repeat(300)).len() <= 160);
    }
}
