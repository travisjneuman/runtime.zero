use std::collections::BTreeSet;
#[cfg(target_os = "linux")]
use std::env;
#[cfg(target_os = "linux")]
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::time::Instant;

use rz0_inventory_contract::AppRecord;

use super::desktop_entry::{DesktopEntry, read_desktop_entry};
use super::{
    MAX_APP_RECORDS, PlatformAppCollection, RootSpec, finish_collection, fnv1a, has_extension,
    open_root,
};

#[cfg(target_os = "linux")]
const MAX_APP_ROOTS: usize = 32;

#[cfg(target_os = "linux")]
pub(super) fn collect_installed_apps() -> PlatformAppCollection {
    let (roots, warnings) = application_roots();
    collect_roots(&roots, warnings)
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
                    let identity = format!("{}|{desktop_id}", name.to_ascii_lowercase());
                    apps.push(AppRecord {
                        id: format!("linux.app.{:016x}", fnv1a(identity.as_bytes())),
                        name,
                        source_id: "linux.desktop_entries".to_string(),
                        version: None,
                        publisher: None,
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
