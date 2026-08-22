use runtime_zero::ui::messages::{UiEvent, UiIntent};
use runtime_zero::ui::model::{ActionDisposition, BoundedId, BoundedText, ConfirmationPrompt};
use runtime_zero::ui::state::{UiPage, UiState};
use runtime_zero::ui::testkit::{fixture_model, render_text};

#[test]
fn operator_can_follow_home_evidence_review_confirmation_and_verified_activity() {
    let mut state = UiState::new(fixture_model());
    assert_eq!(state.page, UiPage::Home);
    assert!(!state.current_records().is_empty());

    state.apply(UiIntent::OpenInventory);
    let action_index = state
        .current_records()
        .iter()
        .position(|locator| {
            state.model.route(locator.route).records[locator.index]
                .action_refs
                .iter()
                .any(|action| action.disposition == ActionDisposition::Reviewable)
        })
        .expect("fixture has one foundation action");
    state.apply(UiIntent::SelectIndex(action_index));
    state.apply(UiIntent::OpenSelected);
    assert_eq!(state.page, UiPage::Evidence);
    state.apply(UiIntent::OpenSelected);
    assert_eq!(state.page, UiPage::Review);

    let prepare = state.apply(UiIntent::BeginConfirmation);
    assert!(matches!(prepare, Some(UiIntent::PrepareAction(_))));
    assert_eq!(state.page, UiPage::Activity);

    state.set_confirmation(ConfirmationPrompt {
        action_id: BoundedId::try_new("fixture/review-action").expect("id"),
        plan_id: BoundedId::try_new("fixture/plan").expect("id"),
        plan_sha256: BoundedText::try_new("fixture-plan-digest").expect("text"),
        target: BoundedText::try_new("fixture evidence").expect("text"),
        expected_phrase: BoundedText::try_new("CONFIRM fixture").expect("text"),
        risk: BoundedText::try_new("medium").expect("text"),
        expires_unix_seconds: 1,
        rollback_available: true,
        manual_recovery_acknowledged: false,
    });
    assert_eq!(state.page, UiPage::Confirmation);
    state.apply(UiIntent::ConfirmationCharacter('C'));
    assert_eq!(state.confirmation_input, "C");
    assert_eq!(
        state.apply(UiIntent::Back),
        Some(UiIntent::CancelConfirmation)
    );
    assert_eq!(state.page, UiPage::Review);

    state.apply_event(UiEvent::JobRunning {
        job_id: BoundedId::try_new("fixture/job").expect("id"),
        phase: BoundedText::try_new("executing foundation transaction").expect("text"),
    });
    state.apply_event(UiEvent::JobSucceeded {
        receipt: BoundedId::try_new("fixture/receipt").expect("id"),
        verification: BoundedId::try_new("fixture/verification").expect("id"),
    });
    assert_eq!(state.page, UiPage::Activity);
    assert_eq!(state.view_state().label(), "verified");
}

#[test]
fn recovery_and_cancelled_states_are_not_collapsed_into_success() {
    let mut state = UiState::new(fixture_model());
    state.apply_event(UiEvent::RecoveryRequired {
        transaction: BoundedId::try_new("fixture/transaction").expect("id"),
        decision: BoundedText::try_new("read-only review required").expect("text"),
    });
    assert_eq!(state.view_state().label(), "recovery-required");
    state.apply_event(UiEvent::JobCancelled {
        job_id: BoundedId::try_new("fixture/job").expect("id"),
        reason: BoundedText::try_new("operator cancelled").expect("text"),
    });
    assert_eq!(state.view_state().label(), "cancelled");
}

#[test]
fn buffers_are_deterministic_at_resize_sizes_and_no_color_keeps_semantics() {
    for (width, height) in [(58, 16), (80, 24), (118, 30), (160, 50)] {
        let plain = render_text(fixture_model(), width, height, false);
        let no_color_again = render_text(fixture_model(), width, height, false);
        let color = render_text(fixture_model(), width, height, true);
        assert_eq!(plain, no_color_again);
        assert_eq!(plain, color);
        assert!(plain.contains("HOME"), "missing home at {width}x{height}");
        assert!(
            plain.contains("Enter inspect"),
            "missing controls at {width}x{height}"
        );
    }
    let small = render_text(fixture_model(), 42, 10, false);
    assert!(small.contains("Terminal too small"));
    assert!(small.contains("rz0 --no-tui"));
}
