#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TuiState {
    pub selected_section: usize,
    pub show_help: bool,
    section_count: usize,
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
    NextSection,
    PreviousSection,
    FirstSection,
    LastSection,
    Resize,
    Other,
}

impl TuiState {
    pub fn new(section_count: usize) -> Self {
        Self {
            selected_section: 0,
            show_help: false,
            section_count,
        }
    }

    pub fn apply(&mut self, input: TuiInput) -> TuiAction {
        match input {
            TuiInput::Quit => TuiAction::Quit,
            TuiInput::ToggleHelp => {
                self.show_help = !self.show_help;
                TuiAction::Continue
            }
            TuiInput::NextSection => {
                if self.section_count > 0 {
                    self.selected_section = (self.selected_section + 1) % self.section_count;
                }
                TuiAction::Continue
            }
            TuiInput::PreviousSection => {
                if self.section_count > 0 {
                    self.selected_section =
                        (self.selected_section + self.section_count - 1) % self.section_count;
                }
                TuiAction::Continue
            }
            TuiInput::FirstSection => {
                self.selected_section = 0;
                TuiAction::Continue
            }
            TuiInput::LastSection => {
                if self.section_count > 0 {
                    self.selected_section = self.section_count - 1;
                }
                TuiAction::Continue
            }
            TuiInput::Resize | TuiInput::Other => TuiAction::Continue,
        }
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
        let _ = state.apply(TuiInput::NextSection);
        assert_eq!(state.selected_section, 1);
        let _ = state.apply(TuiInput::NextSection);
        assert_eq!(state.selected_section, 0);
        let _ = state.apply(TuiInput::PreviousSection);
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
}
