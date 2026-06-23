use std::io::{self, Write};

use crossterm::cursor::{Hide, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::{Backend, CrosstermBackend};

use crate::launch_routing::LaunchRoutingReport;
use crate::tui_dashboard;
use crate::tui_ratatui::draw_dashboard;
use crate::tui_state::{TuiAction, TuiInput, TuiState};

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
    let dashboard = tui_dashboard::dashboard();
    let mut state = TuiState::new(dashboard.sections.len());
    render(terminal, &dashboard, &state, launch_context, color)?;
    loop {
        let input = match event::read()? {
            Event::Key(key) => input_from_key(key),
            Event::Resize(_, _) => Some(TuiInput::Resize),
            _ => None,
        };
        if let Some(input) = input {
            if state.apply(input) == TuiAction::Quit {
                break;
            }
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

fn input_from_key(key: KeyEvent) -> Option<TuiInput> {
    if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return None;
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
        KeyCode::Down | KeyCode::Right => TuiInput::NextItem,
        KeyCode::Up | KeyCode::Left => TuiInput::PreviousItem,
        _ => TuiInput::Other,
    })
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter<W: Write>(output: &mut W) -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(err) = execute!(output, EnterAlternateScreen, Hide) {
            let _ = disable_raw_mode();
            return Err(err);
        }
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launch_routing::{LaunchEnvironment, resolve_launch_mode};
    use crate::tui_render::render_dashboard_with_state;
    use crossterm::event::KeyModifiers;

    #[test]
    fn q_key_maps_to_quit_without_printable_output() {
        let input = input_from_key(KeyEvent::from(KeyCode::Char('q')));
        assert_eq!(input, Some(TuiInput::Quit));
    }

    #[test]
    fn help_and_navigation_keys_are_supported() {
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('?'))),
            Some(TuiInput::ToggleHelp)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Tab)),
            Some(TuiInput::FocusNext)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Up)),
            Some(TuiInput::PreviousItem)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('j'))),
            Some(TuiInput::NextItem)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('k'))),
            Some(TuiInput::PreviousItem)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Home)),
            Some(TuiInput::FirstSection)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::End)),
            Some(TuiInput::LastSection)
        );
    }

    #[test]
    fn key_release_events_do_not_move_selection_twice() {
        let release =
            KeyEvent::new_with_kind(KeyCode::Down, KeyModifiers::NONE, KeyEventKind::Release);
        let repeat =
            KeyEvent::new_with_kind(KeyCode::Down, KeyModifiers::NONE, KeyEventKind::Repeat);
        assert_eq!(input_from_key(release), None);
        assert_eq!(input_from_key(repeat), Some(TuiInput::NextItem));
    }

    #[test]
    fn activation_and_back_keys_are_read_only_navigation_inputs() {
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Enter)),
            Some(TuiInput::Activate)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char(' '))),
            Some(TuiInput::Activate)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Esc)),
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
