use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use runtime_zero::module_manifest::{
    HashAlgorithm, IntegrityRootPolicy, ModuleKind, ModuleManifest, ModuleSafety, ModuleStatus,
    PackageFileIntegrity, PackageFileRole, PackageFormat, PackageIntegrity, RiskLevel,
};
use runtime_zero::package_integrity::{FileIntegrityStatus, verify_package_integrity};

#[test]
fn installed_manifest_requires_integrity() {
    let report = verify_package_integrity(
        Path::new("rz0-module.json"),
        &manifest(ModuleStatus::Installed, None),
    );
    assert!(!report.valid);
    assert!(report.errors.iter().any(|error| error.contains("required")));
}

#[test]
fn planned_manifest_without_integrity_warns_only() {
    let report = verify_package_integrity(
        Path::new("rz0-module.json"),
        &manifest(ModuleStatus::Planned, None),
    );
    assert!(report.valid, "{:?}", report.errors);
    assert_eq!(report.warnings.len(), 1);
}

#[test]
fn verifies_listed_file_sha256_and_size() {
    let root = temp_dir("valid-integrity");
    let payload = root.join("payload.txt");
    fs::write(&payload, b"runtime.zero\n").expect("payload written");
    let hash = "fb0ea974eb0ee094b6866120467df42fe274a7e500b79fec656bc26da197e4de";
    let report = verify_package_integrity(
        &root.join("rz0-module.json"),
        &manifest(
            ModuleStatus::Installed,
            Some(integrity(vec![package_file("payload.txt", hash, Some(13))])),
        ),
    );
    cleanup(root);
    assert!(report.valid, "{:?}", report.errors);
    assert_eq!(report.checked_files[0].status, FileIntegrityStatus::Ok);
}

#[test]
fn rejects_hash_mismatch() {
    let root = temp_dir("hash-mismatch");
    let payload = root.join("payload.txt");
    fs::write(&payload, b"runtime.zero\n").expect("payload written");
    let wrong_hash = "0000000000000000000000000000000000000000000000000000000000000000";
    let report = verify_package_integrity(
        &root.join("rz0-module.json"),
        &manifest(
            ModuleStatus::Installed,
            Some(integrity(vec![package_file(
                "payload.txt",
                wrong_hash,
                Some(13),
            )])),
        ),
    );
    cleanup(root);
    assert!(!report.valid);
    assert_eq!(
        report.checked_files[0].status,
        FileIntegrityStatus::HashMismatch
    );
}

#[test]
fn rejects_traversal_path() {
    let report = verify_package_integrity(
        Path::new("rz0-module.json"),
        &manifest(
            ModuleStatus::Installed,
            Some(integrity(vec![package_file(
                "../payload.txt",
                "0000000000000000000000000000000000000000000000000000000000000000",
                None,
            )])),
        ),
    );
    assert!(!report.valid);
    assert_eq!(
        report.checked_files[0].status,
        FileIntegrityStatus::PathInvalid
    );
}

fn manifest(status: ModuleStatus, integrity: Option<PackageIntegrity>) -> ModuleManifest {
    let mut manifest = ModuleManifest::new(
        "first-party.inventory",
        "Inventory",
        "0.1.0",
        "runtime.zero",
        ModuleKind::FirstPartyModule,
        status,
        "Read-only inventory.",
        &["inventory"],
        &["windows"],
        RiskLevel::ReadOnly,
        ModuleSafety::module_contract_default(),
    );
    manifest.integrity = integrity;
    manifest
}

fn package_file(path: &str, sha256: &str, size_bytes: Option<u64>) -> PackageFileIntegrity {
    PackageFileIntegrity {
        path: path.to_string(),
        sha256: sha256.to_string(),
        size_bytes,
        role: PackageFileRole::Payload,
    }
}

fn integrity(files: Vec<PackageFileIntegrity>) -> PackageIntegrity {
    PackageIntegrity {
        package_format: PackageFormat::Directory,
        root_policy: IntegrityRootPolicy::ManifestDirectory,
        hash_algorithm: HashAlgorithm::Sha256,
        files,
        signature: None,
        provenance: None,
    }
}

fn temp_dir(name: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock works")
        .as_nanos();
    let path = std::env::temp_dir().join(format!("rz0-{name}-{stamp}"));
    fs::create_dir(&path).expect("temp dir created");
    path
}

fn cleanup(root: PathBuf) {
    fs::remove_dir_all(root).expect("temp dir removed");
}
