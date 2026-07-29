use std::collections::BTreeSet;
use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::time::Instant;

use rz0_inventory_contract::{InventorySource, PathEntry};

pub const MAX_PATH_ENTRIES: usize = rz0_resource_contract::MAX_INVENTORY_PATH_ENTRIES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathCandidate {
    pub path: String,
    pub exists: bool,
    pub entry_kind: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathCollection {
    pub source: InventorySource,
    pub entries: Vec<PathEntry>,
}

pub fn collect_process_path() -> PathCollection {
    collect_process_path_value(env::var_os("PATH"), env::consts::OS)
}

pub fn collect_process_path_value(value: Option<OsString>, platform: &str) -> PathCollection {
    let started = Instant::now();
    let mut source_warnings = Vec::new();
    let Some(value) = value else {
        return PathCollection {
            source: source(
                "process.path",
                "environment",
                "unavailable",
                elapsed_ms(started),
                vec!["PATH is not present in the process environment".to_string()],
            ),
            entries: Vec::new(),
        };
    };

    let mut candidates = Vec::new();
    for path in env::split_paths(&value).take(MAX_PATH_ENTRIES + 1) {
        if candidates.len() == MAX_PATH_ENTRIES {
            source_warnings.push(format!(
                "PATH entry limit of {MAX_PATH_ENTRIES} reached; remaining entries were skipped"
            ));
            break;
        }
        let display = path.to_string_lossy();
        let mut warnings = Vec::new();
        if matches!(&display, std::borrow::Cow::Owned(_)) {
            warnings
                .push("path contained non-Unicode data and was lossily represented".to_string());
        }
        let (exists, entry_kind) = classify_path(&path);
        if !exists {
            warnings.push("path does not exist".to_string());
        } else if entry_kind == "file" {
            warnings.push("PATH entry is a file, not a directory".to_string());
        }
        candidates.push(PathCandidate {
            path: display.into_owned(),
            exists,
            entry_kind,
            warnings,
        });
    }

    match normalize_candidates("process", platform, candidates) {
        Ok(mut collection) => {
            collection.source.duration_ms = Some(elapsed_ms(started));
            collection.source.warnings.extend(source_warnings);
            if !collection.source.warnings.is_empty()
                || collection
                    .entries
                    .iter()
                    .any(|entry| !entry.warnings.is_empty())
            {
                collection.source.status = "partial".to_string();
            }
            collection
        }
        Err(error) => PathCollection {
            source: source(
                "process.path",
                "environment",
                "error",
                elapsed_ms(started),
                vec![error],
            ),
            entries: Vec::new(),
        },
    }
}

#[cfg(windows)]
pub fn inspect_path_candidate(path: String) -> PathCandidate {
    let (exists, entry_kind) = classify_path(Path::new(&path));
    let mut warnings = Vec::new();
    if !exists {
        warnings.push("path does not exist".to_string());
    } else if entry_kind == "file" {
        warnings.push("PATH entry is a file, not a directory".to_string());
    }
    PathCandidate {
        path,
        exists,
        entry_kind,
        warnings,
    }
}

pub fn normalize_candidates(
    scope: &str,
    platform: &str,
    candidates: Vec<PathCandidate>,
) -> Result<PathCollection, String> {
    if !matches!(scope, "process" | "user" | "machine" | "fixture") {
        return Err(format!("unsupported PATH scope '{scope}'"));
    }
    if candidates.len() > MAX_PATH_ENTRIES {
        return Err(format!(
            "PATH source exceeds the {MAX_PATH_ENTRIES} entry limit"
        ));
    }

    let mut seen = BTreeSet::new();
    let mut entries = Vec::with_capacity(candidates.len());
    for (index, candidate) in candidates.into_iter().enumerate() {
        validate_candidate(&candidate)?;
        let mut warnings = candidate.warnings;
        let key = comparison_key(&candidate.path, platform);
        if !seen.insert(key) {
            warnings.push("duplicate PATH entry in this scope".to_string());
        }
        entries.push(PathEntry {
            path: candidate.path,
            scope: scope.to_string(),
            order: u32::try_from(index).unwrap_or(u32::MAX),
            exists: candidate.exists,
            entry_kind: candidate.entry_kind,
            warnings,
        });
    }

    let status = if entries.is_empty() {
        "unavailable"
    } else if entries.iter().any(|entry| !entry.warnings.is_empty()) {
        "partial"
    } else {
        "ok"
    };
    Ok(PathCollection {
        source: source(
            &format!("{scope}.path"),
            if scope == "fixture" {
                "fixture"
            } else {
                "environment"
            },
            status,
            0,
            Vec::new(),
        ),
        entries,
    })
}

fn validate_candidate(candidate: &PathCandidate) -> Result<(), String> {
    if candidate.path.trim().is_empty() {
        return Err("PATH entries must not be empty".to_string());
    }
    if candidate.path.chars().any(char::is_control) {
        return Err("PATH entries must not contain control characters".to_string());
    }
    if !matches!(
        candidate.entry_kind.as_str(),
        "directory" | "file" | "missing" | "unknown"
    ) {
        return Err(format!(
            "unsupported PATH entry kind '{}'",
            candidate.entry_kind
        ));
    }
    if candidate.exists && candidate.entry_kind == "missing" {
        return Err("existing PATH entries cannot use kind missing".to_string());
    }
    if !candidate.exists && matches!(candidate.entry_kind.as_str(), "directory" | "file") {
        return Err("absent PATH entries cannot claim a filesystem type".to_string());
    }
    Ok(())
}

fn classify_path(path: &Path) -> (bool, String) {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => (true, "directory".to_string()),
        Ok(metadata) if metadata.is_file() => (true, "file".to_string()),
        Ok(_) => (true, "unknown".to_string()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            (false, "missing".to_string())
        }
        Err(_) => (false, "unknown".to_string()),
    }
}

fn comparison_key(path: &str, platform: &str) -> String {
    let trimmed = path.trim_end_matches(['/', '\\']);
    if platform == "windows" {
        trimmed.replace('/', "\\").to_ascii_lowercase()
    } else {
        trimmed.to_string()
    }
}

fn source(
    id: &str,
    kind: &str,
    status: &str,
    duration_ms: u64,
    warnings: Vec<String>,
) -> InventorySource {
    InventorySource {
        id: id.to_string(),
        kind: kind.to_string(),
        status: status.to_string(),
        duration_ms: Some(duration_ms),
        read_only: true,
        warnings,
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}
