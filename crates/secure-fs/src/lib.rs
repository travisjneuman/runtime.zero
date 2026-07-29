use std::{
    ffi::OsStr,
    fmt,
    fs::{self, File},
    io::{Read, Write},
    path::{Component, Path},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecureFsErrorCode {
    UnsafeName,
    UnsafeDirectory,
    UnsupportedOperation,
    AlreadyExists,
    IdentityChanged,
    LimitExceeded,
    PublicationIncomplete,
    Io,
}

#[derive(Debug)]
pub struct SecureFsError {
    pub code: SecureFsErrorCode,
    detail: String,
}

impl SecureFsError {
    fn new(code: SecureFsErrorCode, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }

    pub const fn foundation_code(&self) -> rz0_error_contract::FoundationErrorCode {
        use rz0_error_contract::FoundationErrorCode as Foundation;

        match self.code {
            SecureFsErrorCode::UnsafeName => Foundation::InvalidContract,
            SecureFsErrorCode::UnsafeDirectory => Foundation::PermissionDenied,
            SecureFsErrorCode::UnsupportedOperation => Foundation::UnsupportedOperation,
            SecureFsErrorCode::AlreadyExists => Foundation::Conflict,
            SecureFsErrorCode::IdentityChanged => Foundation::ArtifactIdentityChanged,
            SecureFsErrorCode::LimitExceeded => Foundation::InputLimitExceeded,
            SecureFsErrorCode::PublicationIncomplete => Foundation::RecoveryRequired,
            SecureFsErrorCode::Io => Foundation::IoUnavailable,
        }
    }
}

impl fmt::Display for SecureFsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.detail)
    }
}

impl std::error::Error for SecureFsError {}

#[derive(Debug)]
pub struct SecureDirectory {
    file: File,
}

#[derive(Debug)]
pub struct SecureOpenedFile {
    file: File,
    metadata: fs::Metadata,
}

impl SecureOpenedFile {
    pub fn file(&self) -> &File {
        &self.file
    }

    pub fn metadata(&self) -> &fs::Metadata {
        &self.metadata
    }

    pub fn into_file(self) -> File {
        self.file
    }
}

impl SecureDirectory {
    pub fn open(path: &Path) -> Result<Self, SecureFsError> {
        platform::open_directory(path).map(|file| Self { file })
    }

    pub fn sync(&self) -> Result<(), SecureFsError> {
        self.file
            .sync_all()
            .map_err(|error| io_error("sync opened directory", error))
    }

    pub fn open_child_directory(&self, name: &OsStr) -> Result<Self, SecureFsError> {
        validate_child_name(name)?;
        platform::open_child_directory(&self.file, name).map(|file| Self { file })
    }

    pub fn create_child_directory(&self, name: &OsStr) -> Result<Self, SecureFsError> {
        validate_child_name(name)?;
        platform::create_child_directory(&self.file, name)?;
        let opened = self.open_child_directory(name)?;
        self.sync()?;
        Ok(opened)
    }

    pub fn write_new_child(
        &self,
        name: &OsStr,
        bytes: &[u8],
        maximum_bytes: u64,
    ) -> Result<SecureOpenedFile, SecureFsError> {
        validate_child_name(name)?;
        if bytes.len() as u64 > maximum_bytes {
            return Err(SecureFsError::new(
                SecureFsErrorCode::LimitExceeded,
                "new child exceeds its byte ceiling",
            ));
        }
        let mut file = platform::create_new_child_file(&self.file, name)?;
        file.write_all(bytes)
            .map_err(|error| io_error("write new child", error))?;
        file.sync_all()
            .map_err(|error| io_error("sync new child", error))?;
        let metadata = checked_regular_metadata(&file)?;
        self.sync()?;
        Ok(SecureOpenedFile { file, metadata })
    }

    pub fn open_child_file(&self, name: &OsStr) -> Result<SecureOpenedFile, SecureFsError> {
        validate_child_name(name)?;
        let file = platform::open_child_file(&self.file, name)?;
        let metadata = checked_regular_metadata(&file)?;
        Ok(SecureOpenedFile { file, metadata })
    }

    pub fn read_child(&self, name: &OsStr, maximum_bytes: u64) -> Result<Vec<u8>, SecureFsError> {
        let opened = self.open_child_file(name)?;
        if opened.metadata.len() > maximum_bytes {
            return Err(SecureFsError::new(
                SecureFsErrorCode::LimitExceeded,
                "child exceeds its read ceiling",
            ));
        }
        let mut bytes = Vec::with_capacity(opened.metadata.len() as usize);
        opened
            .file
            .take(maximum_bytes + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| io_error("read opened child", error))?;
        if bytes.len() as u64 > maximum_bytes || bytes.len() as u64 != opened.metadata.len() {
            return Err(SecureFsError::new(
                SecureFsErrorCode::IdentityChanged,
                "child size changed while reading",
            ));
        }
        Ok(bytes)
    }

    pub fn open_or_create_lock_file(&self, name: &OsStr) -> Result<File, SecureFsError> {
        validate_child_name(name)?;
        let file = platform::open_or_create_lock_file(&self.file, name)?;
        checked_regular_metadata(&file)?;
        Ok(file)
    }

    /// Publishes a complete pending file without overwriting an existing name.
    /// On Unix this uses link-at plus unlink-at under held directory handles.
    pub fn publish_child_noreplace(
        &self,
        pending_name: &OsStr,
        destination: &SecureDirectory,
        destination_name: &OsStr,
    ) -> Result<SecureOpenedFile, SecureFsError> {
        validate_child_name(pending_name)?;
        validate_child_name(destination_name)?;
        self.open_child_file(pending_name)?;
        platform::publish_child_noreplace(
            &self.file,
            pending_name,
            &destination.file,
            destination_name,
        )?;
        destination.sync()?;
        platform::unlink_child(&self.file, pending_name).map_err(|error| {
            SecureFsError::new(
                SecureFsErrorCode::PublicationIncomplete,
                format!("published child but could not retire pending link: {error}"),
            )
        })?;
        self.sync()?;
        destination.open_child_file(destination_name)
    }
}

fn validate_child_name(name: &OsStr) -> Result<(), SecureFsError> {
    let path = Path::new(name);
    let mut components = path.components();
    let valid = matches!(components.next(), Some(Component::Normal(_)))
        && components.next().is_none()
        && !name.is_empty();
    if valid {
        Ok(())
    } else {
        Err(SecureFsError::new(
            SecureFsErrorCode::UnsafeName,
            "child name must contain exactly one normal path component",
        ))
    }
}

fn checked_regular_metadata(file: &File) -> Result<fs::Metadata, SecureFsError> {
    let metadata = file
        .metadata()
        .map_err(|error| io_error("inspect opened child", error))?;
    if !metadata.is_file() || !platform::has_single_link(file, &metadata)? {
        return Err(SecureFsError::new(
            SecureFsErrorCode::IdentityChanged,
            "opened child is not a single-link regular file",
        ));
    }
    Ok(metadata)
}

fn io_error(context: &str, error: std::io::Error) -> SecureFsError {
    let code = if error.kind() == std::io::ErrorKind::AlreadyExists {
        SecureFsErrorCode::AlreadyExists
    } else {
        SecureFsErrorCode::Io
    };
    SecureFsError::new(code, format!("{context}: {error}"))
}

#[cfg(unix)]
mod platform {
    use std::{
        ffi::{CString, OsStr},
        fs::{File, Metadata},
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::ffi::OsStrExt,
        },
        path::Path,
    };

    use super::{SecureFsError, SecureFsErrorCode, io_error};

    pub fn open_directory(path: &Path) -> Result<File, SecureFsError> {
        let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| {
            SecureFsError::new(
                SecureFsErrorCode::UnsafeDirectory,
                "directory contains a null byte",
            )
        })?;
        // SAFETY: path is a valid C string and ownership of a successful
        // descriptor transfers immediately to File.
        let descriptor = unsafe {
            libc::open(
                path.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        opened_directory(descriptor, "open directory without following links")
    }

    pub fn open_child_directory(parent: &File, name: &OsStr) -> Result<File, SecureFsError> {
        let name = child_c_string(name)?;
        // SAFETY: parent is a live directory and name is one valid component.
        let descriptor = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        opened_directory(descriptor, "open child directory relative to held parent")
    }

    pub fn create_child_directory(parent: &File, name: &OsStr) -> Result<(), SecureFsError> {
        let name = child_c_string(name)?;
        // SAFETY: parent is live and name is one valid component.
        if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } == 0 {
            Ok(())
        } else {
            Err(io_error(
                "create child directory relative to held parent",
                std::io::Error::last_os_error(),
            ))
        }
    }

    pub fn create_new_child_file(parent: &File, name: &OsStr) -> Result<File, SecureFsError> {
        open_child(
            parent,
            name,
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0o600,
            "create new child relative to held parent",
        )
    }

    pub fn open_child_file(parent: &File, name: &OsStr) -> Result<File, SecureFsError> {
        open_child(
            parent,
            name,
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0,
            "open child file relative to held parent",
        )
    }

    pub fn open_or_create_lock_file(parent: &File, name: &OsStr) -> Result<File, SecureFsError> {
        open_child(
            parent,
            name,
            libc::O_RDWR | libc::O_CREAT | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0o600,
            "open lock file relative to held parent",
        )
    }

    pub fn publish_child_noreplace(
        source: &File,
        source_name: &OsStr,
        destination: &File,
        destination_name: &OsStr,
    ) -> Result<(), SecureFsError> {
        let source_name = child_c_string(source_name)?;
        let destination_name = child_c_string(destination_name)?;
        // SAFETY: both descriptors are held directories and both names are one
        // validated component. linkat fails rather than replacing a destination.
        if unsafe {
            libc::linkat(
                source.as_raw_fd(),
                source_name.as_ptr(),
                destination.as_raw_fd(),
                destination_name.as_ptr(),
                0,
            )
        } == 0
        {
            Ok(())
        } else {
            Err(io_error(
                "publish child without replacement",
                std::io::Error::last_os_error(),
            ))
        }
    }

    pub fn unlink_child(parent: &File, name: &OsStr) -> Result<(), SecureFsError> {
        let name = child_c_string(name)?;
        // SAFETY: parent is held and the name is one validated component.
        if unsafe { libc::unlinkat(parent.as_raw_fd(), name.as_ptr(), 0) } == 0 {
            Ok(())
        } else {
            Err(io_error(
                "unlink published pending child",
                std::io::Error::last_os_error(),
            ))
        }
    }

    pub fn has_single_link(_file: &File, metadata: &Metadata) -> Result<bool, SecureFsError> {
        use std::os::unix::fs::MetadataExt;
        Ok(metadata.nlink() == 1)
    }

    fn open_child(
        parent: &File,
        name: &OsStr,
        flags: i32,
        mode: libc::c_uint,
        context: &str,
    ) -> Result<File, SecureFsError> {
        let name = child_c_string(name)?;
        // SAFETY: parent is held, name is valid, and successful ownership moves
        // immediately to File.
        let descriptor = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags, mode) };
        if descriptor == -1 {
            Err(io_error(context, std::io::Error::last_os_error()))
        } else {
            // SAFETY: openat returned one new owned descriptor.
            Ok(unsafe { File::from_raw_fd(descriptor) })
        }
    }

    fn opened_directory(descriptor: i32, context: &str) -> Result<File, SecureFsError> {
        if descriptor == -1 {
            return Err(SecureFsError::new(
                SecureFsErrorCode::UnsafeDirectory,
                format!("{context}: {}", std::io::Error::last_os_error()),
            ));
        }
        // SAFETY: open/openat returned one new owned descriptor.
        let file = unsafe { File::from_raw_fd(descriptor) };
        let metadata = file
            .metadata()
            .map_err(|error| io_error("inspect opened directory", error))?;
        if metadata.is_dir() {
            Ok(file)
        } else {
            Err(SecureFsError::new(
                SecureFsErrorCode::UnsafeDirectory,
                "opened directory has the wrong filesystem type",
            ))
        }
    }

    fn child_c_string(name: &OsStr) -> Result<CString, SecureFsError> {
        CString::new(name.as_bytes()).map_err(|_| {
            SecureFsError::new(
                SecureFsErrorCode::UnsafeName,
                "child name contains a null byte",
            )
        })
    }
}

#[cfg(windows)]
mod platform {
    use std::{
        ffi::OsStr,
        fs::{File, Metadata, OpenOptions},
        os::windows::{fs::OpenOptionsExt, io::AsRawHandle},
        path::Path,
    };

    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        GetFileInformationByHandle,
    };

    use super::{SecureFsError, SecureFsErrorCode, io_error};

    pub fn open_directory(path: &Path) -> Result<File, SecureFsError> {
        let mut options = OpenOptions::new();
        options
            .read(true)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
        let file = options
            .open(path)
            .map_err(|error| io_error("open Windows directory handle", error))?;
        let metadata = file
            .metadata()
            .map_err(|error| io_error("inspect Windows directory handle", error))?;
        use std::os::windows::fs::MetadataExt;
        if metadata.is_dir() && metadata.file_attributes() & 0x0400 == 0 {
            Ok(file)
        } else {
            Err(SecureFsError::new(
                SecureFsErrorCode::UnsafeDirectory,
                "Windows directory is reparse-backed or the wrong type",
            ))
        }
    }

    pub fn open_child_directory(_parent: &File, _name: &OsStr) -> Result<File, SecureFsError> {
        unsupported()
    }

    pub fn create_child_directory(_parent: &File, _name: &OsStr) -> Result<(), SecureFsError> {
        unsupported()
    }

    pub fn create_new_child_file(_parent: &File, _name: &OsStr) -> Result<File, SecureFsError> {
        unsupported()
    }

    pub fn open_child_file(_parent: &File, _name: &OsStr) -> Result<File, SecureFsError> {
        unsupported()
    }

    pub fn open_or_create_lock_file(_parent: &File, _name: &OsStr) -> Result<File, SecureFsError> {
        unsupported()
    }

    pub fn publish_child_noreplace(
        _source: &File,
        _source_name: &OsStr,
        _destination: &File,
        _destination_name: &OsStr,
    ) -> Result<(), SecureFsError> {
        unsupported()
    }

    pub fn unlink_child(_parent: &File, _name: &OsStr) -> Result<(), SecureFsError> {
        unsupported()
    }

    pub fn has_single_link(file: &File, _metadata: &Metadata) -> Result<bool, SecureFsError> {
        let mut information = BY_HANDLE_FILE_INFORMATION::default();
        // SAFETY: the structure is writable and the handle is borrowed from a
        // live file for this synchronous metadata query.
        if unsafe { GetFileInformationByHandle(file.as_raw_handle(), &raw mut information) } == 0 {
            return Err(io_error(
                "query Windows child link count",
                std::io::Error::last_os_error(),
            ));
        }
        Ok(information.nNumberOfLinks == 1)
    }

    fn unsupported<T>() -> Result<T, SecureFsError> {
        Err(SecureFsError::new(
            SecureFsErrorCode::UnsupportedOperation,
            "Windows root-relative mutation requires reviewed NT handle semantics",
        ))
    }
}

#[cfg(not(any(unix, windows)))]
mod platform {
    use std::{
        ffi::OsStr,
        fs::{File, Metadata},
        path::Path,
    };

    use super::{SecureFsError, SecureFsErrorCode};

    pub fn open_directory(_path: &Path) -> Result<File, SecureFsError> {
        unsupported()
    }
    pub fn open_child_directory(_parent: &File, _name: &OsStr) -> Result<File, SecureFsError> {
        unsupported()
    }
    pub fn create_child_directory(_parent: &File, _name: &OsStr) -> Result<(), SecureFsError> {
        unsupported()
    }
    pub fn create_new_child_file(_parent: &File, _name: &OsStr) -> Result<File, SecureFsError> {
        unsupported()
    }
    pub fn open_child_file(_parent: &File, _name: &OsStr) -> Result<File, SecureFsError> {
        unsupported()
    }
    pub fn open_or_create_lock_file(_parent: &File, _name: &OsStr) -> Result<File, SecureFsError> {
        unsupported()
    }
    pub fn publish_child_noreplace(
        _source: &File,
        _source_name: &OsStr,
        _destination: &File,
        _destination_name: &OsStr,
    ) -> Result<(), SecureFsError> {
        unsupported()
    }
    pub fn unlink_child(_parent: &File, _name: &OsStr) -> Result<(), SecureFsError> {
        unsupported()
    }
    pub fn has_single_link(_file: &File, _metadata: &Metadata) -> Result<bool, SecureFsError> {
        unsupported()
    }
    fn unsupported<T>() -> Result<T, SecureFsError> {
        Err(SecureFsError::new(
            SecureFsErrorCode::UnsupportedOperation,
            "platform is unsupported",
        ))
    }
}
