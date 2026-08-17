use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use rz0_inventory_contract::{InventorySource, PathEntry, ToolRecord};

use crate::command_probe::run_version_probe;
use crate::tool_specs::{ToolSpec, tool_specs};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCollection {
    pub source: InventorySource,
    pub tools: Vec<ToolRecord>,
}

pub fn discover_known_tools(path_entries: &[PathEntry], probe_versions: bool) -> ToolCollection {
    let started = Instant::now();
    let mut tools = Vec::new();
    let mut source_warnings = Vec::new();

    for spec in tool_specs() {
        if let Some((path, source_id)) = find_tool(spec.names, path_entries) {
            let mut warnings = Vec::new();
            let version = if probe_versions {
                probe_version(spec, &path, &mut warnings)
            } else {
                None
            };
            tools.push(ToolRecord {
                id: spec.id.to_string(),
                display_name: spec.display_name.to_string(),
                category: spec.category.to_string(),
                executable_path: Some(path.display().to_string()),
                version,
                source_ids: vec![source_id],
                confidence: "exact_path_match".to_string(),
                warnings,
            });
        }
    }

    let known_paths = tools
        .iter()
        .filter_map(|tool| tool.executable_path.as_deref())
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    let known_names = tool_specs()
        .iter()
        .flat_map(|spec| spec.names.iter().copied())
        .map(|name| {
            name.trim_end_matches(".cmd")
                .trim_end_matches(".exe")
                .to_string()
        })
        .collect::<BTreeSet<_>>();
    let (mut path_tools, path_truncated) =
        discover_path_executables(path_entries, &known_paths, &known_names);
    if !path_tools.is_empty() {
        source_warnings.push(
            "unrecognized PATH executables are inventory-only until a provider adapter proves a safe update channel"
                .to_string(),
        );
    }
    if path_truncated {
        source_warnings.push(
            "the bounded PATH executable inventory reached its record ceiling; remaining entries were skipped"
                .to_string(),
        );
    }
    tools.append(&mut path_tools);

    tools.sort_by(|left, right| left.id.cmp(&right.id));
    if probe_versions && tools.is_empty() {
        source_warnings.push("no known executable was available for version probing".to_string());
    }
    let status = if tools.is_empty() {
        "unavailable"
    } else if !source_warnings.is_empty() || tools.iter().any(|tool| !tool.warnings.is_empty()) {
        "partial"
    } else {
        "ok"
    };
    ToolCollection {
        source: InventorySource {
            id: "known.executables".to_string(),
            kind: "filesystem".to_string(),
            status: status.to_string(),
            duration_ms: Some(elapsed_ms(started)),
            read_only: true,
            warnings: source_warnings,
        },
        tools,
    }
}

fn discover_path_executables(
    path_entries: &[PathEntry],
    known_paths: &BTreeSet<String>,
    known_names: &BTreeSet<String>,
) -> (Vec<ToolRecord>, bool) {
    let mut tools = Vec::new();
    let mut seen_paths = known_paths.clone();
    let mut seen_names = known_names.clone();
    let mut truncated = false;
    let remaining_capacity =
        rz0_resource_contract::MAX_INVENTORY_TOOL_RECORDS.saturating_sub(known_paths.len());
    for entry in path_entries {
        if !entry.exists || entry.entry_kind != "directory" {
            continue;
        }
        if is_system_tool_directory(Path::new(&entry.path)) {
            continue;
        }
        let Ok(entries) = fs::read_dir(&entry.path) else {
            continue;
        };
        for directory_entry in entries.flatten() {
            if tools.len() == remaining_capacity {
                truncated = true;
                return (tools, truncated);
            }
            let path = directory_entry.path();
            let display_path = path.display().to_string();
            if !seen_paths.insert(display_path.clone()) || !is_executable_file(&path) {
                continue;
            }
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if name.is_empty() || name.len() > 160 || name.chars().any(char::is_control) {
                continue;
            }
            if !seen_names.insert(name.to_string()) {
                continue;
            }
            tools.push(ToolRecord {
                id: format!("path.{:016x}", fnv1a(display_path.as_bytes())),
                display_name: name.to_string(),
                category: "path_executable".to_string(),
                executable_path: Some(display_path),
                version: None,
                source_ids: vec!["known.executables".to_string()],
                confidence: "observed_path".to_string(),
                warnings: Vec::new(),
            });
        }
    }
    (tools, truncated)
}

fn is_system_tool_directory(path: &Path) -> bool {
    let normalized = path.to_string_lossy().replace('\\', "/");
    matches!(
        normalized.as_str(),
        "/bin" | "/sbin" | "/usr/bin" | "/usr/sbin" | "/usr/libexec"
    ) || normalized.starts_with("/System/")
        || normalized.starts_with("/Library/Apple/")
        || normalized.starts_with("C:/Windows/")
        || normalized.starts_with("C:/Program Files/WindowsApps/")
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if !metadata.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
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

fn probe_version(spec: &ToolSpec, path: &Path, warnings: &mut Vec<String>) -> Option<String> {
    match path_contains_symlink_or_reparse(path) {
        Ok(true) => {
            warnings.push(
                "version probe refused a path containing a symlink or reparse point".to_string(),
            );
            return None;
        }
        Ok(false) => {}
        Err(error) => {
            warnings.push(format!("version probe path inspection failed: {error}"));
            return None;
        }
    }
    let Some(args) = spec.version_args else {
        warnings.push("version probe is disabled for command-script executables".to_string());
        return None;
    };
    match run_version_probe(path, args) {
        Ok(version) => Some(version),
        Err(error) => {
            warnings.push(error);
            None
        }
    }
}

fn find_tool(names: &[&str], path_entries: &[PathEntry]) -> Option<(PathBuf, String)> {
    for entry in path_entries {
        if !entry.exists || entry.entry_kind != "directory" {
            continue;
        }
        for name in names {
            let candidate = Path::new(&entry.path).join(name);
            if fs::metadata(&candidate).is_ok_and(|metadata| metadata.is_file()) {
                return Some((candidate, source_id_for_scope(&entry.scope)));
            }
        }
    }
    None
}

fn path_contains_symlink_or_reparse(path: &Path) -> Result<bool, String> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if matches!(component, std::path::Component::Prefix(_)) {
            continue;
        }
        let metadata = fs::symlink_metadata(&current)
            .map_err(|error| format!("could not inspect an executable path component: {error}"))?;
        if metadata.file_type().is_symlink() || is_reparse_point(&metadata) {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(windows)]
fn is_reparse_point(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0400;
    metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0
}

#[cfg(not(windows))]
fn is_reparse_point(_metadata: &fs::Metadata) -> bool {
    false
}

fn source_id_for_scope(scope: &str) -> String {
    match scope {
        "process" => "process.path",
        "user" => "user.path",
        "machine" => "machine.path",
        "fixture" => "fixture.process_path",
        other => other,
    }
    .to_string()
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn detects_symlinked_probe_path() {
        use std::os::unix::fs::symlink;
        use std::time::{SystemTime, UNIX_EPOCH};

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("rz0-probe-link-{stamp}"));
        fs::create_dir_all(&root).expect("temp root");
        let target = root.join("target");
        fs::write(&target, b"fixture").expect("target");
        let link = root.join("link");
        symlink(&target, &link).expect("symlink");
        assert!(path_contains_symlink_or_reparse(&link).expect("inspection"));
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn no_existing_directories_produces_no_tools() {
        let collection = discover_known_tools(&[], false);
        assert!(collection.tools.is_empty());
        assert_eq!(collection.source.status, "unavailable");
    }

    #[test]
    fn generic_path_inventory_excludes_os_owned_tool_roots() {
        assert!(is_system_tool_directory(Path::new("/usr/bin")));
        assert!(is_system_tool_directory(Path::new("/System/Applications")));
        assert!(!is_system_tool_directory(Path::new("/opt/homebrew/bin")));
        assert!(!is_system_tool_directory(Path::new(
            "/Users/test/.local/bin"
        )));
    }
}
