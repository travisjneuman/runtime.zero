use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Component, Path};

use crate::module_manifest::{
    HashAlgorithm, IntegrityRootPolicy, ModuleManifest, ModuleStatus, PackageFormat,
};

pub const MAX_PACKAGE_FILE_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_PACKAGE_FILE_COUNT: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackageIntegrityReport {
    pub valid: bool,
    pub checked_files: Vec<FileIntegrityReport>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FileIntegrityReport {
    pub path: String,
    pub status: FileIntegrityStatus,
    pub expected_sha256: String,
    pub actual_sha256: Option<String>,
    pub expected_size_bytes: Option<u64>,
    pub actual_size_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileIntegrityStatus {
    Ok,
    Missing,
    PathInvalid,
    MalformedHash,
    TooLarge,
    SizeMismatch,
    HashMismatch,
    UnsupportedFileType,
    ReadError,
}

impl PackageIntegrityReport {
    fn new() -> Self {
        Self {
            valid: true,
            checked_files: Vec::new(),
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    fn fail(&mut self, error: impl Into<String>) {
        self.valid = false;
        self.errors.push(error.into());
    }
}

pub fn verify_package_integrity(
    manifest_path: &Path,
    manifest: &ModuleManifest,
) -> PackageIntegrityReport {
    let Some(integrity) = &manifest.integrity else {
        return missing_integrity_report(manifest);
    };

    let mut report = PackageIntegrityReport::new();
    if !matches!(integrity.package_format, PackageFormat::Directory) {
        report.fail("unsupported package_format; only directory is supported");
    }
    if !matches!(
        integrity.root_policy,
        IntegrityRootPolicy::ManifestDirectory
    ) {
        report.fail("unsupported root_policy; only manifest_directory is supported");
    }
    if !matches!(integrity.hash_algorithm, HashAlgorithm::Sha256) {
        report.fail("unsupported hash_algorithm; only sha256 is supported");
    }
    if integrity.signature.is_some() {
        report.fail("signature verification is not supported in this local-only slice");
    }
    if integrity.files.len() > MAX_PACKAGE_FILE_COUNT {
        report.fail(format!(
            "integrity file list exceeds {MAX_PACKAGE_FILE_COUNT} entries"
        ));
    }
    if manifest.status == ModuleStatus::Installed && integrity.files.is_empty() {
        report.fail("installed manifests with integrity metadata must list at least one file");
    }

    let package_root = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let mut seen_paths = BTreeSet::new();
    for file in integrity.files.iter().take(MAX_PACKAGE_FILE_COUNT) {
        if !seen_paths.insert(file.path.clone()) {
            report.fail(format!("duplicate integrity path '{}'", file.path));
            continue;
        }
        verify_one_file(
            package_root,
            &file.path,
            &file.sha256,
            file.size_bytes,
            &mut report,
        );
    }

    report
}

fn missing_integrity_report(manifest: &ModuleManifest) -> PackageIntegrityReport {
    let mut report = PackageIntegrityReport::new();
    match manifest.status {
        ModuleStatus::Installed => {
            report.fail("integrity metadata is required for installed module manifests");
        }
        ModuleStatus::Planned => {
            report.warnings.push(
                "integrity metadata missing; planned manifests are not package-verified"
                    .to_string(),
            );
        }
        ModuleStatus::Active => {}
    }
    report
}

fn verify_one_file(
    package_root: &Path,
    relative_path: &str,
    expected_sha256: &str,
    expected_size_bytes: Option<u64>,
    report: &mut PackageIntegrityReport,
) {
    let mut file_report = FileIntegrityReport {
        path: relative_path.to_string(),
        status: FileIntegrityStatus::Ok,
        expected_sha256: expected_sha256.to_ascii_lowercase(),
        actual_sha256: None,
        expected_size_bytes,
        actual_size_bytes: None,
    };

    if !is_valid_sha256(expected_sha256) {
        file_report.status = FileIntegrityStatus::MalformedHash;
        report.fail(format!("malformed SHA-256 for '{}'", relative_path));
        report.checked_files.push(file_report);
        return;
    }
    if let Err(err) = validate_relative_path(relative_path) {
        file_report.status = FileIntegrityStatus::PathInvalid;
        report.fail(format!("invalid integrity path '{}': {err}", relative_path));
        report.checked_files.push(file_report);
        return;
    }

    let candidate = package_root.join(relative_path);
    match path_contains_reparse_or_symlink(package_root, relative_path) {
        Ok(true) => {
            file_report.status = FileIntegrityStatus::UnsupportedFileType;
            report.fail(format!(
                "integrity path '{}' must not pass through a symlink or reparse point",
                relative_path
            ));
            report.checked_files.push(file_report);
            return;
        }
        Ok(false) => {}
        Err(err) if err.kind() == io::ErrorKind::NotFound => {}
        Err(err) => {
            file_report.status = FileIntegrityStatus::ReadError;
            report.fail(format!(
                "failed to inspect path components for '{}': {err}",
                relative_path
            ));
            report.checked_files.push(file_report);
            return;
        }
    }
    let metadata = match fs::symlink_metadata(&candidate) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            file_report.status = FileIntegrityStatus::Missing;
            report.fail(format!("missing integrity file '{}'", relative_path));
            report.checked_files.push(file_report);
            return;
        }
        Err(err) => {
            file_report.status = FileIntegrityStatus::ReadError;
            report.fail(format!(
                "failed to read metadata for '{}': {err}",
                relative_path
            ));
            report.checked_files.push(file_report);
            return;
        }
    };

    if is_reparse_or_symlink(&metadata) || !metadata.file_type().is_file() {
        file_report.status = FileIntegrityStatus::UnsupportedFileType;
        report.fail(format!(
            "integrity path '{}' must be a regular file and must not be a symlink or reparse point",
            relative_path
        ));
        report.checked_files.push(file_report);
        return;
    }

    let actual_size = metadata.len();
    file_report.actual_size_bytes = Some(actual_size);
    if actual_size > MAX_PACKAGE_FILE_BYTES {
        file_report.status = FileIntegrityStatus::TooLarge;
        report.fail(format!(
            "integrity file '{}' exceeds {MAX_PACKAGE_FILE_BYTES} bytes",
            relative_path
        ));
        report.checked_files.push(file_report);
        return;
    }
    if expected_size_bytes.is_some_and(|expected| expected != actual_size) {
        file_report.status = FileIntegrityStatus::SizeMismatch;
        report.fail(format!("size mismatch for '{}'", relative_path));
        report.checked_files.push(file_report);
        return;
    }

    let actual_sha256 = match sha256_file(&candidate) {
        Ok(hash) => hash,
        Err(err) => {
            file_report.status = FileIntegrityStatus::ReadError;
            report.fail(format!("failed to hash '{}': {err}", relative_path));
            report.checked_files.push(file_report);
            return;
        }
    };
    file_report.actual_sha256 = Some(actual_sha256.clone());
    if actual_sha256 != expected_sha256.to_ascii_lowercase() {
        file_report.status = FileIntegrityStatus::HashMismatch;
        report.fail(format!("hash mismatch for '{}'", relative_path));
    }
    report.checked_files.push(file_report);
}

fn validate_relative_path(path: &str) -> Result<(), &'static str> {
    if path.trim().is_empty() {
        return Err("path must not be empty");
    }
    if looks_url_like(path) {
        return Err("URL-like paths are not supported");
    }
    if path.contains('\\') {
        return Err("backslash paths are not supported");
    }
    let path = Path::new(path);
    if path.is_absolute() {
        return Err("absolute paths are not supported");
    }
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir => {}
            Component::ParentDir => return Err(".. traversal is not supported"),
            Component::RootDir | Component::Prefix(_) => {
                return Err("absolute paths are not supported");
            }
        }
    }
    Ok(())
}

fn looks_url_like(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("://")
        || lower.starts_with("file:")
        || lower.starts_with("http:")
        || lower.starts_with("https:")
}

fn is_valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(to_lower_hex(&hasher.finalize()))
}

fn to_lower_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(hex_char(byte >> 4));
        output.push(hex_char(byte & 0x0f));
    }
    output
}

fn hex_char(value: u8) -> char {
    match value {
        0..=9 => char::from(b'0' + value),
        10..=15 => char::from(b'a' + value - 10),
        _ => unreachable!("nibble is always 0..=15"),
    }
}

fn is_reparse_or_symlink(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn path_contains_reparse_or_symlink(package_root: &Path, relative_path: &str) -> io::Result<bool> {
    let mut current = package_root.to_path_buf();
    for component in Path::new(relative_path).components() {
        let Component::Normal(part) = component else {
            continue;
        };
        current.push(part);
        let metadata = fs::symlink_metadata(&current)?;
        if is_reparse_or_symlink(&metadata) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::module_manifest::{
        HashAlgorithm, IntegrityRootPolicy, ModuleKind, ModuleSafety, PackageFileIntegrity,
        PackageFileRole, PackageFormat, PackageIntegrity, RiskLevel,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

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

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock works")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("rz0-{name}-{stamp}"));
        fs::create_dir(&path).expect("temp dir created");
        path
    }

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
        fs::remove_file(payload).expect("payload removed");
        fs::remove_dir(root).expect("temp dir removed");
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
        fs::remove_file(payload).expect("payload removed");
        fs::remove_dir(root).expect("temp dir removed");
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
}
