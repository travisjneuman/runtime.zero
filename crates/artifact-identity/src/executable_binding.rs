use std::path::{Path, PathBuf};

#[cfg(not(windows))]
use crate::ArtifactIdentityErrorCode;
use crate::{ArtifactIdentityError, VerifiedArtifact};

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutableBindingMechanism {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    ProcHeldDescriptorPath,
    #[cfg(target_os = "macos")]
    PathIdentityRevalidated,
    #[cfg(windows)]
    DenyWriteDeleteHandle,
    #[doc(hidden)]
    UnsupportedPlatformMarker,
}

impl ExecutableBindingMechanism {
    pub const fn as_str(self) -> &'static str {
        match self {
            #[cfg(any(target_os = "linux", target_os = "android"))]
            Self::ProcHeldDescriptorPath => "proc_held_descriptor_path",
            #[cfg(target_os = "macos")]
            Self::PathIdentityRevalidated => "path_identity_revalidated",
            #[cfg(windows)]
            Self::DenyWriteDeleteHandle => "deny_write_delete_handle",
            Self::UnsupportedPlatformMarker => "unsupported_platform_marker",
        }
    }
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

    /// Returns the canonical visible path whose identity was verified before
    /// this binding was created. A bound process host must ensure its requested
    /// executable matches this path before substituting the platform launch
    /// primitive.
    pub fn verified_path(&self) -> &Path {
        &self._artifact.canonical_path
    }

    pub const fn execution_authorized(&self) -> bool {
        false
    }

    /// Rechecks the visible launch path immediately before the serialized
    /// process-host spawn boundary. Linux uses the held descriptor path and
    /// Windows uses the deny-write handle; macOS has no public fexecve-style
    /// primitive, so it uses a path identity and digest revalidation at the
    /// last possible boundary.
    pub fn verify_spawn_path(&self) -> Result<(), ArtifactIdentityError> {
        #[cfg(target_os = "macos")]
        {
            verify_macos_path_identity(self._artifact)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Ok(())
        }
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
    artifact: &VerifiedArtifact,
) -> Result<BoundExecutable<'_>, ArtifactIdentityError> {
    #[cfg(target_os = "macos")]
    {
        verify_macos_path_identity(artifact)?;
        Ok(BoundExecutable {
            launch_path: artifact.canonical_path.clone(),
            mechanism: ExecutableBindingMechanism::PathIdentityRevalidated,
            _artifact: artifact,
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = artifact;
        Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::UnsafeFilesystemType,
            "this platform has no reviewed exact executable-handle spawn binding",
        ))
    }
}

#[cfg(target_os = "macos")]
fn verify_macos_path_identity(artifact: &VerifiedArtifact) -> Result<(), ArtifactIdentityError> {
    use sha2::{Digest, Sha256};
    use std::{
        fs,
        io::{Read, Seek, SeekFrom},
        os::unix::fs::MetadataExt,
    };

    let visible = fs::symlink_metadata(&artifact.canonical_path).map_err(|error| {
        ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::IdentityChanged,
            format!("inspect macOS manager path before spawn: {error}"),
        )
    })?;
    if visible.file_type().is_symlink() || !visible.is_file() || visible.mode() & 0o111 == 0 {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::UnsafeFilesystemType,
            "macOS manager path must remain a direct executable regular file",
        ));
    }
    let mut current = fs::File::open(&artifact.canonical_path).map_err(|error| {
        ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::IdentityChanged,
            format!("open macOS manager path before spawn: {error}"),
        )
    })?;
    let metadata = current.metadata().map_err(|error| {
        ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::Io,
            format!("inspect opened macOS manager path: {error}"),
        )
    })?;
    let identity = matches!(&artifact.identity, crate::ArtifactFileIdentity::Unix {
            device,
            inode,
            link_count,
        } if metadata.dev() == *device
            && metadata.ino() == *inode
            && metadata.nlink() == *link_count
            && *link_count == 1);
    if !identity || metadata.len() != artifact.size_bytes {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::IdentityChanged,
            "macOS manager path no longer identifies the sealed executable",
        ));
    }
    current.seek(SeekFrom::Start(0)).map_err(|error| {
        ArtifactIdentityError::new(ArtifactIdentityErrorCode::Io, error.to_string())
    })?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    current
        .take(crate::MAX_EXECUTABLE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| {
            ArtifactIdentityError::new(
                ArtifactIdentityErrorCode::Io,
                format!("read macOS manager path before spawn: {error}"),
            )
        })?;
    if bytes.len() as u64 != artifact.size_bytes
        || format!("{:x}", Sha256::digest(&bytes)) != artifact.sha256
    {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::DigestMismatch,
            "macOS manager path bytes no longer match the sealed executable",
        ));
    }
    Ok(())
}
