//! Terminal lifecycle and event translation for the new presentation path.

use std::io::{self, Write};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::Duration;

use crossterm::cursor::{Hide, Show};
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind,
    KeyModifiers, MouseEvent, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::{Backend, CrosstermBackend};
use ratatui::layout::Rect;

use super::foundation_adapter;
use super::layout::LayoutPlan;
use super::messages::UiIntent;
use super::model::Route;
use super::screens;
use super::state::{FocusRegion, UiState};
use crate::launch_routing::LaunchRoutingReport;
use crate::tui_dashboard;

type SnapshotResult = (u64, Result<tui_dashboard::TuiDashboard, String>);

pub fn run_interactive_tui(_launch_context: &LaunchRoutingReport, color: bool) -> io::Result<()> {
    let mut stdout = io::stdout();
    let _terminal_guard = TerminalGuard::enter(&mut stdout)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    run_event_loop(&mut terminal, color)
}

fn run_event_loop<B: Backend<Error = io::Error>>(
    terminal: &mut Terminal<B>,
    color: bool,
) -> io::Result<()> {
    let mut generation = 0u64;
    let (mut controller, receiver) = start_snapshot_load(generation);
    let mut receiver = Some(receiver);
    let mut state = UiState::new(foundation_adapter::loading_model(generation));
    draw(terminal, &state, color)?;

    loop {
        if let Some((result_generation, result)) = poll_snapshot(&mut receiver, generation) {
            if result_generation == generation {
                match result {
                    Ok(dashboard) => {
                        state.apply_model(foundation_adapter::model_from_dashboard(
                            &dashboard, generation,
                        ));
                    }
                    Err(reason) => state.mark_snapshot_unavailable(
                        generation,
                        super::model::BoundedText::try_new(reason)
                            .unwrap_or_else(|_| super::model::BoundedText::redacted()),
                    ),
                }
                draw(terminal, &state, color)?;
            }
        }

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let event = event::read()?;
        let Some(intent) = intent_from_event(event, terminal.size()?.into(), &state) else {
            continue;
        };
        if matches!(intent, UiIntent::Quit) {
            controller.cancel(rz0_cancellation_contract::CancellationReason::UserRequested);
            break;
        }
        if matches!(intent, UiIntent::Refresh) {
            controller.cancel(rz0_cancellation_contract::CancellationReason::UserRequested);
            generation = generation.wrapping_add(1);
            let (new_controller, new_receiver) = start_snapshot_load(generation);
            controller = new_controller;
            receiver = Some(new_receiver);
            state.apply_model(foundation_adapter::loading_model(generation));
        } else {
            state.apply(intent);
        }
        draw(terminal, &state, color)?;
    }
    Ok(())
}

fn draw<B: Backend<Error = io::Error>>(
    terminal: &mut Terminal<B>,
    state: &UiState,
    color: bool,
) -> io::Result<()> {
    terminal
        .draw(|frame| screens::draw(frame, state, color))
        .map(|_| ())
}

fn start_snapshot_load(
    generation: u64,
) -> (
    rz0_cancellation_contract::CancellationController,
    Receiver<SnapshotResult>,
) {
    let (controller, cancellation) = rz0_cancellation_contract::cancellation_pair();
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = tui_dashboard::dashboard_cancellable(&cancellation);
        let _ = sender.send((generation, result));
    });
    (controller, receiver)
}

fn poll_snapshot(
    receiver: &mut Option<Receiver<SnapshotResult>>,
    generation: u64,
) -> Option<SnapshotResult> {
    match receiver.as_ref().map(Receiver::try_recv) {
        Some(Ok(result)) => {
            *receiver = None;
            Some(result)
        }
        Some(Err(TryRecvError::Disconnected)) => {
            *receiver = None;
            Some((generation, Err("snapshot worker disconnected".to_string())))
        }
        Some(Err(TryRecvError::Empty)) | None => None,
    }
}

fn intent_from_event(event: Event, area: Rect, state: &UiState) -> Option<UiIntent> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => intent_from_key(key, state),
        Event::Mouse(mouse) => intent_from_mouse(mouse, area, state),
        Event::Resize(_, _) => None,
        _ => None,
    }
}

fn intent_from_key(key: KeyEvent, state: &UiState) -> Option<UiIntent> {
    if state.search_active {
        return match key.code {
            KeyCode::Esc => Some(UiIntent::Back),
            KeyCode::Enter => Some(UiIntent::AcceptSearch),
            KeyCode::Backspace => Some(UiIntent::SearchBackspace),
            KeyCode::Char(character) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(UiIntent::SearchCharacter(character))
            }
            _ => None,
        };
    }
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => Some(UiIntent::Quit),
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(UiIntent::Quit),
        KeyCode::Esc => Some(UiIntent::Back),
        KeyCode::Tab => Some(UiIntent::FocusNext),
        KeyCode::BackTab => Some(UiIntent::FocusPrevious),
        KeyCode::Up | KeyCode::Char('k') => Some(UiIntent::SelectPrevious),
        KeyCode::Down | KeyCode::Char('j') => Some(UiIntent::SelectNext),
        KeyCode::Home => Some(UiIntent::SelectFirst),
        KeyCode::End => Some(UiIntent::SelectLast),
        KeyCode::Enter if state.focus == FocusRegion::Detail => Some(UiIntent::ReviewSelected),
        KeyCode::Enter if state.focus == FocusRegion::Primary => Some(UiIntent::OpenDetail),
        KeyCode::Char('u') | KeyCode::Char('U') => Some(UiIntent::ReviewSelected),
        KeyCode::Char('/') => Some(UiIntent::BeginSearch),
        KeyCode::Char('?') | KeyCode::Char('h') | KeyCode::Char('H') => Some(UiIntent::ToggleHelp),
        KeyCode::Char('r') | KeyCode::Char('R') => Some(UiIntent::Refresh),
        KeyCode::Char(character) => Route::ALL
            .iter()
            .find(|route| route.number().to_string().chars().next() == Some(character))
            .copied()
            .map(UiIntent::Navigate),
        _ => None,
    }
}

fn intent_from_mouse(mouse: MouseEvent, area: Rect, _state: &UiState) -> Option<UiIntent> {
    match mouse.kind {
        MouseEventKind::ScrollUp => Some(UiIntent::SelectPrevious),
        MouseEventKind::ScrollDown => Some(UiIntent::SelectNext),
        MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
            let plan = LayoutPlan::for_area(area);
            let x = mouse.column;
            let y = mouse.row;
            if plan.routes.contains((x, y).into()) {
                let route_index = usize::from(y.saturating_sub(plan.routes.y));
                return Route::ALL.get(route_index).copied().map(UiIntent::Navigate);
            }
            if plan.primary.contains((x, y).into()) {
                let index = usize::from(y.saturating_sub(plan.primary.y + 1));
                return Some(UiIntent::SelectIndex(index));
            }
            if plan.detail.contains((x, y).into()) {
                return Some(UiIntent::ReviewSelected);
            }
            None
        }
        _ => None,
    }
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter<W: Write>(output: &mut W) -> io::Result<Self> {
        enable_raw_mode()?;
        if let Err(error) = execute!(output, EnterAlternateScreen, EnableMouseCapture, Hide) {
            let _ = disable_raw_mode();
            return Err(error);
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
    use super::super::state::Overlay;
    use super::*;
    use crossterm::event::{KeyModifiers, MouseButton};

    #[test]
    fn keyboard_flow_is_read_only_and_returns_safely() {
        let model = super::super::testkit::fixture_model();
        let mut state = UiState::new(model);
        state.focus = FocusRegion::Primary;
        state.apply(intent_from_key(KeyEvent::from(KeyCode::Enter), &state).expect("detail"));
        assert_eq!(state.overlay, Overlay::Detail);
        state.focus = FocusRegion::Detail;
        state.apply(intent_from_key(KeyEvent::from(KeyCode::Enter), &state).expect("review"));
        assert!(matches!(state.overlay, Overlay::ActionReview(_)));
        state.apply(UiIntent::Back);
        assert_eq!(state.overlay, Overlay::None);
        assert!(matches!(state.job, super::super::model::JobState::Idle));
    }

    #[test]
    fn key_mapping_covers_search_quit_and_navigation() {
        let state = UiState::new(super::super::testkit::fixture_model());
        assert_eq!(
            intent_from_key(KeyEvent::from(KeyCode::Char('/')), &state),
            Some(UiIntent::BeginSearch)
        );
        assert_eq!(
            intent_from_key(KeyEvent::from(KeyCode::Char('q')), &state),
            Some(UiIntent::Quit)
        );
        assert_eq!(
            intent_from_key(KeyEvent::from(KeyCode::Char('5')), &state),
            Some(UiIntent::Navigate(Route::Modules))
        );
        assert_eq!(
            intent_from_key(
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                &state
            ),
            Some(UiIntent::Quit)
        );
    }

    #[test]
    fn mouse_navigation_is_bounded_to_named_regions() {
        let state = UiState::new(super::super::testkit::fixture_model());
        let area = Rect::new(0, 0, 118, 30);
        let plan = LayoutPlan::for_area(area);
        let route_event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 3,
            row: plan.routes.y,
            modifiers: KeyModifiers::empty(),
        };
        assert_eq!(
            intent_from_mouse(route_event, area, &state),
            Some(UiIntent::Navigate(Route::Overview))
        );
    }

    #[test]
    fn published_snapshot_receiver_is_consumed_once_per_generation() {
        let (sender, receiver) = mpsc::channel();
        let snapshot: SnapshotResult = (7, Ok(tui_dashboard::dashboard()));
        sender.send(snapshot).expect("snapshot");
        drop(sender);
        let mut receiver = Some(receiver);
        assert!(poll_snapshot(&mut receiver, 7).is_some());
        assert!(poll_snapshot(&mut receiver, 7).is_none());
    }
}
