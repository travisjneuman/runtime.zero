use std::collections::BTreeSet;
#[cfg(target_os = "linux")]
use std::env;
#[cfg(target_os = "linux")]
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::Instant;

use rz0_inventory_contract::{AppRecord, SoftwareIdentifier};

use super::desktop_entry::{DesktopEntry, read_desktop_entry};
use super::{
    MAX_APP_RECORDS, PlatformAppCollection, RootSpec, finish_collection, fnv1a, has_extension,
    open_root, read_direct_bounded_file, sanitize_exact_text,
};

#[cfg(target_os = "linux")]
const MAX_APP_ROOTS: usize = 32;

#[cfg(target_os = "linux")]
pub(super) fn collect_installed_apps() -> Vec<PlatformAppCollection> {
    let (roots, warnings) = application_roots();
    vec![
        collect_roots(&roots, warnings),
        collect_dpkg_status(Path::new("/var/lib/dpkg/status")),
        collect_pacman_local(&RootSpec {
            path: PathBuf::from("/var/lib/pacman/local"),
            label: "pacman-local".to_string(),
        }),
    ]
}

fn collect_roots(roots: &[RootSpec], mut warnings: Vec<String>) -> PlatformAppCollection {
    let started = Instant::now();
    let mut apps = Vec::new();
    let mut opened_roots = 0usize;
    let mut skipped_entries = 0usize;
    let mut inspected_entries = 0usize;
    let mut limit_reached = false;
    let mut seen_desktop_ids = BTreeSet::new();

    'roots: for root in roots {
        let Some(mut entries) = open_root(root, &mut warnings) else {
            continue;
        };
        opened_roots += 1;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            if inspected_entries == MAX_APP_RECORDS {
                limit_reached = true;
                break 'roots;
            }
            inspected_entries = inspected_entries.saturating_add(1);
            let path = entry.path();
            if !has_extension(&path, "desktop") {
                continue;
            }
            let Some(desktop_id) = path
                .file_name()
                .and_then(|value| value.to_str())
                .map(str::to_string)
            else {
                skipped_entries = skipped_entries.saturating_add(1);
                continue;
            };
            if !seen_desktop_ids.insert(desktop_id.clone()) {
                continue;
            }
            match read_desktop_entry(&path) {
                DesktopEntry::Application(name) => {
                    let identity = format!("linux.desktop|{}", desktop_id.to_ascii_lowercase());
                    apps.push(AppRecord {
                        id: format!("linux.app.{:016x}", fnv1a(identity.as_bytes())),
                        name,
                        source_id: "linux.desktop_entries".to_string(),
                        version: None,
                        publisher: None,
                        identifiers: vec![SoftwareIdentifier {
                            kind: "desktop_id".to_string(),
                            value: desktop_id,
                        }],
                        install_location: Some(path.display().to_string()),
                        warnings: Vec::new(),
                    });
                }
                DesktopEntry::Hidden | DesktopEntry::NotApplication => {}
                DesktopEntry::Invalid => skipped_entries = skipped_entries.saturating_add(1),
            }
        }
    }
    if limit_reached {
        warnings.push(format!(
            "application entry inspection limit of {MAX_APP_RECORDS} reached; remaining entries were skipped"
        ));
    }
    if skipped_entries > 0 {
        warnings.push(format!(
            "{skipped_entries} desktop entries were unreadable or invalid and were skipped"
        ));
    }
    finish_collection(
        "linux.desktop_entries",
        "bounded_file_content",
        started,
        opened_roots,
        apps,
        warnings,
    )
}

fn collect_dpkg_status(path: &Path) -> PlatformAppCollection {
    let started = Instant::now();
    let mut warnings = Vec::new();
    let bytes =
        match read_direct_bounded_file(path, rz0_resource_contract::MAX_INVENTORY_REPORT_BYTES) {
            Ok(Some(bytes)) => bytes,
            Ok(None) => {
                return finish_collection(
                    "linux.dpkg.packages",
                    "bounded_file_content",
                    started,
                    0,
                    Vec::new(),
                    warnings,
                );
            }
            Err(error) => {
                warnings.push(error);
                return finish_collection(
                    "linux.dpkg.packages",
                    "bounded_file_content",
                    started,
                    0,
                    Vec::new(),
                    warnings,
                );
            }
        };
    let text = match std::str::from_utf8(&bytes) {
        Ok(text) => text,
        Err(_) => {
            warnings.push("dpkg status is not valid UTF-8 and was skipped".to_string());
            ""
        }
    };
    let mut apps = Vec::new();
    let mut invalid = 0usize;
    for paragraph in text.split("\n\n") {
        if apps.len() == MAX_APP_RECORDS {
            warnings.push(format!(
                "dpkg package limit of {MAX_APP_RECORDS} reached; remaining records were skipped"
            ));
            break;
        }
        let mut name = None;
        let mut version = None;
        let mut architecture = None;
        let mut status = None;
        let mut duplicate_identity_field = false;
        for line in paragraph.lines() {
            if let Some(value) = line.strip_prefix("Package:") {
                duplicate_identity_field |= name.is_some();
                name = sanitize_exact_text(value, 160);
            } else if let Some(value) = line.strip_prefix("Version:") {
                duplicate_identity_field |= version.is_some();
                version = sanitize_exact_text(value, 120);
            } else if let Some(value) = line.strip_prefix("Architecture:") {
                duplicate_identity_field |= architecture.is_some();
                architecture = sanitize_exact_text(value, 80);
            } else if let Some(value) = line.strip_prefix("Status:") {
                duplicate_identity_field |= status.is_some();
                status = sanitize_exact_text(value, 80);
            }
        }
        if status.as_deref() != Some("install ok installed") {
            continue;
        }
        let Some(name) = name else {
            invalid = invalid.saturating_add(1);
            continue;
        };
        if duplicate_identity_field {
            invalid = invalid.saturating_add(1);
            continue;
        }
        let architecture = architecture.unwrap_or_else(|| "unknown".to_string());
        let identity = format!(
            "linux.dpkg|{}|{}",
            name.to_ascii_lowercase(),
            architecture.to_ascii_lowercase()
        );
        apps.push(AppRecord {
            id: format!("linux.dpkg.{:016x}", fnv1a(identity.as_bytes())),
            name: name.clone(),
            source_id: "linux.dpkg.packages".to_string(),
            version,
            publisher: Some("dpkg".to_string()),
            identifiers: vec![SoftwareIdentifier {
                kind: "manager_package".to_string(),
                value: format!("dpkg:{name}:{architecture}"),
            }],
            install_location: Some(path.display().to_string()),
            warnings: Vec::new(),
        });
    }
    if invalid > 0 {
        warnings.push(format!(
            "{invalid} installed dpkg records lacked a safe package identity and were skipped"
        ));
    }
    finish_collection(
        "linux.dpkg.packages",
        "bounded_file_content",
        started,
        1,
        apps,
        warnings,
    )
}

fn collect_pacman_local(root: &RootSpec) -> PlatformAppCollection {
    let started = Instant::now();
    let mut warnings = Vec::new();
    let Some(mut entries) = open_root(root, &mut warnings) else {
        return finish_collection(
            "linux.pacman.packages",
            "bounded_file_content",
            started,
            0,
            Vec::new(),
            warnings,
        );
    };
    entries.sort_by_key(|entry| entry.file_name());
    let mut apps = Vec::new();
    let mut invalid = 0usize;
    if entries.len() > MAX_APP_RECORDS {
        entries.truncate(MAX_APP_RECORDS);
        warnings.push(format!(
            "pacman package limit of {MAX_APP_RECORDS} reached; remaining records were skipped"
        ));
    }
    for entry in entries {
        let Ok(file_type) = entry.file_type() else {
            invalid = invalid.saturating_add(1);
            continue;
        };
        if file_type.is_symlink() || !file_type.is_dir() {
            continue;
        }
        let desc = entry.path().join("desc");
        let bytes = match read_direct_bounded_file(
            &desc,
            rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES,
        ) {
            Ok(Some(bytes)) => bytes,
            _ => {
                invalid = invalid.saturating_add(1);
                continue;
            }
        };
        let Some((name, version)) = parse_pacman_desc(&bytes) else {
            invalid = invalid.saturating_add(1);
            continue;
        };
        let identity = format!("linux.pacman|{}", name.to_ascii_lowercase());
        apps.push(AppRecord {
            id: format!("linux.pacman.{:016x}", fnv1a(identity.as_bytes())),
            name: name.clone(),
            source_id: "linux.pacman.packages".to_string(),
            version: Some(version),
            publisher: Some("pacman".to_string()),
            identifiers: vec![SoftwareIdentifier {
                kind: "manager_package".to_string(),
                value: format!("pacman:{name}"),
            }],
            install_location: Some(desc.display().to_string()),
            warnings: Vec::new(),
        });
    }
    if invalid > 0 {
        warnings.push(format!(
            "{invalid} pacman package records were unreadable or invalid and were skipped"
        ));
    }
    finish_collection(
        "linux.pacman.packages",
        "bounded_file_content",
        started,
        1,
        apps,
        warnings,
    )
}

fn parse_pacman_desc(bytes: &[u8]) -> Option<(String, String)> {
    let text = std::str::from_utf8(bytes).ok()?;
    let mut name = None;
    let mut version = None;
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        match line {
            "%NAME%" => {
                name = lines
                    .next()
                    .and_then(|value| sanitize_exact_text(value, 200))
            }
            "%VERSION%" => {
                version = lines
                    .next()
                    .and_then(|value| sanitize_exact_text(value, 120))
            }
            _ => {}
        }
    }
    Some((name?, version?))
}

#[cfg(target_os = "linux")]
fn application_roots() -> (Vec<RootSpec>, Vec<String>) {
    let mut roots = Vec::new();
    let mut warnings = Vec::new();
    if let Some(data_home) = absolute_env_path("XDG_DATA_HOME")
        .or_else(|| absolute_env_path("HOME").map(|home| home.join(".local").join("share")))
    {
        roots.push(RootSpec {
            path: data_home.join("applications"),
            label: "user".to_string(),
        });
    } else {
        warnings.push("user application root was unavailable".to_string());
    }

    let max_system_roots = MAX_APP_ROOTS - roots.len();
    let mut data_dirs = env::var_os("XDG_DATA_DIRS")
        .map(|value| {
            env::split_paths(&value)
                .take(max_system_roots.saturating_add(1))
                .collect::<Vec<_>>()
        })
        .filter(|paths| !paths.is_empty())
        .unwrap_or_else(|| {
            vec![
                PathBuf::from("/usr/local/share"),
                PathBuf::from("/usr/share"),
            ]
        });
    if data_dirs.len() > max_system_roots {
        data_dirs.truncate(max_system_roots);
        warnings.push(format!(
            "application root limit of {MAX_APP_ROOTS} reached; remaining roots were skipped"
        ));
    }
    let mut seen = BTreeSet::new();
    for (index, path) in data_dirs.into_iter().enumerate() {
        if !path.is_absolute() || !seen.insert(path.clone()) {
            warnings
                .push("an invalid or duplicate system application root was skipped".to_string());
            continue;
        }
        roots.push(RootSpec {
            path: path.join("applications"),
            label: format!("system-{}", index + 1),
        });
    }
    (roots, warnings)
}

#[cfg(target_os = "linux")]
fn absolute_env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute() && path.as_os_str().as_bytes().len() <= 4096)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_package_metadata_parsers_are_bounded_and_installed_only() {
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("platform-apps")
            .join("linux-packages");
        let dpkg = collect_dpkg_status(&fixture.join("dpkg").join("status"));
        assert_eq!(dpkg.source.status, "ok");
        assert_eq!(dpkg.apps.len(), 2);
        assert_eq!(dpkg.apps[0].name, "alpha");
        assert_eq!(dpkg.apps[1].name, "beta");
        assert!(!dpkg.apps.iter().any(|app| app.name == "removed"));

        let pacman = collect_pacman_local(&RootSpec {
            path: fixture.join("pacman"),
            label: "fixture-pacman".to_string(),
        });
        assert_eq!(pacman.source.status, "ok");
        assert_eq!(pacman.apps.len(), 1);
        assert_eq!(pacman.apps[0].name, "alpha");
        assert_eq!(pacman.apps[0].version.as_deref(), Some("1.2.3-1"));
    }

    #[test]
    fn collection_honors_hidden_precedence_and_rejects_invalid_data() {
        let fixture = fixture_root();
        let collection = collect_roots(
            &[
                RootSpec {
                    path: fixture.join("user"),
                    label: "user".to_string(),
                },
                RootSpec {
                    path: fixture.join("system"),
                    label: "system".to_string(),
                },
            ],
            Vec::new(),
        );
        assert_eq!(collection.source.status, "partial");
        assert_eq!(collection.apps.len(), 1);
        assert_eq!(collection.apps[0].name, "Beta Tool");
        assert!(collection.source.warnings[0].contains("1 desktop entries"));
    }

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("platform-apps")
            .join("linux")
    }
}
