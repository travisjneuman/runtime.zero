use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use sha2::{Digest, Sha256};

use crate::{
    ArtifactExpectation, ArtifactIdentityError, ArtifactIdentityErrorCode, MAX_ARTIFACT_BYTES,
    VerifiedArtifact,
    identity::{has_single_link, identity_from_file},
    path_policy::checked_artifact_path,
    platform_open::open_artifact,
};

pub fn revalidate_verified_artifact(
    artifact: &mut VerifiedArtifact,
) -> Result<(), ArtifactIdentityError> {
    let metadata = artifact
        .file
        .metadata()
        .map_err(|error| io_error("re-read held artifact metadata", error))?;
    let identity = identity_from_file(&artifact.file, &metadata)?;
    if identity != artifact.identity || !has_single_link(&identity) {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::IdentityChanged,
            "held artifact identity or link count changed",
        ));
    }
    if metadata.len() != artifact.size_bytes || metadata.len() > MAX_ARTIFACT_BYTES {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::SizeMismatch,
            "held artifact size changed",
        ));
    }
    artifact
        .file
        .seek(SeekFrom::Start(0))
        .map_err(|error| io_error("rewind held artifact", error))?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    (&mut artifact.file)
        .take(MAX_ARTIFACT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| io_error("re-read held artifact", error))?;
    if bytes.len() as u64 != artifact.size_bytes {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::SizeMismatch,
            "held artifact bytes changed size",
        ));
    }
    if format!("{:x}", Sha256::digest(&bytes)) != artifact.sha256 {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::DigestMismatch,
            "held artifact digest changed",
        ));
    }
    let current_metadata = fs::symlink_metadata(&artifact.canonical_path).map_err(|_| {
        ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::IdentityChanged,
            "artifact path disappeared after use",
        )
    })?;
    if current_metadata.file_type().is_symlink() {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::IdentityChanged,
            "artifact path became a symbolic link",
        ));
    }
    let current_file = fs::File::open(&artifact.canonical_path).map_err(|_| {
        ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::IdentityChanged,
            "artifact path could not be reopened after use",
        )
    })?;
    let current_metadata = current_file.metadata().map_err(|_| {
        ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::IdentityChanged,
            "artifact path metadata changed after use",
        )
    })?;
    if identity_from_file(&current_file, &current_metadata)? != artifact.identity {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::IdentityChanged,
            "artifact path no longer identifies the held file",
        ));
    }
    artifact
        .file
        .seek(SeekFrom::Start(0))
        .map_err(|error| io_error("rewind revalidated artifact", error))?;
    Ok(())
}

pub fn open_verified_artifact(
    root: &Path,
    relative_path: &str,
    expectation: &ArtifactExpectation,
) -> Result<VerifiedArtifact, ArtifactIdentityError> {
    validate_expectation(expectation)?;
    let checked = checked_artifact_path(root, relative_path)?;
    let mut file = open_artifact(root, relative_path)?;
    let opened_metadata = file
        .metadata()
        .map_err(|error| io_error("read opened artifact metadata", error))?;
    if !opened_metadata.is_file() {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::UnsafeFilesystemType,
            "opened artifact is not a regular file",
        ));
    }
    if opened_metadata.len() > MAX_ARTIFACT_BYTES {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::TooLarge,
            "opened artifact exceeds the size ceiling",
        ));
    }
    let identity = identity_from_file(&file, &opened_metadata)?;
    if !has_single_link(&identity) {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::HardlinkRejected,
            "opened artifact must have exactly one filesystem link",
        ));
    }

    let mut bytes = Vec::with_capacity(opened_metadata.len() as usize);
    (&mut file)
        .take(MAX_ARTIFACT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| io_error("read opened artifact", error))?;
    if bytes.len() as u64 > MAX_ARTIFACT_BYTES {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::TooLarge,
            "artifact content exceeds the size ceiling",
        ));
    }
    if opened_metadata.len() != bytes.len() as u64 || expectation.size_bytes != bytes.len() as u64 {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::SizeMismatch,
            "artifact size does not match opened bytes and expectation",
        ));
    }
    let observed_sha256 = format!("{:x}", Sha256::digest(&bytes));
    if observed_sha256 != expectation.sha256 {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::DigestMismatch,
            "artifact digest does not match expectation",
        ));
    }

    let canonical_path = fs::canonicalize(&checked.artifact_path).map_err(|_| {
        ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::IdentityChanged,
            "artifact path disappeared after verification",
        )
    })?;
    if !canonical_path.starts_with(&checked.canonical_root) {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::IdentityChanged,
            "artifact path escaped its canonical root",
        ));
    }
    let current_file = fs::File::open(&canonical_path).map_err(|_| {
        ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::IdentityChanged,
            "artifact path could not be reopened after verification",
        )
    })?;
    let current_metadata = current_file.metadata().map_err(|_| {
        ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::IdentityChanged,
            "artifact path metadata changed after verification",
        )
    })?;
    if identity_from_file(&current_file, &current_metadata)? != identity {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::IdentityChanged,
            "artifact path no longer identifies the opened file",
        ));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| io_error("rewind verified artifact", error))?;

    Ok(VerifiedArtifact {
        relative_path: relative_path.to_string(),
        canonical_path,
        sha256: observed_sha256,
        size_bytes: bytes.len() as u64,
        identity,
        file,
    })
}

fn validate_expectation(expectation: &ArtifactExpectation) -> Result<(), ArtifactIdentityError> {
    if expectation.size_bytes > MAX_ARTIFACT_BYTES
        || !rz0_validation_contract::valid_sha256(&expectation.sha256)
    {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::InvalidExpectation,
            "artifact expectation is invalid or exceeds the ceiling",
        ));
    }
    Ok(())
}

fn io_error(context: &str, error: std::io::Error) -> ArtifactIdentityError {
    ArtifactIdentityError::new(ArtifactIdentityErrorCode::Io, format!("{context}: {error}"))
}
