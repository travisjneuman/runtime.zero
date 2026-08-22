use super::model::{
    ActionDisposition as UiActionDisposition, ActionReviewSummary, BoundedId, BoundedText,
    ConfirmationPrompt, DetailField, EvidenceRef, EvidenceSource, Freshness, RecordKind,
    RecordStatus, RedactionState, ReviewBoundary, Route, RouteProjection, SearchTerms, UiActionRef,
    UiModel, UiRecord, ViewState,
};
use crate::tui_dashboard::{TuiDashboard, TuiRow, TuiSection};
use crate::update_cli::TuiUpdateChallenge;
use crate::updates::LiveUpdateReview;
use rz0_action_plan::{ActionDisposition, ActionKind, ActionPlan};

pub fn loading_model(generation: u64) -> UiModel {
    UiModel::loading(generation)
}

pub fn refreshing_model(generation: u64) -> UiModel {
    UiModel::refreshing(generation)
}

pub fn unavailable_model(generation: u64, reason: &str) -> UiModel {
    UiModel::unavailable(generation, public_text(reason).as_str())
}

pub(crate) fn confirmation_prompt(challenge: &TuiUpdateChallenge) -> ConfirmationPrompt {
    let view = &challenge.view;
    ConfirmationPrompt {
        action_id: bounded_id(view.action_id.clone()),
        plan_id: bounded_id(view.plan_id.clone()),
        plan_sha256: bounded(view.plan_sha256.clone()),
        target: bounded(public_text(&view.target)),
        expected_phrase: bounded(public_text(&view.expected_phrase)),
        risk: bounded(format!("{:?}", view.risk).to_ascii_lowercase()),
        expires_unix_seconds: view.expires_unix_seconds,
        rollback_available: view.rollback_available,
        manual_recovery_acknowledged: view.manual_recovery_acknowledged,
    }
}

pub fn model_from_dashboard(dashboard: &TuiDashboard, generation: u64) -> UiModel {
    model_from_dashboard_and_review(dashboard, generation, None, None)
}

pub(crate) fn model_from_dashboard_and_review(
    dashboard: &TuiDashboard,
    generation: u64,
    review: Option<&LiveUpdateReview>,
    review_error: Option<&str>,
) -> UiModel {
    let mut model = UiModel::loading(generation);
    let ready = ViewState::Ready { generation };
    let overview = records_for_section(
        dashboard,
        "overview",
        Route::Overview,
        RecordKind::Readiness,
    );
    let explore = dashboard
        .sections
        .iter()
        .flat_map(|section| records_from_section(section, Route::Explore, RecordKind::Inventory))
        .collect::<Vec<_>>();
    let mut review_records = dashboard
        .sections
        .iter()
        .flat_map(|section| {
            records_from_section(section, Route::Review, RecordKind::ActionReview)
                .into_iter()
                .filter(|record| {
                    record.status == RecordStatus::Plan
                        || record.status == RecordStatus::Warn
                        || record.status == RecordStatus::Blocked
                        || record.review_boundary.as_ref().is_some_and(|boundary| {
                            boundary.disposition != UiActionDisposition::Unavailable
                        })
                })
        })
        .collect::<Vec<_>>();
    if let Some(review) = review {
        review_records.extend(action_records(review.plan.as_ref()));
    }
    let activity = dashboard
        .sections
        .iter()
        .filter(|section| matches!(section.title, "system" | "diagnostics"))
        .flat_map(|section| records_from_section(section, Route::Activity, RecordKind::Activity))
        .collect::<Vec<_>>();
    let modules = dashboard
        .sections
        .iter()
        .filter(|section| section.title == "diagnostics")
        .flat_map(|section| records_from_section(section, Route::Modules, RecordKind::Module))
        .collect::<Vec<_>>();

    set_route(
        &mut model,
        Route::Overview,
        ready.clone(),
        "attention and next safe step",
        overview,
    );
    set_route(
        &mut model,
        Route::Explore,
        ready.clone(),
        "searchable local evidence",
        explore,
    );
    set_route(
        &mut model,
        Route::Review,
        ready.clone(),
        "plans, findings, and blocked boundaries",
        review_records,
    );
    if let Some(reason) = review_error {
        let projection = model.route_mut(Route::Review);
        projection.state = ViewState::Unavailable {
            generation,
            reason: bounded(format!(
                "provider review unavailable · {}",
                public_text(reason)
            )),
        };
        projection.summary = bounded(format!(
            "provider review unavailable · {}",
            public_text(reason)
        ));
    } else if review.is_some() {
        model.route_mut(Route::Review).summary =
            bounded("provider review complete · plans and blocked boundaries");
        model.status = bounded(format!(
            "{} software · {} modules · provider review complete",
            dashboard.installed_software_count, dashboard.installed_module_count
        ));
    }
    set_route(
        &mut model,
        Route::Activity,
        ready.clone(),
        "running, receipt, and recovery evidence",
        activity,
    );
    set_route(
        &mut model,
        Route::Modules,
        ready,
        "module posture from foundation evidence",
        modules,
    );
    model.state = ViewState::Ready { generation };
    model.status = bounded(format!(
        "{} software · {} modules · review first",
        dashboard.installed_software_count, dashboard.installed_module_count
    ));
    model.job = super::model::JobState::Idle;
    model
        .validate()
        .expect("foundation adapter creates a valid UI model");
    model
}

fn action_records(plan: Option<&ActionPlan>) -> Vec<UiRecord> {
    let Some(plan) = plan else {
        return Vec::new();
    };
    plan.actions
        .iter()
        .map(|action| {
            let disposition = action_disposition(action.disposition);
            let status = match action.disposition {
                ActionDisposition::Planned => RecordStatus::Plan,
                ActionDisposition::Blocked => RecordStatus::Blocked,
                ActionDisposition::Unsupported => RecordStatus::Warn,
            };
            let mut fields = Vec::new();
            push_text_field(&mut fields, "operation", action_kind_label(action.kind));
            push_text_field(&mut fields, "target", public_text(&action.target));
            push_text_field(&mut fields, "plan", plan.plan_id.clone());
            push_text_field(&mut fields, "plan sha256", plan_digest(plan));
            if let Some(manager) = action.manager.as_deref() {
                push_text_field(&mut fields, "manager", public_text(manager));
            }
            push_text_field(
                &mut fields,
                "risk",
                format!("{:?}", action.risk).to_ascii_lowercase(),
            );
            push_text_field(
                &mut fields,
                "confirmation",
                if action.requires_confirmation {
                    "required"
                } else {
                    "not required"
                },
            );
            push_text_field(
                &mut fields,
                "elevation",
                if action.requires_elevation {
                    "required"
                } else {
                    "not required"
                },
            );
            push_text_field(
                &mut fields,
                "network",
                if action.network_required {
                    "required"
                } else {
                    "not required"
                },
            );
            push_text_field(
                &mut fields,
                "rollback",
                public_text(&action.rollback.description),
            );
            UiRecord {
                record_id: bounded_id(format!("action/{}", action.action_id)),
                module_id: bounded_id("first-party.updater"),
                kind: RecordKind::ActionReview,
                title: bounded(public_text(&action.target)),
                summary: bounded("foundation action plan · read-only review"),
                status,
                details: vec![super::model::UiDetailSection {
                    title: bounded("Action plan"),
                    fields,
                }],
                evidence: vec![EvidenceRef {
                    source: EvidenceSource::ActionPlan,
                    reference_id: bounded_id(plan.plan_id.clone()),
                    freshness: Freshness::Fresh,
                    redaction: RedactionState::SensitiveOmitted,
                }],
                action_refs: vec![UiActionRef {
                    action_id: bounded_id(action.action_id.clone()),
                    disposition,
                    review: ActionReviewSummary {
                        operation: bounded(action_kind_label(action.kind)),
                        target: bounded(public_text(&action.target)),
                        authority: bounded(
                            "foundation action-plan / confirmation / transaction contracts",
                        ),
                        plan_id: bounded_id(plan.plan_id.clone()),
                        plan_sha256: bounded(plan_digest(plan)),
                        write_set_sha256: bounded(write_set_digest(plan)),
                        risk: bounded(format!("{:?}", action.risk).to_ascii_lowercase()),
                        requires_confirmation: action.requires_confirmation,
                        requires_elevation: action.requires_elevation,
                        network_required: action.network_required,
                        capabilities: action
                            .capabilities
                            .iter()
                            .map(|capability| {
                                bounded(format!("{capability:?}").to_ascii_lowercase())
                            })
                            .collect(),
                        rollback: bounded(public_text(&action.rollback.description)),
                        executed: false,
                    },
                }],
                review_boundary: None,
                search_terms: SearchTerms(vec![
                    bounded("action plan"),
                    bounded(action_kind_label(action.kind)),
                    bounded(public_text(&action.target)),
                ]),
            }
        })
        .collect()
}

fn action_disposition(disposition: ActionDisposition) -> UiActionDisposition {
    match disposition {
        ActionDisposition::Planned => UiActionDisposition::Reviewable,
        ActionDisposition::Blocked | ActionDisposition::Unsupported => UiActionDisposition::Blocked,
    }
}

fn action_kind_label(kind: ActionKind) -> &'static str {
    match kind {
        ActionKind::Update => "update",
        ActionKind::Uninstall => "uninstall",
        ActionKind::Quarantine => "quarantine",
        ActionKind::Restore => "restore",
        ActionKind::ModuleInstall => "module install",
    }
}

fn plan_digest(plan: &ActionPlan) -> String {
    rz0_action_plan::action_plan_digests(plan)
        .map(|digests| digests.plan_sha256)
        .unwrap_or_else(|_| "unsealed-plan".to_string())
}

fn write_set_digest(plan: &ActionPlan) -> String {
    rz0_action_plan::action_plan_digests(plan)
        .map(|digests| digests.write_set_sha256)
        .unwrap_or_else(|_| "unsealed-write-set".to_string())
}

fn push_text_field(fields: &mut Vec<DetailField>, label: &str, value: impl Into<String>) {
    if let Ok(field) = DetailField::text(label, public_text(&value.into())) {
        fields.push(field);
    }
}

fn set_route(
    model: &mut UiModel,
    route: Route,
    ready: ViewState,
    summary: &str,
    records: Vec<UiRecord>,
) {
    let state = if records.is_empty() {
        ViewState::Empty {
            generation: ready.generation(),
        }
    } else if records
        .iter()
        .all(|record| record.status == RecordStatus::Blocked)
    {
        ViewState::Blocked {
            generation: ready.generation(),
            reason: bounded("all evidence is blocked by foundation policy"),
        }
    } else {
        ready
    };
    *model.route_mut(route) = RouteProjection {
        route,
        state,
        summary: bounded(summary.to_string()),
        records,
    };
}

fn records_for_section(
    dashboard: &TuiDashboard,
    title: &str,
    route: Route,
    kind: RecordKind,
) -> Vec<UiRecord> {
    dashboard
        .sections
        .iter()
        .find(|section| section.title == title)
        .map(|section| records_from_section(section, route, kind))
        .unwrap_or_default()
}

fn records_from_section(section: &TuiSection, route: Route, kind: RecordKind) -> Vec<UiRecord> {
    section
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| record_from_row(section, index, row, route, kind))
        .collect()
}

fn record_from_row(
    section: &TuiSection,
    index: usize,
    row: &TuiRow,
    route: Route,
    kind: RecordKind,
) -> UiRecord {
    let title = public_text(&row.value);
    let summary = row
        .preview
        .as_deref()
        .map(public_text)
        .unwrap_or_else(|| public_text(section.summary));
    let status = record_status(row.label, row.tone);
    let reference_id = bounded_id(format!(
        "evidence/{}/{index}",
        route.title().to_ascii_lowercase()
    ));
    let module_id = bounded_id(format!("core.{}", section.title));
    let action_disposition = match status {
        RecordStatus::Plan => UiActionDisposition::Reviewable,
        RecordStatus::Blocked | RecordStatus::Warn => UiActionDisposition::Blocked,
        _ => UiActionDisposition::Unavailable,
    };
    let review_boundary = matches!(
        status,
        RecordStatus::Plan | RecordStatus::Warn | RecordStatus::Blocked
    )
    .then(|| ReviewBoundary {
        reference_id: bounded_id(format!(
            "review-boundary/{}/{index}",
            route.title().to_ascii_lowercase()
        )),
        disposition: action_disposition,
        message: bounded(
            "Read-only review boundary. The foundation must provide an exact action plan before any confirmation or execution path exists.",
        ),
    });
    let mut details = Vec::new();
    if let Ok(field) = DetailField::text("source", section.title) {
        details.push(field);
    }
    if let Ok(field) = DetailField::text("state", row.label) {
        details.push(field);
    }
    if let Some(preview) = row.preview.as_deref()
        && let Ok(field) = DetailField::text("evidence", public_text(preview))
    {
        details.push(field);
    }
    UiRecord {
        record_id: bounded_id(format!(
            "{}/{}/{index}",
            route.title().to_ascii_lowercase(),
            section.code
        )),
        module_id,
        kind,
        title: bounded(title),
        summary: bounded(summary),
        status,
        details: vec![super::model::UiDetailSection {
            title: bounded("Evidence"),
            fields: details,
        }],
        evidence: vec![EvidenceRef {
            source: source_for(kind),
            reference_id,
            freshness: Freshness::Fresh,
            redaction: RedactionState::PathRedacted,
        }],
        action_refs: Vec::new(),
        review_boundary,
        search_terms: SearchTerms(vec![
            bounded(section.title),
            bounded(row.label),
            bounded(&row.value),
        ]),
    }
}

fn source_for(kind: RecordKind) -> EvidenceSource {
    match kind {
        RecordKind::Provider | RecordKind::ActionReview => EvidenceSource::ProviderReview,
        RecordKind::Recovery => EvidenceSource::RecoveryReview,
        RecordKind::Module => EvidenceSource::ModuleRegistry,
        RecordKind::System | RecordKind::Activity => EvidenceSource::SystemMonitor,
        RecordKind::Diagnostic => EvidenceSource::CliContract,
        RecordKind::Inventory => EvidenceSource::Inventory,
        RecordKind::Readiness => EvidenceSource::LocalSnapshot,
        RecordKind::Finding => EvidenceSource::LocalSnapshot,
    }
}

fn record_status(label: &str, tone: &str) -> RecordStatus {
    match label {
        "[OK]" => RecordStatus::Ok,
        "[INFO]" => RecordStatus::Info,
        "[PLAN]" => RecordStatus::Plan,
        "[DRY-RUN]" => RecordStatus::DryRun,
        "[BLOCKED]" => RecordStatus::Blocked,
        "[ERROR]" => RecordStatus::Error,
        "[WARN]" => RecordStatus::Warn,
        "[SKIP]" => RecordStatus::Muted,
        _ if tone == "safe" => RecordStatus::Ok,
        _ if tone == "warn" => RecordStatus::Warn,
        _ => RecordStatus::Observed,
    }
}

fn public_text(value: &str) -> String {
    if value.contains("/Users/")
        || value.contains("/private/")
        || value.contains("\\Users\\")
        || value.contains("/var/")
    {
        return BoundedText::redacted().into_string();
    }
    let value = value.replace(['\n', '\r', '\t'], " ");
    if value.chars().count() > super::model::MAX_UI_TEXT_CHARS {
        value
            .chars()
            .take(super::model::MAX_UI_TEXT_CHARS.saturating_sub(1))
            .chain(std::iter::once('…'))
            .collect()
    } else if value.is_empty() {
        "[no detail]".to_string()
    } else {
        value
    }
}

fn bounded(value: impl Into<String>) -> BoundedText {
    BoundedText::try_new(value).unwrap_or_else(|_| BoundedText::redacted())
}

fn bounded_id(value: impl Into<String>) -> BoundedId {
    let value = value.into().replace(' ', "-");
    BoundedId::try_new(value)
        .unwrap_or_else(|_| BoundedId::try_new("ui.invalid").expect("static id is valid"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui_dashboard;

    #[test]
    fn adapter_produces_all_stable_routes_without_raw_paths() {
        let model = model_from_dashboard(&tui_dashboard::dashboard(), 7);
        assert_eq!(model.state.label(), "ready");
        for route in Route::ALL {
            assert!(!model.route(route).summary.as_str().is_empty());
            for record in &model.route(route).records {
                assert!(!record.title.as_str().contains("/Users/"));
                assert_eq!(record.evidence[0].redaction, RedactionState::PathRedacted);
            }
        }
    }

    #[test]
    fn adapter_marks_action_like_records_as_review_boundaries_only() {
        let model = model_from_dashboard(&tui_dashboard::dashboard(), 1);
        assert!(
            model
                .routes
                .iter()
                .flat_map(|route| route.records.iter())
                .any(|record| record.review_boundary.is_some())
        );
        assert!(
            model
                .routes
                .iter()
                .flat_map(|route| route.records.iter())
                .all(|record| record.action_refs.is_empty())
        );
    }
}
