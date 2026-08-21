use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};

use rz0_quarantine::{QuarantineRecord, validate_quarantine_record};
use serde::Serialize;

use crate::{
    ExitCode, brand,
    module_store::{ModuleStorePlan, module_store_plan},
};

const RECOVERY_REVIEW_CONTRACT: &str = "recovery_review";
const MAX_RECORDS: usize = 256;
const MAX_RECORD_BYTES: u64 = rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RecoveryReview {
    schema_version: u16,
    contract: &'static str,
    read_only: bool,
    writes_attempted: bool,
    quarantine_root_state: &'static str,
    checked_count: usize,
    valid_count: usize,
    invalid_count: usize,
    restore_available_count: usize,
    warnings: Vec<String>,
    records: Vec<RecoveryRecordSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RecoveryRecordSummary {
    plan_id: String,
    action_id: Option<String>,
    original_path: Option<String>,
    sha256: Option<String>,
    size_bytes: Option<u64>,
    record_valid: bool,
    payload_present: bool,
    restore_available: bool,
    errors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Json,
}

pub fn recovery_command(args: &[String]) -> (ExitCode, String, String) {
    if matches!(args, [help] if matches!(help.as_str(), "--help" | "-h" | "help")) {
        return (ExitCode::Ok, usage(), String::new());
    }
    let format = match parse_args(args) {
        Ok(format) => format,
        Err(error) => {
            return (
                ExitCode::Usage,
                String::new(),
                format!("{error}\n\n{}", usage()),
            );
        }
    };
    let store = module_store_plan(None, None, "recovery review");
    let review = build_review(&store);
    render_review(&review, format)
}

fn parse_args(args: &[String]) -> Result<OutputFormat, String> {
    let mut dry_run = false;
    let mut format = OutputFormat::Text;
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--dry-run" if !dry_run => dry_run = true,
            "--dry-run" => return Err("recovery --dry-run was provided more than once".to_string()),
            "--format" => {
                let Some(value) = args.get(index + 1).map(String::as_str) else {
                    return Err("recovery --format requires text or json".to_string());
                };
                format = match value {
                    "text" => OutputFormat::Text,
                    "json" => OutputFormat::Json,
                    _ => return Err("recovery --format requires text or json".to_string()),
                };
                index += 1;
            }
            "--json" => format = OutputFormat::Json,
            value => return Err(format!("unsupported recovery option '{value}'")),
        }
        index += 1;
    }
    if !dry_run {
        return Err("recovery is read-only and requires --dry-run".to_string());
    }
    Ok(format)
}

fn build_review(store: &ModuleStorePlan) -> RecoveryReview {
    let root = PathBuf::from(&store.quarantine_root);
    match fs::symlink_metadata(&root) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return invalid_root_review("quarantine root is a symlink");
        }
        Ok(metadata) if !metadata.is_dir() => {
            return invalid_root_review("quarantine root is not a directory");
        }
        Ok(metadata) if !private_directory(&metadata) => {
            return invalid_root_review("quarantine root is not private");
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return empty_review("absent", vec!["quarantine root is absent".to_string()]);
        }
        Err(_) => return invalid_root_review("quarantine root could not be inspected"),
    }

    let mut entries = match fs::read_dir(&root) {
        Ok(entries) => entries.filter_map(Result::ok).collect::<Vec<_>>(),
        Err(_) => return invalid_root_review("quarantine root could not be read"),
    };
    entries.sort_by_key(|entry| entry.file_name());
    let mut warnings = Vec::new();
    if entries.len() > MAX_RECORDS {
        warnings.push(format!(
            "quarantine record count exceeded the bounded ceiling of {MAX_RECORDS}"
        ));
        entries.truncate(MAX_RECORDS);
    }
    let records = entries
        .iter()
        .map(|entry| inspect_entry(&root, entry))
        .collect::<Vec<_>>();
    let checked_count = records.len();
    let valid_count = records.iter().filter(|record| record.record_valid).count();
    let invalid_count = checked_count.saturating_sub(valid_count);
    let restore_available_count = records
        .iter()
        .filter(|record| record.restore_available)
        .count();
    RecoveryReview {
        schema_version: 1,
        contract: RECOVERY_REVIEW_CONTRACT,
        read_only: true,
        writes_attempted: false,
        quarantine_root_state: "present",
        checked_count,
        valid_count,
        invalid_count,
        restore_available_count,
        warnings,
        records,
    }
}

fn inspect_entry(root: &Path, entry: &fs::DirEntry) -> RecoveryRecordSummary {
    let plan_id = entry.file_name().to_string_lossy().into_owned();
    let mut summary = RecoveryRecordSummary {
        plan_id: plan_id.clone(),
        action_id: None,
        original_path: None,
        sha256: None,
        size_bytes: None,
        record_valid: false,
        payload_present: false,
        restore_available: false,
        errors: Vec::new(),
    };
    let directory = root.join(&plan_id);
    let metadata = match fs::symlink_metadata(&directory) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            summary
                .errors
                .push("record directory is a symlink".to_string());
            return summary;
        }
        Ok(metadata) if !metadata.is_dir() => {
            summary
                .errors
                .push("record entry is not a directory".to_string());
            return summary;
        }
        Ok(metadata) => metadata,
        Err(_) => {
            summary
                .errors
                .push("record directory could not be inspected".to_string());
            return summary;
        }
    };
    if !private_directory(&metadata) {
        summary
            .errors
            .push("record directory is not private".to_string());
    }
    let record_path = directory.join("quarantine.json");
    let record_bytes = match read_private_record(&record_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            summary.errors.push(error.to_string());
            return summary;
        }
    };
    let record = match serde_json::from_slice::<QuarantineRecord>(&record_bytes) {
        Ok(record) => record,
        Err(_) => {
            summary
                .errors
                .push("record JSON is malformed or unsupported".to_string());
            return summary;
        }
    };
    summary.action_id = Some(record.action_id.clone());
    summary.original_path = Some(record.original_path.clone());
    summary.sha256 = Some(record.sha256.clone());
    summary.size_bytes = Some(record.size_bytes);
    let expected_payload = format!("quarantine/{plan_id}/payload.bin");
    summary.record_valid = validate_quarantine_record(&record)
        && record.plan_id == plan_id
        && record.quarantine_path == expected_payload;
    if !summary.record_valid {
        summary
            .errors
            .push("record binding or logical path is invalid".to_string());
    }
    let payload_path = directory.join("payload.bin");
    summary.payload_present = fs::symlink_metadata(&payload_path).is_ok_and(|metadata| {
        !metadata.file_type().is_symlink()
            && metadata.is_file()
            && metadata.len() == record.size_bytes
    });
    if !summary.payload_present {
        summary
            .errors
            .push("verified quarantine payload is absent or invalid".to_string());
    }
    summary.restore_available = summary.record_valid
        && summary.payload_present
        && supported_restore_path(&record.original_path);
    if summary.record_valid && !supported_restore_path(&record.original_path) {
        summary
            .errors
            .push("record namespace is not supported by exact restore".to_string());
    }
    summary
}

fn read_private_record(path: &Path) -> Result<Vec<u8>, &'static str> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "record file is absent")?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("record file is not a regular file");
    }
    if !private_file(&metadata) {
        return Err("record file is not private");
    }
    if metadata.len() > MAX_RECORD_BYTES {
        return Err("record file exceeds the bounded size limit");
    }
    fs::read(path).map_err(|_| "record file could not be read")
}

fn supported_restore_path(path: &str) -> bool {
    path.starts_with("workspace/cache/") || path.starts_with("workspace/modules/")
}

#[cfg(unix)]
fn private_directory(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o077 == 0
}

#[cfg(not(unix))]
fn private_directory(_metadata: &fs::Metadata) -> bool {
    true
}

#[cfg(unix)]
fn private_file(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode() & 0o077 == 0
}

#[cfg(not(unix))]
fn private_file(_metadata: &fs::Metadata) -> bool {
    true
}

fn empty_review(state: &'static str, warnings: Vec<String>) -> RecoveryReview {
    RecoveryReview {
        schema_version: 1,
        contract: RECOVERY_REVIEW_CONTRACT,
        read_only: true,
        writes_attempted: false,
        quarantine_root_state: state,
        checked_count: 0,
        valid_count: 0,
        invalid_count: 0,
        restore_available_count: 0,
        warnings,
        records: Vec::new(),
    }
}

fn invalid_root_review(detail: &'static str) -> RecoveryReview {
    empty_review("invalid", vec![detail.to_string()])
}

fn render_review(review: &RecoveryReview, format: OutputFormat) -> (ExitCode, String, String) {
    match format {
        OutputFormat::Text => {
            let mut output = format!("{} recovery review\n\n", brand::TITLE);
            let _ = writeln!(output, "mode: dry-run read-only");
            let _ = writeln!(output, "contract: {}", review.contract);
            let _ = writeln!(
                output,
                "quarantine_root_state: {}",
                review.quarantine_root_state
            );
            let _ = writeln!(output, "checked_records: {}", review.checked_count);
            let _ = writeln!(output, "valid_records: {}", review.valid_count);
            let _ = writeln!(output, "invalid_records: {}", review.invalid_count);
            let _ = writeln!(
                output,
                "restore_available: {}",
                review.restore_available_count
            );
            let _ = writeln!(output, "writes_attempted: no");
            if !review.warnings.is_empty() {
                output.push_str("warnings:\n");
                for warning in &review.warnings {
                    let _ = writeln!(output, "  - {warning}");
                }
            }
            for record in &review.records {
                let _ = writeln!(
                    output,
                    "record {} · valid {} · payload {} · restore {}",
                    record.plan_id,
                    if record.record_valid { "yes" } else { "no" },
                    if record.payload_present {
                        "present"
                    } else {
                        "missing"
                    },
                    if record.restore_available {
                        "available"
                    } else {
                        "blocked"
                    },
                );
            }
            (ExitCode::Ok, output, String::new())
        }
        OutputFormat::Json => match serde_json::to_string_pretty(review) {
            Ok(json) => (ExitCode::Ok, format!("{json}\n"), String::new()),
            Err(error) => (
                ExitCode::Usage,
                String::new(),
                format!("recovery review JSON rendering failed: {error}\n"),
            ),
        },
    }
}

fn usage() -> String {
    "Usage: rz0 recovery --dry-run [--format text|json]\n\nReviews bounded runtime.zero quarantine records without writing, deleting, restoring, or exposing absolute host paths. Use rz0 restore for one exact validated record.\n".to_string()
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    use sha2::{Digest, Sha256};

    use super::*;

    #[test]
    fn recovery_requires_dry_run_and_is_read_only() {
        let (code, out, error) = recovery_command(&[]);
        assert_eq!(code, ExitCode::Usage);
        assert!(out.is_empty());
        assert!(error.contains("requires --dry-run"));
    }

    #[test]
    fn absent_quarantine_root_is_a_private_empty_review() {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "runtime-zero-recovery-absent-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let store = crate::module_store::module_store_plan_for_data_root(
            root.clone(),
            None,
            None,
            "recovery absent test",
        );
        let review = build_review(&store);
        assert!(review.read_only);
        assert!(!review.writes_attempted);
        assert_eq!(review.quarantine_root_state, "absent");
        assert_eq!(review.checked_count, 0);
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(unix)]
    #[test]
    fn valid_record_reports_restore_availability_without_absolute_paths() {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "runtime-zero-recovery-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let plan_id = "rz0plan-cache-review-fixture";
        let directory = root.join("quarantine").join(plan_id);
        fs::create_dir_all(&directory).expect("recovery fixture directories");
        for path in [root.join("quarantine"), directory.clone()] {
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))
                .expect("private recovery directory");
        }
        let payload = b"recovery payload\n";
        fs::write(directory.join("payload.bin"), payload).expect("payload");
        fs::set_permissions(
            directory.join("payload.bin"),
            fs::Permissions::from_mode(0o600),
        )
        .expect("private payload");
        let mut record = QuarantineRecord {
            schema_version: 1,
            contract: "quarantine_record".to_string(),
            transaction_id: "tx.quarantine.recovery-fixture.1000".to_string(),
            plan_id: plan_id.to_string(),
            action_id: "quarantine-cache-fixture".to_string(),
            original_path: "workspace/cache/first-party-cache/recovery-entry".to_string(),
            quarantine_path: format!("quarantine/{plan_id}/payload.bin"),
            sha256: format!("{:x}", Sha256::digest(payload)),
            size_bytes: payload.len() as u64,
            created_unix_seconds: 900,
            binding_sha256: String::new(),
        };
        rz0_quarantine::seal_quarantine_record(&mut record);
        let record_path = directory.join("quarantine.json");
        fs::write(&record_path, serde_json::to_vec(&record).expect("record")).expect("record");
        fs::set_permissions(&record_path, fs::Permissions::from_mode(0o600))
            .expect("private record");
        let mut store = crate::module_store::module_store_plan_for_data_root(
            root.clone(),
            None,
            None,
            "recovery fixture",
        );
        store.quarantine_root = root.join("quarantine").display().to_string();
        let review = build_review(&store);
        assert_eq!(review.valid_count, 1);
        assert_eq!(review.restore_available_count, 1);
        assert_eq!(
            review.records[0].original_path.as_deref(),
            Some(record.original_path.as_str())
        );
        assert!(
            !serde_json::to_string(&review)
                .expect("review JSON")
                .contains(&root.display().to_string())
        );
        let _ = fs::remove_dir_all(root);
    }
}
