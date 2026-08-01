use serde::{Deserialize, Serialize};

use crate::UpdateRecord;

const MAX_MANAGER_OUTPUT_BYTES: u64 = rz0_resource_contract::MAX_FINDING_REPORT_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagerKind {
    HomebrewFormula,
    HomebrewCask,
    MacPorts,
    Winget,
    Apt,
    Dnf,
    Pacman,
    Zypper,
    Snap,
    Flatpak,
}

impl ManagerKind {
    pub const fn id(self) -> &'static str {
        match self {
            Self::HomebrewFormula => "homebrew-formula",
            Self::HomebrewCask => "homebrew-cask",
            Self::MacPorts => "macports",
            Self::Winget => "winget",
            Self::Apt => "apt",
            Self::Dnf => "dnf",
            Self::Pacman => "pacman",
            Self::Zypper => "zypper",
            Self::Snap => "snap",
            Self::Flatpak => "flatpak",
        }
    }

    pub const fn platform(self) -> &'static str {
        match self {
            Self::HomebrewFormula | Self::HomebrewCask | Self::MacPorts => "macos",
            Self::Winget => "windows",
            Self::Apt | Self::Dnf | Self::Pacman | Self::Zypper | Self::Snap | Self::Flatpak => {
                "linux"
            }
        }
    }

    pub const fn query_arguments(self) -> &'static [&'static str] {
        match self {
            Self::HomebrewFormula => &["outdated", "--json=v2"],
            Self::HomebrewCask => &["outdated", "--cask", "--json=v2"],
            Self::MacPorts => &["outdated"],
            Self::Winget => &[
                "list",
                "--upgrade-available",
                "--accept-source-agreements",
                "--disable-interactivity",
            ],
            Self::Apt => &["list", "--upgradable"],
            Self::Dnf => &["check-update"],
            Self::Pacman => &["-Qu"],
            Self::Zypper => &["list-updates"],
            Self::Snap => &["refresh", "--list"],
            Self::Flatpak => &["remote-ls", "--updates"],
        }
    }

    pub const fn upgrade_arguments(self, package: &str) -> [&str; 3] {
        match self {
            Self::HomebrewFormula => ["upgrade", package, ""],
            Self::HomebrewCask => ["upgrade", "--cask", package],
            Self::MacPorts => ["upgrade", package, ""],
            Self::Winget => ["upgrade", "--id", package],
            Self::Apt => ["install", "--only-upgrade", package],
            Self::Dnf => ["upgrade", package, ""],
            Self::Pacman => ["-S", package, ""],
            Self::Zypper => ["update", package, ""],
            Self::Snap => ["refresh", package, ""],
            Self::Flatpak => ["update", package, ""],
        }
    }

    pub const fn manager_name(self) -> &'static str {
        match self {
            Self::HomebrewFormula | Self::HomebrewCask => "homebrew",
            Self::MacPorts => "macports",
            Self::Winget => "winget",
            Self::Apt => "apt",
            Self::Dnf => "dnf",
            Self::Pacman => "pacman",
            Self::Zypper => "zypper",
            Self::Snap => "snap",
            Self::Flatpak => "flatpak",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ManagerProbeSpec {
    pub manager: ManagerKind,
    pub platform: &'static str,
    pub executable_candidates: &'static [&'static str],
    pub query_arguments: &'static [&'static str],
    pub network_required: bool,
    pub requires_elevation: bool,
    pub read_only: bool,
}

const HOMEBREW_EXECUTABLES: &[&str] = &["/opt/homebrew/bin/brew", "/usr/local/bin/brew"];
const MACPORTS_EXECUTABLES: &[&str] = &["/opt/local/bin/port"];
const WINGET_EXECUTABLES: &[&str] = &[
    r"C:\Windows\System32\winget.exe",
    r"C:\Program Files\WindowsApps\Microsoft.DesktopAppInstaller\winget.exe",
];
const APT_EXECUTABLES: &[&str] = &["/usr/bin/apt"];
const DNF_EXECUTABLES: &[&str] = &["/usr/bin/dnf"];
const PACMAN_EXECUTABLES: &[&str] = &["/usr/bin/pacman"];
const ZYPPER_EXECUTABLES: &[&str] = &["/usr/bin/zypper"];
const SNAP_EXECUTABLES: &[&str] = &["/usr/bin/snap"];
const FLATPAK_EXECUTABLES: &[&str] = &["/usr/bin/flatpak"];

pub fn manager_probe_specs() -> Vec<ManagerProbeSpec> {
    vec![
        spec(
            ManagerKind::HomebrewFormula,
            HOMEBREW_EXECUTABLES,
            true,
            false,
        ),
        spec(ManagerKind::HomebrewCask, HOMEBREW_EXECUTABLES, true, false),
        spec(ManagerKind::MacPorts, MACPORTS_EXECUTABLES, true, false),
        spec(ManagerKind::Winget, WINGET_EXECUTABLES, true, false),
        spec(ManagerKind::Apt, APT_EXECUTABLES, true, true),
        spec(ManagerKind::Dnf, DNF_EXECUTABLES, true, true),
        spec(ManagerKind::Pacman, PACMAN_EXECUTABLES, true, true),
        spec(ManagerKind::Zypper, ZYPPER_EXECUTABLES, true, true),
        spec(ManagerKind::Snap, SNAP_EXECUTABLES, true, false),
        spec(ManagerKind::Flatpak, FLATPAK_EXECUTABLES, true, false),
    ]
}

pub fn manager_probe_specs_for_platform(platform: &str) -> Vec<ManagerProbeSpec> {
    manager_probe_specs()
        .into_iter()
        .filter(|spec| spec.platform == platform)
        .collect()
}

pub fn manager_executable_allowed(manager: &str, platform: &str, executable: &str) -> bool {
    manager_probe_specs().into_iter().any(|spec| {
        spec.platform == platform
            && spec.manager.manager_name() == manager
            && spec.executable_candidates.contains(&executable)
    })
}

fn spec(
    manager: ManagerKind,
    executable_candidates: &'static [&'static str],
    network_required: bool,
    requires_elevation: bool,
) -> ManagerProbeSpec {
    ManagerProbeSpec {
        manager,
        platform: manager.platform(),
        executable_candidates,
        query_arguments: manager.query_arguments(),
        network_required,
        requires_elevation,
        read_only: true,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagerParseContext {
    pub manager: ManagerKind,
    pub executable: Option<String>,
    pub network_required: bool,
    pub requires_elevation: bool,
    pub rollback_supported: bool,
}

pub fn parse_manager_output(
    context: &ManagerParseContext,
    output: &[u8],
) -> Result<Vec<UpdateRecord>, String> {
    if output.is_empty() || output.len() as u64 > MAX_MANAGER_OUTPUT_BYTES {
        return Err("manager output is empty or exceeds the foundation ceiling".to_string());
    }
    let text = std::str::from_utf8(output).map_err(|_| {
        "manager output is not valid UTF-8; locale-safe parsing is required".to_string()
    })?;
    match context.manager {
        ManagerKind::HomebrewFormula | ManagerKind::HomebrewCask => parse_homebrew(context, text),
        ManagerKind::Apt => parse_apt(context, text),
        ManagerKind::Dnf => parse_dnf(context, text),
        ManagerKind::Pacman => parse_pacman(context, text),
        ManagerKind::MacPorts => parse_macports(context, text),
        ManagerKind::Winget | ManagerKind::Zypper | ManagerKind::Snap | ManagerKind::Flatpak => {
            Err(format!(
                "{} output parser is not yet locale-safe; source remains unavailable",
                context.manager.id()
            ))
        }
    }
}

fn parse_homebrew(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|error| format!("parse {} JSON: {error}", context.manager.id()))?;
    let key = match context.manager {
        ManagerKind::HomebrewFormula => "formulae",
        ManagerKind::HomebrewCask => "casks",
        _ => return Err("invalid Homebrew parser manager".to_string()),
    };
    let Some(records) = value.get(key).and_then(serde_json::Value::as_array) else {
        return Err(format!("Homebrew output does not contain '{key}' records"));
    };
    records
        .iter()
        .map(|record| {
            let name = required_string(record, "name")?;
            let installed = array_first_string(record, "installed_versions")
                .or_else(|| optional_string(record, "installed_version"));
            let available = optional_string(record, "current_version")
                .or_else(|| optional_string(record, "version"))
                .ok_or_else(|| format!("Homebrew record '{name}' has no available version"))?;
            make_record(context, &name, installed, available)
        })
        .collect()
}

fn parse_apt(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    let mut records = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("Listing") {
            continue;
        }
        let fields = line.split_whitespace().collect::<Vec<_>>();
        let Some(first) = fields.first().copied() else {
            continue;
        };
        let Some((name, _channel)) = first.split_once('/') else {
            continue;
        };
        let Some(available) = fields.get(1).copied() else {
            continue;
        };
        let installed = line
            .split_once("upgradable from: ")
            .and_then(|(_, value)| value.split(']').next())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        records.push(make_record(
            context,
            name,
            installed,
            available.to_string(),
        )?);
    }
    Ok(records)
}

fn parse_dnf(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    let mut records = Vec::new();
    for line in text.lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 3
            || fields[0].eq_ignore_ascii_case("package")
            || fields[0].starts_with("Last")
            || fields[0].starts_with("Obsoleting")
        {
            continue;
        }
        records.push(make_record(
            context,
            fields[0],
            None,
            fields[2].to_string(),
        )?);
    }
    Ok(records)
}

fn parse_pacman(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    let mut records = Vec::new();
    for line in text.lines() {
        let Some((left, available)) = line.split_once(" -> ") else {
            continue;
        };
        let mut fields = left.split_whitespace();
        let Some(name) = fields.next() else {
            continue;
        };
        let installed = fields.next().map(str::to_string);
        records.push(make_record(
            context,
            name,
            installed,
            available.trim().to_string(),
        )?);
    }
    Ok(records)
}

fn parse_macports(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    let mut records = Vec::new();
    for line in text.lines() {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        let Some(name) = fields.first().copied() else {
            continue;
        };
        let versions = fields
            .iter()
            .filter_map(|field| field.strip_prefix('@'))
            .collect::<Vec<_>>();
        if versions.len() < 2 {
            continue;
        }
        records.push(make_record(
            context,
            name,
            Some(versions[0].to_string()),
            versions[1].to_string(),
        )?);
    }
    Ok(records)
}

fn make_record(
    context: &ManagerParseContext,
    name: &str,
    installed_version: Option<String>,
    available_version: String,
) -> Result<UpdateRecord, String> {
    let slug = slug(name)?;
    for version in installed_version
        .iter()
        .chain(std::iter::once(&available_version))
    {
        if version.is_empty() || version.len() > 120 || version.chars().any(char::is_control) {
            return Err(format!("manager record '{name}' has an invalid version"));
        }
    }
    let arguments = context
        .manager
        .upgrade_arguments(name)
        .into_iter()
        .filter(|argument| !argument.is_empty())
        .map(str::to_string)
        .collect();
    Ok(UpdateRecord {
        finding_id: format!("update.{}.{}", context.manager.id(), slug),
        subject_reference: format!("package:{}:{}", context.manager.id(), slug),
        installed: true,
        manager_record_present: true,
        update_available: true,
        installed_version,
        available_version: Some(available_version),
        manager: Some(context.manager.manager_name().to_string()),
        executable: context.executable.clone(),
        arguments,
        network_required: context.network_required,
        requires_elevation: context.requires_elevation,
        rollback_supported: context.rollback_supported,
    })
}

fn required_string(value: &serde_json::Value, field: &str) -> Result<String, String> {
    optional_string(value, field).ok_or_else(|| format!("manager record is missing '{field}'"))
}

fn optional_string(value: &serde_json::Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.is_empty() && !value.chars().any(char::is_control))
}

fn array_first_string(value: &serde_json::Value, field: &str) -> Option<String> {
    value
        .get(field)
        .and_then(serde_json::Value::as_array)
        .and_then(|values| values.iter().find_map(serde_json::Value::as_str))
        .map(str::to_string)
}

fn slug(value: &str) -> Result<String, String> {
    let mut result = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            result.push(character.to_ascii_lowercase());
        } else if matches!(character, '-' | '_' | '.' | ':') && !result.ends_with('-') {
            result.push('-');
        }
        if result.len() >= 80 {
            break;
        }
    }
    let result = result.trim_matches('-').to_string();
    if result.is_empty() {
        Err("manager package name has no safe identity characters".to_string())
    } else {
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(manager: ManagerKind) -> ManagerParseContext {
        ManagerParseContext {
            manager,
            executable: Some("/usr/bin/manager".to_string()),
            network_required: true,
            requires_elevation: false,
            rollback_supported: true,
        }
    }

    #[test]
    fn probe_specs_cover_all_named_platforms_without_authorizing_execution() {
        for platform in ["windows", "macos", "linux"] {
            let specs = manager_probe_specs_for_platform(platform);
            assert!(!specs.is_empty(), "{platform}");
            assert!(specs.iter().all(|spec| spec.read_only));
        }
    }

    #[test]
    fn manager_executable_policy_rejects_unlisted_paths_on_real_platforms() {
        assert!(manager_executable_allowed(
            "homebrew",
            "macos",
            "/opt/homebrew/bin/brew"
        ));
        assert!(!manager_executable_allowed(
            "homebrew",
            "macos",
            "/usr/bin/true"
        ));
    }

    #[test]
    fn parses_homebrew_json_without_running_homebrew() {
        let records = parse_manager_output(
            &context(ManagerKind::HomebrewFormula),
            br#"{"formulae":[{"name":"alpha","installed_versions":["1.0"],"current_version":"1.1"}]}"#,
        )
        .expect("Homebrew records");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].available_version.as_deref(), Some("1.1"));
        assert_eq!(records[0].arguments, ["upgrade", "alpha"]);
    }

    #[test]
    fn parses_apt_and_pacman_text_with_deterministic_ids() {
        let apt = parse_manager_output(
            &context(ManagerKind::Apt),
            b"Listing...\nalpha/stable 2.0 amd64 [upgradable from: 1.0]\n",
        )
        .expect("apt records");
        assert_eq!(apt[0].installed_version.as_deref(), Some("1.0"));
        assert_eq!(apt[0].available_version.as_deref(), Some("2.0"));
        let pacman = parse_manager_output(&context(ManagerKind::Pacman), b"alpha 1.0 -> 2.0\n")
            .expect("pacman records");
        assert_eq!(pacman[0].finding_id, "update.pacman.alpha");
    }

    #[test]
    fn locale_unsafe_adapters_fail_closed() {
        let result = parse_manager_output(&context(ManagerKind::Winget), b"localized output");
        assert!(result.is_err());
    }
}
