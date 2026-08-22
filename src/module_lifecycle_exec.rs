//! Foundation-owned lifecycle execution for the bounded macOS inventory module.
//!
//! Every mutating operation is scoped to an explicitly supplied initialized
//! store. The command rebuilds the exact plan from current state, binds a
//! short-lived confirmation to that plan, writes only runtime.zero-owned
//! state, and quarantines module bytes before removing their registry record.

use std::ffi::OsStr;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rz0_artifact_identity::{
    ArtifactExpectation, open_verified_artifact, revalidate_verified_artifact,
};
use rz0_module_lifecycle::{
    ModuleLifecycleOperation, ModuleLifecyclePlan, ModuleLifecycleState, module_lifecycle_plan,
};
use rz0_module_trust::{
    SignatureEnvelope, SignatureVerification, TrustedTestKey, verify_detached_signature,
};
use rz0_registry_contract::{
    ACTIVE_MODULE_LIFECYCLE_STATE, INSTALLED_MODULE_LIFECYCLE_STATE, InstalledModuleRecord,
    InstalledRegistry, bytes_sha256, canonical_registry_bytes, parse_registry_document,
};
use rz0_secure_fs::SecureDirectory;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::module_manifest::{ModuleKind, ModuleManifest, ModuleStatus};
use crate::module_store::{ModuleStorePlan, module_store_plan_for_data_root};
use crate::module_validation::load_manifest_file;

pub const MODULE_LIFECYCLE_EXECUTION_SCHEMA_VERSION: u16 = 1;
pub const MODULE_LIFECYCLE_EXECUTION_CONTRACT: &str = "module_lifecycle_execution";
const SUPPORTED_MODULE_ID: &str = "first-party.inventory";
const CONFIRMATION_TTL_SECONDS: u64 = 300;
const MAX_RECEIPT_BYTES: u64 = 128 * 1024;
const MAX_MODULE_FILE_BYTES: u64 = rz0_resource_contract::MAX_ARTIFACT_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleOperation {
    Enable,
    Disable,
    Update {
        package_path: PathBuf,
        signature_path: PathBuf,
        trusted_key_path: PathBuf,
    },
    Uninstall,
    Recover,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleMode {
    DryRun,
    Apply {
        challenge_issued_unix_seconds: u64,
        confirmation: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifecycleRequest {
    pub operation: LifecycleOperation,
    pub module_id: Option<String>,
    pub recovery_id: Option<String>,
    pub store_root: PathBuf,
    pub mode: LifecycleMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LifecycleChallenge {
    pub plan_id: String,
    pub plan_sha256: String,
    pub issued_unix_seconds: u64,
    pub expires_unix_seconds: u64,
    pub expected_phrase: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LifecycleExecutionReport {
    pub schema_version: u16,
    pub contract: &'static str,
    pub valid: bool,
    pub operation: LifecycleOperationLabel,
    pub module_id: Option<String>,
    pub from_state: Option<ModuleLifecycleState>,
    pub to_state: Option<ModuleLifecycleState>,
    pub from_version: Option<String>,
    pub to_version: Option<String>,
    pub dry_run: bool,
    pub read_only: bool,
    pub writes_attempted: bool,
    pub product_execution_authorized: bool,
    pub plan: Option<ModuleLifecyclePlan>,
    pub plan_sha256: Option<String>,
    pub challenge: Option<LifecycleChallenge>,
    pub transaction_id: Option<String>,
    pub receipt_path: Option<String>,
    pub recovery_id: Option<String>,
    pub signature_verification: Option<SignatureVerification>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub guidance: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleOperationLabel {
    Enable,
    Disable,
    Update,
    Uninstall,
    Recover,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryRecord {
    schema_version: u16,
    contract: String,
    recovery_id: String,
    module_id: String,
    module_version: String,
    original_module_dir: String,
    quarantine_dir: String,
    receipt_path: String,
    uninstall_plan_sha256: String,
    status: RecoveryStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum RecoveryStatus {
    Quarantined,
    Recovered,
}

#[derive(Debug, Clone)]
struct PreparedLifecycle {
    store: ModuleStorePlan,
    registry: InstalledRegistry,
    record: Option<InstalledModuleRecord>,
    recovery: Option<RecoveryRecord>,
    plan: ModuleLifecyclePlan,
    challenge: LifecycleChallenge,
    package: Option<VerifiedPackage>,
    signature_verification: Option<SignatureVerification>,
    operation_label: LifecycleOperationLabel,
    receipt_path: String,
    transaction_id: String,
}

#[derive(Debug, Clone)]
struct VerifiedPackage {
    manifest: ModuleManifest,
    manifest_sha256: String,
    files: Vec<PackageFile>,
    signature_verification: SignatureVerification,
}

#[derive(Debug, Clone)]
struct PackageFile {
    path: String,
    sha256: String,
    size_bytes: u64,
    bytes: Vec<u8>,
}

pub fn lifecycle_report(request: &LifecycleRequest) -> LifecycleExecutionReport {
    let dry_run = matches!(request.mode, LifecycleMode::DryRun);
    let operation_label = label(&request.operation);
    let mut report = empty_report(operation_label, dry_run);
    let prepared = match prepare(request) {
        Ok(prepared) => prepared,
        Err(error) => {
            report.errors.push(error);
            return report;
        }
    };
    report.module_id = prepared
        .record
        .as_ref()
        .map(|record| record.id.clone())
        .or_else(|| {
            prepared
                .recovery
                .as_ref()
                .map(|recovery| recovery.module_id.clone())
        });
    report.from_state = Some(prepared.plan.from_state);
    report.to_state = Some(prepared.plan.to_state);
    report.from_version = prepared.plan.from_version.clone();
    report.to_version = prepared.plan.to_version.clone();
    report.plan_sha256 = Some(prepared.plan.plan_sha256.clone());
    report.challenge = Some(prepared.challenge.clone());
    report.transaction_id = Some(prepared.transaction_id.clone());
    report.receipt_path = Some(prepared.receipt_path.clone());
    report.recovery_id = if matches!(prepared.operation_label, LifecycleOperationLabel::Uninstall) {
        Some(format!(
            "module-recovery-{}",
            &prepared.plan.plan_sha256[..16]
        ))
    } else {
        prepared
            .recovery
            .as_ref()
            .map(|recovery| recovery.recovery_id.clone())
    };
    report.signature_verification = prepared.signature_verification.clone();
    report.plan = Some(prepared.plan.clone());
    report.valid = true;
    report.warnings.push(
        "only the explicit runtime.zero store root is eligible for lifecycle mutation".to_string(),
    );
    if matches!(prepared.operation_label, LifecycleOperationLabel::Uninstall) {
        report.warnings.push(
            "uninstall is quarantine-first: module bytes are moved into the runtime.zero quarantine before the registry record is removed".to_string(),
        );
    }
    if dry_run {
        return report;
    }

    let LifecycleMode::Apply {
        challenge_issued_unix_seconds,
        confirmation,
    } = &request.mode
    else {
        unreachable!("dry-run returned above");
    };
    let now = unix_seconds();
    if *challenge_issued_unix_seconds > now
        || now > prepared.challenge.expires_unix_seconds
        || confirmation != &prepared.challenge.expected_phrase
    {
        report.valid = false;
        report.errors.push(
            "module lifecycle confirmation is expired or does not match the exact current plan"
                .to_string(),
        );
        return report;
    }
    match apply(&prepared, now) {
        Ok(()) => {
            report.read_only = false;
            report.writes_attempted = true;
            report.guidance.push(
                "fresh module status should be run before any subsequent lifecycle action"
                    .to_string(),
            );
            if matches!(prepared.operation_label, LifecycleOperationLabel::Uninstall) {
                report.recovery_id = Some(format!(
                    "module-recovery-{}",
                    &prepared.plan.plan_sha256[..16]
                ));
                report.guidance.push("the module is recoverable from the recorded quarantine receipt; do not delete the quarantine directory manually".to_string());
            }
        }
        Err(error) => {
            report.valid = false;
            report.writes_attempted = true;
            report.errors.push(error);
            report.guidance.push("preserve the transaction receipt and quarantine/recovery evidence; do not retry until the store is freshly inspected".to_string());
        }
    }
    report
}

fn prepare(request: &LifecycleRequest) -> Result<PreparedLifecycle, String> {
    let operation_label = label(&request.operation);
    if matches!(&request.operation, LifecycleOperation::Recover) {
        if request.module_id.is_some() {
            return Err("module recover accepts --recovery-id, not --module-id".to_string());
        }
    } else if request.module_id.as_deref() != Some(SUPPORTED_MODULE_ID) {
        return Err(format!(
            "this bounded lifecycle supports only --module-id {SUPPORTED_MODULE_ID}"
        ));
    }
    let store = module_store_plan_for_data_root(
        request.store_root.clone(),
        Some(SUPPORTED_MODULE_ID),
        Some("0.1.0"),
        "module lifecycle execution",
    );
    ensure_ready_store(&store)?;
    let registry_bytes = fs::read(&store.registry_path)
        .map_err(|error| format!("read installed module registry: {error}"))?;
    let registry = parse_registry_document(&registry_bytes)
        .map_err(|error| format!("installed module registry is invalid: {error}"))?;
    let (record, recovery) = match &request.operation {
        LifecycleOperation::Recover => {
            let id = request
                .recovery_id
                .as_deref()
                .ok_or_else(|| "module recover requires --recovery-id <id>".to_string())?;
            (None, Some(load_recovery(&store, id)?))
        }
        _ => {
            let record = registry
                .modules
                .iter()
                .find(|record| record.id == SUPPORTED_MODULE_ID)
                .cloned()
                .ok_or_else(|| {
                    "first-party.inventory is not installed in the selected store".to_string()
                })?;
            (Some(record), None)
        }
    };
    let (from_state, from_version, to_state, to_version, package, signature) =
        match (&request.operation, &record, &recovery) {
            (LifecycleOperation::Enable, Some(record), _) => {
                require_state(record, INSTALLED_MODULE_LIFECYCLE_STATE)?;
                (
                    ModuleLifecycleState::InstalledInactive,
                    record.version.clone(),
                    ModuleLifecycleState::Active,
                    record.version.clone(),
                    None,
                    None,
                )
            }
            (LifecycleOperation::Disable, Some(record), _) => {
                require_state(record, ACTIVE_MODULE_LIFECYCLE_STATE)?;
                (
                    ModuleLifecycleState::Active,
                    record.version.clone(),
                    ModuleLifecycleState::InstalledInactive,
                    record.version.clone(),
                    None,
                    None,
                )
            }
            (LifecycleOperation::Uninstall, Some(record), _) => {
                require_state(record, INSTALLED_MODULE_LIFECYCLE_STATE)?;
                (
                    ModuleLifecycleState::InstalledInactive,
                    record.version.clone(),
                    ModuleLifecycleState::Absent,
                    String::new(),
                    None,
                    None,
                )
            }
            (
                LifecycleOperation::Update {
                    package_path,
                    signature_path,
                    trusted_key_path,
                },
                Some(record),
                _,
            ) => {
                require_state(record, INSTALLED_MODULE_LIFECYCLE_STATE)?;
                let package =
                    verify_release_package(package_path, signature_path, trusted_key_path)?;
                if package.manifest.id != record.id {
                    return Err(
                        "module update package ID does not match the installed module".to_string(),
                    );
                }
                if package.manifest.version == record.version {
                    return Err(
                        "module update package version is not newer than the installed version"
                            .to_string(),
                    );
                }
                let signature = package.signature_verification.clone();
                (
                    ModuleLifecycleState::InstalledInactive,
                    record.version.clone(),
                    ModuleLifecycleState::InstalledInactive,
                    package.manifest.version.clone(),
                    Some(package),
                    Some(signature),
                )
            }
            (LifecycleOperation::Recover, _, Some(recovery)) => {
                if recovery.status != RecoveryStatus::Quarantined {
                    return Err("module recovery record is already recovered".to_string());
                }
                (
                    ModuleLifecycleState::Quarantined,
                    recovery.module_version.clone(),
                    ModuleLifecycleState::InstalledInactive,
                    recovery.module_version.clone(),
                    None,
                    None,
                )
            }
            _ => return Err("module lifecycle request has no usable current state".to_string()),
        };
    let operation = match &request.operation {
        LifecycleOperation::Enable => ModuleLifecycleOperation::Activate,
        LifecycleOperation::Disable => ModuleLifecycleOperation::Deactivate,
        LifecycleOperation::Update { .. } => ModuleLifecycleOperation::Upgrade,
        LifecycleOperation::Uninstall => ModuleLifecycleOperation::Uninstall,
        LifecycleOperation::Recover => ModuleLifecycleOperation::Recover,
    };
    let to_version = if matches!(&request.operation, LifecycleOperation::Uninstall) {
        None
    } else {
        Some(to_version)
    };
    let plan = module_lifecycle_plan(
        format!(
            "module-{}-{}-{}",
            operation_label_name(operation),
            SUPPORTED_MODULE_ID,
            from_version
        ),
        SUPPORTED_MODULE_ID,
        operation,
        from_state,
        to_state,
        Some(from_version),
        to_version,
    )
    .map_err(|validation| validation.errors.join("; "))?;
    let issued = match &request.mode {
        LifecycleMode::DryRun => unix_seconds(),
        LifecycleMode::Apply {
            challenge_issued_unix_seconds,
            ..
        } => *challenge_issued_unix_seconds,
    };
    let challenge = lifecycle_challenge(&plan, issued);
    let transaction_id = format!(
        "tx-{}-{}",
        operation_label_name(operation),
        &plan.plan_sha256[..16]
    );
    let receipt_path = format!("lifecycle-receipts/{transaction_id}.json");
    Ok(PreparedLifecycle {
        store,
        registry,
        record,
        recovery,
        plan,
        challenge,
        package,
        signature_verification: signature,
        operation_label,
        receipt_path,
        transaction_id,
    })
}

fn apply(prepared: &PreparedLifecycle, now: u64) -> Result<(), String> {
    match prepared.operation_label {
        LifecycleOperationLabel::Enable | LifecycleOperationLabel::Disable => {
            let mut registry = prepared.registry.clone();
            let record = registry
                .modules
                .iter_mut()
                .find(|record| record.id == SUPPORTED_MODULE_ID)
                .ok_or_else(|| "installed module disappeared before lifecycle apply".to_string())?;
            record.lifecycle_state = if prepared.operation_label == LifecycleOperationLabel::Enable
            {
                ACTIVE_MODULE_LIFECYCLE_STATE.to_string()
            } else {
                INSTALLED_MODULE_LIFECYCLE_STATE.to_string()
            };
            write_registry(&prepared.store, &registry)?;
            write_receipt(prepared, now, None)?;
        }
        LifecycleOperationLabel::Update => apply_update(prepared, now)?,
        LifecycleOperationLabel::Uninstall => apply_uninstall(prepared, now)?,
        LifecycleOperationLabel::Recover => apply_recover(prepared, now)?,
    }
    Ok(())
}

fn apply_update(prepared: &PreparedLifecycle, now: u64) -> Result<(), String> {
    let package = prepared
        .package
        .as_ref()
        .ok_or_else(|| "update package evidence is missing".to_string())?;
    let old = prepared
        .record
        .as_ref()
        .ok_or_else(|| "installed module record is missing".to_string())?;
    let destination_relative = format!(
        "modules/{}/{}",
        package.manifest.id, package.manifest.version
    );
    let destination = Path::new(&prepared.store.data_root).join(&destination_relative);
    if fs::symlink_metadata(&destination).is_ok() {
        return Err("module update destination already exists; refusing replacement".to_string());
    }
    copy_package(&prepared.store, package, &destination_relative)?;
    let receipt_path = format!("receipts/install-{}.json", prepared.transaction_id);
    write_install_receipt(prepared, package, &destination_relative, &receipt_path)?;
    let mut registry = prepared.registry.clone();
    let record = registry
        .modules
        .iter_mut()
        .find(|record| record.id == old.id)
        .ok_or_else(|| "installed module disappeared before update publication".to_string())?;
    record.version = package.manifest.version.clone();
    record.manifest_path = format!("{destination_relative}/rz0-module.json");
    record.module_dir = Some(destination_relative);
    record.receipt_path = receipt_path;
    write_registry(&prepared.store, &registry)?;
    write_receipt(prepared, now, Some(&package.manifest.version))
}

fn apply_uninstall(prepared: &PreparedLifecycle, now: u64) -> Result<(), String> {
    let record = prepared
        .record
        .as_ref()
        .ok_or_else(|| "installed module record is missing".to_string())?;
    let source_relative = record
        .module_dir
        .clone()
        .unwrap_or_else(|| format!("modules/{}/{}", record.id, record.version));
    let source = Path::new(&prepared.store.data_root).join(&source_relative);
    ensure_regular_module_directory(&source)?;
    let recovery_id = format!("module-recovery-{}", &prepared.plan.plan_sha256[..16]);
    let quarantine_relative = format!(
        "quarantine/modules/{recovery_id}/{}/{}",
        record.id, record.version
    );
    let quarantine = Path::new(&prepared.store.data_root).join(&quarantine_relative);
    let recovery = RecoveryRecord {
        schema_version: 1,
        contract: "module_quarantine_recovery".to_string(),
        recovery_id: recovery_id.clone(),
        module_id: record.id.clone(),
        module_version: record.version.clone(),
        original_module_dir: source_relative,
        quarantine_dir: quarantine_relative,
        receipt_path: record.receipt_path.clone(),
        uninstall_plan_sha256: prepared.plan.plan_sha256.clone(),
        status: RecoveryStatus::Quarantined,
    };
    write_recovery(&prepared.store, &recovery)?;
    fs::create_dir_all(
        quarantine
            .parent()
            .ok_or_else(|| "quarantine path has no parent".to_string())?,
    )
    .map_err(|error| format!("create quarantine parent: {error}"))?;
    fs::rename(&source, &quarantine)
        .map_err(|error| format!("quarantine module bytes: {error}"))?;
    let mut registry = prepared.registry.clone();
    registry
        .modules
        .retain(|candidate| candidate.id != record.id);
    if let Err(error) = write_registry(&prepared.store, &registry) {
        let _ = fs::rename(&quarantine, &source);
        return Err(format!(
            "publish uninstall registry and restore module bytes: {error}"
        ));
    }
    write_receipt(prepared, now, Some(&recovery_id))
}

fn apply_recover(prepared: &PreparedLifecycle, now: u64) -> Result<(), String> {
    let recovery = prepared
        .recovery
        .as_ref()
        .ok_or_else(|| "module recovery record is missing".to_string())?;
    let source = Path::new(&prepared.store.data_root).join(&recovery.quarantine_dir);
    let destination = Path::new(&prepared.store.data_root).join(&recovery.original_module_dir);
    ensure_regular_module_directory(&source)?;
    if fs::symlink_metadata(&destination).is_ok() {
        return Err("recovery destination is occupied; refusing replacement".to_string());
    }
    fs::create_dir_all(
        destination
            .parent()
            .ok_or_else(|| "recovery destination has no parent".to_string())?,
    )
    .map_err(|error| format!("create recovery destination parent: {error}"))?;
    fs::rename(&source, &destination)
        .map_err(|error| format!("restore quarantined module: {error}"))?;
    let mut registry = prepared.registry.clone();
    registry.modules.push(InstalledModuleRecord {
        id: recovery.module_id.clone(),
        version: recovery.module_version.clone(),
        manifest_path: format!("{}/rz0-module.json", recovery.original_module_dir),
        receipt_path: recovery.receipt_path.clone(),
        lifecycle_state: INSTALLED_MODULE_LIFECYCLE_STATE.to_string(),
        module_dir: Some(recovery.original_module_dir.clone()),
    });
    registry
        .modules
        .sort_by(|left, right| left.id.cmp(&right.id));
    if let Err(error) = write_registry(&prepared.store, &registry) {
        let _ = fs::rename(&destination, &source);
        return Err(format!(
            "publish recovered registry and restore quarantine: {error}"
        ));
    }
    let mut recovered = recovery.clone();
    recovered.status = RecoveryStatus::Recovered;
    write_recovery(&prepared.store, &recovered)?;
    write_receipt(prepared, now, Some(&recovery.recovery_id))
}

fn write_receipt(
    prepared: &PreparedLifecycle,
    now: u64,
    result: Option<&str>,
) -> Result<(), String> {
    let value = serde_json::json!({
        "schema_version": 1,
        "contract": MODULE_LIFECYCLE_EXECUTION_CONTRACT,
        "transaction_id": prepared.transaction_id,
        "plan_sha256": prepared.plan.plan_sha256,
        "operation": prepared.operation_label,
        "module_id": SUPPORTED_MODULE_ID,
        "from_state": prepared.plan.from_state,
        "to_state": prepared.plan.to_state,
        "committed_unix_seconds": now,
        "result": result,
        "writes_attempted": true,
        "automatic_mutation_authorized": false
    });
    let bytes = serde_json::to_vec_pretty(&value)
        .map_err(|error| format!("serialize lifecycle receipt: {error}"))?;
    write_new_state_file(&prepared.store, &prepared.receipt_path, &bytes)
}

fn write_install_receipt(
    prepared: &PreparedLifecycle,
    package: &VerifiedPackage,
    destination: &str,
    receipt_path: &str,
) -> Result<(), String> {
    let write_set = package
        .files
        .iter()
        .map(|file| {
            serde_json::json!({
                "path": format!("{destination}/{}", file.path),
                "kind": if file.path == "rz0-module.json" { "manifest" } else { "file" },
                "sha256": file.sha256,
                "size_bytes": file.size_bytes
            })
        })
        .chain(std::iter::once(
            serde_json::json!({"path": receipt_path, "kind": "receipt"}),
        ))
        .collect::<Vec<_>>();
    let value = serde_json::json!({
        "schema_version": 1,
        "module": {"id": package.manifest.id, "version": package.manifest.version},
        "source": {"source_type": "first_party_release", "package_reference": "signed-local-package"},
        "target": {"module_dir": destination, "manifest_path": format!("{destination}/rz0-module.json")},
        "integrity": {"manifest_sha256": package.manifest_sha256, "package_sha256": package_digest(&package.files)},
        "lifecycle": {"state": INSTALLED_MODULE_LIFECYCLE_STATE, "activation_authorized": false, "invocation_authorized": false},
        "write_set": write_set,
        "rollback": {"supported": true, "plan_path": format!("receipts/{}.rollback.json", prepared.transaction_id)},
        "quarantine": {"supported": true, "record_path": format!("quarantine/modules/{}.json", prepared.transaction_id)}
    });
    let bytes = serde_json::to_vec_pretty(&value)
        .map_err(|error| format!("serialize module install receipt: {error}"))?;
    write_new_state_file(&prepared.store, receipt_path, &bytes)
}

fn verify_release_package(
    package_path: &Path,
    signature_path: &Path,
    key_path: &Path,
) -> Result<VerifiedPackage, String> {
    let manifest_path = if package_path.is_dir() {
        package_path.join("rz0-module.json")
    } else {
        package_path.to_path_buf()
    };
    let package_root = manifest_path
        .parent()
        .ok_or_else(|| "module package has no parent".to_string())?;
    let validation = load_manifest_file(&manifest_path);
    let manifest = validation
        .manifest
        .clone()
        .ok_or_else(|| "module update manifest is invalid".to_string())?;
    if !validation.valid
        || manifest.id != SUPPORTED_MODULE_ID
        || manifest.kind != ModuleKind::FirstPartyModule
        || manifest.status != ModuleStatus::Installed
        || manifest.safety.mutates_system
    {
        return Err(
            "module update accepts only the signed read-only first-party.inventory package"
                .to_string(),
        );
    }
    if !manifest
        .supported_platforms
        .iter()
        .any(|platform| platform == "macos")
    {
        return Err("module update package does not declare macOS support".to_string());
    }
    let integrity = manifest
        .integrity
        .as_ref()
        .ok_or_else(|| "module update package has no complete integrity set".to_string())?;
    if !integrity.complete_file_set {
        return Err("module update package must declare a complete immutable file set".to_string());
    }
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|error| format!("read module update manifest: {error}"))?;
    let manifest_sha256 = bytes_sha256(&manifest_bytes);
    let envelope = read_json::<SignatureEnvelope>(signature_path, "module signature envelope")?;
    let key = read_json::<TrustedTestKey>(key_path, "trusted module key")?;
    let signature_verification = verify_detached_signature(&envelope, &key);
    if !signature_verification.verified || signature_verification.test_key_only {
        return Err(format!(
            "module update requires a verified first-party release signature: {}",
            signature_verification.errors.join("; ")
        ));
    }
    if envelope.package_id != manifest.id
        || envelope.package_version != manifest.version
        || envelope.manifest_sha256 != manifest_sha256
    {
        return Err(
            "module update signature does not bind the exact manifest identity".to_string(),
        );
    }
    let mut files = Vec::new();
    let mut paths = vec![PackageFileExpectation {
        path: "rz0-module.json".to_string(),
        sha256: manifest_sha256.clone(),
        size_bytes: manifest_bytes.len() as u64,
    }];
    for file in &integrity.files {
        paths.push(PackageFileExpectation {
            path: file.path.clone(),
            sha256: file.sha256.clone(),
            size_bytes: file
                .size_bytes
                .ok_or_else(|| format!("module file {} has no sealed size", file.path))?,
        });
    }
    paths.sort_by(|left, right| left.path.cmp(&right.path));
    paths.dedup_by(|left, right| left.path == right.path);
    for expected in paths {
        if expected.size_bytes > MAX_MODULE_FILE_BYTES {
            return Err(format!(
                "module file {} exceeds the foundation artifact byte ceiling",
                expected.path
            ));
        }
        let expectation = ArtifactExpectation {
            sha256: expected.sha256.clone(),
            size_bytes: expected.size_bytes,
        };
        let mut artifact = open_verified_artifact(package_root, &expected.path, &expectation)
            .map_err(|error| format!("open verified module file {}: {error}", expected.path))?;
        revalidate_verified_artifact(&mut artifact)
            .map_err(|error| format!("revalidate module file {}: {error}", expected.path))?;
        let mut file = artifact.into_file();
        file.seek(SeekFrom::Start(0))
            .map_err(|error| format!("seek module file {}: {error}", expected.path))?;
        let mut bytes = Vec::new();
        file.take(expected.size_bytes.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|error| format!("read module file {}: {error}", expected.path))?;
        if bytes.len() as u64 != expected.size_bytes || bytes_sha256(&bytes) != expected.sha256 {
            return Err(format!(
                "module file {} changed after identity validation",
                expected.path
            ));
        }
        files.push(PackageFile {
            path: expected.path,
            sha256: expected.sha256,
            size_bytes: bytes.len() as u64,
            bytes,
        });
    }
    Ok(VerifiedPackage {
        manifest,
        manifest_sha256,
        files,
        signature_verification,
    })
}

#[derive(Debug, Clone)]
struct PackageFileExpectation {
    path: String,
    sha256: String,
    size_bytes: u64,
}

fn copy_package(
    store: &ModuleStorePlan,
    package: &VerifiedPackage,
    _destination: &str,
) -> Result<(), String> {
    let modules_root = SecureDirectory::open(Path::new(&store.modules_root))
        .map_err(|error| format!("open module store: {error}"))?;
    modules_root
        .verify_private()
        .map_err(|error| format!("verify module store: {error}"))?;
    let id_root = modules_root
        .open_or_create_child_directory(OsStr::new(&package.manifest.id))
        .map_err(|error| format!("open module ID directory: {error}"))?;
    let version_root = id_root
        .create_child_directory(OsStr::new(&package.manifest.version))
        .map_err(|error| format!("create module version directory: {error}"))?;
    for file in &package.files {
        if !rz0_validation_contract::valid_contract_relative_path(&file.path) {
            return Err(format!("module package file path is unsafe: {}", file.path));
        }
        let parts = file.path.split('/').collect::<Vec<_>>();
        let mut directory = version_root.try_clone().map_err(|error| {
            format!("clone module version directory for {}: {error}", file.path)
        })?;
        for part in &parts[..parts.len() - 1] {
            directory = directory
                .open_or_create_child_directory(OsStr::new(part))
                .map_err(|error| format!("create module file directory {}: {error}", file.path))?;
        }
        directory
            .write_new_child(
                OsStr::new(parts[parts.len() - 1]),
                &file.bytes,
                MAX_MODULE_FILE_BYTES,
            )
            .map_err(|error| format!("write module file {}: {error}", file.path))?;
        if file.path.starts_with("bin/") {
            directory
                .mark_child_executable(OsStr::new(parts[parts.len() - 1]))
                .map_err(|error| format!("mark module executable {}: {error}", file.path))?;
        }
    }
    Ok(())
}

fn ensure_ready_store(store: &ModuleStorePlan) -> Result<(), String> {
    for path in [&store.data_root, &store.state_root, &store.modules_root] {
        let directory = SecureDirectory::open(Path::new(path))
            .map_err(|error| format!("open initialized module store: {error}"))?;
        directory
            .verify_private()
            .map_err(|error| format!("verify initialized module store: {error}"))?;
    }
    if !Path::new(&store.registry_path).is_file() {
        return Err(
            "module lifecycle requires an initialized installed-module registry".to_string(),
        );
    }
    Ok(())
}

fn write_registry(store: &ModuleStorePlan, registry: &InstalledRegistry) -> Result<(), String> {
    let bytes = canonical_registry_bytes(registry)
        .map_err(|error| format!("serialize module registry: {error}"))?;
    let state = SecureDirectory::open(Path::new(&store.state_root))
        .map_err(|error| format!("open module state: {error}"))?;
    state
        .verify_private()
        .map_err(|error| format!("verify module state: {error}"))?;
    let pending_name = format!(".installed-modules.pending-{}", &bytes_sha256(&bytes)[..16]);
    state
        .write_new_child(
            OsStr::new(&pending_name),
            &bytes,
            rz0_resource_contract::MAX_REGISTRY_DOCUMENT_BYTES,
        )
        .map_err(|error| format!("write pending module registry: {error}"))?;
    state
        .replace_child_atomic(
            OsStr::new(&pending_name),
            &state,
            OsStr::new("installed-modules.json"),
        )
        .map_err(|error| format!("publish module registry atomically: {error}"))?;
    Ok(())
}

fn write_new_state_file(
    store: &ModuleStorePlan,
    relative: &str,
    bytes: &[u8],
) -> Result<(), String> {
    let parts = relative.split('/').collect::<Vec<_>>();
    if parts.len() < 2
        || parts
            .iter()
            .any(|part| part.is_empty() || *part == "." || *part == "..")
    {
        return Err("lifecycle state path is unsafe".to_string());
    }
    let state = SecureDirectory::open(Path::new(&store.state_root))
        .map_err(|error| format!("open module state: {error}"))?;
    state
        .verify_private()
        .map_err(|error| format!("verify module state: {error}"))?;
    let mut directory = state;
    for part in &parts[..parts.len() - 1] {
        directory = directory
            .open_or_create_child_directory(OsStr::new(part))
            .map_err(|error| format!("open lifecycle state directory: {error}"))?;
    }
    directory
        .write_new_child(OsStr::new(parts[parts.len() - 1]), bytes, MAX_RECEIPT_BYTES)
        .map_err(|error| format!("write lifecycle state receipt: {error}"))?;
    Ok(())
}

fn write_recovery(store: &ModuleStorePlan, recovery: &RecoveryRecord) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(recovery)
        .map_err(|error| format!("serialize recovery record: {error}"))?;
    write_or_replace_state_file(
        store,
        &format!("recovery/{}.json", recovery.recovery_id),
        &bytes,
    )
}

fn write_or_replace_state_file(
    store: &ModuleStorePlan,
    relative: &str,
    bytes: &[u8],
) -> Result<(), String> {
    let parts = relative.split('/').collect::<Vec<_>>();
    if parts.len() < 2
        || parts
            .iter()
            .any(|part| part.is_empty() || *part == "." || *part == "..")
    {
        return Err("lifecycle state path is unsafe".to_string());
    }
    let state = SecureDirectory::open(Path::new(&store.state_root))
        .map_err(|error| format!("open module state: {error}"))?;
    state
        .verify_private()
        .map_err(|error| format!("verify module state: {error}"))?;
    let mut directory = state;
    for part in &parts[..parts.len() - 1] {
        directory = directory
            .open_or_create_child_directory(OsStr::new(part))
            .map_err(|error| format!("open lifecycle state directory: {error}"))?;
    }
    let target = OsStr::new(parts[parts.len() - 1]);
    let pending = OsStr::new(".recovery.pending");
    directory
        .write_new_child(pending, bytes, MAX_RECEIPT_BYTES)
        .map_err(|error| format!("write pending recovery record: {error}"))?;
    if directory.open_child_file(target).is_ok() {
        directory
            .replace_child_atomic(pending, &directory, target)
            .map_err(|error| format!("publish recovery record: {error}"))?;
    } else {
        directory
            .publish_child_noreplace(pending, &directory, target)
            .map_err(|error| format!("publish recovery record: {error}"))?;
    }
    Ok(())
}

fn load_recovery(store: &ModuleStorePlan, recovery_id: &str) -> Result<RecoveryRecord, String> {
    if !rz0_validation_contract::valid_ledger_id(recovery_id, 96) {
        return Err("recovery ID is invalid".to_string());
    }
    let path = Path::new(&store.state_root)
        .join("recovery")
        .join(format!("{recovery_id}.json"));
    read_json(&path, "module recovery record")
}

fn require_state(record: &InstalledModuleRecord, expected: &str) -> Result<(), String> {
    if record.lifecycle_state != expected {
        return Err(format!(
            "module {} is {}, expected {} for this action",
            record.id, record.lifecycle_state, expected
        ));
    }
    Ok(())
}

fn ensure_regular_module_directory(path: &Path) -> Result<(), String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("inspect module directory: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err("module directory is not a regular directory".to_string());
    }
    Ok(())
}

fn lifecycle_challenge(plan: &ModuleLifecyclePlan, issued: u64) -> LifecycleChallenge {
    let expires = issued.saturating_add(CONFIRMATION_TTL_SECONDS);
    let mut digest = Sha256::new();
    digest.update(b"runtime.zero.module-lifecycle-confirmation.v1\0");
    digest.update(plan.plan_sha256.as_bytes());
    digest.update(issued.to_be_bytes());
    digest.update(expires.to_be_bytes());
    let digest = format!("{:x}", digest.finalize());
    LifecycleChallenge {
        plan_id: plan.transition_id.clone(),
        plan_sha256: plan.plan_sha256.clone(),
        issued_unix_seconds: issued,
        expires_unix_seconds: expires,
        expected_phrase: format!("CONFIRM-MODULE-{}", &digest[..16]),
    }
}

fn label(operation: &LifecycleOperation) -> LifecycleOperationLabel {
    match operation {
        LifecycleOperation::Enable => LifecycleOperationLabel::Enable,
        LifecycleOperation::Disable => LifecycleOperationLabel::Disable,
        LifecycleOperation::Update { .. } => LifecycleOperationLabel::Update,
        LifecycleOperation::Uninstall => LifecycleOperationLabel::Uninstall,
        LifecycleOperation::Recover => LifecycleOperationLabel::Recover,
    }
}

fn operation_label_name(operation: ModuleLifecycleOperation) -> &'static str {
    match operation {
        ModuleLifecycleOperation::Install => "install",
        ModuleLifecycleOperation::Activate => "activate",
        ModuleLifecycleOperation::Invoke => "invoke",
        ModuleLifecycleOperation::Deactivate => "deactivate",
        ModuleLifecycleOperation::Repair => "repair",
        ModuleLifecycleOperation::Migrate => "migrate",
        ModuleLifecycleOperation::Upgrade => "upgrade",
        ModuleLifecycleOperation::Uninstall => "uninstall",
        ModuleLifecycleOperation::Recover => "recover",
    }
}

fn package_digest(files: &[PackageFile]) -> String {
    let mut digest = Sha256::new();
    digest.update(b"runtime.zero.first-party-package.v1\0");
    for file in files {
        digest.update((file.path.len() as u64).to_be_bytes());
        digest.update(file.path.as_bytes());
        digest.update(file.sha256.as_bytes());
        digest.update(file.size_bytes.to_be_bytes());
    }
    format!("{:x}", digest.finalize())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path, label: &str) -> Result<T, String> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| format!("inspect {label}: {error}"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("{label} must be a regular file"));
    }
    if metadata.len() > rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES {
        return Err(format!("{label} exceeds the foundation byte ceiling"));
    }
    let bytes = fs::read(path).map_err(|error| format!("read {label}: {error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("parse {label}: {error}"))
}

fn empty_report(operation: LifecycleOperationLabel, dry_run: bool) -> LifecycleExecutionReport {
    LifecycleExecutionReport {
        schema_version: MODULE_LIFECYCLE_EXECUTION_SCHEMA_VERSION,
        contract: MODULE_LIFECYCLE_EXECUTION_CONTRACT,
        valid: false,
        operation,
        module_id: None,
        from_state: None,
        to_state: None,
        from_version: None,
        to_version: None,
        dry_run,
        read_only: dry_run,
        writes_attempted: false,
        product_execution_authorized: false,
        plan: None,
        plan_sha256: None,
        challenge: None,
        transaction_id: None,
        receipt_path: None,
        recovery_id: None,
        signature_verification: None,
        errors: Vec::new(),
        warnings: Vec::new(),
        guidance: vec![
            "this lifecycle slice never mutates outside the selected runtime.zero module store"
                .to_string(),
        ],
    }
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
