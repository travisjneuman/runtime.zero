use crate::apps::SoftwareView;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiState {
    pub selected_section: usize,
    pub selected_detail_row: usize,
    pub selected_command: usize,
    pub focus_region: TuiFocusRegion,
    pub show_help: bool,
    pub preview_open: bool,
    software_view: SoftwareView,
    search_active: bool,
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
pub enum TuiMouseTarget {
    Navigation,
    Details,
    Commands,
}

impl TuiMouseTarget {
    const fn focus_region(self) -> TuiFocusRegion {
        match self {
            Self::Navigation => TuiFocusRegion::LeftNavigation,
            Self::Details => TuiFocusRegion::DetailsPanel,
            Self::Commands => TuiFocusRegion::CommandRail,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuiAction {
    Quit,
    Refresh,
    RefreshMonitor,
    CheckUpdates,
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
    Refresh,
    RefreshMonitor,
    CheckUpdates,
    OpenMonitor,
    BeginSearch,
    EndSearch,
    SearchCharacter(char),
    SearchBackspace,
    FilterNext,
    SortNext,
    ScrollUp(TuiMouseTarget),
    ScrollDown(TuiMouseTarget),
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
            command_count: crate::tui_command_rail::COMMANDS.len(),
            software_view: SoftwareView::default(),
            search_active: false,
            previous_focus_region: TuiFocusRegion::LeftNavigation,
        }
    }

    pub fn software_view(&self) -> &SoftwareView {
        &self.software_view
    }

    pub fn set_software_view(&mut self, view: SoftwareView) {
        self.software_view = view;
    }

    pub fn search_active(&self) -> bool {
        self.search_active
    }

    pub fn search_query(&self) -> &str {
        self.software_view.query()
    }

    pub fn clamp_detail_row(&mut self, row_count: usize) {
        self.selected_detail_row = if row_count == 0 {
            0
        } else {
            self.selected_detail_row.min(row_count - 1)
        };
    }

    pub fn apply(&mut self, input: TuiInput) -> TuiAction {
        if self.search_active {
            return self.apply_search_input(input);
        }
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
            TuiInput::ScrollDown(target) => {
                self.focus_region = target.focus_region();
                self.move_item_forward_by(3);
                TuiAction::Continue
            }
            TuiInput::ScrollUp(target) => {
                self.focus_region = target.focus_region();
                self.move_item_backward_by(3);
                TuiAction::Continue
            }
            TuiInput::FirstSection => {
                match self.focus_region {
                    TuiFocusRegion::LeftNavigation => {
                        self.selected_section = 0;
                        self.selected_detail_row = 0;
                    }
                    TuiFocusRegion::DetailsPanel => self.selected_detail_row = 0,
                    TuiFocusRegion::CommandRail => self.selected_command = 0,
                    TuiFocusRegion::HelpOverlay => {}
                }
                TuiAction::Continue
            }
            TuiInput::LastSection => {
                match self.focus_region {
                    TuiFocusRegion::LeftNavigation if self.section_count > 0 => {
                        self.selected_section = self.section_count - 1;
                        self.selected_detail_row = 0;
                    }
                    TuiFocusRegion::DetailsPanel => self.selected_detail_row = usize::MAX,
                    TuiFocusRegion::CommandRail if self.command_count > 0 => {
                        self.selected_command = self.command_count - 1;
                    }
                    TuiFocusRegion::LeftNavigation
                    | TuiFocusRegion::CommandRail
                    | TuiFocusRegion::HelpOverlay => {}
                }
                TuiAction::Continue
            }
            TuiInput::Activate => {
                self.activate_read_only_preview();
                TuiAction::Continue
            }
            TuiInput::Back => self.back_or_quit(),
            TuiInput::Refresh if !self.show_help => {
                self.preview_open = false;
                TuiAction::Refresh
            }
            TuiInput::Refresh => TuiAction::Continue,
            TuiInput::RefreshMonitor => TuiAction::RefreshMonitor,
            TuiInput::CheckUpdates if !self.show_help => TuiAction::CheckUpdates,
            TuiInput::CheckUpdates => TuiAction::Continue,
            TuiInput::OpenMonitor if !self.show_help => {
                if self.section_count > 0 {
                    self.selected_section = self.section_count - 1;
                    self.selected_detail_row = 0;
                    self.focus_region = TuiFocusRegion::DetailsPanel;
                    self.preview_open = false;
                }
                TuiAction::Continue
            }
            TuiInput::OpenMonitor => TuiAction::Continue,
            TuiInput::BeginSearch if !self.show_help => {
                self.software_view.clear_query();
                self.search_active = true;
                self.preview_open = false;
                TuiAction::Continue
            }
            TuiInput::FilterNext if !self.show_help => {
                self.software_view.filter = self.software_view.filter.next();
                self.selected_detail_row = 0;
                TuiAction::Continue
            }
            TuiInput::SortNext if !self.show_help => {
                self.software_view.sort = self.software_view.sort.next();
                self.selected_detail_row = 0;
                TuiAction::Continue
            }
            TuiInput::EndSearch
            | TuiInput::SearchCharacter(_)
            | TuiInput::SearchBackspace
            | TuiInput::Resize
            | TuiInput::Other
            | TuiInput::BeginSearch
            | TuiInput::FilterNext
            | TuiInput::SortNext => TuiAction::Continue,
        }
    }

    fn apply_search_input(&mut self, input: TuiInput) -> TuiAction {
        match input {
            TuiInput::EndSearch | TuiInput::Back => {
                self.search_active = false;
                TuiAction::Continue
            }
            TuiInput::SearchCharacter(value) => {
                self.software_view.push_query(value);
                self.selected_detail_row = 0;
                TuiAction::Continue
            }
            TuiInput::SearchBackspace => {
                self.software_view.pop_query();
                self.selected_detail_row = 0;
                TuiAction::Continue
            }
            TuiInput::Quit => TuiAction::Quit,
            TuiInput::Refresh | TuiInput::RefreshMonitor | TuiInput::CheckUpdates => {
                TuiAction::Continue
            }
            TuiInput::OpenMonitor => TuiAction::Continue,
            TuiInput::ScrollUp(_) | TuiInput::ScrollDown(_) => TuiAction::Continue,
            TuiInput::Resize | TuiInput::Other => TuiAction::Continue,
            TuiInput::BeginSearch
            | TuiInput::FilterNext
            | TuiInput::SortNext
            | TuiInput::ToggleHelp
            | TuiInput::FocusNext
            | TuiInput::FocusPrevious
            | TuiInput::NextItem
            | TuiInput::PreviousItem
            | TuiInput::FirstSection
            | TuiInput::LastSection
            | TuiInput::Activate => TuiAction::Continue,
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
        self.move_item_forward_by(1);
    }

    fn move_item_forward_by(&mut self, count: usize) {
        self.preview_open = false;
        for _ in 0..count {
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
    }

    fn move_item_backward(&mut self) {
        self.move_item_backward_by(1);
    }

    fn move_item_backward_by(&mut self, count: usize) {
        self.preview_open = false;
        for _ in 0..count {
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
                self.preview_open = true;
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
        if self.search_active {
            self.search_active = false;
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
