#[cfg(target_os = "macos")]
use std::cmp::Ordering;
#[cfg(target_os = "macos")]
use std::env;
#[cfg(target_os = "macos")]
use std::fs;
use std::io::Cursor;
#[cfg(target_os = "macos")]
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::time::Instant;

use rz0_inventory_contract::{AppRecord, SoftwareIdentifier};

use super::{
    MAX_APP_RECORDS, PlatformAppCollection, RootSpec, finish_collection, fnv1a, has_extension,
    open_root, read_direct_bounded_file, sanitize_exact_text, sanitize_text,
};

#[cfg(target_os = "macos")]
pub(super) fn collect_installed_apps() -> Vec<PlatformAppCollection> {
    let (application_roots, warnings) = application_roots();
    vec![
        collect_roots(&application_roots, warnings),
        collect_manager_roots(
            "macos.homebrew.formulae",
            "package_manager_metadata",
            "Homebrew formula",
            &homebrew_roots("Cellar"),
        ),
        collect_manager_roots(
            "macos.homebrew.casks",
            "package_manager_metadata",
            "Homebrew cask",
            &homebrew_roots("Caskroom"),
        ),
        collect_manager_roots(
            "macos.macports.packages",
            "package_manager_metadata",
            "MacPorts",
            &[root(
                "macports-software",
                "/opt/local/var/macports/software",
            )],
        ),
        collect_package_receipt_roots(&[root("installer-receipts", "/var/db/receipts")]),
    ]
}

fn collect_roots(roots: &[RootSpec], mut warnings: Vec<String>) -> PlatformAppCollection {
    let started = Instant::now();
    let mut apps = Vec::new();
    let mut opened_roots = 0usize;
    let mut skipped_entries = 0usize;
    let mut inspected_entries = 0usize;
    let mut limit_reached = false;

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
            let Ok(file_type) = entry.file_type() else {
                skipped_entries = skipped_entries.saturating_add(1);
                continue;
            };
            if file_type.is_symlink() || !file_type.is_dir() {
                continue;
            }
            let path = entry.path();
            if !has_extension(&path, "app") {
                continue;
            }
            let Some(name) = path
                .file_stem()
                .and_then(|value| value.to_str())
                .and_then(|value| sanitize_text(value, 240))
            else {
                skipped_entries = skipped_entries.saturating_add(1);
                continue;
            };
            let identity = format!("{}|{}", name.to_ascii_lowercase(), root.label);
            let (version, bundle_id) = bundle_metadata(&path);
            apps.push(AppRecord {
                id: format!("macos.app.{:016x}", fnv1a(identity.as_bytes())),
                name,
                source_id: "macos.application_bundles".to_string(),
                version,
                publisher: None,
                identifiers: bundle_id
                    .map(|value| {
                        vec![SoftwareIdentifier {
                            kind: "bundle_id".to_string(),
                            value,
                        }]
                    })
                    .unwrap_or_default(),
                install_location: Some(path.display().to_string()),
                warnings: Vec::new(),
            });
        }
    }
    if limit_reached {
        warnings.push(format!(
            "application entry inspection limit of {MAX_APP_RECORDS} reached; remaining entries were skipped"
        ));
    }
    if skipped_entries > 0 {
        warnings.push(format!(
            "{skipped_entries} application entries were unreadable or invalid and were skipped"
        ));
    }
    finish_collection(
        "macos.application_bundles",
        "filesystem_metadata",
        started,
        opened_roots,
        apps,
        warnings,
    )
}

#[cfg(target_os = "macos")]
fn collect_manager_roots(
    source_id: &str,
    source_kind: &str,
    publisher: &str,
    roots: &[RootSpec],
) -> PlatformAppCollection {
    let started = Instant::now();
    let mut apps = Vec::new();
    let mut warnings = Vec::new();
    let mut opened_roots = 0usize;
    let mut inspected_entries = 0usize;

    'roots: for root in roots {
        let Some(mut entries) = open_root(root, &mut warnings) else {
            continue;
        };
        opened_roots += 1;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            if inspected_entries == MAX_APP_RECORDS {
                warnings.push(format!(
                    "package entry inspection limit of {MAX_APP_RECORDS} reached; remaining entries were skipped"
                ));
                break 'roots;
            }
            inspected_entries = inspected_entries.saturating_add(1);
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() || !file_type.is_dir() {
                continue;
            }
            let path = entry.path();
            let Some(name) = entry
                .file_name()
                .to_str()
                .and_then(|value| sanitize_exact_text(value, 180))
            else {
                continue;
            };
            let identity = format!("{}|{}|{}", source_id, name.to_ascii_lowercase(), root.label);
            apps.push(AppRecord {
                id: format!("macos.package.{:016x}", fnv1a(identity.as_bytes())),
                name: name.clone(),
                source_id: source_id.to_string(),
                version: newest_child_version(&path),
                publisher: Some(publisher.to_string()),
                identifiers: vec![SoftwareIdentifier {
                    kind: "manager_package".to_string(),
                    value: format!("{source_id}:{name}"),
                }],
                install_location: Some(path.display().to_string()),
                warnings: Vec::new(),
            });
        }
    }

    finish_collection(
        source_id,
        source_kind,
        started,
        opened_roots,
        apps,
        warnings,
    )
}

fn collect_package_receipt_roots(roots: &[RootSpec]) -> PlatformAppCollection {
    let started = Instant::now();
    let mut apps = Vec::new();
    let mut warnings = Vec::new();
    let mut opened_roots = 0usize;
    let mut inspected_entries = 0usize;
    let mut unreadable_receipts = 0usize;
    let mut fallback_receipts = 0usize;

    'roots: for root in roots {
        let Some(mut entries) = open_root(root, &mut warnings) else {
            continue;
        };
        opened_roots += 1;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            if inspected_entries == MAX_APP_RECORDS {
                warnings.push(format!(
                    "package receipt inspection limit of {MAX_APP_RECORDS} reached; remaining entries were skipped"
                ));
                break 'roots;
            }
            inspected_entries = inspected_entries.saturating_add(1);
            let Ok(file_type) = entry.file_type() else {
                unreadable_receipts = unreadable_receipts.saturating_add(1);
                continue;
            };
            let path = entry.path();
            if file_type.is_symlink() || !file_type.is_file() || !has_extension(&path, "plist") {
                continue;
            }
            let fallback = path
                .file_stem()
                .and_then(|value| value.to_str())
                .and_then(|value| sanitize_exact_text(value, 240));
            let (name, version) = match package_receipt_metadata(&path) {
                Some(metadata) => metadata,
                None => match fallback {
                    Some(name) => {
                        fallback_receipts = fallback_receipts.saturating_add(1);
                        (name, None)
                    }
                    None => {
                        unreadable_receipts = unreadable_receipts.saturating_add(1);
                        continue;
                    }
                },
            };
            let identity = format!(
                "macos.package_receipts|{}|{}|{}",
                name.to_ascii_lowercase(),
                root.label,
                path.file_name()
                    .and_then(|value| value.to_str())
                    .unwrap_or("receipt")
            );
            apps.push(AppRecord {
                id: format!("macos.receipt.{:016x}", fnv1a(identity.as_bytes())),
                name: name.clone(),
                source_id: "macos.package_receipts".to_string(),
                version,
                publisher: Some("Apple Installer receipt".to_string()),
                identifiers: vec![SoftwareIdentifier {
                    kind: "receipt_id".to_string(),
                    value: name.clone(),
                }],
                install_location: Some(path.display().to_string()),
                warnings: Vec::new(),
            });
        }
    }
    if unreadable_receipts > 0 {
        warnings.push(format!(
            "{unreadable_receipts} package receipts were unreadable or invalid and were skipped"
        ));
    }
    if fallback_receipts > 0 {
        warnings.push(format!(
            "{fallback_receipts} package receipts used filename identifiers because plist metadata was unavailable"
        ));
    }
    finish_collection(
        "macos.package_receipts",
        "package_manager_metadata",
        started,
        opened_roots,
        apps,
        warnings,
    )
}

fn package_receipt_metadata(path: &Path) -> Option<(String, Option<String>)> {
    const MAX_RECEIPT_PLIST_BYTES: u64 = 2 * 1024 * 1024;
    let bytes = read_direct_bounded_file(path, MAX_RECEIPT_PLIST_BYTES)
        .ok()
        .flatten()?;
    let value = plist::Value::from_reader(Cursor::new(bytes)).ok()?;
    let dictionary = value.as_dictionary()?;
    let name = dictionary
        .get("PackageIdentifier")
        .and_then(plist::Value::as_string)
        .and_then(|value| sanitize_exact_text(value, 240))?;
    let version = dictionary
        .get("PackageVersion")
        .and_then(plist::Value::as_string)
        .and_then(|value| sanitize_exact_text(value, 120));
    Some((name, version))
}

#[cfg(target_os = "macos")]
fn bundle_metadata(bundle: &Path) -> (Option<String>, Option<String>) {
    fn read(bundle: &Path) -> Option<(Option<String>, Option<String>)> {
        const MAX_INFO_PLIST_BYTES: u64 = 2 * 1024 * 1024;
        let path = bundle.join("Contents").join("Info.plist");
        let bytes = read_direct_bounded_file(&path, MAX_INFO_PLIST_BYTES)
            .ok()
            .flatten()?;
        let value = plist::Value::from_reader(Cursor::new(bytes)).ok()?;
        let dictionary = value.as_dictionary()?;
        let version = ["CFBundleShortVersionString", "CFBundleVersion"]
            .into_iter()
            .find_map(|key| dictionary.get(key).and_then(plist::Value::as_string))
            .and_then(|value| sanitize_text(value, 120));
        let bundle_id = dictionary
            .get("CFBundleIdentifier")
            .and_then(plist::Value::as_string)
            .and_then(|value| sanitize_exact_text(value, 256));
        Some((version, bundle_id))
    }
    read(bundle).unwrap_or_default()
}

#[cfg(not(target_os = "macos"))]
fn bundle_metadata(_bundle: &std::path::Path) -> (Option<String>, Option<String>) {
    (None, None)
}

#[cfg(target_os = "macos")]
fn newest_child_version(package_root: &Path) -> Option<String> {
    let metadata = fs::symlink_metadata(package_root).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return None;
    }
    let mut versions = fs::read_dir(package_root)
        .ok()?
        .take(128)
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            if file_type.is_symlink() || !file_type.is_dir() {
                return None;
            }
            entry
                .file_name()
                .to_str()
                .and_then(|value| sanitize_exact_text(value, 120))
        })
        .collect::<Vec<_>>();
    versions.sort_by(|left, right| natural_version_cmp(left, right));
    versions.pop()
}

#[cfg(target_os = "macos")]
fn natural_version_cmp(left: &str, right: &str) -> Ordering {
    let left = left.as_bytes();
    let right = right.as_bytes();
    let (mut left_index, mut right_index) = (0usize, 0usize);
    while left_index < left.len() && right_index < right.len() {
        if left[left_index].is_ascii_digit() && right[right_index].is_ascii_digit() {
            let left_end = digit_run_end(left, left_index);
            let right_end = digit_run_end(right, right_index);
            let left_significant = (left_index..left_end)
                .find(|index| left[*index] != b'0')
                .unwrap_or(left_end.saturating_sub(1));
            let right_significant = (right_index..right_end)
                .find(|index| right[*index] != b'0')
                .unwrap_or(right_end.saturating_sub(1));
            let ordering = (left_end - left_significant)
                .cmp(&(right_end - right_significant))
                .then_with(|| {
                    left[left_significant..left_end].cmp(&right[right_significant..right_end])
                });
            if ordering != Ordering::Equal {
                return ordering;
            }
            left_index = left_end;
            right_index = right_end;
            continue;
        }
        let ordering = left[left_index]
            .to_ascii_lowercase()
            .cmp(&right[right_index].to_ascii_lowercase());
        if ordering != Ordering::Equal {
            return ordering;
        }
        left_index += 1;
        right_index += 1;
    }
    left.len().cmp(&right.len()).then_with(|| left.cmp(right))
}

#[cfg(target_os = "macos")]
fn digit_run_end(value: &[u8], start: usize) -> usize {
    (start..value.len())
        .find(|index| !value[*index].is_ascii_digit())
        .unwrap_or(value.len())
}

#[cfg(target_os = "macos")]
fn application_roots() -> (Vec<RootSpec>, Vec<String>) {
    let mut roots = vec![
        root("system", "/System/Applications"),
        root("system-utilities", "/System/Applications/Utilities"),
        root("local", "/Applications"),
        root("local-utilities", "/Applications/Utilities"),
    ];
    let mut warnings = Vec::new();
    if let Some(home) = absolute_env_path("HOME") {
        roots.push(RootSpec {
            path: home.join("Applications"),
            label: "user".to_string(),
        });
    } else {
        warnings.push("user application root was unavailable".to_string());
    }
    (roots, warnings)
}

#[cfg(target_os = "macos")]
fn homebrew_roots(child: &str) -> Vec<RootSpec> {
    ["/opt/homebrew", "/usr/local"]
        .into_iter()
        .map(|prefix| RootSpec {
            path: PathBuf::from(prefix).join(child),
            label: format!(
                "{}-{child}",
                prefix.trim_start_matches('/').replace('/', "-")
            ),
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn absolute_env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute() && path.as_os_str().as_bytes().len() <= 4096)
}

#[cfg(target_os = "macos")]
fn root(label: &str, path: &str) -> RootSpec {
    RootSpec {
        path: PathBuf::from(path),
        label: label.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_receipts_are_shallow_bounded_and_use_declared_identity() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("platform-apps")
            .join("macos-receipts");
        let collection = collect_package_receipt_roots(&[RootSpec {
            path: root,
            label: "fixture-receipts".to_string(),
        }]);
        assert_eq!(collection.source.status, "ok");
        assert_eq!(collection.apps.len(), 1);
        assert_eq!(collection.apps[0].name, "com.example.alpha");
        assert_eq!(collection.apps[0].version.as_deref(), Some("1.2.3"));
        assert_eq!(collection.apps[0].source_id, "macos.package_receipts");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn manager_version_selection_compares_numeric_runs_naturally() {
        assert_eq!(natural_version_cmp("9.9", "10.0"), Ordering::Less);
        assert_eq!(natural_version_cmp("1.10.0", "1.2.9"), Ordering::Greater);
        assert_eq!(natural_version_cmp("1.0", "1.0.1"), Ordering::Less);
    }

    #[test]
    fn collection_is_shallow_and_bounded_to_app_directories() {
        let collection = collect_roots(
            &[RootSpec {
                path: fixture_root(),
                label: "fixture".to_string(),
            }],
            Vec::new(),
        );
        assert_eq!(collection.source.status, "ok");
        assert_eq!(collection.apps.len(), 2);
        assert_eq!(collection.apps[0].name, "Alpha");
        assert_eq!(collection.apps[1].name, "Versioned");
        #[cfg(target_os = "macos")]
        {
            assert_eq!(collection.apps[1].version.as_deref(), Some("1.2.3"));
            assert_eq!(collection.apps[1].identifiers.len(), 1);
            assert_eq!(collection.apps[1].identifiers[0].kind, "bundle_id");
            assert_eq!(
                collection.apps[1].identifiers[0].value,
                "dev.neuman.runtime-zero.fixture"
            );
        }
    }

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("platform-apps")
            .join("macos")
    }
}
