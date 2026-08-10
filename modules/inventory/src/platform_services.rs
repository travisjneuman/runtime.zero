use std::{collections::BTreeSet, fs, path::PathBuf, time::Instant};

use rz0_inventory_contract::{InventorySource, ServiceRecord};

const MAX_SERVICE_RECORDS: usize = rz0_resource_contract::MAX_INVENTORY_SERVICE_RECORDS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceCollection {
    pub source: InventorySource,
    pub services: Vec<ServiceRecord>,
}

#[derive(Debug, Clone)]
struct ServiceRoot {
    path: PathBuf,
    label: &'static str,
    scope: &'static str,
}

#[cfg(target_os = "macos")]
pub fn collect_services() -> Vec<ServiceCollection> {
    let mut user_roots = Vec::new();
    if let Some(home) = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute() && path.as_os_str().len() <= 4_096)
    {
        user_roots.push(ServiceRoot {
            path: home.join("Library/LaunchAgents"),
            label: "user-launch-agents",
            scope: "user",
        });
    }
    vec![
        collect_launchd("macos.launchd.user", &user_roots),
        collect_launchd(
            "macos.launchd.system",
            &[
                ServiceRoot {
                    path: PathBuf::from("/Library/LaunchAgents"),
                    label: "local-launch-agents",
                    scope: "system",
                },
                ServiceRoot {
                    path: PathBuf::from("/Library/LaunchDaemons"),
                    label: "local-launch-daemons",
                    scope: "system",
                },
                ServiceRoot {
                    path: PathBuf::from("/System/Library/LaunchAgents"),
                    label: "system-launch-agents",
                    scope: "system",
                },
                ServiceRoot {
                    path: PathBuf::from("/System/Library/LaunchDaemons"),
                    label: "system-launch-daemons",
                    scope: "system",
                },
            ],
        ),
    ]
}

#[cfg(target_os = "linux")]
pub fn collect_services() -> Vec<ServiceCollection> {
    let mut roots = Vec::new();
    if let Some(home) = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute() && path.as_os_str().len() <= 4_096)
    {
        roots.push(ServiceRoot {
            path: home.join(".config/systemd/user"),
            label: "user-config-systemd",
            scope: "user",
        });
    }
    roots.extend([
        ServiceRoot {
            path: PathBuf::from("/etc/systemd/user"),
            label: "local-user-systemd",
            scope: "user",
        },
        ServiceRoot {
            path: PathBuf::from("/usr/lib/systemd/user"),
            label: "vendor-user-systemd-usr",
            scope: "user",
        },
        ServiceRoot {
            path: PathBuf::from("/lib/systemd/user"),
            label: "vendor-user-systemd-lib",
            scope: "user",
        },
        ServiceRoot {
            path: PathBuf::from("/etc/systemd/system"),
            label: "local-systemd",
            scope: "system",
        },
        ServiceRoot {
            path: PathBuf::from("/usr/lib/systemd/system"),
            label: "vendor-systemd-usr",
            scope: "system",
        },
        ServiceRoot {
            path: PathBuf::from("/lib/systemd/system"),
            label: "vendor-systemd-lib",
            scope: "system",
        },
    ]);
    vec![collect_systemd(&roots)]
}

#[cfg(any(target_os = "macos", test))]
fn collect_launchd(source_id: &str, roots: &[ServiceRoot]) -> ServiceCollection {
    let started = Instant::now();
    let mut warnings = Vec::new();
    let mut services = Vec::new();
    let mut opened_roots = 0usize;
    let mut seen = BTreeSet::new();
    let mut invalid = 0usize;
    let mut fallback_labels = 0usize;
    'roots: for root in roots {
        let Some(mut entries) = open_root(root, &mut warnings) else {
            continue;
        };
        opened_roots += 1;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            if services.len() == MAX_SERVICE_RECORDS {
                warnings.push(format!(
                    "service record limit of {MAX_SERVICE_RECORDS} reached; remaining launchd entries were skipped"
                ));
                break 'roots;
            }
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("plist") {
                continue;
            }
            let Ok(file_type) = entry.file_type() else {
                invalid = invalid.saturating_add(1);
                continue;
            };
            if file_type.is_symlink() || !file_type.is_file() {
                continue;
            }
            let fallback = path
                .file_stem()
                .and_then(|value| value.to_str())
                .and_then(sanitize);
            let (name, enabled) = match launchd_metadata(&path) {
                Some(metadata) => metadata,
                None => match fallback {
                    Some(name) => {
                        fallback_labels = fallback_labels.saturating_add(1);
                        (name, None)
                    }
                    None => {
                        invalid = invalid.saturating_add(1);
                        continue;
                    }
                },
            };
            let identity = format!("{}|{name}", root.label);
            if !seen.insert(identity.clone()) {
                continue;
            }
            services.push(ServiceRecord {
                id: format!("macos.launchd.{:016x}", fnv1a(identity.as_bytes())),
                name,
                source_id: source_id.to_string(),
                kind: "persistence".to_string(),
                scope: root.scope.to_string(),
                enabled,
                location: Some(path.display().to_string()),
                warnings: Vec::new(),
            });
        }
    }
    if invalid > 0 {
        warnings.push(format!(
            "{invalid} launchd entries were unreadable or invalid and were skipped"
        ));
    }
    if fallback_labels > 0 {
        warnings.push(format!(
            "{fallback_labels} launchd entries used filename labels because plist metadata was unavailable"
        ));
    }
    finish(
        source_id,
        "launchd_metadata",
        started,
        opened_roots,
        services,
        warnings,
    )
}

#[cfg(any(target_os = "linux", test))]
fn collect_systemd(roots: &[ServiceRoot]) -> ServiceCollection {
    let started = Instant::now();
    let mut warnings = Vec::new();
    let mut services = Vec::new();
    let mut opened_roots = 0usize;
    let mut seen = BTreeSet::new();
    let mut seen_roots = BTreeSet::new();
    'roots: for root in roots {
        let Some(canonical_root) = fs::canonicalize(&root.path).ok() else {
            let _ = open_root(root, &mut warnings);
            continue;
        };
        if !seen_roots.insert(canonical_root) {
            continue;
        }
        let Some(mut entries) = open_root(root, &mut warnings) else {
            continue;
        };
        opened_roots += 1;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            if services.len() == MAX_SERVICE_RECORDS {
                warnings.push(format!(
                    "service record limit of {MAX_SERVICE_RECORDS} reached; remaining systemd entries were skipped"
                ));
                break 'roots;
            }
            let Some(name) = entry.file_name().to_str().and_then(sanitize) else {
                continue;
            };
            if !name.ends_with(".service") {
                continue;
            }
            let identity = format!("{}|{name}", root.label);
            if !seen.insert(identity.clone()) {
                continue;
            }
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if !file_type.is_file() && !file_type.is_symlink() {
                continue;
            }
            services.push(ServiceRecord {
                id: format!("linux.systemd.{:016x}", fnv1a(identity.as_bytes())),
                name,
                source_id: "linux.systemd.units".to_string(),
                kind: "service".to_string(),
                scope: root.scope.to_string(),
                // Directory placement alone does not prove effective systemd
                // enablement; runtime manager evidence is still required.
                enabled: None,
                location: Some(entry.path().display().to_string()),
                warnings: Vec::new(),
            });
        }
    }
    finish(
        "linux.systemd.units",
        "filesystem_metadata",
        started,
        opened_roots,
        services,
        warnings,
    )
}

fn open_root(root: &ServiceRoot, warnings: &mut Vec<String>) -> Option<Vec<fs::DirEntry>> {
    let metadata = match fs::symlink_metadata(&root.path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
        Err(_) => {
            warnings.push(format!("service root '{}' was unavailable", root.label));
            return None;
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        warnings.push(format!(
            "service root '{}' was not a direct directory and was skipped",
            root.label
        ));
        return None;
    }
    match fs::read_dir(&root.path) {
        Ok(entries) => Some(
            entries
                .take(MAX_SERVICE_RECORDS.saturating_add(1))
                .filter_map(Result::ok)
                .collect(),
        ),
        Err(_) => {
            warnings.push(format!(
                "service root '{}' could not be enumerated",
                root.label
            ));
            None
        }
    }
}

#[cfg(target_os = "macos")]
fn launchd_metadata(path: &std::path::Path) -> Option<(String, Option<bool>)> {
    const MAX_PLIST_BYTES: u64 = rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES;
    let bytes = crate::platform_apps::read_direct_bounded_file(path, MAX_PLIST_BYTES)
        .ok()
        .flatten()?;
    let value = plist::Value::from_reader(std::io::Cursor::new(bytes)).ok()?;
    let dictionary = value.as_dictionary()?;
    let name = dictionary
        .get("Label")
        .and_then(plist::Value::as_string)
        .and_then(sanitize)?;
    let enabled = dictionary
        .get("Disabled")
        .and_then(plist::Value::as_boolean)
        .map(|disabled| !disabled);
    Some((name, enabled))
}

#[cfg(all(test, not(target_os = "macos")))]
fn launchd_metadata(_path: &std::path::Path) -> Option<(String, Option<bool>)> {
    None
}

fn finish(
    source_id: &str,
    kind: &str,
    started: Instant,
    opened_roots: usize,
    mut services: Vec<ServiceRecord>,
    warnings: Vec<String>,
) -> ServiceCollection {
    services.sort_by(|left, right| left.id.cmp(&right.id));
    let status = if opened_roots == 0 {
        "unavailable"
    } else if warnings.is_empty() {
        "ok"
    } else {
        "partial"
    };
    ServiceCollection {
        source: InventorySource {
            id: source_id.to_string(),
            kind: kind.to_string(),
            status: status.to_string(),
            duration_ms: Some(u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)),
            read_only: true,
            warnings,
        },
        services,
    }
}

fn sanitize(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.len() > 240 || value.chars().any(char::is_control) {
        None
    } else {
        Some(value.to_string())
    }
}

fn fnv1a(value: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn systemd_metadata_collection_is_shallow_bounded_and_nonexecuting() {
        let root = temp_root();
        fs::create_dir_all(&root).expect("service fixture root");
        fs::write(
            root.join("alpha.service"),
            b"[Service]\nExecStart=/bin/true\n",
        )
        .expect("service fixture");
        fs::write(root.join("ignored.timer"), b"[Timer]\nOnBootSec=1\n").expect("timer fixture");
        let collection = collect_systemd(&[ServiceRoot {
            path: root.clone(),
            label: "fixture-systemd",
            scope: "system",
        }]);
        assert_eq!(collection.source.status, "ok");
        assert_eq!(collection.services.len(), 1);
        assert_eq!(collection.services[0].name, "alpha.service");
        assert_eq!(collection.services[0].enabled, None);
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn missing_service_roots_are_explicitly_unavailable_and_read_only() {
        let collection = collect_launchd(
            "macos.launchd.fixture",
            &[ServiceRoot {
                path: PathBuf::from("tests/fixtures/does-not-exist"),
                label: "missing-fixture",
                scope: "user",
            }],
        );
        assert_eq!(collection.source.status, "unavailable");
        assert!(collection.source.read_only);
        assert!(collection.services.is_empty());
    }

    fn temp_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "rz0-service-fixture-{}-{}",
            std::process::id(),
            TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ))
    }
}
