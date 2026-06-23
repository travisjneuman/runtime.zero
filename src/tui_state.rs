#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TuiState {
    pub selected_section: usize,
    pub selected_detail_row: usize,
    pub selected_command: usize,
    pub focus_region: TuiFocusRegion,
    pub show_help: bool,
    pub preview_open: bool,
    section_count: usize,
    command_count: usize,
    previous_focus_region: TuiFocusRegion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiFocusRegion {
    LeftNavigation,
    DetailsPanel,
    CommandRail,
    HelpOverlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiAction {
    Quit,
    Continue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiInput {
    Quit,
    ToggleHelp,
    FocusNext,
    FocusPrevious,
    NextItem,
    PreviousItem,
    FirstSection,
    LastSection,
    Activate,
    Back,
    Resize,
    Other,
}

impl TuiState {
    pub fn new(section_count: usize) -> Self {
        Self {
            selected_section: 0,
            selected_detail_row: 0,
            selected_command: 0,
            focus_region: TuiFocusRegion::LeftNavigation,
            show_help: false,
            preview_open: false,
            section_count,
            command_count: 4,
            previous_focus_region: TuiFocusRegion::LeftNavigation,
        }
    }

    pub fn apply(&mut self, input: TuiInput) -> TuiAction {
        match input {
            TuiInput::Quit => TuiAction::Quit,
            TuiInput::ToggleHelp => {
                self.show_help = !self.show_help;
                if self.show_help {
                    self.previous_focus_region = self.focus_region;
                    self.focus_region = TuiFocusRegion::HelpOverlay;
                    self.preview_open = false;
                } else {
                    self.focus_region = self.previous_focus_region;
                }
                TuiAction::Continue
            }
            TuiInput::FocusNext => {
                self.cycle_focus_forward();
                TuiAction::Continue
            }
            TuiInput::FocusPrevious => {
                self.cycle_focus_backward();
                TuiAction::Continue
            }
            TuiInput::NextItem => {
                self.move_item_forward();
                TuiAction::Continue
            }
            TuiInput::PreviousItem => {
                self.move_item_backward();
                TuiAction::Continue
            }
            TuiInput::FirstSection => {
                if self.focus_region == TuiFocusRegion::LeftNavigation {
                    self.selected_section = 0;
                    self.selected_detail_row = 0;
                }
                TuiAction::Continue
            }
            TuiInput::LastSection => {
                if self.focus_region == TuiFocusRegion::LeftNavigation && self.section_count > 0 {
                    self.selected_section = self.section_count - 1;
                    self.selected_detail_row = 0;
                }
                TuiAction::Continue
            }
            TuiInput::Activate => {
                self.activate_read_only_preview();
                TuiAction::Continue
            }
            TuiInput::Back => self.back_or_quit(),
            TuiInput::Resize | TuiInput::Other => TuiAction::Continue,
        }
    }

    fn cycle_focus_forward(&mut self) {
        if self.show_help {
            return;
        }
        self.focus_region = match self.focus_region {
            TuiFocusRegion::LeftNavigation => TuiFocusRegion::DetailsPanel,
            TuiFocusRegion::DetailsPanel => TuiFocusRegion::CommandRail,
            TuiFocusRegion::CommandRail | TuiFocusRegion::HelpOverlay => {
                TuiFocusRegion::LeftNavigation
            }
        };
        self.preview_open = false;
    }

    fn cycle_focus_backward(&mut self) {
        if self.show_help {
            return;
        }
        self.focus_region = match self.focus_region {
            TuiFocusRegion::LeftNavigation | TuiFocusRegion::HelpOverlay => {
                TuiFocusRegion::CommandRail
            }
            TuiFocusRegion::DetailsPanel => TuiFocusRegion::LeftNavigation,
            TuiFocusRegion::CommandRail => TuiFocusRegion::DetailsPanel,
        };
        self.preview_open = false;
    }

    fn move_item_forward(&mut self) {
        match self.focus_region {
            TuiFocusRegion::LeftNavigation => self.next_section(),
            TuiFocusRegion::DetailsPanel => {
                self.selected_detail_row = self.selected_detail_row.saturating_add(1);
            }
            TuiFocusRegion::CommandRail => {
                if self.command_count > 0 {
                    self.selected_command = (self.selected_command + 1) % self.command_count;
                }
            }
            TuiFocusRegion::HelpOverlay => {}
        }
    }

    fn move_item_backward(&mut self) {
        match self.focus_region {
            TuiFocusRegion::LeftNavigation => self.previous_section(),
            TuiFocusRegion::DetailsPanel => {
                self.selected_detail_row = self.selected_detail_row.saturating_sub(1);
            }
            TuiFocusRegion::CommandRail => {
                if self.command_count > 0 {
                    self.selected_command =
                        (self.selected_command + self.command_count - 1) % self.command_count;
                }
            }
            TuiFocusRegion::HelpOverlay => {}
        }
    }

    fn next_section(&mut self) {
        if self.section_count > 0 {
            self.selected_section = (self.selected_section + 1) % self.section_count;
            self.selected_detail_row = 0;
        }
    }

    fn previous_section(&mut self) {
        if self.section_count > 0 {
            self.selected_section =
                (self.selected_section + self.section_count - 1) % self.section_count;
            self.selected_detail_row = 0;
        }
    }

    fn activate_read_only_preview(&mut self) {
        match self.focus_region {
            TuiFocusRegion::LeftNavigation => {
                self.focus_region = TuiFocusRegion::DetailsPanel;
                self.preview_open = false;
            }
            TuiFocusRegion::DetailsPanel | TuiFocusRegion::CommandRail => {
                self.preview_open = !self.preview_open;
            }
            TuiFocusRegion::HelpOverlay => {}
        }
    }

    fn back_or_quit(&mut self) -> TuiAction {
        if self.show_help {
            self.show_help = false;
            self.focus_region = self.previous_focus_region;
            return TuiAction::Continue;
        }
        if self.preview_open {
            self.preview_open = false;
            return TuiAction::Continue;
        }
        if self.focus_region != TuiFocusRegion::LeftNavigation {
            self.focus_region = TuiFocusRegion::LeftNavigation;
            return TuiAction::Continue;
        }
        TuiAction::Quit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
