//! Developer-only signed module staging.
//!
//! This is deliberately narrower than module installation. It accepts only a
//! locally selected first-party package, verifies it with the public test-key
//! trust fixture, copies the verified bytes into the private runtime.zero
//! module store, and records transaction evidence. It does not publish an
//! installed registry record, activate a module, invoke code, or provide a
//! production trust root.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rz0_action_plan::{
    ActionCapability, ActionDisposition, ActionExecutableIdentity, ActionKind, ActionPlan,
    ActionRisk, PlanAction, RollbackPlan, WriteKind, WriteSetEntry, action_plan_digests,
    validate_action_plan,
};
use rz0_artifact_identity::{
    ArtifactExpectation, VerifiedArtifact, open_verified_artifact, revalidate_verified_artifact,
};
use rz0_confirmation_contract::{
    ConfirmationChallenge, ConfirmationConsumption, ConfirmationResponse, ConfirmationRisk,
    ConfirmationSurface, seal_confirmation_challenge, seal_confirmation_consumption,
    validate_confirmation,
};
use rz0_module_trust::{
    SignatureEnvelope, SignatureVerification, TrustedTestKey, verify_detached_signature,
};
use rz0_registry_contract::{
    InstalledRegistry, bytes_sha256, canonical_registry_bytes, parse_registry_document,
};
use rz0_secure_fs::SecureDirectory;
use rz0_transaction_contract::{
    CommitCoordinatorInput, DurabilityRequirements, TransactionCommitReceipt, TransactionEvent,
    TransactionEventKind, TransactionJournal, TransactionOperation, TransactionState,
    inspect_journal_head, publish_committed_state, publish_confirmation_consumption,
    publish_journal_snapshot, seal_commit_receipt, seal_transaction_journal,
    validate_commit_receipt,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::module_manifest::{ModuleKind, ModuleManifest, ModuleStatus};
use crate::module_store::{ModuleStorePlan, module_store_plan_for_data_root};
use crate::module_validation::{ManifestValidationReport, load_manifest_file};

const MODULE_MANIFEST_FILE: &str = "rz0-module.json";
const STORE_INIT_MARKER: &str = "store-init.json";
const STAGING_RECEIPTS_DIRECTORY: &str = "staging-receipts";
const TRANSACTIONS_DIRECTORY: &str = "transactions";
const RECEIPTS_DIRECTORY: &str = "receipts";
const MAX_TRUST_DOCUMENT_BYTES: u64 = rz0_resource_contract::MAX_SMALL_DOCUMENT_BYTES;
const STAGE_SAFETY_NOTE: &str = "Developer-only local staging; test-key trust, activation, invocation, registry installation, network fetch, and production release authority remain disabled.";
const REDACTED_SOURCE_MANIFEST: &str = "<local-package>/rz0-module.json";
const MAX_STAGING_RECEIPTS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeveloperStageMode {
    DryRun,
    Apply {
        challenge_issued_unix_seconds: u64,
        confirmation: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeveloperStageRequest {
    pub package_path: PathBuf,
    pub signature_path: PathBuf,
    pub trusted_key_path: PathBuf,
    pub store_root: PathBuf,
    pub mode: DeveloperStageMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeveloperStageReport {
    pub schema_version: u16,
    pub contract: &'static str,
    pub valid: bool,
    pub developer_only: bool,
    pub test_key_only: bool,
    pub dry_run: bool,
    pub writes_attempted: bool,
    pub product_execution_authorized: bool,
    pub activation_authorized: bool,
    pub invocation_authorized: bool,
    pub package_id: Option<String>,
    pub package_version: Option<String>,
    pub manifest_sha256: Option<String>,
    pub trusted_key_id: Option<String>,
    pub source_manifest_path: String,
    pub destination_relative: Option<String>,
    pub stage_receipt_path: Option<String>,
    pub commit_receipt_path: Option<String>,
    pub transaction_id: Option<String>,
    pub plan_id: Option<String>,
    pub plan_sha256: Option<String>,
    pub write_set_sha256: Option<String>,
    pub manifest_validation: Option<ManifestValidationReport>,
    pub signature_verification: Option<SignatureVerification>,
    pub files: Vec<StageFile>,
    pub challenge: Option<DeveloperStageChallenge>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub safety_note: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageFile {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeveloperStageChallenge {
    pub plan_id: String,
    pub transaction_id: String,
    pub issued_unix_seconds: u64,
    pub expires_unix_seconds: u64,
    pub expected_phrase: String,
    pub challenge_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeveloperStagedModuleStatus {
    pub id: String,
    pub version: String,
    pub state: rz0_module_lifecycle::ModuleLifecycleState,
    pub valid: bool,
    pub manifest_sha256: Option<String>,
    pub trusted_test_key_id: Option<String>,
    pub destination_relative: Option<String>,
    pub errors: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DeveloperStagingInventory {
    pub modules: Vec<DeveloperStagedModuleStatus>,
    pub warnings: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeveloperStageReceipt {
    schema_version: u16,
    contract: String,
    stage_id: String,
    transaction_id: String,
    plan_id: String,
    module_id: String,
    module_version: String,
    manifest_sha256: String,
    trusted_test_key_id: String,
    destination: String,
    files: Vec<StageFile>,
    lifecycle_state: String,
    developer_only: bool,
    test_key_only: bool,
    activation_authorized: bool,
    invocation_authorized: bool,
    product_execution_authorized: bool,
    writes_attempted: bool,
}

struct PreparedStage {
    manifest: ModuleManifest,
    manifest_validation: ManifestValidationReport,
    signature_verification: SignatureVerification,
    trusted_key_id: String,
    manifest_sha256: String,
    files: Vec<StageFileContent>,
    store: ModuleStorePlan,
    registry: InstalledRegistry,
    registry_before_sha256: String,
    registry_after_sha256: String,
    action_plan: ActionPlan,
    plan_sha256: String,
    write_set_sha256: String,
    transaction_id: String,
    stage_id: String,
    destination_relative: String,
    stage_receipt_relative: String,
    commit_receipt_relative: String,
}

struct StageFileContent {
    metadata: StageFile,
    bytes: Vec<u8>,
}

pub fn developer_stage_report(request: &DeveloperStageRequest) -> DeveloperStageReport {
    let source_manifest_path = resolve_manifest_path(&request.package_path);
    let prepared = match prepare_stage(request, &source_manifest_path) {
        Ok(prepared) => prepared,
        Err(failure) => return failure.into_report(&source_manifest_path),
    };

    let (challenge, full_challenge, is_apply) = match &request.mode {
        DeveloperStageMode::DryRun => {
            let full = match build_confirmation_challenge(&prepared, unix_seconds()) {
                Ok(challenge) => challenge,
                Err(error) => {
                    return prepared.report(
                        source_manifest_path,
                        false,
                        false,
                        None,
                        vec![error],
                        Vec::new(),
                    );
                }
            };
            (challenge_view(&prepared, &full), full, false)
        }
        DeveloperStageMode::Apply {
            challenge_issued_unix_seconds,
            ..
        } => match build_confirmation_challenge(&prepared, *challenge_issued_unix_seconds) {
            Ok(challenge) => (challenge_view(&prepared, &challenge), challenge, true),
            Err(error) => {
                return prepared.report(
                    source_manifest_path,
                    false,
                    true,
                    None,
                    vec![error],
                    Vec::new(),
                );
            }
        },
    };

    if !is_apply {
        return prepared.report(
            source_manifest_path,
            true,
            false,
            Some(challenge),
            Vec::new(),
            Vec::new(),
        );
    }

    let DeveloperStageMode::Apply {
        challenge_issued_unix_seconds: _,
        confirmation,
    } = &request.mode
    else {
        unreachable!("dry-run returned above");
    };
    let now = unix_seconds();
    let response = match confirmation_response(&full_challenge, confirmation, now) {
        Ok(response) => response,
        Err(error) => {
            return prepared.report(
                source_manifest_path,
                false,
                true,
                Some(challenge),
                vec![error],
                Vec::new(),
            );
        }
    };
    match apply_stage(&prepared, &full_challenge, &response, now) {
        Ok(receipt) => prepared.report(
            source_manifest_path,
            true,
            true,
            Some(challenge),
            Vec::new(),
            vec![format!(
                "developer stage committed as {}",
                receipt.transaction_id
            )],
        ),
        Err(error) => prepared.report(
            source_manifest_path,
            false,
            true,
            Some(challenge),
            vec![error],
            Vec::new(),
        ),
    }
}

fn prepare_stage(
    request: &DeveloperStageRequest,
    source_manifest_path: &Path,
) -> Result<PreparedStage, StageFailure> {
    let package_root = source_manifest_path
        .parent()
        .ok_or_else(|| StageFailure::new("package manifest has no parent directory"))?;
    let package_root = fs::canonicalize(package_root)
        .map_err(|error| StageFailure::new(format!("canonicalize package root: {error}")))?;
    if source_manifest_path.file_name() != Some(OsStr::new(MODULE_MANIFEST_FILE)) {
        return Err(StageFailure::new(
            "developer staging requires a manifest named rz0-module.json",
        ));
    }
    let manifest_path = package_root.join(MODULE_MANIFEST_FILE);
    let manifest_validation = load_manifest_file(&manifest_path);
    let manifest = manifest_validation.manifest.clone().ok_or_else(|| {
        StageFailure::with_manifest("module manifest is invalid", manifest_validation.clone())
    })?;
    let manifest_sha256 = sha256_bytes(&fs::read(&manifest_path).map_err(|error| {
        StageFailure::with_manifest(
            format!("read module manifest: {error}"),
            manifest_validation.clone(),
        )
    })?);
    if !manifest_validation.valid {
        return Err(StageFailure::with_manifest(
            "module manifest or package integrity validation failed",
            manifest_validation,
        ));
    }
    if manifest.kind != ModuleKind::FirstPartyModule
        || manifest.status != ModuleStatus::Installed
        || manifest.safety.mutates_system
    {
        return Err(StageFailure::with_manifest(
            "developer staging accepts only installed-status, read-only first-party modules",
            manifest_validation,
        ));
    }

    let envelope = read_json::<SignatureEnvelope>(&request.signature_path, "signature envelope")?;
    let trusted_key = read_json::<TrustedTestKey>(&request.trusted_key_path, "trusted test key")?;
    let signature_verification = verify_detached_signature(&envelope, &trusted_key);
    if !signature_verification.verified || !signature_verification.test_key_only {
        return Err(StageFailure {
            manifest_validation: Some(Box::new(manifest_validation)),
            signature_verification: Some(Box::new(signature_verification)),
            errors: vec!["detached signature verification failed".to_string()],
        });
    }
    if envelope.package_id != manifest.id
        || envelope.package_version != manifest.version
        || envelope.manifest_sha256 != manifest_sha256
    {
        return Err(StageFailure {
            manifest_validation: Some(Box::new(manifest_validation)),
            signature_verification: Some(Box::new(signature_verification)),
            errors: vec!["signature identity does not match the exact manifest bytes".to_string()],
        });
    }

    let files = verified_package_files(&package_root, &manifest, &manifest_sha256)?;
    let store = module_store_plan_for_data_root(
        request.store_root.clone(),
        Some(&manifest.id),
        Some(&manifest.version),
        &format!("developer stage {manifest_sha256}"),
    );
    let registry = load_ready_store(&store)?;
    let current_registry_bytes = fs::read(&store.registry_path).map_err(|error| {
        StageFailure::new(format!("read current module registry bytes: {error}"))
    })?;
    let registry_before_sha256 = bytes_sha256(&current_registry_bytes);
    let registry_bytes = canonical_registry_bytes(&registry).map_err(|error| {
        StageFailure::new(format!("canonical current module registry: {error}"))
    })?;
    let registry_after_sha256 = bytes_sha256(&registry_bytes);
    let destination_relative = format!("modules/{}/{}", manifest.id, manifest.version);
    let destination = Path::new(&store.data_root)
        .join("modules")
        .join(&manifest.id)
        .join(&manifest.version);
    if fs::symlink_metadata(&destination).is_ok() {
        return Err(StageFailure::new(
            "developer stage destination already exists; refusing replacement",
        ));
    }

    let stage_id = format!("module-stage-{}", &manifest_sha256[..16]);
    let plan_id = stage_id.clone();
    let transaction_id = format!("tx-module-stage-{}", &manifest_sha256[..16]);
    let stage_receipt_relative = format!("state/{STAGING_RECEIPTS_DIRECTORY}/{plan_id}.json");
    let commit_receipt_relative = format!("state/receipts/{plan_id}.json");
    let action_plan = build_action_plan(
        &plan_id,
        &stage_id,
        &manifest,
        &files,
        &stage_receipt_relative,
        &commit_receipt_relative,
    );
    let validation = validate_action_plan(&action_plan);
    if !validation.valid {
        return Err(StageFailure::new(format!(
            "developer staging action plan is invalid: {:?}",
            validation.errors
        )));
    }
    let digests = action_plan_digests(&action_plan).map_err(|errors| {
        StageFailure::new(format!("developer staging plan digest: {errors:?}"))
    })?;

    Ok(PreparedStage {
        manifest,
        manifest_validation,
        signature_verification,
        trusted_key_id: envelope.key_id,
        manifest_sha256,
        files,
        store,
        registry,
        registry_before_sha256,
        registry_after_sha256,
        action_plan,
        plan_sha256: digests.plan_sha256,
        write_set_sha256: digests.write_set_sha256,
        transaction_id,
        stage_id,
        destination_relative,
        stage_receipt_relative,
        commit_receipt_relative,
    })
}

fn build_action_plan(
    plan_id: &str,
    stage_id: &str,
    manifest: &ModuleManifest,
    files: &[StageFileContent],
    stage_receipt_relative: &str,
    commit_receipt_relative: &str,
) -> ActionPlan {
    let mut write_set = vec![
        WriteSetEntry {
            path: stage_receipt_relative.to_string(),
            kind: WriteKind::RuntimeState,
        },
        WriteSetEntry {
            path: commit_receipt_relative.to_string(),
            kind: WriteKind::RuntimeState,
        },
        WriteSetEntry {
            path: "state/installed-modules.json".to_string(),
            kind: WriteKind::RuntimeState,
        },
    ];
    write_set.extend(files.iter().map(|file| WriteSetEntry {
        path: format!(
            "modules/{}/{}/{}",
            manifest.id, manifest.version, file.metadata.path
        ),
        kind: WriteKind::ModulePayload,
    }));
    write_set.sort_by(|left, right| left.path.cmp(&right.path));
    ActionPlan {
        schema_version: rz0_action_plan::ACTION_PLAN_SCHEMA_VERSION,
        plan_id: plan_id.to_string(),
        module_id: manifest.id.clone(),
        created_at: None,
        expires_at: None,
        dry_run: true,
        writes_attempted: false,
        evidence_contract: rz0_finding_contract::FINDING_CONTRACT.to_string(),
        evidence_report_id: format!("findings:{stage_id}"),
        evidence_sha256: sha256_bytes(manifest.id.as_bytes()),
        actions: vec![PlanAction {
            action_id: format!("{stage_id}-install"),
            finding_id: stage_id.to_string(),
            kind: ActionKind::ModuleInstall,
            disposition: ActionDisposition::Planned,
            target: format!("module:{}@{}", manifest.id, manifest.version),
            source: None,
            manager: None,
            executable: None,
            executable_identity: None::<ActionExecutableIdentity>,
            arguments: Vec::new(),
            would_write: false,
            requires_confirmation: true,
            requires_elevation: false,
            network_required: false,
            risk: ActionRisk::Low,
            capabilities: vec![ActionCapability::RuntimeStateWrite],
            forbidden_path_classes: Vec::new(),
            write_set,
            rollback: RollbackPlan {
                supported: true,
                quarantine_required: false,
                description: "failed or interrupted stage bytes remain under the exact runtime.zero-owned module path for manual review; no replacement or automatic cleanup is attempted".to_string(),
            },
        }],
        warnings: vec![
            "developer-only local stage; test-key trust does not authorize production installation or module execution".to_string(),
            "the installed registry remains unchanged; staged bytes are not active or discoverable runtime modules".to_string(),
        ],
    }
}

fn build_confirmation_challenge(
    prepared: &PreparedStage,
    issued_unix_seconds: u64,
) -> Result<ConfirmationChallenge, String> {
    let mut challenge = ConfirmationChallenge {
        schema_version: rz0_confirmation_contract::CONFIRMATION_SCHEMA_VERSION,
        contract: rz0_confirmation_contract::CONFIRMATION_CHALLENGE_CONTRACT.to_string(),
        challenge_id: format!("challenge.module-stage.{}", &prepared.manifest_sha256[..16]),
        plan_id: prepared.action_plan.plan_id.clone(),
        plan_sha256: prepared.plan_sha256.clone(),
        dry_run_sha256: prepared.plan_sha256.clone(),
        write_set_sha256: prepared.write_set_sha256.clone(),
        before_state_sha256: Some(prepared.registry_before_sha256.clone()),
        expected_after_state_sha256: prepared.registry_after_sha256.clone(),
        risk: ConfirmationRisk::Mutating,
        action_count: 1,
        capabilities: vec![ActionCapability::RuntimeStateWrite],
        issued_unix_seconds,
        expires_unix_seconds: issued_unix_seconds.saturating_add(300),
        dry_run_completed: true,
        dry_run_writes_attempted: false,
        rollback_available: true,
        quarantine_available: false,
        manual_recovery_acknowledged: false,
        expected_phrase: String::new(),
        challenge_sha256: String::new(),
    };
    seal_confirmation_challenge(&mut challenge);
    Ok(challenge)
}

fn challenge_view(
    prepared: &PreparedStage,
    challenge: &ConfirmationChallenge,
) -> DeveloperStageChallenge {
    DeveloperStageChallenge {
        plan_id: challenge.plan_id.clone(),
        transaction_id: prepared.transaction_id.clone(),
        issued_unix_seconds: challenge.issued_unix_seconds,
        expires_unix_seconds: challenge.expires_unix_seconds,
        expected_phrase: challenge.expected_phrase.clone(),
        challenge_sha256: challenge.challenge_sha256.clone(),
    }
}

fn confirmation_response(
    challenge: &ConfirmationChallenge,
    phrase: &str,
    now_unix_seconds: u64,
) -> Result<ConfirmationResponse, String> {
    let response = ConfirmationResponse {
        schema_version: rz0_confirmation_contract::CONFIRMATION_SCHEMA_VERSION,
        contract: rz0_confirmation_contract::CONFIRMATION_RESPONSE_CONTRACT.to_string(),
        challenge_id: challenge.challenge_id.clone(),
        challenge_sha256: challenge.challenge_sha256.clone(),
        confirmed_unix_seconds: now_unix_seconds,
        surface: ConfirmationSurface::Cli,
        phrase: phrase.to_string(),
        interactive: true,
        single_use: true,
        execution_authorized: false,
    };
    let assessment = validate_confirmation(challenge, &response, now_unix_seconds);
    if assessment.valid {
        Ok(response)
    } else {
        Err(assessment.errors.join("; "))
    }
}

fn apply_stage(
    prepared: &PreparedStage,
    challenge: &ConfirmationChallenge,
    response: &ConfirmationResponse,
    now_unix_seconds: u64,
) -> Result<TransactionCommitReceipt, String> {
    let assessment = validate_confirmation(challenge, response, now_unix_seconds);
    if !assessment.valid {
        return Err(format!(
            "developer stage confirmation is invalid: {:?}",
            assessment.errors
        ));
    }
    if prepared.action_plan.plan_id != challenge.plan_id {
        return Err("developer stage confirmation plan identity changed".to_string());
    }

    let state_root = Path::new(&prepared.store.state_root);
    let transactions_root = state_root.join("transactions");
    let transaction_path = transactions_root.join(&prepared.transaction_id);
    if fs::symlink_metadata(&transaction_path).is_ok() {
        return Err("developer stage transaction already exists; refusing replay".to_string());
    }
    let prepared_journal = journal(
        &prepared.transaction_id,
        &prepared.action_plan.plan_id,
        TransactionOperation::ModuleInstall,
        vec![event(TransactionEventKind::Prepared)],
    );
    publish_journal_snapshot(&transactions_root, &prepared_journal)
        .map_err(|error| format!("publish module stage prepared journal: {error}"))?;

    let response_sha256 = rz0_confirmation_contract::confirmation_response_sha256(response);
    let mut consumption = ConfirmationConsumption {
        schema_version: rz0_confirmation_contract::CONFIRMATION_SCHEMA_VERSION,
        contract: rz0_confirmation_contract::CONFIRMATION_CONSUMPTION_CONTRACT.to_string(),
        transaction_id: prepared.transaction_id.clone(),
        plan_id: prepared.action_plan.plan_id.clone(),
        challenge_sha256: challenge.challenge_sha256.clone(),
        response_sha256,
        consumed_unix_seconds: now_unix_seconds,
        single_use_consumed: true,
        execution_authorized: false,
        binding_sha256: String::new(),
    };
    seal_confirmation_consumption(&mut consumption);
    publish_confirmation_consumption(
        state_root,
        &prepared_journal,
        &prepared.action_plan,
        challenge,
        response,
        &consumption,
    )
    .map_err(|error| format!("publish module stage confirmation: {error}"))?;

    let applying = append(&prepared_journal, event(TransactionEventKind::ApplyStarted));
    publish_journal_snapshot(&transactions_root, &applying)
        .map_err(|error| format!("publish module stage apply journal: {error}"))?;

    let modules_root = SecureDirectory::open(Path::new(&prepared.store.modules_root))
        .map_err(|error| format!("open private module store: {error}"))?;
    modules_root
        .verify_private()
        .map_err(|error| format!("verify private module store: {error}"))?;
    let module_id_root = modules_root
        .open_or_create_child_directory(OsStr::new(&prepared.manifest.id))
        .map_err(|error| format!("open module id directory: {error}"))?;
    let module_version_root = module_id_root
        .create_child_directory(OsStr::new(&prepared.manifest.version))
        .map_err(|error| format!("create module version staging directory: {error}"))?;
    module_version_root
        .verify_private()
        .map_err(|error| format!("verify module version directory: {error}"))?;

    let mut current = applying;
    for file in &prepared.files {
        let relative = format!(
            "modules/{}/{}/{}",
            prepared.manifest.id, prepared.manifest.version, file.metadata.path
        );
        let intent = append(
            &current,
            write_event(
                TransactionEventKind::WriteIntent,
                &format!("stage-{}", short_digest(relative.as_bytes())),
                &relative,
                None,
                Some(&file.metadata.sha256),
            ),
        );
        publish_journal_snapshot(&transactions_root, &intent)
            .map_err(|error| format!("publish module stage write intent: {error}"))?;
        if let Err(error) =
            write_relative_file(&module_version_root, &file.metadata.path, &file.bytes)
        {
            let recovery = append(&intent, event(TransactionEventKind::RecoveryRequired));
            let _ = publish_journal_snapshot(&transactions_root, &recovery);
            return Err(format!(
                "module stage write failed and requires review: {error}"
            ));
        }
        let verified =
            verify_relative_file(&module_version_root, &file.metadata.path, &file.metadata)
                .map_err(|error| format!("verify staged module file: {error}"))?;
        let verified_event = append(
            &intent,
            write_event(
                TransactionEventKind::WriteVerified,
                &format!("stage-{}", short_digest(relative.as_bytes())),
                &relative,
                None,
                Some(&verified.sha256),
            ),
        );
        publish_journal_snapshot(&transactions_root, &verified_event)
            .map_err(|error| format!("publish verified module stage write: {error}"))?;
        current = verified_event;
    }

    let stage_receipt = DeveloperStageReceipt {
        schema_version: 1,
        contract: "developer_module_stage_receipt".to_string(),
        stage_id: prepared.stage_id.clone(),
        transaction_id: prepared.transaction_id.clone(),
        plan_id: prepared.action_plan.plan_id.clone(),
        module_id: prepared.manifest.id.clone(),
        module_version: prepared.manifest.version.clone(),
        manifest_sha256: prepared.manifest_sha256.clone(),
        trusted_test_key_id: prepared.trusted_key_id.clone(),
        destination: prepared.destination_relative.clone(),
        files: prepared
            .files
            .iter()
            .map(|file| file.metadata.clone())
            .collect(),
        lifecycle_state: "staged".to_string(),
        developer_only: true,
        test_key_only: true,
        activation_authorized: false,
        invocation_authorized: false,
        product_execution_authorized: false,
        writes_attempted: true,
    };
    let mut stage_receipt_bytes = serde_json::to_vec(&stage_receipt)
        .map_err(|error| format!("serialize developer stage receipt: {error}"))?;
    stage_receipt_bytes.push(b'\n');
    let state = SecureDirectory::open(state_root)
        .map_err(|error| format!("open stage state root: {error}"))?;
    state
        .verify_private()
        .map_err(|error| format!("verify stage state root: {error}"))?;
    let staging_receipts = state
        .open_or_create_child_directory(OsStr::new(STAGING_RECEIPTS_DIRECTORY))
        .map_err(|error| format!("open staging receipt directory: {error}"))?;
    let receipt_sha256 = sha256_bytes(&stage_receipt_bytes);
    let receipt_path = prepared.stage_receipt_relative.clone();
    let receipt_intent = append(
        &current,
        write_event(
            TransactionEventKind::WriteIntent,
            &format!("stage-{}", short_digest(receipt_path.as_bytes())),
            &receipt_path,
            None,
            Some(&receipt_sha256),
        ),
    );
    publish_journal_snapshot(&transactions_root, &receipt_intent)
        .map_err(|error| format!("publish stage receipt intent: {error}"))?;
    staging_receipts
        .write_new_child(
            OsStr::new(&format!("{}.json", prepared.action_plan.plan_id)),
            &stage_receipt_bytes,
            MAX_TRUST_DOCUMENT_BYTES,
        )
        .map_err(|error| format!("publish developer stage receipt: {error}"))?;
    let receipt_verified = append(
        &receipt_intent,
        write_event(
            TransactionEventKind::WriteVerified,
            &format!("stage-{}", short_digest(receipt_path.as_bytes())),
            &receipt_path,
            None,
            Some(&receipt_sha256),
        ),
    );
    publish_journal_snapshot(&transactions_root, &receipt_verified)
        .map_err(|error| format!("publish verified stage receipt: {error}"))?;

    let committing = append(
        &receipt_verified,
        event(TransactionEventKind::CommitStarted),
    );
    publish_journal_snapshot(&transactions_root, &committing)
        .map_err(|error| format!("publish module stage commit journal: {error}"))?;
    let committed = append(&committing, event(TransactionEventKind::Committed));
    publish_journal_snapshot(&transactions_root, &committed)
        .map_err(|error| format!("publish committed module stage journal: {error}"))?;

    let head = committed
        .events
        .last()
        .ok_or_else(|| "committed module stage journal has no head".to_string())?;
    let mut commit_receipt = TransactionCommitReceipt {
        schema_version: rz0_transaction_contract::COMMIT_RECEIPT_SCHEMA_VERSION,
        contract: rz0_transaction_contract::COMMIT_RECEIPT_CONTRACT.to_string(),
        transaction_id: committed.transaction_id.clone(),
        plan_id: committed.plan_id.clone(),
        operation: committed.operation,
        committed_event_sequence: head.sequence,
        committed_event_sha256: head.event_sha256.clone(),
        journal_snapshot_name: format!("{:04}-{}.json", head.sequence, head.event_sha256),
        action_plan_sha256: prepared.plan_sha256.clone(),
        write_set_sha256: prepared.write_set_sha256.clone(),
        confirmation_challenge_sha256: challenge.challenge_sha256.clone(),
        confirmation_response_sha256: rz0_confirmation_contract::confirmation_response_sha256(
            response,
        ),
        confirmation_consumption_sha256: consumption.binding_sha256.clone(),
        confirmation_consumed: true,
        registry_before_sha256: Some(prepared.registry_before_sha256.clone()),
        registry_after_sha256: prepared.registry_after_sha256.clone(),
        publication: rz0_transaction_contract::CommitPublicationRequirements::schema_one(),
        binding_sha256: String::new(),
        automatic_mutation_authorized: false,
    };
    seal_commit_receipt(&mut commit_receipt);
    publish_committed_state(
        state_root,
        CommitCoordinatorInput {
            committed_journal: &committed,
            action_plan: &prepared.action_plan,
            challenge,
            response,
            consumption: &consumption,
            receipt: &commit_receipt,
            next_registry: &prepared.registry,
        },
    )
    .map_err(|error| format!("commit module stage evidence: {error}"))?;
    Ok(commit_receipt)
}

impl PreparedStage {
    fn report(
        &self,
        _source_manifest_path: PathBuf,
        valid: bool,
        apply: bool,
        challenge: Option<DeveloperStageChallenge>,
        errors: Vec<String>,
        mut warnings: Vec<String>,
    ) -> DeveloperStageReport {
        warnings.extend(self.action_plan.warnings.clone());
        DeveloperStageReport {
            schema_version: 1,
            contract: "developer_module_stage",
            valid,
            developer_only: true,
            test_key_only: true,
            dry_run: !apply,
            writes_attempted: apply,
            product_execution_authorized: false,
            activation_authorized: false,
            invocation_authorized: false,
            package_id: Some(self.manifest.id.clone()),
            package_version: Some(self.manifest.version.clone()),
            manifest_sha256: Some(self.manifest_sha256.clone()),
            trusted_key_id: Some(self.trusted_key_id.clone()),
            source_manifest_path: REDACTED_SOURCE_MANIFEST.to_string(),
            destination_relative: Some(self.destination_relative.clone()),
            stage_receipt_path: Some(self.stage_receipt_relative.clone()),
            commit_receipt_path: Some(self.commit_receipt_relative.clone()),
            transaction_id: Some(self.transaction_id.clone()),
            plan_id: Some(self.action_plan.plan_id.clone()),
            plan_sha256: Some(self.plan_sha256.clone()),
            write_set_sha256: Some(self.write_set_sha256.clone()),
            manifest_validation: Some(self.manifest_validation.clone()),
            signature_verification: Some(self.signature_verification.clone()),
            files: self
                .files
                .iter()
                .map(|file| file.metadata.clone())
                .collect(),
            challenge,
            errors,
            warnings,
            safety_note: STAGE_SAFETY_NOTE,
        }
    }
}

struct StageFailure {
    manifest_validation: Option<Box<ManifestValidationReport>>,
    signature_verification: Option<Box<SignatureVerification>>,
    errors: Vec<String>,
}

impl StageFailure {
    fn new(error: impl Into<String>) -> Self {
        Self {
            manifest_validation: None,
            signature_verification: None,
            errors: vec![error.into()],
        }
    }

    fn with_manifest(error: impl Into<String>, report: ManifestValidationReport) -> Self {
        Self {
            manifest_validation: Some(Box::new(report)),
            signature_verification: None,
            errors: vec![error.into()],
        }
    }

    fn into_report(self, _source_manifest_path: &Path) -> DeveloperStageReport {
        DeveloperStageReport {
            schema_version: 1,
            contract: "developer_module_stage",
            valid: false,
            developer_only: true,
            test_key_only: true,
            dry_run: true,
            writes_attempted: false,
            product_execution_authorized: false,
            activation_authorized: false,
            invocation_authorized: false,
            package_id: None,
            package_version: None,
            manifest_sha256: None,
            trusted_key_id: None,
            source_manifest_path: REDACTED_SOURCE_MANIFEST.to_string(),
            destination_relative: None,
            stage_receipt_path: None,
            commit_receipt_path: None,
            transaction_id: None,
            plan_id: None,
            plan_sha256: None,
            write_set_sha256: None,
            manifest_validation: self.manifest_validation.map(|report| *report),
            signature_verification: self.signature_verification.map(|report| *report),
            files: Vec::new(),
            challenge: None,
            errors: self.errors,
            warnings: Vec::new(),
            safety_note: STAGE_SAFETY_NOTE,
        }
    }
}

fn resolve_manifest_path(input: &Path) -> PathBuf {
    if input.is_dir() {
        input.join(MODULE_MANIFEST_FILE)
    } else {
        input.to_path_buf()
    }
}

fn verified_package_files(
    package_root: &Path,
    manifest: &ModuleManifest,
    manifest_sha256: &str,
) -> Result<Vec<StageFileContent>, StageFailure> {
    let manifest_size = fs::symlink_metadata(package_root.join(MODULE_MANIFEST_FILE))
        .map_err(|error| StageFailure::new(format!("inspect module manifest: {error}")))?
        .len();
    let mut expected = BTreeMap::new();
    expected.insert(
        MODULE_MANIFEST_FILE.to_string(),
        (manifest_sha256.to_string(), manifest_size),
    );
    if let Some(integrity) = &manifest.integrity {
        for file in &integrity.files {
            let size = file
                .size_bytes
                .or_else(|| {
                    fs::symlink_metadata(package_root.join(&file.path))
                        .ok()
                        .map(|m| m.len())
                })
                .ok_or_else(|| {
                    StageFailure::new(format!(
                        "cannot determine package file size for {}",
                        file.path
                    ))
                })?;
            expected.insert(file.path.clone(), (file.sha256.clone(), size));
        }
    }
    let mut files = Vec::new();
    for (path, (sha256, size_bytes)) in expected {
        if !rz0_validation_contract::valid_contract_relative_path(&path) {
            return Err(StageFailure::new(format!(
                "package file path is unsafe: {path}"
            )));
        }
        let expectation = ArtifactExpectation {
            sha256: sha256.clone(),
            size_bytes,
        };
        let artifact =
            open_verified_artifact(package_root, &path, &expectation).map_err(|error| {
                StageFailure::new(format!("open verified package file {path}: {error}"))
            })?;
        let bytes = read_verified_artifact(artifact, &expectation).map_err(|error| {
            StageFailure::new(format!("read verified package file {path}: {error}"))
        })?;
        files.push(StageFileContent {
            metadata: StageFile {
                path,
                sha256,
                size_bytes,
            },
            bytes,
        });
    }
    if files.is_empty() || files.len() > 128 {
        return Err(StageFailure::new(
            "developer stage package file count is outside the foundation bounds",
        ));
    }
    Ok(files)
}

fn read_verified_artifact(
    mut artifact: VerifiedArtifact,
    expectation: &ArtifactExpectation,
) -> Result<Vec<u8>, String> {
    revalidate_verified_artifact(&mut artifact).map_err(|error| error.to_string())?;
    let mut file = artifact.into_file();
    file.seek(SeekFrom::Start(0))
        .map_err(|error| error.to_string())?;
    let mut bytes = Vec::with_capacity(expectation.size_bytes as usize);
    file.take(expectation.size_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 != expectation.size_bytes || sha256_bytes(&bytes) != expectation.sha256 {
        return Err("verified package bytes changed after identity validation".to_string());
    }
    Ok(bytes)
}

fn load_ready_store(store: &ModuleStorePlan) -> Result<InstalledRegistry, StageFailure> {
    let data_root = Path::new(&store.data_root);
    let state_root = Path::new(&store.state_root);
    let modules_root = Path::new(&store.modules_root);
    for (label, path) in [
        ("data root", data_root),
        ("state root", state_root),
        ("module root", modules_root),
    ] {
        let directory = SecureDirectory::open(path)
            .map_err(|error| StageFailure::new(format!("open private {label}: {error}")))?;
        directory
            .verify_private()
            .map_err(|error| StageFailure::new(format!("verify private {label}: {error}")))?;
    }
    let marker = state_root.join(STORE_INIT_MARKER);
    let marker_bytes = fs::read(&marker)
        .map_err(|error| StageFailure::new(format!("read store init marker: {error}")))?;
    let marker_json: serde_json::Value = serde_json::from_slice(&marker_bytes)
        .map_err(|error| StageFailure::new(format!("parse store init marker: {error}")))?;
    if marker_json.get("kind").and_then(serde_json::Value::as_str)
        != Some("runtime_zero_store_init")
        || marker_json
            .get("store_schema_version")
            .and_then(serde_json::Value::as_u64)
            != Some(1)
    {
        return Err(StageFailure::new(
            "store init marker is not the exact schema-1 runtime.zero marker",
        ));
    }
    let registry_bytes = fs::read(&store.registry_path)
        .map_err(|error| StageFailure::new(format!("read installed module registry: {error}")))?;
    parse_registry_document(&registry_bytes).map_err(|error| {
        StageFailure::new(format!("installed module registry is not valid: {error}"))
    })
}

pub fn developer_staging_inventory(store: &ModuleStorePlan) -> DeveloperStagingInventory {
    let receipts_root = Path::new(&store.state_root).join(STAGING_RECEIPTS_DIRECTORY);
    let metadata = match fs::symlink_metadata(&receipts_root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return DeveloperStagingInventory {
                modules: Vec::new(),
                warnings: Vec::new(),
            };
        }
        Err(_) => {
            return DeveloperStagingInventory {
                modules: Vec::new(),
                warnings: vec!["developer_staging_receipts_unreadable"],
            };
        }
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return DeveloperStagingInventory {
            modules: Vec::new(),
            warnings: vec!["developer_staging_receipts_not_a_private_directory"],
        };
    }

    let entries = match fs::read_dir(&receipts_root) {
        Ok(entries) => entries,
        Err(_) => {
            return DeveloperStagingInventory {
                modules: Vec::new(),
                warnings: vec!["developer_staging_receipts_unreadable"],
            };
        }
    };
    let mut modules = Vec::new();
    let mut warnings = Vec::new();
    for (index, entry) in entries.enumerate() {
        if index >= MAX_STAGING_RECEIPTS {
            warnings.push("developer_staging_receipt_limit_reached");
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(_) => {
                warnings.push("developer_staging_receipt_entry_unreadable");
                continue;
            }
        };
        let path = entry.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(_) => {
                warnings.push("developer_staging_receipt_entry_unreadable");
                continue;
            }
        };
        let review_id = format!("staged-module-review-{}", index + 1);
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            modules.push(invalid_staged_module(
                review_id,
                "unknown",
                "staging_receipt_invalid",
            ));
            continue;
        }
        if metadata.len() > MAX_TRUST_DOCUMENT_BYTES {
            modules.push(invalid_staged_module(
                review_id,
                "unknown",
                "staging_receipt_oversized",
            ));
            continue;
        }
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => {
                modules.push(invalid_staged_module(
                    review_id,
                    "unknown",
                    "staging_receipt_unreadable",
                ));
                continue;
            }
        };
        let receipt = match serde_json::from_slice::<DeveloperStageReceipt>(&bytes) {
            Ok(receipt) => receipt,
            Err(_) => {
                modules.push(invalid_staged_module(
                    review_id,
                    "unknown",
                    "staging_receipt_invalid",
                ));
                continue;
            }
        };
        let expected_receipt_name = format!("{}.json", receipt.plan_id);
        let receipt_name_matches = path
            .file_name()
            .and_then(OsStr::to_str)
            .is_some_and(|name| name == expected_receipt_name);
        modules.push(review_staged_module(&receipt, store, receipt_name_matches));
    }
    DeveloperStagingInventory { modules, warnings }
}

fn review_staged_module(
    receipt: &DeveloperStageReceipt,
    store: &ModuleStorePlan,
    receipt_name_matches: bool,
) -> DeveloperStagedModuleStatus {
    let id_is_valid = rz0_validation_contract::valid_module_id(&receipt.module_id);
    let version_is_valid = rz0_validation_contract::valid_version(&receipt.module_version);
    let id = if id_is_valid {
        receipt.module_id.clone()
    } else {
        "invalid-staged-module".to_string()
    };
    let version = if version_is_valid {
        receipt.module_version.clone()
    } else {
        "unknown".to_string()
    };
    let destination_relative = if id_is_valid && version_is_valid {
        Some(format!("modules/{id}/{version}"))
    } else {
        None
    };
    let mut errors = Vec::new();
    if receipt.schema_version != 1 {
        errors.push("staging_receipt_schema_unsupported");
    }
    if receipt.contract != "developer_module_stage_receipt" {
        errors.push("staging_receipt_contract_invalid");
    }
    if !rz0_validation_contract::valid_ledger_id(&receipt.transaction_id, 96) {
        errors.push("staging_transaction_id_invalid");
    }
    if !rz0_validation_contract::valid_ledger_id(&receipt.plan_id, 96) {
        errors.push("staging_plan_id_invalid");
    }
    if !receipt_name_matches {
        errors.push("staging_receipt_filename_invalid");
    }
    if !id_is_valid {
        errors.push("staging_module_id_invalid");
    }
    if !version_is_valid {
        errors.push("staging_module_version_invalid");
    }
    if !rz0_validation_contract::valid_sha256(&receipt.manifest_sha256) {
        errors.push("staging_manifest_digest_invalid");
    }
    if receipt.lifecycle_state != "staged" {
        errors.push("staging_lifecycle_state_invalid");
    }
    if !receipt.developer_only
        || !receipt.test_key_only
        || receipt.activation_authorized
        || receipt.invocation_authorized
        || receipt.product_execution_authorized
        || !receipt.writes_attempted
    {
        errors.push("staging_receipt_authority_invalid");
    }
    if destination_relative.as_deref() != Some(receipt.destination.as_str()) {
        errors.push("staging_destination_invalid");
    }
    if receipt.files.is_empty() || receipt.files.len() > 128 {
        errors.push("staging_file_set_invalid");
    }

    let mut seen = BTreeMap::new();
    let mut manifest_file = None;
    for file in &receipt.files {
        if !rz0_validation_contract::valid_contract_relative_path(&file.path)
            || file.path.starts_with("state/")
            || file.path.starts_with("modules/")
        {
            errors.push("staging_file_path_invalid");
        }
        if seen.insert(file.path.clone(), ()).is_some() {
            errors.push("staging_file_set_duplicate");
        }
        if !rz0_validation_contract::valid_sha256(&file.sha256)
            || file.size_bytes > rz0_resource_contract::MAX_ARTIFACT_BYTES
        {
            errors.push("staging_file_identity_invalid");
        }
        if file.path == MODULE_MANIFEST_FILE {
            manifest_file = Some(file);
        }
    }
    if manifest_file.is_none() {
        errors.push("staging_manifest_file_missing");
    }

    if errors.is_empty() {
        let module_root = Path::new(&store.data_root)
            .join("modules")
            .join(&receipt.module_id)
            .join(&receipt.module_version);
        match fs::symlink_metadata(&module_root) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                errors.push("staged_module_destination_invalid");
            }
            Ok(_) => {
                for file in &receipt.files {
                    let expectation = ArtifactExpectation {
                        sha256: file.sha256.clone(),
                        size_bytes: file.size_bytes,
                    };
                    let verified = open_verified_artifact(&module_root, &file.path, &expectation)
                        .map_err(|error| error.to_string())
                        .and_then(|mut artifact| {
                            revalidate_verified_artifact(&mut artifact)
                                .map_err(|error| error.to_string())
                        });
                    if verified.is_err() {
                        errors.push("staged_module_bytes_invalid");
                        break;
                    }
                }
                let manifest_path = module_root.join(MODULE_MANIFEST_FILE);
                let validation = load_manifest_file(&manifest_path);
                match validation.manifest {
                    Some(manifest)
                        if validation.valid
                            && manifest.id == receipt.module_id
                            && manifest.version == receipt.module_version
                            && manifest.status == ModuleStatus::Installed => {}
                    _ => errors.push("staged_module_manifest_invalid"),
                }
            }
            Err(_) => errors.push("staged_module_destination_missing"),
        }
    }

    if errors.is_empty() {
        review_staging_transaction_evidence(receipt, store, &mut errors);
    }

    DeveloperStagedModuleStatus {
        id,
        version,
        state: if errors.is_empty() {
            rz0_module_lifecycle::ModuleLifecycleState::Staged
        } else {
            rz0_module_lifecycle::ModuleLifecycleState::Degraded
        },
        valid: errors.is_empty(),
        manifest_sha256: if rz0_validation_contract::valid_sha256(&receipt.manifest_sha256) {
            Some(receipt.manifest_sha256.clone())
        } else {
            None
        },
        trusted_test_key_id: if receipt.trusted_test_key_id.is_empty()
            || receipt.trusted_test_key_id.len() > 128
            || receipt.trusted_test_key_id.chars().any(char::is_control)
        {
            None
        } else {
            Some(receipt.trusted_test_key_id.clone())
        },
        destination_relative,
        errors: errors.into_iter().collect(),
    }
}

fn review_staging_transaction_evidence(
    receipt: &DeveloperStageReceipt,
    store: &ModuleStorePlan,
    errors: &mut Vec<&'static str>,
) {
    let transactions_root = Path::new(&store.state_root).join(TRANSACTIONS_DIRECTORY);
    let recovered = match inspect_journal_head(&transactions_root, &receipt.transaction_id) {
        Ok(recovered) => recovered,
        Err(_) => {
            errors.push("staging_transaction_journal_invalid");
            return;
        }
    };
    let journal = recovered.journal;
    let committed = journal.state == TransactionState::Committed
        && journal.operation == TransactionOperation::ModuleInstall
        && journal.transaction_id == receipt.transaction_id
        && journal.plan_id == receipt.plan_id
        && journal
            .events
            .last()
            .is_some_and(|event| event.kind == TransactionEventKind::Committed);
    if !committed {
        errors.push("staging_transaction_commit_state_invalid");
        return;
    }

    let commit_receipt_path = Path::new(&store.state_root)
        .join(RECEIPTS_DIRECTORY)
        .join(format!("{}.json", receipt.plan_id));
    let metadata = match fs::symlink_metadata(&commit_receipt_path) {
        Ok(metadata) => metadata,
        Err(_) => {
            errors.push("staging_commit_receipt_missing");
            return;
        }
    };
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_TRUST_DOCUMENT_BYTES
    {
        errors.push("staging_commit_receipt_invalid");
        return;
    }
    let bytes = match fs::read(&commit_receipt_path) {
        Ok(bytes) => bytes,
        Err(_) => {
            errors.push("staging_commit_receipt_unreadable");
            return;
        }
    };
    let commit_receipt = match serde_json::from_slice::<TransactionCommitReceipt>(&bytes) {
        Ok(receipt) => receipt,
        Err(_) => {
            errors.push("staging_commit_receipt_invalid");
            return;
        }
    };
    if !validate_commit_receipt(&commit_receipt, &journal).valid {
        errors.push("staging_commit_receipt_invalid");
    }
}

fn invalid_staged_module(
    id: String,
    version: &str,
    error: &'static str,
) -> DeveloperStagedModuleStatus {
    DeveloperStagedModuleStatus {
        id,
        version: version.to_string(),
        state: rz0_module_lifecycle::ModuleLifecycleState::Degraded,
        valid: false,
        manifest_sha256: None,
        trusted_test_key_id: None,
        destination_relative: None,
        errors: vec![error],
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path, label: &str) -> Result<T, StageFailure> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| StageFailure::new(format!("inspect {label}: {error}")))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(StageFailure::new(format!("{label} must be a regular file")));
    }
    if metadata.len() > MAX_TRUST_DOCUMENT_BYTES {
        return Err(StageFailure::new(format!(
            "{label} exceeds the foundation byte ceiling"
        )));
    }
    let bytes =
        fs::read(path).map_err(|error| StageFailure::new(format!("read {label}: {error}")))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| StageFailure::new(format!("parse {label}: {error}")))
}

fn write_relative_file(root: &SecureDirectory, relative: &str, bytes: &[u8]) -> Result<(), String> {
    let components = safe_components(relative)?;
    let (file_name, parents) = components
        .split_last()
        .ok_or_else(|| "staged file path is empty".to_string())?;
    let mut directory = root
        .try_clone()
        .map_err(|error| format!("clone module version directory: {error}"))?;
    for component in parents {
        directory = directory
            .open_or_create_child_directory(OsStr::new(component))
            .map_err(|error| format!("create staged package directory: {error}"))?;
    }
    directory
        .write_new_child(
            OsStr::new(file_name),
            bytes,
            rz0_resource_contract::MAX_ARTIFACT_BYTES,
        )
        .map_err(|error| format!("write staged package file: {error}"))?;
    Ok(())
}

fn verify_relative_file(
    root: &SecureDirectory,
    relative: &str,
    expected: &StageFile,
) -> Result<StageFile, String> {
    let components = safe_components(relative)?;
    let (file_name, parents) = components
        .split_last()
        .ok_or_else(|| "staged file path is empty".to_string())?;
    let mut directory = root
        .try_clone()
        .map_err(|error| format!("clone module version directory: {error}"))?;
    for component in parents {
        directory = directory
            .open_child_directory(OsStr::new(component))
            .map_err(|error| format!("open staged package directory: {error}"))?;
    }
    let bytes = directory
        .read_child(
            OsStr::new(file_name),
            rz0_resource_contract::MAX_ARTIFACT_BYTES,
        )
        .map_err(|error| format!("read staged package file: {error}"))?;
    let observed = StageFile {
        path: expected.path.clone(),
        sha256: sha256_bytes(&bytes),
        size_bytes: bytes.len() as u64,
    };
    if &observed != expected {
        return Err(
            "staged package file digest or size does not match the sealed source".to_string(),
        );
    }
    Ok(observed)
}

fn safe_components(relative: &str) -> Result<Vec<String>, String> {
    if !rz0_validation_contract::valid_contract_relative_path(relative) {
        return Err("staged package path is not a safe normalized relative path".to_string());
    }
    Path::new(relative)
        .components()
        .map(|component| match component {
            Component::Normal(value) => value
                .to_str()
                .map(str::to_string)
                .ok_or_else(|| "staged package path is not valid UTF-8".to_string()),
            _ => Err("staged package path contains an unsafe component".to_string()),
        })
        .collect()
}

fn journal(
    transaction_id: &str,
    plan_id: &str,
    operation: TransactionOperation,
    events: Vec<TransactionEvent>,
) -> TransactionJournal {
    let mut journal = TransactionJournal {
        schema_version: rz0_transaction_contract::TRANSACTION_SCHEMA_VERSION,
        contract: rz0_transaction_contract::TRANSACTION_CONTRACT.to_string(),
        transaction_id: transaction_id.to_string(),
        plan_id: plan_id.to_string(),
        operation,
        state: TransactionState::Prepared,
        durability: DurabilityRequirements::schema_one(),
        events,
    };
    seal_transaction_journal(&mut journal);
    journal
}

fn append(previous: &TransactionJournal, event: TransactionEvent) -> TransactionJournal {
    let mut next = previous.clone();
    next.events.push(event);
    next.state = match next.events.last().map(|event| event.kind) {
        Some(TransactionEventKind::Prepared) => TransactionState::Prepared,
        Some(TransactionEventKind::ApplyStarted) => TransactionState::Applying,
        Some(TransactionEventKind::CommitStarted) => TransactionState::CommitPending,
        Some(TransactionEventKind::Committed) => TransactionState::Committed,
        Some(TransactionEventKind::RecoveryRequired) => TransactionState::RecoveryRequired,
        _ => next.state,
    };
    seal_transaction_journal(&mut next);
    next
}

fn event(kind: TransactionEventKind) -> TransactionEvent {
    TransactionEvent {
        sequence: 0,
        kind,
        action_id: None,
        path: None,
        before_sha256: None,
        after_sha256: None,
        previous_event_sha256: String::new(),
        event_sha256: String::new(),
    }
}

fn write_event(
    kind: TransactionEventKind,
    action_id: &str,
    path: &str,
    before_sha256: Option<&str>,
    after_sha256: Option<&str>,
) -> TransactionEvent {
    TransactionEvent {
        sequence: 0,
        kind,
        action_id: Some(action_id.to_string()),
        path: Some(path.to_string()),
        before_sha256: before_sha256.map(str::to_string),
        after_sha256: after_sha256.map(str::to_string),
        previous_event_sha256: String::new(),
        event_sha256: String::new(),
    }
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn short_digest(bytes: &[u8]) -> String {
    sha256_bytes(bytes)[..16].to_string()
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn developer_stage_action_plan_is_mutating_but_still_dry_run_evidence() {
        let manifest = ModuleManifest::new(
            "first-party.demo",
            "Demo",
            "0.1.0",
            "runtime.zero",
            ModuleKind::FirstPartyModule,
            ModuleStatus::Installed,
            "Read-only demo.",
            &["demo"],
            &["macos"],
            crate::module_manifest::RiskLevel::ReadOnly,
            crate::module_manifest::ModuleSafety::module_contract_default(),
        );
        let files = vec![StageFileContent {
            metadata: StageFile {
                path: MODULE_MANIFEST_FILE.to_string(),
                sha256: "a".repeat(64),
                size_bytes: 1,
            },
            bytes: vec![b'x'],
        }];
        let plan = build_action_plan(
            "module-stage-demo",
            "module-stage-demo",
            &manifest,
            &files,
            "state/staging-receipts/module-stage-demo.json",
            "state/receipts/module-stage-demo.json",
        );
        let validation = validate_action_plan(&plan);
        assert!(validation.valid, "{:?}", validation.errors);
        assert!(plan.dry_run);
        assert!(!plan.writes_attempted);
        assert_eq!(plan.actions[0].kind, ActionKind::ModuleInstall);
        assert!(
            plan.actions[0]
                .write_set
                .iter()
                .any(|entry| entry.kind == WriteKind::ModulePayload)
        );
    }
}
