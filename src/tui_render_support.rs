use crate::tui_dashboard::{TuiDashboard, TuiSection};
use crate::tui_state::TuiState;

pub(crate) fn selected_section<'a>(
    dashboard: &'a TuiDashboard,
    state: &TuiState,
) -> &'a TuiSection {
    &dashboard.sections[selected_index(dashboard, state)]
}

pub(crate) fn selected_index(dashboard: &TuiDashboard, state: &TuiState) -> usize {
    if dashboard.sections.is_empty() {
        0
    } else {
        state.selected_section.min(dashboard.sections.len() - 1)
    }
}
