use std::io::{self, Write};

use crossterm::cursor::{Hide, Show};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::layout::Rect;

use crate::launch_routing::LaunchRoutingReport;
use crate::tui_dashboard;
use crate::tui_layout::TuiLayoutTier;
use crate::tui_ratatui::draw_dashboard;
use crate::tui_ratatui_support::help_height;
use crate::tui_state::{TuiAction, TuiInput, TuiMouseTarget, TuiState};

pub fn run_interactive_tui(launch_context: &LaunchRoutingReport, color: bool) -> io::Result<()> {
    let mut stdout = io::stdout();
    let _terminal = TerminalGuard::enter(&mut stdout)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    run_event_loop(&mut terminal, launch_context, color)
}

fn run_event_loop<B: Backend<Error = io::Error>>(
    terminal: &mut Terminal<B>,
    launch_context: &LaunchRoutingReport,
    color: bool,
) -> io::Result<()> {
    let mut dashboard = tui_dashboard::dashboard();
    let mut state = TuiState::new(dashboard.sections.len());
    render(terminal, &dashboard, &state, launch_context, color)?;
    loop {
        let input = match event::read()? {
            Event::Key(key) => input_from_key(key, state.search_active()),
            Event::Mouse(mouse) => input_from_mouse(mouse, terminal.size()?.into()),
            Event::Resize(_, _) => Some(TuiInput::Resize),
            _ => None,
        };
        if let Some(input) = input {
            match state.apply(input) {
                TuiAction::Quit => break,
                TuiAction::CheckUpdates => {
                    dashboard.check_updates();
                }
                TuiAction::Refresh => {
                    let selected_section = state.selected_section;
                    let selected_detail_row = state.selected_detail_row;
                    let selected_command = state.selected_command;
                    let focus_region = state.focus_region;
                    let software_view = state.software_view().clone();
                    dashboard = tui_dashboard::dashboard();
                    state = TuiState::new(dashboard.sections.len());
                    state.selected_section = selected_section;
                    state.selected_detail_row = selected_detail_row;
                    state.selected_command = selected_command;
                    state.focus_region = focus_region;
                    state.set_software_view(software_view);
                }
                TuiAction::Continue => {}
            }
            dashboard.apply_software_view(state.software_view());
            let row_count = dashboard
                .sections
                .get(state.selected_section)
                .map_or(0, |section| section.rows.len());
            state.clamp_detail_row(row_count);
            render(terminal, &dashboard, &state, launch_context, color)?;
        }
    }
    Ok(())
}

fn render<B: Backend<Error = io::Error>>(
    terminal: &mut Terminal<B>,
    dashboard: &tui_dashboard::TuiDashboard,
    state: &TuiState,
    _launch_context: &LaunchRoutingReport,
    color: bool,
) -> io::Result<()> {
    draw_dashboard(terminal, dashboard, state, color)
}

fn input_from_key(key: KeyEvent, search_active: bool) -> Option<TuiInput> {
    if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return None;
    }
    if search_active {
        return Some(match key.code {
            KeyCode::Esc => TuiInput::Back,
            KeyCode::Enter => TuiInput::EndSearch,
            KeyCode::Backspace => TuiInput::SearchBackspace,
            KeyCode::Char(value) => TuiInput::SearchCharacter(value),
            _ => TuiInput::Other,
        });
    }
    if key.kind == KeyEventKind::Repeat
        && matches!(
            key.code,
            KeyCode::Char('r') | KeyCode::Char('R') | KeyCode::Char('u') | KeyCode::Char('U')
        )
    {
        return Some(TuiInput::Other);
    }
    Some(match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => TuiInput::Quit,
        KeyCode::Esc => TuiInput::Back,
        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => TuiInput::ToggleHelp,
        KeyCode::Char('j') | KeyCode::Char('J') => TuiInput::NextItem,
        KeyCode::Char('k') | KeyCode::Char('K') => TuiInput::PreviousItem,
        KeyCode::Home => TuiInput::FirstSection,
        KeyCode::End => TuiInput::LastSection,
        KeyCode::Tab => TuiInput::FocusNext,
        KeyCode::BackTab => TuiInput::FocusPrevious,
        KeyCode::Enter | KeyCode::Char(' ') => TuiInput::Activate,
        KeyCode::Char('r') | KeyCode::Char('R') => TuiInput::Refresh,
        KeyCode::Char('u') | KeyCode::Char('U') => TuiInput::CheckUpdates,
        KeyCode::Char('/') => TuiInput::BeginSearch,
        KeyCode::Char('f') | KeyCode::Char('F') => TuiInput::FilterNext,
        KeyCode::Char('s') | KeyCode::Char('S') => TuiInput::SortNext,
        KeyCode::Down | KeyCode::Right => TuiInput::NextItem,
        KeyCode::Up | KeyCode::Left => TuiInput::PreviousItem,
        _ => TuiInput::Other,
    })
}

fn input_from_mouse(mouse: MouseEvent, size: Rect) -> Option<TuiInput> {
    let target = mouse_target(mouse.column, mouse.row, size);
    match mouse.kind {
        MouseEventKind::ScrollUp => Some(TuiInput::ScrollUp(target)),
        MouseEventKind::ScrollDown => Some(TuiInput::ScrollDown(target)),
        _ => None,
    }
}

fn mouse_target(column: u16, row: u16, size: Rect) -> TuiMouseTarget {
    let tier = TuiLayoutTier::from_size(size.width, size.height);
    if tier == TuiLayoutTier::VerySmall {
        return TuiMouseTarget::Details;
    }

    let body_top = 4u16;
    let help = help_height(&TuiState::new(1), size);
    let body_height = size
        .height
        .saturating_sub(body_top)
        .saturating_sub(help)
        .saturating_sub(3);
    if tier == TuiLayoutTier::Wide || size.width >= 92 {
        if column < 26 {
            TuiMouseTarget::Navigation
        } else if row >= body_top.saturating_add(body_height.saturating_sub(7)) {
            TuiMouseTarget::Commands
        } else {
            TuiMouseTarget::Details
        }
    } else if row < body_top.saturating_add(8) {
        TuiMouseTarget::Navigation
    } else {
        TuiMouseTarget::Details
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter<W: Write>(output: &mut W) -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(err) = execute!(output, EnterAlternateScreen, EnableMouseCapture, Hide) {
            let _ = disable_raw_mode();
            return Err(err);
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            Show,
            DisableMouseCapture,
            LeaveAlternateScreen
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launch_routing::{LaunchEnvironment, resolve_launch_mode};
    use crate::tui_render::render_dashboard_with_state;
    use crossterm::event::{KeyModifiers, MouseEvent, MouseEventKind};

    #[test]
    fn q_key_maps_to_quit_without_printable_output() {
        let input = input_from_key(KeyEvent::from(KeyCode::Char('q')), false);
        assert_eq!(input, Some(TuiInput::Quit));
    }

    #[test]
    fn help_and_navigation_keys_are_supported() {
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('?')), false),
            Some(TuiInput::ToggleHelp)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('r')), false),
            Some(TuiInput::Refresh)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('u')), false),
            Some(TuiInput::CheckUpdates)
        );
        assert_eq!(
            input_from_key(
                KeyEvent::new_with_kind(
                    KeyCode::Char('r'),
                    KeyModifiers::NONE,
                    KeyEventKind::Repeat,
                ),
                false
            ),
            Some(TuiInput::Other)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Tab), false),
            Some(TuiInput::FocusNext)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Up), false),
            Some(TuiInput::PreviousItem)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('j')), false),
            Some(TuiInput::NextItem)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('k')), false),
            Some(TuiInput::PreviousItem)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Home), false),
            Some(TuiInput::FirstSection)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::End), false),
            Some(TuiInput::LastSection)
        );
    }

    #[test]
    fn mouse_wheel_targets_the_list_under_the_pointer() {
        let details = input_from_mouse(
            MouseEvent {
                kind: MouseEventKind::ScrollDown,
                column: 80,
                row: 12,
                modifiers: KeyModifiers::NONE,
            },
            Rect::new(0, 0, 120, 34),
        );
        assert_eq!(details, Some(TuiInput::ScrollDown(TuiMouseTarget::Details)));

        let navigation = input_from_mouse(
            MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: 5,
                row: 12,
                modifiers: KeyModifiers::NONE,
            },
            Rect::new(0, 0, 120, 34),
        );
        assert_eq!(
            navigation,
            Some(TuiInput::ScrollUp(TuiMouseTarget::Navigation))
        );
    }

    #[test]
    fn search_keys_are_text_input_only_while_search_is_active() {
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('q')), true),
            Some(TuiInput::SearchCharacter('q'))
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Backspace), true),
            Some(TuiInput::SearchBackspace)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Enter), true),
            Some(TuiInput::EndSearch)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Esc), true),
            Some(TuiInput::Back)
        );
    }

    #[test]
    fn key_release_events_do_not_move_selection_twice() {
        let release =
            KeyEvent::new_with_kind(KeyCode::Down, KeyModifiers::NONE, KeyEventKind::Release);
        let repeat =
            KeyEvent::new_with_kind(KeyCode::Down, KeyModifiers::NONE, KeyEventKind::Repeat);
        assert_eq!(input_from_key(release, false), None);
        assert_eq!(input_from_key(repeat, false), Some(TuiInput::NextItem));
    }

    #[test]
    fn activation_and_back_keys_are_read_only_navigation_inputs() {
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Enter), false),
            Some(TuiInput::Activate)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char(' ')), false),
            Some(TuiInput::Activate)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Esc), false),
            Some(TuiInput::Back)
        );
    }

    #[test]
    fn scripted_render_contains_no_typed_q_echo() {
        let args: Vec<String> = Vec::new();
        let launch = resolve_launch_mode(
            &args,
            LaunchEnvironment {
                stdin_is_tty: true,
                stdout_is_tty: true,
                tui_available: true,
                automation_detected: false,
            },
        );
        let dashboard = tui_dashboard::dashboard();
        let state = TuiState::new(dashboard.sections.len());
        let frame = render_dashboard_with_state(&dashboard, false, 80, 24, &state);
        assert!(frame.contains("runtime.zero"));
        assert!(frame.contains("keys: q quit"));
        assert!(!frame.contains("q\n"));
        assert_eq!(
            launch.launch_mode,
            crate::launch_routing::LaunchMode::TuiDashboard
        );
    }
}
