use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::model::SignatureVerification;
use crate::staging_path::validate_relative_path;

pub const STAGING_SCHEMA_VERSION: u16 = 1;
pub const STAGING_CONTRACT: &str = "module_staging_plan";
pub const MAX_STAGING_FILES: usize = 128;
pub const MAX_STAGING_FILE_BYTES: u64 = rz0_resource_contract::MAX_ARTIFACT_BYTES;
pub const MAX_STAGING_TOTAL_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StagingPlan {
    pub schema_version: u16,
    pub contract: String,
    pub transaction_id: String,
    pub package_id: String,
    pub package_version: String,
    pub manifest_sha256: String,
    pub signature_proof: StagingSignatureProof,
    pub simulation_only: bool,
    pub dry_run: bool,
    pub writes_attempted: bool,
    pub root_class: StagingRootClass,
    pub source_root: String,
    pub staging_root: String,
    pub publication_root: String,
    pub files: Vec<StagingFile>,
    pub atomic_publish: bool,
    pub rollback: StagingRollback,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StagingSignatureProof {
    pub verified: bool,
    pub test_key_only: bool,
    pub key_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StagingRootClass {
    TemporaryFixture,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StagingFile {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub role: StagingFileRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StagingFileRole {
    Manifest,
    Payload,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StagingRollback {
    pub supported: bool,
    pub unpublished_stage_only: bool,
    pub preserve_failed_stage_for_review: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StagingPlanValidation {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn validate_staging_plan(plan: &StagingPlan) -> StagingPlanValidation {
    let mut errors = Vec::new();
    if plan.schema_version != STAGING_SCHEMA_VERSION {
        errors.push(format!(
            "staging schema_version must be {STAGING_SCHEMA_VERSION}"
        ));
    }
    if plan.contract != STAGING_CONTRACT {
        errors.push(format!("staging contract must be {STAGING_CONTRACT}"));
    }
    validate_id(&plan.transaction_id, "transaction_id", &mut errors);
    validate_id(&plan.package_id, "package_id", &mut errors);
    if !plan.package_id.starts_with("first-party.") {
        errors.push("staging simulation supports first-party package IDs only".to_string());
    }
    validate_version(&plan.package_version, &mut errors);
    validate_sha256(&plan.manifest_sha256, "manifest_sha256", &mut errors);
    validate_signature_proof(plan, &mut errors);
    if !plan.simulation_only || !plan.dry_run || plan.writes_attempted {
        errors.push(
            "staging plans must be simulation_only dry-runs with writes_attempted false"
                .to_string(),
        );
    }
    if !plan.atomic_publish {
        errors.push("staging plan must require atomic publication".to_string());
    }
    if !plan.rollback.supported
        || !plan.rollback.unpublished_stage_only
        || !plan.rollback.preserve_failed_stage_for_review
    {
        errors.push(
            "staging rollback must preserve failed stages and be limited to unpublished bytes"
                .to_string(),
        );
    }
    validate_root(&plan.source_root, "source_root", "input/", &mut errors);
    validate_root(&plan.staging_root, "staging_root", "staging/", &mut errors);
    validate_root(
        &plan.publication_root,
        "publication_root",
        "published/",
        &mut errors,
    );
    if plan.source_root == plan.staging_root
        || plan.source_root == plan.publication_root
        || plan.staging_root == plan.publication_root
    {
        errors.push("source, staging, and publication roots must be distinct".to_string());
    }
    if plan.staging_root != format!("staging/{}", plan.transaction_id) {
        errors.push("staging_root must be bound to transaction_id".to_string());
    }
    if plan.publication_root != format!("published/{}/{}", plan.package_id, plan.package_version) {
        errors.push("publication_root must be bound to package identity and version".to_string());
    }
    validate_files(plan, &mut errors);

    finish_validation(errors)
}

pub fn validate_staging_plan_with_signature(
    plan: &StagingPlan,
    verification: &SignatureVerification,
) -> StagingPlanValidation {
    let mut validation = validate_staging_plan(plan);
    if !verification.verified || !verification.test_key_only || !verification.errors.is_empty() {
        validation
            .errors
            .push("staging plan requires a successful test-key signature verification".to_string());
    }
    if verification.key_id != plan.signature_proof.key_id
        || verification.package_id != plan.package_id
        || verification.package_version != plan.package_version
        || verification.manifest_sha256 != plan.manifest_sha256
    {
        validation.errors.push(
            "staging plan identity or digest does not match the signature verification".to_string(),
        );
    }
    validation.valid = validation.errors.is_empty();
    validation
}

fn finish_validation(errors: Vec<String>) -> StagingPlanValidation {
    StagingPlanValidation {
        valid: errors.is_empty(),
        errors,
        warnings: vec![
            "fixture contract only; validation does not stage, publish, install, or execute bytes"
                .to_string(),
        ],
    }
}

fn validate_signature_proof(plan: &StagingPlan, errors: &mut Vec<String>) {
    if !plan.signature_proof.verified || !plan.signature_proof.test_key_only {
        errors.push("staging simulation requires a verified test-key signature proof".to_string());
    }
    validate_id(
        &plan.signature_proof.key_id,
        "signature_proof.key_id",
        errors,
    );
}

fn validate_files(plan: &StagingPlan, errors: &mut Vec<String>) {
    if plan.files.is_empty() || plan.files.len() > MAX_STAGING_FILES {
        errors.push(format!(
            "staging files must contain between 1 and {MAX_STAGING_FILES} entries"
        ));
    }
    let mut paths = BTreeSet::new();
    let mut manifest_count = 0usize;
    let mut total_bytes = 0u64;
    for file in plan.files.iter().take(MAX_STAGING_FILES) {
        if validate_relative_path(&file.path).is_err() {
            errors.push(format!("invalid staging file path '{}'", file.path));
        }
        if !paths.insert(file.path.clone()) {
            errors.push(format!("duplicate staging file path '{}'", file.path));
        }
        validate_sha256(&file.sha256, "staging file sha256", errors);
        if file.size_bytes > MAX_STAGING_FILE_BYTES {
            errors.push(format!(
                "staging file '{}' exceeds {MAX_STAGING_FILE_BYTES} bytes",
                file.path
            ));
        }
        total_bytes = total_bytes.saturating_add(file.size_bytes);
        if file.role == StagingFileRole::Manifest {
            manifest_count = manifest_count.saturating_add(1);
            if file.path != "rz0-module.json" || file.sha256 != plan.manifest_sha256 {
                errors.push(
                    "manifest staging entry must be rz0-module.json and match manifest_sha256"
                        .to_string(),
                );
            }
        }
    }
    if total_bytes > MAX_STAGING_TOTAL_BYTES {
        errors.push(format!(
            "staging files exceed {MAX_STAGING_TOTAL_BYTES} total bytes"
        ));
    }
    if manifest_count != 1 {
        errors.push("staging files must contain exactly one manifest entry".to_string());
    }
}

fn validate_root(value: &str, field: &str, prefix: &str, errors: &mut Vec<String>) {
    if !value.starts_with(prefix) || validate_relative_path(value).is_err() {
        errors.push(format!(
            "{field} must be a safe temporary-fixture path under {prefix}"
        ));
    }
}

fn validate_id(value: &str, field: &str, errors: &mut Vec<String>) {
    let valid = !value.is_empty()
        && value.len() <= 100
        && !value.starts_with(['.', '-'])
        && !value.ends_with(['.', '-'])
        && !value.contains("..")
        && value.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '.' | '-')
        });
    if !valid {
        errors.push(format!(
            "{field} must use bounded lowercase letters, digits, dots, or hyphens"
        ));
    }
}

fn validate_version(value: &str, errors: &mut Vec<String>) {
    if value.is_empty()
        || value.len() > 40
        || !value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '+' | '-')
        })
    {
        errors.push("package_version must use bounded ASCII version characters".to_string());
    }
}

fn validate_sha256(value: &str, field: &str, errors: &mut Vec<String>) {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        errors.push(format!(
            "{field} must be 64 lowercase hexadecimal characters"
        ));
    }
}
