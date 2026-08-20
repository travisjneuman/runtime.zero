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

use crate::launch_routing::LaunchRoutingReport;
use crate::tui_dashboard;
use crate::tui_layout::TuiLayoutTier;
use crate::tui_ratatui::draw_dashboard;
use crate::tui_state::{TuiAction, TuiInput, TuiMouseTarget, TuiState};
use crate::update_cli::TuiUpdateChallenge;
use crate::update_execution::UpdateExecutionReport;
use crate::updates::LiveUpdateReview;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdatePhase {
    Check,
    Prepare,
    Execute,
}

type DashboardLoadResult = Result<tui_dashboard::TuiDashboard, String>;

enum TuiUpdateResult {
    Review(Result<LiveUpdateReview, String>),
    Challenge(Result<TuiUpdateChallenge, String>),
    Execution(Result<UpdateExecutionReport, String>),
}

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
    let mut dashboard = tui_dashboard::loading_dashboard();
    let mut state = TuiState::new(dashboard.sections.len());
    let mut dashboard_receiver = Some(start_dashboard_load());
    let mut update_receiver: Option<Receiver<TuiUpdateResult>> = None;
    let mut update_controller = None;
    let mut update_phase = None;
    let mut pending_update_selection = None;
    render(terminal, &dashboard, &state, launch_context, color)?;
    loop {
        if let Some(result) = poll_dashboard_result(&mut dashboard_receiver) {
            match result {
                Ok(loaded) => dashboard = loaded,
                Err(detail) => tui_dashboard::mark_startup_load_failed(&mut dashboard, &detail),
            }
            dashboard.apply_software_view(state.software_view());
            let row_count = dashboard
                .sections
                .get(state.selected_section)
                .map_or(0, |section| section.rows.len());
            state.clamp_detail_row(row_count);
            render(terminal, &dashboard, &state, launch_context, color)?;
        }
        if let Some(result) = poll_update_result(&mut update_receiver, update_phase) {
            finish_update_result(
                &mut dashboard,
                &mut state,
                result,
                &mut update_receiver,
                &mut update_controller,
                &mut update_phase,
                &mut pending_update_selection,
            );
            dashboard.apply_software_view(state.software_view());
            render(terminal, &dashboard, &state, launch_context, color)?;
        }
        let input = if event::poll(Duration::from_secs(1))? {
            match event::read()? {
                Event::Key(key) => input_from_key(
                    key,
                    state.search_active(),
                    state.update_confirmation_active(),
                ),
                Event::Mouse(mouse) => input_from_mouse(mouse, terminal.size()?.into()),
                Event::Resize(_, _) => Some(TuiInput::Resize),
                _ => None,
            }
        } else {
            Some(TuiInput::RefreshMonitor)
        };
        if let Some(input) = input {
            let confirmation_was_active = state.update_confirmation_active();
            match state.apply(input) {
                TuiAction::Quit => {
                    if update_phase.is_some() {
                        if let Some(controller) = update_controller.as_ref() {
                            controller.cancel(
                                rz0_cancellation_contract::CancellationReason::UserRequested,
                            );
                        }
                        dashboard.cancel_update_action();
                    }
                    break;
                }
                TuiAction::CheckUpdates => {
                    pending_update_selection = None;
                    start_update_check(
                        &mut dashboard,
                        &mut update_receiver,
                        &mut update_controller,
                        &mut update_phase,
                    );
                }
                TuiAction::UpdateSelected => {
                    start_selected_update(
                        &mut dashboard,
                        &state,
                        &mut update_receiver,
                        &mut update_controller,
                        &mut update_phase,
                        &mut pending_update_selection,
                    );
                }
                TuiAction::SubmitUpdateConfirmation => {
                    let phrase = state.finish_update_confirmation();
                    start_update_execution(
                        &mut dashboard,
                        phrase,
                        &mut update_receiver,
                        &mut update_controller,
                        &mut update_phase,
                    );
                }
                TuiAction::RefreshMonitor => {
                    dashboard.refresh_monitor();
                }
                TuiAction::Refresh => {
                    let selected_section = state.selected_section;
                    let selected_detail_row = state.selected_detail_row;
                    let focus_region = state.focus_region;
                    let software_view = state.software_view().clone();
                    dashboard = tui_dashboard::loading_dashboard();
                    state = TuiState::new(dashboard.sections.len());
                    state.selected_section = selected_section;
                    state.selected_detail_row = selected_detail_row;
                    state.focus_region = focus_region;
                    state.set_software_view(software_view);
                    if dashboard_receiver.is_none() {
                        dashboard_receiver = Some(start_dashboard_load());
                    }
                }
                TuiAction::Continue => {}
            }
            if confirmation_was_active && matches!(input, TuiInput::Back) {
                dashboard.cancel_update_action();
            }
            if let Some(result) = poll_update_result(&mut update_receiver, update_phase) {
                finish_update_result(
                    &mut dashboard,
                    &mut state,
                    result,
                    &mut update_receiver,
                    &mut update_controller,
                    &mut update_phase,
                    &mut pending_update_selection,
                );
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

fn start_dashboard_load() -> Receiver<DashboardLoadResult> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let _ = sender.send(Ok(tui_dashboard::dashboard()));
    });
    receiver
}

fn poll_dashboard_result(
    receiver: &mut Option<Receiver<DashboardLoadResult>>,
) -> Option<DashboardLoadResult> {
    match receiver.as_ref().map(Receiver::try_recv) {
        Some(Ok(result)) => {
            *receiver = None;
            Some(result)
        }
        Some(Err(TryRecvError::Disconnected)) => {
            *receiver = None;
            Some(Err("dashboard worker disconnected".to_string()))
        }
        Some(Err(TryRecvError::Empty)) | None => None,
    }
}

fn poll_update_result(
    receiver: &mut Option<Receiver<TuiUpdateResult>>,
    phase: Option<UpdatePhase>,
) -> Option<TuiUpdateResult> {
    match receiver.as_ref().map(Receiver::try_recv) {
        Some(Ok(result)) => Some(result),
        Some(Err(TryRecvError::Disconnected)) => Some(match phase {
            Some(UpdatePhase::Check) | None => {
                TuiUpdateResult::Review(Err("update worker disconnected".to_string()))
            }
            Some(UpdatePhase::Prepare) => TuiUpdateResult::Challenge(Err(
                "update preparation worker disconnected".to_string(),
            )),
            Some(UpdatePhase::Execute) => {
                TuiUpdateResult::Execution(Err("update execution worker disconnected".to_string()))
            }
        }),
        Some(Err(TryRecvError::Empty)) | None => None,
    }
}

fn start_update_check(
    dashboard: &mut tui_dashboard::TuiDashboard,
    receiver: &mut Option<Receiver<TuiUpdateResult>>,
    controller: &mut Option<rz0_cancellation_contract::CancellationController>,
    phase: &mut Option<UpdatePhase>,
) {
    if receiver.is_some() {
        return;
    }
    let Some(catalog) = dashboard.start_update_check() else {
        return;
    };
    let (new_controller, cancellation) = rz0_cancellation_contract::cancellation_pair();
    let (sender, new_receiver) = mpsc::channel();
    thread::spawn(move || {
        let result =
            crate::updates::collect_live_update_review_cancellable(&catalog, Some(&cancellation));
        let _ = sender.send(TuiUpdateResult::Review(result));
    });
    *controller = Some(new_controller);
    *receiver = Some(new_receiver);
    *phase = Some(UpdatePhase::Check);
}

fn start_selected_update(
    dashboard: &mut tui_dashboard::TuiDashboard,
    state: &TuiState,
    receiver: &mut Option<Receiver<TuiUpdateResult>>,
    controller: &mut Option<rz0_cancellation_contract::CancellationController>,
    phase: &mut Option<UpdatePhase>,
    pending_selection: &mut Option<String>,
) {
    if receiver.is_some() || dashboard.pending_update_challenge().is_some() {
        return;
    }
    let selected_id = dashboard.selected_software_id(
        state.selected_section,
        state.selected_detail_row,
        state.software_view(),
    );
    if let Some(action) = dashboard.selected_update_action(
        state.selected_section,
        state.selected_detail_row,
        state.software_view(),
    ) {
        if action.disposition != rz0_action_plan::ActionDisposition::Planned {
            dashboard.update_action_unavailable(&format!(
                "selected update is currently {:?}",
                action.disposition
            ));
            return;
        }
        dashboard.start_update_prepare(&action);
        start_update_prepare(action.action_id, receiver, controller, phase);
        return;
    }
    if dashboard.has_update_review() {
        dashboard
            .update_action_unavailable("selected item has no current provider update candidate");
        return;
    }
    *pending_selection = selected_id;
    start_update_check(dashboard, receiver, controller, phase);
    if pending_selection.is_none() && receiver.is_none() {
        dashboard.update_action_unavailable("select an installed software or provider row first");
    }
}

fn start_update_prepare(
    action_id: String,
    receiver: &mut Option<Receiver<TuiUpdateResult>>,
    controller: &mut Option<rz0_cancellation_contract::CancellationController>,
    phase: &mut Option<UpdatePhase>,
) {
    if receiver.is_some() {
        return;
    }
    let (new_controller, cancellation) = rz0_cancellation_contract::cancellation_pair();
    let (sender, new_receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = crate::update_cli::prepare_tui_update(&action_id, Some(&cancellation));
        let _ = sender.send(TuiUpdateResult::Challenge(result));
    });
    *controller = Some(new_controller);
    *receiver = Some(new_receiver);
    *phase = Some(UpdatePhase::Prepare);
}

fn start_update_execution(
    dashboard: &mut tui_dashboard::TuiDashboard,
    phrase: String,
    receiver: &mut Option<Receiver<TuiUpdateResult>>,
    controller: &mut Option<rz0_cancellation_contract::CancellationController>,
    phase: &mut Option<UpdatePhase>,
) {
    if receiver.is_some() {
        return;
    }
    let Some(prepared) = dashboard.pending_update_challenge().cloned() else {
        dashboard.update_action_unavailable("no pending update confirmation exists");
        return;
    };
    let (new_controller, cancellation) = rz0_cancellation_contract::cancellation_pair();
    let (sender, new_receiver) = mpsc::channel();
    dashboard.begin_update_execution();
    thread::spawn(move || {
        let result = crate::update_cli::execute_tui_update(prepared, &phrase, &cancellation);
        let _ = sender.send(TuiUpdateResult::Execution(result));
    });
    *controller = Some(new_controller);
    *receiver = Some(new_receiver);
    *phase = Some(UpdatePhase::Execute);
}

fn finish_update_result(
    dashboard: &mut tui_dashboard::TuiDashboard,
    state: &mut TuiState,
    result: TuiUpdateResult,
    receiver: &mut Option<Receiver<TuiUpdateResult>>,
    controller: &mut Option<rz0_cancellation_contract::CancellationController>,
    phase: &mut Option<UpdatePhase>,
    pending_selection: &mut Option<String>,
) {
    *receiver = None;
    match result {
        TuiUpdateResult::Review(result) => match result {
            Ok(review) => {
                dashboard.complete_update_review(review);
                *controller = None;
                *phase = None;
                if let Some(selected_id) = pending_selection.take() {
                    if let Some(action) = dashboard.update_action_for_software_id(&selected_id) {
                        if action.disposition == rz0_action_plan::ActionDisposition::Planned {
                            dashboard.start_update_prepare(&action);
                            start_update_prepare(action.action_id, receiver, controller, phase);
                        } else {
                            dashboard.update_action_unavailable(&format!(
                                "selected update is currently {:?}",
                                action.disposition
                            ));
                        }
                    } else {
                        dashboard.update_action_unavailable(
                            "selected item has no current provider update candidate",
                        );
                    }
                }
            }
            Err(error) => {
                *controller = None;
                *phase = None;
                pending_selection.take();
                dashboard.fail_update_check_with_error(&error);
            }
        },
        TuiUpdateResult::Challenge(result) => {
            *controller = None;
            *phase = None;
            match result {
                Ok(challenge) => {
                    dashboard.complete_update_challenge(challenge);
                    state.begin_update_confirmation();
                }
                Err(error) => dashboard.fail_update_action(&error),
            }
        }
        TuiUpdateResult::Execution(result) => {
            *phase = None;
            *controller = None;
            match result {
                Ok(report) => dashboard.complete_update_execution(&report),
                Err(error) => dashboard.fail_update_action(&error),
            }
        }
    }
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

fn input_from_key(
    key: KeyEvent,
    search_active: bool,
    update_confirmation_active: bool,
) -> Option<TuiInput> {
    if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return None;
    }
    if update_confirmation_active {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return Some(TuiInput::Back);
        }
        return Some(match key.code {
            KeyCode::Esc => TuiInput::Back,
            KeyCode::Enter => TuiInput::SubmitUpdateConfirmation,
            KeyCode::Backspace => TuiInput::ConfirmBackspace,
            KeyCode::Char(value) => TuiInput::ConfirmCharacter(value),
            _ => TuiInput::Other,
        });
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
        KeyCode::Char('u') => TuiInput::CheckUpdates,
        KeyCode::Char('U') => TuiInput::UpdateSelected,
        KeyCode::Char('m') | KeyCode::Char('M') => TuiInput::OpenMonitor,
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

    let body_top = 3u16;
    let body_height = size.height.saturating_sub(5);
    if row < body_top {
        return TuiMouseTarget::Navigation;
    }
    if tier == TuiLayoutTier::Wide || size.width >= 82 {
        let detail_width = if tier == TuiLayoutTier::Wide { 38 } else { 32 };
        if column >= size.width.saturating_sub(detail_width) {
            TuiMouseTarget::Context
        } else {
            TuiMouseTarget::Details
        }
    } else if row >= body_top.saturating_add(body_height.saturating_sub(6)) {
        TuiMouseTarget::Context
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
        let input = input_from_key(KeyEvent::from(KeyCode::Char('q')), false, false);
        assert_eq!(input, Some(TuiInput::Quit));
    }

    #[test]
    fn help_and_navigation_keys_are_supported() {
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('?')), false, false),
            Some(TuiInput::ToggleHelp)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('r')), false, false),
            Some(TuiInput::Refresh)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('u')), false, false),
            Some(TuiInput::CheckUpdates)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('U')), false, false),
            Some(TuiInput::UpdateSelected)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('m')), false, false),
            Some(TuiInput::OpenMonitor)
        );
        assert_eq!(
            input_from_key(
                KeyEvent::new_with_kind(
                    KeyCode::Char('r'),
                    KeyModifiers::NONE,
                    KeyEventKind::Repeat,
                ),
                false,
                false,
            ),
            Some(TuiInput::Other)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Tab), false, false),
            Some(TuiInput::FocusNext)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Up), false, false),
            Some(TuiInput::PreviousItem)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('j')), false, false),
            Some(TuiInput::NextItem)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('k')), false, false),
            Some(TuiInput::PreviousItem)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Home), false, false),
            Some(TuiInput::FirstSection)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::End), false, false),
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

        let primary = input_from_mouse(
            MouseEvent {
                kind: MouseEventKind::ScrollUp,
                column: 5,
                row: 12,
                modifiers: KeyModifiers::NONE,
            },
            Rect::new(0, 0, 120, 34),
        );
        assert_eq!(primary, Some(TuiInput::ScrollUp(TuiMouseTarget::Details)));
    }

    #[test]
    fn search_keys_are_text_input_only_while_search_is_active() {
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('q')), true, false),
            Some(TuiInput::SearchCharacter('q'))
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Backspace), true, false),
            Some(TuiInput::SearchBackspace)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Enter), true, false),
            Some(TuiInput::EndSearch)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Esc), true, false),
            Some(TuiInput::Back)
        );
    }

    #[test]
    fn confirmation_keys_are_text_input_only_until_submit() {
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char('q')), false, true),
            Some(TuiInput::ConfirmCharacter('q'))
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Backspace), false, true),
            Some(TuiInput::ConfirmBackspace)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Enter), false, true),
            Some(TuiInput::SubmitUpdateConfirmation)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Esc), false, true),
            Some(TuiInput::Back)
        );
    }

    #[test]
    fn key_release_events_do_not_move_selection_twice() {
        let release =
            KeyEvent::new_with_kind(KeyCode::Down, KeyModifiers::NONE, KeyEventKind::Release);
        let repeat =
            KeyEvent::new_with_kind(KeyCode::Down, KeyModifiers::NONE, KeyEventKind::Repeat);
        assert_eq!(input_from_key(release, false, false), None);
        assert_eq!(
            input_from_key(repeat, false, false),
            Some(TuiInput::NextItem)
        );
    }

    #[test]
    fn activation_and_back_keys_are_read_only_navigation_inputs() {
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Enter), false, false),
            Some(TuiInput::Activate)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Char(' ')), false, false),
            Some(TuiInput::Activate)
        );
        assert_eq!(
            input_from_key(KeyEvent::from(KeyCode::Esc), false, false),
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
        assert!(frame.contains("q quit"));
        assert!(!frame.contains("q\n"));
        assert_eq!(
            launch.launch_mode,
            crate::launch_routing::LaunchMode::TuiDashboard
        );
    }
}
