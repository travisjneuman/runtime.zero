use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use sha2::{Digest, Sha256};

use crate::{
    ArtifactExpectation, ArtifactIdentityError, ArtifactIdentityErrorCode, ArtifactObservation,
    MAX_ARTIFACT_BYTES, MAX_EXECUTABLE_BYTES, VerifiedArtifact,
    identity::{has_single_link, identity_from_file},
    path_policy::checked_artifact_path,
    platform_open::open_artifact,
};

pub fn revalidate_verified_artifact(
    artifact: &mut VerifiedArtifact,
) -> Result<(), ArtifactIdentityError> {
    revalidate_verified_artifact_with_limit(artifact, MAX_ARTIFACT_BYTES)
}

pub fn revalidate_verified_executable(
    artifact: &mut VerifiedArtifact,
) -> Result<(), ArtifactIdentityError> {
    revalidate_verified_artifact_with_limit(artifact, MAX_EXECUTABLE_BYTES)
}

fn revalidate_verified_artifact_with_limit(
    artifact: &mut VerifiedArtifact,
    maximum_bytes: u64,
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
    if metadata.len() != artifact.size_bytes || metadata.len() > maximum_bytes {
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
        .take(maximum_bytes + 1)
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
    validate_expectation(expectation, MAX_ARTIFACT_BYTES)?;
    let artifact = open_artifact_evidence(root, relative_path, MAX_ARTIFACT_BYTES)?;
    if artifact.size_bytes != expectation.size_bytes {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::SizeMismatch,
            "artifact size does not match the sealed expectation",
        ));
    }
    if artifact.sha256 != expectation.sha256 {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::DigestMismatch,
            "artifact digest does not match the sealed expectation",
        ));
    }
    Ok(artifact)
}

pub fn open_verified_executable(
    root: &Path,
    relative_path: &str,
    expectation: &ArtifactExpectation,
) -> Result<VerifiedArtifact, ArtifactIdentityError> {
    validate_expectation(expectation, MAX_EXECUTABLE_BYTES)?;
    let artifact = open_artifact_evidence(root, relative_path, MAX_EXECUTABLE_BYTES)?;
    if artifact.size_bytes != expectation.size_bytes {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::SizeMismatch,
            "executable size does not match the sealed expectation",
        ));
    }
    if artifact.sha256 != expectation.sha256 {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::DigestMismatch,
            "executable digest does not match the sealed expectation",
        ));
    }
    Ok(artifact)
}

/// Opens one direct artifact relative to a held root and records its observed
/// identity, size, and digest without treating those observations as trust.
///
/// Callers may use this only to create evidence that is subsequently sealed by
/// a plan or receipt. Execution must reopen the artifact against that sealed
/// expectation and retain a platform binding through spawn.
pub fn open_observed_artifact(
    root: &Path,
    relative_path: &str,
) -> Result<ArtifactObservation, ArtifactIdentityError> {
    let artifact = open_artifact_evidence(root, relative_path, MAX_ARTIFACT_BYTES)?;
    Ok(ArtifactObservation {
        relative_path: artifact.relative_path,
        canonical_path: artifact.canonical_path,
        sha256: artifact.sha256,
        size_bytes: artifact.size_bytes,
    })
}

pub fn open_observed_executable(
    root: &Path,
    relative_path: &str,
) -> Result<ArtifactObservation, ArtifactIdentityError> {
    let artifact = open_artifact_evidence(root, relative_path, MAX_EXECUTABLE_BYTES)?;
    Ok(ArtifactObservation {
        relative_path: artifact.relative_path,
        canonical_path: artifact.canonical_path,
        sha256: artifact.sha256,
        size_bytes: artifact.size_bytes,
    })
}

fn open_artifact_evidence(
    root: &Path,
    relative_path: &str,
    maximum_bytes: u64,
) -> Result<VerifiedArtifact, ArtifactIdentityError> {
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
    if opened_metadata.len() > maximum_bytes {
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
        .take(maximum_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| io_error("read opened artifact", error))?;
    if bytes.len() as u64 > maximum_bytes {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::TooLarge,
            "artifact content exceeds the size ceiling",
        ));
    }
    if opened_metadata.len() != bytes.len() as u64 {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::SizeMismatch,
            "artifact size changed while reading the opened bytes",
        ));
    }
    let observed_sha256 = format!("{:x}", Sha256::digest(&bytes));

    let canonical_path = fs::canonicalize(&checked.artifact_path).map_err(|_| {
        ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::IdentityChanged,
            "artifact path disappeared after observation",
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
            "artifact path could not be reopened after observation",
        )
    })?;
    let current_metadata = current_file.metadata().map_err(|_| {
        ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::IdentityChanged,
            "artifact path metadata changed after observation",
        )
    })?;
    if identity_from_file(&current_file, &current_metadata)? != identity {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::IdentityChanged,
            "artifact path no longer identifies the opened file",
        ));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| io_error("rewind observed artifact", error))?;

    Ok(VerifiedArtifact {
        relative_path: relative_path.to_string(),
        canonical_path,
        sha256: observed_sha256,
        size_bytes: bytes.len() as u64,
        identity,
        file,
    })
}

fn validate_expectation(
    expectation: &ArtifactExpectation,
    maximum_bytes: u64,
) -> Result<(), ArtifactIdentityError> {
    if expectation.size_bytes > maximum_bytes
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
