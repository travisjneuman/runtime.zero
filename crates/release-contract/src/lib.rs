use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

pub const RELEASE_SCHEMA_VERSION: u16 = 1;
pub const RELEASE_CONTRACT: &str = "release_acceptance_assessment";
pub const MAX_RELEASE_TARGETS: usize = 256;
pub const LIFECYCLE_STAGES_PER_CELL: usize = 12;
pub const MODULE_FAMILY_COUNT: usize = 7;
pub const ACCEPTANCE_CELLS_PER_TARGET: usize = LIFECYCLE_STAGES_PER_CELL * MODULE_FAMILY_COUNT;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseAcceptanceAssessment {
    pub schema_version: u16,
    pub contract: String,
    pub assessment_id: String,
    pub scope_revision: String,
    pub decision: ReleaseDecision,
    pub release_authorized: bool,
    pub targets: Vec<ReleaseTarget>,
    pub cells: Vec<AcceptanceCell>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseDecision {
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReleaseTarget {
    pub target_id: String,
    pub platform: PlatformFamily,
    pub generation: String,
    pub variant: String,
    pub architecture: Architecture,
    pub artifact: ArtifactKind,
    pub tier: SupportTier,
    pub vendor_supported: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlatformFamily {
    WindowsClient,
    WindowsServer,
    Macos,
    Ubuntu,
    Debian,
    Rhel,
    Arch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Architecture {
    X86,
    X86_64,
    Arm64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    PortableZip,
    Dmg,
    Pkg,
    Deb,
    Rpm,
    ArchPackage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportTier {
    ReleaseBlocking,
    LegacyCompatibility,
    Research,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AcceptanceCell {
    pub acceptance_id: String,
    pub target_id: String,
    pub module: ModuleFamily,
    pub stage: LifecycleStage,
    pub status: AcceptanceStatus,
    pub mechanism: Option<String>,
    pub evidence_reference: Option<String>,
    pub rationale: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleFamily {
    InventoryEnvironment,
    Updater,
    Uninstall,
    Leftovers,
    CacheManagement,
    SecurityIntegrity,
    ReportExport,
}

pub const ALL_MODULE_FAMILIES: [ModuleFamily; MODULE_FAMILY_COUNT] = [
    ModuleFamily::InventoryEnvironment,
    ModuleFamily::Updater,
    ModuleFamily::Uninstall,
    ModuleFamily::Leftovers,
    ModuleFamily::CacheManagement,
    ModuleFamily::SecurityIntegrity,
    ModuleFamily::ReportExport,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleStage {
    RequirementsThreatPrivacy,
    AdversarialFixtures,
    BoundedDiscoveryNormalization,
    FindingActionEvidence,
    TextJsonTuiReview,
    CapabilityIsolation,
    Confirmation,
    TransactionRecovery,
    PostActionVerification,
    LifecycleMigrationRepair,
    TestPerformanceSoak,
    SecurityAccessibilityReleaseReview,
}

pub const ALL_LIFECYCLE_STAGES: [LifecycleStage; LIFECYCLE_STAGES_PER_CELL] = [
    LifecycleStage::RequirementsThreatPrivacy,
    LifecycleStage::AdversarialFixtures,
    LifecycleStage::BoundedDiscoveryNormalization,
    LifecycleStage::FindingActionEvidence,
    LifecycleStage::TextJsonTuiReview,
    LifecycleStage::CapabilityIsolation,
    LifecycleStage::Confirmation,
    LifecycleStage::TransactionRecovery,
    LifecycleStage::PostActionVerification,
    LifecycleStage::LifecycleMigrationRepair,
    LifecycleStage::TestPerformanceSoak,
    LifecycleStage::SecurityAccessibilityReleaseReview,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceStatus {
    Missing,
    Proven,
    NotApplicable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseValidation {
    pub valid: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AcceptanceSummary {
    pub targets: usize,
    pub cells: usize,
    pub missing: usize,
    pub proven: usize,
    pub not_applicable: usize,
}

pub fn missing_cells_for_targets(targets: &[ReleaseTarget]) -> Vec<AcceptanceCell> {
    let capacity = targets.len().saturating_mul(ACCEPTANCE_CELLS_PER_TARGET);
    let mut cells = Vec::with_capacity(capacity);
    for target in targets {
        for module in ALL_MODULE_FAMILIES {
            for stage in ALL_LIFECYCLE_STAGES {
                cells.push(AcceptanceCell {
                    acceptance_id: acceptance_id(&target.target_id, module, stage),
                    target_id: target.target_id.clone(),
                    module,
                    stage,
                    status: AcceptanceStatus::Missing,
                    mechanism: None,
                    evidence_reference: None,
                    rationale: None,
                });
            }
        }
    }
    cells
}

pub fn summarize_acceptance(assessment: &ReleaseAcceptanceAssessment) -> AcceptanceSummary {
    let mut summary = AcceptanceSummary {
        targets: assessment.targets.len(),
        cells: assessment.cells.len(),
        missing: 0,
        proven: 0,
        not_applicable: 0,
    };
    for cell in &assessment.cells {
        match cell.status {
            AcceptanceStatus::Missing => summary.missing += 1,
            AcceptanceStatus::Proven => summary.proven += 1,
            AcceptanceStatus::NotApplicable => summary.not_applicable += 1,
        }
    }
    summary
}

pub fn validate_release_assessment(assessment: &ReleaseAcceptanceAssessment) -> ReleaseValidation {
    let mut errors = Vec::new();
    if assessment.schema_version != RELEASE_SCHEMA_VERSION {
        errors.push(format!("schema_version must be {RELEASE_SCHEMA_VERSION}"));
    }
    if assessment.contract != RELEASE_CONTRACT {
        errors.push(format!("contract must be {RELEASE_CONTRACT}"));
    }
    validate_id(&assessment.assessment_id, "assessment_id", &mut errors);
    validate_id(&assessment.scope_revision, "scope_revision", &mut errors);
    if assessment.decision != ReleaseDecision::Blocked || assessment.release_authorized {
        errors
            .push("schema-1 release assessments must remain blocked and unauthorized".to_string());
    }
    validate_targets(&assessment.targets, &mut errors);
    validate_cells(assessment, &mut errors);
    errors.sort();
    errors.dedup();
    ReleaseValidation {
        valid: errors.is_empty(),
        errors,
    }
}

fn validate_targets(targets: &[ReleaseTarget], errors: &mut Vec<String>) {
    if targets.is_empty() || targets.len() > MAX_RELEASE_TARGETS {
        errors.push(format!(
            "targets must contain 1..={MAX_RELEASE_TARGETS} entries"
        ));
    }
    let mut ids = BTreeSet::new();
    for target in targets.iter().take(MAX_RELEASE_TARGETS + 1) {
        validate_id(&target.target_id, "target_id", errors);
        if target.target_id.len() > 80 {
            errors.push("target_id exceeds 80 bytes".to_string());
        }
        validate_detail(&target.generation, "generation", 80, errors);
        validate_detail(&target.variant, "variant", 120, errors);
        if !ids.insert(target.target_id.as_str()) {
            errors.push("target IDs must be unique".to_string());
        }
        if target.tier == SupportTier::ReleaseBlocking && !target.vendor_supported {
            errors.push(
                "vendor-retired targets cannot be release-blocking supported systems".to_string(),
            );
        }
    }
    if targets
        .windows(2)
        .any(|pair| pair[0].target_id >= pair[1].target_id)
    {
        errors.push("targets must be sorted by target_id".to_string());
    }
}

fn validate_cells(assessment: &ReleaseAcceptanceAssessment, errors: &mut Vec<String>) {
    let expected_len = assessment
        .targets
        .len()
        .checked_mul(ACCEPTANCE_CELLS_PER_TARGET);
    if expected_len != Some(assessment.cells.len()) {
        errors.push("cells must contain the exact target x module x lifecycle matrix".to_string());
    }
    let mut index = 0usize;
    for target in &assessment.targets {
        for module in ALL_MODULE_FAMILIES {
            for stage in ALL_LIFECYCLE_STAGES {
                let Some(cell) = assessment.cells.get(index) else {
                    return;
                };
                let expected_id = acceptance_id(&target.target_id, module, stage);
                if cell.acceptance_id != expected_id
                    || cell.target_id != target.target_id
                    || cell.module != module
                    || cell.stage != stage
                {
                    errors.push("cells are not the exact canonical acceptance matrix".to_string());
                }
                validate_cell_evidence(cell, errors);
                index += 1;
            }
        }
    }
}

fn validate_cell_evidence(cell: &AcceptanceCell, errors: &mut Vec<String>) {
    if !valid_id(&cell.acceptance_id) {
        errors.push("acceptance_id is invalid".to_string());
    }
    for value in [cell.mechanism.as_deref(), cell.rationale.as_deref()]
        .into_iter()
        .flatten()
    {
        if !valid_ascii_detail(value, 240) {
            errors.push("cell mechanism or rationale is invalid".to_string());
        }
    }
    if cell
        .evidence_reference
        .as_deref()
        .is_some_and(|value| !valid_reference(value))
    {
        errors.push("cell evidence_reference is invalid".to_string());
    }
    match cell.status {
        AcceptanceStatus::Missing => {
            if cell.mechanism.is_some()
                || cell.evidence_reference.is_some()
                || cell.rationale.is_some()
            {
                errors.push("missing cells cannot claim evidence or rationale".to_string());
            }
        }
        AcceptanceStatus::Proven => {
            if cell.mechanism.is_none()
                || cell.evidence_reference.is_none()
                || cell.rationale.is_some()
            {
                errors.push("proven cells require mechanism/evidence and no rationale".to_string());
            }
        }
        AcceptanceStatus::NotApplicable => {
            if cell.mechanism.is_some()
                || cell.evidence_reference.is_none()
                || cell.rationale.is_none()
            {
                errors.push("not-applicable cells require rationale/evidence only".to_string());
            }
        }
    }
}

fn acceptance_id(target_id: &str, module: ModuleFamily, stage: LifecycleStage) -> String {
    format!("{target_id}.{}.{}", module_name(module), stage_name(stage))
}

fn module_name(module: ModuleFamily) -> &'static str {
    match module {
        ModuleFamily::InventoryEnvironment => "inventory_environment",
        ModuleFamily::Updater => "updater",
        ModuleFamily::Uninstall => "uninstall",
        ModuleFamily::Leftovers => "leftovers",
        ModuleFamily::CacheManagement => "cache_management",
        ModuleFamily::SecurityIntegrity => "security_integrity",
        ModuleFamily::ReportExport => "report_export",
    }
}

fn stage_name(stage: LifecycleStage) -> &'static str {
    match stage {
        LifecycleStage::RequirementsThreatPrivacy => "requirements_threat_privacy",
        LifecycleStage::AdversarialFixtures => "adversarial_fixtures",
        LifecycleStage::BoundedDiscoveryNormalization => "bounded_discovery_normalization",
        LifecycleStage::FindingActionEvidence => "finding_action_evidence",
        LifecycleStage::TextJsonTuiReview => "text_json_tui_review",
        LifecycleStage::CapabilityIsolation => "capability_isolation",
        LifecycleStage::Confirmation => "confirmation",
        LifecycleStage::TransactionRecovery => "transaction_recovery",
        LifecycleStage::PostActionVerification => "post_action_verification",
        LifecycleStage::LifecycleMigrationRepair => "lifecycle_migration_repair",
        LifecycleStage::TestPerformanceSoak => "test_performance_soak",
        LifecycleStage::SecurityAccessibilityReleaseReview => {
            "security_accessibility_release_review"
        }
    }
}

fn validate_id(value: &str, field: &str, errors: &mut Vec<String>) {
    if !valid_id(value) {
        errors.push(format!("{field} is invalid"));
    }
}

fn valid_id(value: &str) -> bool {
    rz0_validation_contract::valid_ledger_id(value, 180)
}

fn validate_detail(value: &str, field: &str, maximum: usize, errors: &mut Vec<String>) {
    if !valid_ascii_detail(value, maximum) {
        errors.push(format!("{field} is invalid"));
    }
}

fn valid_ascii_detail(value: &str, maximum: usize) -> bool {
    rz0_validation_contract::valid_ascii_text(value, maximum)
}

fn valid_reference(value: &str) -> bool {
    rz0_validation_contract::valid_ledger_id(value, 120)
}
