use runtime_zero::tui_state::{TuiAction, TuiFocusRegion, TuiInput, TuiMouseTarget, TuiState};

#[test]
fn q_requests_quit_without_state_mutation() {
    let mut state = TuiState::new(3);
    assert_eq!(state.apply(TuiInput::Quit), TuiAction::Quit);
    assert_eq!(state.selected_section, 0);
}

#[test]
fn refresh_requests_a_new_read_only_snapshot_without_quitting() {
    let mut state = TuiState::new(3);
    state.apply(TuiInput::FocusNext);
    state.apply(TuiInput::Activate);
    assert_eq!(state.apply(TuiInput::Refresh), TuiAction::Refresh);
    assert!(!state.preview_open);
    assert_eq!(state.apply(TuiInput::CheckUpdates), TuiAction::CheckUpdates);

    state.apply(TuiInput::ToggleHelp);
    assert_eq!(state.apply(TuiInput::Refresh), TuiAction::Continue);
}

#[test]
fn search_filter_and_sort_stay_bounded_and_read_only() {
    let mut state = TuiState::new(3);
    assert_eq!(state.apply(TuiInput::BeginSearch), TuiAction::Continue);
    for value in "brew".chars() {
        state.apply(TuiInput::SearchCharacter(value));
    }
    assert!(state.search_active());
    assert_eq!(state.search_query(), "brew");
    state.apply(TuiInput::EndSearch);
    assert!(!state.search_active());

    state.apply(TuiInput::FilterNext);
    state.apply(TuiInput::SortNext);
    assert_eq!(state.software_view().filter.label(), "applications");
    assert_eq!(state.software_view().sort.label(), "version");
    assert_eq!(state.apply(TuiInput::Refresh), TuiAction::Refresh);
}

#[test]
fn escape_cancels_search_before_quitting() {
    let mut state = TuiState::new(3);
    state.apply(TuiInput::BeginSearch);
    state.apply(TuiInput::SearchCharacter('x'));
    assert_eq!(state.apply(TuiInput::Back), TuiAction::Continue);
    assert!(!state.search_active());
    assert_eq!(state.search_query(), "x");
    assert_eq!(state.apply(TuiInput::Back), TuiAction::Quit);
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

    let _ = state.apply(TuiInput::FocusNext);
    state.selected_detail_row = 2;
    let _ = state.apply(TuiInput::FirstSection);
    assert_eq!(state.selected_detail_row, 0);
    let _ = state.apply(TuiInput::LastSection);
    assert_eq!(state.selected_detail_row, usize::MAX);
}

#[test]
fn tab_cycles_focus_regions_without_mutating_actions() {
    let mut state = TuiState::new(4);
    assert_eq!(state.focus_region, TuiFocusRegion::LeftNavigation);
    let _ = state.apply(TuiInput::FocusNext);
    assert_eq!(state.focus_region, TuiFocusRegion::DetailsPanel);
    let _ = state.apply(TuiInput::FocusNext);
    assert_eq!(state.focus_region, TuiFocusRegion::ContextPane);
    let _ = state.apply(TuiInput::FocusNext);
    assert_eq!(state.focus_region, TuiFocusRegion::LeftNavigation);
    assert!(!state.preview_open);
}

#[test]
fn update_check_key_works_from_every_content_focus_region() {
    for focus_region in [
        TuiFocusRegion::LeftNavigation,
        TuiFocusRegion::DetailsPanel,
        TuiFocusRegion::ContextPane,
    ] {
        let mut state = TuiState::new(4);
        state.focus_region = focus_region;
        assert_eq!(
            state.apply(TuiInput::CheckUpdates),
            TuiAction::CheckUpdates,
            "update checks should not depend on focus region: {focus_region:?}"
        );
    }
}

#[test]
fn uppercase_update_enters_exact_confirmation_text_mode() {
    let mut state = TuiState::new(4);
    assert_eq!(
        state.apply(TuiInput::UpdateSelected),
        TuiAction::UpdateSelected
    );
    state.begin_update_confirmation();
    state.apply(TuiInput::ConfirmCharacter('a'));
    state.apply(TuiInput::ConfirmCharacter('b'));
    assert_eq!(state.update_confirmation_phrase(), "ab");
    assert_eq!(
        state.apply(TuiInput::SubmitUpdateConfirmation),
        TuiAction::SubmitUpdateConfirmation
    );
    assert_eq!(state.finish_update_confirmation(), "ab");
    assert!(!state.update_confirmation_active());
}

#[test]
fn shift_tab_cycles_focus_backward() {
    let mut state = TuiState::new(4);
    let _ = state.apply(TuiInput::FocusPrevious);
    assert_eq!(state.focus_region, TuiFocusRegion::ContextPane);
    let _ = state.apply(TuiInput::FocusPrevious);
    assert_eq!(state.focus_region, TuiFocusRegion::DetailsPanel);
}

#[test]
fn context_pane_stays_scoped_to_the_selected_row() {
    let mut state = TuiState::new(4);
    let _ = state.apply(TuiInput::FocusNext);
    let _ = state.apply(TuiInput::FocusNext);
    assert_eq!(state.focus_region, TuiFocusRegion::ContextPane);
    state.selected_detail_row = 2;
    let _ = state.apply(TuiInput::NextItem);
    assert_eq!(state.selected_detail_row, 2);
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
fn activation_from_navigation_opens_selected_details() {
    let mut state = TuiState::new(4);
    assert_eq!(state.focus_region, TuiFocusRegion::LeftNavigation);
    assert_eq!(state.apply(TuiInput::Activate), TuiAction::Continue);
    assert_eq!(state.focus_region, TuiFocusRegion::DetailsPanel);
    assert!(state.preview_open);
}

#[test]
fn monitor_shortcut_jumps_to_the_last_section() {
    let mut state = TuiState::new(5);
    assert_eq!(state.apply(TuiInput::OpenMonitor), TuiAction::Continue);
    assert_eq!(state.selected_section, 3);
    assert_eq!(state.focus_region, TuiFocusRegion::DetailsPanel);
}

#[test]
fn mouse_scroll_moves_the_list_under_the_pointer() {
    let mut state = TuiState::new(4);
    assert_eq!(
        state.apply(TuiInput::ScrollDown(TuiMouseTarget::Details)),
        TuiAction::Continue
    );
    assert_eq!(state.focus_region, TuiFocusRegion::DetailsPanel);
    assert_eq!(state.selected_detail_row, 3);
    assert_eq!(
        state.apply(TuiInput::ScrollUp(TuiMouseTarget::Details)),
        TuiAction::Continue
    );
    assert_eq!(state.selected_detail_row, 0);

    let _ = state.apply(TuiInput::ScrollDown(TuiMouseTarget::Navigation));
    assert_eq!(state.focus_region, TuiFocusRegion::LeftNavigation);
    assert_eq!(state.selected_section, 3);
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
