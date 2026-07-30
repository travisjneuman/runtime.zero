pub use rz0_capability_contract::Capability as ActionCapability;
use serde::{Deserialize, Serialize};

pub const ACTION_PLAN_SCHEMA_VERSION: u16 = 1;
pub const MAX_ACTIONS: usize = 128;
pub const MAX_ARGUMENTS: usize = 64;
pub const MAX_WRITE_SET: usize = 256;
pub const MAX_ACTION_SOURCE_BYTES: u64 = rz0_resource_contract::MAX_ARTIFACT_BYTES;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionPlan {
    pub schema_version: u16,
    pub plan_id: String,
    pub module_id: String,
    pub created_at: Option<String>,
    pub expires_at: Option<String>,
    pub dry_run: bool,
    pub writes_attempted: bool,
    pub evidence_contract: String,
    pub evidence_report_id: String,
    pub evidence_sha256: String,
    pub actions: Vec<PlanAction>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanAction {
    pub action_id: String,
    pub finding_id: String,
    pub kind: ActionKind,
    pub disposition: ActionDisposition,
    pub target: String,
    pub source: Option<ActionSource>,
    pub manager: Option<String>,
    pub executable: Option<String>,
    pub arguments: Vec<String>,
    pub would_write: bool,
    pub requires_confirmation: bool,
    pub requires_elevation: bool,
    pub network_required: bool,
    pub risk: ActionRisk,
    pub capabilities: Vec<ActionCapability>,
    pub forbidden_path_classes: Vec<ForbiddenPathClass>,
    pub write_set: Vec<WriteSetEntry>,
    pub rollback: RollbackPlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Update,
    Uninstall,
    Quarantine,
    Restore,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionDisposition {
    Planned,
    Blocked,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionRisk {
    Low,
    Medium,
    High,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ForbiddenPathClass {
    Credentials,
    BrowserProfiles,
    OauthSessions,
    ProjectWorkspaces,
    Backups,
    UnknownUserData,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionSource {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WriteSetEntry {
    pub path: String,
    pub kind: WriteKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteKind {
    RuntimeState,
    QuarantineRecord,
    QuarantinedPayload,
    RestoredPayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RollbackPlan {
    pub supported: bool,
    pub quarantine_required: bool,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ActionPlanValidation {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ActionPlanValidation {
    pub(crate) fn fail(&mut self, error: impl Into<String>) {
        self.valid = false;
        self.errors.push(error.into());
    }
}
