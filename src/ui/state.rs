use super::messages::UiIntent;
use super::model::{BoundedId, BoundedText, JobState, Route, UiModel, ViewState};

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
            UiIntent::ToggleHelp => {
                self.overlay = if self.overlay == Overlay::Help {
                    Overlay::None
                } else {
                    Overlay::Help
                };
                None
            }
            UiIntent::Back => {
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
}
