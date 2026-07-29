use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::path_inventory::{PathCandidate, PathCollection, normalize_candidates};

const MAX_FIXTURE_BYTES: u64 = rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct InventoryFixture {
    schema_version: u16,
    platform: String,
    path_entries: Vec<FixturePathEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FixturePathEntry {
    path: String,
    exists: bool,
    entry_kind: String,
    #[serde(default)]
    warnings: Vec<String>,
}

pub fn load_path_fixture(path: &Path) -> Result<PathCollection, String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("failed to inspect fixture: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("fixture must be a regular file and must not be a symlink".to_string());
    }
    if metadata.len() > MAX_FIXTURE_BYTES {
        return Err(format!(
            "fixture exceeds the {MAX_FIXTURE_BYTES} byte limit"
        ));
    }
    let source = fs::read_to_string(path)
        .map_err(|error| format!("failed to read fixture as UTF-8: {error}"))?;
    let fixture: InventoryFixture =
        serde_json::from_str(&source).map_err(|error| format!("invalid fixture JSON: {error}"))?;
    if fixture.schema_version != 1 {
        return Err("fixture schema_version must be 1".to_string());
    }
    if !matches!(fixture.platform.as_str(), "windows" | "macos" | "linux") {
        return Err(format!(
            "unsupported fixture platform '{}'",
            fixture.platform
        ));
    }

    let candidates = fixture
        .path_entries
        .into_iter()
        .map(|entry| PathCandidate {
            path: entry.path,
            exists: entry.exists,
            entry_kind: entry.entry_kind,
            warnings: entry.warnings,
        })
        .collect();
    let mut collection = normalize_candidates("fixture", &fixture.platform, candidates)?;
    collection.source.id = "fixture.process_path".to_string();
    collection.source.kind = "fixture".to_string();
    collection.source.duration_ms = Some(0);
    Ok(collection)
}
