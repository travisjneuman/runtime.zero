use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};

use rz0_quarantine::{QuarantineRecord, validate_quarantine_record};
use rz0_transaction_contract::{
    DurableJournalErrorCode, RecoveryDecision, TransactionOperation, TransactionState,
    assess_recovery, inspect_journal_head,
};
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
    transaction_root_state: &'static str,
    checked_count: usize,
    valid_count: usize,
    invalid_count: usize,
    restore_available_count: usize,
    checked_transaction_count: usize,
    valid_transaction_count: usize,
    invalid_transaction_count: usize,
    transaction_action_required_count: usize,
    transaction_warning_count: usize,
    warnings: Vec<String>,
    records: Vec<RecoveryRecordSummary>,
    transactions: Vec<RecoveryTransactionSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RecoverySummary {
    pub(crate) quarantine_root_state: &'static str,
    pub(crate) checked_count: usize,
    pub(crate) valid_count: usize,
    pub(crate) invalid_count: usize,
    pub(crate) restore_available_count: usize,
    pub(crate) checked_transaction_count: usize,
    pub(crate) invalid_transaction_count: usize,
    pub(crate) transaction_action_required_count: usize,
    pub(crate) transaction_warning_count: usize,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct RecoveryTransactionSummary {
    transaction_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    plan_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation: Option<TransactionOperation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<TransactionState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recovery_decision: Option<RecoveryDecision>,
    journal_valid: bool,
    action_required: bool,
    operator_guidance: &'static str,
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
            return finish_review(
                store,
                "invalid",
                vec!["quarantine root is a symlink".to_string()],
                Vec::new(),
            );
        }
        Ok(metadata) if !metadata.is_dir() => {
            return finish_review(
                store,
                "invalid",
                vec!["quarantine root is not a directory".to_string()],
                Vec::new(),
            );
        }
        Ok(metadata) if !private_directory(&metadata) => {
            return finish_review(
                store,
                "invalid",
                vec!["quarantine root is not private".to_string()],
                Vec::new(),
            );
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return finish_review(
                store,
                "absent",
                vec!["quarantine root is absent".to_string()],
                Vec::new(),
            );
        }
        Err(_) => {
            return finish_review(
                store,
                "invalid",
                vec!["quarantine root could not be inspected".to_string()],
                Vec::new(),
            );
        }
    }

    let mut entries = match fs::read_dir(&root) {
        Ok(entries) => entries.filter_map(Result::ok).collect::<Vec<_>>(),
        Err(_) => {
            return finish_review(
                store,
                "invalid",
                vec!["quarantine root could not be read".to_string()],
                Vec::new(),
            );
        }
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
    finish_review(store, "present", warnings, records)
}

fn finish_review(
    store: &ModuleStorePlan,
    quarantine_root_state: &'static str,
    mut warnings: Vec<String>,
    records: Vec<RecoveryRecordSummary>,
) -> RecoveryReview {
    let checked_count = records.len();
    let valid_count = records.iter().filter(|record| record.record_valid).count();
    let invalid_count = checked_count.saturating_sub(valid_count);
    let restore_available_count = records
        .iter()
        .filter(|record| record.restore_available)
        .count();
    let (transaction_root_state, transaction_warnings, transactions) =
        inspect_transactions(&PathBuf::from(&store.state_root).join("transactions"));
    let transaction_warning_count = transaction_warnings.len();
    let checked_transaction_count = transactions.len();
    let valid_transaction_count = transactions
        .iter()
        .filter(|transaction| transaction.journal_valid)
        .count();
    let invalid_transaction_count =
        checked_transaction_count.saturating_sub(valid_transaction_count);
    let transaction_action_required_count = transactions
        .iter()
        .filter(|transaction| transaction.action_required)
        .count();
    warnings.extend(transaction_warnings);
    RecoveryReview {
        schema_version: 1,
        contract: RECOVERY_REVIEW_CONTRACT,
        read_only: true,
        writes_attempted: false,
        quarantine_root_state,
        transaction_root_state,
        checked_count,
        valid_count,
        invalid_count,
        restore_available_count,
        checked_transaction_count,
        valid_transaction_count,
        invalid_transaction_count,
        transaction_action_required_count,
        transaction_warning_count,
        warnings,
        records,
        transactions,
    }
}

pub(crate) fn recovery_summary(store: &ModuleStorePlan) -> RecoverySummary {
    let review = build_review(store);
    RecoverySummary {
        quarantine_root_state: review.quarantine_root_state,
        checked_count: review.checked_count,
        valid_count: review.valid_count,
        invalid_count: review.invalid_count,
        restore_available_count: review.restore_available_count,
        checked_transaction_count: review.checked_transaction_count,
        invalid_transaction_count: review.invalid_transaction_count,
        transaction_action_required_count: review.transaction_action_required_count,
        transaction_warning_count: review.transaction_warning_count,
    }
}

fn inspect_transactions(
    root: &Path,
) -> (&'static str, Vec<String>, Vec<RecoveryTransactionSummary>) {
    match fs::symlink_metadata(root) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            return (
                "invalid",
                vec!["transaction root is a symlink".to_string()],
                Vec::new(),
            );
        }
        Ok(metadata) if !metadata.is_dir() => {
            return (
                "invalid",
                vec!["transaction root is not a directory".to_string()],
                Vec::new(),
            );
        }
        Ok(metadata) if !private_directory(&metadata) => {
            return (
                "invalid",
                vec!["transaction root is not private".to_string()],
                Vec::new(),
            );
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return ("absent", Vec::new(), Vec::new());
        }
        Err(_) => {
            return (
                "invalid",
                vec!["transaction root could not be inspected".to_string()],
                Vec::new(),
            );
        }
    }

    let mut entries = match fs::read_dir(root) {
        Ok(entries) => entries.filter_map(Result::ok).collect::<Vec<_>>(),
        Err(_) => {
            return (
                "invalid",
                vec!["transaction root could not be read".to_string()],
                Vec::new(),
            );
        }
    };
    entries.sort_by_key(|entry| entry.file_name());
    let mut warnings = Vec::new();
    if entries.len() > MAX_RECORDS {
        warnings.push(format!(
            "transaction count exceeded the bounded ceiling of {MAX_RECORDS}"
        ));
        entries.truncate(MAX_RECORDS);
    }
    let mut transaction_entries = Vec::new();
    let mut writer_lock_marker_count = 0usize;
    for entry in entries {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            if name.ends_with(".writer.lock") {
                writer_lock_marker_count += 1;
            } else {
                warnings.push("hidden transaction-root entry was ignored".to_string());
            }
            continue;
        }
        match entry.file_type() {
            Ok(file_type) if file_type.is_dir() => transaction_entries.push(entry),
            Ok(_) => warnings.push("non-directory transaction-root entry was ignored".to_string()),
            Err(_) => {
                warnings.push("transaction-root entry type could not be inspected".to_string())
            }
        }
    }
    if writer_lock_marker_count > 0 {
        warnings.push(format!(
            "{writer_lock_marker_count} transaction writer-lock marker(s) present; active writer state is not determined by this read-only review"
        ));
    }
    let transactions = transaction_entries
        .iter()
        .enumerate()
        .map(|(index, entry)| inspect_transaction(root, entry, index))
        .collect();
    ("present", warnings, transactions)
}

fn inspect_transaction(
    root: &Path,
    entry: &fs::DirEntry,
    index: usize,
) -> RecoveryTransactionSummary {
    let name = entry.file_name();
    let Some(transaction_id) = name.to_str() else {
        return invalid_transaction_summary(index, "transaction ID is not portable UTF-8");
    };
    if !rz0_validation_contract::valid_ledger_id(transaction_id, 96) {
        return invalid_transaction_summary(index, "transaction ID is invalid");
    }
    match inspect_journal_head(root, transaction_id) {
        Ok(recovered) => {
            let assessment = assess_recovery(&recovered.journal);
            let action_required = !matches!(assessment.decision, RecoveryDecision::NoAction);
            RecoveryTransactionSummary {
                transaction_id: transaction_id.to_string(),
                plan_id: Some(recovered.journal.plan_id),
                operation: Some(recovered.journal.operation),
                state: Some(recovered.journal.state),
                recovery_decision: Some(assessment.decision),
                journal_valid: true,
                action_required,
                operator_guidance: recovery_guidance(assessment.decision),
                errors: Vec::new(),
            }
        }
        Err(error) => RecoveryTransactionSummary {
            transaction_id: transaction_id.to_string(),
            plan_id: None,
            operation: None,
            state: None,
            recovery_decision: Some(RecoveryDecision::RefuseInvalidJournal),
            journal_valid: false,
            action_required: true,
            operator_guidance: "preserve evidence; refuse automatic recovery",
            errors: vec![journal_error_label(error.code).to_string()],
        },
    }
}

fn invalid_transaction_summary(index: usize, error: &'static str) -> RecoveryTransactionSummary {
    RecoveryTransactionSummary {
        transaction_id: format!("invalid-transaction-{index}"),
        plan_id: None,
        operation: None,
        state: None,
        recovery_decision: Some(RecoveryDecision::RefuseInvalidJournal),
        journal_valid: false,
        action_required: true,
        operator_guidance: "preserve evidence; refuse automatic recovery",
        errors: vec![error.to_string()],
    }
}

fn journal_error_label(code: DurableJournalErrorCode) -> &'static str {
    match code {
        DurableJournalErrorCode::InvalidJournal => "journal_invalid",
        DurableJournalErrorCode::UnsafeRoot => "journal_root_unsafe",
        DurableJournalErrorCode::UnsafeFilesystemType => "journal_filesystem_type_unsafe",
        DurableJournalErrorCode::WriterBusy => "journal_writer_busy_during_review",
        DurableJournalErrorCode::HistoryConflict => "journal_history_conflict",
        DurableJournalErrorCode::SnapshotLimitExceeded => "journal_snapshot_limit_exceeded",
        DurableJournalErrorCode::CorruptSnapshot => "journal_snapshot_corrupt",
        DurableJournalErrorCode::RecoveryRequired => "journal_recovery_required",
        DurableJournalErrorCode::Io => "journal_io_unavailable",
    }
}

fn recovery_guidance(decision: RecoveryDecision) -> &'static str {
    match decision {
        RecoveryDecision::AbortWithoutWrites => "preserve evidence; no writes were recorded",
        RecoveryDecision::RollBackVerifiedWrites => {
            "stop and perform separately approved rollback/recovery review"
        }
        RecoveryDecision::VerifyCommittedState => {
            "verify committed state and receipt before proceeding"
        }
        RecoveryDecision::ResumeRollback => {
            "resume rollback only through an explicit recovery workflow"
        }
        RecoveryDecision::NoAction => "no recovery action indicated",
        RecoveryDecision::RefuseInvalidJournal => "preserve evidence; refuse automatic recovery",
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
            let _ = writeln!(
                output,
                "transaction_root_state: {}",
                review.transaction_root_state
            );
            let _ = writeln!(output, "checked_records: {}", review.checked_count);
            let _ = writeln!(output, "valid_records: {}", review.valid_count);
            let _ = writeln!(output, "invalid_records: {}", review.invalid_count);
            let _ = writeln!(
                output,
                "restore_available: {}",
                review.restore_available_count
            );
            let _ = writeln!(
                output,
                "checked_transactions: {}",
                review.checked_transaction_count
            );
            let _ = writeln!(
                output,
                "valid_transactions: {}",
                review.valid_transaction_count
            );
            let _ = writeln!(
                output,
                "invalid_transactions: {}",
                review.invalid_transaction_count
            );
            let _ = writeln!(
                output,
                "transaction_action_required: {}",
                review.transaction_action_required_count
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
            for transaction in &review.transactions {
                let _ = writeln!(
                    output,
                    "transaction {} · valid {} · state {:?} · decision {:?} · action_required {}",
                    transaction.transaction_id,
                    if transaction.journal_valid {
                        "yes"
                    } else {
                        "no"
                    },
                    transaction.state,
                    transaction.recovery_decision,
                    if transaction.action_required {
                        "yes"
                    } else {
                        "no"
                    },
                );
                let _ = writeln!(output, "  guidance: {}", transaction.operator_guidance);
                for error in &transaction.errors {
                    let _ = writeln!(output, "  error: {error}");
                }
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

    #[test]
    fn recovery_reports_transaction_guidance_without_authorizing_mutation() {
        static SEQUENCE: AtomicU64 = AtomicU64::new(100);
        let root = std::env::temp_dir().join(format!(
            "runtime-zero-recovery-journal-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let transactions = root.join("transactions");
        fs::create_dir_all(&transactions).expect("transaction root");
        #[cfg(unix)]
        fs::set_permissions(&transactions, fs::Permissions::from_mode(0o700))
            .expect("private transaction root");
        let mut journal = rz0_transaction_contract::TransactionJournal {
            schema_version: rz0_transaction_contract::TRANSACTION_SCHEMA_VERSION,
            contract: rz0_transaction_contract::TRANSACTION_CONTRACT.to_string(),
            transaction_id: "rz0tx-recovery-review".to_string(),
            plan_id: "rz0plan-recovery-review".to_string(),
            operation: rz0_transaction_contract::TransactionOperation::Quarantine,
            state: rz0_transaction_contract::TransactionState::Prepared,
            durability: rz0_transaction_contract::DurabilityRequirements::schema_one(),
            events: vec![rz0_transaction_contract::TransactionEvent {
                sequence: 0,
                kind: rz0_transaction_contract::TransactionEventKind::Prepared,
                action_id: None,
                path: None,
                before_sha256: None,
                after_sha256: None,
                previous_event_sha256: String::new(),
                event_sha256: String::new(),
            }],
        };
        rz0_transaction_contract::seal_transaction_journal(&mut journal);
        rz0_transaction_contract::publish_journal_snapshot(&transactions, &journal)
            .expect("publish journal");
        let lock = transactions.join(format!(".{}.writer.lock", journal.transaction_id));
        fs::write(&lock, b"review fixture lock").expect("writer lock fixture");
        #[cfg(unix)]
        fs::set_permissions(&lock, fs::Permissions::from_mode(0o600)).expect("private writer lock");

        let (state, warnings, summaries) = inspect_transactions(&transactions);
        assert_eq!(state, "present");
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("writer-lock"))
        );
        assert_eq!(summaries.len(), 1);
        assert!(summaries[0].journal_valid);
        assert!(summaries[0].action_required);
        assert_eq!(
            summaries[0].recovery_decision,
            Some(rz0_transaction_contract::RecoveryDecision::AbortWithoutWrites)
        );
        assert!(!summaries[0].errors.iter().any(|error| error.contains("/")));
        assert!(lock.exists());
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
