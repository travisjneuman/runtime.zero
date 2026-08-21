use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use quick_xml::{Reader, events::Event};
use serde::{Deserialize, Serialize};

use crate::UpdateRecord;

const MAX_MANAGER_OUTPUT_BYTES: u64 = rz0_resource_contract::MAX_FINDING_REPORT_BYTES;
const MAX_UPDATE_RECORDS: usize = rz0_resource_contract::MAX_FINDINGS;
const MAX_PACKAGE_NAME_BYTES: usize = 240;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ManagerKind {
    HomebrewFormula,
    HomebrewCask,
    MacPorts,
    MacAppStore,
    AppleSoftwareUpdate,
    Winget,
    Apt,
    Dnf,
    Pacman,
    Zypper,
    Snap,
    Flatpak,
    NpmGlobal,
    Pip,
    RubyGems,
    Grok,
    Hermes,
    OhMyPi,
    Warp,
    Rustup,
    UvTools,
    Deno,
    Aiup,
    CargoInstall,
}

impl ManagerKind {
    pub const fn id(self) -> &'static str {
        match self {
            Self::HomebrewFormula => "homebrew-formula",
            Self::HomebrewCask => "homebrew-cask",
            Self::MacPorts => "macports",
            Self::MacAppStore => "mac-app-store",
            Self::AppleSoftwareUpdate => "apple-software-update",
            Self::Winget => "winget",
            Self::Apt => "apt",
            Self::Dnf => "dnf",
            Self::Pacman => "pacman",
            Self::Zypper => "zypper",
            Self::Snap => "snap",
            Self::Flatpak => "flatpak",
            Self::NpmGlobal => "npm-global",
            Self::Pip => "pip",
            Self::RubyGems => "ruby-gems",
            Self::Grok => "grok",
            Self::Hermes => "hermes",
            Self::OhMyPi => "oh-my-pi",
            Self::Warp => "warp",
            Self::Rustup => "rustup",
            Self::UvTools => "uv-tools",
            Self::Deno => "deno",
            Self::Aiup => "aiup",
            Self::CargoInstall => "cargo-install",
        }
    }

    pub const fn platform(self) -> &'static str {
        match self {
            Self::HomebrewFormula
            | Self::HomebrewCask
            | Self::MacPorts
            | Self::MacAppStore
            | Self::AppleSoftwareUpdate => "macos",
            Self::Winget => "windows",
            Self::Apt | Self::Dnf | Self::Pacman | Self::Zypper | Self::Snap | Self::Flatpak => {
                "linux"
            }
            Self::NpmGlobal
            | Self::Pip
            | Self::RubyGems
            | Self::Grok
            | Self::Hermes
            | Self::OhMyPi => "any",
            Self::Warp => "any",
            Self::Rustup | Self::UvTools | Self::Deno | Self::Aiup | Self::CargoInstall => "any",
        }
    }

    pub const fn query_arguments(self) -> &'static [&'static str] {
        match self {
            Self::HomebrewFormula => &["outdated", "--json=v2"],
            Self::HomebrewCask => &["outdated", "--cask", "--greedy", "--json=v2"],
            Self::MacPorts => &["outdated"],
            Self::MacAppStore => &["outdated", "--json"],
            Self::AppleSoftwareUpdate => &["--list"],
            Self::Winget => &[
                "list",
                "--upgrade-available",
                "--accept-source-agreements",
                "--disable-interactivity",
            ],
            Self::Apt => &["list", "--upgradable"],
            Self::Dnf => &["check-update"],
            Self::Pacman => &["-Qu"],
            Self::Zypper => &[
                "--no-color",
                "--no-abbrev",
                "--non-interactive",
                "--no-refresh",
                "--xmlout",
                "list-updates",
            ],
            Self::Snap => &["refresh", "--list"],
            Self::Flatpak => &[
                "remote-ls",
                "--updates",
                "--app",
                "--json",
                "--columns=application,version,branch,arch,origin,commit",
            ],
            Self::NpmGlobal => &["outdated", "--global", "--json"],
            Self::Pip => &["-m", "pip", "list", "--outdated", "--format=json"],
            Self::RubyGems => &["outdated"],
            Self::Grok => &["update", "--check", "--json"],
            Self::Hermes => &["update", "--check"],
            Self::OhMyPi => &["update", "--check"],
            Self::Warp => &["--version"],
            Self::Rustup => &["check", "--no-self-update"],
            Self::UvTools => &["tool", "list", "--outdated"],
            Self::Deno => &["upgrade", "--dry-run"],
            Self::Aiup => &["--no-install", "--dry-run"],
            Self::CargoInstall => &["install", "--list"],
        }
    }

    pub fn upgrade_arguments(self, package: &str) -> Vec<String> {
        match self {
            Self::HomebrewFormula => vec!["upgrade".to_string(), package.to_string()],
            Self::HomebrewCask => vec![
                "upgrade".to_string(),
                "--cask".to_string(),
                "--greedy".to_string(),
                package.to_string(),
            ],
            Self::MacPorts => vec!["upgrade".to_string(), package.to_string()],
            Self::MacAppStore => vec!["upgrade".to_string(), package.to_string()],
            Self::AppleSoftwareUpdate => vec!["--install".to_string(), package.to_string()],
            Self::Winget => vec![
                "upgrade".to_string(),
                "--id".to_string(),
                package.to_string(),
            ],
            Self::Apt => vec![
                "install".to_string(),
                "--only-upgrade".to_string(),
                package.to_string(),
            ],
            Self::Dnf => vec!["upgrade".to_string(), package.to_string()],
            Self::Pacman => vec!["-S".to_string(), package.to_string()],
            Self::Zypper => vec!["update".to_string(), package.to_string()],
            Self::Snap => vec!["refresh".to_string(), package.to_string()],
            Self::Flatpak => vec!["update".to_string(), package.to_string()],
            Self::NpmGlobal => vec![
                "update".to_string(),
                "--global".to_string(),
                package.to_string(),
            ],
            Self::Pip => vec![
                "-m".to_string(),
                "pip".to_string(),
                "install".to_string(),
                "--upgrade".to_string(),
                package.to_string(),
            ],
            Self::RubyGems => vec!["update".to_string(), package.to_string()],
            Self::Grok => vec!["update".to_string(), "--stable".to_string()],
            Self::Hermes => vec!["update".to_string()],
            Self::OhMyPi => vec!["update".to_string()],
            Self::Warp => Vec::new(),
            Self::Rustup => vec!["update".to_string(), package.to_string()],
            Self::UvTools => vec![
                "tool".to_string(),
                "upgrade".to_string(),
                package.to_string(),
            ],
            Self::Deno => vec!["upgrade".to_string()],
            Self::Aiup | Self::CargoInstall => Vec::new(),
        }
    }

    pub const fn manager_name(self) -> &'static str {
        match self {
            Self::HomebrewFormula | Self::HomebrewCask => "homebrew",
            Self::MacPorts => "macports",
            Self::MacAppStore => "mas",
            Self::AppleSoftwareUpdate => "softwareupdate",
            Self::Winget => "winget",
            Self::Apt => "apt",
            Self::Dnf => "dnf",
            Self::Pacman => "pacman",
            Self::Zypper => "zypper",
            Self::Snap => "snap",
            Self::Flatpak => "flatpak",
            Self::NpmGlobal => "npm",
            Self::Pip => "python",
            Self::RubyGems => "gem",
            Self::Grok => "grok",
            Self::Hermes => "hermes",
            Self::OhMyPi => "omp",
            Self::Warp => "warp",
            Self::Rustup => "rustup",
            Self::UvTools => "uv",
            Self::Deno => "deno",
            Self::Aiup => "aiup",
            Self::CargoInstall => "cargo",
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

/// A provider specification resolved against the current machine. Static
/// manager specs are useful for explicit CLI selection; this resolved form is
/// what aggregate discovery uses for user-prefix and self-updating tools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProviderProbeSpec {
    pub manager: ManagerKind,
    pub instance_id: String,
    pub executable: PathBuf,
    pub query_arguments: Vec<String>,
    pub network_required: bool,
    pub requires_elevation: bool,
    pub read_only: bool,
    pub update_prefix: Option<PathBuf>,
}

const HOMEBREW_EXECUTABLES: &[&str] = &["/opt/homebrew/bin/brew", "/usr/local/bin/brew"];
const MACPORTS_EXECUTABLES: &[&str] = &["/opt/local/bin/port"];
const MAS_EXECUTABLES: &[&str] = &[
    "/opt/homebrew/bin/mas",
    "/usr/local/bin/mas",
    "/opt/local/bin/mas",
];
const SOFTWAREUPDATE_EXECUTABLES: &[&str] = &["/usr/sbin/softwareupdate"];
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
const NO_STATIC_EXECUTABLES: &[&str] = &[];

pub fn manager_probe_specs() -> Vec<ManagerProbeSpec> {
    vec![
        spec(
            ManagerKind::HomebrewFormula,
            HOMEBREW_EXECUTABLES,
            true,
            false,
        ),
        spec(ManagerKind::HomebrewCask, HOMEBREW_EXECUTABLES, true, false),
        spec(ManagerKind::MacPorts, MACPORTS_EXECUTABLES, true, true),
        spec(ManagerKind::MacAppStore, MAS_EXECUTABLES, true, false),
        spec(
            ManagerKind::AppleSoftwareUpdate,
            SOFTWAREUPDATE_EXECUTABLES,
            true,
            true,
        ),
        spec(ManagerKind::Winget, WINGET_EXECUTABLES, true, false),
        spec(ManagerKind::Apt, APT_EXECUTABLES, true, true),
        spec(ManagerKind::Dnf, DNF_EXECUTABLES, true, true),
        spec(ManagerKind::Pacman, PACMAN_EXECUTABLES, true, true),
        spec(ManagerKind::Zypper, ZYPPER_EXECUTABLES, true, true),
        spec(ManagerKind::Snap, SNAP_EXECUTABLES, true, false),
        spec(ManagerKind::Flatpak, FLATPAK_EXECUTABLES, true, false),
        spec(ManagerKind::NpmGlobal, NO_STATIC_EXECUTABLES, true, false),
        spec(ManagerKind::Pip, NO_STATIC_EXECUTABLES, true, false),
        spec(ManagerKind::RubyGems, NO_STATIC_EXECUTABLES, true, false),
        spec(ManagerKind::Grok, NO_STATIC_EXECUTABLES, true, false),
        spec(ManagerKind::Hermes, NO_STATIC_EXECUTABLES, true, false),
        spec(ManagerKind::OhMyPi, NO_STATIC_EXECUTABLES, true, false),
        spec(ManagerKind::Warp, NO_STATIC_EXECUTABLES, false, false),
        spec(ManagerKind::Rustup, NO_STATIC_EXECUTABLES, true, false),
        spec(ManagerKind::UvTools, NO_STATIC_EXECUTABLES, true, false),
        spec(ManagerKind::Deno, NO_STATIC_EXECUTABLES, true, false),
        spec(ManagerKind::Aiup, NO_STATIC_EXECUTABLES, false, false),
        spec(
            ManagerKind::CargoInstall,
            NO_STATIC_EXECUTABLES,
            false,
            false,
        ),
    ]
}

pub fn manager_probe_specs_for_platform(platform: &str) -> Vec<ManagerProbeSpec> {
    manager_probe_specs()
        .into_iter()
        .filter(|spec| spec.platform == platform || spec.platform == "any")
        .collect()
}

pub fn manager_executable_allowed(manager: &str, platform: &str, executable: &str) -> bool {
    if manager_probe_specs().into_iter().any(|spec| {
        spec.platform == platform
            && spec.manager.manager_name() == manager
            && spec.executable_candidates.contains(&executable)
    }) {
        return true;
    }
    custom_executable_allowed(manager, platform, executable)
}

/// Resolves every built-in provider whose exact executable is present. A
/// missing provider is intentionally absent here; the CLI reports that state
/// separately so absence is never mistaken for universal coverage.
pub fn discover_provider_specs_for_platform(platform: &str) -> Vec<ProviderProbeSpec> {
    let mut providers = Vec::new();
    for spec in manager_probe_specs_for_platform(platform) {
        for candidate in spec.executable_candidates {
            if let Some(executable) = resolve_direct_executable(Path::new(candidate)) {
                providers.push(ProviderProbeSpec {
                    manager: spec.manager,
                    instance_id: spec.manager.id().to_string(),
                    executable,
                    query_arguments: spec
                        .query_arguments
                        .iter()
                        .map(|argument| (*argument).to_string())
                        .collect(),
                    network_required: spec.network_required,
                    requires_elevation: spec.requires_elevation,
                    read_only: spec.read_only,
                    update_prefix: None,
                });
            }
        }
    }
    if platform == std::env::consts::OS {
        discover_dynamic_providers(&mut providers);
    }
    providers.sort_by(|left, right| {
        left.manager
            .id()
            .cmp(right.manager.id())
            .then_with(|| left.instance_id.cmp(&right.instance_id))
            .then_with(|| left.executable.cmp(&right.executable))
    });
    providers.dedup_by(|left, right| {
        left.manager == right.manager
            && left.instance_id == right.instance_id
            && left.executable == right.executable
            && left.query_arguments == right.query_arguments
    });
    providers
}

pub const fn dynamic_provider_ids() -> &'static [&'static str] {
    &[
        "npm-global",
        "pip",
        "ruby-gems",
        "grok",
        "hermes",
        "oh-my-pi",
        "warp",
        "rustup",
        "uv-tools",
        "deno",
        "aiup",
        "cargo-install",
    ]
}

fn discover_dynamic_providers(providers: &mut Vec<ProviderProbeSpec>) {
    #[cfg(target_os = "macos")]
    for (command, manager, instance_id) in [
        ("port", ManagerKind::MacPorts, "macports:default"),
        ("mas", ManagerKind::MacAppStore, "mac-app-store:default"),
    ] {
        if providers.iter().any(|provider| provider.manager == manager) {
            continue;
        }
        if let Some(executable) = find_command(command) {
            providers.push(ProviderProbeSpec {
                manager,
                instance_id: instance_id.to_string(),
                executable,
                query_arguments: manager
                    .query_arguments()
                    .iter()
                    .map(|argument| (*argument).to_string())
                    .collect(),
                network_required: true,
                requires_elevation: manager == ManagerKind::MacPorts,
                read_only: true,
                update_prefix: None,
            });
        }
    }

    if let Some(npm) = find_command("npm") {
        let mut prefixes = vec![None];
        for prefix in npm_prefix_candidates() {
            if prefix.is_dir() && !prefixes.iter().flatten().any(|known| known == &prefix) {
                prefixes.push(Some(prefix));
            }
        }
        for prefix in prefixes {
            let mut query_arguments = ManagerKind::NpmGlobal
                .query_arguments()
                .iter()
                .map(|argument| (*argument).to_string())
                .collect::<Vec<_>>();
            let instance_id = match prefix.as_ref() {
                Some(prefix) => {
                    query_arguments
                        .splice(2..2, ["--prefix".to_string(), prefix.display().to_string()]);
                    format!("npm-global:{}", prefix.display())
                }
                None => "npm-global:default".to_string(),
            };
            providers.push(ProviderProbeSpec {
                manager: ManagerKind::NpmGlobal,
                instance_id,
                executable: npm.clone(),
                query_arguments,
                network_required: true,
                requires_elevation: false,
                read_only: true,
                update_prefix: prefix,
            });
        }
    }

    if let Some(python) = find_command("python3").or_else(|| find_command("python")) {
        providers.push(ProviderProbeSpec {
            manager: ManagerKind::Pip,
            instance_id: "pip:default".to_string(),
            requires_elevation: python.starts_with("/usr/") || python.starts_with("/Library/"),
            executable: python,
            query_arguments: ManagerKind::Pip
                .query_arguments()
                .iter()
                .map(|argument| (*argument).to_string())
                .collect(),
            network_required: true,
            read_only: true,
            update_prefix: None,
        });
    }

    if let Some(gem) = find_command("gem") {
        providers.push(ProviderProbeSpec {
            manager: ManagerKind::RubyGems,
            instance_id: "ruby-gems:default".to_string(),
            requires_elevation: gem.starts_with("/usr/"),
            executable: gem,
            query_arguments: vec!["outdated".to_string()],
            network_required: true,
            read_only: true,
            update_prefix: None,
        });
    }

    for (command, manager, instance_id) in [
        ("grok", ManagerKind::Grok, "grok:stable"),
        ("hermes", ManagerKind::Hermes, "hermes:default"),
        ("omp", ManagerKind::OhMyPi, "oh-my-pi:default"),
        ("warp", ManagerKind::Warp, "warp:observed-only"),
        ("rustup", ManagerKind::Rustup, "rustup:default"),
        ("uv", ManagerKind::UvTools, "uv-tools:default"),
        ("deno", ManagerKind::Deno, "deno:default"),
        ("aiup", ManagerKind::Aiup, "aiup:managed"),
        ("cargo", ManagerKind::CargoInstall, "cargo-install:registry"),
    ] {
        if let Some(executable) = find_command(command) {
            providers.push(ProviderProbeSpec {
                manager,
                instance_id: instance_id.to_string(),
                executable,
                query_arguments: manager
                    .query_arguments()
                    .iter()
                    .map(|argument| (*argument).to_string())
                    .collect(),
                network_required: manager != ManagerKind::Warp,
                requires_elevation: false,
                read_only: true,
                update_prefix: None,
            });
        }
    }
}

fn npm_prefix_candidates() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return Vec::new();
    };
    let mut candidates = vec![
        home.join(".local/share/aiup/npm"),
        home.join(".npm-global"),
        home.join(".local/npm"),
    ];
    let share = home.join(".local/share");
    if let Ok(entries) = fs::read_dir(share) {
        for entry in entries.flatten().take(128) {
            let path = entry.path().join("npm");
            if path.join("lib/node_modules").is_dir() {
                candidates.push(path);
            }
        }
    }
    for (root, needs_installation_child) in [
        (home.join(".nvm/versions/node"), false),
        (home.join(".volta/tools/image/node"), false),
        (home.join(".asdf/installs/nodejs"), false),
        (home.join(".local/share/mise/installs/node"), false),
        (home.join(".local/share/fnm/node-versions"), true),
    ] {
        let Ok(entries) = fs::read_dir(root) else {
            continue;
        };
        for entry in entries.flatten().take(128) {
            let path = if needs_installation_child {
                entry.path().join("installation")
            } else {
                entry.path()
            };
            if path.join("lib/node_modules").is_dir() {
                candidates.push(path);
            }
        }
    }
    candidates.sort();
    candidates.dedup();
    candidates
}

fn find_command(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for directory in std::env::split_paths(&path) {
        let candidate = directory.join(name);
        if let Some(executable) = resolve_direct_executable(&candidate) {
            return Some(executable);
        }
    }
    None
}

fn resolve_direct_executable(path: &Path) -> Option<PathBuf> {
    let canonical = fs::canonicalize(path).ok()?;
    let metadata = fs::symlink_metadata(&canonical).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return None;
    }
    Some(canonical)
}

fn custom_executable_allowed(manager: &str, platform: &str, executable: &str) -> bool {
    if platform != "any" && platform != std::env::consts::OS {
        return false;
    }
    let path = Path::new(executable);
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return false;
    }
    let home = std::env::var_os("HOME")
        .as_deref()
        .map(Path::new)
        .map(Path::to_path_buf);
    let under_home = |suffix: &str| {
        home.as_deref()
            .map(|root| path.starts_with(root.join(suffix)))
            .unwrap_or(false)
    };
    let under_system_prefix = path.starts_with("/opt/homebrew") || path.starts_with("/usr/local");
    match manager {
        "macports" => {
            path.file_name().is_some_and(|name| name == "port")
                && (path.starts_with("/opt/local") || under_system_prefix)
        }
        "mas" => {
            path.file_name().is_some_and(|name| name == "mas")
                && (under_home(".local") || path.starts_with("/opt/local") || under_system_prefix)
        }
        "npm" => {
            path.file_name()
                .is_some_and(|name| matches!(name.to_str(), Some("npm" | "npm-cli.js")))
                && (under_home(".local") || under_home(".npm-global") || under_system_prefix)
        }
        "python" => {
            path.file_name().is_some_and(|name| {
                name.to_str().is_some_and(|value| {
                    matches!(value, "python" | "python3" | "pip" | "pip3")
                        || value.strip_prefix("python3.").is_some_and(|suffix| {
                            !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
                        })
                        || value.strip_prefix("pip3.").is_some_and(|suffix| {
                            !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_digit())
                        })
                })
            }) && (under_home(".local")
                || under_home(".pyenv")
                || under_system_prefix
                || path.starts_with("/Library/Frameworks/Python.framework")
                || path.starts_with("/System/Library/Frameworks/Python.framework")
                || path.starts_with("/usr"))
        }
        "gem" => {
            path.file_name().is_some_and(|name| name == "gem")
                && (under_home(".local") || under_system_prefix || path.starts_with("/usr"))
        }
        "grok" => {
            path.file_name().is_some_and(|name| {
                matches!(name.to_str(), Some("grok"))
                    || name
                        .to_str()
                        .is_some_and(|value| value.starts_with("grok-"))
            }) && (under_home(".grok") || under_home(".local/bin"))
        }
        "hermes" => {
            path.file_name().is_some_and(|name| {
                matches!(name.to_str(), Some("hermes"))
                    || name
                        .to_str()
                        .is_some_and(|value| value.starts_with("hermes-"))
            }) && (under_home(".local") || under_home(".hermes") || under_home(".cache"))
        }
        "omp" => {
            path.file_name().is_some_and(|name| {
                matches!(name.to_str(), Some("omp"))
                    || name.to_str().is_some_and(|value| value.starts_with("omp-"))
            }) && (under_home(".local") || under_home(".omp") || under_system_prefix)
        }
        "warp" => {
            path.file_name().is_some_and(|name| {
                matches!(name.to_str(), Some("warp"))
                    || name
                        .to_str()
                        .is_some_and(|value| value.starts_with("warp-"))
            }) && (under_home(".local") || under_home(".warp") || under_system_prefix)
        }
        "warp-agent-cli" => {
            path.file_name()
                .is_some_and(|name| name == "warp-tui-stable")
                && path
                    .parent()
                    .and_then(Path::file_name)
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with('v'))
                && under_home(".warp/tui/versions")
        }
        "rustup" => {
            path.file_name().is_some_and(|name| name == "rustup")
                && (under_home(".cargo/bin") || under_home(".rustup") || under_system_prefix)
        }
        "uv" => {
            path.file_name().is_some_and(|name| name == "uv")
                && (under_home(".local") || under_home(".cargo/bin") || under_system_prefix)
        }
        "deno" => {
            path.file_name().is_some_and(|name| name == "deno")
                && (under_home(".deno") || under_home(".local") || under_system_prefix)
        }
        "aiup" => path.file_name().is_some_and(|name| name == "aiup") && under_home(".local"),
        "cargo" => {
            path.file_name().is_some_and(|name| name == "cargo")
                && (under_home(".cargo/bin") || under_system_prefix)
        }
        "electron-squirrel" => {
            path.file_name().is_some_and(|name| name == "ShipIt")
                && path
                    .parent()
                    .and_then(Path::file_name)
                    .is_some_and(|name| name == "Resources")
                && path.ancestors().any(|ancestor| {
                    ancestor
                        .file_name()
                        .is_some_and(|name| name == "Squirrel.framework")
                })
                && path.ancestors().any(|ancestor| {
                    ancestor
                        .extension()
                        .is_some_and(|extension| extension == "app")
                })
        }
        _ => false,
    }
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
    pub executable_sha256: Option<String>,
    pub executable_size_bytes: Option<u64>,
    pub network_required: bool,
    pub requires_elevation: bool,
    pub rollback_supported: bool,
}

/// Bounded text evidence emitted by AIUP's `--no-install --dry-run` mode.
/// AIUP remains the provider-owned orchestrator; runtime.zero owns only this
/// parser and the later plan/confirmation boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiupDryRunReport {
    pub commands: BTreeMap<String, Vec<String>>,
    pub versions: BTreeMap<String, String>,
}

pub fn parse_aiup_dry_run(bytes: &[u8]) -> Result<AiupDryRunReport, String> {
    if bytes.len() as u64 > MAX_MANAGER_OUTPUT_BYTES {
        return Err("AIUP dry-run output exceeds the foundation ceiling".to_string());
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "AIUP dry-run output is not valid UTF-8".to_string())?;
    let mut commands = BTreeMap::<String, Vec<String>>::new();
    let mut versions = BTreeMap::<String, String>::new();
    let mut current_tool = None;
    let mut in_versions = false;
    let mut recognized_tool_section = false;
    let mut recognized_versions_section = false;
    for line in text.lines() {
        if let Some(value) = line.split_once("TOOL START: ").map(|(_, value)| value)
            && let Some(tool) = value.strip_suffix(" ==========")
            && valid_aiup_field(tool, 80)
        {
            current_tool = Some(tool.to_string());
            commands.entry(tool.to_string()).or_default();
            in_versions = false;
            recognized_tool_section = true;
            continue;
        }
        if line.contains("=== Detected tool versions ===") {
            current_tool = None;
            in_versions = true;
            recognized_versions_section = true;
            continue;
        }
        if let Some(command) = line.split_once("DRY-RUN: ").map(|(_, value)| value.trim())
            && let Some(tool) = current_tool.as_ref()
            && !command.is_empty()
            && command.len() <= 512
            && !command.chars().any(char::is_control)
        {
            commands
                .entry(tool.clone())
                .or_default()
                .push(command.to_string());
            continue;
        }
        if in_versions {
            let mut fields = line.split_whitespace();
            let Some(tool) = fields.next() else {
                continue;
            };
            let version = fields.collect::<Vec<_>>().join(" ");
            if version.is_empty() {
                continue;
            }
            if valid_aiup_field(tool, 80) && valid_aiup_version(&version) {
                versions.insert(tool.to_string(), version);
            }
        }
    }
    if !recognized_tool_section && !recognized_versions_section {
        return Err("AIUP dry-run output contained no recognized catalog sections".to_string());
    }
    Ok(AiupDryRunReport { commands, versions })
}

pub fn aiup_command_is_delegated(command: &str) -> bool {
    let command = command.trim_start();
    command == "brew update"
        || command.starts_with("brew ")
        || command.starts_with("npm ")
        || command.starts_with("npm-update ")
}

fn valid_aiup_field(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

fn valid_aiup_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 120
        && !value.chars().any(char::is_control)
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b' '))
}

pub fn parse_manager_output(
    context: &ManagerParseContext,
    output: &[u8],
) -> Result<Vec<UpdateRecord>, String> {
    if (output.is_empty() && !empty_manager_output_is_valid(context.manager))
        || output.len() as u64 > MAX_MANAGER_OUTPUT_BYTES
    {
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
        ManagerKind::MacAppStore => parse_mas(context, text),
        ManagerKind::AppleSoftwareUpdate => parse_softwareupdate(context, text),
        ManagerKind::NpmGlobal => parse_npm(context, text),
        ManagerKind::Pip => parse_pip(context, text),
        ManagerKind::RubyGems => parse_ruby_gems(context, text),
        ManagerKind::Grok => parse_grok(context, text),
        ManagerKind::Hermes => parse_hermes(context, text),
        ManagerKind::OhMyPi => parse_oh_my_pi(context, text),
        ManagerKind::Warp => Ok(Vec::new()),
        ManagerKind::Rustup => parse_rustup(context, text),
        ManagerKind::UvTools => parse_uv_tools(context, text),
        ManagerKind::Deno => parse_deno(context, text),
        ManagerKind::Aiup => Err(
            "AIUP output requires the provider-specific dry-run adapter; it is not a generic manager record stream".to_string(),
        ),
        ManagerKind::CargoInstall => Ok(Vec::new()),
        ManagerKind::Flatpak => parse_flatpak(context, text),
        ManagerKind::Snap => parse_snap(context, text),
        ManagerKind::Zypper => parse_zypper(context, text),
        ManagerKind::Winget => Err(format!(
            "{} output parser is not yet locale-safe; source remains unavailable",
            context.manager.id()
        )),
    }
}

/// Snap documents a stable five-column `refresh --list` table when the
/// updater process runs under the foundation's forced `C` locale. The parser
/// accepts only that exact header and one-token rows; any column drift remains
/// unavailable instead of becoming a guessed update candidate.
fn parse_snap(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    if text.trim() == "All snaps up to date." {
        return Ok(Vec::new());
    }
    let mut lines = text
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty());
    let header = lines
        .next()
        .ok_or_else(|| "snap refresh output is empty".to_string())?;
    if header.split_whitespace().collect::<Vec<_>>()
        != ["Name", "Version", "Rev", "Publisher", "Notes"]
    {
        return Err("snap refresh output has an unrecognized table header".to_string());
    }
    let mut records = Vec::new();
    for line in lines {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() != 5
            || !valid_snap_name(fields[0])
            || !valid_snap_version(fields[1])
            || !valid_snap_revision(fields[2])
            || !valid_snap_publisher(fields[3])
            || !valid_snap_notes(fields[4])
        {
            return Err("snap refresh output contains an unsafe or malformed row".to_string());
        }
        push_record(
            &mut records,
            make_record(context, fields[0], None, fields[1].to_string())?,
            context.manager,
        )?;
    }
    if records.is_empty() {
        return Err("snap refresh output contains no recognized update rows".to_string());
    }
    Ok(records)
}

fn valid_snap_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

fn valid_snap_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 120
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'.' | b'-' | b'_' | b'+' | b':' | b'/' | b'~')
        })
}

fn valid_snap_revision(value: &str) -> bool {
    !value.is_empty() && value.len() <= 24 && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn valid_snap_publisher(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_' | '+' | '✓')
        })
}

fn valid_snap_notes(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

/// Zypper's `--xmlout list-updates` stream is the machine-readable provider
/// contract. Only package updates in the exact `update-list` are actionable;
/// patches, malformed XML, missing identity attributes, and schema drift stay
/// unavailable instead of becoming guessed manager records.
fn parse_zypper(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    let mut reader = Reader::from_str(text);
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut saw_stream = false;
    let mut saw_update_list = false;
    let mut inside_update_list = false;
    let mut records = Vec::new();
    loop {
        match reader
            .read_event_into(&mut buffer)
            .map_err(|error| format!("parse zypper XML output: {error}"))?
        {
            Event::Start(element) => match element.name().as_ref() {
                b"stream" if !saw_stream => saw_stream = true,
                b"update-list" if saw_stream && !inside_update_list => {
                    saw_update_list = true;
                    inside_update_list = true;
                }
                b"update" if inside_update_list => {
                    let element = element.into_owned();
                    let record = parse_zypper_update(context, &element)?;
                    push_record(&mut records, record, context.manager)?;
                    reader
                        .read_to_end_into(element.name(), &mut buffer)
                        .map_err(|error| format!("read zypper update element: {error}"))?;
                }
                _ => {}
            },
            Event::Empty(element) => match element.name().as_ref() {
                b"stream" if !saw_stream => saw_stream = true,
                b"update-list" if saw_stream && !inside_update_list => {
                    saw_update_list = true;
                }
                b"update" if inside_update_list => {
                    let record = parse_zypper_update(context, &element)?;
                    push_record(&mut records, record, context.manager)?;
                }
                _ => {}
            },
            Event::End(element) if element.name().as_ref() == b"update-list" => {
                inside_update_list = false;
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    if !saw_stream || !saw_update_list || inside_update_list {
        return Err("zypper XML output has an invalid stream/update-list envelope".to_string());
    }
    Ok(records)
}

fn parse_zypper_update(
    context: &ManagerParseContext,
    element: &quick_xml::events::BytesStart<'_>,
) -> Result<UpdateRecord, String> {
    let mut name = None;
    let mut edition = None;
    let mut kind = None;
    for attribute in element.attributes() {
        let attribute =
            attribute.map_err(|error| format!("parse zypper update attribute: {error}"))?;
        let key = attribute.key.as_ref();
        let value = std::str::from_utf8(&attribute.value)
            .map_err(|error| format!("decode zypper update attribute: {error}"))?
            .to_string();
        match key {
            b"name" => name = Some(value),
            b"edition" => edition = Some(value),
            b"kind" => kind = Some(value),
            b"arch" | b"repo" | b"repository" | b"status" => {}
            _ => return Err("zypper update contains an unrecognized attribute".to_string()),
        }
    }
    if kind.as_deref() != Some("package") {
        return Err("zypper update is not an exact package update row".to_string());
    }
    let name = name.ok_or_else(|| "zypper update has no package name".to_string())?;
    let edition = edition.ok_or_else(|| "zypper update has no package edition".to_string())?;
    if !valid_zypper_field(&name, 240) || !valid_zypper_field(&edition, 160) {
        return Err("zypper update has an unsafe package identity".to_string());
    }
    make_record(context, &name, None, edition)
}

fn valid_zypper_field(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'.' | b'-' | b'_' | b'+' | b':' | b'/' | b'~' | b'@')
        })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct FlatpakUpdateRow {
    application_id: String,
    version: String,
    branch: String,
    arch: String,
    origin: String,
    commit: String,
}

fn parse_flatpak(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    let rows: Vec<FlatpakUpdateRow> = serde_json::from_str(text)
        .map_err(|error| format!("parse flatpak remote-ls JSON: {error}"))?;
    if rows.len() > MAX_UPDATE_RECORDS {
        return Err("flatpak output exceeds the update-record ceiling".to_string());
    }
    rows.into_iter()
        .map(|row| {
            if !valid_flatpak_token(&row.application_id, 240)
                || !valid_flatpak_token(&row.branch, 120)
                || !valid_flatpak_token(&row.arch, 80)
                || !valid_flatpak_token(&row.origin, 160)
                || !valid_flatpak_commit(&row.commit)
            {
                return Err("flatpak update row contains unsafe identity evidence".to_string());
            }
            if row.version.len() > 120 || row.version.chars().any(char::is_control) {
                return Err(
                    "flatpak update row contains an oversized or unsafe version".to_string()
                );
            }
            let reference = format!("app/{}/{}/{}", row.application_id, row.arch, row.branch);
            // The remote commit is the immutable update identity. The displayed
            // AppStream version is retained only as bounded input evidence.
            make_record(
                context,
                &reference,
                None,
                format!("commit:{}", row.commit.to_ascii_lowercase()),
            )
        })
        .collect()
}

fn valid_flatpak_token(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

fn valid_flatpak_commit(value: &str) -> bool {
    value.len() == 12 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn empty_manager_output_is_valid(manager: ManagerKind) -> bool {
    matches!(
        manager,
        ManagerKind::Apt
            | ManagerKind::Dnf
            | ManagerKind::Pacman
            | ManagerKind::MacPorts
            | ManagerKind::RubyGems
            | ManagerKind::Rustup
            | ManagerKind::Deno
            | ManagerKind::Aiup
            | ManagerKind::CargoInstall
    )
}

fn parse_npm(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|error| format!("parse npm outdated JSON: {error}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "npm outdated output must be a JSON object".to_string())?;
    object
        .iter()
        .map(|(name, record)| {
            let installed = required_string(record, "current")?;
            let available = required_string(record, "latest")?;
            make_record(context, name, Some(installed), available)
        })
        .collect()
}

fn parse_pip(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    let values: Vec<serde_json::Value> =
        serde_json::from_str(text).map_err(|error| format!("parse pip outdated JSON: {error}"))?;
    if values.len() > MAX_UPDATE_RECORDS {
        return Err("pip output exceeds the update-record ceiling".to_string());
    }
    values
        .iter()
        .map(|record| {
            let name = required_string(record, "name")?;
            let installed = required_string(record, "version")?;
            let available = required_string(record, "latest_version")?;
            make_record(context, &name, Some(installed), available)
        })
        .collect()
}

fn parse_ruby_gems(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    let mut records = Vec::new();
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let Some((name, versions)) = line.split_once(" (") else {
            continue;
        };
        let versions = versions.strip_suffix(')').unwrap_or(versions);
        let Some((installed, available)) = versions.split_once(" < ") else {
            continue;
        };
        push_record(
            &mut records,
            make_record(
                context,
                name,
                Some(installed.to_string()),
                available.to_string(),
            )?,
            context.manager,
        )?;
    }
    Ok(records)
}

fn parse_grok(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|error| format!("parse grok update JSON: {error}"))?;
    if value.get("error").is_some_and(|error| !error.is_null()) {
        return Err("grok update check reported an error".to_string());
    }
    if !value
        .get("updateAvailable")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(Vec::new());
    }
    let installed = required_string(&value, "currentVersion")?;
    let available = required_string(&value, "latestVersion")?;
    Ok(vec![make_record(
        context,
        "grok",
        Some(installed),
        available,
    )?])
}

fn parse_hermes(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    parse_version_pair_text(context, text, "hermes")
}

fn parse_oh_my_pi(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    parse_version_pair_text(context, text, "omp")
}

fn parse_rustup(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    let mut records = Vec::new();
    let mut recognized = false;
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let Some((toolchain, detail)) = line.split_once(" - Update available : ") else {
            if line.contains(" - Up to date : ") {
                recognized = true;
                continue;
            }
            return Err("rustup check output has an unrecognized status line".to_string());
        };
        recognized = true;
        let Some((installed, available)) = detail.split_once(" -> ") else {
            return Err("rustup check output has an invalid update range".to_string());
        };
        push_record(
            &mut records,
            make_record(
                context,
                toolchain.trim(),
                Some(installed.trim().to_string()),
                available.trim().to_string(),
            )?,
            context.manager,
        )?;
    }
    if !text.trim().is_empty() && !recognized {
        return Err("rustup check output contains no recognized toolchain status".to_string());
    }
    Ok(records)
}

fn parse_uv_tools(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    let mut records = Vec::new();
    let mut recognized = false;
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if line.eq_ignore_ascii_case("no tools installed") {
            return Ok(records);
        }
        let Some((name, versions)) = line.split_once(" (latest: ") else {
            return Err("uv tool list output has an unrecognized status line".to_string());
        };
        recognized = true;
        let Some(available) = versions.strip_suffix(')') else {
            return Err("uv tool list output has an incomplete latest-version field".to_string());
        };
        let mut fields = name.split_whitespace();
        let Some(package) = fields.next() else {
            continue;
        };
        let Some(installed) = fields.next() else {
            return Err("uv tool list output has no installed version".to_string());
        };
        if installed == available {
            continue;
        }
        push_record(
            &mut records,
            make_record(
                context,
                package,
                Some(installed.trim_start_matches('v').to_string()),
                available.trim().trim_start_matches('v').to_string(),
            )?,
            context.manager,
        )?;
    }
    if !text.trim().is_empty() && !recognized {
        return Err("uv tool list output contains no recognized tool status".to_string());
    }
    Ok(records)
}

fn parse_deno(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let lower = text.to_ascii_lowercase();
    if lower.contains("up to date") || lower.contains("already up to date") {
        return Ok(Vec::new());
    }
    for line in text.lines().map(str::trim) {
        if let Some((installed, available)) = line.split_once(" -> ") {
            let installed = installed.trim().trim_start_matches('v');
            let available = available.trim().trim_start_matches('v');
            if !installed.is_empty() && !available.is_empty() {
                return Ok(vec![make_record(
                    context,
                    "deno",
                    Some(installed.to_string()),
                    available.to_string(),
                )?]);
            }
        }
    }
    Err("deno upgrade dry-run output has no recognized version result".to_string())
}

fn parse_version_pair_text(
    context: &ManagerParseContext,
    text: &str,
    name: &str,
) -> Result<Vec<UpdateRecord>, String> {
    let current = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("Current version: "))
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let available = text
        .lines()
        .find_map(|line| line.trim().strip_prefix("New version available: "))
        .map(str::trim)
        .filter(|value| !value.is_empty());
    match (current, available) {
        (Some(current), Some(available)) => Ok(vec![make_record(
            context,
            name,
            Some(current.to_string()),
            available.to_string(),
        )?]),
        _ if text.to_ascii_lowercase().contains("up to date")
            || text.to_ascii_lowercase().contains("already up to date") =>
        {
            Ok(Vec::new())
        }
        _ => Err(format!(
            "{} update check output has no recognized version pair",
            context.manager.id()
        )),
    }
}

fn parse_mas(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    let mut records = Vec::new();
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let value: serde_json::Value = serde_json::from_str(line)
            .map_err(|error| format!("parse mac-app-store JSON line: {error}"))?;
        let adam_id = value
            .get("adamID")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| "mac-app-store record is missing numeric 'adamID'".to_string())?;
        let _name = required_string(&value, "name")?;
        let installed = required_string(&value, "version")?;
        let available = required_string(&value, "newVersion")?;
        push_record(
            &mut records,
            make_record(context, &adam_id.to_string(), Some(installed), available)?,
            context.manager,
        )?;
    }
    if records.is_empty() && !text.lines().any(|line| line.trim() == "No outdated apps.") {
        return Err("mac-app-store output contains no recognized JSON app records".to_string());
    }
    Ok(records)
}

fn parse_softwareupdate(
    context: &ManagerParseContext,
    text: &str,
) -> Result<Vec<UpdateRecord>, String> {
    if text
        .lines()
        .any(|line| line.trim() == "No new software available.")
    {
        return Ok(Vec::new());
    }
    let mut records = Vec::new();
    let mut label: Option<String> = None;
    for line in text.lines().map(str::trim).filter(|line| !line.is_empty()) {
        if let Some(value) = line
            .strip_prefix("* Label: ")
            .or_else(|| line.strip_prefix("- Label: "))
        {
            if label.is_some() {
                return Err(
                    "softwareupdate output contains a label without a recognized detail line"
                        .to_string(),
                );
            }
            if value.is_empty() {
                return Err("softwareupdate output contains an empty update label".to_string());
            }
            label = Some(value.to_string());
            continue;
        }
        let Some(details) = line.strip_prefix("Title: ") else {
            continue;
        };
        let Some((_, version_tail)) = details.split_once(", Version: ") else {
            return Err("softwareupdate detail line has no exact Version field".to_string());
        };
        let available = version_tail
            .split_once(", ")
            .map_or(version_tail, |(version, _)| version)
            .trim_end_matches(',')
            .trim();
        let label = label
            .take()
            .ok_or_else(|| "softwareupdate detail line preceded its Label line".to_string())?;
        if available.is_empty() {
            return Err("softwareupdate detail line has an empty Version field".to_string());
        }
        push_record(
            &mut records,
            make_record(context, &label, None, available.to_string())?,
            context.manager,
        )?;
    }
    if label.is_some() {
        return Err("softwareupdate output ended with an incomplete update record".to_string());
    }
    if records.is_empty() {
        return Err("softwareupdate output contains no recognized update records".to_string());
    }
    Ok(records)
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
    if records.len() > MAX_UPDATE_RECORDS {
        return Err(format!(
            "{} output exceeds the update-record ceiling",
            context.manager.id()
        ));
    }
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
        let mut fields = line.split_whitespace();
        let Some(first) = fields.next() else {
            continue;
        };
        let Some((name, _channel)) = first.split_once('/') else {
            continue;
        };
        let Some(available) = fields.next() else {
            continue;
        };
        let installed = line
            .split_once("upgradable from: ")
            .and_then(|(_, value)| value.split(']').next())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        push_record(
            &mut records,
            make_record(context, name, installed, available.to_string())?,
            context.manager,
        )?;
    }
    Ok(records)
}

fn parse_dnf(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    let mut records = Vec::new();
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let Some(name) = fields.next() else {
            continue;
        };
        let Some(version) = fields.next() else {
            continue;
        };
        let Some(_repository) = fields.next() else {
            continue;
        };
        if name.eq_ignore_ascii_case("package")
            || name.starts_with("Last")
            || name.starts_with("Obsoleting")
        {
            continue;
        }
        push_record(
            &mut records,
            make_record(context, name, None, version.to_string())?,
            context.manager,
        )?;
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
        push_record(
            &mut records,
            make_record(context, name, installed, available.trim().to_string())?,
            context.manager,
        )?;
    }
    Ok(records)
}

fn parse_macports(context: &ManagerParseContext, text: &str) -> Result<Vec<UpdateRecord>, String> {
    let mut records = Vec::new();
    for line in text.lines() {
        let mut fields = line.split_whitespace();
        let Some(name) = fields.next() else {
            continue;
        };
        let mut versions = fields.filter_map(|field| field.strip_prefix('@'));
        let (Some(installed), Some(available)) = (versions.next(), versions.next()) else {
            continue;
        };
        push_record(
            &mut records,
            make_record(
                context,
                name,
                Some(installed.to_string()),
                available.to_string(),
            )?,
            context.manager,
        )?;
    }
    Ok(records)
}

fn push_record(
    records: &mut Vec<UpdateRecord>,
    record: UpdateRecord,
    manager: ManagerKind,
) -> Result<(), String> {
    if records.len() == MAX_UPDATE_RECORDS {
        return Err(format!(
            "{} output exceeds the update-record ceiling",
            manager.id()
        ));
    }
    records.push(record);
    Ok(())
}

fn make_record(
    context: &ManagerParseContext,
    name: &str,
    installed_version: Option<String>,
    available_version: String,
) -> Result<UpdateRecord, String> {
    if name.is_empty() || name.len() > MAX_PACKAGE_NAME_BYTES || name.chars().any(char::is_control)
    {
        return Err("manager package name is empty, oversized, or unsafe".to_string());
    }
    let slug = slug(name)?;
    let reference = reference_slug(name)?;
    for version in installed_version
        .iter()
        .chain(std::iter::once(&available_version))
    {
        if version.is_empty() || version.len() > 120 || version.chars().any(char::is_control) {
            return Err(format!("manager record '{name}' has an invalid version"));
        }
    }
    let arguments = context.manager.upgrade_arguments(name);
    Ok(UpdateRecord {
        finding_id: format!("update.{}.{}", context.manager.id(), slug),
        subject_reference: format!("package:{}:{}", context.manager.id(), reference),
        installed: true,
        manager_record_present: true,
        update_available: true,
        installed_version,
        available_version: Some(available_version),
        manager: Some(context.manager.manager_name().to_string()),
        executable: context.executable.clone(),
        executable_sha256: context.executable_sha256.clone(),
        executable_size_bytes: context.executable_size_bytes,
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

fn reference_slug(value: &str) -> Result<String, String> {
    let mut result = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            result.push(character.to_ascii_lowercase());
        } else if character == '/' {
            if !result.ends_with(':') {
                result.push(':');
            }
        } else if matches!(character, '-' | '_' | '.' | ':') && !result.ends_with(character) {
            result.push(character);
        }
        if result.len() >= 160 {
            break;
        }
    }
    let result = result.trim_matches([':', '.', '-', '_']).to_string();
    if result.is_empty() {
        Err("manager package name has no safe reference characters".to_string())
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
            executable_sha256: Some(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            ),
            executable_size_bytes: Some(4096),
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
        let macos_specs = manager_probe_specs_for_platform("macos");
        assert_eq!(
            macos_specs
                .iter()
                .find(|spec| spec.manager == ManagerKind::MacAppStore)
                .map(|spec| spec.requires_elevation),
            Some(false)
        );
        assert_eq!(
            macos_specs
                .iter()
                .find(|spec| spec.manager == ManagerKind::MacPorts)
                .map(|spec| spec.requires_elevation),
            Some(true)
        );
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
        #[cfg(unix)]
        assert!(manager_executable_allowed(
            "npm",
            std::env::consts::OS,
            if cfg!(target_os = "macos") {
                "/opt/homebrew/opt/node/bin/npm"
            } else {
                "/usr/local/bin/npm"
            }
        ));
        #[cfg(target_os = "macos")]
        assert!(manager_executable_allowed(
            "npm",
            "macos",
            "/opt/homebrew/Cellar/node/26.5.0/libexec/lib/node_modules/npm/bin/npm-cli.js"
        ));
        #[cfg(target_os = "macos")]
        assert!(manager_executable_allowed(
            "python",
            "macos",
            "/Library/Frameworks/Python.framework/Versions/3.12/bin/python3"
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

        let cask = parse_manager_output(
            &context(ManagerKind::HomebrewCask),
            br#"{"casks":[{"name":"alpha","installed_versions":["1.0"],"current_version":"1.1"}]}"#,
        )
        .expect("Homebrew cask records");
        assert_eq!(
            cask[0].arguments,
            ["upgrade", "--cask", "--greedy", "alpha"]
        );
    }

    #[test]
    fn parses_mac_app_store_json_lines_and_softwareupdate_labels() {
        let mas = parse_manager_output(
            &context(ManagerKind::MacAppStore),
            br#"{"adamID":409183694,"name":"Keynote","newVersion":"12.0","version":"11.0"}
{"adamID":123,"name":"Pages","newVersion":"14.0","version":"13.0"}
"#,
        )
        .expect("Mac App Store records");
        assert_eq!(mas.len(), 2);
        assert_eq!(mas[0].finding_id, "update.mac-app-store.409183694");
        assert_eq!(mas[0].arguments, ["upgrade", "409183694"]);

        let softwareupdate = parse_manager_output(
            &context(ManagerKind::AppleSoftwareUpdate),
            b"Software Update Tool\n* Label: macOS-15.6-24G90\n    Title: macOS Sequoia, Version: 15.6, Size: 1K, Recommended: YES, Action: restart,\n",
        )
        .expect("Apple software update records");
        assert_eq!(softwareupdate.len(), 1);
        assert_eq!(
            softwareupdate[0].arguments,
            ["--install", "macOS-15.6-24G90"]
        );
        assert!(
            parse_manager_output(
                &context(ManagerKind::AppleSoftwareUpdate),
                b"No new software available.\n"
            )
            .expect("no Apple updates")
            .is_empty()
        );
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
    fn dnf_parser_uses_the_version_field_not_the_repository_field() {
        let records =
            parse_manager_output(&context(ManagerKind::Dnf), b"alpha.x86_64 2.0-1 updates\n")
                .expect("dnf records");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].available_version.as_deref(), Some("2.0-1"));
    }

    #[test]
    fn parses_language_and_self_updating_provider_outputs() {
        let npm = parse_manager_output(
            &context(ManagerKind::NpmGlobal),
            br#"{"@openai/codex":{"current":"0.146.0","latest":"0.147.0"}}"#,
        )
        .expect("npm records");
        assert_eq!(npm[0].subject_reference, "package:npm-global:openai:codex");
        assert_eq!(npm[0].arguments, ["update", "--global", "@openai/codex"]);

        let pip = parse_manager_output(
            &context(ManagerKind::Pip),
            br#"[{"name":"ruff","version":"0.8.0","latest_version":"0.9.0"}]"#,
        )
        .expect("pip records");
        assert_eq!(pip[0].subject_reference, "package:pip:ruff");

        let gems = parse_manager_output(
            &context(ManagerKind::RubyGems),
            b"bundler (2.5.0 < 2.6.0)\nnokogiri (1.16.0 < 1.17.0)\n",
        )
        .expect("gem records");
        assert_eq!(gems.len(), 2);

        let grok = parse_manager_output(
            &context(ManagerKind::Grok),
            br#"{"currentVersion":"1.0.4","latestVersion":"1.0.5","updateAvailable":true}"#,
        )
        .expect("grok records");
        assert_eq!(grok[0].arguments, ["update", "--stable"]);

        let hermes = parse_manager_output(
            &context(ManagerKind::Hermes),
            b"Current version: 0.1.0\nNew version available: 0.2.0\n",
        )
        .expect("Hermes records");
        assert_eq!(hermes.len(), 1);

        let omp = parse_manager_output(
            &context(ManagerKind::OhMyPi),
            b"Current version: 17.2.15\nNew version available: 17.3.5\n",
        )
        .expect("OMP records");
        assert_eq!(omp[0].arguments, ["update"]);

        let rustup = parse_manager_output(
            &context(ManagerKind::Rustup),
            b"stable-aarch64-apple-darwin - Update available : 1.96.0 -> 1.97.0\n",
        )
        .expect("rustup records");
        assert_eq!(
            rustup[0].arguments,
            ["update", "stable-aarch64-apple-darwin"]
        );

        let uv = parse_manager_output(
            &context(ManagerKind::UvTools),
            b"ruff v0.8.0 (latest: v0.9.0)\n",
        )
        .expect("uv records");
        assert_eq!(uv[0].arguments, ["tool", "upgrade", "ruff"]);

        let deno = parse_manager_output(&context(ManagerKind::Deno), b"v2.9.3 -> v2.9.4\n")
            .expect("deno records");
        assert_eq!(deno[0].arguments, ["upgrade"]);

        assert!(
            parse_manager_output(&context(ManagerKind::RubyGems), b"")
                .expect("empty gem output means no outdated gems")
                .is_empty()
        );

        assert!(
            parse_manager_output(&context(ManagerKind::Warp), b"warp-tui 1.0.0\n")
                .expect("Warp observation")
                .is_empty()
        );
    }

    #[test]
    fn parses_aiup_dry_run_as_bounded_provider_evidence() {
        let report = parse_aiup_dry_run(
            b"[INFO] ========== TOOL START: antigravity ==========\n\
[INFO] DRY-RUN: curl -fsSL https://example.invalid/install.sh | bash\n\
=== Detected tool versions ===\n\
antigravity 1.1.12\n",
        )
        .expect("AIUP dry-run report");
        assert_eq!(
            report.versions.get("antigravity").map(String::as_str),
            Some("1.1.12")
        );
        assert_eq!(report.commands["antigravity"].len(), 1);
        assert!(!aiup_command_is_delegated(
            &report.commands["antigravity"][0]
        ));
        assert!(aiup_command_is_delegated("npm install -g @openai/codex"));
        assert!(parse_manager_output(&context(ManagerKind::Aiup), b"").is_err());
    }

    #[test]
    fn parses_flatpak_json_as_commit_bound_ref_evidence() {
        let records = parse_manager_output(
            &context(ManagerKind::Flatpak),
            br#"[{"application_id":"org.example.App","version":"1.2.3","branch":"stable","arch":"x86_64","origin":"flathub","commit":"0123456789ab"}]"#,
        )
        .expect("flatpak JSON");
        assert_eq!(records.len(), 1);
        assert_eq!(
            records[0].subject_reference,
            "package:flatpak:app:org.example.app:x86_64:stable"
        );
        assert_eq!(
            records[0].available_version.as_deref(),
            Some("commit:0123456789ab")
        );
        assert_eq!(
            records[0].arguments,
            vec![
                "update".to_string(),
                "app/org.example.App/x86_64/stable".to_string()
            ]
        );
        assert!(records[0].network_required);
    }

    #[test]
    fn flatpak_json_parser_rejects_unknown_fields_and_bad_commit_identity() {
        assert!(parse_manager_output(
            &context(ManagerKind::Flatpak),
            br#"[{"application_id":"org.example.App","version":"1.2.3","branch":"stable","arch":"x86_64","origin":"flathub","commit":"0123456789ab","extra":"reject"}]"#,
        )
        .is_err());
        assert!(parse_manager_output(
            &context(ManagerKind::Flatpak),
            br#"[{"application_id":"org.example.App","version":"1.2.3","branch":"stable","arch":"x86_64","origin":"flathub","commit":"not-a-commit"}]"#,
        )
        .is_err());
    }

    #[test]
    fn parses_snap_refresh_list_table_with_exact_columns() {
        let records = parse_manager_output(
            &context(ManagerKind::Snap),
            "Name           Version                    Rev   Publisher     Notes\ncore           16-2.45.1+git2022.b6b3c25  9584  canonical✓    core\n".as_bytes(),
        )
        .expect("snap refresh table");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].subject_reference, "package:snap:core");
        assert_eq!(
            records[0].available_version.as_deref(),
            Some("16-2.45.1+git2022.b6b3c25")
        );
        assert_eq!(
            records[0].arguments,
            vec!["refresh".to_string(), "core".to_string()]
        );
    }

    #[test]
    fn snap_refresh_table_rejects_header_and_row_drift() {
        assert!(
            parse_manager_output(
                &context(ManagerKind::Snap),
                "Name Version Rev Publisher Notes\ncore 1.0 42 canonical✓ - extra\n".as_bytes(),
            )
            .is_err()
        );
        assert!(
            parse_manager_output(
                &context(ManagerKind::Snap),
                "Name Version Rev Publisher Notes\ncore 1.0 42 publisher with-space -\n".as_bytes(),
            )
            .is_err()
        );
        assert!(
            parse_manager_output(
                &context(ManagerKind::Snap),
                "All snaps up to date.".as_bytes()
            )
            .is_ok()
        );
    }

    #[test]
    fn parses_zypper_xml_package_updates_with_exact_identity() {
        let records = parse_manager_output(
            &context(ManagerKind::Zypper),
            r#"<?xml version='1.0'?><stream><message type='info'>Loading repository data...</message><update-list><update name='zypper' edition='1.14.42-1.2' arch='x86_64' kind='package' repo='repo-update'/></update-list></stream>"#.as_bytes(),
        )
        .expect("zypper XML update stream");
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].subject_reference, "package:zypper:zypper");
        assert_eq!(records[0].available_version.as_deref(), Some("1.14.42-1.2"));
        assert_eq!(records[0].arguments, ["update", "zypper"]);
    }

    #[test]
    fn zypper_xml_parser_rejects_non_package_or_schema_drift() {
        assert!(parse_manager_output(
            &context(ManagerKind::Zypper),
            r#"<stream><update-list><update name='test' edition='1.0' kind='patch'/></update-list></stream>"#.as_bytes(),
        )
        .is_err());
        assert!(parse_manager_output(
            &context(ManagerKind::Zypper),
            r#"<stream><update-list><update name='test' edition='1.0' kind='package' unexpected='x'/></update-list></stream>"#.as_bytes(),
        )
        .is_err());
        assert!(
            parse_manager_output(&context(ManagerKind::Zypper), b"localized table output",)
                .is_err()
        );
    }

    #[test]
    fn aiup_dry_run_rejects_invalid_encoding_and_oversized_output() {
        assert!(parse_aiup_dry_run(&[0xff]).is_err());
        assert!(parse_aiup_dry_run(&vec![b'x'; MAX_MANAGER_OUTPUT_BYTES as usize + 1]).is_err());
    }

    #[test]
    fn aiup_dry_run_rejects_unrecognized_or_malformed_catalog_output() {
        assert!(parse_aiup_dry_run(b"aiup completed successfully\n").is_err());
        assert!(parse_aiup_dry_run(b"TOOL START: invalid tool ==========").is_err());
        assert!(parse_aiup_dry_run(b"=== Detected tool versions ===\n").is_ok());
    }

    #[test]
    fn deterministic_untrusted_byte_corpus_never_panics_or_exceeds_bounds() {
        let managers = [
            ManagerKind::HomebrewFormula,
            ManagerKind::HomebrewCask,
            ManagerKind::MacPorts,
            ManagerKind::MacAppStore,
            ManagerKind::AppleSoftwareUpdate,
            ManagerKind::Winget,
            ManagerKind::Apt,
            ManagerKind::Dnf,
            ManagerKind::Pacman,
            ManagerKind::Zypper,
            ManagerKind::Snap,
            ManagerKind::Flatpak,
            ManagerKind::NpmGlobal,
            ManagerKind::Pip,
            ManagerKind::RubyGems,
            ManagerKind::Grok,
            ManagerKind::Hermes,
            ManagerKind::OhMyPi,
            ManagerKind::Warp,
            ManagerKind::Rustup,
            ManagerKind::UvTools,
            ManagerKind::Deno,
            ManagerKind::Aiup,
            ManagerKind::CargoInstall,
        ];
        let mut state = 0x5eed_cafe_d00d_beefu64;
        for manager in managers {
            for length in 0..256usize {
                let mut bytes = Vec::with_capacity(length);
                for _ in 0..length {
                    state = state
                        .wrapping_mul(6_364_136_223_846_793_005)
                        .wrapping_add(1);
                    bytes.push((state >> 32) as u8);
                }
                if let Ok(records) = parse_manager_output(&context(manager), &bytes) {
                    assert!(records.len() <= rz0_resource_contract::MAX_FINDINGS);
                    assert!(records.iter().all(|record| {
                        !record.finding_id.chars().any(char::is_control)
                            && !record.subject_reference.chars().any(char::is_control)
                            && record.arguments.len() <= 4
                    }));
                }
            }
        }
    }

    #[test]
    fn record_and_package_identity_ceilings_fail_closed() {
        let mut output = String::new();
        for index in 0..=MAX_UPDATE_RECORDS {
            output.push_str(&format!("package-{index} 1.0 -> 2.0\n"));
        }
        assert!(parse_manager_output(&context(ManagerKind::Pacman), output.as_bytes()).is_err());
        let oversized = format!("{} 1.0 -> 2.0\n", "x".repeat(MAX_PACKAGE_NAME_BYTES + 1));
        assert!(parse_manager_output(&context(ManagerKind::Pacman), oversized.as_bytes()).is_err());
    }

    #[test]
    fn locale_unsafe_adapters_fail_closed() {
        let result = parse_manager_output(&context(ManagerKind::Winget), b"localized output");
        assert!(result.is_err());
    }
}
