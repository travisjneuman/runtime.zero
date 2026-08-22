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
use super::model::{BoundedId, BoundedText, JobState};
use super::state::{FocusRegion, UiPage, UiState};
use super::widgets;
use crate::launch_routing::LaunchRoutingReport;
use crate::tui_dashboard;
use crate::update_cli::TuiUpdateChallenge;
use crate::update_execution::{UpdateExecutionReport, UpdateExecutionStatus};
use crate::updates::{self, LiveUpdateReview};

type SnapshotResult = (u64, Result<SnapshotData, String>);

struct SnapshotData {
    dashboard: tui_dashboard::TuiDashboard,
}

type ReviewResult = (u64, Result<LiveUpdateReview, String>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionPhase {
    Prepare,
    Execute,
}

enum ActionResult {
    Challenge(Box<Result<TuiUpdateChallenge, String>>),
    Execution(Box<Result<UpdateExecutionReport, String>>),
}

struct IntentContext<'a> {
    state: &'a mut UiState,
    dashboard: Option<&'a tui_dashboard::TuiDashboard>,
    generation: u64,
    review_controller: &'a mut Option<rz0_cancellation_contract::CancellationController>,
    review_receiver: &'a mut Option<Receiver<ReviewResult>>,
    action_controller: &'a mut Option<rz0_cancellation_contract::CancellationController>,
    action_receiver: &'a mut Option<Receiver<ActionResult>>,
    action_phase: &'a mut Option<ActionPhase>,
    pending_challenge: &'a mut Option<TuiUpdateChallenge>,
}

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
    let mut dashboard: Option<tui_dashboard::TuiDashboard> = None;
    let mut review_controller = None;
    let mut review_receiver: Option<Receiver<ReviewResult>> = None;
    let mut state = UiState::new(foundation_adapter::loading_model(generation));
    let mut action_controller = None;
    let mut action_receiver = None;
    let mut action_phase = None;
    let mut pending_challenge: Option<TuiUpdateChallenge> = None;
    draw(terminal, &state, color)?;

    loop {
        if let Some((result_generation, result)) = poll_snapshot(&mut receiver, generation)
            && result_generation == generation
        {
            match result {
                Ok(snapshot) => {
                    dashboard = Some(snapshot.dashboard);
                    state.apply_model(foundation_adapter::model_from_dashboard(
                        dashboard.as_ref().expect("dashboard snapshot"),
                        generation,
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

        if let Some(result) = poll_action_result(&mut action_receiver, action_phase) {
            finish_action_result(
                &mut state,
                result,
                &mut action_controller,
                &mut action_phase,
                &mut pending_challenge,
            );
            draw(terminal, &state, color)?;
        }

        if let Some((review_generation, result)) =
            poll_review_result(&mut review_receiver, generation)
            && review_generation == generation
        {
            finish_review_result(
                &mut state,
                dashboard.as_ref().expect("dashboard snapshot"),
                result,
                &mut review_controller,
            );
            draw(terminal, &state, color)?;
        }

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        let event = event::read()?;
        if matches!(event, Event::Resize(_, _)) {
            draw(terminal, &state, color)?;
            continue;
        }
        let Some(intent) = intent_from_event(event, terminal.size()?.into(), &state) else {
            continue;
        };
        if matches!(intent, UiIntent::Quit) {
            controller.cancel(rz0_cancellation_contract::CancellationReason::UserRequested);
            cancel_action(
                &mut action_controller,
                &mut action_receiver,
                &mut action_phase,
                &mut pending_challenge,
                &mut state,
            );
            cancel_review(&mut review_controller, &mut review_receiver, &mut state);
            break;
        }
        if matches!(intent, UiIntent::Refresh) {
            controller.cancel(rz0_cancellation_contract::CancellationReason::UserRequested);
            cancel_action(
                &mut action_controller,
                &mut action_receiver,
                &mut action_phase,
                &mut pending_challenge,
                &mut state,
            );
            cancel_review(&mut review_controller, &mut review_receiver, &mut state);
            generation = generation.wrapping_add(1);
            let (new_controller, new_receiver) = start_snapshot_load(generation);
            controller = new_controller;
            receiver = Some(new_receiver);
            dashboard = None;
            state.apply_model(foundation_adapter::refreshing_model(generation));
        } else {
            let outward = state.apply(intent);
            handle_outward_intent(
                outward,
                IntentContext {
                    state: &mut state,
                    dashboard: dashboard.as_ref(),
                    generation,
                    review_controller: &mut review_controller,
                    review_receiver: &mut review_receiver,
                    action_controller: &mut action_controller,
                    action_receiver: &mut action_receiver,
                    action_phase: &mut action_phase,
                    pending_challenge: &mut pending_challenge,
                },
            );
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
        .draw(|frame| widgets::draw_shell(frame, state, color))
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
        let result = tui_dashboard::dashboard_cancellable(&cancellation)
            .map(|dashboard| SnapshotData { dashboard });
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

fn poll_review_result(
    receiver: &mut Option<Receiver<ReviewResult>>,
    generation: u64,
) -> Option<ReviewResult> {
    match receiver.as_ref().map(Receiver::try_recv) {
        Some(Ok(result)) => {
            *receiver = None;
            Some(result)
        }
        Some(Err(TryRecvError::Disconnected)) => {
            *receiver = None;
            Some((
                generation,
                Err("provider review worker disconnected".to_string()),
            ))
        }
        Some(Err(TryRecvError::Empty)) | None => None,
    }
}

fn start_review(
    generation: u64,
    dashboard: &tui_dashboard::TuiDashboard,
    state: &mut UiState,
    controller: &mut Option<rz0_cancellation_contract::CancellationController>,
    receiver: &mut Option<Receiver<ReviewResult>>,
) {
    if receiver.is_some() {
        return;
    }
    let Some(catalog) = dashboard.clone().start_update_check() else {
        state.apply_event(super::messages::UiEvent::ActionReviewUnavailable {
            action_id: bounded_id("provider-review/unavailable"),
            reason: bounded("software evidence is unavailable"),
        });
        return;
    };
    state.set_job(JobState::Running {
        job_id: bounded_id("provider-review"),
        phase: bounded("reading provider evidence · no writes"),
    });
    let (new_controller, cancellation) = rz0_cancellation_contract::cancellation_pair();
    let (sender, new_receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = updates::collect_macos_homebrew_update_review_cancellable(
            &catalog,
            Some(&cancellation),
        );
        let _ = sender.send((generation, result));
    });
    *controller = Some(new_controller);
    *receiver = Some(new_receiver);
}

fn finish_review_result(
    state: &mut UiState,
    dashboard: &tui_dashboard::TuiDashboard,
    result: Result<LiveUpdateReview, String>,
    controller: &mut Option<rz0_cancellation_contract::CancellationController>,
) {
    *controller = None;
    match result {
        Ok(review) => {
            let generation = state.model.generation;
            state.apply_model(foundation_adapter::model_from_dashboard_and_review(
                dashboard,
                generation,
                Some(&review),
                None,
            ));
            state.set_job(JobState::Idle);
        }
        Err(reason) => {
            state.apply_event(super::messages::UiEvent::ActionReviewUnavailable {
                action_id: bounded_id("provider-review/unavailable"),
                reason: bounded(reason),
            });
        }
    }
}

fn cancel_review(
    controller: &mut Option<rz0_cancellation_contract::CancellationController>,
    receiver: &mut Option<Receiver<ReviewResult>>,
    state: &mut UiState,
) {
    if let Some(controller) = controller.take() {
        controller.cancel(rz0_cancellation_contract::CancellationReason::UserRequested);
    }
    if receiver.take().is_some() {
        state.set_job(JobState::Cancelled {
            job_id: bounded_id("provider-review"),
            reason: bounded("user requested cancellation; no writes were attempted"),
        });
    }
}

fn poll_action_result(
    receiver: &mut Option<Receiver<ActionResult>>,
    phase: Option<ActionPhase>,
) -> Option<ActionResult> {
    match receiver.as_ref().map(Receiver::try_recv) {
        Some(Ok(result)) => {
            *receiver = None;
            Some(result)
        }
        Some(Err(TryRecvError::Disconnected)) => {
            *receiver = None;
            Some(match phase {
                Some(ActionPhase::Prepare) | None => ActionResult::Challenge(Box::new(Err(
                    "foundation confirmation worker disconnected".to_string(),
                ))),
                Some(ActionPhase::Execute) => ActionResult::Execution(Box::new(Err(
                    "foundation execution worker disconnected".to_string(),
                ))),
            })
        }
        Some(Err(TryRecvError::Empty)) | None => None,
    }
}

fn start_action_prepare(
    action_id: BoundedId,
    state: &mut UiState,
    receiver: &mut Option<Receiver<ActionResult>>,
    controller: &mut Option<rz0_cancellation_contract::CancellationController>,
    phase: &mut Option<ActionPhase>,
) {
    if receiver.is_some() {
        return;
    }
    let job_id = action_id.clone();
    state.set_job(JobState::Running {
        job_id,
        phase: bounded("preparing exact foundation confirmation"),
    });
    let (new_controller, cancellation) = rz0_cancellation_contract::cancellation_pair();
    let (sender, new_receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = crate::update_cli::prepare_tui_update(action_id.as_str(), Some(&cancellation));
        let _ = sender.send(ActionResult::Challenge(Box::new(result)));
    });
    *controller = Some(new_controller);
    *receiver = Some(new_receiver);
    *phase = Some(ActionPhase::Prepare);
}

fn start_action_execution(
    challenge: TuiUpdateChallenge,
    phrase: String,
    state: &mut UiState,
    receiver: &mut Option<Receiver<ActionResult>>,
    controller: &mut Option<rz0_cancellation_contract::CancellationController>,
    phase: &mut Option<ActionPhase>,
) {
    if receiver.is_some() {
        return;
    }
    let job_id = bounded_id(challenge.action.action_id.clone());
    state.clear_confirmation();
    state.set_job(JobState::Running {
        job_id,
        phase: bounded("executing foundation transaction"),
    });
    let (new_controller, cancellation) = rz0_cancellation_contract::cancellation_pair();
    let (sender, new_receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = crate::update_cli::execute_tui_update(challenge, &phrase, &cancellation);
        let _ = sender.send(ActionResult::Execution(Box::new(result)));
    });
    *controller = Some(new_controller);
    *receiver = Some(new_receiver);
    *phase = Some(ActionPhase::Execute);
}

fn handle_outward_intent(intent: Option<UiIntent>, context: IntentContext<'_>) {
    let IntentContext {
        state,
        dashboard,
        generation,
        review_controller,
        review_receiver,
        action_controller,
        action_receiver,
        action_phase,
        pending_challenge,
    } = context;
    match intent {
        Some(UiIntent::LoadProviderReview) => {
            if let Some(dashboard) = dashboard {
                start_review(
                    generation,
                    dashboard,
                    state,
                    review_controller,
                    review_receiver,
                );
            }
        }
        Some(UiIntent::PrepareAction(action_id)) => {
            start_action_prepare(
                action_id,
                state,
                action_receiver,
                action_controller,
                action_phase,
            );
        }
        Some(UiIntent::SubmitConfirmation) => {
            let Some(challenge) = pending_challenge.take() else {
                return;
            };
            let phrase = state.confirmation_input.clone();
            start_action_execution(
                challenge,
                phrase,
                state,
                action_receiver,
                action_controller,
                action_phase,
            );
        }
        Some(UiIntent::CancelConfirmation) => {
            cancel_action(
                action_controller,
                action_receiver,
                action_phase,
                pending_challenge,
                state,
            );
        }
        Some(UiIntent::CancelJob) => {
            cancel_action(
                action_controller,
                action_receiver,
                action_phase,
                pending_challenge,
                state,
            );
            cancel_review(review_controller, review_receiver, state);
        }
        _ => {}
    }
}

fn finish_action_result(
    state: &mut UiState,
    result: ActionResult,
    controller: &mut Option<rz0_cancellation_contract::CancellationController>,
    phase: &mut Option<ActionPhase>,
    pending_challenge: &mut Option<TuiUpdateChallenge>,
) {
    *controller = None;
    *phase = None;
    match result {
        ActionResult::Challenge(result) => match *result {
            Ok(challenge) => {
                let prompt = foundation_adapter::confirmation_prompt(&challenge);
                state.set_confirmation(prompt);
                state.set_job(JobState::Idle);
                *pending_challenge = Some(challenge);
            }
            Err(reason) => {
                let action_id = state
                    .selected_record()
                    .and_then(|record| record.action_refs.first())
                    .map(|action| action.action_id.clone())
                    .unwrap_or_else(|| bounded_id("action-review/unavailable"));
                state.apply_event(super::messages::UiEvent::ActionReviewUnavailable {
                    action_id,
                    reason: bounded(reason),
                });
            }
        },
        ActionResult::Execution(result) => match *result {
            Ok(report) => match report.status {
                UpdateExecutionStatus::Committed => {
                    state.apply_event(super::messages::UiEvent::JobSucceeded {
                        receipt: bounded_id(report.receipt_reference),
                        verification: bounded_id(report.verification),
                    });
                }
                UpdateExecutionStatus::RecoveryRequired => {
                    state.apply_event(super::messages::UiEvent::RecoveryRequired {
                        transaction: bounded_id(report.transaction_id),
                        decision: bounded("foundation recovery evidence requires review"),
                    });
                }
            },
            Err(reason) => {
                let job_id = state
                    .selected_record()
                    .and_then(|record| record.action_refs.first())
                    .map(|action| action.action_id.clone())
                    .unwrap_or_else(|| bounded_id("action/unknown"));
                state.apply_event(super::messages::UiEvent::JobFailed {
                    job_id,
                    reason: bounded(reason),
                });
            }
        },
    }
}

fn cancel_action(
    controller: &mut Option<rz0_cancellation_contract::CancellationController>,
    receiver: &mut Option<Receiver<ActionResult>>,
    phase: &mut Option<ActionPhase>,
    pending_challenge: &mut Option<TuiUpdateChallenge>,
    state: &mut UiState,
) {
    if let Some(controller) = controller.take() {
        controller.cancel(rz0_cancellation_contract::CancellationReason::UserRequested);
    }
    receiver.take();
    *phase = None;
    *pending_challenge = None;
    state.clear_confirmation();
    state.set_job(JobState::Cancelled {
        job_id: bounded_id("action/cancelled"),
        reason: bounded("user requested cancellation; no rollback was attempted"),
    });
}

fn bounded(value: impl Into<String>) -> BoundedText {
    BoundedText::try_new(value).unwrap_or_else(|_| BoundedText::redacted())
}

fn bounded_id(value: impl Into<String>) -> BoundedId {
    BoundedId::try_new(value).unwrap_or_else(|_| BoundedId::try_new("action/unknown").expect("id"))
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
    if state.confirmation.is_some() {
        return match key.code {
            KeyCode::Esc => Some(UiIntent::Back),
            KeyCode::Enter => Some(UiIntent::SubmitConfirmation),
            KeyCode::Backspace => Some(UiIntent::ConfirmationBackspace),
            KeyCode::Char(character) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                Some(UiIntent::ConfirmationCharacter(character))
            }
            _ => None,
        };
    }
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
        KeyCode::Esc if matches!(state.job, JobState::Running { .. }) => Some(UiIntent::CancelJob),
        KeyCode::Esc => Some(UiIntent::Back),
        KeyCode::Tab => Some(UiIntent::FocusNext),
        KeyCode::BackTab => Some(UiIntent::FocusPrevious),
        KeyCode::Up | KeyCode::Char('k') => Some(UiIntent::SelectPrevious),
        KeyCode::Down | KeyCode::Char('j') => Some(UiIntent::SelectNext),
        KeyCode::Home => Some(UiIntent::SelectFirst),
        KeyCode::End => Some(UiIntent::SelectLast),
        KeyCode::Enter => Some(UiIntent::OpenSelected),
        KeyCode::Char('u') | KeyCode::Char('U') => Some(UiIntent::LoadProviderReview),
        KeyCode::Char('c') | KeyCode::Char('C')
            if state.page == UiPage::Review && state.focus != FocusRegion::Footer =>
        {
            Some(UiIntent::BeginConfirmation)
        }
        KeyCode::Char('h') | KeyCode::Char('H') => Some(UiIntent::OpenHome),
        KeyCode::Char('i') | KeyCode::Char('I') => Some(UiIntent::OpenInventory),
        KeyCode::Char('a') | KeyCode::Char('A') => Some(UiIntent::OpenActivity),
        KeyCode::Char('/') => Some(UiIntent::BeginSearch),
        KeyCode::Char('?') => Some(UiIntent::ToggleHelp),
        KeyCode::Char('r') | KeyCode::Char('R') => Some(UiIntent::Refresh),
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
            if plan.primary.contains((x, y).into()) {
                let index = usize::from(y.saturating_sub(plan.primary.y + 1)) / 2;
                return Some(UiIntent::SelectIndex(index));
            }
            if plan.detail.contains((x, y).into()) {
                return Some(UiIntent::OpenSelected);
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
    use super::super::state::{Overlay, UiPage};
    use super::*;
    use crossterm::event::{KeyModifiers, MouseButton};

    #[test]
    fn keyboard_flow_is_read_only_and_returns_safely() {
        let model = super::super::testkit::fixture_model();
        let mut state = UiState::new(model);
        state
            .apply(intent_from_key(KeyEvent::from(KeyCode::Char('i')), &state).expect("inventory"));
        state.apply(UiIntent::SelectIndex(0));
        state.apply(intent_from_key(KeyEvent::from(KeyCode::Enter), &state).expect("evidence"));
        assert_eq!(state.page, UiPage::Evidence);
        state.apply(UiIntent::OpenReview);
        assert_eq!(state.page, UiPage::Review);
        state.apply(UiIntent::Back);
        assert_eq!(state.page, UiPage::Evidence);
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
            None
        );
        assert_eq!(
            intent_from_key(
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                &state
            ),
            Some(UiIntent::Quit)
        );
        assert_eq!(
            intent_from_key(KeyEvent::from(KeyCode::Char('c')), &state),
            None
        );
    }

    #[test]
    fn confirmation_key_mapping_never_treats_phrase_input_as_navigation() {
        let mut state = UiState::new(super::super::testkit::fixture_model());
        state.set_confirmation(super::super::model::ConfirmationPrompt {
            action_id: super::super::model::BoundedId::try_new("fixture/action").expect("id"),
            plan_id: super::super::model::BoundedId::try_new("fixture/plan").expect("id"),
            plan_sha256: super::super::model::BoundedText::try_new("digest").expect("text"),
            target: super::super::model::BoundedText::try_new("fixture").expect("text"),
            expected_phrase: super::super::model::BoundedText::try_new("CONFIRM").expect("text"),
            risk: super::super::model::BoundedText::try_new("medium").expect("text"),
            expires_unix_seconds: 1,
            rollback_available: true,
            manual_recovery_acknowledged: false,
        });
        assert_eq!(
            intent_from_key(KeyEvent::from(KeyCode::Char('r')), &state),
            Some(UiIntent::ConfirmationCharacter('r'))
        );
        assert_eq!(
            intent_from_key(KeyEvent::from(KeyCode::Enter), &state),
            Some(UiIntent::SubmitConfirmation)
        );
        assert_eq!(
            intent_from_key(KeyEvent::from(KeyCode::Esc), &state),
            Some(UiIntent::Back)
        );
    }

    #[test]
    fn mouse_navigation_is_bounded_to_named_regions() {
        let state = UiState::new(super::super::testkit::fixture_model());
        let area = Rect::new(0, 0, 118, 30);
        let plan = LayoutPlan::for_area(area);
        let item_event = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 3,
            row: plan.primary.y + 1,
            modifiers: KeyModifiers::empty(),
        };
        assert_eq!(
            intent_from_mouse(item_event, area, &state),
            Some(UiIntent::SelectIndex(0))
        );
    }

    #[test]
    fn published_snapshot_receiver_is_consumed_once_per_generation() {
        let (sender, receiver) = mpsc::channel();
        let snapshot: SnapshotResult = (
            7,
            Ok(SnapshotData {
                dashboard: tui_dashboard::dashboard(),
            }),
        );
        sender.send(snapshot).expect("snapshot");
        drop(sender);
        let mut receiver = Some(receiver);
        assert!(poll_snapshot(&mut receiver, 7).is_some());
        assert!(poll_snapshot(&mut receiver, 7).is_none());
    }
}
