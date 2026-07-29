#[cfg(target_os = "macos")]
use std::env;
#[cfg(target_os = "macos")]
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::time::Instant;

use rz0_inventory_contract::AppRecord;

use super::{
    MAX_APP_RECORDS, PlatformAppCollection, RootSpec, finish_collection, fnv1a, has_extension,
    open_root, sanitize_text,
};

#[cfg(target_os = "macos")]
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
            apps.push(AppRecord {
                id: format!("macos.app.{:016x}", fnv1a(identity.as_bytes())),
                name,
                source_id: "macos.application_bundles".to_string(),
                version: None,
                publisher: None,
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
    fn collection_is_shallow_and_bounded_to_app_directories() {
        let collection = collect_roots(
            &[RootSpec {
                path: fixture_root(),
                label: "fixture".to_string(),
            }],
            Vec::new(),
        );
        assert_eq!(collection.source.status, "ok");
        assert_eq!(collection.apps.len(), 1);
        assert_eq!(collection.apps[0].name, "Alpha");
    }

    fn fixture_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("platform-apps")
            .join("macos")
    }
}
