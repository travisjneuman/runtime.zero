//! Deterministic buffer, state, and event fixtures for the operator UI.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

use super::messages::{EventTrace, UiEvent, UiIntent};
use super::model::{
    ActionDisposition, ActionReviewSummary, BoundedId, BoundedText, DetailField, EvidenceRef,
    EvidenceSource, Freshness, ModulePosture, RecordKind, RecordStatus, RedactionState,
    ReviewBoundary, Route, SearchTerms, UI_MODEL_SCHEMA_VERSION, UiActionRef, UiDetailSection,
    UiModel, UiRecord, ViewState,
};
use super::state::UiState;
use super::widgets;

pub fn fixture_model() -> UiModel {
    let mut model = UiModel::loading(11);
    for route in Route::ALL {
        let records = vec![fixture_record(route, 0), fixture_record(route, 1)];
        *model.route_mut(route) = super::model::RouteProjection {
            route,
            state: ViewState::Ready { generation: 11 },
            summary: text(format!("{} evidence is ready", route.title())),
            records,
        };
    }
    model.state = ViewState::Ready { generation: 11 };
    model.status = text("local evidence ready · read-only operator path");
    model.validate().expect("fixture model is valid");
    model
}

pub fn fixture_contribution() -> super::model::ModuleUiContribution {
    super::model::ModuleUiContribution {
        schema_version: UI_MODEL_SCHEMA_VERSION,
        module_id: id("fixture.module"),
        display_name: text("Fixture module"),
        posture: ModulePosture::EnabledReadOnly,
        records: vec![fixture_record(Route::Modules, 0)],
        detail_sections: vec![UiDetailSection {
            title: text("Module posture"),
            fields: vec![DetailField::text("mode", "read-only").expect("fixture field")],
        }],
        action_refs: Vec::new(),
    }
}

pub fn render_text(model: UiModel, width: u16, height: u16, color: bool) -> String {
    render_state_text(UiState::new(model), width, height, color)
}

pub fn render_route_text(route: Route, width: u16, height: u16, color: bool) -> String {
    let mut state = UiState::new(fixture_model());
    state.apply(UiIntent::OpenInventory);
    state.selected = state
        .current_records()
        .iter()
        .position(|locator| locator.route == route)
        .unwrap_or(0);
    state.apply(UiIntent::OpenSelected);
    render_state_text(state, width, height, color)
}

pub fn overview_slice_trace() -> EventTrace {
    let mut trace = EventTrace::new();
    trace.push(UiEvent::SnapshotReady {
        generation: 11,
        model: fixture_model(),
    });
    trace.push(UiEvent::Input(UiIntent::OpenInventory));
    trace.push(UiEvent::Input(UiIntent::SelectIndex(0)));
    trace.push(UiEvent::Input(UiIntent::OpenSelected));
    trace.push(UiEvent::Input(UiIntent::OpenReview));
    trace
}

fn render_state_text(state: UiState, width: u16, height: u16, color: bool) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("fixture terminal");
    terminal
        .draw(|frame| widgets::draw_shell(frame, &state, color))
        .expect("fixture draw");
    frame_text(terminal.backend().buffer())
}

pub fn frame_text(buffer: &Buffer) -> String {
    let area = buffer.area;
    (0..area.height)
        .map(|row| {
            (0..area.width)
                .map(|column| buffer[(area.x + column, area.y + row)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn assert_frame_has_no_control_text(buffer: &Buffer) {
    assert!(
        buffer
            .content()
            .iter()
            .flat_map(|cell| cell.symbol().chars())
            .all(|character| !character.is_control() || character == '\n')
    );
}

fn fixture_record(route: Route, index: usize) -> UiRecord {
    let status = match (route, index) {
        (Route::Overview, 0) => RecordStatus::Ok,
        (Route::Overview, 1) => RecordStatus::Plan,
        (Route::Review, 0) => RecordStatus::Plan,
        (Route::Activity, 0) => RecordStatus::Observed,
        (Route::Modules, 0) => RecordStatus::Info,
        _ => RecordStatus::Info,
    };
    let action_refs = if route == Route::Review && index == 0 {
        vec![UiActionRef {
            action_id: id("fixture/review-action"),
            disposition: ActionDisposition::Reviewable,
            review: ActionReviewSummary {
                operation: text("inspect prepared plan"),
                target: text("fixture evidence"),
                authority: text("foundation action broker"),
                plan_id: id("fixture/plan"),
                plan_sha256: text("fixture-plan-digest"),
                write_set_sha256: text("fixture-write-set-digest"),
                risk: text("medium"),
                requires_confirmation: true,
                requires_elevation: false,
                network_required: false,
                capabilities: vec![text("manager-execution")],
                rollback: text("foundation recovery evidence"),
                executed: false,
            },
        }]
    } else {
        Vec::new()
    };
    let review_boundary = if route == Route::Overview && index == 0 {
        Some(ReviewBoundary {
            reference_id: id("fixture/review-boundary"),
            disposition: ActionDisposition::Reviewable,
            message: text("Read-only boundary; the foundation owns any later confirmation."),
        })
    } else {
        None
    };
    UiRecord {
        record_id: id(format!(
            "fixture/{}/{index}",
            route.title().to_ascii_lowercase()
        )),
        module_id: id("fixture.module"),
        kind: if route == Route::Review {
            RecordKind::ActionReview
        } else {
            RecordKind::Readiness
        },
        title: text(format!("{} evidence {index}", route.title())),
        summary: text("bounded fixture evidence; no mutation authority"),
        status,
        details: vec![UiDetailSection {
            title: text("Evidence"),
            fields: vec![
                DetailField::text("source", "deterministic testkit").expect("fixture field"),
                DetailField::text("mode", "read-only").expect("fixture field"),
            ],
        }],
        evidence: vec![EvidenceRef {
            source: EvidenceSource::LocalSnapshot,
            reference_id: id(format!("fixture/evidence/{index}")),
            freshness: Freshness::Fresh,
            redaction: RedactionState::Public,
        }],
        action_refs,
        review_boundary,
        search_terms: SearchTerms(vec![text(route.title()), text("fixture")]),
    }
}

fn text(value: impl Into<String>) -> BoundedText {
    BoundedText::try_new(value).expect("fixture text is bounded")
}

fn id(value: impl Into<String>) -> BoundedId {
    BoundedId::try_new(value).expect("fixture id is valid")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::model::{ModuleUiContribution, UiRegistry, UiValidationError};

    #[test]
    fn fixture_covers_typed_evidence_and_foundation_action_boundary() {
        let model = fixture_model();
        assert_eq!(model.routes.len(), Route::ALL.len());
        assert!(
            model.route(Route::Review).records[0].action_refs[0]
                .review
                .requires_confirmation
        );
        assert!(
            !model.route(Route::Review).records[0].action_refs[0]
                .review
                .executed
        );
    }

    #[test]
    fn contribution_registry_is_deterministic_and_rejects_duplicate_module() {
        let mut registry = UiRegistry::default();
        registry
            .register(fixture_contribution())
            .expect("first registration");
        let duplicate = registry.register(fixture_contribution());
        assert!(matches!(
            duplicate,
            Err(UiValidationError::DuplicateModuleId(_))
        ));
        assert_eq!(
            registry.contributions()[0].module_id.as_str(),
            "fixture.module"
        );
    }

    #[test]
    fn buffer_rendering_is_stable_across_color_modes() {
        let no_color = render_text(fixture_model(), 118, 30, false);
        let color = render_text(fixture_model(), 118, 30, true);
        assert_eq!(no_color, color);
        assert!(no_color.contains("runtime.zero"));
        assert!(no_color.contains("HOME"));
        assert!(no_color.contains("read-only"));
    }

    #[test]
    fn terminal_floor_is_bounded_and_explains_cli_escape() {
        let text = render_text(fixture_model(), 42, 10, false);
        assert!(text.contains("Terminal too small"));
        assert!(text.contains("rz0 --no-tui"));
    }

    #[test]
    fn every_evidence_category_is_reachable_without_a_route_bar() {
        for route in Route::ALL {
            let rendered = render_route_text(route, 118, 30, false);
            assert!(
                rendered.contains("EVIDENCE"),
                "missing evidence page for {route:?}"
            );
            assert!(rendered.contains("read-only"));
        }
    }

    #[test]
    fn task_flow_renders_evidence_and_exact_plan_at_all_required_sizes() {
        for (width, height) in [(58, 16), (80, 24), (118, 30), (160, 50)] {
            for color in [false, true] {
                let mut state = UiState::new(fixture_model());
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
                    .expect("fixture action");
                state.apply(UiIntent::SelectIndex(action_index));
                state.apply(UiIntent::OpenSelected);
                assert!(
                    render_state_text(state.clone(), width, height, color).contains("Evidence")
                );
                state.apply(UiIntent::OpenReview);
                let review = render_state_text(state, width, height, color);
                assert!(review.contains("Exact plan"), "review at {width}x{height}");
                assert!(review.contains("not executed"));
            }
        }
    }

    #[test]
    fn vertical_slice_trace_is_explicit_and_bounded() {
        let trace = overview_slice_trace();
        assert_eq!(trace.events().len(), 5);
        assert!(matches!(
            trace.events()[0],
            UiEvent::SnapshotReady { generation: 11, .. }
        ));
        assert!(matches!(
            trace.events()[4],
            UiEvent::Input(UiIntent::OpenReview)
        ));
    }

    #[test]
    fn hostile_terminal_text_is_rejected_before_rendering() {
        assert!(matches!(
            BoundedText::try_new("\u{1b}[31mred"),
            Err(UiValidationError::ControlText)
        ));
        assert!(matches!(
            BoundedText::try_new("/Users/private/raw-evidence"),
            Err(UiValidationError::RawPath)
        ));
        assert!(matches!(
            BoundedId::try_new("module with spaces"),
            Err(UiValidationError::InvalidId)
        ));
        let too_many = ModuleUiContribution {
            records: (0..257)
                .map(|index| fixture_record(Route::Modules, index))
                .collect(),
            ..fixture_contribution()
        };
        assert!(matches!(
            too_many.validate(),
            Err(UiValidationError::TooManyRecords { .. })
        ));
    }

    #[test]
    fn module_contribution_cannot_claim_execution_or_cross_ownership() {
        let mut contribution = fixture_contribution();
        contribution.records[0].module_id = id("other.module");
        assert!(matches!(
            contribution.validate(),
            Err(UiValidationError::MismatchedModuleId(_))
        ));
        let mut contribution = fixture_contribution();
        contribution.action_refs.push(UiActionRef {
            action_id: id("fixture/executed-claim"),
            disposition: ActionDisposition::Reviewable,
            review: ActionReviewSummary {
                operation: text("unsafe claim"),
                target: text("fixture"),
                authority: text("foundation"),
                plan_id: id("fixture/plan"),
                plan_sha256: text("fixture-plan-digest"),
                write_set_sha256: text("fixture-write-set-digest"),
                risk: text("blocked"),
                requires_confirmation: true,
                requires_elevation: false,
                network_required: false,
                capabilities: Vec::new(),
                rollback: text("foundation recovery evidence"),
                executed: true,
            },
        });
        assert!(matches!(
            contribution.validate(),
            Err(UiValidationError::ExecutionClaim(_))
        ));
    }
}
