use super::messages::{UiEvent, UiIntent};
use super::model::{
    BoundedId, BoundedText, ConfirmationPrompt, JobState, Route, UiModel, ViewState,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusRegion {
    Routes,
    Primary,
    Detail,
    Footer,
}

impl FocusRegion {
    pub const ALL: [Self; 4] = [Self::Routes, Self::Primary, Self::Detail, Self::Footer];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Routes => "routes",
            Self::Primary => "records",
            Self::Detail => "selected detail",
            Self::Footer => "footer",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Overlay {
    None,
    Help,
    Search,
    Detail,
    ActionReview(BoundedId),
    Confirmation(BoundedId),
    Recovery(BoundedId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiState {
    pub route: Route,
    pub focus: FocusRegion,
    pub selected: usize,
    pub route_selected: usize,
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
            route: Route::Overview,
            focus: FocusRegion::Routes,
            selected: 0,
            route_selected: 0,
            overlay: Overlay::None,
            search_query: String::new(),
            search_active: false,
            job: model.job.clone(),
            model,
            confirmation: None,
            confirmation_input: String::new(),
        }
    }

    pub fn current_records(&self) -> Vec<usize> {
        let query = self.search_query.to_ascii_lowercase();
        self.model
            .route(self.route)
            .records
            .iter()
            .enumerate()
            .filter(|(_, record)| {
                query.is_empty()
                    || record.title.as_str().to_ascii_lowercase().contains(&query)
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
            .map(|(index, _)| index)
            .collect()
    }

    pub fn selected_record(&self) -> Option<&super::model::UiRecord> {
        let visible = self.current_records();
        visible
            .get(self.selected)
            .and_then(|index| self.model.route(self.route).records.get(*index))
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
            UiIntent::Navigate(route) => {
                self.route = route;
                self.route_selected = route.number() - 1;
                self.selected = 0;
                self.focus = FocusRegion::Primary;
                self.overlay = Overlay::None;
                None
            }
            UiIntent::FocusRoute(route) => {
                self.route = route;
                self.route_selected = route.number() - 1;
                self.selected = 0;
                self.focus = FocusRegion::Routes;
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
                self.move_focus_selection(1);
                None
            }
            UiIntent::SelectPrevious => {
                self.move_focus_selection(-1);
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
                self.focus = FocusRegion::Primary;
                None
            }
            UiIntent::OpenDetail => {
                if self.selected_record().is_some() {
                    self.overlay = Overlay::Detail;
                    self.focus = FocusRegion::Detail;
                }
                None
            }
            UiIntent::ReviewSelected => {
                if let Some(record) = self.selected_record().cloned() {
                    if let Some(action) = record.action_refs.first() {
                        self.overlay = Overlay::ActionReview(action.action_id.clone());
                    } else if let Some(boundary) = &record.review_boundary {
                        self.overlay = Overlay::ActionReview(boundary.reference_id.clone());
                    }
                }
                None
            }
            UiIntent::BeginConfirmation => {
                if let Some(action) = self.selected_record().and_then(|record| {
                    record.action_refs.iter().find(|action| {
                        action.disposition == super::model::ActionDisposition::Reviewable
                    })
                }) {
                    return Some(UiIntent::PrepareAction(action.action_id.clone()));
                }
                None
            }
            UiIntent::PrepareAction(_) => None,
            UiIntent::LoadProviderReview => Some(UiIntent::LoadProviderReview),
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
                self.confirmation = None;
                self.confirmation_input.clear();
                self.overlay = Overlay::ActionReview(
                    self.selected_record()
                        .and_then(|record| record.action_refs.first())
                        .map(|action| action.action_id.clone())
                        .unwrap_or_else(|| BoundedId::try_new("review/unavailable").expect("id")),
                );
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
            UiIntent::Back => {
                if self.confirmation.is_some() {
                    self.confirmation = None;
                    self.confirmation_input.clear();
                    self.overlay = Overlay::None;
                    return Some(UiIntent::CancelConfirmation);
                }
                if self.overlay != Overlay::None {
                    self.overlay = Overlay::None;
                } else if self.focus != FocusRegion::Routes {
                    self.focus = FocusRegion::Routes;
                } else {
                    return Some(UiIntent::Quit);
                }
                None
            }
            UiIntent::Refresh => Some(UiIntent::Refresh),
            UiIntent::BeginSearch => {
                self.search_active = true;
                self.overlay = Overlay::Search;
                None
            }
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
            UiIntent::AcceptSearch => {
                self.search_active = false;
                self.overlay = Overlay::None;
                None
            }
            UiIntent::ReviewAction(action_id) => {
                self.overlay = Overlay::ActionReview(action_id);
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
                self.overlay = Overlay::ActionReview(action.action_id);
                self.set_job(JobState::Idle);
                None
            }
            UiEvent::ActionReviewUnavailable { action_id, reason } => {
                self.overlay = Overlay::ActionReview(action_id);
                self.model.status = reason;
                self.set_job(JobState::Failed {
                    job_id: BoundedId::try_new("action-review").expect("id"),
                    reason: self.model.status.clone(),
                });
                self.model.state = ViewState::Failed {
                    generation: self.model.generation,
                    reason: self.model.status.clone(),
                };
                self.model.route_mut(self.route).state = ViewState::Failed {
                    generation: self.model.generation,
                    reason: self.model.status.clone(),
                };
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
                    receipt,
                    verification,
                });
                self.mark_stale("action completed; refresh for new evidence");
                None
            }
            UiEvent::JobCancelled { job_id, reason } => {
                self.set_job(JobState::Cancelled { job_id, reason });
                None
            }
            UiEvent::JobFailed { job_id, reason } => {
                self.set_job(JobState::Failed {
                    job_id,
                    reason: reason.clone(),
                });
                self.model.state = ViewState::Failed {
                    generation: self.model.generation,
                    reason: reason.clone(),
                };
                for route in &mut self.model.routes {
                    route.state = ViewState::Failed {
                        generation: self.model.generation,
                        reason: reason.clone(),
                    };
                }
                None
            }
            UiEvent::RecoveryRequired {
                transaction,
                decision,
            } => {
                self.overlay = Overlay::Recovery(transaction.clone());
                self.set_job(JobState::Recovery {
                    transaction,
                    decision,
                });
                None
            }
        }
    }

    pub fn set_confirmation(&mut self, prompt: ConfirmationPrompt) {
        self.overlay = Overlay::Confirmation(prompt.action_id.clone());
        self.confirmation_input.clear();
        self.confirmation = Some(prompt);
    }

    pub fn clear_confirmation(&mut self) {
        self.confirmation = None;
        self.confirmation_input.clear();
    }

    pub fn mark_stale(&mut self, reason: impl Into<String>) {
        let reason =
            BoundedText::try_new(reason.into()).unwrap_or_else(|_| BoundedText::redacted());
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
        self.model.job = job;
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
            UiIntent::CancelConfirmation
            | UiIntent::BeginConfirmation
            | UiIntent::PrepareAction(_)
            | UiIntent::SubmitConfirmation => None,
            _ => None,
        }
    }

    fn move_selection(&mut self, delta: isize) {
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

    fn move_focus_selection(&mut self, delta: isize) {
        if self.focus == FocusRegion::Routes {
            let current = self.route.number().saturating_sub(1) as isize;
            let last = Route::ALL.len().saturating_sub(1) as isize;
            let next = (current + delta).clamp(0, last) as usize;
            if let Some(route) = Route::ALL.get(next).copied() {
                self.route = route;
                self.route_selected = next;
                self.selected = 0;
            }
        } else {
            self.move_selection(delta);
        }
    }
}

const fn next_focus(focus: FocusRegion) -> FocusRegion {
    match focus {
        FocusRegion::Routes => FocusRegion::Primary,
        FocusRegion::Primary => FocusRegion::Detail,
        FocusRegion::Detail => FocusRegion::Footer,
        FocusRegion::Footer => FocusRegion::Routes,
    }
}

const fn previous_focus(focus: FocusRegion) -> FocusRegion {
    match focus {
        FocusRegion::Routes => FocusRegion::Footer,
        FocusRegion::Primary => FocusRegion::Routes,
        FocusRegion::Detail => FocusRegion::Primary,
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
    fn safe_return_closes_overlay_then_returns_to_routes_before_quit() {
        let mut state = UiState::new(fixture_model());
        state.focus = FocusRegion::Primary;
        state.apply(UiIntent::OpenDetail);
        assert_eq!(state.focus, FocusRegion::Detail);
        assert_eq!(state.overlay, Overlay::Detail);

        state.apply(UiIntent::Back);
        assert_eq!(state.overlay, Overlay::None);
        state.apply(UiIntent::Back);
        assert_eq!(state.focus, FocusRegion::Routes);
        assert_eq!(state.apply(UiIntent::Back), Some(UiIntent::Quit));
    }

    #[test]
    fn foundation_job_events_cover_success_stale_recovery_cancel_and_failure() {
        let mut state = UiState::new(fixture_model());
        state.apply_event(UiEvent::JobRunning {
            job_id: BoundedId::try_new("job/1").expect("id"),
            phase: BoundedText::try_new("preparing").expect("text"),
        });
        assert!(matches!(state.job, JobState::Running { .. }));
        state.apply_event(UiEvent::JobSucceeded {
            receipt: BoundedId::try_new("receipt/1").expect("id"),
            verification: BoundedId::try_new("verify/1").expect("id"),
        });
        assert_eq!(state.view_state().label(), "stale");
        state.apply_event(UiEvent::RecoveryRequired {
            transaction: BoundedId::try_new("transaction/1").expect("id"),
            decision: BoundedText::try_new("review required").expect("text"),
        });
        assert!(matches!(state.job, JobState::Recovery { .. }));
        state.apply_event(UiEvent::JobCancelled {
            job_id: BoundedId::try_new("job/1").expect("id"),
            reason: BoundedText::try_new("user requested").expect("text"),
        });
        assert!(matches!(state.job, JobState::Cancelled { .. }));
        state.apply_event(UiEvent::JobFailed {
            job_id: BoundedId::try_new("job/1").expect("id"),
            reason: BoundedText::try_new("foundation failure").expect("text"),
        });
        assert_eq!(state.view_state().label(), "failed");
    }

    #[test]
    fn confirmation_input_is_local_and_submission_is_outbound() {
        let mut state = UiState::new(fixture_model());
        state.route = Route::Review;
        state.apply(UiIntent::BeginConfirmation);
        assert_eq!(
            state
                .selected_record()
                .and_then(|record| record.action_refs.first()),
            Some(&state.model.route(Route::Review).records[0].action_refs[0])
        );
        state.set_confirmation(ConfirmationPrompt {
            action_id: BoundedId::try_new("fixture/review-action").expect("id"),
            plan_id: BoundedId::try_new("fixture/plan").expect("id"),
            plan_sha256: BoundedText::try_new("digest").expect("text"),
            target: BoundedText::try_new("fixture").expect("text"),
            expected_phrase: BoundedText::try_new("CONFIRM fixture").expect("text"),
            risk: BoundedText::try_new("medium").expect("text"),
            expires_unix_seconds: 1,
            rollback_available: true,
            manual_recovery_acknowledged: false,
        });
        state.apply(UiIntent::ConfirmationCharacter('C'));
        assert_eq!(state.confirmation_input, "C");
        assert_eq!(
            state.apply(UiIntent::SubmitConfirmation),
            Some(UiIntent::SubmitConfirmation)
        );
    }

    #[test]
    fn route_focus_moves_destinations_without_moving_record_selection() {
        let mut state = UiState::new(fixture_model());
        state.apply(UiIntent::SelectNext);
        assert_eq!(state.route, Route::Explore);
        assert_eq!(state.selected, 0);
        state.apply(UiIntent::FocusNext);
        state.apply(UiIntent::SelectNext);
        assert_eq!(state.selected, 1);
    }

    #[test]
    fn action_review_failure_is_visible_as_a_failed_view() {
        let mut state = UiState::new(fixture_model());
        state.apply_event(UiEvent::ActionReviewUnavailable {
            action_id: BoundedId::try_new("review/1").expect("id"),
            reason: BoundedText::try_new("provider review failed").expect("text"),
        });
        assert_eq!(state.view_state().label(), "failed");
        assert!(matches!(state.job, JobState::Failed { .. }));
    }

    #[test]
    fn recovery_event_opens_read_only_recovery_evidence() {
        let mut state = UiState::new(fixture_model());
        state.apply_event(UiEvent::RecoveryRequired {
            transaction: BoundedId::try_new("transaction/1").expect("id"),
            decision: BoundedText::try_new("review required").expect("text"),
        });
        assert_eq!(
            state.overlay,
            Overlay::Recovery(BoundedId::try_new("transaction/1").expect("id"))
        );
        assert!(matches!(state.job, JobState::Recovery { .. }));
    }
}
