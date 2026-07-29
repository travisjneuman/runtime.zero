use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use rz0_transaction_contract::{
    DurabilityRequirements, RecoveryDecision, TRANSACTION_CONTRACT, TRANSACTION_SCHEMA_VERSION,
    TransactionEvent, TransactionEventKind, TransactionJournal, TransactionOperation,
    TransactionState, assess_recovery, seal_transaction_journal, validate_transaction_journal,
};

const ROOT_PREFIX: &str = "rz0-journal-sim-";
const MARKER: &str = ".rz0-journal-simulation-v1";
const MARKER_BYTES: &[u8] = b"schema_version=1\nsimulation_only=true\n";
const MAX_SNAPSHOT_BYTES: u64 = 2 * 1024 * 1024;
static SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn immutable_synced_snapshots_recover_the_latest_valid_head() {
    let root = SimulationRoot::new();
    let mut journal = journal(
        vec![event(TransactionEventKind::Prepared)],
        TransactionState::Prepared,
    );
    persist_snapshot(root.path(), &journal).expect("prepared snapshot");

    journal
        .events
        .push(event(TransactionEventKind::ApplyStarted));
    journal.state = TransactionState::Applying;
    seal_transaction_journal(&mut journal);
    persist_snapshot(root.path(), &journal).expect("applying snapshot");

    let recovered = load_head(root.path()).expect("recover head");
    assert_eq!(recovered.state, TransactionState::Applying);
    assert_eq!(recovered.events.len(), 2);
    assert_eq!(
        assess_recovery(&recovered).decision,
        RecoveryDecision::RollBackVerifiedWrites
    );
}

#[test]
fn interruption_before_next_snapshot_preserves_the_previous_recovery_point() {
    let root = SimulationRoot::new();
    let journal = journal(
        vec![event(TransactionEventKind::Prepared)],
        TransactionState::Prepared,
    );
    persist_snapshot(root.path(), &journal).expect("prepared snapshot");

    let mut unpersisted = journal.clone();
    unpersisted
        .events
        .push(event(TransactionEventKind::ApplyStarted));
    unpersisted.state = TransactionState::Applying;
    seal_transaction_journal(&mut unpersisted);

    let recovered = load_head(root.path()).expect("recover prior head");
    assert_eq!(recovered.events.len(), 1);
    assert_eq!(
        assess_recovery(&recovered).decision,
        RecoveryDecision::AbortWithoutWrites
    );
}

#[test]
fn corruption_and_history_rewrite_fail_closed() {
    let root = SimulationRoot::new();
    let mut journal = journal(
        vec![event(TransactionEventKind::Prepared)],
        TransactionState::Prepared,
    );
    persist_snapshot(root.path(), &journal).expect("first snapshot");
    journal
        .events
        .push(event(TransactionEventKind::ApplyStarted));
    journal.state = TransactionState::Applying;
    seal_transaction_journal(&mut journal);
    let latest = persist_snapshot(root.path(), &journal).expect("second snapshot");

    fs::write(&latest, b"{\"truncated\":").expect("inject truncated snapshot");
    assert!(load_head(root.path()).unwrap_err().contains("parse"));
}

#[cfg(unix)]
#[test]
fn symlinked_snapshot_fails_closed() {
    use std::os::unix::fs::symlink;

    let root = SimulationRoot::new();
    let journal = journal(
        vec![event(TransactionEventKind::Prepared)],
        TransactionState::Prepared,
    );
    let snapshot = persist_snapshot(root.path(), &journal).expect("snapshot");
    let moved = snapshot.with_extension("moved");
    fs::rename(&snapshot, &moved).expect("move snapshot");
    symlink(&moved, &snapshot).expect("symlink snapshot");
    assert!(load_head(root.path()).unwrap_err().contains("regular"));
}

fn persist_snapshot(root: &Path, journal: &TransactionJournal) -> Result<PathBuf, String> {
    validate_root(root)?;
    let validation = validate_transaction_journal(journal);
    if !validation.valid {
        return Err(format!("invalid journal: {:?}", validation.errors));
    }
    let head = journal
        .events
        .last()
        .ok_or_else(|| "missing head".to_string())?;
    let directory = root.join("heads");
    fs::create_dir_all(&directory).map_err(|error| format!("create heads: {error}"))?;
    ensure_direct_directory(root, &directory)?;
    let path = directory.join(format!("{:04}-{}.json", head.sequence, head.event_sha256));
    let bytes =
        serde_json::to_vec(journal).map_err(|error| format!("serialize snapshot: {error}"))?;
    if bytes.len() as u64 > MAX_SNAPSHOT_BYTES {
        return Err("snapshot exceeds byte ceiling".to_string());
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| format!("create immutable snapshot: {error}"))?;
    file.write_all(&bytes)
        .map_err(|error| format!("write snapshot: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("sync snapshot: {error}"))?;
    sync_directory(&directory)?;
    Ok(path)
}

fn load_head(root: &Path) -> Result<TransactionJournal, String> {
    validate_root(root)?;
    let directory = root.join("heads");
    ensure_direct_directory(root, &directory)?;
    let mut paths = fs::read_dir(&directory)
        .map_err(|error| format!("read heads: {error}"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("read head entry: {error}"))?;
    paths.sort();
    if paths.is_empty() || paths.len() > 1024 {
        return Err("snapshot count is absent or exceeds the ceiling".to_string());
    }
    let mut previous: Option<TransactionJournal> = None;
    for path in paths {
        ensure_direct_regular_file(root, &path)?;
        let mut bytes = Vec::new();
        File::open(&path)
            .map_err(|error| format!("open snapshot: {error}"))?
            .take(MAX_SNAPSHOT_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("read snapshot: {error}"))?;
        if bytes.len() as u64 > MAX_SNAPSHOT_BYTES {
            return Err("snapshot exceeds byte ceiling".to_string());
        }
        let journal: TransactionJournal =
            serde_json::from_slice(&bytes).map_err(|error| format!("parse snapshot: {error}"))?;
        let validation = validate_transaction_journal(&journal);
        if !validation.valid {
            return Err(format!("invalid snapshot: {:?}", validation.errors));
        }
        validate_snapshot_name(&path, &journal)?;
        if let Some(prior) = previous.as_ref()
            && (journal.transaction_id != prior.transaction_id
                || journal.plan_id != prior.plan_id
                || journal.operation != prior.operation
                || journal.events.len() != prior.events.len() + 1
                || journal.events[..prior.events.len()] != prior.events)
        {
            return Err("snapshot history is not an append-only prefix".to_string());
        }
        previous = Some(journal);
    }
    previous.ok_or_else(|| "no snapshot head".to_string())
}

fn validate_snapshot_name(path: &Path, journal: &TransactionJournal) -> Result<(), String> {
    let head = journal
        .events
        .last()
        .ok_or_else(|| "missing head".to_string())?;
    let expected = format!("{:04}-{}.json", head.sequence, head.event_sha256);
    if path.file_name().and_then(|name| name.to_str()) != Some(expected.as_str()) {
        return Err("snapshot name does not bind its sequence and head".to_string());
    }
    Ok(())
}

fn validate_root(root: &Path) -> Result<(), String> {
    let temp = fs::canonicalize(std::env::temp_dir()).map_err(|error| error.to_string())?;
    let root = fs::canonicalize(root).map_err(|error| error.to_string())?;
    if root.parent() != Some(temp.as_path())
        || !root
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(ROOT_PREFIX))
        || !fs::symlink_metadata(root.join(MARKER))
            .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
        || fs::read(root.join(MARKER)).ok().as_deref() != Some(MARKER_BYTES)
    {
        return Err("invalid journal simulation root".to_string());
    }
    Ok(())
}

fn ensure_direct_directory(root: &Path, path: &Path) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("inspect directory: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() || !path.starts_with(root) {
        return Err("journal directory is not a direct regular directory".to_string());
    }
    Ok(())
}

fn ensure_direct_regular_file(root: &Path, path: &Path) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("inspect snapshot: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || !path.starts_with(root) {
        return Err("snapshot is not a direct regular file".to_string());
    }
    Ok(())
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> Result<(), String> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("sync snapshot directory: {error}"))
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> Result<(), String> {
    Ok(())
}

fn journal(events: Vec<TransactionEvent>, state: TransactionState) -> TransactionJournal {
    let mut journal = TransactionJournal {
        schema_version: TRANSACTION_SCHEMA_VERSION,
        contract: TRANSACTION_CONTRACT.to_string(),
        transaction_id: "rz0tx-durable-sim".to_string(),
        plan_id: "rz0plan-durable-sim".to_string(),
        operation: TransactionOperation::ModuleInstall,
        state,
        durability: DurabilityRequirements::schema_one(),
        events,
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

struct SimulationRoot {
    path: PathBuf,
}

impl SimulationRoot {
    fn new() -> Self {
        let suffix = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("{ROOT_PREFIX}{}-{suffix}", std::process::id()));
        fs::create_dir(&path).expect("create simulation root");
        fs::write(path.join(MARKER), MARKER_BYTES).expect("write marker");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SimulationRoot {
    fn drop(&mut self) {
        if validate_root(&self.path).is_ok() {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
