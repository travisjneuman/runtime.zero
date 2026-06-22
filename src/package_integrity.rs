use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::Path;

use crate::module_manifest::{
    HashAlgorithm, IntegrityRootPolicy, ModuleManifest, ModuleStatus, PackageFormat,
};
use crate::package_integrity_io::{
    is_reparse_or_symlink, is_valid_sha256, path_contains_reparse_or_symlink, sha256_file,
    validate_relative_path,
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
