use std::{
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(unix)]
use rz0_artifact_identity::revalidate_verified_artifact;
use rz0_artifact_identity::{
    ArtifactExpectation, ArtifactIdentityErrorCode, open_observed_artifact, open_verified_artifact,
};
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::io::{Seek, SeekFrom};

const ROOT_PREFIX: &str = "rz0-artifact-identity-sim-";
const ROOT_MARKER: &str = ".rz0-artifact-identity-test-v1";
const ROOT_MARKER_CONTENT: &[u8] = b"schema_version=1\ntest_only=true\n";
const PAYLOAD: &[u8] = b"synthetic runtime.zero executable bytes\n";
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[test]
fn opens_hashes_revalidates_and_rewinds_the_same_artifact_handle() {
    let root = TestRoot::new();
    let expectation = expectation(PAYLOAD);
    let verified = open_verified_artifact(root.path(), "bin/module.bin", &expectation)
        .expect("verified artifact");
    assert_eq!(verified.relative_path, "bin/module.bin");
    assert_eq!(verified.size_bytes, PAYLOAD.len() as u64);
    assert_eq!(verified.sha256, expectation.sha256);
    assert!(
        verified
            .canonical_path
            .starts_with(fs::canonicalize(root.path()).expect("canonical root"))
    );

    let mut held = verified.into_file();
    let mut bytes = Vec::new();
    held.read_to_end(&mut bytes).expect("read held artifact");
    assert_eq!(bytes, PAYLOAD);
}

#[test]
fn observed_identity_becomes_evidence_but_not_execution_authority() {
    let root = TestRoot::new();
    let observed =
        open_observed_artifact(root.path(), "bin/module.bin").expect("observed artifact");
    assert_eq!(observed.sha256, expectation(PAYLOAD).sha256);
    assert_eq!(observed.size_bytes, PAYLOAD.len() as u64);
    assert_eq!(observed.relative_path, "bin/module.bin");
}

#[test]
fn invalid_path_size_and_digest_expectations_fail_closed() {
    let root = TestRoot::new();
    let expectation = expectation(PAYLOAD);
    for path in ["../module.bin", "/bin/module.bin", "bin\\module.bin"] {
        let error = open_verified_artifact(root.path(), path, &expectation).unwrap_err();
        assert_eq!(error.code, ArtifactIdentityErrorCode::UnsafeRelativePath);
    }

    let mut wrong_size = expectation.clone();
    wrong_size.size_bytes += 1;
    let error = open_verified_artifact(root.path(), "bin/module.bin", &wrong_size).unwrap_err();
    assert_eq!(error.code, ArtifactIdentityErrorCode::SizeMismatch);

    let mut wrong_digest = expectation;
    wrong_digest.sha256 = "0".repeat(64);
    let error = open_verified_artifact(root.path(), "bin/module.bin", &wrong_digest).unwrap_err();
    assert_eq!(error.code, ArtifactIdentityErrorCode::DigestMismatch);
}

#[test]
fn hardlinked_artifacts_are_rejected() {
    let root = TestRoot::new();
    fs::hard_link(
        root.path().join("bin/module.bin"),
        root.path().join("bin/alias.bin"),
    )
    .expect("create hardlink fixture");
    let error =
        open_verified_artifact(root.path(), "bin/module.bin", &expectation(PAYLOAD)).unwrap_err();
    assert_eq!(error.code, ArtifactIdentityErrorCode::HardlinkRejected);
}

#[cfg(unix)]
#[test]
fn symlinked_roots_and_artifacts_are_rejected() {
    use std::os::unix::fs::symlink;

    let root = TestRoot::new();
    symlink(
        root.path().join("bin/module.bin"),
        root.path().join("bin/link.bin"),
    )
    .expect("artifact symlink");
    let error =
        open_verified_artifact(root.path(), "bin/link.bin", &expectation(PAYLOAD)).unwrap_err();
    assert_eq!(error.code, ArtifactIdentityErrorCode::UnsafeFilesystemType);

    let linked_root = root.path().with_extension("linked");
    symlink(root.path(), &linked_root).expect("root symlink");
    let error =
        open_verified_artifact(&linked_root, "bin/module.bin", &expectation(PAYLOAD)).unwrap_err();
    assert_eq!(error.code, ArtifactIdentityErrorCode::UnsafeRoot);
    fs::remove_file(linked_root).expect("remove root symlink fixture");
}

#[cfg(unix)]
#[test]
fn verified_open_handle_keeps_original_bytes_and_detects_path_replacement() {
    let root = TestRoot::new();
    let mut verified = open_verified_artifact(root.path(), "bin/module.bin", &expectation(PAYLOAD))
        .expect("verified artifact");
    fs::rename(
        root.path().join("bin/module.bin"),
        root.path().join("bin/original.bin"),
    )
    .expect("move verified path");
    write_new(
        &root.path().join("bin/module.bin"),
        b"replacement must not change held bytes\n",
    )
    .expect("replacement fixture");

    let error = revalidate_verified_artifact(&mut verified).unwrap_err();
    assert_eq!(error.code, ArtifactIdentityErrorCode::IdentityChanged);
    let mut held = verified.into_file();
    held.seek(SeekFrom::Start(0)).expect("rewind held artifact");
    let mut bytes = Vec::new();
    held.read_to_end(&mut bytes).expect("read held artifact");
    assert_eq!(bytes, PAYLOAD);
}

fn expectation(bytes: &[u8]) -> ArtifactExpectation {
    ArtifactExpectation {
        sha256: format!("{:x}", Sha256::digest(bytes)),
        size_bytes: bytes.len() as u64,
    }
}

struct TestRoot {
    path: PathBuf,
}

impl TestRoot {
    fn new() -> Self {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("{ROOT_PREFIX}{}-{sequence}", std::process::id()));
        fs::create_dir(&path).expect("create test root");
        write_new(&path.join(ROOT_MARKER), ROOT_MARKER_CONTENT).expect("root marker");
        fs::create_dir(path.join("bin")).expect("bin directory");
        write_new(&path.join("bin/module.bin"), PAYLOAD).expect("artifact fixture");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let Ok(temp) = fs::canonicalize(std::env::temp_dir()) else {
            return;
        };
        let Ok(root) = fs::canonicalize(&self.path) else {
            return;
        };
        let marker = root.join(ROOT_MARKER);
        if root.parent() == Some(temp.as_path())
            && root
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(ROOT_PREFIX))
            && fs::symlink_metadata(&marker).is_ok_and(|metadata| metadata.is_file())
            && fs::read(marker).ok().as_deref() == Some(ROOT_MARKER_CONTENT)
        {
            let _ = fs::remove_dir_all(root);
        }
    }
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("create fixture: {error}"))?;
    file.write_all(bytes)
        .map_err(|error| format!("write fixture: {error}"))
}
