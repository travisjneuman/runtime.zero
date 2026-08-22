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
    NotFound,
    AlreadyExists,
    LockBusy,
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
            SecureFsErrorCode::NotFound => Foundation::IoUnavailable,
            SecureFsErrorCode::AlreadyExists | SecureFsErrorCode::LockBusy => Foundation::Conflict,
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

pub struct SecureFileLock {
    file: File,
    state: platform::LockState,
}

impl SecureFileLock {
    pub fn try_exclusive(file: File) -> Result<Self, SecureFsError> {
        let state = platform::try_lock_exclusive(&file)?;
        Ok(Self { file, state })
    }

    pub fn file(&self) -> &File {
        &self.file
    }
}

impl Drop for SecureFileLock {
    fn drop(&mut self) {
        platform::unlock(&self.file, &mut self.state);
    }
}

impl SecureOpenedFile {
    pub fn verify_private(&self) -> Result<(), SecureFsError> {
        platform::verify_private_file(&self.file, &self.metadata)
    }

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
    /// Duplicates the held directory handle without re-resolving its path.
    ///
    /// Callers use this when walking separate root-relative paths. The clone
    /// preserves the no-follow and opened-directory guarantees of the
    /// original handle.
    pub fn try_clone(&self) -> Result<Self, SecureFsError> {
        self.file
            .try_clone()
            .map(|file| Self { file })
            .map_err(|error| io_error("clone held directory handle", error))
    }

    pub fn open(path: &Path) -> Result<Self, SecureFsError> {
        platform::open_directory(path).map(|file| Self { file })
    }

    pub fn verify_private(&self) -> Result<(), SecureFsError> {
        let metadata = self
            .file
            .metadata()
            .map_err(|error| io_error("inspect opened directory privacy", error))?;
        platform::verify_private_directory(&self.file, &metadata)
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

    /// Opens a normal child directory, creating it privately when absent.
    ///
    /// The create-first ordering preserves a useful distinction on platforms
    /// whose no-follow directory-open primitive reports a missing child as a
    /// generic unsafe-directory error.
    pub fn open_or_create_child_directory(&self, name: &OsStr) -> Result<Self, SecureFsError> {
        validate_child_name(name)?;
        match platform::create_child_directory(&self.file, name) {
            Ok(()) => {
                let opened = self.open_child_directory(name)?;
                self.sync()?;
                Ok(opened)
            }
            Err(error) if error.code == SecureFsErrorCode::AlreadyExists => {
                self.open_child_directory(name)
            }
            Err(error) => Err(error),
        }
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
        let opened = SecureOpenedFile { file, metadata };
        opened.verify_private()?;
        Ok(opened)
    }

    /// Marks one already-created regular child as owner-executable without
    /// reopening its path through an unrestricted filesystem lookup.
    pub fn mark_child_executable(&self, name: &OsStr) -> Result<(), SecureFsError> {
        validate_child_name(name)?;
        platform::mark_child_executable(&self.file, name)?;
        self.sync()
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
        let metadata = checked_regular_metadata(&file)?;
        platform::verify_private_file(&file, &metadata)?;
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
        let pending_link_remains = platform::publish_child_noreplace(
            &self.file,
            pending_name,
            &destination.file,
            destination_name,
        )?;
        destination.sync()?;
        if pending_link_remains {
            platform::unlink_child(&self.file, pending_name).map_err(|error| {
                SecureFsError::new(
                    SecureFsErrorCode::PublicationIncomplete,
                    format!("published child but could not retire pending link: {error}"),
                )
            })?;
        }
        self.sync()?;
        destination.open_child_file(destination_name)
    }

    /// Atomically replaces one destination name with a complete pending file.
    /// Policy callers must retain and verify rollback evidence before invoking
    /// this primitive; the filesystem operation itself grants no authority.
    pub fn replace_child_atomic(
        &self,
        pending_name: &OsStr,
        destination: &SecureDirectory,
        destination_name: &OsStr,
    ) -> Result<SecureOpenedFile, SecureFsError> {
        validate_child_name(pending_name)?;
        validate_child_name(destination_name)?;
        self.open_child_file(pending_name)?;
        platform::replace_child_atomic(
            &self.file,
            pending_name,
            &destination.file,
            destination_name,
        )?;
        destination.sync()?;
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
    if !platform::is_regular_file(file, &metadata)? || !platform::has_single_link(file, &metadata)?
    {
        return Err(SecureFsError::new(
            SecureFsErrorCode::IdentityChanged,
            "opened child is not a single-link regular file",
        ));
    }
    Ok(metadata)
}

fn io_error(context: &str, error: std::io::Error) -> SecureFsError {
    let code = match error.kind() {
        std::io::ErrorKind::NotFound => SecureFsErrorCode::NotFound,
        std::io::ErrorKind::AlreadyExists => SecureFsErrorCode::AlreadyExists,
        _ => SecureFsErrorCode::Io,
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

    #[derive(Debug)]
    pub struct LockState;

    pub fn try_lock_exclusive(file: &File) -> Result<LockState, SecureFsError> {
        // SAFETY: the descriptor remains owned by SecureFileLock for the lock lifetime.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            Ok(LockState)
        } else {
            let error = std::io::Error::last_os_error();
            let raw = error.raw_os_error();
            let code = if raw == Some(libc::EWOULDBLOCK) || raw == Some(libc::EAGAIN) {
                SecureFsErrorCode::LockBusy
            } else {
                SecureFsErrorCode::Io
            };
            Err(SecureFsError::new(
                code,
                format!("lock opened file: {error}"),
            ))
        }
    }

    pub fn unlock(file: &File, _state: &mut LockState) {
        // SAFETY: the descriptor is live until SecureFileLock finishes dropping.
        let _ = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
    }

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

    pub fn mark_child_executable(parent: &File, name: &OsStr) -> Result<(), SecureFsError> {
        let name = child_c_string(name)?;
        // SAFETY: parent is a held directory and name is one validated
        // component. The descriptor is owned immediately by File.
        let descriptor = unsafe {
            libc::openat(
                parent.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if descriptor == -1 {
            return Err(io_error(
                "open child for executable permission",
                std::io::Error::last_os_error(),
            ));
        }
        // SAFETY: descriptor was opened successfully and is now owned by File.
        let file = unsafe { File::from_raw_fd(descriptor) };
        if unsafe { libc::fchmod(file.as_raw_fd(), 0o700) } != 0 {
            return Err(io_error(
                "mark child executable",
                std::io::Error::last_os_error(),
            ));
        }
        file.sync_all()
            .map_err(|error| io_error("sync executable child", error))
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
    ) -> Result<bool, SecureFsError> {
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
            Ok(true)
        } else {
            Err(io_error(
                "publish child without replacement",
                std::io::Error::last_os_error(),
            ))
        }
    }

    pub fn replace_child_atomic(
        source: &File,
        source_name: &OsStr,
        destination: &File,
        destination_name: &OsStr,
    ) -> Result<(), SecureFsError> {
        let source_name = child_c_string(source_name)?;
        let destination_name = child_c_string(destination_name)?;
        // SAFETY: both descriptors are held directories and both names are one
        // validated component. renameat atomically replaces the destination.
        if unsafe {
            libc::renameat(
                source.as_raw_fd(),
                source_name.as_ptr(),
                destination.as_raw_fd(),
                destination_name.as_ptr(),
            )
        } == 0
        {
            Ok(())
        } else {
            Err(io_error(
                "atomically replace child",
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

    pub fn verify_private_directory(
        _file: &File,
        metadata: &Metadata,
    ) -> Result<(), SecureFsError> {
        verify_unix_privacy(metadata, true)
    }

    pub fn verify_private_file(_file: &File, metadata: &Metadata) -> Result<(), SecureFsError> {
        verify_unix_privacy(metadata, false)
    }

    pub fn is_regular_file(_file: &File, metadata: &Metadata) -> Result<bool, SecureFsError> {
        Ok(metadata.is_file())
    }

    pub fn has_single_link(_file: &File, metadata: &Metadata) -> Result<bool, SecureFsError> {
        use std::os::unix::fs::MetadataExt;
        Ok(metadata.nlink() == 1)
    }

    fn verify_unix_privacy(metadata: &Metadata, directory: bool) -> Result<(), SecureFsError> {
        use std::os::unix::fs::MetadataExt;

        let expected_type = if directory {
            metadata.is_dir()
        } else {
            metadata.is_file()
        };
        // SAFETY: geteuid has no preconditions.
        let owned = metadata.uid() == unsafe { libc::geteuid() };
        if expected_type && owned && metadata.mode() & 0o077 == 0 {
            Ok(())
        } else {
            Err(SecureFsError::new(
                SecureFsErrorCode::UnsafeDirectory,
                "opened Unix object is not private to the effective user",
            ))
        }
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
        mem::{offset_of, size_of},
        os::windows::{
            ffi::OsStrExt,
            fs::{MetadataExt, OpenOptionsExt},
            io::{AsRawHandle, FromRawHandle, OwnedHandle},
        },
        path::Path,
        ptr::{copy_nonoverlapping, null, null_mut},
    };

    use windows_sys::{
        Wdk::{
            Foundation::OBJECT_ATTRIBUTES,
            Storage::FileSystem::{
                FILE_CREATE, FILE_DIRECTORY_FILE, FILE_DISPOSITION_INFORMATION,
                FILE_NON_DIRECTORY_FILE, FILE_OPEN, FILE_OPEN_FOR_BACKUP_INTENT, FILE_OPEN_IF,
                FILE_OPEN_REPARSE_POINT, FILE_RENAME_INFORMATION, FILE_SYNCHRONOUS_IO_NONALERT,
                FileDispositionInformation, FileRenameInformation, NtCreateFile,
                NtSetInformationFile,
            },
        },
        Win32::{
            Foundation::{
                HANDLE, INVALID_HANDLE_VALUE, LocalFree, OBJ_CASE_INSENSITIVE,
                RtlNtStatusToDosError, UNICODE_STRING,
            },
            Security::{
                ACCESS_ALLOWED_ACE, ACL, ACL_SIZE_INFORMATION, AclSizeInformation,
                Authorization::{
                    EXPLICIT_ACCESS_W, GRANT_ACCESS, GetSecurityInfo, NO_MULTIPLE_TRUSTEE,
                    SE_FILE_OBJECT, SetEntriesInAclW, SetSecurityInfo, TRUSTEE_IS_SID,
                    TRUSTEE_IS_USER, TRUSTEE_IS_WELL_KNOWN_GROUP, TRUSTEE_W,
                },
                CreateWellKnownSid, DACL_SECURITY_INFORMATION, EqualSid, GetAce, GetAclInformation,
                GetTokenInformation, OWNER_SECURITY_INFORMATION,
                PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
                SECURITY_MAX_SID_SIZE, TOKEN_QUERY, TOKEN_USER, TokenUser,
                WinBuiltinAdministratorsSid, WinLocalSystemSid,
            },
            Storage::FileSystem::{
                BY_HANDLE_FILE_INFORMATION, DELETE, FILE_ATTRIBUTE_NORMAL,
                FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT, FILE_GENERIC_READ,
                FILE_GENERIC_WRITE, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
                GetFileInformationByHandle, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
                LockFileEx, UnlockFileEx, WRITE_DAC, WRITE_OWNER,
            },
            System::{
                IO::IO_STATUS_BLOCK,
                SystemServices::{ACCESS_ALLOWED_ACE_TYPE, ACCESS_DENIED_ACE_TYPE},
                Threading::{GetCurrentProcess, OpenProcessToken},
            },
        },
    };

    use super::{SecureFsError, SecureFsErrorCode, io_error};

    pub struct LockState;

    pub fn try_lock_exclusive(file: &File) -> Result<LockState, SecureFsError> {
        let mut overlapped =
            std::mem::MaybeUninit::<windows_sys::Win32::System::IO::OVERLAPPED>::zeroed();
        // SAFETY: the handle is live and this non-overlapped lock request
        // completes synchronously before the OVERLAPPED storage is released.
        if unsafe {
            LockFileEx(
                file.as_raw_handle(),
                LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
                0,
                u32::MAX,
                u32::MAX,
                overlapped.as_mut_ptr(),
            )
        } != 0
        {
            Ok(LockState)
        } else {
            let error = std::io::Error::last_os_error();
            let code = if error.raw_os_error() == Some(33) {
                SecureFsErrorCode::LockBusy
            } else {
                SecureFsErrorCode::Io
            };
            Err(SecureFsError::new(
                code,
                format!("lock opened file: {error}"),
            ))
        }
    }

    pub fn unlock(file: &File, _state: &mut LockState) {
        let mut overlapped =
            std::mem::MaybeUninit::<windows_sys::Win32::System::IO::OVERLAPPED>::zeroed();
        // SAFETY: releases only the byte range locked by this live handle.
        let _ = unsafe {
            UnlockFileEx(
                file.as_raw_handle(),
                0,
                u32::MAX,
                u32::MAX,
                overlapped.as_mut_ptr(),
            )
        };
    }

    const SHARE_ALL: u32 = FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE;
    const DIRECTORY_OPTIONS: u32 = FILE_DIRECTORY_FILE
        | FILE_OPEN_REPARSE_POINT
        | FILE_OPEN_FOR_BACKUP_INTENT
        | FILE_SYNCHRONOUS_IO_NONALERT;
    const FILE_OPTIONS: u32 =
        FILE_NON_DIRECTORY_FILE | FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT;

    pub fn open_directory(path: &Path) -> Result<File, SecureFsError> {
        let mut options = OpenOptions::new();
        options
            .access_mode(FILE_GENERIC_READ | FILE_GENERIC_WRITE)
            .share_mode(SHARE_ALL)
            .custom_flags(FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT);
        let file = options
            .open(path)
            .map_err(|error| io_error("open Windows directory handle", error))?;
        validate_directory(&file)?;
        Ok(file)
    }

    pub fn open_child_directory(parent: &File, name: &OsStr) -> Result<File, SecureFsError> {
        let file = nt_open_child(
            parent,
            name,
            FILE_GENERIC_READ | FILE_GENERIC_WRITE,
            FILE_OPEN,
            DIRECTORY_OPTIONS,
            "open Windows child directory relative to held parent",
        )?;
        validate_directory(&file)?;
        Ok(file)
    }

    pub fn create_child_directory(parent: &File, name: &OsStr) -> Result<(), SecureFsError> {
        let file = nt_open_child(
            parent,
            name,
            FILE_GENERIC_READ | FILE_GENERIC_WRITE | DELETE | WRITE_DAC | WRITE_OWNER,
            FILE_CREATE,
            DIRECTORY_OPTIONS,
            "create Windows child directory relative to held parent",
        )?;
        set_private_security(&file)?;
        validate_directory(&file)
    }

    pub fn create_new_child_file(parent: &File, name: &OsStr) -> Result<File, SecureFsError> {
        let file = nt_open_child(
            parent,
            name,
            FILE_GENERIC_READ | FILE_GENERIC_WRITE | DELETE | WRITE_DAC | WRITE_OWNER,
            FILE_CREATE,
            FILE_OPTIONS,
            "create Windows child file relative to held parent",
        )?;
        set_private_security(&file)?;
        Ok(file)
    }

    pub fn open_child_file(parent: &File, name: &OsStr) -> Result<File, SecureFsError> {
        nt_open_child(
            parent,
            name,
            FILE_GENERIC_READ,
            FILE_OPEN,
            FILE_OPTIONS,
            "open Windows child file relative to held parent",
        )
    }

    pub fn mark_child_executable(_parent: &File, _name: &OsStr) -> Result<(), SecureFsError> {
        Ok(())
    }

    pub fn open_or_create_lock_file(parent: &File, name: &OsStr) -> Result<File, SecureFsError> {
        nt_open_child(
            parent,
            name,
            FILE_GENERIC_READ | FILE_GENERIC_WRITE,
            FILE_OPEN_IF,
            FILE_OPTIONS,
            "open Windows lock file relative to held parent",
        )
    }

    pub fn publish_child_noreplace(
        source: &File,
        source_name: &OsStr,
        destination: &File,
        destination_name: &OsStr,
    ) -> Result<bool, SecureFsError> {
        rename_child(
            source,
            source_name,
            destination,
            destination_name,
            false,
            "publish Windows child without replacement",
        )?;
        Ok(false)
    }

    pub fn replace_child_atomic(
        source: &File,
        source_name: &OsStr,
        destination: &File,
        destination_name: &OsStr,
    ) -> Result<(), SecureFsError> {
        rename_child(
            source,
            source_name,
            destination,
            destination_name,
            true,
            "atomically replace Windows child",
        )
    }

    pub fn unlink_child(parent: &File, name: &OsStr) -> Result<(), SecureFsError> {
        let child = nt_open_child(
            parent,
            name,
            DELETE,
            FILE_OPEN,
            FILE_OPTIONS,
            "open Windows child for root-relative unlink",
        )?;
        let disposition = FILE_DISPOSITION_INFORMATION { DeleteFile: true };
        let mut status_block = IO_STATUS_BLOCK::default();
        // SAFETY: child is a live DELETE-capable handle and disposition has the
        // exact fixed-size layout required by FileDispositionInformation.
        let status = unsafe {
            NtSetInformationFile(
                child.as_raw_handle(),
                &raw mut status_block,
                (&raw const disposition).cast(),
                size_of::<FILE_DISPOSITION_INFORMATION>() as u32,
                FileDispositionInformation,
            )
        };
        nt_result(status, "unlink Windows child relative to held parent")
    }

    pub fn verify_private_directory(
        file: &File,
        _metadata: &Metadata,
    ) -> Result<(), SecureFsError> {
        verify_windows_private(file)
    }

    pub fn verify_private_file(file: &File, _metadata: &Metadata) -> Result<(), SecureFsError> {
        verify_windows_private(file)
    }

    fn verify_windows_private(file: &File) -> Result<(), SecureFsError> {
        const MAX_PRIVATE_ACES: u32 = 256;

        let token = current_process_token()?;
        let mut required = 0u32;
        // SAFETY: the null probe requests only the required byte length.
        let _ = unsafe {
            GetTokenInformation(
                token.as_raw_handle(),
                TokenUser,
                null_mut(),
                0,
                &raw mut required,
            )
        };
        if required == 0 {
            return Err(io_error(
                "size Windows token user information",
                std::io::Error::last_os_error(),
            ));
        }
        let token_words = (required as usize).div_ceil(size_of::<usize>());
        let mut token_buffer = vec![0usize; token_words];
        // SAFETY: the aligned buffer is at least the byte count returned by the
        // sizing call and remains live while its embedded SID is inspected.
        if unsafe {
            GetTokenInformation(
                token.as_raw_handle(),
                TokenUser,
                token_buffer.as_mut_ptr().cast(),
                required,
                &raw mut required,
            )
        } == 0
        {
            return Err(io_error(
                "read Windows token user information",
                std::io::Error::last_os_error(),
            ));
        }
        // SAFETY: successful TokenUser output begins with TOKEN_USER.
        let user_sid = unsafe { (*(token_buffer.as_ptr().cast::<TOKEN_USER>())).User.Sid };
        if user_sid.is_null() {
            return Err(private_policy_error("Windows token user SID is absent"));
        }

        let system_sid = well_known_sid(WinLocalSystemSid)?;
        let administrators_sid = well_known_sid(WinBuiltinAdministratorsSid)?;
        let mut owner: PSID = null_mut();
        let mut dacl: *mut ACL = null_mut();
        let mut descriptor: PSECURITY_DESCRIPTOR = null_mut();
        // SAFETY: the file handle is live and output pointers remain valid until
        // the LocalFree-owned security descriptor guard is dropped.
        let result = unsafe {
            GetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
                &raw mut owner,
                null_mut(),
                &raw mut dacl,
                null_mut(),
                &raw mut descriptor,
            )
        };
        if result != 0 {
            return Err(io_error(
                "query Windows owner and DACL",
                std::io::Error::from_raw_os_error(result as i32),
            ));
        }
        let _descriptor = LocalDescriptor(descriptor);
        if owner.is_null() || dacl.is_null() {
            return Err(private_policy_error(
                "Windows private object requires an owner and non-null DACL",
            ));
        }
        // SAFETY: all SIDs are live and came from successful Windows APIs.
        if unsafe { EqualSid(owner, user_sid) } == 0 {
            return Err(private_policy_error(
                "Windows private object owner does not match the process user",
            ));
        }

        let mut information = ACL_SIZE_INFORMATION::default();
        // SAFETY: dacl belongs to the live descriptor and information has the
        // exact ACL_SIZE_INFORMATION layout.
        if unsafe {
            GetAclInformation(
                dacl,
                (&raw mut information).cast(),
                size_of::<ACL_SIZE_INFORMATION>() as u32,
                AclSizeInformation,
            )
        } == 0
        {
            return Err(io_error(
                "query Windows DACL size information",
                std::io::Error::last_os_error(),
            ));
        }
        if information.AceCount == 0 || information.AceCount > MAX_PRIVATE_ACES {
            return Err(private_policy_error(
                "Windows private DACL has an empty or excessive ACE set",
            ));
        }

        let mut user_allow_present = false;
        for index in 0..information.AceCount {
            let mut raw_ace = null_mut();
            // SAFETY: index is below the ACE count reported for this live DACL.
            if unsafe { GetAce(dacl, index, &raw mut raw_ace) } == 0 || raw_ace.is_null() {
                return Err(io_error(
                    "read Windows DACL ACE",
                    std::io::Error::last_os_error(),
                ));
            }
            // SAFETY: every ACE starts with an ACE_HEADER, and GetAce returned a
            // pointer owned by the live security descriptor.
            let ace_type =
                unsafe { (*(raw_ace.cast::<windows_sys::Win32::Security::ACE_HEADER>())).AceType };
            if u32::from(ace_type) == ACCESS_DENIED_ACE_TYPE {
                continue;
            }
            if u32::from(ace_type) != ACCESS_ALLOWED_ACE_TYPE {
                return Err(private_policy_error(
                    "Windows private DACL contains an unsupported ACE type",
                ));
            }
            let allowed = raw_ace.cast::<ACCESS_ALLOWED_ACE>();
            // SAFETY: ACCESS_ALLOWED_ACE stores the variable SID beginning at
            // SidStart and the descriptor remains live for the comparison.
            let sid = unsafe { (&raw mut (*allowed).SidStart).cast() };
            // SAFETY: the compared SIDs are live and validated by Windows.
            let is_user = unsafe { EqualSid(sid, user_sid) } != 0;
            let is_system = unsafe { EqualSid(sid, system_sid.as_ptr().cast_mut().cast()) } != 0;
            let is_administrator =
                unsafe { EqualSid(sid, administrators_sid.as_ptr().cast_mut().cast()) } != 0;
            if !(is_user || is_system || is_administrator) {
                return Err(private_policy_error(
                    "Windows private DACL grants access outside user, SYSTEM, or Administrators",
                ));
            }
            user_allow_present |= is_user;
        }
        if !user_allow_present {
            return Err(private_policy_error(
                "Windows private DACL does not grant the process user access",
            ));
        }
        Ok(())
    }

    fn set_private_security(file: &File) -> Result<(), SecureFsError> {
        let token = current_process_token()?;
        let mut required = 0u32;
        // SAFETY: the null probe requests only the required byte length.
        let _ = unsafe {
            GetTokenInformation(
                token.as_raw_handle(),
                TokenUser,
                null_mut(),
                0,
                &raw mut required,
            )
        };
        if required == 0 {
            return Err(io_error(
                "size Windows token user information for private creation",
                std::io::Error::last_os_error(),
            ));
        }
        let token_words = (required as usize).div_ceil(size_of::<usize>());
        let mut token_buffer = vec![0usize; token_words];
        // SAFETY: the aligned buffer is at least the byte count returned by the
        // sizing call and remains live while its embedded SID is inspected.
        if unsafe {
            GetTokenInformation(
                token.as_raw_handle(),
                TokenUser,
                token_buffer.as_mut_ptr().cast(),
                required,
                &raw mut required,
            )
        } == 0
        {
            return Err(io_error(
                "read Windows token user information for private creation",
                std::io::Error::last_os_error(),
            ));
        }
        // SAFETY: successful TokenUser output begins with TOKEN_USER.
        let user_sid = unsafe { (*(token_buffer.as_ptr().cast::<TOKEN_USER>())).User.Sid };
        if user_sid.is_null() {
            return Err(private_policy_error(
                "Windows token user SID is absent during private creation",
            ));
        }
        let system_sid = well_known_sid(WinLocalSystemSid)?;
        let administrators_sid = well_known_sid(WinBuiltinAdministratorsSid)?;
        let entries = [
            private_access_entry(user_sid, TRUSTEE_IS_USER),
            private_access_entry(
                system_sid.as_ptr().cast_mut().cast(),
                TRUSTEE_IS_WELL_KNOWN_GROUP,
            ),
            private_access_entry(
                administrators_sid.as_ptr().cast_mut().cast(),
                TRUSTEE_IS_WELL_KNOWN_GROUP,
            ),
        ];
        let mut dacl: *mut ACL = null_mut();
        // SAFETY: each trustee SID and the entry array remain live through the
        // synchronous ACL construction call; oldacl is null because creation
        // intentionally replaces inherited permissions.
        let status = unsafe {
            SetEntriesInAclW(
                entries.len() as u32,
                entries.as_ptr(),
                null(),
                &raw mut dacl,
            )
        };
        if status != 0 {
            return Err(io_error(
                "construct Windows private DACL",
                std::io::Error::from_raw_os_error(status as i32),
            ));
        }
        if dacl.is_null() {
            return Err(private_policy_error(
                "Windows private DACL construction returned no ACL",
            ));
        }
        // SAFETY: dacl is a LocalAlloc-owned ACL returned by SetEntriesInAclW
        // and is released after the synchronous SetSecurityInfo call.
        let status = unsafe {
            SetSecurityInfo(
                file.as_raw_handle(),
                SE_FILE_OBJECT,
                OWNER_SECURITY_INFORMATION
                    | DACL_SECURITY_INFORMATION
                    | PROTECTED_DACL_SECURITY_INFORMATION,
                user_sid,
                null_mut(),
                dacl,
                null_mut(),
            )
        };
        // SAFETY: SetEntriesInAclW allocated this ACL with the Windows local
        // allocator, and no pointer escapes this function.
        unsafe {
            let _ = LocalFree(dacl.cast());
        }
        if status != 0 {
            return Err(io_error(
                "apply Windows private owner and DACL",
                std::io::Error::from_raw_os_error(status as i32),
            ));
        }
        Ok(())
    }

    fn private_access_entry(sid: PSID, trustee_type: i32) -> EXPLICIT_ACCESS_W {
        EXPLICIT_ACCESS_W {
            grfAccessPermissions: windows_sys::Win32::Foundation::GENERIC_ALL,
            grfAccessMode: GRANT_ACCESS,
            grfInheritance: windows_sys::Win32::Security::NO_INHERITANCE,
            Trustee: TRUSTEE_W {
                pMultipleTrustee: null_mut(),
                MultipleTrusteeOperation: NO_MULTIPLE_TRUSTEE,
                TrusteeForm: TRUSTEE_IS_SID,
                TrusteeType: trustee_type,
                ptstrName: sid.cast(),
            },
        }
    }

    fn current_process_token() -> Result<OwnedHandle, SecureFsError> {
        let mut token: HANDLE = INVALID_HANDLE_VALUE;
        // SAFETY: GetCurrentProcess returns the current pseudo-handle and the
        // successful token handle transfers immediately to OwnedHandle.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &raw mut token) } == 0 {
            return Err(io_error(
                "open Windows process token",
                std::io::Error::last_os_error(),
            ));
        }
        if token.is_null() || token == INVALID_HANDLE_VALUE {
            return Err(private_policy_error(
                "Windows process token API returned an invalid handle",
            ));
        }
        // SAFETY: OpenProcessToken returned one owned valid handle.
        Ok(unsafe { OwnedHandle::from_raw_handle(token) })
    }

    fn well_known_sid(kind: i32) -> Result<Vec<u8>, SecureFsError> {
        let mut sid = vec![0u8; SECURITY_MAX_SID_SIZE as usize];
        let mut bytes = SECURITY_MAX_SID_SIZE;
        // SAFETY: the output buffer is SECURITY_MAX_SID_SIZE bytes and the null
        // domain SID is valid for the requested well-known local SIDs.
        if unsafe { CreateWellKnownSid(kind, null_mut(), sid.as_mut_ptr().cast(), &raw mut bytes) }
            == 0
        {
            return Err(io_error(
                "create Windows well-known SID",
                std::io::Error::last_os_error(),
            ));
        }
        sid.truncate(bytes as usize);
        Ok(sid)
    }

    fn private_policy_error(detail: &'static str) -> SecureFsError {
        SecureFsError::new(SecureFsErrorCode::UnsafeDirectory, detail)
    }

    struct LocalDescriptor(PSECURITY_DESCRIPTOR);

    impl Drop for LocalDescriptor {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: GetSecurityInfo allocated this descriptor with LocalAlloc.
                let _ = unsafe { LocalFree(self.0) };
            }
        }
    }

    pub fn is_regular_file(_file: &File, metadata: &Metadata) -> Result<bool, SecureFsError> {
        Ok(metadata.is_file() && metadata.file_attributes() & 0x0400 == 0)
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

    fn rename_child(
        source: &File,
        source_name: &OsStr,
        destination: &File,
        destination_name: &OsStr,
        replace: bool,
        context: &str,
    ) -> Result<(), SecureFsError> {
        let pending = nt_open_child(
            source,
            source_name,
            FILE_GENERIC_READ | DELETE,
            FILE_OPEN,
            FILE_OPTIONS,
            "open Windows pending file for atomic publication",
        )?;
        let metadata = pending
            .metadata()
            .map_err(|error| io_error("inspect Windows pending file", error))?;
        if !is_regular_file(&pending, &metadata)? || !has_single_link(&pending, &metadata)? {
            return Err(SecureFsError::new(
                SecureFsErrorCode::IdentityChanged,
                "Windows pending file changed identity before publication",
            ));
        }
        let destination_name = wide_child(destination_name)?;
        let name_bytes = destination_name
            .len()
            .checked_mul(size_of::<u16>())
            .ok_or_else(|| {
                SecureFsError::new(
                    SecureFsErrorCode::LimitExceeded,
                    "Windows child name is too long",
                )
            })?;
        let header_bytes = offset_of!(FILE_RENAME_INFORMATION, FileName);
        let information_bytes = header_bytes.checked_add(name_bytes).ok_or_else(|| {
            SecureFsError::new(
                SecureFsErrorCode::LimitExceeded,
                "Windows rename buffer is too large",
            )
        })?;
        let words = information_bytes.div_ceil(size_of::<usize>());
        let mut buffer = vec![0_usize; words];
        let information = buffer.as_mut_ptr().cast::<FILE_RENAME_INFORMATION>();
        // SAFETY: the usize buffer is aligned, large enough for the fixed header
        // and validated UTF-16 name, and remains live through the synchronous call.
        unsafe {
            (*information).Anonymous.ReplaceIfExists = replace;
            (*information).RootDirectory = destination.as_raw_handle();
            (*information).FileNameLength = name_bytes as u32;
            copy_nonoverlapping(
                destination_name.as_ptr(),
                (*information).FileName.as_mut_ptr(),
                destination_name.len(),
            );
        }
        let mut status_block = IO_STATUS_BLOCK::default();
        // SAFETY: pending and destination are live handles and the information
        // buffer has the exact FILE_RENAME_INFORMATION layout.
        let status = unsafe {
            NtSetInformationFile(
                pending.as_raw_handle(),
                &raw mut status_block,
                information.cast(),
                information_bytes as u32,
                FileRenameInformation,
            )
        };
        nt_result(status, context)
    }

    fn nt_open_child(
        parent: &File,
        name: &OsStr,
        desired_access: u32,
        disposition: u32,
        options: u32,
        context: &str,
    ) -> Result<File, SecureFsError> {
        let mut wide_name = wide_child(name)?;
        let byte_length = wide_name
            .len()
            .checked_mul(size_of::<u16>())
            .ok_or_else(|| {
                SecureFsError::new(
                    SecureFsErrorCode::LimitExceeded,
                    "Windows child name is too long",
                )
            })?;
        let length = u16::try_from(byte_length).map_err(|_| {
            SecureFsError::new(
                SecureFsErrorCode::LimitExceeded,
                "Windows child name is too long",
            )
        })?;
        let unicode_name = UNICODE_STRING {
            Length: length,
            MaximumLength: length,
            Buffer: wide_name.as_mut_ptr(),
        };
        let attributes = OBJECT_ATTRIBUTES {
            Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
            RootDirectory: parent.as_raw_handle(),
            ObjectName: &raw const unicode_name,
            Attributes: OBJ_CASE_INSENSITIVE,
            SecurityDescriptor: null(),
            SecurityQualityOfService: null(),
        };
        let mut handle: HANDLE = INVALID_HANDLE_VALUE;
        let mut status_block = IO_STATUS_BLOCK::default();
        // SAFETY: all pointers reference live stack/vector storage for this
        // synchronous call; successful handle ownership moves directly to File.
        let status = unsafe {
            NtCreateFile(
                &raw mut handle,
                desired_access,
                &raw const attributes,
                &raw mut status_block,
                null(),
                FILE_ATTRIBUTE_NORMAL,
                SHARE_ALL,
                disposition,
                options,
                null(),
                0,
            )
        };
        nt_result(status, context)?;
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            return Err(SecureFsError::new(
                SecureFsErrorCode::Io,
                format!("{context}: NT returned an invalid handle"),
            ));
        }
        // SAFETY: NtCreateFile returned one owned valid handle.
        Ok(unsafe { File::from_raw_handle(handle) })
    }

    fn validate_directory(file: &File) -> Result<(), SecureFsError> {
        let metadata = file
            .metadata()
            .map_err(|error| io_error("inspect Windows directory handle", error))?;
        if metadata.is_dir() && metadata.file_attributes() & 0x0400 == 0 {
            Ok(())
        } else {
            Err(SecureFsError::new(
                SecureFsErrorCode::UnsafeDirectory,
                "Windows directory is reparse-backed or the wrong type",
            ))
        }
    }

    fn wide_child(name: &OsStr) -> Result<Vec<u16>, SecureFsError> {
        let wide: Vec<u16> = name.encode_wide().collect();
        if wide.is_empty() || wide.contains(&0) {
            Err(SecureFsError::new(
                SecureFsErrorCode::UnsafeName,
                "Windows child name is empty or contains a null code unit",
            ))
        } else {
            Ok(wide)
        }
    }

    fn nt_result(status: i32, context: &str) -> Result<(), SecureFsError> {
        if status >= 0 {
            return Ok(());
        }
        // SAFETY: conversion is pure for any NTSTATUS value.
        let code = unsafe { RtlNtStatusToDosError(status) };
        Err(io_error(
            context,
            std::io::Error::from_raw_os_error(code as i32),
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

    pub struct LockState;
    pub fn try_lock_exclusive(_file: &File) -> Result<LockState, SecureFsError> {
        unsupported()
    }
    pub fn unlock(_file: &File, _state: &mut LockState) {}

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
    pub fn mark_child_executable(_parent: &File, _name: &OsStr) -> Result<(), SecureFsError> {
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
    ) -> Result<bool, SecureFsError> {
        unsupported()
    }
    pub fn replace_child_atomic(
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
    pub fn verify_private_directory(
        _file: &File,
        _metadata: &Metadata,
    ) -> Result<(), SecureFsError> {
        unsupported()
    }
    pub fn verify_private_file(_file: &File, _metadata: &Metadata) -> Result<(), SecureFsError> {
        unsupported()
    }
    pub fn is_regular_file(_file: &File, _metadata: &Metadata) -> Result<bool, SecureFsError> {
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
