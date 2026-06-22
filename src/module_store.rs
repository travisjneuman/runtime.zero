use serde::Serialize;
use std::env;
use std::path::{Path, PathBuf};

pub const STORE_SCHEMA_VERSION: u16 = 1;
pub const REGISTRY_FILE: &str = "installed-modules.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModuleStorePlan {
    pub store_schema_version: u16,
    pub plan_id: String,
    pub data_root: String,
    pub state_root: String,
    pub cache_root: String,
    pub log_root: String,
    pub quarantine_root: String,
    pub modules_root: String,
    pub registry_path: String,
    pub transaction_path: String,
    pub module_dir: Option<String>,
    pub receipt_path: Option<String>,
    pub rollback_plan_path: Option<String>,
    pub quarantine_record_path: Option<String>,
    pub rollback_supported: bool,
    pub quarantine_supported: bool,
    pub forbidden_path_classes: Vec<ForbiddenPathClass>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ForbiddenPathClass {
    Credentials,
    BrowserProfiles,
    OauthSessions,
    UnknownUserData,
    Backups,
    ProjectWorkspaces,
}

pub fn module_store_plan(
    module_id: Option<&str>,
    version: Option<&str>,
    seed: &str,
) -> ModuleStorePlan {
    module_store_plan_from_roots(StoreRoots::current(), module_id, version, seed)
}

pub fn module_store_plan_for_data_root(
    data_root: PathBuf,
    module_id: Option<&str>,
    version: Option<&str>,
    seed: &str,
) -> ModuleStorePlan {
    module_store_plan_from_roots(
        StoreRoots::from_data_root(data_root),
        module_id,
        version,
        seed,
    )
}

fn module_store_plan_from_roots(
    roots: StoreRoots,
    module_id: Option<&str>,
    version: Option<&str>,
    seed: &str,
) -> ModuleStorePlan {
    let modules_root = roots.data_root.join("modules");
    let plan_id = stable_plan_id(seed);
    let transaction_path = roots
        .state_root
        .join("transactions")
        .join(format!("{plan_id}.json"));
    let module_dir = module_id.zip(version).map(|(id, version)| {
        modules_root
            .join(id)
            .join(safe_path_segment(version))
            .display()
            .to_string()
    });

    ModuleStorePlan {
        store_schema_version: STORE_SCHEMA_VERSION,
        plan_id: plan_id.clone(),
        data_root: display(&roots.data_root),
        state_root: display(&roots.state_root),
        cache_root: display(&roots.cache_root),
        log_root: display(&roots.log_root),
        quarantine_root: display(&roots.quarantine_root),
        modules_root: display(&modules_root),
        registry_path: display(&roots.state_root.join(REGISTRY_FILE)),
        transaction_path: display(&transaction_path),
        receipt_path: module_dir.as_ref().map(|_| {
            display(
                &roots
                    .state_root
                    .join("receipts")
                    .join(format!("{plan_id}.json")),
            )
        }),
        rollback_plan_path: module_dir.as_ref().map(|_| {
            display(
                &roots
                    .state_root
                    .join("receipts")
                    .join(format!("{plan_id}.rollback.json")),
            )
        }),
        quarantine_record_path: module_dir.as_ref().map(|_| {
            display(
                &roots
                    .quarantine_root
                    .join("modules")
                    .join(&plan_id)
                    .join("quarantine.json"),
            )
        }),
        module_dir,
        rollback_supported: true,
        quarantine_supported: true,
        forbidden_path_classes: forbidden_path_classes(),
    }
}

fn forbidden_path_classes() -> Vec<ForbiddenPathClass> {
    vec![
        ForbiddenPathClass::Credentials,
        ForbiddenPathClass::BrowserProfiles,
        ForbiddenPathClass::OauthSessions,
        ForbiddenPathClass::UnknownUserData,
        ForbiddenPathClass::Backups,
        ForbiddenPathClass::ProjectWorkspaces,
    ]
}

fn stable_plan_id(seed: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in seed.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("rz0plan_{hash:016x}")
}

fn safe_path_segment(value: &str) -> String {
    value
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '-' | '_' => c,
            _ => '_',
        })
        .collect()
}

fn display(path: &Path) -> String {
    path.display().to_string()
}

struct StoreRoots {
    data_root: PathBuf,
    state_root: PathBuf,
    cache_root: PathBuf,
    log_root: PathBuf,
    quarantine_root: PathBuf,
}

impl StoreRoots {
    fn current() -> Self {
        match env::consts::OS {
            "windows" => windows_roots(),
            "macos" => macos_roots(),
            _ => linux_roots(),
        }
    }

    fn from_data_root(data_root: PathBuf) -> Self {
        StoreRoots {
            state_root: data_root.join("state"),
            cache_root: data_root.join("cache"),
            log_root: data_root.join("logs"),
            quarantine_root: data_root.join("quarantine"),
            data_root,
        }
    }
}

fn windows_roots() -> StoreRoots {
    let data_root = env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("%LOCALAPPDATA%"))
        .join("runtime.zero");
    StoreRoots {
        state_root: data_root.join("state"),
        cache_root: data_root.join("cache"),
        log_root: data_root.join("logs"),
        quarantine_root: data_root.join("quarantine"),
        data_root,
    }
}

fn macos_roots() -> StoreRoots {
    let home = home_dir().unwrap_or_else(|| PathBuf::from("~"));
    let data_root = home
        .join("Library")
        .join("Application Support")
        .join("runtime.zero");
    StoreRoots {
        state_root: data_root.join("state"),
        cache_root: home.join("Library").join("Caches").join("runtime.zero"),
        log_root: home.join("Library").join("Logs").join("runtime.zero"),
        quarantine_root: data_root.join("quarantine"),
        data_root,
    }
}

fn linux_roots() -> StoreRoots {
    let home = home_dir().unwrap_or_else(|| PathBuf::from("~"));
    let data_root = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local").join("share"))
        .join("runtime.zero");
    let state_root = env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".local").join("state"))
        .join("runtime.zero");
    StoreRoots {
        cache_root: env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".cache"))
            .join("runtime.zero"),
        log_root: state_root.join("logs"),
        quarantine_root: state_root.join("quarantine"),
        state_root,
        data_root,
    }
}

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_store_plan_without_creating_paths() {
        let plan = module_store_plan(Some("first-party.inventory"), Some("0.1.0"), "seed");
        assert_eq!(plan.store_schema_version, STORE_SCHEMA_VERSION);
        assert!(plan.registry_path.contains(REGISTRY_FILE));
        assert!(plan.module_dir.is_some());
        assert!(plan.receipt_path.is_some());
        assert!(
            plan.forbidden_path_classes
                .contains(&ForbiddenPathClass::Credentials)
        );
    }

    #[test]
    fn plan_id_is_stable_for_same_seed() {
        let first = module_store_plan(None, None, "same-seed");
        let second = module_store_plan(None, None, "same-seed");
        assert_eq!(first.plan_id, second.plan_id);
    }

    #[test]
    fn computes_store_plan_for_explicit_data_root() {
        let root = PathBuf::from("fixture-store-root");
        let plan =
            module_store_plan_for_data_root(root, Some("first-party.demo"), Some("1.0.0"), "seed");
        assert!(plan.data_root.ends_with("fixture-store-root"));
        assert!(plan.registry_path.contains(REGISTRY_FILE));
        assert!(plan.registry_path.contains("state"));
    }
}
