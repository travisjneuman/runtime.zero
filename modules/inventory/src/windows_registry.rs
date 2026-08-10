#![cfg(windows)]

use std::collections::BTreeSet;
use std::io;
use std::time::Instant;

use rz0_inventory_contract::{AppRecord, InventorySource, ServiceRecord, SoftwareIdentifier};
use sha2::{Digest, Sha256};
use winreg::enums::{
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
};
use winreg::{HKEY, RegKey};

use crate::path_inventory::{
    MAX_PATH_ENTRIES, PathCollection, inspect_path_candidate, normalize_candidates,
};

const USER_ENVIRONMENT_KEY: &str = "Environment";
const MACHINE_ENVIRONMENT_KEY: &str =
    r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";
const UNINSTALL_KEY: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
const SERVICES_KEY: &str = r"SYSTEM\CurrentControlSet\Services";
const MAX_APP_RECORDS: usize = rz0_resource_contract::MAX_INVENTORY_APP_RECORDS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsAppCollection {
    pub source: InventorySource,
    pub apps: Vec<AppRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowsServiceCollection {
    pub source: InventorySource,
    pub services: Vec<ServiceRecord>,
}

pub fn collect_persisted_paths() -> Vec<PathCollection> {
    vec![
        read_path_value(HKEY_CURRENT_USER, USER_ENVIRONMENT_KEY, "user", "user.path"),
        read_path_value(
            HKEY_LOCAL_MACHINE,
            MACHINE_ENVIRONMENT_KEY,
            "machine",
            "machine.path",
        ),
    ]
}

pub fn collect_installed_apps() -> WindowsAppCollection {
    let started = Instant::now();
    let mut apps = Vec::new();
    let mut warnings = Vec::new();
    let mut opened_roots = 0usize;
    let mut limit_reached = false;
    let roots = [
        (HKEY_CURRENT_USER, KEY_READ | KEY_WOW64_64KEY, "user64"),
        (HKEY_CURRENT_USER, KEY_READ | KEY_WOW64_32KEY, "user32"),
        (HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_64KEY, "machine64"),
        (HKEY_LOCAL_MACHINE, KEY_READ | KEY_WOW64_32KEY, "machine32"),
    ];

    for (hive, flags, scope) in roots {
        match RegKey::predef(hive).open_subkey_with_flags(UNINSTALL_KEY, flags) {
            Ok(root) => {
                opened_roots += 1;
                limit_reached |= collect_apps_from_root(&root, scope, &mut apps, &mut warnings);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => warnings.push(format!(
                "{scope} uninstall inventory was unavailable: {error}"
            )),
        }
    }

    deduplicate_apps(&mut apps);
    apps.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then_with(|| left.version.cmp(&right.version))
    });
    if limit_reached {
        warnings.push(format!(
            "application record limit of {MAX_APP_RECORDS} reached; remaining records were skipped"
        ));
    }
    let status = if opened_roots == 0 {
        "unavailable"
    } else if warnings.is_empty() {
        "ok"
    } else {
        "partial"
    };
    WindowsAppCollection {
        source: InventorySource {
            id: "windows.installed_apps".to_string(),
            kind: "registry".to_string(),
            status: status.to_string(),
            duration_ms: Some(elapsed_ms(started)),
            read_only: true,
            warnings,
        },
        apps,
    }
}

pub fn collect_services() -> WindowsServiceCollection {
    let started = Instant::now();
    let mut warnings = Vec::new();
    let mut services = Vec::new();
    let root =
        match RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(SERVICES_KEY, KEY_READ) {
            Ok(root) => Some(root),
            Err(error) => {
                warnings.push(format!(
                    "Windows service inventory was unavailable: {error}"
                ));
                None
            }
        };
    if let Some(root) = root.as_ref() {
        let mut skipped = 0usize;
        for key_result in root.enum_keys() {
            let Ok(key_name) = key_result else {
                skipped = skipped.saturating_add(1);
                continue;
            };
            if services.len() == rz0_resource_contract::MAX_INVENTORY_SERVICE_RECORDS {
                warnings.push(format!(
                    "service record limit of {} reached; remaining records were skipped",
                    rz0_resource_contract::MAX_INVENTORY_SERVICE_RECORDS
                ));
                break;
            }
            let Ok(key) = root.open_subkey_with_flags(&key_name, KEY_READ) else {
                skipped = skipped.saturating_add(1);
                continue;
            };
            let service_type = key.get_value::<u32, _>("Type").unwrap_or_default();
            if service_type == 0 {
                continue;
            }
            let name = optional_text(&key, "DisplayName", 240)
                .or_else(|| sanitize_registry_text(&key_name, 240));
            let Some(name) = name else {
                continue;
            };
            let start = key.get_value::<u32, _>("Start").ok();
            let enabled = match start {
                Some(0..=3) => Some(true),
                Some(4) => Some(false),
                _ => None,
            };
            let location = optional_text(&key, "ImagePath", 2048);
            let identity = format!("windows.service|{}", key_name.to_ascii_lowercase());
            services.push(ServiceRecord {
                id: format!("windows.service.{:016x}", fnv1a(identity.as_bytes())),
                name,
                source_id: "windows.services".to_string(),
                kind: "service".to_string(),
                scope: "system".to_string(),
                enabled,
                location,
                warnings: Vec::new(),
            });
        }
        if skipped > 0 {
            warnings.push(format!(
                "{skipped} Windows service registry records could not be read"
            ));
        }
    }
    services.sort_by(|left, right| left.id.cmp(&right.id));
    let status = if root.is_none() {
        "unavailable"
    } else if warnings.is_empty() {
        "ok"
    } else {
        "partial"
    };
    WindowsServiceCollection {
        source: InventorySource {
            id: "windows.services".to_string(),
            kind: "registry".to_string(),
            status: status.to_string(),
            duration_ms: Some(elapsed_ms(started)),
            read_only: true,
            warnings,
        },
        services,
    }
}

fn read_path_value(hive: HKEY, subkey: &str, scope: &str, source_id: &str) -> PathCollection {
    let started = Instant::now();
    let result = RegKey::predef(hive)
        .open_subkey_with_flags(subkey, KEY_READ)
        .and_then(|key| key.get_value::<String, _>("Path"));
    let value = match result {
        Ok(value) => value,
        Err(error) => {
            return PathCollection {
                source: InventorySource {
                    id: source_id.to_string(),
                    kind: "registry".to_string(),
                    status: "unavailable".to_string(),
                    duration_ms: Some(elapsed_ms(started)),
                    read_only: true,
                    warnings: vec![format!("persisted {scope} PATH was unavailable: {error}")],
                },
                entries: Vec::new(),
            };
        }
    };

    let mut source_warnings = Vec::new();
    let mut candidates = Vec::new();
    for raw in value.split(';') {
        if candidates.len() == MAX_PATH_ENTRIES {
            source_warnings.push(format!(
                "PATH entry limit of {MAX_PATH_ENTRIES} reached; remaining entries were skipped"
            ));
            break;
        }
        let path = raw.trim();
        if path.is_empty() {
            source_warnings.push("an empty persisted PATH entry was skipped".to_string());
            continue;
        }
        let mut candidate = inspect_path_candidate(path.to_string());
        if path.contains('%') {
            candidate.warnings.push(
                "path contains an unexpanded environment reference; existence may be unknown"
                    .to_string(),
            );
        }
        candidates.push(candidate);
    }

    match normalize_candidates(scope, "windows", candidates) {
        Ok(mut collection) => {
            collection.source.id = source_id.to_string();
            collection.source.kind = "registry".to_string();
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
            source: InventorySource {
                id: source_id.to_string(),
                kind: "registry".to_string(),
                status: "error".to_string(),
                duration_ms: Some(elapsed_ms(started)),
                read_only: true,
                warnings: vec![error],
            },
            entries: Vec::new(),
        },
    }
}

fn collect_apps_from_root(
    root: &RegKey,
    scope: &str,
    apps: &mut Vec<AppRecord>,
    warnings: &mut Vec<String>,
) -> bool {
    for key_name in root.enum_keys().filter_map(Result::ok) {
        if apps.len() >= MAX_APP_RECORDS {
            return true;
        }
        let Ok(key) = root.open_subkey_with_flags(&key_name, KEY_READ) else {
            continue;
        };
        if key.get_value::<u32, _>("SystemComponent").unwrap_or(0) == 1 {
            continue;
        }
        let Ok(name) = key.get_value::<String, _>("DisplayName") else {
            continue;
        };
        let Some(name) = sanitize_registry_text(&name, 240) else {
            warnings.push(format!("an invalid {scope} application name was skipped"));
            continue;
        };
        let version = optional_text(&key, "DisplayVersion", 120);
        let publisher = optional_text(&key, "Publisher", 160);
        let install_location = optional_text(&key, "InstallLocation", 1024);
        let normalized_key = key_name.to_ascii_lowercase();
        let identity = format!("{scope}|{normalized_key}");
        let identifiers = sanitize_registry_text(&key_name, 256)
            .map(|value| {
                let (kind, value) = if is_product_code(&value) {
                    ("product_code", value.to_ascii_lowercase())
                } else {
                    (
                        "registry_product_key_digest",
                        format!("{:x}", Sha256::digest(value.as_bytes())),
                    )
                };
                vec![SoftwareIdentifier {
                    kind: kind.to_string(),
                    value,
                }]
            })
            .unwrap_or_default();
        apps.push(AppRecord {
            id: format!("windows.app.{:016x}", fnv1a(identity.as_bytes())),
            name,
            source_id: "windows.installed_apps".to_string(),
            version,
            publisher,
            identifiers,
            install_location,
            warnings: Vec::new(),
        });
    }
    false
}

fn optional_text(key: &RegKey, name: &str, max_len: usize) -> Option<String> {
    key.get_value::<String, _>(name)
        .ok()
        .and_then(|value| sanitize_registry_text(&value, max_len))
}

fn sanitize_registry_text(value: &str, max_len: usize) -> Option<String> {
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

fn is_product_code(value: &str) -> bool {
    value.len() == 38
        && value.starts_with('{')
        && value.ends_with('}')
        && value.bytes().enumerate().all(|(index, byte)| match index {
            0 => byte == b'{',
            9 | 14 | 19 | 24 => byte == b'-',
            37 => byte == b'}',
            _ => byte.is_ascii_hexdigit(),
        })
}

fn deduplicate_apps(apps: &mut Vec<AppRecord>) {
    let mut seen = BTreeSet::new();
    apps.retain(|app| seen.insert(app.id.clone()));
}

fn fnv1a(value: &[u8]) -> u64 {
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
