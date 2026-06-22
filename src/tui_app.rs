use std::io::{self, Write};

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use crossterm::execute;
use crossterm::style::Print;
use crossterm::terminal::{
    Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
    enable_raw_mode, size,
};

use crate::launch_routing::LaunchRoutingReport;
use crate::tui_dashboard;
use crate::tui_render::render_dashboard_with_state;
use crate::tui_state::{TuiAction, TuiInput, TuiState};

pub fn run_interactive_tui(launch_context: &LaunchRoutingReport) -> io::Result<()> {
    let mut stdout = io::stdout();
    let _terminal = TerminalGuard::enter(&mut stdout)?;
    run_event_loop(&mut stdout, launch_context, color_enabled())
}

fn run_event_loop<W: Write>(
    output: &mut W,
    launch_context: &LaunchRoutingReport,
    color: bool,
) -> io::Result<()> {
    let dashboard = tui_dashboard::dashboard();
    let mut state = TuiState::new(dashboard.sections.len());
    render(output, &dashboard, &state, launch_context, color)?;
    loop {
        let input = match event::read()? {
            Event::Key(key) => input_from_key(key),
            Event::Resize(_, _) => TuiInput::Resize,
            _ => TuiInput::Other,
        };
        if state.apply(input) == TuiAction::Quit {
            break;
        }
        if input != TuiInput::Other {
            render(output, &dashboard, &state, launch_context, color)?;
        }
    }
    Ok(())
}

fn render<W: Write>(
    output: &mut W,
    dashboard: &tui_dashboard::TuiDashboard,
    state: &TuiState,
    launch_context: &LaunchRoutingReport,
    color: bool,
) -> io::Result<()> {
    let (width, height) = size().unwrap_or((80, 24));
    let mut frame = render_dashboard_with_state(dashboard, color, width, height, state);
    frame.push_str(&format!(
        "launch_mode: {:?} | {}\n",
        launch_context.launch_mode, launch_context.reason
    ));
    execute!(output, MoveTo(0, 0), Clear(ClearType::All), Print(frame))?;
    output.flush()
}

fn input_from_key(key: KeyEvent) -> TuiInput {
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => TuiInput::Quit,
        KeyCode::Char('h') | KeyCode::Char('H') | KeyCode::Char('?') => TuiInput::ToggleHelp,
        KeyCode::Tab | KeyCode::Down | KeyCode::Right => TuiInput::NextSection,
        KeyCode::BackTab | KeyCode::Up | KeyCode::Left => TuiInput::PreviousSection,
        _ => TuiInput::Other,
    }
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

fn color_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::launch_routing::{LaunchEnvironment, resolve_launch_mode};

    #[test]
    fn q_key_maps_to_quit_without_printable_output() {
        let input = input_from_key(KeyEvent::from(KeyCode::Char('q')));
        assert_eq!(input, TuiInput::Quit);
    }

    #[test]
    fn help_and_navigation_keys_are_supported() {
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('?'))),
            TuiInput::ToggleHelp
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Tab)),
            TuiInput::NextSection
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Up)),
            TuiInput::PreviousSection
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
