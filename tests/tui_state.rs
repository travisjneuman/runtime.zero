use runtime_zero::tui_state::{TuiAction, TuiFocusRegion, TuiInput, TuiState};

#[test]
fn q_requests_quit_without_state_mutation() {
    let mut state = TuiState::new(3);
    assert_eq!(state.apply(TuiInput::Quit), TuiAction::Quit);
    assert_eq!(state.selected_section, 0);
}

#[test]
fn help_toggles_and_navigation_wraps() {
    let mut state = TuiState::new(2);
    assert_eq!(state.apply(TuiInput::ToggleHelp), TuiAction::Continue);
    assert!(state.show_help);
    let _ = state.apply(TuiInput::ToggleHelp);
    let _ = state.apply(TuiInput::NextItem);
    assert_eq!(state.selected_section, 1);
    let _ = state.apply(TuiInput::NextItem);
    assert_eq!(state.selected_section, 0);
    let _ = state.apply(TuiInput::PreviousItem);
    assert_eq!(state.selected_section, 1);
}

#[test]
fn home_and_end_jump_to_edges() {
    let mut state = TuiState::new(4);
    let _ = state.apply(TuiInput::LastSection);
    assert_eq!(state.selected_section, 3);
    let _ = state.apply(TuiInput::FirstSection);
    assert_eq!(state.selected_section, 0);
}

#[test]
fn tab_cycles_focus_regions_without_mutating_actions() {
    let mut state = TuiState::new(4);
    assert_eq!(state.focus_region, TuiFocusRegion::LeftNavigation);
    let _ = state.apply(TuiInput::FocusNext);
    assert_eq!(state.focus_region, TuiFocusRegion::DetailsPanel);
    let _ = state.apply(TuiInput::FocusNext);
    assert_eq!(state.focus_region, TuiFocusRegion::CommandRail);
    let _ = state.apply(TuiInput::FocusNext);
    assert_eq!(state.focus_region, TuiFocusRegion::LeftNavigation);
    assert!(!state.preview_open);
}

#[test]
fn shift_tab_cycles_focus_backward() {
    let mut state = TuiState::new(4);
    let _ = state.apply(TuiInput::FocusPrevious);
    assert_eq!(state.focus_region, TuiFocusRegion::CommandRail);
    let _ = state.apply(TuiInput::FocusPrevious);
    assert_eq!(state.focus_region, TuiFocusRegion::DetailsPanel);
}

#[test]
fn enter_space_only_toggle_read_only_preview() {
    let mut state = TuiState::new(4);
    let _ = state.apply(TuiInput::FocusNext);
    assert_eq!(state.focus_region, TuiFocusRegion::DetailsPanel);
    assert_eq!(state.apply(TuiInput::Activate), TuiAction::Continue);
    assert!(state.preview_open);
    assert_eq!(state.apply(TuiInput::Activate), TuiAction::Continue);
    assert!(!state.preview_open);
}

#[test]
fn activation_from_navigation_moves_to_details_without_execution_preview() {
    let mut state = TuiState::new(4);
    assert_eq!(state.focus_region, TuiFocusRegion::LeftNavigation);
    assert_eq!(state.apply(TuiInput::Activate), TuiAction::Continue);
    assert_eq!(state.focus_region, TuiFocusRegion::DetailsPanel);
    assert!(!state.preview_open);
}

#[test]
fn escape_closes_preview_help_or_focus_before_quitting() {
    let mut state = TuiState::new(4);
    let _ = state.apply(TuiInput::FocusNext);
    let _ = state.apply(TuiInput::Activate);
    assert_eq!(state.apply(TuiInput::Back), TuiAction::Continue);
    assert!(!state.preview_open);
    assert_eq!(state.apply(TuiInput::Back), TuiAction::Continue);
    assert_eq!(state.focus_region, TuiFocusRegion::LeftNavigation);
    assert_eq!(state.apply(TuiInput::ToggleHelp), TuiAction::Continue);
    assert_eq!(state.focus_region, TuiFocusRegion::HelpOverlay);
    assert_eq!(state.apply(TuiInput::Back), TuiAction::Continue);
    assert!(!state.show_help);
    assert_eq!(state.apply(TuiInput::Back), TuiAction::Quit);
}
