use serde::{Deserialize, Serialize};

pub const MAX_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_SMALL_DOCUMENT_BYTES: u64 = 64 * 1024;
pub const MAX_JOURNAL_SNAPSHOT_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_INVENTORY_PATH_ENTRIES: usize = 512;
pub const MAX_INVENTORY_APP_RECORDS: usize = 4096;
pub const MAX_VERSION_OUTPUT_BYTES: usize = 64 * 1024;
pub const VERSION_PROBE_TIMEOUT_MS: u64 = 2_000;
pub const VERSION_PROBE_READER_GRACE_MS: u64 = 250;

const _: () = assert!(MAX_SMALL_DOCUMENT_BYTES < MAX_JOURNAL_SNAPSHOT_BYTES);
const _: () = assert!(MAX_JOURNAL_SNAPSHOT_BYTES < MAX_ARTIFACT_BYTES);
const _: () = assert!(MAX_INVENTORY_PATH_ENTRIES < MAX_INVENTORY_APP_RECORDS);
const _: () =
    assert!(VERSION_PROBE_TIMEOUT_MS <= ProcessLimitCeilings::MODULE_SCHEMA_ONE.timeout_ms);

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessLimits {
    pub timeout_ms: u64,
    pub stdin_bytes: u64,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessLimitCeilings {
    pub timeout_ms: u64,
    pub stdin_bytes: u64,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
}

impl ProcessLimitCeilings {
    pub const MODULE_SCHEMA_ONE: Self = Self {
        timeout_ms: 10_000,
        stdin_bytes: MAX_SMALL_DOCUMENT_BYTES,
        stdout_bytes: 1024 * 1024,
        stderr_bytes: MAX_SMALL_DOCUMENT_BYTES,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProcessLimitField {
    TimeoutMs,
    StdinBytes,
    StdoutBytes,
    StderrBytes,
}

impl ProcessLimits {
    pub fn violations(&self, ceilings: ProcessLimitCeilings) -> Vec<ProcessLimitField> {
        let mut violations = Vec::with_capacity(4);
        if self.timeout_ms == 0 || self.timeout_ms > ceilings.timeout_ms {
            violations.push(ProcessLimitField::TimeoutMs);
        }
        if self.stdin_bytes == 0 || self.stdin_bytes > ceilings.stdin_bytes {
            violations.push(ProcessLimitField::StdinBytes);
        }
        if self.stdout_bytes == 0 || self.stdout_bytes > ceilings.stdout_bytes {
            violations.push(ProcessLimitField::StdoutBytes);
        }
        if self.stderr_bytes == 0 || self.stderr_bytes > ceilings.stderr_bytes {
            violations.push(ProcessLimitField::StderrBytes);
        }
        violations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_one_limits_accept_only_positive_bounded_values() {
        let ceilings = ProcessLimitCeilings::MODULE_SCHEMA_ONE;
        let valid = ProcessLimits {
            timeout_ms: ceilings.timeout_ms,
            stdin_bytes: ceilings.stdin_bytes,
            stdout_bytes: ceilings.stdout_bytes,
            stderr_bytes: ceilings.stderr_bytes,
        };
        assert!(valid.violations(ceilings).is_empty());

        let invalid = ProcessLimits {
            timeout_ms: 0,
            stdin_bytes: ceilings.stdin_bytes + 1,
            stdout_bytes: 0,
            stderr_bytes: ceilings.stderr_bytes + 1,
        };
        assert_eq!(
            invalid.violations(ceilings),
            [
                ProcessLimitField::TimeoutMs,
                ProcessLimitField::StdinBytes,
                ProcessLimitField::StdoutBytes,
                ProcessLimitField::StderrBytes,
            ]
        );
    }
}
