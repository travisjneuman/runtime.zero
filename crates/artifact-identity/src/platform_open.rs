use std::{fs::File, path::Path};

use crate::{ArtifactIdentityError, ArtifactIdentityErrorCode};

#[cfg(unix)]
pub(crate) fn open_artifact(root: &Path, relative: &str) -> Result<File, ArtifactIdentityError> {
    use std::{
        ffi::CString,
        os::fd::{AsRawFd, FromRawFd},
        os::unix::ffi::OsStrExt,
        path::Component,
    };

    let root_path = CString::new(root.as_os_str().as_bytes()).map_err(|_| {
        ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::UnsafeRoot,
            "artifact root contains a null byte",
        )
    })?;
    // SAFETY: root_path is a valid C string; a successful descriptor is
    // immediately owned by File and closed on every return path.
    let root_descriptor = unsafe {
        libc::open(
            root_path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if root_descriptor == -1 {
        return Err(open_error("open artifact root without following links"));
    }
    // SAFETY: open returned a new owned descriptor on success.
    let mut directory = unsafe { File::from_raw_fd(root_descriptor) };
    let components = Path::new(relative).components().collect::<Vec<_>>();
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(component) = component else {
            return Err(ArtifactIdentityError::new(
                ArtifactIdentityErrorCode::UnsafeRelativePath,
                "artifact path contains a non-normal component",
            ));
        };
        let component = CString::new(component.as_bytes()).map_err(|_| {
            ArtifactIdentityError::new(
                ArtifactIdentityErrorCode::UnsafeRelativePath,
                "artifact path component contains a null byte",
            )
        })?;
        let is_final = index + 1 == components.len();
        let mut flags = libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW;
        if !is_final {
            flags |= libc::O_DIRECTORY;
        }
        // SAFETY: directory is a live opened directory, component is a valid C
        // string, and a successful descriptor is immediately transferred to File.
        let descriptor = unsafe { libc::openat(directory.as_raw_fd(), component.as_ptr(), flags) };
        if descriptor == -1 {
            return Err(open_error("open artifact component relative to held root"));
        }
        // SAFETY: openat returned a new owned descriptor on success.
        let opened = unsafe { File::from_raw_fd(descriptor) };
        if is_final {
            return Ok(opened);
        }
        directory = opened;
    }
    Err(ArtifactIdentityError::new(
        ArtifactIdentityErrorCode::UnsafeRelativePath,
        "artifact path has no components",
    ))
}

#[cfg(unix)]
fn open_error(context: &str) -> ArtifactIdentityError {
    let error = std::io::Error::last_os_error();
    let code = match error.raw_os_error() {
        Some(code) if code == libc::ELOOP || code == libc::ENOTDIR => {
            ArtifactIdentityErrorCode::UnsafeFilesystemType
        }
        _ => ArtifactIdentityErrorCode::Io,
    };
    ArtifactIdentityError::new(code, format!("{context}: {error}"))
}

#[cfg(windows)]
pub(crate) fn open_artifact(root: &Path, relative: &str) -> Result<File, ArtifactIdentityError> {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ};

    let mut options = std::fs::OpenOptions::new();
    options
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    options.open(root.join(relative)).map_err(|error| {
        ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::Io,
            format!("open Windows artifact: {error}"),
        )
    })
}
