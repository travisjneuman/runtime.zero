use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Component, Path};

pub(crate) fn validate_relative_path(path: &str) -> Result<(), &'static str> {
    if path.trim().is_empty() {
        return Err("path must not be empty");
    }
    if looks_url_like(path) {
        return Err("URL-like paths are not supported");
    }
    if path.contains('\\') {
        return Err("backslash paths are not supported");
    }
    let path = Path::new(path);
    if path.is_absolute() {
        return Err("absolute paths are not supported");
    }
    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir => return Err(".. traversal is not supported"),
            Component::RootDir | Component::Prefix(_) => {
                return Err("absolute paths are not supported");
            }
        }
    }
    Ok(())
}

pub(crate) fn is_valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(crate) fn sha256_file(path: &Path) -> io::Result<String> {
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(to_lower_hex(&hasher.finalize()))
}

pub(crate) fn is_reparse_or_symlink(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
        metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}

pub(crate) fn path_contains_reparse_or_symlink(
    package_root: &Path,
    relative_path: &str,
) -> io::Result<bool> {
    let mut current = package_root.to_path_buf();
    for component in Path::new(relative_path).components() {
        let Component::Normal(part) = component else {
            continue;
        };
        current.push(part);
        let metadata = fs::symlink_metadata(&current)?;
        if is_reparse_or_symlink(&metadata) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn looks_url_like(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.contains("://")
        || lower.starts_with("file:")
        || lower.starts_with("http:")
        || lower.starts_with("https:")
}

fn to_lower_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(hex_char(byte >> 4));
        output.push(hex_char(byte & 0x0f));
    }
    output
}

fn hex_char(value: u8) -> char {
    match value {
        0..=9 => char::from(b'0' + value),
        10..=15 => char::from(b'a' + value - 10),
        _ => unreachable!("nibble is always 0..=15"),
    }
}
