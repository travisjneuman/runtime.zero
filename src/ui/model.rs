use std::collections::BTreeSet;
use std::fmt;

pub const UI_MODEL_SCHEMA_VERSION: u16 = 1;
pub const MAX_UI_TEXT_CHARS: usize = 256;
pub const MAX_UI_ID_CHARS: usize = 120;
pub const MAX_UI_RECORDS_PER_CONTRIBUTION: usize = 256;
pub const MAX_UI_DETAIL_FIELDS: usize = 64;
pub const MAX_UI_ACTION_REFS: usize = 64;
pub const MAX_UI_ACTION_CAPABILITIES: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoundedText(String);

impl BoundedText {
    pub fn try_new(value: impl Into<String>) -> Result<Self, UiValidationError> {
        let value = value.into();
        if value.is_empty() {
            return Err(UiValidationError::EmptyText);
        }
        if looks_like_raw_path(&value) {
            return Err(UiValidationError::RawPath);
        }
        if value.chars().count() > MAX_UI_TEXT_CHARS {
            return Err(UiValidationError::TextTooLong {
                max: MAX_UI_TEXT_CHARS,
            });
        }
        if value.chars().any(char::is_control) {
            return Err(UiValidationError::ControlText);
        }
        Ok(Self(value))
    }

    pub fn redacted() -> Self {
        Self("[redacted evidence]".to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for BoundedText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoundedId(String);

impl BoundedId {
    pub fn try_new(value: impl Into<String>) -> Result<Self, UiValidationError> {
        let value = value.into();
        if value.is_empty() {
            return Err(UiValidationError::EmptyId);
        }
        if value.chars().count() > MAX_UI_ID_CHARS {
            return Err(UiValidationError::IdTooLong {
                max: MAX_UI_ID_CHARS,
            });
        }
        if !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".:/_-".contains(character))
        {
            return Err(UiValidationError::InvalidId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BoundedId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiValidationError {
    UnsupportedSchema { expected: u16, actual: u16 },
    EmptyText,
    TextTooLong { max: usize },
    ControlText,
    RawPath,
    EmptyId,
    IdTooLong { max: usize },
    InvalidId,
    TooManyRecords { max: usize },
    TooManyDetails { max: usize },
    TooManyActions { max: usize },
    TooManyCapabilities { max: usize },
    DuplicateRecordId(String),
    DuplicateActionId(String),
    DuplicateModuleId(String),
    InvalidRouteSet,
    MismatchedGeneration,
    EmptyModuleId,
    MismatchedModuleId(String),
    ExecutionClaim(String),
}

impl fmt::Display for UiValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedSchema { expected, actual } => {
                write!(
                    formatter,
                    "unsupported UI schema {actual}; expected {expected}"
                )
            }
            Self::EmptyText => formatter.write_str("UI text must not be empty"),
            Self::TextTooLong { max } => write!(formatter, "UI text exceeds {max} characters"),
            Self::ControlText => formatter.write_str("UI text contains terminal control text"),
            Self::RawPath => formatter.write_str("UI text contains an unredacted local path"),
            Self::EmptyId => formatter.write_str("UI identifier must not be empty"),
            Self::IdTooLong { max } => write!(formatter, "UI identifier exceeds {max} characters"),
            Self::InvalidId => formatter.write_str("UI identifier contains unsupported characters"),
            Self::TooManyRecords { max } => {
                write!(formatter, "UI contribution exceeds {max} records")
            }
            Self::TooManyDetails { max } => {
                write!(formatter, "UI detail section exceeds {max} fields")
            }
            Self::TooManyActions { max } => {
                write!(formatter, "UI contribution exceeds {max} actions")
            }
            Self::TooManyCapabilities { max } => {
                write!(formatter, "UI action review exceeds {max} capabilities")
            }
            Self::DuplicateRecordId(id) => write!(formatter, "duplicate UI record id {id}"),
            Self::DuplicateActionId(id) => write!(formatter, "duplicate UI action id {id}"),
            Self::DuplicateModuleId(id) => write!(formatter, "duplicate UI module id {id}"),
            Self::InvalidRouteSet => {
                formatter.write_str("UI model must contain each stable route exactly once")
            }
            Self::MismatchedGeneration => formatter.write_str("UI model generations must agree"),
            Self::EmptyModuleId => {
                formatter.write_str("module UI contribution must identify a module")
            }
            Self::MismatchedModuleId(id) => {
                write!(formatter, "UI record {id} is owned by another module")
            }
            Self::ExecutionClaim(id) => {
                write!(formatter, "UI action {id} cannot claim execution")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Route {
    Overview,
    Explore,
    Review,
    Activity,
    Modules,
}

impl Route {
    pub const ALL: [Self; 5] = [
        Self::Overview,
        Self::Explore,
        Self::Review,
        Self::Activity,
        Self::Modules,
    ];

    pub const fn title(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Explore => "Explore",
            Self::Review => "Review",
            Self::Activity => "Activity",
            Self::Modules => "Modules",
        }
    }

    pub const fn number(self) -> usize {
        match self {
            Self::Overview => 1,
            Self::Explore => 2,
            Self::Review => 3,
            Self::Activity => 4,
            Self::Modules => 5,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ViewState {
    Loading {
        generation: u64,
    },
    Ready {
        generation: u64,
    },
    Unavailable {
        generation: u64,
        reason: BoundedText,
    },
    Empty {
        generation: u64,
    },
    Blocked {
        generation: u64,
        reason: BoundedText,
    },
    Stale {
        generation: u64,
        reason: BoundedText,
    },
    Failed {
        generation: u64,
        reason: BoundedText,
    },
}

impl ViewState {
    pub const fn generation(&self) -> u64 {
        match self {
            Self::Loading { generation }
            | Self::Ready { generation }
            | Self::Unavailable { generation, .. }
            | Self::Empty { generation }
            | Self::Blocked { generation, .. }
            | Self::Stale { generation, .. }
            | Self::Failed { generation, .. } => *generation,
        }
    }

    pub const fn label(&self) -> &'static str {
        match self {
            Self::Loading { .. } => "loading",
            Self::Ready { .. } => "ready",
            Self::Unavailable { .. } => "unavailable",
            Self::Empty { .. } => "empty",
            Self::Blocked { .. } => "blocked",
            Self::Stale { .. } => "stale",
            Self::Failed { .. } => "failed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobState {
    Idle,
    Running {
        job_id: BoundedId,
        phase: BoundedText,
    },
    Succeeded {
        receipt: BoundedId,
        verification: BoundedId,
    },
    Cancelled {
        job_id: BoundedId,
        reason: BoundedText,
    },
    Recovery {
        transaction: BoundedId,
        decision: BoundedText,
    },
    Failed {
        job_id: BoundedId,
        reason: BoundedText,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordStatus {
    Ok,
    Info,
    Plan,
    DryRun,
    Warn,
    Blocked,
    Error,
    Observed,
    Muted,
}

impl RecordStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Ok => "[OK]",
            Self::Info => "[INFO]",
            Self::Plan => "[PLAN]",
            Self::DryRun => "[DRY-RUN]",
            Self::Warn => "[WARN]",
            Self::Blocked => "[BLOCKED]",
            Self::Error => "[ERROR]",
            Self::Observed => "[OBSERVED]",
            Self::Muted => "[SKIP]",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordKind {
    Readiness,
    Inventory,
    Provider,
    Finding,
    ActionReview,
    Activity,
    Recovery,
    Module,
    Diagnostic,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModulePosture {
    InstalledInactive,
    EnabledReadOnly,
    Staged,
    Degraded,
    Unavailable,
    Blocked,
}

impl ModulePosture {
    pub const fn label(self) -> &'static str {
        match self {
            Self::InstalledInactive => "installed-inactive",
            Self::EnabledReadOnly => "enabled-read-only",
            Self::Staged => "staged",
            Self::Degraded => "degraded",
            Self::Unavailable => "unavailable",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    Fresh,
    Stale,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionState {
    Public,
    PathRedacted,
    SensitiveOmitted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceSource {
    LocalSnapshot,
    Inventory,
    ProviderReview,
    ActionPlan,
    ModuleRegistry,
    RecoveryReview,
    SystemMonitor,
    CliContract,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRef {
    pub source: EvidenceSource,
    pub reference_id: BoundedId,
    pub freshness: Freshness,
    pub redaction: RedactionState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetailValue {
    Text(BoundedText),
    Count(usize),
    Version(BoundedText),
    Status(RecordStatus),
    Digest(BoundedText),
    Timestamp(BoundedText),
    Reference(BoundedId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetailField {
    pub label: BoundedText,
    pub value: DetailValue,
}

impl DetailField {
    pub fn text(
        label: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, UiValidationError> {
        Ok(Self {
            label: BoundedText::try_new(label)?,
            value: DetailValue::Text(BoundedText::try_new(value)?),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiDetailSection {
    pub title: BoundedText,
    pub fields: Vec<DetailField>,
}

impl UiDetailSection {
    pub fn validate(&self) -> Result<(), UiValidationError> {
        if self.fields.len() > MAX_UI_DETAIL_FIELDS {
            return Err(UiValidationError::TooManyDetails {
                max: MAX_UI_DETAIL_FIELDS,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionDisposition {
    ReadOnly,
    Reviewable,
    Blocked,
    Unavailable,
}

impl ActionDisposition {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::Reviewable => "reviewable",
            Self::Blocked => "blocked",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionReviewSummary {
    pub operation: BoundedText,
    pub target: BoundedText,
    pub authority: BoundedText,
    pub plan_id: BoundedId,
    pub plan_sha256: BoundedText,
    pub write_set_sha256: BoundedText,
    pub risk: BoundedText,
    pub requires_confirmation: bool,
    pub requires_elevation: bool,
    pub network_required: bool,
    pub capabilities: Vec<BoundedText>,
    pub rollback: BoundedText,
    pub executed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmationPrompt {
    pub action_id: BoundedId,
    pub plan_id: BoundedId,
    pub plan_sha256: BoundedText,
    pub target: BoundedText,
    pub expected_phrase: BoundedText,
    pub risk: BoundedText,
    pub expires_unix_seconds: u64,
    pub rollback_available: bool,
    pub manual_recovery_acknowledged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiActionRef {
    pub action_id: BoundedId,
    pub disposition: ActionDisposition,
    pub review: ActionReviewSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewBoundary {
    pub reference_id: BoundedId,
    pub disposition: ActionDisposition,
    pub message: BoundedText,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchTerms(pub Vec<BoundedText>);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiRecord {
    pub record_id: BoundedId,
    pub module_id: BoundedId,
    pub kind: RecordKind,
    pub title: BoundedText,
    pub summary: BoundedText,
    pub status: RecordStatus,
    pub details: Vec<UiDetailSection>,
    pub evidence: Vec<EvidenceRef>,
    pub action_refs: Vec<UiActionRef>,
    pub review_boundary: Option<ReviewBoundary>,
    pub search_terms: SearchTerms,
}

impl UiRecord {
    pub fn validate(&self) -> Result<(), UiValidationError> {
        if self.details.len() > MAX_UI_DETAIL_FIELDS {
            return Err(UiValidationError::TooManyDetails {
                max: MAX_UI_DETAIL_FIELDS,
            });
        }
        for detail in &self.details {
            detail.validate()?;
        }
        for action in &self.action_refs {
            validate_action_ref(action)?;
        }
        if self.action_refs.len() > MAX_UI_ACTION_REFS {
            return Err(UiValidationError::TooManyActions {
                max: MAX_UI_ACTION_REFS,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleUiContribution {
    pub schema_version: u16,
    pub module_id: BoundedId,
    pub display_name: BoundedText,
    pub posture: ModulePosture,
    pub records: Vec<UiRecord>,
    pub detail_sections: Vec<UiDetailSection>,
    pub action_refs: Vec<UiActionRef>,
}

impl ModuleUiContribution {
    pub fn validate(&self) -> Result<(), UiValidationError> {
        if self.schema_version != UI_MODEL_SCHEMA_VERSION {
            return Err(UiValidationError::UnsupportedSchema {
                expected: UI_MODEL_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
        if self.module_id.as_str().is_empty() {
            return Err(UiValidationError::EmptyModuleId);
        }
        if self.records.len() > MAX_UI_RECORDS_PER_CONTRIBUTION {
            return Err(UiValidationError::TooManyRecords {
                max: MAX_UI_RECORDS_PER_CONTRIBUTION,
            });
        }
        if self.action_refs.len() > MAX_UI_ACTION_REFS {
            return Err(UiValidationError::TooManyActions {
                max: MAX_UI_ACTION_REFS,
            });
        }
        for detail in &self.detail_sections {
            detail.validate()?;
        }
        let mut record_ids = BTreeSet::new();
        for record in &self.records {
            record.validate()?;
            if record.module_id != self.module_id {
                return Err(UiValidationError::MismatchedModuleId(
                    record.record_id.to_string(),
                ));
            }
            if !record_ids.insert(record.record_id.clone()) {
                return Err(UiValidationError::DuplicateRecordId(
                    record.record_id.to_string(),
                ));
            }
        }
        let mut action_ids = BTreeSet::new();
        for action in self.action_refs.iter().chain(
            self.records
                .iter()
                .flat_map(|record| record.action_refs.iter()),
        ) {
            validate_action_ref(action)?;
            if !action_ids.insert(action.action_id.clone()) {
                return Err(UiValidationError::DuplicateActionId(
                    action.action_id.to_string(),
                ));
            }
        }
        Ok(())
    }
}

fn validate_action_ref(action: &UiActionRef) -> Result<(), UiValidationError> {
    if action.review.executed {
        return Err(UiValidationError::ExecutionClaim(
            action.action_id.to_string(),
        ));
    }
    if action.review.capabilities.len() > MAX_UI_ACTION_CAPABILITIES {
        return Err(UiValidationError::TooManyCapabilities {
            max: MAX_UI_ACTION_CAPABILITIES,
        });
    }
    Ok(())
}

fn looks_like_raw_path(value: &str) -> bool {
    value.contains("/Users/")
        || value.contains("/home/")
        || value.contains("/root/")
        || value.contains("/Volumes/")
        || value.contains("/private/")
        || value.contains("/var/")
        || value.contains("/tmp/")
        || value.contains("file://")
        || value.contains("\\Users\\")
        || value.as_bytes().windows(3).any(|window| {
            window[0].is_ascii_alphabetic() && window[1] == b':' && window[2] == b'\\'
        })
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UiRegistry {
    contributions: Vec<ModuleUiContribution>,
}

impl UiRegistry {
    pub fn register(
        &mut self,
        contribution: ModuleUiContribution,
    ) -> Result<(), UiValidationError> {
        contribution.validate()?;
        if self
            .contributions
            .iter()
            .any(|existing| existing.module_id == contribution.module_id)
        {
            return Err(UiValidationError::DuplicateModuleId(
                contribution.module_id.to_string(),
            ));
        }
        self.contributions.push(contribution);
        self.contributions
            .sort_by(|left, right| left.module_id.cmp(&right.module_id));
        Ok(())
    }

    pub fn contributions(&self) -> &[ModuleUiContribution] {
        &self.contributions
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteProjection {
    pub route: Route,
    pub state: ViewState,
    pub summary: BoundedText,
    pub records: Vec<UiRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiModel {
    pub schema_version: u16,
    pub generation: u64,
    pub state: ViewState,
    pub status: BoundedText,
    pub routes: Vec<RouteProjection>,
    pub job: JobState,
}

impl UiModel {
    pub fn loading(generation: u64) -> Self {
        let state = ViewState::Loading { generation };
        Self {
            schema_version: UI_MODEL_SCHEMA_VERSION,
            generation,
            state: state.clone(),
            status: BoundedText::try_new("loading local snapshot · no action is running")
                .expect("static UI status is valid"),
            routes: Route::ALL
                .into_iter()
                .map(|route| RouteProjection {
                    route,
                    state: state.clone(),
                    summary: BoundedText::try_new("loading evidence")
                        .expect("static UI summary is valid"),
                    records: Vec::new(),
                })
                .collect(),
            job: JobState::Idle,
        }
    }

    pub fn unavailable(generation: u64, reason: impl Into<String>) -> Self {
        let reason = BoundedText::try_new(reason).unwrap_or_else(|_| BoundedText::redacted());
        let state = ViewState::Unavailable {
            generation,
            reason: reason.clone(),
        };
        Self {
            schema_version: UI_MODEL_SCHEMA_VERSION,
            generation,
            state: state.clone(),
            status: BoundedText::try_new("local evidence unavailable · refresh is explicit")
                .expect("static UI status is valid"),
            routes: Route::ALL
                .into_iter()
                .map(|route| RouteProjection {
                    route,
                    state: state.clone(),
                    summary: reason.clone(),
                    records: Vec::new(),
                })
                .collect(),
            job: JobState::Idle,
        }
    }

    pub fn route(&self, route: Route) -> &RouteProjection {
        self.routes
            .iter()
            .find(|projection| projection.route == route)
            .expect("all stable routes are present")
    }

    pub fn route_mut(&mut self, route: Route) -> &mut RouteProjection {
        self.routes
            .iter_mut()
            .find(|projection| projection.route == route)
            .expect("all stable routes are present")
    }

    pub fn validate(&self) -> Result<(), UiValidationError> {
        if self.schema_version != UI_MODEL_SCHEMA_VERSION {
            return Err(UiValidationError::UnsupportedSchema {
                expected: UI_MODEL_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
        if self.routes.len() != Route::ALL.len() {
            return Err(UiValidationError::InvalidRouteSet);
        }
        if self.state.generation() != self.generation {
            return Err(UiValidationError::MismatchedGeneration);
        }
        let mut routes = BTreeSet::new();
        let mut record_ids = BTreeSet::new();
        let mut action_ids = BTreeSet::new();
        for projection in &self.routes {
            if !routes.insert(projection.route) || projection.state.generation() != self.generation
            {
                return Err(UiValidationError::InvalidRouteSet);
            }
            for record in &projection.records {
                record.validate()?;
                if !record_ids.insert(record.record_id.clone()) {
                    return Err(UiValidationError::DuplicateRecordId(
                        record.record_id.to_string(),
                    ));
                }
                for action in &record.action_refs {
                    if !action_ids.insert(action.action_id.clone()) {
                        return Err(UiValidationError::DuplicateActionId(
                            action.action_id.to_string(),
                        ));
                    }
                }
            }
        }
        if routes.len() != Route::ALL.len()
            || !Route::ALL.iter().all(|route| routes.contains(route))
        {
            return Err(UiValidationError::InvalidRouteSet);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructors_cover_loading_and_unavailable_without_unbounded_text() {
        let loading = UiModel::loading(3);
        assert_eq!(loading.state.label(), "loading");
        assert_eq!(loading.routes.len(), Route::ALL.len());

        let unavailable = UiModel::unavailable(4, "provider snapshot unavailable");
        assert_eq!(unavailable.state.label(), "unavailable");
        assert!(unavailable.validate().is_ok());
    }

    #[test]
    fn model_validation_rejects_missing_routes_and_generation_drift() {
        let mut model = UiModel::loading(3);
        model.routes.pop();
        assert_eq!(model.validate(), Err(UiValidationError::InvalidRouteSet));

        let mut model = UiModel::loading(3);
        model.generation = 4;
        assert_eq!(
            model.validate(),
            Err(UiValidationError::MismatchedGeneration)
        );
    }

    #[test]
    fn bounded_text_rejects_common_host_path_forms() {
        for value in [
            "/home/operator/file",
            "/root/file",
            "/Volumes/Drive/file",
            "file:///etc/hosts",
        ] {
            assert!(matches!(
                BoundedText::try_new(value),
                Err(UiValidationError::RawPath)
            ));
        }
    }

    #[test]
    fn all_view_and_job_states_are_explicitly_named() {
        let generation = 9;
        let states = [
            ViewState::Loading { generation },
            ViewState::Ready { generation },
            ViewState::Unavailable {
                generation,
                reason: BoundedText::redacted(),
            },
            ViewState::Empty { generation },
            ViewState::Blocked {
                generation,
                reason: BoundedText::redacted(),
            },
            ViewState::Stale {
                generation,
                reason: BoundedText::redacted(),
            },
            ViewState::Failed {
                generation,
                reason: BoundedText::redacted(),
            },
        ];
        assert_eq!(
            states.iter().map(ViewState::label).collect::<Vec<_>>(),
            [
                "loading",
                "ready",
                "unavailable",
                "empty",
                "blocked",
                "stale",
                "failed"
            ]
        );

        let job_states = [
            JobState::Idle,
            JobState::Running {
                job_id: BoundedId::try_new("job/1").expect("id"),
                phase: BoundedText::try_new("reading").expect("text"),
            },
            JobState::Succeeded {
                receipt: BoundedId::try_new("receipt/1").expect("id"),
                verification: BoundedId::try_new("verify/1").expect("id"),
            },
            JobState::Cancelled {
                job_id: BoundedId::try_new("job/1").expect("id"),
                reason: BoundedText::try_new("user requested").expect("text"),
            },
            JobState::Recovery {
                transaction: BoundedId::try_new("transaction/1").expect("id"),
                decision: BoundedText::try_new("review required").expect("text"),
            },
            JobState::Failed {
                job_id: BoundedId::try_new("job/1").expect("id"),
                reason: BoundedText::try_new("foundation rejected request").expect("text"),
            },
        ];
        assert_eq!(job_states.len(), 6);
    }
}
