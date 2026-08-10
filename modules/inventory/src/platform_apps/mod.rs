use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;
use std::time::Instant;

use rz0_inventory_contract::{AppRecord, InventorySource};

#[cfg(any(target_os = "linux", test))]
mod desktop_entry;
#[cfg(any(target_os = "linux", test))]
mod linux;
#[cfg(any(target_os = "macos", test))]
mod macos;

pub(super) const MAX_APP_RECORDS: usize = rz0_resource_contract::MAX_INVENTORY_APP_RECORDS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformAppCollection {
    pub source: InventorySource,
    pub apps: Vec<AppRecord>,
}

#[derive(Debug, Clone)]
pub(super) struct RootSpec {
    pub path: PathBuf,
    pub label: String,
}

#[cfg(target_os = "macos")]
pub fn collect_installed_apps() -> Vec<PlatformAppCollection> {
    macos::collect_installed_apps()
}

#[cfg(target_os = "linux")]
pub fn collect_installed_apps() -> Vec<PlatformAppCollection> {
    linux::collect_installed_apps()
}

pub(super) fn open_root(root: &RootSpec, warnings: &mut Vec<String>) -> Option<Vec<fs::DirEntry>> {
    let metadata = match fs::symlink_metadata(&root.path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return None,
        Err(_) => {
            warnings.push(format!("application root '{}' was unavailable", root.label));
            return None;
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        warnings.push(format!(
            "application root '{}' was not a direct directory and was skipped",
            root.label
        ));
        return None;
    }
    let entries = match fs::read_dir(&root.path) {
        Ok(entries) => entries
            .take(MAX_APP_RECORDS.saturating_add(1))
            .filter_map(Result::ok)
            .collect(),
        Err(_) => {
            warnings.push(format!(
                "application root '{}' could not be enumerated",
                root.label
            ));
            return None;
        }
    };
    Some(entries)
}

pub(super) fn finish_collection(
    source_id: &str,
    source_kind: &str,
    started: Instant,
    opened_roots: usize,
    mut apps: Vec<AppRecord>,
    warnings: Vec<String>,
) -> PlatformAppCollection {
    apps.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    let status = if opened_roots == 0 {
        "unavailable"
    } else if warnings.is_empty() {
        "ok"
    } else {
        "partial"
    };
    PlatformAppCollection {
        source: InventorySource {
            id: source_id.to_string(),
            kind: source_kind.to_string(),
            status: status.to_string(),
            duration_ms: Some(elapsed_ms(started)),
            read_only: true,
            warnings,
        },
        apps,
    }
}

pub(super) fn read_direct_bounded_file(
    path: &std::path::Path,
    maximum: u64,
) -> Result<Option<Vec<u8>>, String> {
    let observed = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("metadata file was unavailable".to_string()),
    };
    if observed.file_type().is_symlink()
        || !observed.is_file()
        || observed.len() == 0
        || observed.len() > maximum
    {
        return Err("metadata file was linked, empty, non-regular, or oversized".to_string());
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    }
    let mut file = options
        .open(path)
        .map_err(|_| "metadata file could not be opened directly".to_string())?;
    let opened = file
        .metadata()
        .map_err(|_| "opened metadata file could not be inspected".to_string())?;
    if !opened.is_file() || opened.len() != observed.len() || !same_file(&observed, &opened) {
        return Err("metadata file identity changed while opening".to_string());
    }
    let mut bytes = Vec::with_capacity(usize::try_from(opened.len()).unwrap_or(0));
    (&mut file)
        .take(maximum.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|_| "metadata file could not be read".to_string())?;
    let final_metadata = file
        .metadata()
        .map_err(|_| "metadata file could not be reinspected".to_string())?;
    if bytes.len() as u64 != opened.len()
        || bytes.len() as u64 > maximum
        || final_metadata.len() != opened.len()
        || !same_file(&opened, &final_metadata)
    {
        return Err("metadata file changed or exceeded its bound while reading".to_string());
    }
    Ok(Some(bytes))
}

#[cfg(unix)]
fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt as _;
    left.dev() == right.dev() && left.ino() == right.ino()
}

#[cfg(not(unix))]
fn same_file(left: &fs::Metadata, right: &fs::Metadata) -> bool {
    left.len() == right.len()
}

pub(super) fn has_extension(path: &std::path::Path, expected: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(expected))
}

pub(super) fn sanitize_text(value: &str, max_len: usize) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return None;
    }
    let mut output = String::new();
    for character in trimmed.chars() {
        if output.len().saturating_add(character.len_utf8()) > max_len {
            break;
        }
        output.push(character);
    }
    (!output.is_empty()).then_some(output)
}

pub(super) fn sanitize_exact_text(value: &str, max_len: usize) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > max_len || trimmed.chars().any(char::is_control) {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub(super) fn fnv1a(value: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn exact_identity_text_is_rejected_instead_of_truncated() {
        assert_eq!(sanitize_exact_text(" alpha ", 8).as_deref(), Some("alpha"));
        assert!(sanitize_exact_text("oversized", 4).is_none());
        assert!(sanitize_exact_text("bad\nname", 32).is_none());
    }

    #[test]
    fn bounded_metadata_reads_reject_symlinked_files() {
        let temp = temp_root();
        fs::create_dir_all(&temp).expect("temp root");
        let target = temp.join("target");
        fs::write(&target, b"metadata").expect("target");
        let link = temp.join("link");
        symlink(&target, &link).expect("symlink");
        assert!(read_direct_bounded_file(&link, 1_024).is_err());
        fs::remove_dir_all(temp).expect("cleanup");
    }

    #[test]
    fn symlinked_application_roots_fail_closed() {
        let temp = temp_root();
        let target = temp.join("target");
        fs::create_dir_all(&target).expect("target");
        let link = temp.join("link");
        symlink(&target, &link).expect("symlink");
        let root = RootSpec {
            path: link,
            label: "fixture-link".to_string(),
        };
        let mut warnings = Vec::new();
        assert!(open_root(&root, &mut warnings).is_none());
        assert!(warnings[0].contains("direct directory"));
        fs::remove_dir_all(temp).expect("cleanup");
    }

    fn temp_root() -> PathBuf {
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "rz0-platform-apps-link-{}-{sequence}",
            std::process::id()
        ))
    }
}
