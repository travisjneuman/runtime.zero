#![cfg(unix)]

use std::{
    ffi::OsStr,
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use rz0_secure_fs::{SecureDirectory, SecureFileLock, SecureFsErrorCode};

#[test]
fn creates_reads_and_syncs_private_children_relative_to_held_directories() {
    let root = TestRoot::new();
    let directory = SecureDirectory::open(root.path()).expect("open root");
    let child = directory
        .create_child_directory(OsStr::new("state"))
        .expect("create child");
    let opened = child
        .write_new_child(OsStr::new("registry.json"), b"{}", 64)
        .expect("write child");
    assert_eq!(opened.metadata().len(), 2);
    assert_eq!(
        child.read_child(OsStr::new("registry.json"), 64).unwrap(),
        b"{}"
    );
}

#[test]
fn unsafe_names_limits_and_existing_destinations_fail_closed() {
    let root = TestRoot::new();
    let directory = SecureDirectory::open(root.path()).expect("open root");
    for name in ["", ".", "..", "a/b"] {
        assert_eq!(
            directory
                .write_new_child(OsStr::new(name), b"x", 1)
                .unwrap_err()
                .code,
            SecureFsErrorCode::UnsafeName
        );
    }
    assert_eq!(
        directory
            .write_new_child(OsStr::new("large"), b"xx", 1)
            .unwrap_err()
            .code,
        SecureFsErrorCode::LimitExceeded
    );
    directory
        .write_new_child(OsStr::new("one"), b"1", 1)
        .expect("first");
    assert_eq!(
        directory
            .write_new_child(OsStr::new("one"), b"2", 1)
            .unwrap_err()
            .code,
        SecureFsErrorCode::AlreadyExists
    );
}

#[test]
fn publication_is_noreplace_and_retires_the_pending_link() {
    let root = TestRoot::new();
    let directory = SecureDirectory::open(root.path()).expect("open root");
    let pending = directory
        .create_child_directory(OsStr::new("pending"))
        .expect("pending");
    let published = directory
        .create_child_directory(OsStr::new("published"))
        .expect("published");
    pending
        .write_new_child(OsStr::new("next"), b"complete", 64)
        .expect("pending file");
    let opened = pending
        .publish_child_noreplace(OsStr::new("next"), &published, OsStr::new("head"))
        .expect("publish");
    assert_eq!(opened.metadata().len(), 8);
    assert!(!root.path().join("pending/next").exists());
    assert_eq!(
        fs::read(root.path().join("published/head")).unwrap(),
        b"complete"
    );

    pending
        .write_new_child(OsStr::new("again"), b"other", 64)
        .expect("second pending");
    assert_eq!(
        pending
            .publish_child_noreplace(OsStr::new("again"), &published, OsStr::new("head"))
            .unwrap_err()
            .code,
        SecureFsErrorCode::AlreadyExists
    );
    assert_eq!(
        fs::read(root.path().join("published/head")).unwrap(),
        b"complete"
    );
}

#[test]
fn explicit_privacy_verification_checks_owner_and_unix_permission_bits() {
    let root = TestRoot::new();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).expect("public mode");
    let directory = SecureDirectory::open(root.path()).expect("open root");
    assert_eq!(
        directory.verify_private().unwrap_err().code,
        SecureFsErrorCode::UnsafeDirectory
    );
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).expect("private mode");
    directory.verify_private().expect("private directory");
    let file = directory
        .write_new_child(OsStr::new("private"), b"x", 1)
        .expect("private file");
    file.verify_private().expect("private file policy");
}

#[test]
fn opened_lock_file_enforces_exclusive_nonblocking_ownership() {
    let root = TestRoot::new();
    let directory = SecureDirectory::open(root.path()).expect("open root");
    let first = directory
        .open_or_create_lock_file(OsStr::new("writer.lock"))
        .expect("first open");
    let second = directory
        .open_or_create_lock_file(OsStr::new("writer.lock"))
        .expect("second open");
    let held = SecureFileLock::try_exclusive(first).expect("first lock");
    let error = match SecureFileLock::try_exclusive(second) {
        Ok(_) => panic!("second lock unexpectedly succeeded"),
        Err(error) => error,
    };
    assert_eq!(error.code, SecureFsErrorCode::LockBusy);
    drop(held);
    let third = directory
        .open_or_create_lock_file(OsStr::new("writer.lock"))
        .expect("third open");
    SecureFileLock::try_exclusive(third).expect("lock after release");
}

#[test]
fn atomic_replacement_publishes_complete_bytes_and_retires_pending_name() {
    let root = TestRoot::new();
    let directory = SecureDirectory::open(root.path()).expect("open root");
    let pending = directory
        .create_child_directory(OsStr::new("pending-replace"))
        .expect("pending");
    let published = directory
        .create_child_directory(OsStr::new("published-replace"))
        .expect("published");
    published
        .write_new_child(OsStr::new("registry"), b"before", 64)
        .expect("before");
    pending
        .write_new_child(OsStr::new("next"), b"after", 64)
        .expect("next");
    let opened = pending
        .replace_child_atomic(OsStr::new("next"), &published, OsStr::new("registry"))
        .expect("replace");
    assert_eq!(opened.metadata().len(), 5);
    assert_eq!(
        fs::read(root.path().join("published-replace/registry")).unwrap(),
        b"after"
    );
    assert!(!root.path().join("pending-replace/next").exists());
}

#[test]
fn symlinks_and_hardlinks_are_rejected() {
    let root = TestRoot::new();
    let directory = SecureDirectory::open(root.path()).expect("open root");
    fs::write(root.path().join("source"), b"x").expect("source");
    fs::hard_link(root.path().join("source"), root.path().join("linked")).expect("hardlink");
    assert_eq!(
        directory
            .open_child_file(OsStr::new("source"))
            .unwrap_err()
            .code,
        SecureFsErrorCode::IdentityChanged
    );
    symlink(root.path().join("source"), root.path().join("symbolic")).expect("symlink");
    assert!(directory.open_child_file(OsStr::new("symbolic")).is_err());

    let linked_root = root.path().with_extension("link");
    symlink(root.path(), &linked_root).expect("root symlink");
    assert!(SecureDirectory::open(&linked_root).is_err());
    fs::remove_file(linked_root).expect("remove symlink fixture");
}

#[test]
fn held_directory_survives_path_replacement_without_redirecting_writes() {
    let root = TestRoot::new();
    let directory = SecureDirectory::open(root.path()).expect("open root");
    let moved = root.path().with_extension("moved");
    fs::rename(root.path(), &moved).expect("move held root");
    fs::create_dir(root.path()).expect("replacement root");

    directory
        .write_new_child(OsStr::new("held"), b"original", 64)
        .expect("write through held root");
    assert_eq!(fs::read(moved.join("held")).unwrap(), b"original");
    assert!(!root.path().join("held").exists());

    fs::remove_dir(root.path()).expect("remove replacement");
    fs::rename(&moved, root.path()).expect("restore test root");
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
            "rz0-secure-fs-{}-{nanos}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("create root");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
        let _ = fs::remove_dir_all(self.0.with_extension("moved"));
    }
}
