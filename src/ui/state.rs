use std::collections::BTreeSet;

use super::messages::{UiEvent, UiIntent};
use super::model::{
    ActionDisposition, BoundedId, BoundedText, ConfirmationPrompt, JobState, RecordStatus, Route,
    UiModel, UiRecord, ViewState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiPage {
    Home,
    Inventory,
    Evidence,
    Review,
    Confirmation,
    Activity,
}

impl UiPage {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Inventory => "inventory",
            Self::Evidence => "evidence",
            Self::Review => "plan review",
            Self::Confirmation => "confirmation",
            Self::Activity => "activity",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowseScope {
    Home,
    Inventory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusRegion {
    Queue,
    Detail,
    Footer,
}

impl FocusRegion {
    pub const ALL: [Self; 3] = [Self::Queue, Self::Detail, Self::Footer];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Queue => "task queue",
            Self::Detail => "detail",
            Self::Footer => "controls",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Overlay {
    None,
    Help,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordLocator {
    pub route: Route,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiState {
    pub page: UiPage,
    pub browse: BrowseScope,
    pub focus: FocusRegion,
    pub selected: usize,
    pub overlay: Overlay,
    pub search_query: String,
    pub search_active: bool,
    pub model: UiModel,
    pub job: JobState,
    pub confirmation: Option<ConfirmationPrompt>,
    pub confirmation_input: String,
}

impl UiState {
    pub fn new(model: UiModel) -> Self {
        Self {
            page: UiPage::Home,
            browse: BrowseScope::Home,
            focus: FocusRegion::Queue,
            selected: 0,
            overlay: Overlay::None,
            search_query: String::new(),
            search_active: false,
            job: model.job.clone(),
            model,
            confirmation: None,
            confirmation_input: String::new(),
        }
    }

    pub fn current_records(&self) -> Vec<RecordLocator> {
        let mut records = match self.page {
            UiPage::Home => self.home_records(),
            UiPage::Inventory | UiPage::Evidence | UiPage::Review | UiPage::Confirmation => {
                self.inventory_records()
            }
            UiPage::Activity => self
                .model
                .route(Route::Activity)
                .records
                .iter()
                .enumerate()
                .map(|(index, _)| RecordLocator {
                    route: Route::Activity,
                    index,
                })
                .collect(),
        };
        let query = self.search_query.to_ascii_lowercase();
        if !query.is_empty() {
            records.retain(|locator| {
                self.record(locator).is_some_and(|record| {
                    record.title.as_str().to_ascii_lowercase().contains(&query)
                        || record
                            .summary
                            .as_str()
                            .to_ascii_lowercase()
                            .contains(&query)
                        || record
                            .search_terms
                            .0
                            .iter()
                            .any(|term| term.as_str().to_ascii_lowercase().contains(&query))
                })
            });
        }
        records
    }

    pub fn selected_locator(&self) -> Option<RecordLocator> {
        self.current_records().get(self.selected).copied()
    }

    pub fn selected_record(&self) -> Option<&UiRecord> {
        self.selected_locator()
            .and_then(|locator| self.record(&locator))
    }

    pub fn selected_record_id(&self) -> Option<BoundedId> {
        self.selected_record()
            .map(|record| record.record_id.clone())
    }

    pub fn apply(&mut self, intent: UiIntent) -> Option<UiIntent> {
        if self.search_active {
            return self.apply_search(intent);
        }
        match intent {
            UiIntent::Quit => Some(UiIntent::Quit),
            UiIntent::OpenHome => {
                self.open_browse(BrowseScope::Home);
                None
            }
            UiIntent::OpenInventory => {
                self.open_browse(BrowseScope::Inventory);
                None
            }
            UiIntent::OpenActivity => {
                self.page = UiPage::Activity;
                self.focus = FocusRegion::Detail;
                self.overlay = Overlay::None;
                None
            }
            UiIntent::FocusNext => {
                self.focus = next_focus(self.focus);
                None
            }
            UiIntent::FocusPrevious => {
                self.focus = previous_focus(self.focus);
                None
            }
            UiIntent::SelectNext => {
                self.move_selection(1);
                None
            }
            UiIntent::SelectPrevious => {
                self.move_selection(-1);
                None
            }
            UiIntent::SelectFirst => {
                self.selected = 0;
                None
            }
            UiIntent::SelectLast => {
                self.selected = self.current_records().len().saturating_sub(1);
                None
            }
            UiIntent::SelectIndex(index) => {
                self.selected = index.min(self.current_records().len().saturating_sub(1));
                self.focus = FocusRegion::Queue;
                None
            }
            UiIntent::OpenSelected => {
                match self.page {
                    UiPage::Home | UiPage::Inventory => {
                        if self.selected_record().is_some() {
                            self.page = UiPage::Evidence;
                            self.focus = FocusRegion::Detail;
                        }
                    }
                    UiPage::Evidence => {
                        self.open_review_if_available();
                    }
                    UiPage::Review => {}
                    UiPage::Confirmation | UiPage::Activity => {}
                }
                None
            }
            UiIntent::OpenReview => {
                self.open_review_if_available();
                None
            }
            UiIntent::LoadProviderReview => {
                self.page = UiPage::Activity;
                self.focus = FocusRegion::Detail;
                Some(UiIntent::LoadProviderReview)
            }
            UiIntent::BeginConfirmation => {
                let action_id = self.selected_record().and_then(|record| {
                    record
                        .action_refs
                        .iter()
                        .find(|action| action.disposition == ActionDisposition::Reviewable)
                        .map(|action| action.action_id.clone())
                });
                if let Some(action_id) = action_id {
                    self.page = UiPage::Activity;
                    self.focus = FocusRegion::Detail;
                    return Some(UiIntent::PrepareAction(action_id));
                }
                None
            }
            UiIntent::PrepareAction(_) => None,
            UiIntent::ConfirmationCharacter(character) => {
                if !character.is_control() && self.confirmation_input.chars().count() < 256 {
                    self.confirmation_input.push(character);
                }
                None
            }
            UiIntent::ConfirmationBackspace => {
                self.confirmation_input.pop();
                None
            }
            UiIntent::SubmitConfirmation => Some(UiIntent::SubmitConfirmation),
            UiIntent::CancelConfirmation => {
                self.clear_confirmation();
                self.page = UiPage::Review;
                Some(UiIntent::CancelConfirmation)
            }
            UiIntent::ToggleHelp => {
                self.overlay = if self.overlay == Overlay::Help {
                    Overlay::None
                } else {
                    Overlay::Help
                };
                None
            }
            UiIntent::Back => self.back(),
            UiIntent::Refresh => Some(UiIntent::Refresh),
            UiIntent::BeginSearch => {
                self.search_active = true;
                self.overlay = Overlay::Search;
                None
            }
            UiIntent::SearchCharacter(_) | UiIntent::SearchBackspace | UiIntent::AcceptSearch => {
                None
            }
            UiIntent::CancelJob => Some(UiIntent::CancelJob),
        }
    }

    pub fn apply_model(&mut self, model: UiModel) {
        if model.generation < self.model.generation {
            return;
        }
        self.model = model;
        self.job = self.model.job.clone();
        self.selected = self
            .selected
            .min(self.current_records().len().saturating_sub(1));
    }

    pub fn mark_snapshot_unavailable(&mut self, generation: u64, reason: BoundedText) {
        if generation < self.model.generation {
            return;
        }
        self.model = UiModel::unavailable(generation, reason.as_str());
        self.page = UiPage::Home;
        self.browse = BrowseScope::Home;
        self.focus = FocusRegion::Queue;
        self.selected = 0;
    }

    pub fn apply_event(&mut self, event: UiEvent) -> Option<UiIntent> {
        match event {
            UiEvent::Input(intent) => self.apply(intent),
            UiEvent::Resize { .. } => None,
            UiEvent::SnapshotReady { generation, model } => {
                if generation >= self.model.generation {
                    self.apply_model(model);
                }
                None
            }
            UiEvent::SnapshotUnavailable { generation, reason } => {
                self.mark_snapshot_unavailable(generation, reason);
                None
            }
            UiEvent::SnapshotCancelled { generation, reason } => {
                if generation >= self.model.generation {
                    self.mark_snapshot_unavailable(generation, reason.clone());
                    self.set_job(JobState::Cancelled {
                        job_id: BoundedId::try_new(format!("snapshot/{generation}"))
                            .expect("snapshot id"),
                        reason,
                    });
                }
                None
            }
            UiEvent::ActionReviewReady { action } => {
                self.page = UiPage::Review;
                self.set_job(JobState::Idle);
                self.select_action(&action.action_id);
                None
            }
            UiEvent::ActionReviewUnavailable {
                action_id: _,
                reason,
            } => {
                self.page = UiPage::Review;
                self.model.status = reason.clone();
                self.set_all_view_state(ViewState::Failed {
                    generation: self.model.generation,
                    reason: reason.clone(),
                });
                self.set_job(JobState::Failed {
                    job_id: BoundedId::try_new("action-review").expect("id"),
                    reason,
                });
                None
            }
            UiEvent::JobRunning { job_id, phase } => {
                self.set_job(JobState::Running { job_id, phase });
                None
            }
            UiEvent::JobSucceeded {
                receipt,
                verification,
            } => {
                self.set_job(JobState::Succeeded {
                    receipt: receipt.clone(),
                    verification: verification.clone(),
                });
                self.model.status = bounded(format!(
                    "verified · receipt {} · fresh verification {}",
                    receipt, verification
                ));
                self.set_all_view_state(ViewState::Verified {
                    generation: self.model.generation,
                });
                None
            }
            UiEvent::JobCancelled { job_id, reason } => {
                self.set_job(JobState::Cancelled { job_id, reason });
                None
            }
            UiEvent::JobFailed { job_id, reason } => {
                self.set_job(JobState::Failed { job_id, reason });
                None
            }
            UiEvent::RecoveryRequired {
                transaction,
                decision,
            } => {
                self.set_job(JobState::Recovery {
                    transaction,
                    decision,
                });
                None
            }
        }
    }

    pub fn set_confirmation(&mut self, prompt: ConfirmationPrompt) {
        self.page = UiPage::Confirmation;
        self.focus = FocusRegion::Detail;
        self.confirmation_input.clear();
        self.confirmation = Some(prompt);
        self.job = JobState::Idle;
    }

    pub fn clear_confirmation(&mut self) {
        self.confirmation = None;
        self.confirmation_input.clear();
    }

    pub fn mark_stale(&mut self, reason: impl Into<String>) {
        let reason = bounded(reason.into());
        self.model.state = ViewState::Stale {
            generation: self.model.generation,
            reason: reason.clone(),
        };
        for route in &mut self.model.routes {
            route.state = ViewState::Stale {
                generation: self.model.generation,
                reason: reason.clone(),
            };
        }
        self.model.status = reason;
    }

    pub fn set_job(&mut self, job: JobState) {
        self.job = job.clone();
        self.model.job = job.clone();
        match job {
            JobState::Idle => {}
            JobState::Running { .. } => {
                self.page = UiPage::Activity;
            }
            JobState::Succeeded { .. } => {
                self.page = UiPage::Activity;
                self.set_all_view_state(ViewState::Verified {
                    generation: self.model.generation,
                });
            }
            JobState::Cancelled { reason, .. } => {
                self.page = UiPage::Activity;
                self.set_all_view_state(ViewState::Cancelled {
                    generation: self.model.generation,
                    reason,
                });
            }
            JobState::Recovery { decision, .. } => {
                self.page = UiPage::Activity;
                self.set_all_view_state(ViewState::RecoveryRequired {
                    generation: self.model.generation,
                    reason: decision,
                });
            }
            JobState::Failed { reason, .. } => {
                self.page = UiPage::Activity;
                self.set_all_view_state(ViewState::Failed {
                    generation: self.model.generation,
                    reason,
                });
            }
        }
    }

    pub fn view_state(&self) -> &ViewState {
        &self.model.state
    }

    fn apply_search(&mut self, intent: UiIntent) -> Option<UiIntent> {
        match intent {
            UiIntent::SearchCharacter(character) => {
                if !character.is_control() && self.search_query.chars().count() < 120 {
                    self.search_query.push(character);
                    self.selected = 0;
                }
                None
            }
            UiIntent::SearchBackspace => {
                self.search_query.pop();
                self.selected = 0;
                None
            }
            UiIntent::AcceptSearch | UiIntent::Back => {
                self.search_active = false;
                self.overlay = Overlay::None;
                None
            }
            UiIntent::Quit => Some(UiIntent::Quit),
            _ => None,
        }
    }

    fn open_browse(&mut self, browse: BrowseScope) {
        self.browse = browse;
        self.page = match browse {
            BrowseScope::Home => UiPage::Home,
            BrowseScope::Inventory => UiPage::Inventory,
        };
        self.focus = FocusRegion::Queue;
        self.selected = 0;
        self.overlay = Overlay::None;
    }

    fn open_review_if_available(&mut self) {
        if self.selected_record().is_some_and(has_review_boundary) {
            self.page = UiPage::Review;
            self.focus = FocusRegion::Detail;
        }
    }

    fn back(&mut self) -> Option<UiIntent> {
        if self.overlay != Overlay::None {
            self.overlay = Overlay::None;
            return None;
        }
        if self.confirmation.is_some() || self.page == UiPage::Confirmation {
            self.clear_confirmation();
            self.page = UiPage::Review;
            return Some(UiIntent::CancelConfirmation);
        }
        match self.page {
            UiPage::Home => Some(UiIntent::Quit),
            UiPage::Inventory => {
                self.open_browse(BrowseScope::Home);
                None
            }
            UiPage::Evidence => {
                self.open_browse(self.browse);
                None
            }
            UiPage::Review => {
                self.page = UiPage::Evidence;
                self.focus = FocusRegion::Detail;
                None
            }
            UiPage::Activity => {
                self.open_browse(self.browse);
                None
            }
            UiPage::Confirmation => unreachable!(),
        }
    }

    fn move_selection(&mut self, delta: isize) {
        if self.focus != FocusRegion::Queue {
            return;
        }
        let count = self.current_records().len();
        if count == 0 {
            self.selected = 0;
            return;
        }
        self.selected = if delta.is_negative() {
            self.selected.saturating_sub(delta.unsigned_abs())
        } else {
            (self.selected + delta as usize).min(count - 1)
        };
    }

    fn select_action(&mut self, action_id: &BoundedId) {
        if let Some(index) = self.current_records().iter().position(|locator| {
            self.record(locator).is_some_and(|record| {
                record
                    .action_refs
                    .iter()
                    .any(|action| action.action_id == *action_id)
            })
        }) {
            self.selected = index;
        }
    }

    fn home_records(&self) -> Vec<RecordLocator> {
        let mut result = Vec::new();
        let mut ids = BTreeSet::new();
        for route in [Route::Overview, Route::Review] {
            for (index, record) in self.records_for_route(route).into_iter().enumerate() {
                let attention_key = format!("{}\n{}", record.title, record.summary);
                if is_attention(record) && ids.insert(attention_key) {
                    result.push(RecordLocator { route, index });
                }
            }
        }
        if result.is_empty() {
            result.extend(
                self.model
                    .route(Route::Overview)
                    .records
                    .iter()
                    .enumerate()
                    .take(1)
                    .map(|(index, _)| RecordLocator {
                        route: Route::Overview,
                        index,
                    }),
            );
        }
        result.truncate(8);
        result
    }

    fn inventory_records(&self) -> Vec<RecordLocator> {
        Route::ALL
            .into_iter()
            .flat_map(|route| {
                self.model
                    .route(route)
                    .records
                    .iter()
                    .enumerate()
                    .map(move |(index, _)| RecordLocator { route, index })
            })
            .collect()
    }

    fn records_for_route(&self, route: Route) -> Vec<&UiRecord> {
        self.model.route(route).records.iter().collect()
    }

    fn record(&self, locator: &RecordLocator) -> Option<&UiRecord> {
        self.model.route(locator.route).records.get(locator.index)
    }

    fn set_all_view_state(&mut self, view_state: ViewState) {
        self.model.state = view_state.clone();
        for route in &mut self.model.routes {
            route.state = view_state.clone();
        }
    }
}

fn has_review_boundary(record: &UiRecord) -> bool {
    record.action_refs.iter().any(|action| {
        matches!(
            action.disposition,
            ActionDisposition::Reviewable | ActionDisposition::Blocked
        )
    }) || record
        .review_boundary
        .as_ref()
        .is_some_and(|boundary| boundary.disposition != ActionDisposition::Unavailable)
}

fn is_attention(record: &UiRecord) -> bool {
    has_review_boundary(record)
        || matches!(
            record.status,
            RecordStatus::Plan | RecordStatus::Warn | RecordStatus::Blocked | RecordStatus::Error
        )
}

fn bounded(value: impl Into<String>) -> BoundedText {
    BoundedText::try_new(value).unwrap_or_else(|_| BoundedText::redacted())
}

const fn next_focus(focus: FocusRegion) -> FocusRegion {
    match focus {
        FocusRegion::Queue => FocusRegion::Detail,
        FocusRegion::Detail => FocusRegion::Footer,
        FocusRegion::Footer => FocusRegion::Queue,
    }
}

const fn previous_focus(focus: FocusRegion) -> FocusRegion {
    match focus {
        FocusRegion::Queue => FocusRegion::Footer,
        FocusRegion::Detail => FocusRegion::Queue,
        FocusRegion::Footer => FocusRegion::Detail,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::messages::UiIntent;
    use crate::ui::testkit::fixture_model;

    #[test]
    fn stale_snapshot_cannot_replace_newer_generation() {
        let mut state = UiState::new(fixture_model());
        let mut older = state.model.clone();
        older.generation = 10;
        older.state = ViewState::Loading { generation: 10 };
        state.apply_model(older);
        assert_eq!(state.model.generation, 11);
        assert_eq!(state.view_state().label(), "ready");
    }

    #[test]
    fn task_flow_moves_home_to_evidence_to_review_without_confirmation_shortcut() {
        let mut state = UiState::new(fixture_model());
        state.apply(UiIntent::OpenInventory);
        let non_action_index = state
            .current_records()
            .iter()
            .position(|locator| {
                state.record(locator).is_some_and(|record| {
                    !record
                        .action_refs
                        .iter()
                        .any(|action| action.disposition == ActionDisposition::Reviewable)
                })
            })
            .expect("non-action evidence");
        state.apply(UiIntent::SelectIndex(non_action_index));
        state.apply(UiIntent::OpenSelected);
        assert_eq!(state.page, UiPage::Evidence);
        state.apply(UiIntent::OpenReview);
        assert_eq!(state.page, UiPage::Review);
        assert_eq!(state.apply(UiIntent::BeginConfirmation), None);
        assert_eq!(state.page, UiPage::Review);
    }

    #[test]
    fn reviewable_action_enters_activity_before_foundation_prepare() {
        let mut state = UiState::new(fixture_model());
        state.apply(UiIntent::OpenInventory);
        let action_index = state
            .current_records()
            .iter()
            .position(|locator| {
                state.record(locator).is_some_and(|record| {
                    record
                        .action_refs
                        .iter()
                        .any(|action| action.disposition == ActionDisposition::Reviewable)
                })
            })
            .expect("review action");
        state.apply(UiIntent::SelectIndex(action_index));
        state.apply(UiIntent::OpenSelected);
        state.apply(UiIntent::OpenReview);
        let outward = state.apply(UiIntent::BeginConfirmation);
        assert!(matches!(outward, Some(UiIntent::PrepareAction(_))));
        assert_eq!(state.page, UiPage::Activity);
    }

    #[test]
    fn confirmation_is_a_dedicated_state_and_back_cancels_without_execution() {
        let mut state = UiState::new(fixture_model());
        state.set_confirmation(ConfirmationPrompt {
            action_id: BoundedId::try_new("fixture/action").expect("id"),
            plan_id: BoundedId::try_new("fixture/plan").expect("id"),
            plan_sha256: BoundedText::try_new("digest").expect("text"),
            target: BoundedText::try_new("fixture").expect("text"),
            expected_phrase: BoundedText::try_new("CONFIRM").expect("text"),
            risk: BoundedText::try_new("medium").expect("text"),
            expires_unix_seconds: 1,
            rollback_available: true,
            manual_recovery_acknowledged: false,
        });
        assert_eq!(state.page, UiPage::Confirmation);
        assert_eq!(
            state.apply(UiIntent::Back),
            Some(UiIntent::CancelConfirmation)
        );
        assert_eq!(state.page, UiPage::Review);
        assert!(state.confirmation.is_none());
    }

    #[test]
    fn foundation_job_outcomes_are_explicit_and_visible() {
        let mut state = UiState::new(fixture_model());
        state.apply_event(UiEvent::JobRunning {
            job_id: BoundedId::try_new("job/1").expect("id"),
            phase: BoundedText::try_new("preparing").expect("text"),
        });
        assert_eq!(state.page, UiPage::Activity);
        state.apply_event(UiEvent::JobSucceeded {
            receipt: BoundedId::try_new("receipt/1").expect("id"),
            verification: BoundedId::try_new("verify/1").expect("id"),
        });
        assert_eq!(state.view_state().label(), "verified");
        state.apply_event(UiEvent::RecoveryRequired {
            transaction: BoundedId::try_new("transaction/1").expect("id"),
            decision: BoundedText::try_new("review required").expect("text"),
        });
        assert_eq!(state.view_state().label(), "recovery-required");
        state.apply_event(UiEvent::JobCancelled {
            job_id: BoundedId::try_new("job/1").expect("id"),
            reason: BoundedText::try_new("user requested").expect("text"),
        });
        assert_eq!(state.view_state().label(), "cancelled");
    }
}
