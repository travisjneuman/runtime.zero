//! Foundation-owned first-party module invocation.
//!
//! This is the first process-backed lifecycle slice. It resolves one installed
//! `first-party.inventory` record, revalidates its manifest and complete file
//! set, binds the exact executable identity to the shared process host,
//! and accepts only the module's path-redacted read-only JSON contract. It is
//! bounded to the active macOS inventory package; the developer-trial lane
//! remains a local fixture path. Neither lane is a native sandbox or a
//! third-party execution API.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rz0_artifact_identity::{
    ArtifactExpectation, bind_verified_executable, open_verified_executable,
    revalidate_verified_executable,
};
use rz0_cancellation_contract::cancellation_pair;
use rz0_inventory_contract::{InventoryReport, parse_inventory_report, validate_inventory_report};
use rz0_module_protocol::{
    EnvironmentPolicy, ExecutableBinding, ExecutablePathSource, INVOCATION_PLAN_CONTRACT,
    InventoryInvocation, InvocationPlan, PROTOCOL_SCHEMA_VERSION, ProtocolCapability,
    ProtocolOperation, ProtocolPlatform, SignatureBinding,
};
use rz0_process_host::{ProcessRequest, run_bound_read_only_process};
use rz0_resource_contract::{ProcessLimitCeilings, ProcessLimits};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub const DEVELOPER_INVOCATION_SCHEMA_VERSION: u16 = 1;
pub const DEVELOPER_INVOCATION_CONTRACT: &str = "developer_module_invocation";
pub const SIGNED_INVOCATION_CONTRACT: &str = "signed_module_invocation";
const INVENTORY_MODULE_ID: &str = "first-party.inventory";
const INVENTORY_EXECUTABLE: &str = "bin/rz0-inventory";
const INVENTORY_EXECUTABLE_WINDOWS: &str = "bin/rz0-inventory.exe";
const DEVELOPER_TRIAL_RECEIPT_PREFIX: &str = "receipts/install-";
const INVOCATION_TIMEOUT_MS: u64 = 2_000;
const INVOCATION_CONFIRMATION_TTL_SECONDS: u64 = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeveloperInvocationMode {
    DryRun,
    Apply {
        challenge_issued_unix_seconds: u64,
        confirmation: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeveloperInvocationRequest {
    pub module_id: String,
    pub store_root: PathBuf,
    pub mode: DeveloperInvocationMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedInvocationRequest {
    pub module_id: String,
    pub store_root: PathBuf,
    pub mode: DeveloperInvocationMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeveloperInvocationChallenge {
    pub plan_id: String,
    pub plan_sha256: String,
    pub issued_unix_seconds: u64,
    pub expires_unix_seconds: u64,
    pub expected_phrase: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeveloperInvocationStatus {
    NotExecuted,
    Success,
    Failed,
    TimedOut,
}

#[derive(Debug, Serialize)]
pub struct DeveloperInvocationReport {
    pub schema_version: u16,
    pub contract: &'static str,
    pub valid: bool,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub developer_trial: bool,
    pub product_execution_authorized: bool,
    pub module_id: String,
    pub module_version: Option<String>,
    pub dry_run: bool,
    pub execution_attempted: bool,
    pub status: DeveloperInvocationStatus,
    pub exit_code: Option<i32>,
    pub stdout_bytes: u64,
    pub stderr_bytes: u64,
    pub output_truncated: bool,
    pub payload_sha256: Option<String>,
    pub binding_mechanism: Option<String>,
    pub plan_id: Option<String>,
    pub plan_sha256: Option<String>,
    pub challenge: Option<DeveloperInvocationChallenge>,
    pub inventory: Option<InventoryReport>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

struct PreparedInvocation {
    module_version: String,
    module_root: PathBuf,
    executable_relative: String,
    executable_sha256: String,
    executable_size_bytes: u64,
    plan: InvocationPlan,
    plan_sha256: String,
    environment: Vec<(String, String)>,
}

pub fn developer_invocation_report(
    request: &DeveloperInvocationRequest,
) -> DeveloperInvocationReport {
    invocation_report(request, true)
}

pub fn signed_invocation_report(request: &SignedInvocationRequest) -> DeveloperInvocationReport {
    let internal = DeveloperInvocationRequest {
        module_id: request.module_id.clone(),
        store_root: request.store_root.clone(),
        mode: request.mode.clone(),
    };
    invocation_report(&internal, false)
}

fn invocation_report(
    request: &DeveloperInvocationRequest,
    developer_trial: bool,
) -> DeveloperInvocationReport {
    let dry_run = matches!(request.mode, DeveloperInvocationMode::DryRun);
    let mut report = empty_report(&request.module_id, dry_run, developer_trial);
    let prepared = match prepare_invocation(request, developer_trial) {
        Ok(prepared) => prepared,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };

    report.module_version = Some(prepared.module_version.clone());
    report.plan_id = Some(prepared.plan.request_id.clone());
    report.plan_sha256 = Some(prepared.plan_sha256.clone());
    report.warnings.extend([
        if developer_trial {
            "developer-only first-party inventory invocation; product execution authorization remains disabled".to_string()
        } else {
            "first-party release-key inventory invocation; only the active macOS module may execute".to_string()
        },
        "process containment is bounded transport, not a filesystem, network, privilege, syscall, or sandbox boundary".to_string(),
        "no module activation or registry mutation was performed".to_string(),
    ]);
    report.valid = true;

    let issued = match &request.mode {
        DeveloperInvocationMode::DryRun => now_unix_seconds(),
        DeveloperInvocationMode::Apply {
            challenge_issued_unix_seconds,
            ..
        } => *challenge_issued_unix_seconds,
    };
    let challenge = invocation_challenge(&prepared, issued);
    report.challenge = Some(challenge.clone());
    if dry_run {
        return report;
    }
    report.valid = false;

    let DeveloperInvocationMode::Apply {
        challenge_issued_unix_seconds,
        confirmation,
    } = &request.mode
    else {
        unreachable!("dry-run returned above");
    };
    let now = now_unix_seconds();
    if *challenge_issued_unix_seconds > now
        || now > challenge.expires_unix_seconds
        || confirmation != &challenge.expected_phrase
    {
        report.errors.push(
            "module invocation confirmation is expired or does not match the exact dry-run plan"
                .to_string(),
        );
        return report;
    }

    report.execution_attempted = true;
    let expectation = ArtifactExpectation {
        sha256: prepared.executable_sha256.clone(),
        size_bytes: prepared.executable_size_bytes,
    };
    let mut artifact = match open_verified_executable(
        &prepared.module_root,
        &prepared.executable_relative,
        &expectation,
    ) {
        Ok(artifact) => artifact,
        Err(error) => {
            report
                .errors
                .push(format!("open verified module executable: {error}"));
            return report;
        }
    };
    let binding = match bind_verified_executable(&artifact) {
        Ok(binding) => binding,
        Err(error) => {
            report
                .errors
                .push(format!("bind module executable: {error}"));
            return report;
        }
    };
    report.binding_mechanism = Some(binding.mechanism().as_str().to_string());
    let process_request = ProcessRequest {
        executable: artifact.canonical_path.clone(),
        arguments: vec!["--format".to_string(), "json".to_string()],
        working_directory: prepared.module_root.clone(),
        environment: prepared.environment.clone(),
        timeout: std::time::Duration::from_millis(INVOCATION_TIMEOUT_MS),
        output_limit: ProcessLimitCeilings::MODULE_SCHEMA_ONE.stdout_bytes,
    };
    let (_, cancellation) = cancellation_pair();
    let output = match run_bound_read_only_process(&process_request, &binding, &cancellation) {
        Ok(output) => output,
        Err(error) => {
            report.status =
                if error.foundation_code() == rz0_error_contract::FoundationErrorCode::Cancelled {
                    DeveloperInvocationStatus::TimedOut
                } else {
                    DeveloperInvocationStatus::Failed
                };
            report.errors.push(format!("run module process: {error}"));
            return report;
        }
    };
    drop(binding);
    if let Err(error) = revalidate_verified_executable(&mut artifact) {
        report.status = DeveloperInvocationStatus::Failed;
        report
            .errors
            .push(format!("revalidate module executable: {error}"));
        return report;
    }

    report.exit_code = output.status.code();
    report.stdout_bytes = output.stdout.total_bytes;
    report.stderr_bytes = output.stderr.total_bytes;
    report.output_truncated = output.stdout.truncated || output.stderr.truncated;
    report.payload_sha256 = Some(sha256_bytes(&output.stdout.bytes));
    if output.timed_out {
        report.status = DeveloperInvocationStatus::TimedOut;
        report
            .errors
            .push("module process exceeded the bounded deadline".to_string());
        return report;
    }
    if output.stdout.truncated
        || output.stderr.truncated
        || output.stderr.total_bytes > ProcessLimitCeilings::MODULE_SCHEMA_ONE.stderr_bytes
    {
        report.status = DeveloperInvocationStatus::Failed;
        report.errors.push(
            "module process output exceeded the schema-one stdout/stderr contract".to_string(),
        );
        return report;
    }
    if !output.status.success() {
        report.status = DeveloperInvocationStatus::Failed;
        report
            .errors
            .push("module process returned a non-success exit status".to_string());
        return report;
    }
    let inventory = match parse_inventory_report(&output.stdout.bytes) {
        Ok(inventory) => inventory,
        Err(error) => {
            report.status = DeveloperInvocationStatus::Failed;
            report
                .errors
                .push(format!("parse module inventory response: {error}"));
            return report;
        }
    };
    let validation = validate_inventory_report(&inventory);
    if !validation.valid
        || !inventory.path_values_redacted
        || inventory.writes_attempted
        || !inventory.read_only
        || inventory.runtime.module_id.as_deref() != Some(INVENTORY_MODULE_ID)
    {
        report.status = DeveloperInvocationStatus::Failed;
        report.errors.extend(validation.errors);
        report.errors.push(
            "module response did not satisfy the path-redacted read-only inventory contract"
                .to_string(),
        );
        return report;
    }
    report.status = DeveloperInvocationStatus::Success;
    report.inventory = Some(inventory);
    report.developer_trial = developer_trial;
    report.product_execution_authorized = !developer_trial;
    report.valid = true;
    report
}

fn prepare_invocation(
    request: &DeveloperInvocationRequest,
    developer_trial: bool,
) -> Result<PreparedInvocation, String> {
    if request.module_id != INVENTORY_MODULE_ID {
        return Err("module invocation supports only first-party.inventory".to_string());
    }
    let store =
        crate::store_status::store_status_report_for_root(&[], Some(request.store_root.clone()));
    if !store.read_only || store.writes_attempted {
        return Err("module store review unexpectedly reported a write".to_string());
    }
    if !matches!(
        store.registry.status,
        crate::installed_registry::InstalledRegistryState::Valid
    ) {
        return Err("installed module registry is not valid".to_string());
    }
    let record = store
        .registry
        .records
        .iter()
        .find(|record| record.valid && record.id == request.module_id)
        .ok_or_else(|| {
            "first-party.inventory is not installed in the selected store".to_string()
        })?;
    if developer_trial
        && !record
            .receipt_path
            .starts_with(DEVELOPER_TRIAL_RECEIPT_PREFIX)
    {
        return Err(
            "installed module receipt is not a developer-trial promotion receipt".to_string(),
        );
    }
    if !developer_trial
        && record.lifecycle_state != rz0_registry_contract::ACTIVE_MODULE_LIFECYCLE_STATE
    {
        return Err("first-party.inventory must be enabled before signed invocation".to_string());
    }
    let receipt = store
        .receipts
        .receipts
        .iter()
        .find(|receipt| receipt.reference_path == record.receipt_path)
        .ok_or_else(|| "installed module receipt was not assessed".to_string())?;
    if receipt.status != crate::install_receipt::InstallReceiptState::Valid
        || !receipt.module_matches_registry
    {
        return Err("installed module receipt is not valid for invocation".to_string());
    }
    let module_root = Path::new(&store.store.data_root).join(
        record
            .module_dir
            .as_deref()
            .unwrap_or(&format!("modules/{}/{}", record.id, record.version)),
    );
    let manifest_path = module_root.join("rz0-module.json");
    let manifest_report = crate::module_validation::load_manifest_file(&manifest_path);
    let manifest = manifest_report
        .manifest
        .ok_or_else(|| "installed module manifest could not be loaded".to_string())?;
    if !manifest_report.valid
        || manifest.id != INVENTORY_MODULE_ID
        || manifest.version != record.version
        || !matches!(
            manifest.status,
            crate::module_manifest::ModuleStatus::Installed
        )
        || manifest.safety.mutates_system
        || manifest.safety.remote_execution_allowed
    {
        return Err(
            "installed module manifest is not a valid read-only first-party package".to_string(),
        );
    }
    if !manifest
        .supported_platforms
        .iter()
        .any(|platform| platform == current_platform_name())
    {
        return Err("installed module does not support the current platform".to_string());
    }
    let integrity = manifest
        .integrity
        .as_ref()
        .ok_or_else(|| "installed module has no complete package integrity metadata".to_string())?;
    if !integrity.complete_file_set {
        return Err("module invocation requires a complete immutable package file set".to_string());
    }
    let executable_relative = if cfg!(windows) {
        INVENTORY_EXECUTABLE_WINDOWS
    } else {
        INVENTORY_EXECUTABLE
    };
    let executable = integrity
        .files
        .iter()
        .find(|file| file.path == executable_relative)
        .ok_or_else(|| format!("installed module does not declare {executable_relative}"))?;
    let executable_size_bytes = executable
        .size_bytes
        .ok_or_else(|| "installed module executable has no sealed size".to_string())?;
    let environment = current_environment()?;
    let plan = build_invocation_plan(
        &record.version,
        executable_relative,
        &executable.sha256,
        executable_size_bytes,
        &manifest_path,
        &environment,
        developer_trial,
    )?;
    let plan_bytes = serde_json::to_vec(&plan)
        .map_err(|error| format!("serialize module invocation plan: {error}"))?;
    Ok(PreparedInvocation {
        module_version: record.version.clone(),
        module_root,
        executable_relative: executable_relative.to_string(),
        executable_sha256: executable.sha256.clone(),
        executable_size_bytes,
        plan,
        plan_sha256: sha256_bytes(&plan_bytes),
        environment,
    })
}

fn build_invocation_plan(
    module_version: &str,
    executable_relative: &str,
    executable_sha256: &str,
    executable_size_bytes: u64,
    manifest_path: &Path,
    environment: &[(String, String)],
    developer_trial: bool,
) -> Result<InvocationPlan, String> {
    let manifest_sha256 = sha256_file(manifest_path)?;
    let allowed_names = environment
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>();
    let plan = InvocationPlan {
        schema_version: PROTOCOL_SCHEMA_VERSION,
        contract: INVOCATION_PLAN_CONTRACT.to_string(),
        request_id: format!("module-invoke-{}", &manifest_sha256[..16]),
        module_id: INVENTORY_MODULE_ID.to_string(),
        module_version: module_version.to_string(),
        platform: current_protocol_platform(),
        operation: ProtocolOperation::CollectInventory,
        dry_run: true,
        read_only: true,
        execution_authorized: false,
        execution_attempted: false,
        mutation_allowed: false,
        network_allowed: false,
        executable: ExecutableBinding {
            source: ExecutablePathSource::VerifiedReceipt,
            relative_path: executable_relative.to_string(),
            sha256: executable_sha256.to_string(),
            size_bytes: executable_size_bytes,
        },
        signature: SignatureBinding {
            verified: true,
            test_key_only: developer_trial,
            key_id: if developer_trial {
                "developer-trial"
            } else {
                "first-party-release"
            }
            .to_string(),
            manifest_sha256,
        },
        limits: ProcessLimits {
            timeout_ms: INVOCATION_TIMEOUT_MS,
            stdin_bytes: ProcessLimitCeilings::MODULE_SCHEMA_ONE.stdin_bytes,
            stdout_bytes: ProcessLimitCeilings::MODULE_SCHEMA_ONE.stdout_bytes,
            stderr_bytes: ProcessLimitCeilings::MODULE_SCHEMA_ONE.stderr_bytes,
        },
        environment: EnvironmentPolicy {
            clear_parent: true,
            inherit_parent: false,
            allowed_names,
        },
        capabilities: vec![
            ProtocolCapability::ProcessEnvironmentRead,
            ProtocolCapability::FilesystemMetadataRead,
        ],
        inventory: InventoryInvocation {
            include_apps: false,
            probe_versions: false,
            redact_paths: true,
        },
    };
    let validation = rz0_module_protocol::validate_invocation_plan(&plan);
    if validation.valid {
        Ok(plan)
    } else {
        Err(format!(
            "module invocation plan is invalid: {:?}",
            validation.errors
        ))
    }
}

fn current_environment() -> Result<Vec<(String, String)>, String> {
    let names = if cfg!(windows) {
        ["PATH", "SystemRoot"]
    } else {
        ["HOME", "PATH"]
    };
    let mut environment = BTreeMap::new();
    for name in names {
        let value = env::var(name)
            .map_err(|_| format!("required process environment variable {name} is unavailable"))?;
        environment.insert(name.to_string(), value);
    }
    Ok(environment.into_iter().collect())
}

fn current_platform_name() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

fn current_protocol_platform() -> ProtocolPlatform {
    if cfg!(windows) {
        ProtocolPlatform::Windows
    } else if cfg!(target_os = "macos") {
        ProtocolPlatform::Macos
    } else {
        ProtocolPlatform::Linux
    }
}

fn invocation_challenge(
    prepared: &PreparedInvocation,
    issued_unix_seconds: u64,
) -> DeveloperInvocationChallenge {
    let expires = issued_unix_seconds.saturating_add(INVOCATION_CONFIRMATION_TTL_SECONDS);
    let mut digest = Sha256::new();
    digest.update(b"runtime.zero.developer-module-invocation.v1\0");
    digest.update(prepared.plan_sha256.as_bytes());
    digest.update(prepared.executable_sha256.as_bytes());
    digest.update(issued_unix_seconds.to_be_bytes());
    digest.update(expires.to_be_bytes());
    let digest = format!("{:x}", digest.finalize());
    DeveloperInvocationChallenge {
        plan_id: prepared.plan.request_id.clone(),
        plan_sha256: prepared.plan_sha256.clone(),
        issued_unix_seconds,
        expires_unix_seconds: expires,
        expected_phrase: format!("RUN-MODULE-{}", &digest[..16]),
    }
}

fn empty_report(
    module_id: &str,
    dry_run: bool,
    developer_trial: bool,
) -> DeveloperInvocationReport {
    DeveloperInvocationReport {
        schema_version: DEVELOPER_INVOCATION_SCHEMA_VERSION,
        contract: if developer_trial {
            DEVELOPER_INVOCATION_CONTRACT
        } else {
            SIGNED_INVOCATION_CONTRACT
        },
        valid: false,
        read_only: true,
        writes_attempted: false,
        developer_trial,
        product_execution_authorized: false,
        module_id: module_id.to_string(),
        module_version: None,
        dry_run,
        execution_attempted: false,
        status: DeveloperInvocationStatus::NotExecuted,
        exit_code: None,
        stdout_bytes: 0,
        stderr_bytes: 0,
        output_truncated: false,
        payload_sha256: None,
        binding_mechanism: None,
        plan_id: None,
        plan_sha256: None,
        challenge: None,
        inventory: None,
        errors: Vec::new(),
        warnings: Vec::new(),
    }
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    Ok(sha256_bytes(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn developer_preview_plan_is_read_only_and_non_authorizing() {
        let environment = vec![
            ("HOME".to_string(), "/fixture/home".to_string()),
            ("PATH".to_string(), "/fixture/bin".to_string()),
        ];
        let plan = build_invocation_plan(
            "0.1.0",
            INVENTORY_EXECUTABLE,
            &"a".repeat(64),
            1024,
            Path::new("tests/fixtures/store-roots/valid-registry-valid-receipt/modules/first-party.inventory/0.1.0/rz0-module.json"),
            &environment,
            true,
        )
        .expect("read-only developer preview plan");
        assert!(plan.dry_run);
        assert!(plan.read_only);
        assert!(!plan.execution_authorized);
        assert!(!plan.execution_attempted);
        assert!(!plan.mutation_allowed);
        assert!(!plan.network_allowed);
    }

    #[test]
    fn invocation_fails_closed_without_a_promoted_store_record() {
        let report = developer_invocation_report(&DeveloperInvocationRequest {
            module_id: INVENTORY_MODULE_ID.to_string(),
            store_root: PathBuf::from("target/nonexistent-developer-invocation-store"),
            mode: DeveloperInvocationMode::DryRun,
        });
        assert!(!report.valid);
        assert!(!report.execution_attempted);
        assert!(!report.product_execution_authorized);
        assert!(!report.writes_attempted);
    }
}
