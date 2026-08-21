use std::path::Path;
use std::time::Duration;

use rz0_cancellation_contract::CancellationToken;
use rz0_process_host::{
    ReadOnlyProcessRequest, run_read_only_process, run_read_only_process_cancellable,
};

const PROBE_TIMEOUT: Duration =
    Duration::from_millis(rz0_resource_contract::VERSION_PROBE_TIMEOUT_MS);
const MAX_CAPTURE_BYTES: usize = rz0_resource_contract::MAX_VERSION_OUTPUT_BYTES;

#[cfg(test)]
pub(crate) fn run_version_probe(path: &Path, args: &[&str]) -> Result<String, String> {
    run_version_probe_cancellable(path, args, None)
}

pub(crate) fn run_version_probe_cancellable(
    path: &Path,
    args: &[&str],
    cancellation: Option<&CancellationToken>,
) -> Result<String, String> {
    let request = ReadOnlyProcessRequest {
        executable: path.to_path_buf(),
        arguments: args
            .iter()
            .map(|argument| (*argument).to_string())
            .collect(),
        working_directory: Path::new("/").to_path_buf(),
        environment: Vec::new(),
        timeout: PROBE_TIMEOUT,
        output_limit: MAX_CAPTURE_BYTES as u64,
    };
    let output = match cancellation {
        Some(cancellation) => run_read_only_process_cancellable(&request, cancellation),
        None => run_read_only_process(&request),
    }
    .map_err(|error| format!("version probe process host failed: {error}"))?;
    let rz0_process_host::ProcessOutput {
        status,
        stdout,
        stderr,
        timed_out,
        cancellation_reason,
    } = output;
    if timed_out {
        return Err("version probe exceeded the 2 second timeout".to_string());
    }
    if let Some(reason) = cancellation_reason {
        return Err(format!("version probe was cancelled: {reason:?}"));
    }
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
        let started = std::time::Instant::now();
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
