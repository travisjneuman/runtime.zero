use std::path::{Path, PathBuf};

#[cfg(not(windows))]
use crate::ArtifactIdentityErrorCode;
use crate::{ArtifactIdentityError, VerifiedArtifact};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutableBindingMechanism {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    ProcHeldDescriptorPath,
    #[cfg(windows)]
    DenyWriteDeleteHandle,
}

#[derive(Debug)]
pub struct BoundExecutable<'a> {
    launch_path: PathBuf,
    mechanism: ExecutableBindingMechanism,
    _artifact: &'a VerifiedArtifact,
}

impl BoundExecutable<'_> {
    pub fn launch_path(&self) -> &Path {
        &self.launch_path
    }

    pub const fn mechanism(&self) -> ExecutableBindingMechanism {
        self.mechanism
    }

    pub const fn execution_authorized(&self) -> bool {
        false
    }
}

/// Binds a future platform spawn to the already verified open artifact.
///
/// This closes only executable identity-to-spawn replacement. The returned
/// lease grants no capability, trust, confirmation, isolation, or execution
/// authority and must remain alive until the child has been created.
pub fn bind_verified_executable(
    artifact: &VerifiedArtifact,
) -> Result<BoundExecutable<'_>, ArtifactIdentityError> {
    platform_binding(artifact)
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn platform_binding(
    artifact: &VerifiedArtifact,
) -> Result<BoundExecutable<'_>, ArtifactIdentityError> {
    use std::os::{fd::AsRawFd, unix::fs::MetadataExt};

    let metadata = artifact.file.metadata().map_err(|error| {
        ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::Io,
            format!("inspect executable artifact: {error}"),
        )
    })?;
    if metadata.mode() & 0o111 == 0 {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::UnsafeFilesystemType,
            "verified artifact has no executable permission bit",
        ));
    }
    let launch_path = PathBuf::from(format!("/proc/self/fd/{}", artifact.file.as_raw_fd()));
    let bound = std::fs::metadata(&launch_path).map_err(|error| {
        ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::Io,
            format!("inspect held-descriptor executable path: {error}"),
        )
    })?;
    if bound.dev() != metadata.dev() || bound.ino() != metadata.ino() {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::IdentityChanged,
            "held-descriptor executable path does not resolve to the verified identity",
        ));
    }
    Ok(BoundExecutable {
        launch_path,
        mechanism: ExecutableBindingMechanism::ProcHeldDescriptorPath,
        _artifact: artifact,
    })
}

#[cfg(windows)]
fn platform_binding(
    artifact: &VerifiedArtifact,
) -> Result<BoundExecutable<'_>, ArtifactIdentityError> {
    // open_artifact holds a share-read-only handle, denying write and delete
    // opens while this lease remains alive. CreateProcess therefore resolves a
    // path whose underlying file cannot be replaced through normal Win32 share
    // semantics during the identity-to-spawn interval.
    Ok(BoundExecutable {
        launch_path: artifact.canonical_path.clone(),
        mechanism: ExecutableBindingMechanism::DenyWriteDeleteHandle,
        _artifact: artifact,
    })
}

#[cfg(not(any(target_os = "linux", target_os = "android", windows)))]
fn platform_binding(
    _artifact: &VerifiedArtifact,
) -> Result<BoundExecutable<'_>, ArtifactIdentityError> {
    Err(ArtifactIdentityError::new(
        ArtifactIdentityErrorCode::UnsafeFilesystemType,
        "this platform has no reviewed exact executable-handle spawn binding",
    ))
}
