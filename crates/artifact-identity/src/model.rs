use std::{fmt, fs::File, path::PathBuf};

pub use rz0_resource_contract::{MAX_ARTIFACT_BYTES, MAX_EXECUTABLE_BYTES};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactExpectation {
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactFileIdentity {
    #[cfg(unix)]
    Unix {
        device: u64,
        inode: u64,
        link_count: u64,
    },
    #[cfg(windows)]
    Windows {
        volume_serial_number: u32,
        file_index: u64,
        link_count: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactObservation {
    pub relative_path: String,
    pub canonical_path: PathBuf,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug)]
pub struct VerifiedArtifact {
    pub relative_path: String,
    pub canonical_path: PathBuf,
    pub sha256: String,
    pub size_bytes: u64,
    pub identity: ArtifactFileIdentity,
    pub(crate) file: File,
}

impl VerifiedArtifact {
    pub fn file(&self) -> &File {
        &self.file
    }

    pub fn into_file(self) -> File {
        self.file
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactIdentityErrorCode {
    InvalidExpectation,
    UnsafeRoot,
    UnsafeRelativePath,
    UnsafeFilesystemType,
    HardlinkRejected,
    TooLarge,
    SizeMismatch,
    DigestMismatch,
    IdentityChanged,
    Io,
}

#[derive(Debug)]
pub struct ArtifactIdentityError {
    pub code: ArtifactIdentityErrorCode,
    detail: String,
}

impl ArtifactIdentityError {
    pub(crate) fn new(code: ArtifactIdentityErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for ArtifactIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.detail)
    }
}

impl std::error::Error for ArtifactIdentityError {}
