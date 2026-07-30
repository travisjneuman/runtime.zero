#![cfg(unix)]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(any(target_os = "linux", target_os = "android"))]
use std::{os::unix::fs::PermissionsExt, process::Command};

#[cfg(any(target_os = "linux", target_os = "android"))]
use rz0_artifact_identity::ExecutableBindingMechanism;
use rz0_artifact_identity::{
    ArtifactExpectation, bind_verified_executable, open_verified_artifact,
};
use sha2::{Digest, Sha256};

#[cfg(any(target_os = "linux", target_os = "android"))]
#[test]
fn held_descriptor_binding_executes_verified_identity_after_visible_path_replacement() {
    let root = TestRoot::new();
    let source = std::env::current_exe().expect("current test executable");
    let executable = root.path().join("verified-test-binary");
    fs::copy(&source, &executable).expect("copy executable fixture");
    let bytes = fs::read(&executable).expect("read fixture");
    let mut verified = open_verified_artifact(
        root.path(),
        "verified-test-binary",
        &ArtifactExpectation {
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            size_bytes: bytes.len() as u64,
        },
    )
    .expect("verify executable");
    let binding = bind_verified_executable(&verified).expect("bind held executable");
    assert_eq!(
        binding.mechanism(),
        ExecutableBindingMechanism::ProcHeldDescriptorPath
    );
    assert!(!binding.execution_authorized());

    fs::rename(&executable, root.path().join("original-moved"))
        .expect("move visible executable path");
    fs::write(&executable, b"#!/bin/sh\necho replacement-executed\n").expect("write replacement");
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).expect("replacement mode");

    let output = Command::new(binding.launch_path())
        .arg("--list")
        .output()
        .expect("execute held descriptor binding");
    assert!(output.status.success());
    assert!(!String::from_utf8_lossy(&output.stdout).contains("replacement-executed"));
    drop(binding);

    // Post-spawn revalidation remains a separate mandatory check and detects
    // that the visible receipt-relative path changed.
    assert!(rz0_artifact_identity::revalidate_verified_artifact(&mut verified).is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn macos_fails_closed_without_a_reviewed_handle_to_spawn_primitive() {
    let root = TestRoot::new();
    let source = std::env::current_exe().expect("current test executable");
    let executable = root.path().join("verified-test-binary");
    fs::copy(source, &executable).expect("copy executable fixture");
    let bytes = fs::read(&executable).expect("read fixture");
    let verified = open_verified_artifact(
        root.path(),
        "verified-test-binary",
        &ArtifactExpectation {
            sha256: format!("{:x}", Sha256::digest(&bytes)),
            size_bytes: bytes.len() as u64,
        },
    )
    .expect("verify executable");
    assert!(bind_verified_executable(&verified).is_err());
}

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "rz0-executable-binding-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
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
    }
}
