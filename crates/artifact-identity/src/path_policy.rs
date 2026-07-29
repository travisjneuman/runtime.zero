use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use crate::{ArtifactIdentityError, ArtifactIdentityErrorCode};

pub(crate) struct CheckedArtifactPath {
    pub canonical_root: PathBuf,
    pub artifact_path: PathBuf,
}

pub(crate) fn checked_artifact_path(
    root: &Path,
    relative: &str,
) -> Result<CheckedArtifactPath, ArtifactIdentityError> {
    if !valid_relative_path(relative) {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::UnsafeRelativePath,
            "artifact path must be normalized and relative",
        ));
    }
    let root_metadata = fs::symlink_metadata(root).map_err(|error| {
        ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::Io,
            format!("read artifact root metadata: {error}"),
        )
    })?;
    if !root_metadata.is_dir() || unsafe_link_type(&root_metadata) {
        return Err(ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::UnsafeRoot,
            "artifact root must be a direct directory",
        ));
    }
    let canonical_root = fs::canonicalize(root).map_err(|error| {
        ArtifactIdentityError::new(
            ArtifactIdentityErrorCode::Io,
            format!("canonicalize artifact root: {error}"),
        )
    })?;
    let mut artifact_path = root.to_path_buf();
    let components = Path::new(relative).components().collect::<Vec<_>>();
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(component) = component else {
            return Err(ArtifactIdentityError::new(
                ArtifactIdentityErrorCode::UnsafeRelativePath,
                "artifact path contains a non-normal component",
            ));
        };
        artifact_path.push(component);
        let metadata = fs::symlink_metadata(&artifact_path).map_err(|error| {
            ArtifactIdentityError::new(
                ArtifactIdentityErrorCode::Io,
                format!("read artifact component metadata: {error}"),
            )
        })?;
        let is_final = index + 1 == components.len();
        if unsafe_link_type(&metadata)
            || (is_final && !metadata.is_file())
            || (!is_final && !metadata.is_dir())
        {
            return Err(ArtifactIdentityError::new(
                ArtifactIdentityErrorCode::UnsafeFilesystemType,
                "artifact path includes an unsafe filesystem type",
            ));
        }
    }
    Ok(CheckedArtifactPath {
        canonical_root,
        artifact_path,
    })
}

fn valid_relative_path(value: &str) -> bool {
    if value.is_empty()
        || value.len() > 1024
        || value.contains('\\')
        || value.contains(':')
        || value.chars().any(char::is_control)
        || value
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        return false;
    }
    let path = Path::new(value);
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

#[cfg(windows)]
fn unsafe_link_type(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    metadata.file_type().is_symlink()
        || metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn unsafe_link_type(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}
