use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

const PROBE_TIMEOUT: Duration =
    Duration::from_millis(rz0_resource_contract::VERSION_PROBE_TIMEOUT_MS);
const READER_GRACE: Duration =
    Duration::from_millis(rz0_resource_contract::VERSION_PROBE_READER_GRACE_MS);
const MAX_CAPTURE_BYTES: usize = rz0_resource_contract::MAX_VERSION_OUTPUT_BYTES;

pub(crate) fn run_version_probe(path: &Path, args: &[&str]) -> Result<String, String> {
    let mut child = Command::new(path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
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

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() < PROBE_TIMEOUT => {
                thread::sleep(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("version probe exceeded the 2 second timeout".to_string());
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("version probe wait failed: {error}"));
            }
        }
    };

    let stdout = receive_output(stdout_reader, "stdout")?;
    let stderr = receive_output(stderr_reader, "stderr")?;
    if !status.success() {
        return Err(format!("version probe exited with status {status}"));
    }
    let selected = if stdout.iter().any(|byte| !byte.is_ascii_whitespace()) {
        stdout
    } else {
        stderr
    };
    let text = String::from_utf8_lossy(&selected);
    let sanitized = sanitize_version(&text);
    if sanitized.is_empty() {
        Err("version probe returned no usable version text".to_string())
    } else {
        Ok(sanitized)
    }
}

fn spawn_reader(reader: impl Read + Send + 'static) -> Receiver<Result<Vec<u8>, String>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(read_bounded(reader));
    });
    receiver
}

fn receive_output(
    receiver: Receiver<Result<Vec<u8>, String>>,
    stream: &str,
) -> Result<Vec<u8>, String> {
    receiver
        .recv_timeout(READER_GRACE)
        .map_err(|_| format!("version probe {stream} did not close after process exit"))?
}

fn read_bounded(mut reader: impl Read) -> Result<Vec<u8>, String> {
    let mut captured = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|error| format!("version output read failed: {error}"))?;
        if read == 0 {
            return Ok(captured);
        }
        let remaining = MAX_CAPTURE_BYTES.saturating_sub(captured.len());
        captured.extend_from_slice(&buffer[..read.min(remaining)]);
    }
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
    fn kills_probe_after_timeout() {
        let started = Instant::now();
        let error = run_version_probe(Path::new("/bin/sleep"), &["3"])
            .expect_err("sleep should exceed timeout");
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
