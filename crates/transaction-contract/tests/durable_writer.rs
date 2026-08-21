use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use rz0_transaction_contract::{
    DurabilityRequirements, DurableJournalErrorCode, SnapshotPublicationStatus,
    TRANSACTION_CONTRACT, TRANSACTION_SCHEMA_VERSION, TransactionEvent, TransactionEventKind,
    TransactionJournal, TransactionOperation, TransactionState, inspect_journal_head,
    publish_journal_snapshot, recover_journal_head, seal_transaction_journal,
};

#[test]
fn publishes_recovers_and_idempotently_reuses_immutable_snapshots() {
    let root = TestRoot::new();
    let mut journal = journal();
    let first = publish_journal_snapshot(root.path(), &journal).expect("publish prepared");
    assert_eq!(first.status, SnapshotPublicationStatus::Published);
    let duplicate = publish_journal_snapshot(root.path(), &journal).expect("idempotent publish");
    assert_eq!(
        duplicate.status,
        SnapshotPublicationStatus::AlreadyPublished
    );

    journal
        .events
        .push(event(TransactionEventKind::ApplyStarted));
    journal.state = TransactionState::Applying;
    seal_transaction_journal(&mut journal);
    publish_journal_snapshot(root.path(), &journal).expect("publish applying");

    let recovered = recover_journal_head(root.path(), &journal.transaction_id).expect("recover");
    assert_eq!(recovered.journal, journal);
    assert_eq!(recovered.snapshot_count, 2);
    assert_eq!(
        recovered.snapshot_name,
        snapshot_path(root.path(), &journal)
    );
}

#[test]
fn read_only_journal_inspection_does_not_create_or_require_a_writer_lock() {
    let root = TestRoot::new();
    let journal = journal();
    publish_journal_snapshot(root.path(), &journal).expect("publish prepared");
    let lock_path = root
        .path()
        .join(format!(".{}.writer.lock", journal.transaction_id));
    fs::remove_file(&lock_path).expect("remove test lock marker");

    let inspected = inspect_journal_head(root.path(), &journal.transaction_id)
        .expect("inspect without mutation");
    assert_eq!(inspected.journal, journal);
    assert!(!lock_path.exists());
}

#[test]
fn first_snapshot_and_every_successor_must_preserve_the_exact_prefix() {
    let root = TestRoot::new();
    let mut skipped = journal();
    skipped
        .events
        .push(event(TransactionEventKind::ApplyStarted));
    skipped.state = TransactionState::Applying;
    seal_transaction_journal(&mut skipped);
    let error = publish_journal_snapshot(root.path(), &skipped).expect_err("skip rejected");
    assert_eq!(error.code, DurableJournalErrorCode::HistoryConflict);

    let root = TestRoot::new();
    let prepared = journal();
    publish_journal_snapshot(root.path(), &prepared).expect("publish prepared");
    let mut drifted = prepared.clone();
    drifted.plan_id = "rz0plan-other".to_string();
    drifted
        .events
        .push(event(TransactionEventKind::ApplyStarted));
    drifted.state = TransactionState::Applying;
    seal_transaction_journal(&mut drifted);
    let error = publish_journal_snapshot(root.path(), &drifted).expect_err("identity rejected");
    assert_eq!(error.code, DurableJournalErrorCode::HistoryConflict);
}

#[test]
fn corruption_blocks_recovery_instead_of_skipping_history() {
    let root = TestRoot::new();
    let journal = journal();
    let publication = publish_journal_snapshot(root.path(), &journal).expect("publish");
    fs::write(
        root.path()
            .join(&journal.transaction_id)
            .join("heads")
            .join(publication.snapshot_name),
        b"{\"truncated\":",
    )
    .expect("inject corruption");
    let error = recover_journal_head(root.path(), &journal.transaction_id)
        .expect_err("corrupt history rejected");
    assert_eq!(error.code, DurableJournalErrorCode::CorruptSnapshot);
    assert_eq!(
        error.foundation_code(),
        rz0_error_contract::FoundationErrorCode::TransactionInvalid
    );
}

#[cfg(unix)]
#[test]
fn symlinked_history_and_busy_writer_lock_fail_closed() {
    use std::{
        fs::OpenOptions,
        os::unix::fs::{OpenOptionsExt, symlink},
    };

    let root = TestRoot::new();
    let journal = journal();
    let lock_path = root
        .path()
        .join(format!(".{}.writer.lock", journal.transaction_id));
    let lock = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(&lock_path)
        .expect("open competing lock");
    let _held =
        rz0_secure_fs::SecureFileLock::try_exclusive(lock).expect("acquire competing test lock");
    let error = publish_journal_snapshot(root.path(), &journal).expect_err("busy lock rejected");
    assert_eq!(error.code, DurableJournalErrorCode::WriterBusy);
    drop(_held);

    publish_journal_snapshot(root.path(), &journal).expect("publish after unlock");
    let transaction = root.path().join(&journal.transaction_id);
    let heads = transaction.join("heads");
    let moved = transaction.join("heads-moved");
    fs::rename(&heads, &moved).expect("move heads");
    symlink(&moved, &heads).expect("replace heads with symlink");
    let error = recover_journal_head(root.path(), &journal.transaction_id)
        .expect_err("symlink history rejected");
    assert_eq!(error.code, DurableJournalErrorCode::UnsafeFilesystemType);
}

#[test]
fn hardlinked_snapshot_fails_closed() {
    let root = TestRoot::new();
    let journal = journal();
    let publication = publish_journal_snapshot(root.path(), &journal).expect("publish");
    let snapshot = root
        .path()
        .join(&journal.transaction_id)
        .join("heads")
        .join(publication.snapshot_name);
    fs::hard_link(&snapshot, root.path().join("external-snapshot-link")).expect("create hardlink");
    let error =
        recover_journal_head(root.path(), &journal.transaction_id).expect_err("hardlink rejected");
    assert_eq!(error.code, DurableJournalErrorCode::UnsafeFilesystemType);
}

#[cfg(unix)]
#[test]
fn writer_creates_private_journal_directories_and_snapshots() {
    use std::os::unix::fs::PermissionsExt;

    let root = TestRoot::new();
    let journal = journal();
    let publication = publish_journal_snapshot(root.path(), &journal).expect("publish");
    let transaction = root.path().join(&journal.transaction_id);
    let snapshot = transaction.join("heads").join(publication.snapshot_name);
    assert_eq!(
        fs::metadata(transaction).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(snapshot).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

fn journal() -> TransactionJournal {
    let mut journal = TransactionJournal {
        schema_version: TRANSACTION_SCHEMA_VERSION,
        contract: TRANSACTION_CONTRACT.to_string(),
        transaction_id: "rz0tx-durable-writer".to_string(),
        plan_id: "rz0plan-durable-writer".to_string(),
        operation: TransactionOperation::ModuleInstall,
        state: TransactionState::Prepared,
        durability: DurabilityRequirements::schema_one(),
        events: vec![event(TransactionEventKind::Prepared)],
    };
    seal_transaction_journal(&mut journal);
    journal
}

fn event(kind: TransactionEventKind) -> TransactionEvent {
    TransactionEvent {
        sequence: 0,
        kind,
        action_id: None,
        path: None,
        before_sha256: None,
        after_sha256: None,
        previous_event_sha256: String::new(),
        event_sha256: String::new(),
    }
}

fn snapshot_path(_root: &Path, journal: &TransactionJournal) -> String {
    let head = journal.events.last().expect("head");
    format!("{:04}-{}.json", head.sequence, head.event_sha256)
}

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "rz0-durable-writer-{}-{nanos}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create test root");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
