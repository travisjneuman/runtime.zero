use std::{
    fmt,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
#[cfg(not(any(unix, windows)))]
use std::{fs::OpenOptions, io::Write};

use rz0_resource_contract::MAX_JOURNAL_SNAPSHOT_BYTES;

use crate::{MAX_EVENTS, TransactionJournal, validate_transaction_journal};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const HEADS_DIRECTORY: &str = "heads";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurableJournalErrorCode {
    InvalidJournal,
    UnsafeRoot,
    UnsafeFilesystemType,
    WriterBusy,
    HistoryConflict,
    SnapshotLimitExceeded,
    CorruptSnapshot,
    RecoveryRequired,
    Io,
}

#[derive(Debug)]
pub struct DurableJournalError {
    pub code: DurableJournalErrorCode,
    detail: String,
}

impl DurableJournalError {
    fn new(code: DurableJournalErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }

    pub const fn foundation_code(&self) -> rz0_error_contract::FoundationErrorCode {
        use rz0_error_contract::FoundationErrorCode as Foundation;

        match self.code {
            DurableJournalErrorCode::InvalidJournal | DurableJournalErrorCode::CorruptSnapshot => {
                Foundation::TransactionInvalid
            }
            DurableJournalErrorCode::UnsafeRoot => Foundation::PermissionDenied,
            DurableJournalErrorCode::UnsafeFilesystemType => Foundation::ArtifactIdentityChanged,
            DurableJournalErrorCode::WriterBusy | DurableJournalErrorCode::HistoryConflict => {
                Foundation::Conflict
            }
            DurableJournalErrorCode::SnapshotLimitExceeded => Foundation::InputLimitExceeded,
            DurableJournalErrorCode::RecoveryRequired => Foundation::RecoveryRequired,
            DurableJournalErrorCode::Io => Foundation::IoUnavailable,
        }
    }
}

impl fmt::Display for DurableJournalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for DurableJournalError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotPublicationStatus {
    Published,
    AlreadyPublished,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotPublication {
    pub status: SnapshotPublicationStatus,
    pub snapshot_name: String,
    pub sequence: u32,
    pub event_sha256: String,
    pub snapshot_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredJournalHead {
    pub journal: TransactionJournal,
    pub snapshot_count: usize,
    pub snapshot_name: String,
}

/// Publishes one immutable journal snapshot under an exclusive cross-process
/// writer lock. Existing history must be an exact prefix of the new journal.
pub fn publish_journal_snapshot(
    transaction_root: &Path,
    journal: &TransactionJournal,
) -> Result<SnapshotPublication, DurableJournalError> {
    validate_for_persistence(journal)?;
    validate_root(transaction_root)?;
    let _writer = WriterLock::acquire(transaction_root, &journal.transaction_id)?;
    let transaction_directory = ensure_transaction_directory(transaction_root, journal)?;
    let heads = ensure_direct_directory(&transaction_directory, HEADS_DIRECTORY)?;
    let prior = recover_locked(&heads, Some(journal))?;
    let head = journal.events.last().ok_or_else(|| {
        DurableJournalError::new(
            DurableJournalErrorCode::InvalidJournal,
            "journal has no head",
        )
    })?;
    let snapshot_name = snapshot_name(head.sequence, &head.event_sha256);
    let bytes = serde_json::to_vec(journal).map_err(|error| {
        DurableJournalError::new(
            DurableJournalErrorCode::InvalidJournal,
            format!("serialize journal: {error}"),
        )
    })?;
    if bytes.len() as u64 > MAX_JOURNAL_SNAPSHOT_BYTES {
        return Err(DurableJournalError::new(
            DurableJournalErrorCode::SnapshotLimitExceeded,
            "journal snapshot exceeds the shared byte ceiling",
        ));
    }

    if let Some(prior) = prior {
        if prior.journal == *journal {
            return Ok(publication(
                SnapshotPublicationStatus::AlreadyPublished,
                snapshot_name,
                head.sequence,
                &head.event_sha256,
                bytes.len(),
            ));
        }
        validate_append_only_successor(&prior.journal, journal)?;
    } else if journal.events.len() != 1 {
        return Err(DurableJournalError::new(
            DurableJournalErrorCode::HistoryConflict,
            "the first durable snapshot must contain only the prepared event",
        ));
    }

    let temporary = transaction_directory.join(format!(
        ".pending-{}-{}",
        std::process::id(),
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    write_new_synced_file(&temporary, &bytes)?;
    let final_path = heads.join(&snapshot_name);
    publish_pending_file(&temporary, &heads, &snapshot_name)?;
    sync_directory(&heads)?;
    verify_direct_regular_file(&final_path)?;

    Ok(publication(
        SnapshotPublicationStatus::Published,
        snapshot_name,
        head.sequence,
        &head.event_sha256,
        bytes.len(),
    ))
}

/// Recovers and validates the complete immutable prefix for one transaction.
/// A writer lock prevents observing an in-flight publication.
pub fn recover_journal_head(
    transaction_root: &Path,
    transaction_id: &str,
) -> Result<RecoveredJournalHead, DurableJournalError> {
    if !rz0_validation_contract::valid_ledger_id(transaction_id, 96) {
        return Err(DurableJournalError::new(
            DurableJournalErrorCode::UnsafeRoot,
            "transaction ID is invalid",
        ));
    }
    validate_root(transaction_root)?;
    let _writer = WriterLock::acquire(transaction_root, transaction_id)?;
    let transaction_directory = transaction_root.join(transaction_id);
    verify_direct_directory(&transaction_directory)?;
    let heads = transaction_directory.join(HEADS_DIRECTORY);
    verify_direct_directory(&heads)?;
    recover_locked(&heads, None)?.ok_or_else(|| {
        DurableJournalError::new(
            DurableJournalErrorCode::CorruptSnapshot,
            "transaction has no journal snapshots",
        )
    })
}

/// Inspects one immutable journal head without acquiring the writer lock or
/// creating any lock file. Callers must treat a concurrent publication as
/// incomplete evidence and keep the result report-only; this API never
/// authorizes recovery or mutation.
pub fn inspect_journal_head(
    transaction_root: &Path,
    transaction_id: &str,
) -> Result<RecoveredJournalHead, DurableJournalError> {
    if !rz0_validation_contract::valid_ledger_id(transaction_id, 96) {
        return Err(DurableJournalError::new(
            DurableJournalErrorCode::UnsafeRoot,
            "transaction ID is invalid",
        ));
    }
    validate_root(transaction_root)?;
    let transaction_directory = transaction_root.join(transaction_id);
    verify_direct_directory(&transaction_directory)?;
    let heads = transaction_directory.join(HEADS_DIRECTORY);
    verify_direct_directory(&heads)?;
    recover_locked(&heads, None)?.ok_or_else(|| {
        DurableJournalError::new(
            DurableJournalErrorCode::CorruptSnapshot,
            "transaction has no journal snapshots",
        )
    })
}

fn validate_for_persistence(journal: &TransactionJournal) -> Result<(), DurableJournalError> {
    let validation = validate_transaction_journal(journal);
    if validation.valid {
        Ok(())
    } else {
        Err(DurableJournalError::new(
            DurableJournalErrorCode::InvalidJournal,
            format!("journal validation failed: {:?}", validation.errors),
        ))
    }
}

fn validate_root(root: &Path) -> Result<(), DurableJournalError> {
    verify_direct_directory(root).map_err(|error| {
        DurableJournalError::new(
            DurableJournalErrorCode::UnsafeRoot,
            format!("transaction root is unsafe: {error}"),
        )
    })
}

fn ensure_transaction_directory(
    root: &Path,
    journal: &TransactionJournal,
) -> Result<PathBuf, DurableJournalError> {
    let path = root.join(&journal.transaction_id);
    match fs::symlink_metadata(&path) {
        Ok(_) => verify_direct_directory(&path)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            create_private_directory(&path)
                .map_err(|error| io_error("create transaction journal directory", error))?;
            verify_direct_directory(&path)?;
            sync_directory(root)?;
        }
        Err(error) => return Err(io_error("inspect transaction directory", error)),
    }
    Ok(path)
}

fn ensure_direct_directory(parent: &Path, name: &str) -> Result<PathBuf, DurableJournalError> {
    let path = parent.join(name);
    match fs::symlink_metadata(&path) {
        Ok(_) => verify_direct_directory(&path)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            create_private_directory(&path)
                .map_err(|error| io_error("create journal heads directory", error))?;
            verify_direct_directory(&path)?;
            sync_directory(parent)?;
        }
        Err(error) => return Err(io_error("inspect journal heads directory", error)),
    }
    Ok(path)
}

fn recover_locked(
    heads: &Path,
    expected: Option<&TransactionJournal>,
) -> Result<Option<RecoveredJournalHead>, DurableJournalError> {
    let mut paths = fs::read_dir(heads)
        .map_err(|error| io_error("read journal heads", error))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| io_error("read journal head entry", error))?;
    paths.sort();
    let snapshot_count = paths.len();
    if snapshot_count > MAX_EVENTS {
        return Err(DurableJournalError::new(
            DurableJournalErrorCode::SnapshotLimitExceeded,
            "journal snapshot count exceeds the event ceiling",
        ));
    }

    let mut prior: Option<TransactionJournal> = None;
    let mut prior_name = String::new();
    for path in paths {
        let (file, metadata) = open_direct_snapshot(&path)?;
        if metadata.len() > MAX_JOURNAL_SNAPSHOT_BYTES {
            return Err(DurableJournalError::new(
                DurableJournalErrorCode::SnapshotLimitExceeded,
                "journal snapshot exceeds the shared byte ceiling",
            ));
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        file.take(MAX_JOURNAL_SNAPSHOT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| io_error("read journal snapshot", error))?;
        if bytes.len() as u64 > MAX_JOURNAL_SNAPSHOT_BYTES || bytes.len() as u64 != metadata.len() {
            return Err(DurableJournalError::new(
                DurableJournalErrorCode::CorruptSnapshot,
                "journal snapshot size changed while reading",
            ));
        }
        let journal: TransactionJournal = serde_json::from_slice(&bytes).map_err(|error| {
            DurableJournalError::new(
                DurableJournalErrorCode::CorruptSnapshot,
                format!("parse journal snapshot: {error}"),
            )
        })?;
        validate_for_persistence(&journal).map_err(|error| {
            DurableJournalError::new(
                DurableJournalErrorCode::CorruptSnapshot,
                format!("invalid journal snapshot: {error}"),
            )
        })?;
        let head = journal.events.last().ok_or_else(|| {
            DurableJournalError::new(
                DurableJournalErrorCode::CorruptSnapshot,
                "snapshot has no journal head",
            )
        })?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                DurableJournalError::new(
                    DurableJournalErrorCode::CorruptSnapshot,
                    "snapshot name is not portable UTF-8",
                )
            })?;
        if name != snapshot_name(head.sequence, &head.event_sha256) {
            return Err(DurableJournalError::new(
                DurableJournalErrorCode::CorruptSnapshot,
                "snapshot name does not bind the journal head",
            ));
        }
        if let Some(previous) = prior.as_ref() {
            validate_append_only_successor(previous, &journal).map_err(|error| {
                DurableJournalError::new(
                    DurableJournalErrorCode::CorruptSnapshot,
                    format!("snapshot history is not append-only: {error}"),
                )
            })?;
        }
        if let Some(expected) = expected
            && (journal.transaction_id != expected.transaction_id
                || journal.plan_id != expected.plan_id
                || journal.operation != expected.operation)
        {
            return Err(DurableJournalError::new(
                DurableJournalErrorCode::HistoryConflict,
                "existing journal identity conflicts with publication",
            ));
        }
        prior = Some(journal);
        prior_name = name.to_string();
    }
    Ok(prior.map(|journal| RecoveredJournalHead {
        snapshot_count,
        journal,
        snapshot_name: prior_name,
    }))
}

fn validate_append_only_successor(
    prior: &TransactionJournal,
    next: &TransactionJournal,
) -> Result<(), DurableJournalError> {
    let same_identity = next.transaction_id == prior.transaction_id
        && next.plan_id == prior.plan_id
        && next.operation == prior.operation
        && next.durability == prior.durability;
    let exact_successor =
        next.events.len() == prior.events.len() + 1 && next.events.starts_with(&prior.events);
    if same_identity && exact_successor {
        Ok(())
    } else {
        Err(DurableJournalError::new(
            DurableJournalErrorCode::HistoryConflict,
            "journal publication must append exactly one event to the durable head",
        ))
    }
}

fn publication(
    status: SnapshotPublicationStatus,
    snapshot_name: String,
    sequence: u32,
    event_sha256: &str,
    bytes: usize,
) -> SnapshotPublication {
    SnapshotPublication {
        status,
        snapshot_name,
        sequence,
        event_sha256: event_sha256.to_string(),
        snapshot_bytes: bytes as u64,
    }
}

fn snapshot_name(sequence: u32, event_sha256: &str) -> String {
    format!("{sequence:04}-{event_sha256}.json")
}

#[cfg(any(unix, windows))]
fn write_new_synced_file(path: &Path, bytes: &[u8]) -> Result<(), DurableJournalError> {
    let parent = path.parent().ok_or_else(|| {
        DurableJournalError::new(
            DurableJournalErrorCode::UnsafeRoot,
            "pending path has no parent",
        )
    })?;
    let name = path.file_name().ok_or_else(|| {
        DurableJournalError::new(
            DurableJournalErrorCode::UnsafeRoot,
            "pending path has no name",
        )
    })?;
    rz0_secure_fs::SecureDirectory::open(parent)
        .and_then(|directory| directory.write_new_child(name, bytes, MAX_JOURNAL_SNAPSHOT_BYTES))
        .map(|_| ())
        .map_err(|error| secure_error("write pending journal snapshot", error))
}

#[cfg(not(any(unix, windows)))]
fn write_new_synced_file(path: &Path, bytes: &[u8]) -> Result<(), DurableJournalError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| io_error("create pending journal snapshot", error))?;
    file.write_all(bytes)
        .map_err(|error| io_error("write pending journal snapshot", error))?;
    file.sync_all()
        .map_err(|error| io_error("sync pending journal snapshot", error))
}

#[cfg(any(unix, windows))]
fn create_private_directory(path: &Path) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("journal directory has no parent"))?;
    let name = path
        .file_name()
        .ok_or_else(|| std::io::Error::other("journal directory has no name"))?;
    rz0_secure_fs::SecureDirectory::open(parent)
        .and_then(|directory| directory.create_child_directory(name))
        .map(|_| ())
        .map_err(|error| std::io::Error::other(error.to_string()))
}

#[cfg(not(any(unix, windows)))]
fn create_private_directory(path: &Path) -> std::io::Result<()> {
    fs::create_dir(path)
}

fn verify_direct_directory(path: &Path) -> Result<(), DurableJournalError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| io_error("inspect journal directory", error))?;
    if metadata.is_dir() && !unsafe_link_type(&metadata) {
        Ok(())
    } else {
        Err(DurableJournalError::new(
            DurableJournalErrorCode::UnsafeFilesystemType,
            "journal directory is a symlink, reparse point, or wrong type",
        ))
    }
}

fn verify_direct_regular_file(path: &Path) -> Result<(), DurableJournalError> {
    open_direct_snapshot(path).map(|_| ())
}

#[cfg(any(unix, windows))]
fn open_direct_snapshot(path: &Path) -> Result<(File, fs::Metadata), DurableJournalError> {
    let parent = path.parent().ok_or_else(|| {
        DurableJournalError::new(
            DurableJournalErrorCode::UnsafeRoot,
            "snapshot has no parent",
        )
    })?;
    let name = path.file_name().ok_or_else(|| {
        DurableJournalError::new(DurableJournalErrorCode::UnsafeRoot, "snapshot has no name")
    })?;
    let opened = rz0_secure_fs::SecureDirectory::open(parent)
        .and_then(|directory| directory.open_child_file(name))
        .map_err(|error| secure_error("open journal snapshot", error))?;
    let metadata = opened
        .file()
        .metadata()
        .map_err(|error| io_error("inspect root-relative opened journal snapshot", error))?;
    Ok((opened.into_file(), metadata))
}

#[cfg(not(any(unix, windows)))]
fn open_direct_snapshot(path: &Path) -> Result<(File, fs::Metadata), DurableJournalError> {
    let mut options = OpenOptions::new();
    options.read(true);
    let file = options
        .open(path)
        .map_err(|error| io_error("open journal snapshot without following links", error))?;
    let metadata = file
        .metadata()
        .map_err(|error| io_error("inspect opened journal snapshot", error))?;
    if !metadata.is_file() || unsafe_link_type(&metadata) || !has_single_link(&file, &metadata)? {
        return Err(DurableJournalError::new(
            DurableJournalErrorCode::UnsafeFilesystemType,
            "journal snapshot is linked, reparse-backed, or the wrong type",
        ));
    }
    Ok((file, metadata))
}

#[cfg(not(any(unix, windows)))]
fn has_single_link(_file: &File, _metadata: &fs::Metadata) -> Result<bool, DurableJournalError> {
    Ok(false)
}

#[cfg(windows)]
fn unsafe_link_type(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    metadata.file_type().is_symlink() || metadata.file_attributes() & 0x0400 != 0
}

#[cfg(not(windows))]
fn unsafe_link_type(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(any(unix, windows))]
fn sync_directory(path: &Path) -> Result<(), DurableJournalError> {
    rz0_secure_fs::SecureDirectory::open(path)
        .and_then(|directory| directory.sync())
        .map_err(|error| secure_error("sync journal directory", error))
}

#[cfg(not(any(unix, windows)))]
fn sync_directory(_path: &Path) -> Result<(), DurableJournalError> {
    Ok(())
}

#[cfg(any(unix, windows))]
fn publish_pending_file(
    temporary: &Path,
    heads: &Path,
    snapshot_name: &str,
) -> Result<(), DurableJournalError> {
    let source_parent = temporary.parent().ok_or_else(|| {
        DurableJournalError::new(
            DurableJournalErrorCode::UnsafeRoot,
            "pending path has no parent",
        )
    })?;
    let pending_name = temporary.file_name().ok_or_else(|| {
        DurableJournalError::new(
            DurableJournalErrorCode::UnsafeRoot,
            "pending path has no name",
        )
    })?;
    let source = rz0_secure_fs::SecureDirectory::open(source_parent)
        .map_err(|error| secure_error("open pending snapshot directory", error))?;
    let destination = rz0_secure_fs::SecureDirectory::open(heads)
        .map_err(|error| secure_error("open snapshot heads directory", error))?;
    source
        .publish_child_noreplace(
            pending_name,
            &destination,
            std::ffi::OsStr::new(snapshot_name),
        )
        .map(|_| ())
        .map_err(|error| secure_error("publish immutable journal snapshot", error))
}

#[cfg(not(any(unix, windows)))]
fn publish_pending_file(
    temporary: &Path,
    heads: &Path,
    snapshot_name: &str,
) -> Result<(), DurableJournalError> {
    let final_path = heads.join(snapshot_name);
    if let Err(error) = fs::rename(temporary, &final_path) {
        let _ = fs::remove_file(temporary);
        return Err(io_error("publish immutable journal snapshot", error));
    }
    Ok(())
}

#[cfg(any(unix, windows))]
fn secure_error(context: &str, error: rz0_secure_fs::SecureFsError) -> DurableJournalError {
    use rz0_error_contract::FoundationErrorCode as Foundation;

    let code = match error.foundation_code() {
        Foundation::Conflict => DurableJournalErrorCode::HistoryConflict,
        Foundation::ArtifactIdentityChanged | Foundation::PermissionDenied => {
            DurableJournalErrorCode::UnsafeFilesystemType
        }
        Foundation::InputLimitExceeded => DurableJournalErrorCode::SnapshotLimitExceeded,
        Foundation::RecoveryRequired => DurableJournalErrorCode::RecoveryRequired,
        _ => DurableJournalErrorCode::Io,
    };
    DurableJournalError::new(code, format!("{context}: {error}"))
}

fn io_error(context: &str, error: std::io::Error) -> DurableJournalError {
    DurableJournalError::new(DurableJournalErrorCode::Io, format!("{context}: {error}"))
}

struct WriterLock {
    #[cfg(any(unix, windows))]
    _lock: rz0_secure_fs::SecureFileLock,
}

impl WriterLock {
    #[cfg(any(unix, windows))]
    fn acquire(root: &Path, transaction_id: &str) -> Result<Self, DurableJournalError> {
        let name = format!(".{transaction_id}.writer.lock");
        let file = rz0_secure_fs::SecureDirectory::open(root)
            .and_then(|directory| directory.open_or_create_lock_file(std::ffi::OsStr::new(&name)))
            .map_err(|error| secure_error("open journal writer lock", error))?;
        let lock = rz0_secure_fs::SecureFileLock::try_exclusive(file).map_err(|error| {
            if error.code == rz0_secure_fs::SecureFsErrorCode::LockBusy {
                DurableJournalError::new(
                    DurableJournalErrorCode::WriterBusy,
                    format!("acquire journal writer lock: {error}"),
                )
            } else {
                secure_error("acquire journal writer lock", error)
            }
        })?;
        Ok(Self { _lock: lock })
    }

    #[cfg(not(any(unix, windows)))]
    fn acquire(_root: &Path, _transaction_id: &str) -> Result<Self, DurableJournalError> {
        Err(DurableJournalError::new(
            DurableJournalErrorCode::Io,
            "journal writer locks are unsupported on this platform",
        ))
    }
}
