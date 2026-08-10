use serde::{Deserialize, Serialize};

pub const MAX_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_SMALL_DOCUMENT_BYTES: u64 = 64 * 1024;
pub const MAX_REGISTRY_DOCUMENT_BYTES: u64 = 128 * 1024;
pub const MAX_JOURNAL_SNAPSHOT_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_FINDING_REPORT_BYTES: u64 = 4 * 1024 * 1024;
pub const MAX_INSTALLED_MODULE_RECORDS: usize = 1024;
pub const MAX_FINDING_SOURCES: usize = 64;
pub const MAX_FINDINGS: usize = 4096;
pub const MAX_FINDING_SOURCE_REFERENCES: usize = 16;
pub const MAX_INVENTORY_REPORT_BYTES: u64 = 16 * 1024 * 1024;
pub const MAX_INVENTORY_SOURCES: usize = 64;
pub const MAX_INVENTORY_PATH_ENTRIES: usize = 512;
pub const MAX_INVENTORY_TOOL_RECORDS: usize = 1024;
pub const MAX_INVENTORY_APP_RECORDS: usize = 4096;
pub const MAX_INVENTORY_SERVICE_RECORDS: usize = 4096;
pub const MAX_INVENTORY_EVENTS: usize = 8192;
pub const MAX_INVENTORY_WARNINGS: usize = 8192;
pub const MAX_VERSION_OUTPUT_BYTES: usize = 64 * 1024;
pub const MAX_REDACTION_TOKENS: usize = 9_999;
pub const MAX_DIAGNOSTIC_CHECKS: usize = 128;
pub const MAX_PERFORMANCE_SAMPLES: u32 = 100;
pub const MAX_PERFORMANCE_OPERATIONS: usize = 16;
pub const VERSION_PROBE_TIMEOUT_MS: u64 = 2_000;
pub const VERSION_PROBE_READER_GRACE_MS: u64 = 250;
pub const MAX_PROCESS_ARGUMENTS: usize = 64;
pub const MAX_PROCESS_ARGUMENT_BYTES: usize = 512;
pub const MAX_PROCESS_CAPTURE_BYTES: u64 = MAX_FINDING_REPORT_BYTES;
pub const MAX_MANAGER_PROCESS_TIMEOUT_MS: u64 = 30 * 60 * 1_000;

const _: () = assert!(MAX_SMALL_DOCUMENT_BYTES < MAX_REGISTRY_DOCUMENT_BYTES);
const _: () = assert!(MAX_REGISTRY_DOCUMENT_BYTES < MAX_JOURNAL_SNAPSHOT_BYTES);
const _: () = assert!(MAX_JOURNAL_SNAPSHOT_BYTES < MAX_FINDING_REPORT_BYTES);
const _: () = assert!(MAX_FINDING_REPORT_BYTES < MAX_INVENTORY_REPORT_BYTES);
const _: () = assert!(MAX_INVENTORY_REPORT_BYTES < MAX_ARTIFACT_BYTES);
const _: () = assert!(MAX_FINDING_SOURCE_REFERENCES < MAX_FINDING_SOURCES);
const _: () = assert!(MAX_FINDING_SOURCES < MAX_INVENTORY_PATH_ENTRIES);
const _: () = assert!(MAX_INVENTORY_PATH_ENTRIES < MAX_INVENTORY_TOOL_RECORDS);
const _: () = assert!(MAX_INSTALLED_MODULE_RECORDS <= MAX_INVENTORY_TOOL_RECORDS);
const _: () = assert!(MAX_INVENTORY_TOOL_RECORDS < MAX_INVENTORY_APP_RECORDS);
const _: () = assert!(MAX_FINDINGS <= MAX_INVENTORY_APP_RECORDS);
const _: () = assert!(MAX_INVENTORY_SERVICE_RECORDS == MAX_INVENTORY_APP_RECORDS);
const _: () = assert!(MAX_INVENTORY_APP_RECORDS < MAX_INVENTORY_EVENTS);
const _: () = assert!(MAX_INVENTORY_EVENTS <= MAX_INVENTORY_WARNINGS);
const _: () = assert!(MAX_PERFORMANCE_OPERATIONS < MAX_DIAGNOSTIC_CHECKS);
const _: () = assert!(MAX_PERFORMANCE_SAMPLES < MAX_DIAGNOSTIC_CHECKS as u32);
const _: () = assert!(MAX_DIAGNOSTIC_CHECKS < MAX_INVENTORY_PATH_ENTRIES);
const _: () = assert!(MAX_REDACTION_TOKENS <= 9_999);
const _: () = assert!(
    MAX_INVENTORY_PATH_ENTRIES
        + MAX_INVENTORY_TOOL_RECORDS
        + MAX_INVENTORY_APP_RECORDS
        + MAX_INVENTORY_SERVICE_RECORDS
        < MAX_REDACTION_TOKENS
);
const _: () =
    assert!(VERSION_PROBE_TIMEOUT_MS <= ProcessLimitCeilings::MODULE_SCHEMA_ONE.timeout_ms);
const _: () =
    assert!(ProcessLimitCeilings::MODULE_SCHEMA_ONE.stdout_bytes <= MAX_PROCESS_CAPTURE_BYTES);
const _: () =
    assert!(ProcessLimitCeilings::MODULE_SCHEMA_ONE.timeout_ms <= MAX_MANAGER_PROCESS_TIMEOUT_MS);

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
