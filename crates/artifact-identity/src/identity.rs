use std::fs::{File, Metadata};

#[cfg(windows)]
use std::io;

use crate::{ArtifactFileIdentity, ArtifactIdentityError};

#[cfg(windows)]
use crate::ArtifactIdentityErrorCode;

#[cfg(unix)]
pub(crate) fn identity_from_file(
    _file: &File,
    metadata: &Metadata,
) -> Result<ArtifactFileIdentity, ArtifactIdentityError> {
    use std::os::unix::fs::MetadataExt;

    Ok(ArtifactFileIdentity::Unix {
        device: metadata.dev(),
        inode: metadata.ino(),
        link_count: metadata.nlink(),
    })
}

#[cfg(windows)]
pub(crate) fn identity_from_file(
    file: &File,
    _metadata: &Metadata,
) -> Result<ArtifactFileIdentity, ArtifactIdentityError> {
    use std::{mem::MaybeUninit, os::windows::io::AsRawHandle};
    use windows_sys::Win32::Storage::FileSystem::{
        BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle,
    };

    let mut information = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::zeroed();
    // SAFETY: information points to writable initialized storage and the handle
    // is borrowed from a live File for the duration of this read-only query.
    let result =
        unsafe { GetFileInformationByHandle(file.as_raw_handle(), information.as_mut_ptr()) };
    if result == 0 {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::Io,
            format!(
                "query opened Windows artifact identity: {}",
                io::Error::last_os_error()
            ),
        ));
    }
    // SAFETY: a successful GetFileInformationByHandle call initialized the structure.
    let information = unsafe { information.assume_init() };
    Ok(ArtifactFileIdentity::Windows {
        volume_serial_number: information.dwVolumeSerialNumber,
        file_index: (u64::from(information.nFileIndexHigh) << 32)
            | u64::from(information.nFileIndexLow),
        link_count: information.nNumberOfLinks,
    })
}

pub(crate) fn has_single_link(identity: &ArtifactFileIdentity) -> bool {
    match identity {
        #[cfg(unix)]
        ArtifactFileIdentity::Unix { link_count, .. } => *link_count == 1,
        #[cfg(windows)]
        ArtifactFileIdentity::Windows { link_count, .. } => *link_count == 1,
    }
}
