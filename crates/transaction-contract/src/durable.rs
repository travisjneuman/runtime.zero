use std::{
    fmt,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

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
    if let Err(error) = fs::rename(&temporary, &final_path) {
        let _ = fs::remove_file(&temporary);
        return Err(io_error("publish immutable journal snapshot", error));
    }
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

fn write_new_synced_file(path: &Path, bytes: &[u8]) -> Result<(), DurableJournalError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(path)
        .map_err(|error| io_error("create pending journal snapshot", error))?;
    file.write_all(bytes)
        .map_err(|error| io_error("write pending journal snapshot", error))?;
    file.sync_all()
        .map_err(|error| io_error("sync pending journal snapshot", error))
}

#[cfg(unix)]
fn create_private_directory(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    let mut builder = fs::DirBuilder::new();
    builder.mode(0o700).create(path)
}

#[cfg(not(unix))]
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

fn open_direct_snapshot(path: &Path) -> Result<(File, fs::Metadata), DurableJournalError> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
        options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    }
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

#[cfg(unix)]
fn has_single_link(_file: &File, metadata: &fs::Metadata) -> Result<bool, DurableJournalError> {
    use std::os::unix::fs::MetadataExt;

    Ok(metadata.nlink() == 1)
}

#[cfg(windows)]
fn has_single_link(file: &File, _metadata: &fs::Metadata) -> Result<bool, DurableJournalError> {
    use std::{mem::MaybeUninit, os::windows::io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };

    let mut information = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::zeroed();
    // SAFETY: the structure is writable and the handle is borrowed from the
    // live opened snapshot for this synchronous metadata query.
    if unsafe { GetFileInformationByHandle(file.as_raw_handle(), information.as_mut_ptr()) } == 0 {
        return Err(io_error(
            "query journal snapshot link count",
            std::io::Error::last_os_error(),
        ));
    }
    // SAFETY: a successful call initialized the complete structure.
    Ok(unsafe { information.assume_init() }.nNumberOfLinks == 1)
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

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), DurableJournalError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| io_error("sync journal directory", error))
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), DurableJournalError> {
    Ok(())
}

fn io_error(context: &str, error: std::io::Error) -> DurableJournalError {
    DurableJournalError::new(DurableJournalErrorCode::Io, format!("{context}: {error}"))
}

struct WriterLock {
    file: File,
}

impl WriterLock {
    fn acquire(root: &Path, transaction_id: &str) -> Result<Self, DurableJournalError> {
        let path = root.join(format!(".{transaction_id}.writer.lock"));
        if let Ok(metadata) = fs::symlink_metadata(&path)
            && unsafe_link_type(&metadata)
        {
            return Err(DurableJournalError::new(
                DurableJournalErrorCode::UnsafeFilesystemType,
                "journal writer lock is a symlink or reparse point",
            ));
        }
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options
                .mode(0o600)
                .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            use windows_sys::Win32::Storage::FileSystem::FILE_FLAG_OPEN_REPARSE_POINT;
            options.custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
        }
        let file = options
            .open(&path)
            .map_err(|error| io_error("open journal writer lock", error))?;
        let metadata = file
            .metadata()
            .map_err(|error| io_error("inspect opened journal writer lock", error))?;
        if !metadata.is_file() || unsafe_link_type(&metadata) {
            return Err(DurableJournalError::new(
                DurableJournalErrorCode::UnsafeFilesystemType,
                "journal writer lock is reparse-backed or the wrong type",
            ));
        }
        lock_file(&file)?;
        Ok(Self { file })
    }
}

impl Drop for WriterLock {
    fn drop(&mut self) {
        unlock_file(&self.file);
    }
}

#[cfg(unix)]
fn lock_file(file: &File) -> Result<(), DurableJournalError> {
    use std::os::fd::AsRawFd;

    // SAFETY: flock operates on the borrowed live descriptor and stores lock
    // ownership in the open file description held by WriterLock.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        Ok(())
    } else {
        let error = std::io::Error::last_os_error();
        let code = if error.kind() == std::io::ErrorKind::WouldBlock {
            DurableJournalErrorCode::WriterBusy
        } else {
            DurableJournalErrorCode::Io
        };
        Err(DurableJournalError::new(
            code,
            format!("acquire journal writer lock: {error}"),
        ))
    }
}

#[cfg(unix)]
fn unlock_file(file: &File) {
    use std::os::fd::AsRawFd;

    // SAFETY: unlocks only the advisory lock held by this live descriptor.
    let _ = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
}

#[cfg(windows)]
fn lock_file(file: &File) -> Result<(), DurableJournalError> {
    use std::{mem::MaybeUninit, os::windows::io::AsRawHandle};
    use windows_sys::Win32::{
        Storage::FileSystem::{LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx},
        System::IO::OVERLAPPED,
    };

    let mut overlapped = MaybeUninit::<OVERLAPPED>::zeroed();
    // SAFETY: the handle is live and the zeroed OVERLAPPED storage remains
    // valid for this synchronous non-overlapped lock request.
    let result = unsafe {
        LockFileEx(
            file.as_raw_handle(),
            LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
            0,
            u32::MAX,
            u32::MAX,
            overlapped.as_mut_ptr(),
        )
    };
    if result != 0 {
        Ok(())
    } else {
        let error = std::io::Error::last_os_error();
        let code = if error.raw_os_error() == Some(33) {
            DurableJournalErrorCode::WriterBusy
        } else {
            DurableJournalErrorCode::Io
        };
        Err(DurableJournalError::new(
            code,
            format!("acquire journal writer lock: {error}"),
        ))
    }
}

#[cfg(windows)]
fn unlock_file(file: &File) {
    use std::{mem::MaybeUninit, os::windows::io::AsRawHandle};
    use windows_sys::Win32::{Storage::FileSystem::UnlockFileEx, System::IO::OVERLAPPED};

    let mut overlapped = MaybeUninit::<OVERLAPPED>::zeroed();
    // SAFETY: releases only the byte range locked by this live descriptor.
    let _ = unsafe {
        UnlockFileEx(
            file.as_raw_handle(),
            0,
            u32::MAX,
            u32::MAX,
            overlapped.as_mut_ptr(),
        )
    };
}

#[cfg(not(any(unix, windows)))]
fn lock_file(_file: &File) -> Result<(), DurableJournalError> {
    Err(DurableJournalError::new(
        DurableJournalErrorCode::Io,
        "journal writer locks are unsupported on this platform",
    ))
}

#[cfg(not(any(unix, windows)))]
fn unlock_file(_file: &File) {}
